use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{SortBy, default_toml_table};

/// Compute the URL path for a page given its parent directory and slug.
/// e.g. ("posts", "hello") -> "/posts/hello/"
///      ("", "hello") -> "/hello/"
pub(crate) fn page_url_path(parent_dir: &str, slug: &str) -> String {
    if parent_dir.is_empty() {
        format!("/{slug}/")
    } else {
        format!("/{parent_dir}/{slug}/")
    }
}

/// Compute the URL path for a section given the directory of its _index.md.
/// e.g. "posts" -> "/posts/"
///      "" -> "/"
pub(crate) fn section_url_path(dir: &str) -> String {
    if dir.is_empty() {
        "/".to_string()
    } else {
        format!("/{dir}/")
    }
}

/// Compute the parent directory string from a relative path.
/// e.g. "posts/hello.md" -> "posts"
///      "hello.md" -> ""
pub(crate) fn parent_dir(relative_path: &str) -> String {
    Path::new(relative_path)
        .parent()
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .to_string()
}

/// Compute the section key (_index.md path) for a given content relative path.
/// e.g. "posts/hello.md" -> "posts/_index.md"
///      "hello.md" -> "_index.md"
pub(crate) fn section_key_for(relative_path: &str) -> String {
    let dir = parent_dir(relative_path);
    if dir.is_empty() {
        "_index.md".to_string()
    } else {
        format!("{dir}/_index.md")
    }
}

/// TOML frontmatter parsed from +++ delimiters
#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub date: Option<toml::Value>,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub draft: bool,
    pub slug: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub sort_by: Option<SortBy>,
    pub paginate_by: Option<usize>,
    #[serde(default = "default_toml_table")]
    pub extra: toml::Value,
}

/// A rendered page (non-_index.md file)
#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub title: String,
    pub date: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub draft: bool,
    pub slug: String,
    pub path: String,
    pub permalink: String,
    pub content: String,
    pub summary: Option<String>,
    pub raw_content: String,
    pub taxonomies: HashMap<String, Vec<String>>,
    pub extra: serde_json::Value,
    pub aliases: Vec<String>,
    pub word_count: usize,
    pub reading_time: usize,
    pub relative_path: String,
}

/// A section (_index.md file)
#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub title: String,
    pub description: Option<String>,
    pub path: String,
    pub permalink: String,
    pub content: String,
    pub raw_content: String,
    pub pages: Vec<Page>,
    pub sort_by: Option<SortBy>,
    pub paginate_by: Option<usize>,
    pub extra: serde_json::Value,
    pub relative_path: String,
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            title: None,
            date: None,
            author: None,
            description: None,
            draft: false,
            slug: None,
            aliases: vec![],
            tags: vec![],
            sort_by: None,
            paginate_by: None,
            extra: default_toml_table(),
        }
    }
}

/// Parse TOML frontmatter from +++ delimiters
pub fn parse_frontmatter(content: &str) -> anyhow::Result<(Frontmatter, String)> {
    let content = content.trim_start_matches('\u{feff}'); // strip BOM
    if !content.starts_with("+++") {
        return Ok((Frontmatter::default(), content.to_string()));
    }

    let rest = &content[3..];
    let end = rest
        .find("\n+++")
        .ok_or_else(|| anyhow::anyhow!("Unclosed frontmatter"))?;
    let frontmatter_str = &rest[..end];
    let body = &rest[end + 4..]; // skip \n+++
    let body = body.strip_prefix('\n').unwrap_or(body);

    let fm: Frontmatter = toml::from_str(frontmatter_str)?;
    Ok((fm, body.to_string()))
}

/// Convert a TOML value (datetime or string) to a date string
fn value_to_date_string(v: &toml::Value) -> String {
    match v {
        toml::Value::Datetime(dt) => dt.to_string(),
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        _ => v.to_string(),
    }
}

/// Build a Page from a frontmatter + raw body
pub fn build_page(
    fm: Frontmatter,
    raw_content: String,
    relative_path: &str,
    base_url: &str,
) -> Page {
    let title = fm.title.unwrap_or_default();

    // Compute slug from frontmatter or filename
    let slug = fm.slug.unwrap_or_else(|| {
        let filename = Path::new(relative_path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        slug::slugify(&filename)
    });

    let path = page_url_path(&parent_dir(relative_path), &slug);
    let permalink = format!("{base_url}{path}");

    let date = fm.date.as_ref().map(value_to_date_string);

    // Build taxonomies
    let mut taxonomies = HashMap::new();
    if !fm.tags.is_empty() {
        taxonomies.insert("tags".to_string(), fm.tags);
    }

    let word_count = raw_content.split_whitespace().count();
    let reading_time = (word_count / 200).max(1);

    let extra = toml_to_json(&fm.extra);

    Page {
        title,
        date,
        author: fm.author,
        description: fm.description,
        draft: fm.draft,
        slug,
        path,
        permalink,
        content: String::new(), // filled during rendering
        summary: None,          // filled during rendering
        raw_content,
        taxonomies,
        extra,
        aliases: fm.aliases,
        word_count,
        reading_time,
        relative_path: relative_path.to_string(),
    }
}

/// Build a Section from a frontmatter + raw body
pub fn build_section(
    fm: Frontmatter,
    raw_content: String,
    relative_path: &str,
    base_url: &str,
) -> Section {
    let title = fm.title.unwrap_or_default();

    let path = section_url_path(&parent_dir(relative_path));
    let permalink = format!("{base_url}{path}");
    let extra = toml_to_json(&fm.extra);

    Section {
        title,
        description: fm.description,
        path,
        permalink,
        content: String::new(),
        raw_content,
        pages: vec![],
        sort_by: fm.sort_by,
        paginate_by: fm.paginate_by,
        extra,
        relative_path: relative_path.to_string(),
    }
}

/// Content loaded from disk
pub type LoadedContent = (
    HashMap<String, Section>,
    HashMap<String, Page>,
    Vec<PathBuf>,
);

/// Walk the content directory and return (sections, pages, assets)
pub fn load_content(content_dir: &Path, base_url: &str) -> anyhow::Result<LoadedContent> {
    let mut sections = HashMap::new();
    let mut pages = HashMap::new();
    let mut assets = Vec::new();

    for entry in WalkDir::new(content_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let relative = path
            .strip_prefix(content_dir)
            .unwrap()
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy();

        if filename == "_index.md" {
            let content = std::fs::read_to_string(path)?;
            let (fm, body) = parse_frontmatter(&content)?;
            let section = build_section(fm, body, &relative, base_url);
            sections.insert(relative, section);
        } else if filename.ends_with(".md") {
            let content = std::fs::read_to_string(path)?;
            let (fm, body) = parse_frontmatter(&content)?;
            let page = build_page(fm, body, &relative, base_url);
            pages.insert(relative, page);
        } else {
            // Static asset co-located with content
            assets.push(path.to_path_buf());
        }
    }

    Ok((sections, pages, assets))
}

/// Sort pages by date in reverse chronological order
pub fn sort_pages_by_date(pages: &mut [Page]) {
    pages.sort_by(|a, b| {
        let da = a.date.as_deref().unwrap_or("");
        let db = b.date.as_deref().unwrap_or("");
        db.cmp(da)
    });
}

/// Assign pages to their parent sections
pub fn assign_pages_to_sections(
    sections: &mut HashMap<String, Section>,
    pages: &HashMap<String, Page>,
) {
    for (rel_path, page) in pages {
        let key = section_key_for(rel_path);
        if let Some(section) = sections.get_mut(&key) {
            section.pages.push(page.clone());
        }
    }

    // Sort pages in each section
    for section in sections.values_mut() {
        match section.sort_by.unwrap_or_default() {
            SortBy::Date => sort_pages_by_date(&mut section.pages),
            SortBy::Title => section.pages.sort_by(|a, b| a.title.cmp(&b.title)),
        }
    }
}

/// Escape special characters for XML/HTML output
pub(crate) fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convert a `toml::Value` to `serde_json::Value`
pub(crate) fn toml_to_json(v: &toml::Value) -> serde_json::Value {
    match v {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(a) => serde_json::Value::Array(a.iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            let map: serde_json::Map<String, serde_json::Value> = t
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Frontmatter parsing ---

    #[test]
    fn test_parse_frontmatter_basic() {
        let input = "+++\ntitle = \"Hello\"\n+++\nBody text here";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("Hello"));
        assert_eq!(body, "Body text here");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let input = "Just plain markdown content";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert!(fm.title.is_none());
        assert!(!fm.draft);
        assert_eq!(body, "Just plain markdown content");
    }

    #[test]
    fn test_parse_frontmatter_all_fields() {
        let input = r#"+++
title = "Full Post"
date = "2025-01-15"
author = "Cody"
description = "A test post"
draft = true
slug = "custom-slug"
aliases = ["/old-url/"]
tags = ["rust", "test"]
sort_by = "date"
paginate_by = 5

[extra]
foo = "bar"
+++
Content goes here"#;
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("Full Post"));
        assert_eq!(fm.author.as_deref(), Some("Cody"));
        assert_eq!(fm.description.as_deref(), Some("A test post"));
        assert!(fm.draft);
        assert_eq!(fm.slug.as_deref(), Some("custom-slug"));
        assert_eq!(fm.aliases, vec!["/old-url/"]);
        assert_eq!(fm.tags, vec!["rust", "test"]);
        assert_eq!(fm.sort_by, Some(SortBy::Date));
        assert_eq!(fm.paginate_by, Some(5));
        assert_eq!(body, "Content goes here");
    }

    #[test]
    fn test_parse_frontmatter_date_datetime() {
        let input = "+++\ndate = 2025-06-15T10:30:00\n+++\n";
        let (fm, _) = parse_frontmatter(input).unwrap();
        let date_val = fm.date.unwrap();
        match date_val {
            toml::Value::Datetime(_) => {} // expected
            other => panic!("Expected Datetime, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_frontmatter_date_string() {
        let input = "+++\ndate = \"2025-06-15\"\n+++\n";
        let (fm, _) = parse_frontmatter(input).unwrap();
        let date_val = fm.date.unwrap();
        match date_val {
            toml::Value::String(s) => assert_eq!(s, "2025-06-15"),
            other => panic!("Expected String, got {other:?}"),
        }
    }

    // --- Page building ---

    #[test]
    fn test_build_page_slug_from_filename() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "body".into(), "hello-world.md", "https://example.com");
        assert_eq!(page.slug, "hello-world");
    }

    #[test]
    fn test_build_page_slug_from_frontmatter() {
        let fm = Frontmatter {
            slug: Some("custom".into()),
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "hello-world.md", "https://example.com");
        assert_eq!(page.slug, "custom");
    }

    #[test]
    fn test_build_page_path_nested() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "body".into(), "posts/hello.md", "https://example.com");
        assert_eq!(page.path, "/posts/hello/");
    }

    #[test]
    fn test_build_page_path_root() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "body".into(), "hello.md", "https://example.com");
        assert_eq!(page.path, "/hello/");
    }

    #[test]
    fn test_build_page_permalink() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "body".into(), "posts/hello.md", "https://example.com");
        assert_eq!(page.permalink, "https://example.com/posts/hello/");
    }

    #[test]
    fn test_build_page_word_count() {
        let fm = Frontmatter::default();
        let body = "one two three four five six seven eight nine ten";
        let page = build_page(fm, body.into(), "test.md", "https://example.com");
        assert_eq!(page.word_count, 10);
        assert_eq!(page.reading_time, 1); // 10/200 = 0, max(1) = 1
    }

    #[test]
    fn test_build_page_tags() {
        let fm = Frontmatter {
            tags: vec!["rust".into(), "test".into()],
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert_eq!(
            page.taxonomies.get("tags").unwrap(),
            &vec!["rust".to_string(), "test".to_string()]
        );
    }

    // --- Section building ---

    #[test]
    fn test_build_section_root() {
        let fm = Frontmatter {
            title: Some("Home".into()),
            ..Default::default()
        };
        let section = build_section(fm, "body".into(), "_index.md", "https://example.com");
        assert_eq!(section.path, "/");
        assert_eq!(section.permalink, "https://example.com/");
        assert_eq!(section.title, "Home");
    }

    #[test]
    fn test_build_section_nested() {
        let fm = Frontmatter {
            title: Some("Blog".into()),
            ..Default::default()
        };
        let section = build_section(fm, "body".into(), "posts/_index.md", "https://example.com");
        assert_eq!(section.path, "/posts/");
        assert_eq!(section.permalink, "https://example.com/posts/");
    }

    // --- toml_to_json ---

    #[test]
    fn test_toml_to_json_primitives() {
        assert_eq!(
            toml_to_json(&toml::Value::String("hello".into())),
            serde_json::json!("hello")
        );
        assert_eq!(
            toml_to_json(&toml::Value::Integer(42)),
            serde_json::json!(42)
        );
        assert_eq!(
            toml_to_json(&toml::Value::Boolean(true)),
            serde_json::json!(true)
        );
        assert_eq!(
            toml_to_json(&toml::Value::Float(3.14)),
            serde_json::json!(3.14)
        );
    }

    #[test]
    fn test_toml_to_json_nested() {
        let mut table = toml::map::Map::new();
        table.insert("key".into(), toml::Value::String("value".into()));
        table.insert(
            "nums".into(),
            toml::Value::Array(vec![toml::Value::Integer(1), toml::Value::Integer(2)]),
        );
        let result = toml_to_json(&toml::Value::Table(table));
        assert_eq!(result["key"], serde_json::json!("value"));
        assert_eq!(result["nums"], serde_json::json!([1, 2]));
    }
}
