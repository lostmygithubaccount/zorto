//! Page listing and editing routes.

use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse, Redirect};
use std::sync::Arc;

use crate::html;
use crate::{AppState, escape, validate_path};

pub async fn list(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ListQuery>,
) -> Html<String> {
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
            if name == "_index.md" || !name.ends_with(".md") {
                continue;
            }

            let relative = path
                .strip_prefix(&content_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let content = std::fs::read_to_string(path).unwrap_or_default();
            let fm = ParsedFrontmatter::parse(&content);
            let title = fm.title.clone().unwrap_or_else(|| relative.clone());

            let draft_badge = if fm.draft {
                r#" <span class="badge badge-draft">draft</span>"#
            } else {
                ""
            };

            let date_col = fm
                .date
                .as_deref()
                .map(|d| {
                    format!(
                        r#"<td style="color: #666680; font-size: 0.85rem;">{}</td>"#,
                        escape(d)
                    )
                })
                .unwrap_or_else(|| r#"<td style="color: #444;"></td>"#.to_string());

            rows.push(format!(
                r#"<tr>
  <td><a href="/pages/{path}">{title}</a>{draft_badge}</td>
  {date_col}
  <td style="color: #666680; font-family: monospace; font-size: 0.8rem;">{path}</td>
</tr>"#,
                path = escape(&relative),
                title = escape(&title),
            ));
        }
    }

    rows.sort();

    // Flash message from page creation
    let flash_html = if params.created.is_some() {
        r#"<div class="flash flash-success">Page created and site rebuilt.</div>"#
    } else {
        ""
    };

    let table_html = if rows.is_empty() {
        r#"<div class="empty-state">
  <p>No pages yet — create one!</p>
  <a href="/pages/new" class="btn btn-primary">New Page</a>
</div>"#
            .to_string()
    } else {
        let table_body = rows.join("\n");
        format!(
            r#"<div class="card">
  <table>
    <thead>
      <tr><th>Title</th><th>Date</th><th>Path</th></tr>
    </thead>
    <tbody>
      {table_body}
    </tbody>
  </table>
</div>"#
        )
    };

    let body = format!(
        r#"{flash_html}<div class="toolbar">
  <h2>Pages</h2>
  <div class="toolbar-right">
    <a href="/pages/new" class="btn btn-primary">New Page</a>
  </div>
</div>
{table_html}"#
    );

    Html(html::page("Pages", &site_title, "pages", &body, &base_url))
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    created: Option<String>,
}

pub async fn edit(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    axum::extract::Query(params): axum::extract::Query<EditQuery>,
) -> Html<String> {
    let site_title = state.site_title();
    let base_url = state.site_base_url();
    let content_dir = state.root.join("content");
    let file_path = match validate_path(&content_dir, &path) {
        Ok(p) => p,
        Err(_) => {
            return Html(html::page(
                "Error",
                &site_title,
                "pages",
                "<p>Invalid path.</p>",
                &base_url,
            ));
        }
    };
    let content = std::fs::read_to_string(&file_path).unwrap_or_default();

    let flash = if params.created.is_some() {
        Some(("success", "Page created and site rebuilt."))
    } else {
        None
    };

    Html(render_editor(
        &site_title,
        &path,
        &content,
        flash,
        &base_url,
    ))
}

#[derive(serde::Deserialize)]
pub struct EditQuery {
    #[serde(default)]
    created: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct NewFormQuery {
    /// Slug of a section to pre-select in the Section dropdown. Carried in
    /// from `/sections/new?then=page` after an inline section create.
    #[serde(default)]
    preselect: Option<String>,
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
                "pages",
                "<p>Invalid path.</p>",
                &base_url,
            ));
        }
    };

    // Reconstruct the file from form fields
    let new_content = form.to_file_content();
    let result = std::fs::write(&file_path, &new_content);

    let flash_msg: Option<(String, String)> = match result {
        Ok(()) => match rebuild_site(&state) {
            Ok(()) => Some(("success".into(), "Page saved and site rebuilt.".into())),
            Err(e) => Some(("error".into(), format!("Saved but build failed: {e}"))),
        },
        Err(e) => Some(("error".into(), format!("Error saving: {e}"))),
    };
    let flash = flash_msg.as_ref().map(|(k, v)| (k.as_str(), v.as_str()));

    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    Html(render_editor(
        &site_title,
        &path,
        &content,
        flash,
        &base_url,
    ))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> axum::response::Response {
    let content_dir = state.root.join("content");
    let file_path = match validate_path(&content_dir, &path) {
        Ok(p) => p,
        Err(_) => return Redirect::to("/pages").into_response(),
    };
    if file_path.exists() {
        if let Err(e) = std::fs::remove_file(&file_path) {
            eprintln!("Error deleting page {path}: {e}");
        } else if let Err(e) = rebuild_site(&state) {
            eprintln!("Page deleted but site rebuild failed: {e}");
        }
    }
    Redirect::to("/pages").into_response()
}

pub async fn new_form(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<NewFormQuery>,
) -> Html<String> {
    let site_title = state.site_title();
    let base_url = state.site_base_url();

    // Detect available sections for the dropdown
    let content_dir = state.root.join("content");
    let mut sections = vec![String::new()];
    if content_dir.exists() {
        for entry in walkdir::WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == "_index.md" {
                let rel = entry
                    .path()
                    .parent()
                    .and_then(|p| p.strip_prefix(&content_dir).ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !rel.is_empty() && !sections.contains(&rel) {
                    sections.push(rel);
                }
            }
        }
    }
    sections.sort();

    // `?preselect=<slug>` comes from the inline "+ New section" flow: the user
    // just created a section from this form and we want the dropdown to
    // remember it. Falls back to `posts` (the default scaffold has one).
    let preselect = params.preselect.as_deref().unwrap_or("posts");

    let flash_html = if params.preselect.is_some() {
        r#"<div class="flash flash-success">Section created — selected below.</div>"#
    } else {
        ""
    };

    let section_options: String = sections
        .iter()
        .map(|s| {
            let label = if s.is_empty() { "(root)" } else { s.as_str() };
            let selected = if s == preselect { " selected" } else { "" };
            format!(
                r#"<option value="{s}"{selected}>{label}</option>"#,
                s = escape(s),
                label = escape(label)
            )
        })
        .collect::<Vec<_>>()
        .join("\n          ");

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let body = format!(
        r#"{flash_html}<div class="toolbar">
  <h2>New Page</h2>
  <div class="toolbar-right">
    <a href="/pages" class="btn">Back to Pages</a>
  </div>
</div>
<form method="POST" action="/pages/new">
  <div class="card">
    <div class="form-row">
      <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" placeholder="My New Page" required>
      </div>
      <div class="form-group">
        <label>Section <a href="/sections/new?then=page" style="font-weight: normal; text-transform: none; font-size: 0.75rem; margin-left: 8px;">+ new section</a></label>
        <select name="section">
          {section_options}
        </select>
      </div>
    </div>
    <div class="form-row">
      <div class="form-group">
        <label>Date</label>
        <input type="date" name="date" value="{today}">
      </div>
      <div class="form-group">
        <label>Status</label>
        <select name="draft">
          <option value="false">Published</option>
          <option value="true">Draft</option>
        </select>
      </div>
    </div>
    <div class="form-group">
      <label>Description</label>
      <input type="text" name="description" placeholder="A brief description">
    </div>
    <div class="form-group">
      <label>Tags (comma-separated)</label>
      <input type="text" name="tags" placeholder="news, update">
    </div>
    <div class="form-group">
      <label>Content</label>
      <textarea name="body" rows="20" placeholder="Write your content here..." id="editor"></textarea>
    </div>
    <button type="submit" class="btn btn-primary">Create Page</button>
  </div>
</form>
<script>
document.getElementById('editor').addEventListener('keydown', function(e) {{
  if (e.key === 'Tab') {{
    e.preventDefault();
    var start = this.selectionStart;
    var end = this.selectionEnd;
    this.value = this.value.substring(0, start) + '  ' + this.value.substring(end);
    this.selectionStart = this.selectionEnd = start + 2;
  }}
}});
</script>"#
    );

    Html(html::page(
        "New Page",
        &site_title,
        "pages",
        &body,
        &base_url,
    ))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<NewPageForm>,
) -> axum::response::Response {
    // Validate required fields
    if form.title.trim().is_empty() {
        let site_title = state.site_title();
        let base_url = state.site_base_url();
        let body = r#"<div class="flash flash-error">Title is required.</div>
<div class="toolbar">
  <h2>New Page</h2>
  <div class="toolbar-right">
    <a href="/pages" class="btn">Back to Pages</a>
  </div>
</div>
<p style="color: #8c8ca6;">Please go back and fill in the title field.</p>"#;
        return Html(html::page(
            "New Page",
            &site_title,
            "pages",
            body,
            &base_url,
        ))
        .into_response();
    }

    let slug = slug::slugify(&form.title);
    let section = form.section.clone();
    let content_dir = state.root.join("content");

    // Validate section path doesn't escape content dir
    if !section.is_empty() && validate_path(&content_dir, &section).is_err() {
        return Redirect::to("/pages").into_response();
    }

    let file_dir = if section.is_empty() {
        content_dir.clone()
    } else {
        content_dir.join(&section)
    };
    let _ = std::fs::create_dir_all(&file_dir);

    let relative = if section.is_empty() {
        format!("{slug}.md")
    } else {
        format!("{section}/{slug}.md")
    };

    let mut table = toml::map::Map::new();
    table.insert("title".into(), toml::Value::String(form.title.clone()));
    if !form.date.is_empty() {
        table.insert("date".into(), toml::Value::String(form.date.clone()));
    }
    if !form.description.is_empty() {
        table.insert(
            "description".into(),
            toml::Value::String(form.description.clone()),
        );
    }
    if form.draft == "true" {
        table.insert("draft".into(), toml::Value::Boolean(true));
    }
    let tags: Vec<&str> = form
        .tags
        .split(',')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .collect();
    if !tags.is_empty() {
        let tag_vals: Vec<toml::Value> = tags
            .iter()
            .map(|t| toml::Value::String(t.to_string()))
            .collect();
        table.insert("tags".into(), toml::Value::Array(tag_vals));
    }
    let fm_toml = toml::to_string(&toml::Value::Table(table)).unwrap_or_default();
    let mut fm = format!("+++\n{fm_toml}+++\n");

    if !form.body.is_empty() {
        fm.push_str(&form.body);
        if !form.body.ends_with('\n') {
            fm.push('\n');
        }
    }

    let file_path = file_dir.join(format!("{slug}.md"));
    if let Err(e) = std::fs::write(&file_path, &fm) {
        eprintln!("Error creating page: {e}");
    } else if let Err(e) = rebuild_site(&state) {
        eprintln!("Page created but site rebuild failed: {e}");
    }

    Redirect::to(&format!("/pages/{relative}?created=1")).into_response()
}

// --- Form types ---

#[derive(serde::Deserialize)]
pub struct SaveForm {
    title: String,
    date: String,
    description: String,
    draft: String,
    tags: String,
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
        if !self.date.is_empty() {
            table.insert("date".into(), toml::Value::String(self.date.clone()));
        }
        if !self.description.is_empty() {
            table.insert(
                "description".into(),
                toml::Value::String(self.description.clone()),
            );
        }
        if self.draft == "true" {
            table.insert("draft".into(), toml::Value::Boolean(true));
        }
        let tags: Vec<&str> = self
            .tags
            .split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if !tags.is_empty() {
            let tag_vals: Vec<toml::Value> = tags
                .iter()
                .map(|t| toml::Value::String(t.to_string()))
                .collect();
            table.insert("tags".into(), toml::Value::Array(tag_vals));
        }
        let fm_toml = toml::to_string(&toml::Value::Table(table)).unwrap_or_default();
        let mut fm = format!("+++\n{fm_toml}");
        // Preserve any extra frontmatter lines we didn't parse into fields
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

#[derive(serde::Deserialize)]
pub struct NewPageForm {
    title: String,
    section: String,
    date: String,
    description: String,
    draft: String,
    tags: String,
    body: String,
}

// --- Frontmatter parsing ---

struct ParsedFrontmatter {
    title: Option<String>,
    date: Option<String>,
    description: Option<String>,
    draft: bool,
    tags: Vec<String>,
    /// Lines of frontmatter we don't have dedicated fields for (extra, sort_by, etc.)
    extra_lines: String,
}

impl ParsedFrontmatter {
    fn parse(content: &str) -> Self {
        let mut fm = ParsedFrontmatter {
            title: None,
            date: None,
            description: None,
            draft: false,
            tags: Vec::new(),
            extra_lines: String::new(),
        };

        let trimmed = content.trim_start_matches('\u{feff}');
        if !trimmed.starts_with("+++") {
            return fm;
        }

        let rest = &trimmed[3..];
        let Some(end) = rest.find("\n+++") else {
            return fm;
        };
        let fm_str = &rest[..end];

        for line in fm_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(val) = strip_toml_string(line, "title") {
                fm.title = Some(val);
            } else if let Some(val) = strip_toml_string(line, "date") {
                fm.date = Some(val);
            } else if let Some(val) = strip_toml_string(line, "description") {
                fm.description = Some(val);
            } else if line.starts_with("draft") && line.contains("true") {
                fm.draft = true;
            } else if line.starts_with("tags") {
                // Parse tags = ["a", "b"]
                if let Some(arr_start) = line.find('[') {
                    let arr = &line[arr_start..];
                    fm.tags = arr
                        .trim_matches(|c| c == '[' || c == ']')
                        .split(',')
                        .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|t| !t.is_empty())
                        .collect();
                }
            } else {
                fm.extra_lines.push_str(line);
                fm.extra_lines.push('\n');
            }
        }

        fm
    }
}

fn strip_toml_string(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with(key) {
        return None;
    }
    let after_key = line[key.len()..].trim_start();
    let after_eq = after_key.strip_prefix('=')?;
    let val = after_eq.trim();
    // Handle quoted and unquoted values
    Some(val.trim_matches('"').to_string())
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

// --- Editor rendering ---

fn render_editor(
    site_title: &str,
    path: &str,
    content: &str,
    flash: Option<(&str, &str)>,
    base_url: &str,
) -> String {
    let fm = ParsedFrontmatter::parse(content);
    let body_content = extract_body(content);
    let title = fm.title.clone().unwrap_or_default();
    let date = fm.date.clone().unwrap_or_default();
    let description = fm.description.clone().unwrap_or_default();
    let tags_str = fm.tags.join(", ");

    let flash_html = flash
        .map(|(kind, msg)| {
            format!(
                r#"<div class="flash flash-{kind}">{msg}</div>"#,
                kind = escape(kind),
                msg = escape(msg)
            )
        })
        .unwrap_or_default();

    let draft_options = if fm.draft {
        r#"<option value="false">Published</option><option value="true" selected>Draft</option>"#
    } else {
        r#"<option value="false" selected>Published</option><option value="true">Draft</option>"#
    };

    let display_title = escape(if title.is_empty() { path } else { &title });
    let e_path = escape(path);
    let e_title = escape(&title);
    let e_date = escape(&date);
    let e_description = escape(&description);
    let e_tags = escape(&tags_str);
    let e_body = escape(&body_content);
    let e_extra = escape(&fm.extra_lines);

    let body = [
        &*flash_html,
        r#"<div class="toolbar">
  <h2>Edit: "#, &display_title, r#"</h2>
  <div class="toolbar-right">
    <a href="/pages" class="btn">Back to Pages</a>
    <button type="button" class="btn" style="color: #f87171; border-color: #5c2a2a;" onclick="document.getElementById('delete-dialog').style.display='flex'">Delete</button>
    <div id="delete-dialog" class="confirm-overlay" style="display:none;" onclick="if(event.target===this)this.style.display='none'">
      <div class="confirm-dialog">
        <h3>Delete this page?</h3>
        <p>This will permanently remove the file. This action cannot be undone.</p>
        <div class="confirm-actions">
          <button type="button" class="btn" onclick="document.getElementById('delete-dialog').style.display='none'">Cancel</button>
          <form method="POST" action="/pages/delete/"#, &e_path, r#"" style="display:inline;">
            <button type="submit" class="btn btn-danger">Delete Page</button>
          </form>
        </div>
      </div>
    </div>
  </div>
</div>
<form method="POST" action="/pages/"#, &e_path, r#"">
  <div class="card" style="margin-bottom: 16px;">
    <div class="form-row">
      <div class="form-group">
        <label>Title</label>
        <input type="text" name="title" value=""#, &e_title, r#"">
      </div>
      <div class="form-group">
        <label>Date</label>
        <input type="text" name="date" value=""#, &e_date, r#"" placeholder="YYYY-MM-DD">
      </div>
    </div>
    <div class="form-row">
      <div class="form-group">
        <label>Description</label>
        <input type="text" name="description" value=""#, &e_description, r#"">
      </div>
      <div class="form-group" style="max-width: 160px;">
        <label>Status</label>
        <select name="draft">"#, draft_options, r#"</select>
      </div>
    </div>
    <div class="form-group">
      <label>Tags (comma-separated)</label>
      <input type="text" name="tags" value=""#, &e_tags, r#"" placeholder="tag1, tag2">
    </div>
    <input type="hidden" name="extra_frontmatter" value=""#, &e_extra, r##"">
  </div>
  <div class="editor-layout">
    <div class="editor-pane">
      <div class="form-group">
        <label>Content <span style="color: #666680; font-size: 0.7rem; text-transform: none;">(Markdown)</span></label>
        <textarea name="body" rows="28" id="editor"
                  hx-post="/_render-markdown" hx-trigger="keyup changed delay:500ms"
                  hx-target="#preview" hx-swap="innerHTML">"##, &e_body, r##"</textarea>
      </div>
      <div style="display: flex; gap: 8px;">
        <button type="submit" class="btn btn-primary">Save &amp; Rebuild</button>
        <span style="font-size: 0.8rem; color: #666680; line-height: 2.2;" id="save-hint">ctrl+s to save</span>
      </div>
    </div>
    <div class="editor-pane">
      <label>Preview</label>
      <div class="preview-panel" id="preview"
           style="overflow-y: auto; max-height: calc(28 * 1.5em + 24px);">
        <div style="color: #666680; font-style: italic;">Start typing to see preview...</div>
      </div>
    </div>
  </div>
</form>
<script>
document.addEventListener('keydown', function(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault();
    document.querySelector('form[action^="/pages/"]').submit();
  }
});
document.addEventListener('DOMContentLoaded', function() {
  var editor = document.getElementById('editor');
  if (editor && editor.value.trim()) {
    htmx.trigger(editor, 'keyup');
  }
  if (editor) {
    editor.addEventListener('keydown', function(e) {
      if (e.key === 'Tab') {
        e.preventDefault();
        var start = this.selectionStart;
        var end = this.selectionEnd;
        this.value = this.value.substring(0, start) + '  ' + this.value.substring(end);
        this.selectionStart = this.selectionEnd = start + 2;
      }
    });
  }
});
</script>"##,
    ].concat();

    html::page(
        &format!("Edit: {}", if title.is_empty() { path } else { &title }),
        site_title,
        "pages",
        &body,
        base_url,
    )
}

use crate::rebuild_site;
