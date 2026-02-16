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
    /// Arbitrary extra values accessible in templates as `config.extra`.
    #[serde(default = "default_toml_table", serialize_with = "serialize_extra")]
    pub extra: toml::Value,
    /// Taxonomy definitions (default: a single `"tags"` taxonomy).
    #[serde(default, skip_serializing)]
    pub taxonomies: Vec<TaxonomyConfig>,
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

/// A taxonomy definition from `[[taxonomies]]` in `config.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct TaxonomyConfig {
    /// Taxonomy name (e.g. `"tags"`, `"categories"`).
    pub name: String,
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
            .map_err(|e| anyhow::anyhow!("Failed to read config.toml: {e}"))?;
        let mut config: Config = toml::from_str(&content)?;

        // Default taxonomy is tags if none specified
        if config.taxonomies.is_empty() {
            config.taxonomies.push(TaxonomyConfig {
                name: "tags".to_string(),
            });
        }

        // Ensure base_url has no trailing slash
        config.base_url = config.base_url.trim_end_matches('/').to_string();

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
}
