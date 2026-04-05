//! Config editor routes.
//!
//! Provides both a visual form for common fields and a raw TOML editor
//! for the full config.

use axum::extract::State;
use axum::response::Html;
use std::sync::Arc;

use crate::html;
use crate::{AppState, escape};

pub async fn edit(State(state): State<Arc<AppState>>) -> Html<String> {
    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let config_path = state.root.join("config.toml");
    let content = std::fs::read_to_string(&config_path).unwrap_or_default();

    Html(render_config_editor(&site_title, &content, None, &base_url))
}

pub async fn save(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<SaveForm>,
) -> Html<String> {
    let config_path = state.root.join("config.toml");

    let new_content = if form.mode == "visual" {
        // Rebuild config from visual form fields, preserving raw sections
        build_config_from_form(&form, &config_path)
    } else {
        form.content.clone()
    };

    let flash = match toml::from_str::<toml::Value>(&new_content) {
        Ok(_) => match std::fs::write(&config_path, &new_content) {
            Ok(()) => {
                let _ = rebuild_site(&state);
                Some(("success", "Config saved and site rebuilt.".to_string()))
            }
            Err(e) => Some(("error", format!("Error writing: {e}"))),
        },
        Err(e) => Some(("error", format!("Invalid TOML: {e}"))),
    };

    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
    let flash_ref = flash.as_ref().map(|(k, v)| (*k, v.as_str()));

    Html(render_config_editor(
        &site_title,
        &content,
        flash_ref,
        &base_url,
    ))
}

#[derive(serde::Deserialize)]
pub struct SaveForm {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    content: String,
    // Visual form fields
    #[serde(default)]
    title: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    theme: String,
    #[serde(default)]
    generate_feed: String,
    #[serde(default)]
    generate_sitemap: String,
}

fn render_config_editor(
    site_title: &str,
    content: &str,
    flash: Option<(&str, &str)>,
    base_url: &str,
) -> String {
    let flash_html = flash
        .map(|(kind, msg)| {
            format!(
                r#"<div class="flash flash-{kind}">{msg}</div>"#,
                msg = escape(msg)
            )
        })
        .unwrap_or_default();

    // Parse config for visual form
    let parsed = toml::from_str::<toml::Value>(content).ok();
    let get_str = |key: &str| -> String {
        parsed
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let get_bool = |key: &str| -> bool {
        parsed
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };

    let title = get_str("title");
    let config_base_url = get_str("base_url");
    let description = get_str("description");
    let theme = get_str("theme");
    let generate_feed = get_bool("generate_feed");
    let generate_sitemap = get_bool("generate_sitemap");

    // Theme dropdown options
    let themes = zorto_core::themes::Theme::available();
    let theme_options: String = std::iter::once(format!(
        r#"<option value=""{}>(none — use custom templates)</option>"#,
        if theme.is_empty() { " selected" } else { "" }
    ))
    .chain(themes.iter().map(|t| {
        let selected = if *t == theme { " selected" } else { "" };
        format!(r#"<option value="{t}"{selected}>{t}</option>"#)
    }))
    .collect::<Vec<_>>()
    .join("\n              ");

    let feed_checked = if generate_feed { " checked" } else { "" };
    let sitemap_checked = if generate_sitemap { " checked" } else { "" };

    let body = format!(
        r##"{flash_html}
<h2>Site Configuration</h2>

<div class="card" style="margin-bottom: 16px;">
  <h3>Settings</h3>
  <form method="POST" action="/config" style="margin-top: 12px;">
    <input type="hidden" name="mode" value="visual">
    <input type="hidden" name="content" value="{e_content}">
    <div class="form-row">
      <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" value="{e_title}">
      </div>
      <div class="form-group">
        <label>Base URL</label>
        <input type="text" name="base_url" value="{e_base_url}">
      </div>
    </div>
    <div class="form-row">
      <div class="form-group">
        <label>Description</label>
        <input type="text" name="description" value="{e_description}">
      </div>
      <div class="form-group" style="max-width: 200px;">
        <label>Theme</label>
        <select name="theme">
              {theme_options}
        </select>
      </div>
    </div>
    <div style="display: flex; gap: 16px; margin-bottom: 16px;">
      <label style="display: flex; align-items: center; gap: 6px; text-transform: none; font-size: 0.85rem; cursor: pointer;">
        <input type="checkbox" name="generate_feed" value="true"{feed_checked}> Generate Atom Feed
      </label>
      <label style="display: flex; align-items: center; gap: 6px; text-transform: none; font-size: 0.85rem; cursor: pointer;">
        <input type="checkbox" name="generate_sitemap" value="true"{sitemap_checked}> Generate Sitemap
      </label>
    </div>
    <button type="submit" class="btn btn-primary">Save Settings</button>
  </form>
</div>

<div class="card">
  <h3>Raw Config <span style="color: #666680; font-size: 0.75rem; font-weight: normal;">(config.toml)</span></h3>
  <form method="POST" action="/config" style="margin-top: 12px;">
    <input type="hidden" name="mode" value="raw">
    <div class="form-group">
      <textarea name="content" rows="24">{e_content}</textarea>
    </div>
    <button type="submit" class="btn btn-primary">Save Raw Config</button>
  </form>
</div>"##,
        e_title = escape(&title),
        e_base_url = escape(&config_base_url),
        e_description = escape(&description),
        e_content = escape(content),
    );

    html::page("Config", site_title, "config", &body, base_url)
}

/// Build a config.toml string from the visual form fields while preserving
/// sections and fields that aren't in the visual form.
fn build_config_from_form(form: &SaveForm, config_path: &std::path::Path) -> String {
    let existing = std::fs::read_to_string(config_path).unwrap_or_default();

    // Parse existing config
    let mut config: toml::Value =
        toml::from_str(&existing).unwrap_or(toml::Value::Table(toml::map::Map::new()));

    if let Some(table) = config.as_table_mut() {
        // Update visual form fields
        if !form.title.is_empty() {
            table.insert("title".into(), toml::Value::String(form.title.clone()));
        }
        if !form.base_url.is_empty() {
            table.insert(
                "base_url".into(),
                toml::Value::String(form.base_url.clone()),
            );
        }
        if !form.description.is_empty() {
            table.insert(
                "description".into(),
                toml::Value::String(form.description.clone()),
            );
        } else {
            table.remove("description");
        }
        if !form.theme.is_empty() {
            table.insert("theme".into(), toml::Value::String(form.theme.clone()));
        } else {
            table.remove("theme");
        }

        table.insert(
            "generate_feed".into(),
            toml::Value::Boolean(form.generate_feed == "true"),
        );
        table.insert(
            "generate_sitemap".into(),
            toml::Value::Boolean(form.generate_sitemap == "true"),
        );
    }

    toml::to_string_pretty(&config).unwrap_or(existing)
}

use crate::rebuild_site;
