use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level site configuration, loaded from `config.toml`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Site base URL without trailing slash (e.g. `"https://example.com"`).
    pub base_url: String,
    /// Site title, used in feeds, templates, and `llms.txt`.
    #[serde(default)]
    pub title: String,
    /// Site description, used in feeds and `llms.txt`.
    #[serde(default)]
    pub description: String,
    /// Default language code (default: `"en"`).
    #[serde(default = "default_en")]
    pub default_language: String,
    /// Compile SCSS files from `sass/` directory (default: `true`).
    #[serde(default = "default_true", skip_serializing)]
    pub compile_sass: bool,
    /// Generate an Atom feed at `/atom.xml` (default: `false`).
    #[serde(default)]
    pub generate_feed: bool,
    /// Generate a sitemap at `/sitemap.xml` (default: `true`).
    #[serde(default = "default_true", skip_serializing)]
    pub generate_sitemap: bool,
    /// Generate `llms.txt` and `llms-full.txt` (default: `true`).
    #[serde(default = "default_true", skip_serializing)]
    pub generate_llms_txt: bool,
    /// Markdown rendering options.
    #[serde(default)]
    pub markdown: MarkdownConfig,
    /// Built-in theme name (e.g. `"zorto"`, `"dkdc"`, `"default"`).
    ///
    /// When set, the theme provides default templates and SCSS. Local
    /// `templates/` and `sass/` files override theme defaults.
    #[serde(default, skip_serializing)]
    pub theme: Option<String>,
    /// Arbitrary extra values accessible in templates as `config.extra`.
    #[serde(default = "default_toml_table", serialize_with = "serialize_extra")]
    pub extra: toml::Value,
    /// Taxonomy definitions (default: a single `"tags"` taxonomy).
    #[serde(default, skip_serializing)]
    pub taxonomies: Vec<TaxonomyConfig>,
    /// Generate a SQLite FTS5 search index at `/search.db` (default: `false`).
    ///
    /// When enabled, a `search.db` file is generated at build time containing
    /// a full-text search index of all pages. The client-side uses sql.js
    /// (SQLite WASM) to query the database in the browser.
    #[serde(default)]
    pub generate_search: bool,
    /// Generate `.md` output files alongside HTML for every page (default: `false`).
    #[serde(default)]
    pub generate_md_files: bool,
    /// Compile CSS for all available themes as `style-{name}.css` (default: `false`).
    ///
    /// When enabled, every built-in theme's SCSS is compiled in addition to the
    /// active theme's `style.css`. Useful for theme preview/switcher pages.
    #[serde(default)]
    pub compile_all_themes: bool,
    /// External content directories to load as pages/sections.
    #[serde(default, skip_serializing)]
    pub content_dirs: Vec<ContentDirConfig>,
    /// Code block execution cache configuration.
    #[serde(default, skip_serializing)]
    pub cache: CacheConfig,
}

/// Where to insert anchor links on headings.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnchorLinks {
    /// No anchor links.
    #[default]
    None,
    /// Anchor link appended after heading text.
    Right,
}

/// How pages in a section are sorted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    /// Reverse chronological (newest first). Pages without dates sort last.
    #[default]
    Date,
    /// Alphabetical by title.
    Title,
}

/// Configuration for the Markdown rendering pipeline.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarkdownConfig {
    /// Enable syntax highlighting for fenced code blocks (default: `true`).
    #[serde(default = "default_true")]
    pub highlight_code: bool,
    /// Insert anchor links on headings.
    #[serde(default)]
    pub insert_anchor_links: AnchorLinks,
    /// Syntect theme name (default: `"base16-ocean.dark"`).
    #[serde(default)]
    pub highlight_theme: Option<String>,
    /// Open external links in a new tab.
    #[serde(default)]
    pub external_links_target_blank: bool,
    /// Add `rel="nofollow"` to external links.
    #[serde(default)]
    pub external_links_no_follow: bool,
    /// Add `rel="noreferrer"` to external links.
    #[serde(default)]
    pub external_links_no_referrer: bool,
    /// Enable smart punctuation (curly quotes, em dashes, etc.).
    #[serde(default)]
    pub smart_punctuation: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            highlight_code: true,
            insert_anchor_links: AnchorLinks::None,
            highlight_theme: None,
            external_links_target_blank: false,
            external_links_no_follow: false,
            external_links_no_referrer: false,
            smart_punctuation: false,
        }
    }
}

/// Configuration for code block execution caching.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Enable caching of executable code block results (default: `false`).
    #[serde(default)]
    pub enable: bool,
}

/// A taxonomy definition from `[[taxonomies]]` in `config.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct TaxonomyConfig {
    /// Taxonomy name (e.g. `"tags"`, `"categories"`).
    pub name: String,
}

/// Configuration for loading an external directory of plain markdown as content.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentDirConfig {
    /// Path to the external directory (relative to site root).
    pub path: String,
    /// URL prefix for generated pages (e.g. `"docs"` → `/docs/...`).
    pub url_prefix: String,
    /// Template for generated pages (default: `"page.html"`).
    #[serde(default = "default_page_html")]
    pub template: String,
    /// Template for generated sections (default: `"section.html"`).
    #[serde(default = "default_section_html")]
    pub section_template: String,
    /// Sort order for pages within generated sections.
    #[serde(default)]
    pub sort_by: Option<SortBy>,
    /// Rewrite relative `.md` links in content to clean URL paths.
    #[serde(default)]
    pub rewrite_links: bool,
    /// Files to exclude (relative to the external directory, e.g. `"reference/cli.md"`).
    /// Excluded files are expected to exist as manual content in `content/`.
    #[serde(default)]
    pub exclude: Vec<String>,
}

fn default_page_html() -> String {
    "page.html".to_string()
}

fn default_section_html() -> String {
    "section.html".to_string()
}

fn default_true() -> bool {
    true
}

fn default_en() -> String {
    "en".to_string()
}

pub(crate) fn default_toml_table() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

fn serialize_extra<S: serde::Serializer>(v: &toml::Value, s: S) -> Result<S::Ok, S::Error> {
    crate::content::toml_to_json(v).serialize(s)
}

impl Config {
    /// Load and validate configuration from `config.toml` in the given root directory.
    ///
    /// # Errors
    ///
    /// Returns an error if `config.toml` is missing, unreadable, or contains
    /// invalid TOML.
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let config_path = root.join("config.toml");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", config_path.display()))?;
        let mut config: Config =
            toml::from_str(&content).map_err(|e| anyhow::anyhow!("invalid config.toml: {e}"))?;

        // Default taxonomy is tags if none specified
        if config.taxonomies.is_empty() {
            config.taxonomies.push(TaxonomyConfig {
                name: "tags".to_string(),
            });
        }

        // Ensure base_url has no trailing slash
        config.base_url = config.base_url.trim_end_matches('/').to_string();

        // Validate theme name if set
        if let Some(ref theme_name) = config.theme {
            if crate::themes::Theme::from_name(theme_name).is_none() {
                let available = crate::themes::Theme::available();
                if available.is_empty() {
                    anyhow::bail!(
                        "Unknown theme '{theme_name}'. No built-in themes are available \
                         (theme features may be disabled)."
                    );
                } else {
                    anyhow::bail!(
                        "Unknown theme '{theme_name}', available: {}",
                        available.join(", ")
                    );
                }
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_config(tmp: &TempDir, content: &str) {
        std::fs::write(tmp.path().join("config.toml"), content).unwrap();
    }

    #[test]
    fn test_load_minimal_config() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com""#);
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.base_url, "https://example.com");
        assert_eq!(config.title, "");
        assert_eq!(config.description, "");
        assert!(config.compile_sass);
        assert!(config.generate_sitemap);
        assert!(config.generate_llms_txt);
        assert_eq!(config.markdown.insert_anchor_links, AnchorLinks::None);
        assert!(config.markdown.highlight_code);
        // Default taxonomy is "tags"
        assert_eq!(config.taxonomies.len(), 1);
        assert_eq!(config.taxonomies[0].name, "tags");
    }

    #[test]
    fn test_load_full_config() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
title = "My Site"
default_language = "fr"
compile_sass = false

[markdown]
highlight_code = false
insert_anchor_links = "right"
external_links_target_blank = true

[[taxonomies]]
name = "categories"
feed = true
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.title, "My Site");
        assert_eq!(config.default_language, "fr");
        assert!(!config.compile_sass);
        assert!(!config.markdown.highlight_code);
        assert_eq!(config.markdown.insert_anchor_links, AnchorLinks::Right);
        assert!(config.markdown.external_links_target_blank);
        assert_eq!(config.taxonomies.len(), 1);
        assert_eq!(config.taxonomies[0].name, "categories");
    }

    #[test]
    fn test_trailing_slash_stripped() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com/""#);
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.base_url, "https://example.com");
    }

    #[test]
    fn test_default_language_fallback() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com""#);
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.default_language, "en");
    }

    #[test]
    fn test_default_language_set() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
default_language = "ja"
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.default_language, "ja");
    }

    #[test]
    fn test_missing_config_file() {
        let tmp = TempDir::new().unwrap();
        let result = Config::load(tmp.path());
        assert!(result.is_err());
    }

    // --- Additional config parsing tests ---

    #[test]
    fn test_invalid_toml_syntax() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, "base_url = ");
        let result = Config::load(tmp.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid config.toml")
        );
    }

    #[test]
    fn test_missing_base_url() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"title = "No base URL""#);
        let result = Config::load(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_theme_name() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
theme = "nonexistent-theme"
"#,
        );
        let result = Config::load(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown theme"));
    }

    #[test]
    fn test_multiple_trailing_slashes_stripped() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com///""#);
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.base_url, "https://example.com");
    }

    #[test]
    fn test_multiple_taxonomies() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"

[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.taxonomies.len(), 2);
        assert_eq!(config.taxonomies[0].name, "tags");
        assert_eq!(config.taxonomies[1].name, "categories");
    }

    #[test]
    fn test_default_values() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com""#);
        let config = Config::load(tmp.path()).unwrap();
        assert!(config.compile_sass);
        assert!(!config.generate_feed);
        assert!(config.generate_sitemap);
        assert!(config.generate_llms_txt);
        assert!(!config.generate_search);
        assert!(!config.generate_md_files);
        assert!(!config.compile_all_themes);
        assert_eq!(config.default_language, "en");
        assert!(config.theme.is_none());
        assert!(config.content_dirs.is_empty());
    }

    #[test]
    fn test_markdown_config_defaults() {
        let tmp = TempDir::new().unwrap();
        write_config(&tmp, r#"base_url = "https://example.com""#);
        let config = Config::load(tmp.path()).unwrap();
        assert!(config.markdown.highlight_code);
        assert_eq!(config.markdown.insert_anchor_links, AnchorLinks::None);
        assert!(config.markdown.highlight_theme.is_none());
        assert!(!config.markdown.external_links_target_blank);
        assert!(!config.markdown.external_links_no_follow);
        assert!(!config.markdown.external_links_no_referrer);
        assert!(!config.markdown.smart_punctuation);
    }

    #[test]
    fn test_full_markdown_config() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"

[markdown]
highlight_code = true
insert_anchor_links = "right"
highlight_theme = "InspiredGitHub"
external_links_target_blank = true
external_links_no_follow = true
external_links_no_referrer = true
smart_punctuation = true
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert!(config.markdown.highlight_code);
        assert_eq!(config.markdown.insert_anchor_links, AnchorLinks::Right);
        assert_eq!(
            config.markdown.highlight_theme.as_deref(),
            Some("InspiredGitHub")
        );
        assert!(config.markdown.external_links_target_blank);
        assert!(config.markdown.external_links_no_follow);
        assert!(config.markdown.external_links_no_referrer);
        assert!(config.markdown.smart_punctuation);
    }

    #[test]
    fn test_extra_config_values() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"

[extra]
author = "Cody"
year = 2025
social = { github = "user", twitter = "user" }
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        let extra = &config.extra;
        assert_eq!(extra.get("author").and_then(|v| v.as_str()), Some("Cody"));
        assert_eq!(extra.get("year").and_then(|v| v.as_integer()), Some(2025));
        let social = extra.get("social").unwrap().as_table().unwrap();
        assert_eq!(social.get("github").and_then(|v| v.as_str()), Some("user"));
    }

    #[test]
    fn test_content_dirs_config() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"

[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "doc.html"
section_template = "doc-section.html"
sort_by = "title"
rewrite_links = true
exclude = ["internal.md", "draft.md"]
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.content_dirs.len(), 1);
        let dir = &config.content_dirs[0];
        assert_eq!(dir.path, "../docs");
        assert_eq!(dir.url_prefix, "docs");
        assert_eq!(dir.template, "doc.html");
        assert_eq!(dir.section_template, "doc-section.html");
        assert_eq!(dir.sort_by, Some(SortBy::Title));
        assert!(dir.rewrite_links);
        assert_eq!(dir.exclude, vec!["internal.md", "draft.md"]);
    }

    #[test]
    fn test_content_dirs_defaults() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"

[[content_dirs]]
path = "../docs"
url_prefix = "docs"
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        let dir = &config.content_dirs[0];
        assert_eq!(dir.template, "page.html");
        assert_eq!(dir.section_template, "section.html");
        assert!(dir.sort_by.is_none());
        assert!(!dir.rewrite_links);
        assert!(dir.exclude.is_empty());
    }

    #[test]
    fn test_generate_feed_enabled() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
generate_feed = true
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert!(config.generate_feed);
    }

    #[test]
    fn test_generate_md_files_enabled() {
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
generate_md_files = true
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert!(config.generate_md_files);
    }

    #[test]
    fn test_unknown_top_level_keys_accepted() {
        // Config does not use #[serde(deny_unknown_fields)], so unknown keys are silently ignored
        let tmp = TempDir::new().unwrap();
        write_config(
            &tmp,
            r#"
base_url = "https://example.com"
some_future_field = true
"#,
        );
        let config = Config::load(tmp.path()).expect("unknown top-level keys should be accepted");
        assert_eq!(config.base_url, "https://example.com");
    }
}
