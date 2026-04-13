//! Template linting — opinionated checks for Zorto templates.
//!
//! Inspired by clippy and rustfmt: warns about patterns that make themes
//! harder to maintain, like hardcoded user-facing strings in templates.
//! Strings should live in `config.toml` (`[extra]`) or content markdown
//! files, not in HTML templates.

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use crate::content::{Page, Section, section_key_for};

/// A lint warning produced by the linter.
#[derive(Debug)]
pub struct LintWarning {
    /// The lint rule that triggered (e.g. `"hardcoded-string"`, `"broken-link"`).
    pub rule: String,
    /// Relative path to the file.
    pub file: String,
    /// 1-based line number (0 if not applicable).
    pub line: usize,
    /// The offending text snippet.
    pub text: String,
    /// Human-readable message.
    pub message: String,
}

impl std::fmt::Display for LintWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(
                f,
                "warning[{}]: {}:{}: \"{}\" -- {}",
                self.rule, self.file, self.line, self.text, self.message
            )
        } else {
            write!(
                f,
                "warning[{}]: {}: \"{}\" -- {}",
                self.rule, self.file, self.text, self.message
            )
        }
    }
}

/// Regex to match Tera expressions and tags: {{ ... }}, {% ... %}, {# ... #}
static TERA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\{\{.*?\}\}|\{%.*?%\}|\{#.*?#\}").unwrap());

/// Regex to match HTML tags (including self-closing)
static HTML_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<[^>]+>").unwrap());

/// Regex to detect user-facing text: 2+ word characters in a row
static TEXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Za-z][A-Za-z]{1,}\b").unwrap());

/// Common structural strings that are acceptable in templates.
const ALLOWLIST: &[&str] = &[
    // HTML/structural
    "DOCTYPE", "html", "head", "body", "main", "nav", "footer", "div", "span", "ul", "li", "button",
    "label", "input", "meta", "link", "script", "style", "svg", "path", "circle", "line", "rect",
    "polyline", "xmlns", "viewBox", "fill", "stroke", "width", "height", "cx", "cy",
    // Accessibility
    "aria", // CSS class names
    "class", "id", "href", "src", "alt", "rel", "type", "name", "content", "charset", "viewport",
    "robots", "noodp", // Common template text
    "if", "else", "endif", "for", "endfor", "block", "endblock", "extends", "macro", "import",
    "set", "true", "false", // HTML entities / symbols
    "larr", "rarr", "copy", "amp", "nbsp", "lt", "gt", "xFE",
];

/// Lint all HTML template files in the given directory.
///
/// Returns warnings for lines that appear to contain hardcoded user-facing
/// strings. Skips files in `shortcodes/` subdirectory and content inside
/// `<script>` and `<style>` blocks.
pub fn lint_templates(templates_dir: &Path) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    if !templates_dir.exists() {
        return warnings;
    }

    for entry in walkdir::WalkDir::new(templates_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip non-HTML files
        if path.extension().and_then(|e| e.to_str()) != Some("html") {
            continue;
        }

        // Skip shortcodes directory
        let rel = path
            .strip_prefix(templates_dir)
            .unwrap_or(path)
            .to_string_lossy();
        if rel.starts_with("shortcodes") {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };

        lint_template_content(&rel, &content, &mut warnings);
    }

    warnings
}

/// Lint a single template's content.
fn lint_template_content(file: &str, content: &str, warnings: &mut Vec<LintWarning>) {
    // Remove <script>...</script> and <style>...</style> blocks
    let no_script = remove_blocks(content, "script");
    let cleaned = remove_blocks(&no_script, "style");

    for (line_idx, line) in cleaned.lines().enumerate() {
        let line_num = line_idx + 1;

        // Strip Tera expressions/tags from the line
        let no_tera = TERA_RE.replace_all(line, " ");

        // Strip HTML tags
        let no_html = HTML_TAG_RE.replace_all(&no_tera, " ");

        // Look for remaining text that looks like user-facing content
        for m in TEXT_RE.find_iter(&no_html) {
            let word = m.as_str();

            // Skip allowlisted words
            if ALLOWLIST.iter().any(|&a| word.eq_ignore_ascii_case(a)) {
                continue;
            }

            // Skip single short words (likely CSS classes or HTML attributes)
            if word.len() <= 3 {
                continue;
            }

            warnings.push(LintWarning {
                rule: "hardcoded-string".to_string(),
                file: file.to_string(),
                line: line_num,
                text: word.to_string(),
                message: "consider moving to config.extra or content".to_string(),
            });
        }
    }
}

/// Remove all `<tag>...</tag>` blocks from content.
fn remove_blocks(content: &str, tag: &str) -> String {
    let re = Regex::new(&format!(r"(?si)<{tag}[\s>].*?</{tag}>")).unwrap();
    re.replace_all(content, " ").to_string()
}

/// Regex to match `@/` internal links in markdown content.
static CONTENT_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@/([^)#\s]+\.md)(#[^)\s]+)?").unwrap());

/// Regex to match image references in markdown: `![alt](path)` and HTML `<img src="path">`.
static MD_IMAGE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"!\[[^\]]*\]\((/[^)]+)\)|<img[^>]+src="(/[^"]+)""#).unwrap());

/// Lint `@/` internal links: verify they resolve to a known page or section.
///
/// Returns warnings for any `@/path.md` links that don't match a loaded page or section.
pub fn lint_internal_links(
    pages: &HashMap<String, Page>,
    sections: &HashMap<String, Section>,
) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    // Check pages
    for page in pages.values() {
        for (line_idx, line) in page.raw_content.lines().enumerate() {
            for caps in CONTENT_LINK_RE.captures_iter(line) {
                let link_path = &caps[1];
                if !pages.contains_key(link_path) && !sections.contains_key(link_path) {
                    warnings.push(LintWarning {
                        rule: "broken-link".to_string(),
                        file: page.relative_path.clone(),
                        line: line_idx + 1,
                        text: format!("@/{link_path}"),
                        message: "internal link target does not exist".to_string(),
                    });
                }
            }
        }
    }

    // Check sections
    for section in sections.values() {
        for (line_idx, line) in section.raw_content.lines().enumerate() {
            for caps in CONTENT_LINK_RE.captures_iter(line) {
                let link_path = &caps[1];
                if !pages.contains_key(link_path) && !sections.contains_key(link_path) {
                    warnings.push(LintWarning {
                        rule: "broken-link".to_string(),
                        file: section.relative_path.clone(),
                        line: line_idx + 1,
                        text: format!("@/{link_path}"),
                        message: "internal link target does not exist".to_string(),
                    });
                }
            }
        }
    }

    warnings
}

/// Lint frontmatter: check required fields.
///
/// - All pages must have a `title`
/// - Pages in date-sorted sections must have a `date`
/// - Dates must be valid format (YYYY-MM-DD, YYYY-MM-DDThh:mm:ss, or RFC 3339)
pub fn lint_frontmatter(
    pages: &HashMap<String, Page>,
    sections: &HashMap<String, Section>,
) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    for page in pages.values() {
        // Title is required for all pages
        if page.title.is_empty() {
            warnings.push(LintWarning {
                rule: "missing-title".to_string(),
                file: page.relative_path.clone(),
                line: 0,
                text: String::new(),
                message: "page is missing required `title` in frontmatter".to_string(),
            });
        }

        // Date is required for pages in date-sorted sections
        let section_key = section_key_for(&page.relative_path);
        let in_date_section = sections
            .get(&section_key)
            .is_some_and(|s| matches!(s.sort_by, Some(crate::config::SortBy::Date)));
        if in_date_section && page.date.is_none() {
            warnings.push(LintWarning {
                rule: "missing-date".to_string(),
                file: page.relative_path.clone(),
                line: 0,
                text: String::new(),
                message: "page in date-sorted section is missing required `date` in frontmatter"
                    .to_string(),
            });
        }

        // Validate date format if present
        if let Some(ref date_str) = page.date {
            if !is_valid_date(date_str) {
                warnings.push(LintWarning {
                    rule: "invalid-date".to_string(),
                    file: page.relative_path.clone(),
                    line: 0,
                    text: date_str.clone(),
                    message: "date is not a valid format (expected YYYY-MM-DD, YYYY-MM-DDThh:mm:ss, or RFC 3339)".to_string(),
                });
            }
        }
    }

    warnings
}

/// Reveal.js transition keywords. Anything else is silently ignored by reveal,
/// so a typo in `[extra] transition` produces a deck that just defaults to
/// `slide` without telling the author. Used by [`lint_presentation_transitions`].
const REVEAL_TRANSITIONS: &[&str] = &["slide", "fade", "convex", "concave", "zoom", "none"];

/// Lint per-slide and per-deck `transition` values against the reveal.js allowlist.
///
/// Only sections rendered with `presentation.html` are inspected (the keyword set
/// is reveal-specific). For each such section, the section's own `transition` and
/// each child page's `transition` are validated.
pub fn lint_presentation_transitions(
    pages: &HashMap<String, Page>,
    sections: &HashMap<String, Section>,
) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    for section in sections.values() {
        if section.template.as_deref() != Some("presentation.html") {
            continue;
        }

        check_transition(
            section.extra.get("transition"),
            &section.relative_path,
            "section",
            &mut warnings,
        );

        let section_key = section.relative_path.as_str();
        let prefix = section_key.trim_end_matches("_index.md");
        for page in pages.values() {
            if !page.relative_path.starts_with(prefix)
                || page.relative_path == section.relative_path
            {
                continue;
            }
            check_transition(
                page.extra.get("transition"),
                &page.relative_path,
                "slide",
                &mut warnings,
            );
        }
    }

    warnings
}

fn check_transition(
    value: Option<&serde_json::Value>,
    file: &str,
    scope: &str,
    warnings: &mut Vec<LintWarning>,
) {
    let Some(t) = value.and_then(|v| v.as_str()) else {
        return;
    };
    if REVEAL_TRANSITIONS.contains(&t) {
        return;
    }
    warnings.push(LintWarning {
        rule: "invalid-transition".to_string(),
        file: file.to_string(),
        line: 0,
        text: t.to_string(),
        message: format!(
            "{scope} `transition` must be one of {} -- reveal.js silently ignores unknown values",
            REVEAL_TRANSITIONS.join(", ")
        ),
    });
}

/// Check if a date string is a valid format.
fn is_valid_date(s: &str) -> bool {
    // RFC 3339 / offset datetime
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return true;
    }
    // Naive datetime
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return true;
    }
    // Date only
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return true;
    }
    false
}

/// Lint missing assets: check that images referenced with absolute paths exist in static/.
///
/// Checks `![alt](/path/to/image.png)` and `<img src="/path/to/image.png">` patterns.
/// Only checks paths starting with `/` (site-relative) — external URLs are skipped.
pub fn lint_missing_assets(
    pages: &HashMap<String, Page>,
    sections: &HashMap<String, Section>,
    static_dir: &Path,
) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    let check_content = |raw_content: &str, file: &str, warnings: &mut Vec<LintWarning>| {
        for (line_idx, line) in raw_content.lines().enumerate() {
            for caps in MD_IMAGE_RE.captures_iter(line) {
                // Group 1 is markdown image, group 2 is HTML img
                let path = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str());
                if let Some(asset_path) = path {
                    // Strip leading / to make it relative to static/
                    let relative = asset_path.trim_start_matches('/');
                    if !static_dir.join(relative).exists() {
                        warnings.push(LintWarning {
                            rule: "missing-asset".to_string(),
                            file: file.to_string(),
                            line: line_idx + 1,
                            text: asset_path.to_string(),
                            message: "referenced image does not exist in static/".to_string(),
                        });
                    }
                }
            }
        }
    };

    for page in pages.values() {
        check_content(&page.raw_content, &page.relative_path, &mut warnings);
    }

    for section in sections.values() {
        check_content(&section.raw_content, &section.relative_path, &mut warnings);
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lint_detects_hardcoded_string() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), "<h1>Welcome to my site</h1>").unwrap();
        let warnings = lint_templates(&dir);
        assert!(
            warnings.iter().any(|w| w.text == "Welcome"),
            "Should flag 'Welcome': {warnings:?}"
        );
    }

    #[test]
    fn test_lint_allows_tera_expressions() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.html"), "<h1>{{ config.title }}</h1>").unwrap();
        let warnings = lint_templates(&dir);
        assert!(
            warnings.is_empty(),
            "Should not flag Tera expressions: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_skips_script_blocks() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("test.html"),
            "<script>var message = 'Hello World';</script>",
        )
        .unwrap();
        let warnings = lint_templates(&dir);
        assert!(
            warnings.is_empty(),
            "Should not flag content in script tags: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_skips_style_blocks() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("templates");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("test.html"),
            "<style>.greeting { color: red; }</style>",
        )
        .unwrap();
        let warnings = lint_templates(&dir);
        assert!(
            warnings.is_empty(),
            "Should not flag content in style tags: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_skips_shortcodes_dir() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("templates");
        let sc_dir = dir.join("shortcodes");
        std::fs::create_dir_all(&sc_dir).unwrap();
        std::fs::write(
            sc_dir.join("note.html"),
            "<div>Warning: important notice</div>",
        )
        .unwrap();
        let warnings = lint_templates(&dir);
        assert!(
            warnings.is_empty(),
            "Should not lint shortcode templates: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_nonexistent_dir() {
        let warnings = lint_templates(Path::new("/nonexistent"));
        assert!(warnings.is_empty());
    }

    // --- Internal link tests ---

    use crate::content::{Frontmatter, build_page, build_section};

    fn make_page(relative_path: &str, raw_content: &str) -> Page {
        let fm = Frontmatter {
            title: Some("Test".to_string()),
            date: Some(toml::Value::String("2025-01-01".to_string())),
            ..Frontmatter::default()
        };
        let mut page = build_page(fm, String::new(), relative_path, "https://example.com");
        page.raw_content = raw_content.to_string();
        page
    }

    fn make_section_with(relative_path: &str, raw_content: &str) -> Section {
        let fm = Frontmatter::default();
        let mut section = build_section(fm, String::new(), relative_path, "https://example.com");
        section.raw_content = raw_content.to_string();
        section
    }

    #[test]
    fn test_lint_broken_internal_link() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "Check [missing](@/posts/nonexistent.md)"),
        );
        let sections = HashMap::new();
        let warnings = lint_internal_links(&pages, &sections);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "broken-link");
        assert!(warnings[0].text.contains("nonexistent"));
    }

    #[test]
    fn test_lint_valid_internal_link() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "See [other](@/posts/other.md)"),
        );
        pages.insert(
            "posts/other.md".into(),
            make_page("posts/other.md", "Other page"),
        );
        let sections = HashMap::new();
        let warnings = lint_internal_links(&pages, &sections);
        assert!(
            warnings.is_empty(),
            "Valid link should not warn: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_internal_link_to_section() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "See [blog](@/posts/_index.md)"),
        );
        let mut sections = HashMap::new();
        sections.insert(
            "posts/_index.md".into(),
            make_section_with("posts/_index.md", ""),
        );
        let warnings = lint_internal_links(&pages, &sections);
        assert!(
            warnings.is_empty(),
            "Link to section should not warn: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_broken_link_in_section() {
        let pages = HashMap::new();
        let mut sections = HashMap::new();
        sections.insert(
            "posts/_index.md".into(),
            make_section_with("posts/_index.md", "See [missing](@/posts/gone.md)"),
        );
        let warnings = lint_internal_links(&pages, &sections);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "broken-link");
    }

    // --- Frontmatter validation tests ---

    #[test]
    fn test_lint_missing_title() {
        let mut pages = HashMap::new();
        let fm = Frontmatter::default(); // no title
        let page = build_page(fm, "content".into(), "about.md", "https://example.com");
        pages.insert("about.md".into(), page);
        let sections = HashMap::new();
        let warnings = lint_frontmatter(&pages, &sections);
        assert!(
            warnings.iter().any(|w| w.rule == "missing-title"),
            "Should warn about missing title: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_with_title_no_warning() {
        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("About".to_string()),
            ..Frontmatter::default()
        };
        let page = build_page(fm, "content".into(), "about.md", "https://example.com");
        pages.insert("about.md".into(), page);
        let sections = HashMap::new();
        let warnings = lint_frontmatter(&pages, &sections);
        assert!(
            !warnings.iter().any(|w| w.rule == "missing-title"),
            "Should not warn with title set: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_missing_date_in_date_section() {
        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("Post".to_string()),
            // No date set
            ..Frontmatter::default()
        };
        let page = build_page(
            fm,
            "content".into(),
            "posts/no-date.md",
            "https://example.com",
        );
        pages.insert("posts/no-date.md".into(), page);

        let mut sections = HashMap::new();
        let sfm = Frontmatter {
            sort_by: Some(crate::config::SortBy::Date),
            ..Frontmatter::default()
        };
        let section = build_section(sfm, String::new(), "posts/_index.md", "https://example.com");
        sections.insert("posts/_index.md".into(), section);

        let warnings = lint_frontmatter(&pages, &sections);
        assert!(
            warnings.iter().any(|w| w.rule == "missing-date"),
            "Should warn about missing date in date-sorted section: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_date_not_required_in_title_section() {
        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("Item".to_string()),
            // No date, but section is title-sorted
            ..Frontmatter::default()
        };
        let page = build_page(
            fm,
            "content".into(),
            "items/thing.md",
            "https://example.com",
        );
        pages.insert("items/thing.md".into(), page);

        let mut sections = HashMap::new();
        let sfm = Frontmatter {
            sort_by: Some(crate::config::SortBy::Title),
            ..Frontmatter::default()
        };
        let section = build_section(sfm, String::new(), "items/_index.md", "https://example.com");
        sections.insert("items/_index.md".into(), section);

        let warnings = lint_frontmatter(&pages, &sections);
        assert!(
            !warnings.iter().any(|w| w.rule == "missing-date"),
            "Should not require date in title-sorted section: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_invalid_date_format() {
        let mut pages = HashMap::new();
        let fm = Frontmatter {
            title: Some("Post".to_string()),
            date: Some(toml::Value::String("not-a-date".to_string())),
            ..Frontmatter::default()
        };
        let page = build_page(fm, "content".into(), "posts/bad.md", "https://example.com");
        pages.insert("posts/bad.md".into(), page);
        let sections = HashMap::new();
        let warnings = lint_frontmatter(&pages, &sections);
        assert!(
            warnings.iter().any(|w| w.rule == "invalid-date"),
            "Should warn about invalid date: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_valid_date_formats() {
        assert!(is_valid_date("2025-01-15"));
        assert!(is_valid_date("2025-01-15T10:30:00"));
        assert!(is_valid_date("2025-01-15T10:30:00Z"));
        assert!(is_valid_date("2025-01-15T10:30:00+05:00"));
        assert!(!is_valid_date("Jan 15, 2025"));
        assert!(!is_valid_date("2025/01/15"));
        assert!(!is_valid_date("garbage"));
    }

    // --- Missing asset tests ---

    #[test]
    fn test_lint_missing_image() {
        let tmp = TempDir::new().unwrap();
        let static_dir = tmp.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();

        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "![photo](/img/missing.png)"),
        );
        let sections = HashMap::new();
        let warnings = lint_missing_assets(&pages, &sections, &static_dir);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "missing-asset");
        assert!(warnings[0].text.contains("missing.png"));
    }

    #[test]
    fn test_lint_existing_image() {
        let tmp = TempDir::new().unwrap();
        let static_dir = tmp.path().join("static");
        let img_dir = static_dir.join("img");
        std::fs::create_dir_all(&img_dir).unwrap();
        std::fs::write(img_dir.join("photo.png"), "fake png").unwrap();

        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "![photo](/img/photo.png)"),
        );
        let sections = HashMap::new();
        let warnings = lint_missing_assets(&pages, &sections, &static_dir);
        assert!(
            warnings.is_empty(),
            "Should not warn for existing image: {warnings:?}"
        );
    }

    #[test]
    fn test_lint_html_img_tag() {
        let tmp = TempDir::new().unwrap();
        let static_dir = tmp.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();

        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page(
                "posts/hello.md",
                r#"<img src="/img/missing.jpg" alt="photo">"#,
            ),
        );
        let sections = HashMap::new();
        let warnings = lint_missing_assets(&pages, &sections, &static_dir);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "missing-asset");
    }

    #[test]
    fn test_lint_missing_asset_in_section() {
        let tmp = TempDir::new().unwrap();
        let static_dir = tmp.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();

        let pages = HashMap::new();
        let mut sections = HashMap::new();
        sections.insert(
            "_index.md".into(),
            make_section_with("_index.md", "![banner](/img/banner.png)"),
        );
        let warnings = lint_missing_assets(&pages, &sections, &static_dir);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "missing-asset");
    }

    // --- Presentation transition tests ---

    fn presentation_section(relative_path: &str, extra: toml::Value) -> Section {
        let fm = Frontmatter {
            template: Some("presentation.html".into()),
            extra,
            ..Frontmatter::default()
        };
        build_section(fm, String::new(), relative_path, "https://example.com")
    }

    fn slide_page(relative_path: &str, extra: toml::Value) -> Page {
        let fm = Frontmatter {
            title: Some("Slide".into()),
            extra,
            ..Frontmatter::default()
        };
        build_page(fm, String::new(), relative_path, "https://example.com")
    }

    fn extra_with_transition(t: &str) -> toml::Value {
        let mut tbl = toml::map::Map::new();
        tbl.insert("transition".to_string(), toml::Value::String(t.into()));
        toml::Value::Table(tbl)
    }

    #[test]
    fn test_lint_transition_accepts_known_keywords() {
        for keyword in REVEAL_TRANSITIONS {
            let pages = HashMap::new();
            let mut sections = HashMap::new();
            sections.insert(
                "deck/_index.md".into(),
                presentation_section("deck/_index.md", extra_with_transition(keyword)),
            );
            let warnings = lint_presentation_transitions(&pages, &sections);
            assert!(
                warnings.is_empty(),
                "{keyword} should be accepted: {warnings:?}"
            );
        }
    }

    #[test]
    fn test_lint_transition_rejects_typo_on_section() {
        let pages = HashMap::new();
        let mut sections = HashMap::new();
        sections.insert(
            "deck/_index.md".into(),
            presentation_section("deck/_index.md", extra_with_transition("slid")),
        );
        let warnings = lint_presentation_transitions(&pages, &sections);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "invalid-transition");
        assert!(warnings[0].text == "slid", "got: {:?}", warnings[0]);
        assert!(
            warnings[0].message.contains("section"),
            "got: {}",
            warnings[0].message
        );
    }

    #[test]
    fn test_lint_transition_rejects_typo_on_slide() {
        let mut pages = HashMap::new();
        pages.insert(
            "deck/title.md".into(),
            slide_page("deck/title.md", extra_with_transition("fadee")),
        );
        let mut sections = HashMap::new();
        sections.insert(
            "deck/_index.md".into(),
            presentation_section("deck/_index.md", default_toml_table_for_test()),
        );
        let warnings = lint_presentation_transitions(&pages, &sections);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].rule, "invalid-transition");
        assert_eq!(warnings[0].text, "fadee");
        assert!(
            warnings[0].message.contains("slide"),
            "got: {}",
            warnings[0].message
        );
    }

    #[test]
    fn test_lint_transition_skips_non_presentation_sections() {
        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            slide_page("posts/hello.md", extra_with_transition("garbage")),
        );
        let mut sections = HashMap::new();
        // No template = "presentation.html" — section.html is the default.
        sections.insert(
            "posts/_index.md".into(),
            make_section_with("posts/_index.md", ""),
        );
        let warnings = lint_presentation_transitions(&pages, &sections);
        assert!(warnings.is_empty(), "got: {warnings:?}");
    }

    #[test]
    fn test_lint_transition_no_value_no_warning() {
        let pages = HashMap::new();
        let mut sections = HashMap::new();
        sections.insert(
            "deck/_index.md".into(),
            presentation_section("deck/_index.md", default_toml_table_for_test()),
        );
        let warnings = lint_presentation_transitions(&pages, &sections);
        assert!(warnings.is_empty(), "got: {warnings:?}");
    }

    fn default_toml_table_for_test() -> toml::Value {
        toml::Value::Table(toml::map::Map::new())
    }

    #[test]
    fn test_lint_no_images_no_warnings() {
        let tmp = TempDir::new().unwrap();
        let static_dir = tmp.path().join("static");
        std::fs::create_dir_all(&static_dir).unwrap();

        let mut pages = HashMap::new();
        pages.insert(
            "posts/hello.md".into(),
            make_page("posts/hello.md", "Just text, no images."),
        );
        let sections = HashMap::new();
        let warnings = lint_missing_assets(&pages, &sections, &static_dir);
        assert!(warnings.is_empty());
    }
}
