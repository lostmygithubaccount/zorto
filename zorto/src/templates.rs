use std::collections::HashMap;

use crate::config::Config;
use crate::content::{Page, Section};

/// A taxonomy term for template rendering
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxonomyTerm {
    pub name: String,
    pub slug: String,
    pub permalink: String,
    pub pages: Vec<Page>,
}

/// Paginator for template rendering
#[derive(Debug, Clone, serde::Serialize)]
pub struct Paginator {
    pub pages: Vec<Page>,
    pub current_index: usize,
    pub number_pagers: usize,
    pub previous: Option<String>,
    pub next: Option<String>,
    pub first: String,
    pub last: String,
}

/// Set up Tera engine with custom functions, filters, and tests
pub fn setup_tera(
    templates_dir: &std::path::Path,
    config: &Config,
    sections: &HashMap<String, Section>,
) -> anyhow::Result<tera::Tera> {
    let templates_glob = format!("{}/**/*.html", templates_dir.display());
    let mut tera = tera::Tera::new(&templates_glob)?;

    // Register custom functions
    register_functions(&mut tera, config, sections);

    // Register custom filters
    register_filters(&mut tera);

    // Register custom tests
    register_tests(&mut tera);

    Ok(tera)
}

fn register_functions(tera: &mut tera::Tera, config: &Config, sections: &HashMap<String, Section>) {
    // get_url function
    let base_url = config.base_url.clone();
    tera.register_function(
        "get_url",
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("get_url requires a 'path' argument"))?;

            if let Some(content_path) = path.strip_prefix("@/") {
                // Check if it's a section
                if content_path.ends_with("_index.md") {
                    let dir = std::path::Path::new(content_path)
                        .parent()
                        .unwrap_or(std::path::Path::new(""))
                        .to_string_lossy()
                        .to_string();
                    let url = if dir.is_empty() {
                        format!("{}/", base_url)
                    } else {
                        format!("{}/{dir}/", base_url)
                    };
                    return Ok(tera::Value::String(url));
                }
                // Regular page
                let stem = std::path::Path::new(content_path)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                let parent = std::path::Path::new(content_path)
                    .parent()
                    .unwrap_or(std::path::Path::new(""))
                    .to_string_lossy()
                    .to_string();
                let slug = slug::slugify(stem.as_ref());
                let url = if parent.is_empty() {
                    format!("{}/{slug}/", base_url)
                } else {
                    format!("{}/{parent}/{slug}/", base_url)
                };
                Ok(tera::Value::String(url))
            } else {
                // Static file or external URL
                if path.starts_with("http://") || path.starts_with("https://") {
                    Ok(tera::Value::String(path.to_string()))
                } else {
                    let url = format!("{}/{}", base_url, path.trim_start_matches('/'));
                    Ok(tera::Value::String(url))
                }
            }
        },
    );

    // get_section function
    let sections_clone = sections.clone();
    tera.register_function(
        "get_section",
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("get_section requires a 'path' argument"))?;

            if let Some(section) = sections_clone.get(path) {
                let val = serde_json::to_value(section)
                    .map_err(|e| tera::Error::msg(format!("Serialization error: {e}")))?;
                Ok(val)
            } else {
                Err(tera::Error::msg(format!("Section not found: {path}")))
            }
        },
    );

    // get_taxonomy_url function
    let base_url2 = config.base_url.clone();
    tera.register_function(
        "get_taxonomy_url",
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let kind = args
                .get("kind")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("get_taxonomy_url requires 'kind'"))?;
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| tera::Error::msg("get_taxonomy_url requires 'name'"))?;

            let slug = slug::slugify(name);
            let url = format!("{}/{kind}/{slug}/", base_url2);
            Ok(tera::Value::String(url))
        },
    );

    // now() function
    tera.register_function(
        "now",
        |_args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
            Ok(tera::Value::String(now))
        },
    );
}

fn register_filters(tera: &mut tera::Tera) {
    // pluralize filter
    tera.register_filter(
        "pluralize",
        |value: &tera::Value, _args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let n = value
                .as_u64()
                .or_else(|| value.as_i64().map(|i| i as u64))
                .or_else(|| value.as_f64().map(|f| f as u64))
                .unwrap_or(0);
            if n == 1 {
                Ok(tera::Value::String(String::new()))
            } else {
                Ok(tera::Value::String("s".to_string()))
            }
        },
    );

    // slice filter with named 'end' parameter (Zola-compatible)
    tera.register_filter(
        "slice",
        |value: &tera::Value, args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let arr = value
                .as_array()
                .ok_or_else(|| tera::Error::msg("slice filter requires an array"))?;

            let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let end = args
                .get("end")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(arr.len());

            let end = end.min(arr.len());
            let start = start.min(end);

            Ok(tera::Value::Array(arr[start..end].to_vec()))
        },
    );

    // date filter (enhance the built-in one to handle our string dates)
    tera.register_filter(
        "date",
        |value: &tera::Value, args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let date_str = value
                .as_str()
                .ok_or_else(|| tera::Error::msg("date filter requires a string"))?;

            let format = args
                .get("format")
                .and_then(|v| v.as_str())
                .unwrap_or("%Y-%m-%d");

            // Try parsing various date formats
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
                return Ok(tera::Value::String(dt.format(format).to_string()));
            }
            if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                return Ok(tera::Value::String(d.format(format).to_string()));
            }

            // Return as-is if parsing fails
            Ok(tera::Value::String(date_str.to_string()))
        },
    );
}

fn register_tests(tera: &mut tera::Tera) {
    // starting_with test
    tera.register_tester(
        "starting_with",
        |value: Option<&tera::Value>, args: &[tera::Value]| -> tera::Result<bool> {
            let value = value.and_then(|v| v.as_str()).unwrap_or("");
            let prefix = args.first().and_then(|v| v.as_str()).unwrap_or("");

            Ok(value.starts_with(prefix))
        },
    );
}

/// Build Tera context for a page template
pub fn page_context(page: &Page, config: &Config) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("page", page);
    ctx.insert("config", &config_to_value(config));
    ctx.insert("section", &tera::Value::Null);
    ctx
}

/// Build Tera context for a section template
pub fn section_context(
    section: &Section,
    config: &Config,
    paginator: Option<&Paginator>,
) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("section", section);
    ctx.insert("config", &config_to_value(config));
    ctx.insert("page", &tera::Value::Null);
    if let Some(pag) = paginator {
        ctx.insert("paginator", pag);
    }
    ctx
}

/// Build Tera context for taxonomy list template
pub fn taxonomy_list_context(terms: &[TaxonomyTerm], config: &Config) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("terms", terms);
    ctx.insert("config", &config_to_value(config));
    ctx.insert("page", &tera::Value::Null);
    ctx.insert("section", &tera::Value::Null);
    ctx
}

/// Build Tera context for taxonomy single template
pub fn taxonomy_single_context(term: &TaxonomyTerm, config: &Config) -> tera::Context {
    let mut ctx = tera::Context::new();
    ctx.insert("term", term);
    ctx.insert("config", &config_to_value(config));
    ctx.insert("page", &tera::Value::Null);
    ctx.insert("section", &tera::Value::Null);
    ctx
}

/// Convert Config to a Tera-compatible Value
pub fn config_to_value(config: &Config) -> serde_json::Value {
    serde_json::to_value(config).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::content::{Frontmatter, build_page, build_section};
    use tempfile::TempDir;

    fn minimal_config() -> Config {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("config.toml"),
            r#"
base_url = "https://example.com"
title = "Test Site"

[extra]
author = "Tester"
"#,
        )
        .unwrap();
        Config::load(tmp.path()).unwrap()
    }

    fn minimal_page() -> Page {
        build_page(
            Frontmatter {
                title: Some("Test Page".into()),
                ..Default::default()
            },
            "Hello world".into(),
            "posts/test.md",
            "https://example.com",
        )
    }

    fn minimal_section() -> Section {
        build_section(
            Frontmatter {
                title: Some("Blog".into()),
                ..Default::default()
            },
            "Section content".into(),
            "posts/_index.md",
            "https://example.com",
        )
    }

    #[test]
    fn test_config_to_value_fields() {
        let config = minimal_config();
        let val = config_to_value(&config);
        assert_eq!(val["base_url"], "https://example.com");
        assert_eq!(val["title"], "Test Site");
        assert_eq!(val["extra"]["author"], "Tester");
        assert!(val["markdown"].is_object());
    }

    #[test]
    fn test_page_context_keys() {
        let config = minimal_config();
        let page = minimal_page();
        let ctx = page_context(&page, &config);
        let json = ctx.into_json();
        assert!(json.get("page").is_some());
        assert!(json.get("config").is_some());
        assert!(json.get("section").unwrap().is_null());
    }

    #[test]
    fn test_section_context_keys() {
        let config = minimal_config();
        let section = minimal_section();
        let ctx = section_context(&section, &config, None);
        let json = ctx.into_json();
        assert!(json.get("section").is_some());
        assert!(json.get("config").is_some());
        assert!(json.get("page").unwrap().is_null());
    }

    #[test]
    fn test_section_context_with_paginator() {
        let config = minimal_config();
        let section = minimal_section();
        let pag = Paginator {
            pages: vec![],
            current_index: 1,
            number_pagers: 3,
            previous: None,
            next: Some("https://example.com/posts/page/2/".into()),
            first: "https://example.com/posts/".into(),
            last: "https://example.com/posts/page/3/".into(),
        };
        let ctx = section_context(&section, &config, Some(&pag));
        let json = ctx.into_json();
        let p = json.get("paginator").unwrap();
        assert_eq!(p["current_index"], 1);
        assert_eq!(p["number_pagers"], 3);
    }

    #[test]
    fn test_pluralize_filter() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(tmpl_dir.join("test.html"), "{{ count | pluralize }}").unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("count", &1);
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "");
        ctx.insert("count", &5);
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "s");
    }

    #[test]
    fn test_slice_filter() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(
            tmpl_dir.join("test.html"),
            r#"{% for item in items | slice(end=2) %}{{ item }}{% endfor %}"#,
        )
        .unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("items", &vec!["a", "b", "c", "d"]);
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "ab");
    }

    #[test]
    fn test_date_filter() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(
            tmpl_dir.join("test.html"),
            r#"{{ d | date(format="%B %d, %Y") }}"#,
        )
        .unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("d", "2025-06-15");
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "June 15, 2025");
    }

    #[test]
    fn test_starting_with_tester() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(
            tmpl_dir.join("test.html"),
            r#"{% if path is starting_with("/blog") %}yes{% else %}no{% endif %}"#,
        )
        .unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let mut ctx = tera::Context::new();
        ctx.insert("path", "/blog/post");
        assert_eq!(tera.render("test.html", &ctx).unwrap(), "yes");
        ctx.insert("path", "/about");
        assert_eq!(tera.render("test.html", &ctx).unwrap(), "no");
    }

    #[test]
    fn test_get_url_content_path() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(
            tmpl_dir.join("test.html"),
            r#"{{ get_url(path="@/posts/hello.md") | safe }}"#,
        )
        .unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let ctx = tera::Context::new();
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "https://example.com/posts/hello/");
    }

    #[test]
    fn test_get_url_static_path() {
        let tmp = TempDir::new().unwrap();
        let tmpl_dir = tmp.path().join("templates");
        std::fs::create_dir_all(&tmpl_dir).unwrap();
        std::fs::write(
            tmpl_dir.join("test.html"),
            r#"{{ get_url(path="/img/photo.png") | safe }}"#,
        )
        .unwrap();
        let config = minimal_config();
        let sections = HashMap::new();
        let tera = setup_tera(&tmpl_dir, &config, &sections).unwrap();
        let ctx = tera::Context::new();
        let result = tera.render("test.html", &ctx).unwrap();
        assert_eq!(result, "https://example.com/img/photo.png");
    }
}
