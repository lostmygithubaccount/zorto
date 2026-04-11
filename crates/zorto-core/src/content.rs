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
    /// Sort weight for ordering within a section (lower = first).
    pub weight: Option<i64>,
    /// Whether child pages should be rendered as individual HTML files (default: `true`).
    /// When `false`, pages are still rendered to HTML but only available via `section.pages`
    /// in templates — useful for presentations where slides are assembled into one output.
    #[serde(default = "crate::config::default_true")]
    pub render_pages: bool,
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
            weight: None,
            render_pages: true,
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
    /// Sort weight for ordering within a section (lower values sort first).
    pub weight: Option<i64>,
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
    /// Whether child pages should be rendered as individual HTML files.
    /// When `false`, pages are available via `section.pages` in templates but
    /// do not produce standalone HTML output.
    pub render_pages: bool,
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
    let end = rest.find("\n+++").ok_or_else(|| {
        anyhow::anyhow!("unclosed TOML frontmatter: missing closing '+++' delimiter")
    })?;
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

    const WORDS_PER_MINUTE: usize = 200;
    let word_count = raw_content.split_whitespace().count();
    let reading_time = (word_count / WORDS_PER_MINUTE).max(1);

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
        weight: fm.weight,
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
        render_pages: fm.render_pages,
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
            .map_err(|_| {
                anyhow::anyhow!(
                    "content entry {} is outside content directory {}",
                    path.display(),
                    content_dir.display()
                )
            })?
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            continue;
        }

        let filename = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("content entry has no filename: {}", path.display()))?
            .to_string_lossy();

        if filename == "_index.md" {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
            let (fm, body) = parse_frontmatter(&content)
                .map_err(|e| e.context(format!("in {}", path.display())))?;
            let section = build_section(fm, body, &relative, base_url);
            sections.insert(relative, section);
        } else if filename.ends_with(".md") {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
            let (fm, body) = parse_frontmatter(&content)
                .map_err(|e| e.context(format!("in {}", path.display())))?;
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
            .map_err(|_| {
                anyhow::anyhow!(
                    "content entry {} is outside directory {}",
                    path.display(),
                    dir.display()
                )
            })?
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
        Some(strip_inline_markdown(&desc_lines.join(" ")))
    };
    (title, desc)
}

/// Strip common inline markdown syntax so descriptions read as plain text.
///
/// Converts `[text](url)` → `text`, `**bold**` → `bold`, `` `code` `` → `code`, etc.
fn strip_inline_markdown(s: &str) -> String {
    // [text](url) → text
    let mut result = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '[' {
            // Look for ](
            if let Some(close) = s[i + ch.len_utf8()..].find("](") {
                let text_start = i + ch.len_utf8();
                let text = &s[text_start..text_start + close];
                let after_paren = text_start + close + 2;
                if let Some(end_paren) = s[after_paren..].find(')') {
                    result.push_str(text);
                    // Advance past the closing ')'
                    let skip_to = after_paren + end_paren + 1;
                    while chars.peek().is_some_and(|(idx, _)| *idx < skip_to) {
                        chars.next();
                    }
                    continue;
                }
            }
        }
        result.push(ch);
    }
    // **bold** / __bold__ → bold, *italic* / _italic_ → italic
    let result = result
        .replace("**", "")
        .replace("__", "")
        .replace("*", "")
        .replace("~~", "");
    // `code` → code
    result.replace('`', "")
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
            SortBy::Weight => section.pages.sort_by(|a, b| {
                let wa = a.weight.unwrap_or(i64::MAX);
                let wb = b.weight.unwrap_or(i64::MAX);
                wa.cmp(&wb)
                    .then_with(|| a.relative_path.cmp(&b.relative_path))
            }),
        }
    }
}

/// Escape special characters for HTML/XML output.
///
/// Escapes `&`, `<`, `>`, `"`, and `'`. Safe for use in element content,
/// attribute values, and XML (Atom, sitemap) output.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Alias for [`escape_html`] — used in XML contexts (Atom, sitemap) where
/// the escaping requirements are identical.
pub(crate) fn escape_xml(s: &str) -> String {
    escape_html(s)
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
        assert!(
            matches!(date_val, toml::Value::Datetime(_)),
            "Expected Datetime, got {date_val:?}"
        );
    }

    #[test]
    fn test_parse_frontmatter_date_string() {
        let input = "+++\ndate = \"2025-06-15\"\n+++\n";
        let (fm, _) = parse_frontmatter(input).unwrap();
        let date_val = fm.date.unwrap();
        assert!(
            matches!(&date_val, toml::Value::String(s) if s == "2025-06-15"),
            "Expected String(\"2025-06-15\"), got {date_val:?}"
        );
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

    // --- Frontmatter security tests ---

    #[test]
    fn test_parse_frontmatter_special_chars_in_values() {
        let input = "+++\ntitle = \"Hello <script>alert('xss')</script>\"\n+++\nBody";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(
            fm.title.as_deref(),
            Some("Hello <script>alert('xss')</script>")
        );
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_parse_frontmatter_injection_in_keys() {
        // TOML keys with special characters should either parse correctly or error
        let input = "+++\n\"key with spaces\" = \"value\"\n+++\nBody";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(body, "Body");
        assert!(fm.rest.contains_key("key with spaces"));
    }

    #[test]
    fn test_parse_frontmatter_unclosed() {
        let input = "+++\ntitle = \"Oops\"\nNo closing delimiter";
        let result = parse_frontmatter(input);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unclosed TOML frontmatter")
        );
    }

    #[test]
    fn test_parse_frontmatter_bom_handling() {
        let input = "\u{feff}+++\ntitle = \"BOM\"\n+++\nBody";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("BOM"));
        assert_eq!(body, "Body");
    }

    #[test]
    fn test_parse_frontmatter_nested_delimiters() {
        // Content containing +++ after the frontmatter should not confuse parser
        let input = "+++\ntitle = \"Test\"\n+++\nBody with +++ in it";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("Test"));
        assert_eq!(body, "Body with +++ in it");
    }

    #[test]
    fn test_parse_frontmatter_multiline_string() {
        let input = "+++\ntitle = \"Line1\\nLine2\"\n+++\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        // TOML parses \n as an actual newline character
        assert_eq!(fm.title.as_deref(), Some("Line1\nLine2"));
    }

    // --- strip_inline_markdown security tests ---

    #[test]
    fn test_strip_inline_markdown_basic_link() {
        assert_eq!(strip_inline_markdown("[text](https://example.com)"), "text");
    }

    #[test]
    fn test_strip_inline_markdown_bold_italic() {
        assert_eq!(
            strip_inline_markdown("**bold** and *italic*"),
            "bold and italic"
        );
    }

    #[test]
    fn test_strip_inline_markdown_code() {
        assert_eq!(strip_inline_markdown("use `code` here"), "use code here");
    }

    #[test]
    fn test_strip_inline_markdown_non_ascii() {
        assert_eq!(
            strip_inline_markdown("café résumé naïve"),
            "café résumé naïve"
        );
    }

    #[test]
    fn test_strip_inline_markdown_emoji() {
        assert_eq!(
            strip_inline_markdown("Hello 🌍 World 🎉"),
            "Hello 🌍 World 🎉"
        );
    }

    #[test]
    fn test_strip_inline_markdown_cjk() {
        assert_eq!(strip_inline_markdown("日本語テスト"), "日本語テスト");
    }

    #[test]
    fn test_strip_inline_markdown_cjk_with_formatting() {
        assert_eq!(
            strip_inline_markdown("**日本語**と[リンク](https://example.jp)"),
            "日本語とリンク"
        );
    }

    #[test]
    fn test_strip_inline_markdown_emoji_in_link() {
        assert_eq!(
            strip_inline_markdown("[🔗 link text](https://example.com)"),
            "🔗 link text"
        );
    }

    #[test]
    fn test_strip_inline_markdown_nested_brackets() {
        // Edge case: brackets inside link text
        assert_eq!(strip_inline_markdown("[a [b] c](url)"), "a [b] c");
    }

    #[test]
    fn test_strip_inline_markdown_strikethrough() {
        assert_eq!(strip_inline_markdown("~~deleted~~ text"), "deleted text");
    }

    #[test]
    fn test_strip_inline_markdown_empty_string() {
        assert_eq!(strip_inline_markdown(""), "");
    }

    #[test]
    fn test_strip_inline_markdown_plain_text() {
        assert_eq!(strip_inline_markdown("just plain text"), "just plain text");
    }

    // --- Additional frontmatter variant tests ---

    #[test]
    fn test_parse_frontmatter_empty_frontmatter() {
        let input = "+++\n+++\nBody only";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert!(fm.title.is_none());
        assert!(fm.date.is_none());
        assert!(!fm.draft);
        assert_eq!(body, "Body only");
    }

    #[test]
    fn test_parse_frontmatter_extra_fields_ignored() {
        let input = "+++\ntitle = \"Hello\"\nunknown_field = 42\nanother = true\n+++\nBody";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("Hello"));
        assert_eq!(body, "Body");
        // Extra fields are captured in `rest`
        assert!(fm.rest.contains_key("unknown_field"));
        assert!(fm.rest.contains_key("another"));
    }

    #[test]
    fn test_parse_frontmatter_missing_optional_fields() {
        let input = "+++\ntitle = \"Only title\"\n+++\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("Only title"));
        assert!(fm.date.is_none());
        assert!(fm.author.is_none());
        assert!(fm.description.is_none());
        assert!(!fm.draft);
        assert!(fm.slug.is_none());
        assert!(fm.template.is_none());
        assert!(fm.aliases.is_empty());
        assert!(fm.sort_by.is_none());
        assert!(fm.paginate_by.is_none());
    }

    #[test]
    fn test_parse_frontmatter_invalid_toml() {
        let input = "+++\ntitle = \n+++\nBody";
        let result = parse_frontmatter(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_frontmatter_empty_body() {
        let input = "+++\ntitle = \"No body\"\n+++\n";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.title.as_deref(), Some("No body"));
        assert_eq!(body, "");
    }

    #[test]
    fn test_parse_frontmatter_multiline_body() {
        let input = "+++\ntitle = \"T\"\n+++\nLine 1\nLine 2\nLine 3";
        let (_, body) = parse_frontmatter(input).unwrap();
        assert_eq!(body, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_parse_frontmatter_extra_table() {
        let input = "+++\ntitle = \"T\"\n\n[extra]\ncolor = \"blue\"\ncount = 5\n+++\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        let extra = toml_to_json(&fm.extra);
        assert_eq!(extra["color"], serde_json::json!("blue"));
        assert_eq!(extra["count"], serde_json::json!(5));
    }

    #[test]
    fn test_parse_frontmatter_multiple_taxonomies() {
        let input = "+++\ntitle = \"T\"\ntags = [\"a\", \"b\"]\ncategories = [\"c\"]\n+++\nBody";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert!(fm.rest.contains_key("tags"));
        assert!(fm.rest.contains_key("categories"));
    }

    #[test]
    fn test_parse_frontmatter_sort_by_title() {
        let input = "+++\nsort_by = \"title\"\n+++\n";
        let (fm, _) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.sort_by, Some(SortBy::Title));
    }

    // --- Section hierarchy tests ---

    #[test]
    fn test_section_key_for_nested() {
        assert_eq!(section_key_for("a/b/c.md"), "a/b/_index.md");
    }

    #[test]
    fn test_section_key_for_root() {
        assert_eq!(section_key_for("hello.md"), "_index.md");
    }

    #[test]
    fn test_section_key_for_colocated() {
        assert_eq!(section_key_for("posts/my-post/index.md"), "posts/_index.md");
    }

    #[test]
    fn test_section_key_for_deeply_nested_colocated() {
        assert_eq!(
            section_key_for("blog/2025/my-post/index.md"),
            "blog/2025/_index.md"
        );
    }

    #[test]
    fn test_build_section_deeply_nested() {
        let fm = Frontmatter {
            title: Some("Deep".into()),
            ..Default::default()
        };
        let section = build_section(fm, "body".into(), "a/b/c/_index.md", "https://example.com");
        assert_eq!(section.path, "/a/b/c/");
        assert_eq!(section.permalink, "https://example.com/a/b/c/");
    }

    #[test]
    fn test_build_section_empty_content() {
        let fm = Frontmatter {
            title: Some("Empty".into()),
            ..Default::default()
        };
        let section = build_section(fm, "".into(), "empty/_index.md", "https://example.com");
        assert_eq!(section.raw_content, "");
        assert!(section.pages.is_empty());
    }

    #[test]
    fn test_build_section_with_sort_by() {
        let fm = Frontmatter {
            title: Some("Sorted".into()),
            sort_by: Some(SortBy::Title),
            ..Default::default()
        };
        let section = build_section(fm, "body".into(), "sorted/_index.md", "https://example.com");
        assert_eq!(section.sort_by, Some(SortBy::Title));
    }

    #[test]
    fn test_build_section_with_paginate_by() {
        let fm = Frontmatter {
            paginate_by: Some(10),
            ..Default::default()
        };
        let section = build_section(fm, "".into(), "paged/_index.md", "https://example.com");
        assert_eq!(section.paginate_by, Some(10));
    }

    #[test]
    fn test_build_section_with_custom_template() {
        let fm = Frontmatter {
            template: Some("custom.html".into()),
            ..Default::default()
        };
        let section = build_section(fm, "".into(), "custom/_index.md", "https://example.com");
        assert_eq!(section.template.as_deref(), Some("custom.html"));
    }

    // --- assign_pages_to_sections tests ---

    #[test]
    fn test_assign_pages_to_sections_basic() {
        let mut sections = HashMap::new();
        let fm = Frontmatter {
            title: Some("Blog".into()),
            ..Default::default()
        };
        sections.insert(
            "posts/_index.md".to_string(),
            build_section(fm, "".into(), "posts/_index.md", "https://example.com"),
        );

        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("Post A".into()),
            date: Some(toml::Value::String("2025-01-01".into())),
            ..Default::default()
        };
        pages.insert(
            "posts/a.md".to_string(),
            build_page(fm, "body a".into(), "posts/a.md", "https://example.com"),
        );
        let fm = Frontmatter {
            title: Some("Post B".into()),
            date: Some(toml::Value::String("2025-02-01".into())),
            ..Default::default()
        };
        pages.insert(
            "posts/b.md".to_string(),
            build_page(fm, "body b".into(), "posts/b.md", "https://example.com"),
        );

        assign_pages_to_sections(&mut sections, &pages);
        let section = sections.get("posts/_index.md").unwrap();
        assert_eq!(section.pages.len(), 2);
        // Default sort is by date descending
        assert_eq!(section.pages[0].title, "Post B"); // newer
        assert_eq!(section.pages[1].title, "Post A"); // older
    }

    #[test]
    fn test_assign_pages_to_sections_sort_by_title() {
        let mut sections = HashMap::new();
        let fm = Frontmatter {
            title: Some("Docs".into()),
            sort_by: Some(SortBy::Title),
            ..Default::default()
        };
        sections.insert(
            "docs/_index.md".to_string(),
            build_section(fm, "".into(), "docs/_index.md", "https://example.com"),
        );

        let mut pages = HashMap::new();
        for name in ["Zeta", "Alpha", "Mid"] {
            let fm = Frontmatter {
                title: Some(name.into()),
                ..Default::default()
            };
            let slug = name.to_lowercase();
            pages.insert(
                format!("docs/{slug}.md"),
                build_page(
                    fm,
                    "body".into(),
                    &format!("docs/{slug}.md"),
                    "https://example.com",
                ),
            );
        }

        assign_pages_to_sections(&mut sections, &pages);
        let section = sections.get("docs/_index.md").unwrap();
        assert_eq!(section.pages.len(), 3);
        assert_eq!(section.pages[0].title, "Alpha");
        assert_eq!(section.pages[1].title, "Mid");
        assert_eq!(section.pages[2].title, "Zeta");
    }

    #[test]
    fn test_assign_pages_orphan_page_no_section() {
        // Pages without a matching section just don't get assigned
        let mut sections = HashMap::new();
        let fm = Frontmatter::default();
        sections.insert(
            "_index.md".to_string(),
            build_section(fm, "".into(), "_index.md", "https://example.com"),
        );

        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("Orphan".into()),
            ..Default::default()
        };
        pages.insert(
            "nonexistent-section/orphan.md".to_string(),
            build_page(
                fm,
                "body".into(),
                "nonexistent-section/orphan.md",
                "https://example.com",
            ),
        );

        assign_pages_to_sections(&mut sections, &pages);
        // Root section should have no pages (orphan's section doesn't exist)
        let root = sections.get("_index.md").unwrap();
        assert!(root.pages.is_empty());
    }

    // --- Taxonomy tests ---

    #[test]
    fn test_build_page_multiple_taxonomies() {
        let mut rest = HashMap::new();
        rest.insert(
            "tags".to_string(),
            toml::Value::Array(vec![toml::Value::String("rust".into())]),
        );
        rest.insert(
            "categories".to_string(),
            toml::Value::Array(vec![toml::Value::String("tutorial".into())]),
        );
        rest.insert(
            "series".to_string(),
            toml::Value::Array(vec![
                toml::Value::String("part1".into()),
                toml::Value::String("part2".into()),
            ]),
        );
        let fm = Frontmatter {
            rest,
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert_eq!(page.taxonomies.len(), 3);
        assert_eq!(page.taxonomies["tags"], vec!["rust"]);
        assert_eq!(page.taxonomies["categories"], vec!["tutorial"]);
        assert_eq!(page.taxonomies["series"], vec!["part1", "part2"]);
    }

    #[test]
    fn test_build_page_empty_taxonomy_array() {
        let mut rest = HashMap::new();
        rest.insert("tags".to_string(), toml::Value::Array(vec![]));
        let fm = Frontmatter {
            rest,
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        // Empty arrays are not included in taxonomies
        assert!(!page.taxonomies.contains_key("tags"));
    }

    #[test]
    fn test_build_page_non_array_rest_field_ignored() {
        // Non-array rest values should not become taxonomies
        let mut rest = HashMap::new();
        rest.insert(
            "custom_string".to_string(),
            toml::Value::String("value".into()),
        );
        rest.insert("custom_int".to_string(), toml::Value::Integer(42));
        let fm = Frontmatter {
            rest,
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert!(page.taxonomies.is_empty());
    }

    // --- Content loading from disk tests ---

    #[test]
    fn test_load_content_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content_dir = tmp.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();
        let loaded = load_content(&content_dir, "https://example.com").unwrap();
        assert!(loaded.sections.is_empty());
        assert!(loaded.pages.is_empty());
        assert!(loaded.assets.is_empty());
    }

    #[test]
    fn test_load_content_with_sections_and_pages() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content_dir = tmp.path().join("content");
        let posts = content_dir.join("posts");
        std::fs::create_dir_all(&posts).unwrap();

        std::fs::write(
            content_dir.join("_index.md"),
            "+++\ntitle = \"Home\"\n+++\nWelcome",
        )
        .unwrap();
        std::fs::write(posts.join("_index.md"), "+++\ntitle = \"Blog\"\n+++\n").unwrap();
        std::fs::write(
            posts.join("first.md"),
            "+++\ntitle = \"First Post\"\n+++\nHello",
        )
        .unwrap();

        let loaded = load_content(&content_dir, "https://example.com").unwrap();
        assert_eq!(loaded.sections.len(), 2);
        assert_eq!(loaded.pages.len(), 1);
        assert!(loaded.sections.contains_key("_index.md"));
        assert!(loaded.sections.contains_key("posts/_index.md"));
        assert!(loaded.pages.contains_key("posts/first.md"));
    }

    #[test]
    fn test_load_content_colocated_assets() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content_dir = tmp.path().join("content");
        let post_dir = content_dir.join("posts").join("my-post");
        std::fs::create_dir_all(&post_dir).unwrap();

        std::fs::write(
            post_dir.join("index.md"),
            "+++\ntitle = \"My Post\"\n+++\nContent",
        )
        .unwrap();
        std::fs::write(post_dir.join("image.png"), "fake png").unwrap();

        let loaded = load_content(&content_dir, "https://example.com").unwrap();
        assert_eq!(loaded.pages.len(), 1);
        assert_eq!(loaded.assets.len(), 1);
        assert!(loaded.assets[0].to_string_lossy().contains("image.png"));
    }

    #[test]
    fn test_load_content_no_frontmatter_page() {
        let tmp = tempfile::TempDir::new().unwrap();
        let content_dir = tmp.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();

        std::fs::write(content_dir.join("plain.md"), "Just plain markdown").unwrap();

        let loaded = load_content(&content_dir, "https://example.com").unwrap();
        assert_eq!(loaded.pages.len(), 1);
        let page = loaded.pages.get("plain.md").unwrap();
        assert_eq!(page.title, "");
        assert_eq!(page.raw_content, "Just plain markdown");
    }

    // --- extract_title_description tests ---

    #[test]
    fn test_extract_title_description_basic() {
        let content = "# My Title\n\nThis is the description paragraph.\n\n## Next section";
        let (title, desc) = extract_title_description(content);
        assert_eq!(title.as_deref(), Some("My Title"));
        assert_eq!(desc.as_deref(), Some("This is the description paragraph."));
    }

    #[test]
    fn test_extract_title_description_no_title() {
        let content = "No heading here, just text.\n\nMore text.";
        let (title, desc) = extract_title_description(content);
        assert!(title.is_none());
        assert_eq!(desc.as_deref(), Some("No heading here, just text."));
    }

    #[test]
    fn test_extract_title_description_no_description() {
        let content = "# Title Only\n\n## Subheading\n\n- List item";
        let (title, desc) = extract_title_description(content);
        assert_eq!(title.as_deref(), Some("Title Only"));
        assert!(desc.is_none());
    }

    #[test]
    fn test_extract_title_description_strips_markdown() {
        let content = "# Title\n\nSee [link](https://example.com) and **bold** text.";
        let (_, desc) = extract_title_description(content);
        assert_eq!(desc.as_deref(), Some("See link and bold text."));
    }

    // --- title_from_filename tests ---

    #[test]
    fn test_title_from_filename_basic() {
        assert_eq!(title_from_filename("add-blog"), "Add blog");
    }

    #[test]
    fn test_title_from_filename_single_word() {
        assert_eq!(title_from_filename("overview"), "Overview");
    }

    // --- Page building edge cases ---

    #[test]
    fn test_build_page_reading_time_long() {
        let body = "word ".repeat(1000);
        let fm = Frontmatter::default();
        let page = build_page(fm, body, "test.md", "https://example.com");
        assert_eq!(page.word_count, 1000);
        assert_eq!(page.reading_time, 5); // 1000/200
    }

    #[test]
    fn test_build_page_aliases() {
        let fm = Frontmatter {
            aliases: vec!["/old/".into(), "/legacy/".into()],
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "new.md", "https://example.com");
        assert_eq!(page.aliases, vec!["/old/", "/legacy/"]);
    }

    #[test]
    fn test_build_page_custom_template() {
        let fm = Frontmatter {
            template: Some("custom.html".into()),
            ..Default::default()
        };
        let page = build_page(fm, "body".into(), "test.md", "https://example.com");
        assert_eq!(page.template.as_deref(), Some("custom.html"));
    }

    // --- URL path helper tests ---

    #[test]
    fn test_page_url_path_empty_parent() {
        assert_eq!(page_url_path("", "hello"), "/hello/");
    }

    #[test]
    fn test_page_url_path_nested() {
        assert_eq!(page_url_path("a/b", "slug"), "/a/b/slug/");
    }

    #[test]
    fn test_section_url_path_root() {
        assert_eq!(section_url_path(""), "/");
    }

    #[test]
    fn test_section_url_path_nested() {
        assert_eq!(section_url_path("docs/api"), "/docs/api/");
    }

    #[test]
    fn test_parent_dir_nested() {
        assert_eq!(parent_dir("a/b/c.md"), "a/b");
    }

    #[test]
    fn test_parent_dir_root() {
        assert_eq!(parent_dir("file.md"), "");
    }

    // --- Reading time edge cases ---

    #[test]
    fn test_reading_time_zero_words() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "".into(), "test.md", "https://example.com");
        assert_eq!(page.word_count, 0);
        assert_eq!(page.reading_time, 1); // 0/200 = 0, max(1) = 1
    }

    #[test]
    fn test_reading_time_one_word() {
        let fm = Frontmatter::default();
        let page = build_page(fm, "hello".into(), "test.md", "https://example.com");
        assert_eq!(page.word_count, 1);
        assert_eq!(page.reading_time, 1); // 1/200 = 0, max(1) = 1
    }

    #[test]
    fn test_reading_time_exactly_200_words() {
        let body = "word ".repeat(200);
        let fm = Frontmatter::default();
        let page = build_page(fm, body, "test.md", "https://example.com");
        assert_eq!(page.word_count, 200);
        assert_eq!(page.reading_time, 1); // 200/200 = 1
    }

    #[test]
    fn test_reading_time_201_words() {
        let body = "word ".repeat(201);
        let fm = Frontmatter::default();
        let page = build_page(fm, body, "test.md", "https://example.com");
        assert_eq!(page.word_count, 201);
        assert_eq!(page.reading_time, 1); // 201/200 = 1 (integer division)
    }

    #[test]
    fn test_reading_time_400_words() {
        let body = "word ".repeat(400);
        let fm = Frontmatter::default();
        let page = build_page(fm, body, "test.md", "https://example.com");
        assert_eq!(page.word_count, 400);
        assert_eq!(page.reading_time, 2); // 400/200 = 2
    }
}
