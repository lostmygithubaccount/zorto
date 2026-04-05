//! Dashboard page.

use axum::extract::State;
use axum::response::Html;
use std::sync::Arc;

use crate::AppState;
use crate::html;

#[derive(serde::Deserialize)]
pub struct DashboardQuery {
    #[serde(default)]
    welcome: Option<String>,
}

pub async fn index(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<DashboardQuery>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    // If no site exists, redirect to onboarding
    if !state.site_exists() {
        return axum::response::Redirect::to("/setup").into_response();
    }

    let site_title = state.site_title();
    let content_dir = state.root.join("content");

    let mut page_count = 0;
    let mut section_count = 0;
    let mut draft_count = 0;
    let mut recent_pages: Vec<(String, String, String)> = Vec::new(); // (title, path, date)

    if content_dir.exists() {
        for entry in walkdir::WalkDir::new(&content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name == "_index.md" {
                    section_count += 1;
                } else if name.ends_with(".md") {
                    page_count += 1;
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    if content.contains("draft = true") {
                        draft_count += 1;
                    }
                    let relative = path
                        .strip_prefix(&content_dir)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();
                    let title = extract_title(&content).unwrap_or_else(|| relative.clone());
                    let date = extract_date(&content).unwrap_or_default();
                    recent_pages.push((title, relative, date));
                }
            }
        }
    }

    // Sort recent pages by date descending
    recent_pages.sort_by(|a, b| b.2.cmp(&a.2));
    recent_pages.truncate(8);

    let static_dir = state.root.join("static");
    let asset_count = if static_dir.exists() {
        walkdir::WalkDir::new(&static_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .count()
    } else {
        0
    };

    let base_url = state.site_base_url();

    let welcome_html = if params.welcome.is_some() {
        r#"<div class="flash flash-success">Site created successfully! Start adding content or customize your config.</div>"#
    } else {
        ""
    };

    let recent_html: String = if recent_pages.is_empty() {
        r#"<p style="color: #666680; font-size: 0.85rem;">No pages yet. <a href="/pages/new">Create your first page</a>.</p>"#.to_string()
    } else {
        let items: String = recent_pages
            .iter()
            .map(|(title, path, date)| {
                let date_html = if date.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<span style="color: #666680; font-size: 0.8rem;">{}</span>"#,
                        crate::escape(date)
                    )
                };
                format!(
                    r#"<li><a href="/pages/{path}">{title}</a>{date_html}</li>"#,
                    path = crate::escape(path),
                    title = crate::escape(title),
                )
            })
            .collect::<Vec<_>>()
            .join("\n    ");
        format!(r#"<ul class="recent-list">{items}</ul>"#)
    };

    let body = format!(
        r#"{welcome_html}<h2>Dashboard</h2>
<div class="card">
  <div class="stat">
    <div class="stat-num">{page_count}</div>
    <div class="stat-label">Pages</div>
  </div>
  <div class="stat">
    <div class="stat-num">{section_count}</div>
    <div class="stat-label">Sections</div>
  </div>
  <div class="stat">
    <div class="stat-num">{draft_count}</div>
    <div class="stat-label">Drafts</div>
  </div>
  <div class="stat">
    <div class="stat-num">{asset_count}</div>
    <div class="stat-label">Assets</div>
  </div>
</div>

<div class="card">
  <h3>Quick Actions</h3>
  <div style="display: flex; gap: 8px; margin-top: 12px; flex-wrap: wrap;">
    <a href="/pages/new" class="btn btn-primary">New Page</a>
    <a href="/config" class="btn">Edit Config</a>
    <a href="{base_url}" target="_blank" class="btn btn-success">View Site</a>
  </div>
</div>

<div class="card">
  <h3>Recent Pages</h3>
  <div style="margin-top: 12px;">
    {recent_html}
  </div>
</div>"#,
        base_url = crate::escape(&base_url),
    );

    Html(html::page(
        "Dashboard",
        &site_title,
        "dashboard",
        &body,
        &base_url,
    ))
    .into_response()
}

fn extract_title(content: &str) -> Option<String> {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("+++") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("\n+++")?;
    let fm = &rest[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(after) = line.strip_prefix("title") {
            let after = after.trim_start();
            if let Some(val) = after.strip_prefix('=') {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}

fn extract_date(content: &str) -> Option<String> {
    let trimmed = content.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("+++") {
        return None;
    }
    let rest = &trimmed[3..];
    let end = rest.find("\n+++")?;
    let fm = &rest[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(after) = line.strip_prefix("date") {
            let after = after.trim_start();
            if let Some(val) = after.strip_prefix('=') {
                return Some(val.trim().trim_matches('"').to_string());
            }
        }
    }
    None
}
