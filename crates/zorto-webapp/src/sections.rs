//! Section listing and editing routes.

use axum::extract::{Path, State};
use axum::response::Html;
use std::sync::Arc;

use crate::html;
use crate::{AppState, escape, validate_path};

pub async fn list(State(state): State<Arc<AppState>>) -> Html<String> {
    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let content_dir = state.root.join("content");

    let mut rows = Vec::new();
    if content_dir.exists() {
        for entry in walkdir::WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name != "_index.md" {
                continue;
            }

            let relative = path
                .strip_prefix(&content_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let content = std::fs::read_to_string(path).unwrap_or_default();
            let title = extract_fm_value(&content, "title").unwrap_or_else(|| relative.clone());

            // Count pages in this section
            let section_dir = path.parent().unwrap_or(path);
            let page_count = std::fs::read_dir(section_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            let n = e.file_name();
                            let n = n.to_string_lossy();
                            n.ends_with(".md") && n != "_index.md"
                        })
                        .count()
                })
                .unwrap_or(0);

            let section_path = std::path::Path::new(&relative)
                .parent()
                .unwrap_or(std::path::Path::new(""))
                .to_string_lossy();
            let display_path = if section_path.is_empty() {
                "/".to_string()
            } else {
                format!("/{section_path}/")
            };

            rows.push(format!(
                r#"<tr>
  <td><a href="/sections/{rel}">{title}</a> <span class="badge badge-section">section</span></td>
  <td style="color: #666680;">{display_path}</td>
  <td style="color: #666680;">{page_count} pages</td>
</tr>"#,
                rel = escape(&relative),
                title = escape(&title),
                display_path = escape(&display_path),
            ));
        }
    }

    rows.sort();
    let table_body = rows.join("\n");

    let body = format!(
        r#"<h2>Sections</h2>
<div class="card">
  <table>
    <thead>
      <tr><th>Title</th><th>URL Path</th><th>Pages</th></tr>
    </thead>
    <tbody>
      {table_body}
    </tbody>
  </table>
</div>"#
    );

    Html(html::page(
        "Sections",
        &site_title,
        "sections",
        &body,
        &base_url,
    ))
}

pub async fn edit(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Html<String> {
    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let content_dir = state.root.join("content");
    let file_path = match validate_path(&content_dir, &path) {
        Ok(p) => p,
        Err(_) => {
            return Html(html::page(
                "Error",
                &site_title,
                "sections",
                "<p>Invalid path.</p>",
                &base_url,
            ));
        }
    };
    let content = std::fs::read_to_string(&file_path).unwrap_or_default();

    Html(render_section_editor(
        &site_title,
        &path,
        &content,
        None,
        &base_url,
    ))
}

pub async fn save(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    axum::Form(form): axum::Form<SaveForm>,
) -> Html<String> {
    let content_dir = state.root.join("content");
    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let file_path = match validate_path(&content_dir, &path) {
        Ok(p) => p,
        Err(_) => {
            return Html(html::page(
                "Error",
                &site_title,
                "sections",
                "<p>Invalid path.</p>",
                &base_url,
            ));
        }
    };
    let new_content = form.to_file_content();
    let result = std::fs::write(&file_path, &new_content);

    let flash_msg: Option<(String, String)> = match result {
        Ok(()) => match rebuild_site(&state) {
            Ok(()) => Some(("success".into(), "Section saved and site rebuilt.".into())),
            Err(e) => Some(("error".into(), format!("Saved but build failed: {e}"))),
        },
        Err(e) => Some(("error".into(), format!("Error saving: {e}"))),
    };
    let flash = flash_msg.as_ref().map(|(k, v)| (k.as_str(), v.as_str()));

    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    Html(render_section_editor(
        &site_title,
        &path,
        &content,
        flash,
        &base_url,
    ))
}

#[derive(serde::Deserialize)]
pub struct SaveForm {
    title: String,
    description: String,
    sort_by: String,
    paginate_by: String,
    body: String,
    #[serde(default)]
    extra_frontmatter: String,
}

impl SaveForm {
    fn to_file_content(&self) -> String {
        let mut table = toml::map::Map::new();
        if !self.title.is_empty() {
            table.insert("title".into(), toml::Value::String(self.title.clone()));
        }
        if !self.description.is_empty() {
            table.insert(
                "description".into(),
                toml::Value::String(self.description.clone()),
            );
        }
        if !self.sort_by.is_empty() {
            table.insert("sort_by".into(), toml::Value::String(self.sort_by.clone()));
        }
        if !self.paginate_by.is_empty() {
            if let Ok(n) = self.paginate_by.parse::<i64>() {
                if n > 0 {
                    table.insert("paginate_by".into(), toml::Value::Integer(n));
                }
            }
        }
        let fm_toml = toml::to_string(&toml::Value::Table(table)).unwrap_or_default();
        let mut fm = format!("+++\n{fm_toml}");
        if !self.extra_frontmatter.is_empty() {
            fm.push_str(&self.extra_frontmatter);
            if !self.extra_frontmatter.ends_with('\n') {
                fm.push('\n');
            }
        }
        fm.push_str("+++\n");
        if !self.body.is_empty() {
            fm.push_str(&self.body);
            if !self.body.ends_with('\n') {
                fm.push('\n');
            }
        }
        fm
    }
}

// --- Frontmatter helpers ---

fn extract_fm_value(content: &str, key: &str) -> Option<String> {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("+++") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("\n+++")?;
    let fm = &rest[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(after_key) = line.strip_prefix(key) {
            let after = after_key.trim_start();
            if let Some(val) = after.strip_prefix('=') {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_body(content: &str) -> String {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("+++") {
        return content.to_string();
    }
    let rest = &trimmed[3..];
    if let Some(end) = rest.find("\n+++") {
        let after = &rest[end + 4..];
        after.strip_prefix('\n').unwrap_or(after).to_string()
    } else {
        content.to_string()
    }
}

struct ParsedSectionFm {
    title: String,
    description: String,
    sort_by: String,
    paginate_by: String,
    extra_lines: String,
}

fn parse_section_fm(content: &str) -> ParsedSectionFm {
    let mut result = ParsedSectionFm {
        title: String::new(),
        description: String::new(),
        sort_by: String::new(),
        paginate_by: String::new(),
        extra_lines: String::new(),
    };

    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("+++") {
        return result;
    }
    let rest = &trimmed[3..];
    let Some(end) = rest.find("\n+++") else {
        return result;
    };
    let fm = &rest[..end];

    for line in fm.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(val) = strip_val(line, "title") {
            result.title = val;
        } else if let Some(val) = strip_val(line, "description") {
            result.description = val;
        } else if let Some(val) = strip_val(line, "sort_by") {
            result.sort_by = val;
        } else if let Some(val) = strip_val(line, "paginate_by") {
            result.paginate_by = val;
        } else {
            result.extra_lines.push_str(line);
            result.extra_lines.push('\n');
        }
    }

    result
}

fn strip_val(line: &str, key: &str) -> Option<String> {
    if !line.starts_with(key) {
        return None;
    }
    let after = line[key.len()..].trim_start();
    let val = after.strip_prefix('=')?.trim();
    Some(val.trim_matches('"').to_string())
}

// --- Editor rendering ---

fn render_section_editor(
    site_title: &str,
    path: &str,
    content: &str,
    flash: Option<(&str, &str)>,
    base_url: &str,
) -> String {
    let fm = parse_section_fm(content);
    let body_content = extract_body(content);

    let flash_html = flash
        .map(|(kind, msg)| {
            format!(
                r#"<div class="flash flash-{kind}">{msg}</div>"#,
                kind = escape(kind),
                msg = escape(msg)
            )
        })
        .unwrap_or_default();

    let sort_options = match fm.sort_by.as_str() {
        "title" => {
            r#"<option value="date">Date</option><option value="title" selected>Title</option>"#
        }
        _ => r#"<option value="date" selected>Date</option><option value="title">Title</option>"#,
    };

    let e_path = escape(path);
    let e_title = escape(&fm.title);
    let e_desc = escape(&fm.description);
    let e_paginate = escape(&fm.paginate_by);
    let e_body = escape(&body_content);
    let e_extra = escape(&fm.extra_lines);
    let display = if fm.title.is_empty() { path } else { &fm.title };

    let body = [
        &*flash_html,
        r#"<div class="toolbar">
  <h2>Edit Section: "#,
        &escape(display),
        r#"</h2>
  <div class="toolbar-right">
    <a href="/sections" class="btn">Back to Sections</a>
  </div>
</div>
<form method="POST" action="/sections/"#,
        &e_path,
        r#"">
  <div class="card" style="margin-bottom: 16px;">
    <div class="form-row">
      <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" value=""#,
        &e_title,
        r#"">
      </div>
      <div class="form-group">
        <label>Description</label>
        <input type="text" name="description" value=""#,
        &e_desc,
        r#"">
      </div>
    </div>
    <div class="form-row">
      <div class="form-group" style="max-width: 200px;">
        <label>Sort By</label>
        <select name="sort_by">"#,
        sort_options,
        r#"</select>
      </div>
      <div class="form-group" style="max-width: 200px;">
        <label>Paginate By</label>
        <input type="text" name="paginate_by" value=""#,
        &e_paginate,
        r#"" placeholder="e.g. 10">
      </div>
    </div>
    <input type="hidden" name="extra_frontmatter" value=""#,
        &e_extra,
        r##"">
  </div>
  <div class="editor-layout">
    <div class="editor-pane">
      <div class="form-group">
        <label>Content <span style="color: #666680; font-size: 0.7rem; text-transform: none;">(Markdown — optional body for section page)</span></label>
        <textarea name="body" rows="20" id="editor"
                  hx-post="/_render-markdown" hx-trigger="keyup changed delay:500ms"
                  hx-target="#preview" hx-swap="innerHTML">"##,
        &e_body,
        r##"</textarea>
      </div>
      <div style="display: flex; gap: 8px;">
        <button type="submit" class="btn btn-primary">Save &amp; Rebuild</button>
      </div>
    </div>
    <div class="editor-pane">
      <label>Preview</label>
      <div class="preview-panel" id="preview"
           style="overflow-y: auto; max-height: calc(20 * 1.5em + 24px);">
      </div>
    </div>
  </div>
</form>
<script>
document.addEventListener('keydown', function(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault();
    document.querySelector('form').submit();
  }
});
document.addEventListener('DOMContentLoaded', function() {
  var editor = document.getElementById('editor');
  if (editor && editor.value.trim()) {
    htmx.trigger(editor, 'keyup');
  }
});
</script>"##,
    ]
    .concat();

    html::page(
        &format!("Edit: {display}"),
        site_title,
        "sections",
        &body,
        base_url,
    )
}

use crate::rebuild_site;
