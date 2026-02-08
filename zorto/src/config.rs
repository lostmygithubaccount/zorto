use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub base_url: String,
    #[serde(default)]
    pub title: String,
    pub default_language: Option<String>,
    #[serde(default = "default_true")]
    pub compile_sass: bool,
    #[serde(default)]
    pub build_search_index: bool,
    #[serde(default)]
    pub slugify: SlugifyConfig,
    #[serde(default)]
    pub markdown: MarkdownConfig,
    #[serde(default = "default_toml_table")]
    pub extra: toml::Value,
    #[serde(default)]
    pub taxonomies: Vec<TaxonomyConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SlugifyConfig {
    #[serde(default = "default_safe")]
    pub paths: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkdownConfig {
    #[serde(default = "default_true")]
    pub highlight_code: bool,
    #[serde(default = "default_none_str")]
    pub insert_anchor_links: String,
    #[serde(default)]
    pub highlight_theme: Option<String>,
    #[serde(default)]
    pub highlight_themes_css: Vec<HighlightThemeCss>,
    #[serde(default)]
    pub render_emoji: bool,
    #[serde(default)]
    pub external_links_target_blank: bool,
    #[serde(default)]
    pub external_links_no_follow: bool,
    #[serde(default)]
    pub external_links_no_referrer: bool,
    #[serde(default)]
    pub smart_punctuation: bool,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            highlight_code: true,
            insert_anchor_links: "none".to_string(),
            highlight_theme: None,
            highlight_themes_css: vec![],
            render_emoji: false,
            external_links_target_blank: false,
            external_links_no_follow: false,
            external_links_no_referrer: false,
            smart_punctuation: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HighlightThemeCss {
    pub theme: String,
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaxonomyConfig {
    pub name: String,
    #[serde(default)]
    pub feed: bool,
}

fn default_true() -> bool {
    true
}

fn default_safe() -> String {
    "safe".to_string()
}

fn default_none_str() -> String {
    "none".to_string()
}

fn default_toml_table() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

impl Config {
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let config_path = root.join("config.toml");
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config.toml: {e}"))?;
        let mut config: Config = toml::from_str(&content)?;

        // Default taxonomy is tags if none specified
        if config.taxonomies.is_empty() {
            config.taxonomies.push(TaxonomyConfig {
                name: "tags".to_string(),
                feed: false,
            });
        }

        // Ensure base_url has no trailing slash
        config.base_url = config.base_url.trim_end_matches('/').to_string();

        Ok(config)
    }

    pub fn default_language(&self) -> &str {
        self.default_language.as_deref().unwrap_or("en")
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
        assert!(config.compile_sass);
        assert!(!config.build_search_index);
        assert_eq!(config.slugify.paths, ""); // Default derive gives empty string
        assert_eq!(config.markdown.insert_anchor_links, "none");
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
build_search_index = true

[markdown]
highlight_code = false
insert_anchor_links = "right"
render_emoji = true
external_links_target_blank = true

[[taxonomies]]
name = "categories"
feed = true
"#,
        );
        let config = Config::load(tmp.path()).unwrap();
        assert_eq!(config.title, "My Site");
        assert_eq!(config.default_language.as_deref(), Some("fr"));
        assert!(!config.compile_sass);
        assert!(config.build_search_index);
        assert!(!config.markdown.highlight_code);
        assert_eq!(config.markdown.insert_anchor_links, "right");
        assert!(config.markdown.render_emoji);
        assert!(config.markdown.external_links_target_blank);
        assert_eq!(config.taxonomies.len(), 1);
        assert_eq!(config.taxonomies[0].name, "categories");
        assert!(config.taxonomies[0].feed);
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
        assert_eq!(config.default_language(), "en");
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
        assert_eq!(config.default_language(), "ja");
    }

    #[test]
    fn test_missing_config_file() {
        let tmp = TempDir::new().unwrap();
        let result = Config::load(tmp.path());
        assert!(result.is_err());
    }
}
