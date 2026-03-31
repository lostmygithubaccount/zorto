use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::config::{ContentDirConfig, SortBy, default_toml_table};

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
///      "posts/vibe-coding/index.md" -> "posts/_index.md" (co-located content)
pub(crate) fn section_key_for(relative_path: &str) -> String {
    let p = Path::new(relative_path);
    // Co-located content: "dir/index.md" belongs to the grandparent section
    let is_colocated = p.file_name().is_some_and(|f| f == "index.md");
    let dir = if is_colocated {
        // Go up two levels: posts/vibe-coding/index.md -> posts
        p.parent()
            .and_then(|d| d.parent())
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .to_string()
    } else {
        parent_dir(relative_path)
    };
    if dir.is_empty() {
        "_index.md".to_string()
    } else {
        format!("{dir}/_index.md")
    }
}

/// TOML frontmatter parsed from `+++` delimiters.
///
/// Unknown top-level keys (e.g. `tags`, `categories`) are captured in [`rest`](Self::rest)
/// and interpreted as taxonomy values if they are arrays of strings.
#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub date: Option<toml::Value>,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub draft: bool,
    pub slug: Option<String>,
    /// Custom template name (e.g. `"dev.html"`). Defaults to `page.html` for pages.
    pub template: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub sort_by: Option<SortBy>,
    pub paginate_by: Option<usize>,
    #[serde(default = "default_toml_table")]
    pub extra: toml::Value,
    /// Catch-all for unknown top-level keys (taxonomy values like tags, categories, etc.)
    #[serde(flatten)]
    pub rest: HashMap<String, toml::Value>,
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
            template: None,
            aliases: Vec::new(),
            sort_by: None,
            paginate_by: None,
            extra: default_toml_table(),
            rest: HashMap::new(),
        }
    }
}

/// A rendered page (any `.md` file that is not `_index.md`).
#[derive(Debug, Clone, Serialize)]
pub struct Page {
    /// Page title from frontmatter (empty string if unset).
    pub title: String,
    /// Publication date as a string (e.g. `"2025-01-15"`).
    pub date: Option<String>,
    /// Author name from frontmatter.
    pub author: Option<String>,
    /// Short description from frontmatter.
    pub description: Option<String>,
    /// Whether this page is a draft (excluded from non-draft builds).
    pub draft: bool,
    /// URL slug, derived from frontmatter `slug` field or the filename.
    pub slug: String,
    /// Custom template name (e.g. `"dev.html"`). Defaults to `"page.html"`.
    pub template: Option<String>,
    /// URL path relative to the site root (e.g. `"/posts/hello/"`).
    pub path: String,
    /// Full permalink including base URL.
    pub permalink: String,
    /// Rendered HTML content (populated during build).
    pub content: String,
    /// Rendered HTML summary (content before `<!-- more -->` marker).
    pub summary: Option<String>,
    /// Raw markdown content (after frontmatter extraction).
    pub raw_content: String,
    /// Taxonomy values keyed by taxonomy name (e.g. `{"tags": ["rust", "web"]}`).
    pub taxonomies: HashMap<String, Vec<String>>,
    /// Extra frontmatter values as JSON, accessible in templates as `page.extra`.
    pub extra: serde_json::Value,
    /// Redirect aliases — additional URL paths that redirect to this page.
    pub aliases: Vec<String>,
    /// Approximate word count of the raw content.
    pub word_count: usize,
    /// Estimated reading time in minutes (word_count / 200, minimum 1).
    pub reading_time: usize,
    /// Path of the source file relative to the content directory.
    pub relative_path: String,
}

/// A section defined by an `_index.md` file.
#[derive(Debug, Clone, Serialize)]
pub struct Section {
    /// Section title from frontmatter.
    pub title: String,
    /// Short description from frontmatter.
    pub description: Option<String>,
    /// URL path relative to the site root (e.g. `"/posts/"`).
    pub path: String,
    /// Full permalink including base URL.
    pub permalink: String,
    /// Rendered HTML content of the section's `_index.md` body.
    pub content: String,
    /// Raw markdown content (after frontmatter extraction).
    pub raw_content: String,
    /// Pages belonging to this section (populated by [`assign_pages_to_sections`]).
    pub pages: Vec<Page>,
    /// Sort order for pages in this section.
    pub sort_by: Option<SortBy>,
    /// If set, paginate the section with this many pages per page.
    pub paginate_by: Option<usize>,
    /// Custom template name. Defaults to `"section.html"`.
    pub template: Option<String>,
    /// Extra frontmatter values as JSON, accessible in templates as `section.extra`.
    pub extra: serde_json::Value,
    /// Path of the source `_index.md` relative to the content directory.
    pub relative_path: String,
}

/// Parse TOML frontmatter from `+++` delimiters.
///
/// Returns the parsed frontmatter and the remaining body text. If the content
/// does not start with `+++`, returns a default frontmatter and the full content.
///
/// # Errors
///
/// Returns an error if the frontmatter is unclosed or contains invalid TOML.
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

/// Build a [`Page`] from parsed frontmatter, raw body text, and site context.
pub fn build_page(
    fm: Frontmatter,
    raw_content: String,
    relative_path: &str,
    base_url: &str,
) -> Page {
    let title = fm.title.unwrap_or_default();

    // Co-located content: "dir/index.md" derives slug from the directory name
    let p = Path::new(relative_path);
    let is_colocated = p.file_name().is_some_and(|f| f == "index.md");

    let slug = fm.slug.unwrap_or_else(|| {
        if is_colocated {
            // posts/my-post/index.md -> slug "my-post"
            let dir_name = p
                .parent()
                .and_then(|d| d.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            slug::slugify(&dir_name)
        } else {
            let filename = p
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            slug::slugify(&filename)
        }
    });

    // Co-located pages use grandparent as the section directory
    let parent = if is_colocated {
        p.parent()
            .and_then(|d| d.parent())
            .unwrap_or(Path::new(""))
            .to_string_lossy()
            .to_string()
    } else {
        parent_dir(relative_path)
    };
    let path = page_url_path(&parent, &slug);
    let permalink = format!("{base_url}{path}");

    let date = fm.date.as_ref().map(value_to_date_string);

    // Build taxonomies from any top-level array-of-strings fields
    let mut taxonomies = HashMap::new();
    for (key, value) in &fm.rest {
        if let toml::Value::Array(arr) = value {
            let strings: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !strings.is_empty() {
                taxonomies.insert(key.clone(), strings);
            }
        }
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
        template: fm.template,
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

/// Build a [`Section`] from parsed frontmatter, raw body text, and site context.
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
        template: fm.template,
        extra,
        relative_path: relative_path.to_string(),
    }
}

/// Content loaded from disk: sections, pages, and co-located asset paths.
pub struct LoadedContent {
    /// Sections keyed by their relative `_index.md` path (e.g. `"posts/_index.md"`).
    pub sections: HashMap<String, Section>,
    /// Pages keyed by their relative `.md` path (e.g. `"posts/hello.md"`).
    pub pages: HashMap<String, Page>,
    /// Absolute paths to non-markdown files co-located with content.
    pub assets: Vec<PathBuf>,
}

/// Walk the content directory and return all sections, pages, and co-located assets.
///
/// # Errors
///
/// Returns an error if the content directory cannot be walked or any markdown
/// file has invalid frontmatter.
pub fn load_content(content_dir: &Path, base_url: &str) -> anyhow::Result<LoadedContent> {
    let mut sections = HashMap::new();
    let mut pages = HashMap::new();
    let mut assets = Vec::new();

    for entry in WalkDir::new(content_dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("failed to walk content directory: {e}"))?
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(content_dir)
            .expect("walkdir entry is under content_dir")
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            continue;
        }

        let filename = path
            .file_name()
            .expect("non-directory entry has a filename")
            .to_string_lossy();

        if filename == "_index.md" {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
            let (fm, body) = parse_frontmatter(&content)?;
            let section = build_section(fm, body, &relative, base_url);
            sections.insert(relative, section);
        } else if filename.ends_with(".md") {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
            let (fm, body) = parse_frontmatter(&content)?;
            let page = build_page(fm, body, &relative, base_url);
            pages.insert(relative, page);
        } else {
            // Static asset co-located with content
            assets.push(path.to_path_buf());
        }
    }

    Ok(LoadedContent {
        sections,
        pages,
        assets,
    })
}

/// Load an external directory of plain markdown files as content pages and sections.
///
/// - `README.md` files become sections (like `_index.md`)
/// - Other `.md` files become pages
/// - Title is extracted from the first `# Heading`
/// - Description is extracted from the first paragraph after the heading
/// - Files listed in `config.exclude` are skipped
pub fn load_content_dir(
    dir: &Path,
    config: &ContentDirConfig,
    base_url: &str,
) -> anyhow::Result<LoadedContent> {
    let mut sections = HashMap::new();
    let mut pages = HashMap::new();

    if !dir.exists() {
        return Ok(LoadedContent {
            sections,
            pages,
            assets: vec![],
        });
    }

    for entry in WalkDir::new(dir)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("failed to walk content dir {}: {e}", dir.display()))?
    {
        let path = entry.path();
        if path.is_dir() {
            continue;
        }

        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if !filename.ends_with(".md") {
            continue;
        }

        // Relative path within the external dir (e.g. "getting-started/installation.md")
        let rel_in_dir = path
            .strip_prefix(dir)
            .expect("walkdir entry is under dir")
            .to_string_lossy()
            .to_string();

        // Check exclude list
        if config.exclude.contains(&rel_in_dir) {
            continue;
        }

        let raw = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;

        let is_readme = filename == "README.md";
        let stem = Path::new(&rel_in_dir)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (extracted_title, description) = extract_title_description(&raw);
        // Use extracted H1 title, or derive from filename for non-README files
        let title = extracted_title.unwrap_or_else(|| title_from_filename(&stem));

        // Strip the title heading from body content so it doesn't render twice
        let body = strip_title_heading(&raw);

        // Optionally rewrite .md links to clean URLs
        let body = if config.rewrite_links {
            let include_path = format!("../{}/{}", config.path, rel_in_dir);
            crate::shortcodes::rewrite_md_links(&body, &include_path)
        } else {
            body
        };

        if is_readme {
            // README.md → section
            let rel_path = if config.url_prefix.is_empty() {
                "_index.md".to_string()
            } else {
                let parent = Path::new(&rel_in_dir)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .to_string_lossy();
                if parent.is_empty() {
                    format!("{}/_index.md", config.url_prefix)
                } else {
                    format!("{}/{parent}/_index.md", config.url_prefix)
                }
            };

            let fm = Frontmatter {
                title: Some(title),
                description,
                template: Some(config.section_template.clone()),
                sort_by: config.sort_by,
                ..Default::default()
            };
            let section = build_section(fm, body, &rel_path, base_url);
            sections.insert(rel_path, section);
        } else {
            // Regular .md → page
            let parent = Path::new(&rel_in_dir)
                .parent()
                .unwrap_or(Path::new(""))
                .to_string_lossy();
            let rel_path = if parent.is_empty() {
                format!("{}/{stem}.md", config.url_prefix)
            } else {
                format!("{}/{parent}/{stem}.md", config.url_prefix)
            };

            let fm = Frontmatter {
                title: Some(title),
                description,
                template: Some(config.template.clone()),
                ..Default::default()
            };
            let page = build_page(fm, body, &rel_path, base_url);
            pages.insert(rel_path, page);
        }
    }

    Ok(LoadedContent {
        sections,
        pages,
        assets: vec![],
    })
}

/// Extract title and description from plain markdown (no frontmatter).
///
/// Title: first `# Heading` line, or `None` if absent.
/// Description: first non-empty paragraph of prose (stops at headings, lists, code fences).
/// When no H1 exists, the description is extracted from the start of the content.
pub fn extract_title_description(content: &str) -> (Option<String>, Option<String>) {
    let mut title = None;
    let mut desc_lines = Vec::new();
    let mut found_title = false;
    let mut in_desc = false;

    for line in content.lines() {
        if !found_title {
            if let Some(h1) = line.strip_prefix("# ") {
                title = Some(h1.trim().to_string());
                found_title = true;
                continue;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_desc {
                break;
            }
            continue;
        }

        if trimmed.starts_with('#')
            || trimmed.starts_with('-')
            || trimmed.starts_with("```")
            || trimmed.starts_with('|')
            || trimmed.starts_with('<')
        {
            if !in_desc {
                continue;
            }
            break;
        }

        in_desc = true;
        desc_lines.push(trimmed);
    }

    let desc = if desc_lines.is_empty() {
        None
    } else {
        Some(desc_lines.join(" "))
    };
    (title, desc)
}

/// Derive a human-readable title from a filename stem.
///
/// e.g. `"add-blog"` → `"Add blog"`, `"content-model"` → `"Content model"`
fn title_from_filename(stem: &str) -> String {
    let mut title = stem.replace('-', " ");
    if let Some(first) = title.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    title
}

/// Strip the first `# Heading` line from content so it doesn't render twice
/// (the title is already shown by the template).
fn strip_title_heading(content: &str) -> String {
    let mut lines = content.lines();
    let mut result = Vec::new();
    let mut found = false;

    for line in &mut lines {
        if !found && line.starts_with("# ") {
            found = true;
            continue;
        }
        result.push(line);
    }

    // Trim leading blank lines after stripping the title
    let start = result
        .iter()
        .position(|l| !l.trim().is_empty())
        .unwrap_or(0);
    result[start..].join("\n")
}

/// Sort key: extract date string for reverse chronological ordering (undated sort last).
fn page_date_key(p: &Page) -> &str {
    p.date.as_deref().unwrap_or("")
}

/// Sort pages by date in reverse chronological order. Pages without dates sort last.
pub fn sort_pages_by_date(pages: &mut [Page]) {
    pages.sort_by(|a, b| page_date_key(b).cmp(page_date_key(a)));
}

/// Sort page references by date (reverse chronological). Pages without dates sort last.
pub fn sort_pages_by_date_ref(pages: &mut [&Page]) {
    pages.sort_by(|a, b| page_date_key(b).cmp(page_date_key(a)));
}

/// Assign pages to their parent sections and sort each section's pages.
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

/// Escape special characters for XML/HTML output.
///
/// Escapes `&`, `<`, `>`, `"`, and `'`. Safe for use in element content,
/// attribute values, and XML (Atom, sitemap) output.
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
        // tags are now captured in rest as a generic taxonomy
        let tags = fm.rest.get("tags").unwrap();
        assert_eq!(
            tags,
            &toml::Value::Array(vec![
                toml::Value::String("rust".into()),
                toml::Value::String("test".into()),
            ])
        );
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
    fn test_build_page_colocated_index() {
        let fm = Frontmatter::default();
        let page = build_page(
            fm,
            "body".into(),
            "posts/my-post/index.md",
            "https://example.com",
        );
        assert_eq!(page.slug, "my-post");
        assert_eq!(page.path, "/posts/my-post/");
        assert_eq!(page.permalink, "https://example.com/posts/my-post/");
    }

    #[test]
    fn test_build_page_colocated_with_custom_slug() {
        let fm = Frontmatter {
            slug: Some("custom".into()),
            ..Default::default()
        };
        let page = build_page(
            fm,
            "body".into(),
            "posts/my-post/index.md",
            "https://example.com",
        );
        assert_eq!(page.slug, "custom");
        assert_eq!(page.path, "/posts/custom/");
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
        let mut rest = HashMap::new();
        rest.insert(
            "tags".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("rust".into()),
                toml::Value::String("test".into()),
            ]),
        );
        let fm = Frontmatter {
            rest,
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert_eq!(
            page.taxonomies.get("tags").unwrap(),
            &vec!["rust".to_string(), "test".to_string()]
        );
    }

    #[test]
    fn test_build_page_custom_taxonomy() {
        let mut rest = HashMap::new();
        rest.insert(
            "categories".to_string(),
            toml::Value::Array(vec![toml::Value::String("tutorial".into())]),
        );
        let fm = Frontmatter {
            rest,
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert_eq!(
            page.taxonomies.get("categories").unwrap(),
            &vec!["tutorial".to_string()]
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
            toml_to_json(&toml::Value::Float(1.23)),
            serde_json::json!(1.23)
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
