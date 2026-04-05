//! Asset management routes.

use axum::extract::{Multipart, State};
use axum::response::Html;
use std::sync::Arc;

use crate::html;
use crate::{AppState, escape, validate_path};

const DEFAULT_BASE_URL: &str = "http://localhost:1111";

pub async fn list(State(state): State<Arc<AppState>>) -> Html<String> {
    let site_title = state.site_title();
    let static_dir = state.root.join("static");

    let mut files: Vec<(String, String, u64)> = Vec::new(); // (relative_path, ext, size)
    if static_dir.exists() {
        for entry in walkdir::WalkDir::new(&static_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let relative = path
                .strip_prefix(&static_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

            let ext = path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();

            files.push((relative, ext, size));
        }
    }

    files.sort_by(|a, b| a.0.cmp(&b.0));

    let grid_items: String = files
        .iter()
        .map(|(path, ext, size)| {
            let is_image = matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp"
            );

            let thumb = if is_image {
                // For images served from the preview server
                format!(
                    r#"<div class="asset-thumb-placeholder" style="background: url('{base_url}/{path}') center/contain no-repeat #111118; border-radius: 4px; width: 100%; height: 100px;"></div>"#,
                    base_url = DEFAULT_BASE_URL,
                    path = escape(path),
                )
            } else {
                let icon = match ext.as_str() {
                    "css" | "scss" => "css",
                    "js" => "js",
                    "pdf" => "pdf",
                    "woff" | "woff2" | "ttf" | "otf" => "font",
                    "mp4" | "webm" => "vid",
                    _ => "file",
                };
                format!(
                    r#"<div class="asset-thumb-placeholder">{icon}</div>"#
                )
            };

            format!(
                r#"<div class="asset-card">
  <form method="POST" action="/assets/delete" onsubmit="return confirm('Delete {epath}?')">
    <input type="hidden" name="path" value="{epath}">
    <button type="submit" class="asset-delete" title="Delete">&times;</button>
  </form>
  {thumb}
  <div class="asset-name">{epath}</div>
  <div class="asset-size">{size}</div>
</div>"#,
                epath = escape(path),
                size = format_size(*size),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let file_count = files.len();

    let body = format!(
        r#"<h2>Assets <span style="color: #666680; font-size: 0.85rem; font-weight: normal;">({file_count} files)</span></h2>

<div class="card">
  <h3>Upload</h3>
  <form method="POST" action="/assets/upload" enctype="multipart/form-data" id="upload-form">
    <div class="drop-zone" id="drop-zone" style="margin-top: 12px;">
      <p>Drag &amp; drop files here or click to browse</p>
      <input type="file" name="file" id="file-input" style="display: none;" required>
      <input type="hidden" name="subdir" id="subdir-input" value="">
    </div>
    <div style="display: flex; gap: 8px; align-items: center; margin-top: 12px;">
      <input type="text" name="subdir_display" placeholder="subdirectory (optional)" style="width: 200px;"
             oninput="document.getElementById('subdir-input').value=this.value">
      <button type="submit" class="btn btn-primary" id="upload-btn" disabled>Upload</button>
      <span id="file-name" style="font-size: 0.8rem; color: #666680;"></span>
    </div>
  </form>
</div>

<div class="asset-grid" style="margin-top: 16px;">
  {grid_items}
</div>

<script>
(function() {{
  var dropZone = document.getElementById('drop-zone');
  var fileInput = document.getElementById('file-input');
  var fileName = document.getElementById('file-name');
  var uploadBtn = document.getElementById('upload-btn');
  var form = document.getElementById('upload-form');

  dropZone.addEventListener('click', function() {{ fileInput.click(); }});

  fileInput.addEventListener('change', function() {{
    if (fileInput.files.length > 0) {{
      fileName.textContent = fileInput.files[0].name;
      uploadBtn.disabled = false;
    }}
  }});

  dropZone.addEventListener('dragover', function(e) {{
    e.preventDefault();
    dropZone.classList.add('drag-over');
  }});

  dropZone.addEventListener('dragleave', function() {{
    dropZone.classList.remove('drag-over');
  }});

  dropZone.addEventListener('drop', function(e) {{
    e.preventDefault();
    dropZone.classList.remove('drag-over');
    if (e.dataTransfer.files.length > 0) {{
      fileInput.files = e.dataTransfer.files;
      fileName.textContent = e.dataTransfer.files[0].name;
      uploadBtn.disabled = false;
    }}
  }});
}})();
</script>"#
    );

    Html(html::page("Assets", &site_title, "assets", &body))
}

pub async fn upload(State(state): State<Arc<AppState>>, mut multipart: Multipart) -> Html<String> {
    let static_dir = state.root.join("static");
    let mut subdir = String::new();
    let mut file_saved = false;
    let mut error_msg = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        if name == "subdir" || name == "subdir_display" {
            let val = field.text().await.unwrap_or_default();
            if !val.is_empty() {
                subdir = val;
            }
        } else if name == "file" {
            let filename = field.file_name().unwrap_or("upload").to_string();

            // Reject suspicious filenames
            if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
                error_msg = Some("Invalid filename".to_string());
                continue;
            }

            let dest_dir = if subdir.is_empty() {
                static_dir.clone()
            } else {
                // Validate subdir doesn't escape static directory
                match validate_path(&static_dir, &subdir) {
                    Ok(p) => p,
                    Err(_) => {
                        error_msg = Some("Invalid subdirectory path".to_string());
                        continue;
                    }
                }
            };
            let _ = std::fs::create_dir_all(&dest_dir);

            match field.bytes().await {
                Ok(bytes) => {
                    let dest = dest_dir.join(&filename);
                    match std::fs::write(&dest, &bytes) {
                        Ok(()) => file_saved = true,
                        Err(e) => error_msg = Some(e.to_string()),
                    }
                }
                Err(e) => error_msg = Some(e.to_string()),
            }
        }
    }

    let flash = if file_saved {
        r#"<div class="flash flash-success">File uploaded.</div>"#.to_string()
    } else if let Some(err) = error_msg {
        format!(
            r#"<div class="flash flash-error">Upload error: {}</div>"#,
            escape(&err)
        )
    } else {
        r#"<div class="flash flash-error">No file received.</div>"#.to_string()
    };

    // Re-render the full asset list with flash
    let full = list(State(state))
        .await
        .0
        .replace("<h2>Assets", &format!("{flash}<h2>Assets"));

    Html(full)
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<DeleteForm>,
) -> Html<String> {
    let static_dir = state.root.join("static");
    let file_path = match validate_path(&static_dir, &form.path) {
        Ok(p) => p,
        Err(_) => {
            let full = list(State(state)).await.0.replace(
                "<h2>Assets",
                r#"<div class="flash flash-error">Invalid path.</div><h2>Assets"#,
            );
            return Html(full);
        }
    };
    if file_path.exists() {
        let _ = std::fs::remove_file(&file_path);
    }
    let full = list(State(state)).await.0.replace(
        "<h2>Assets",
        r#"<div class="flash flash-success">File deleted.</div><h2>Assets"#,
    );
    Html(full)
}

#[derive(serde::Deserialize)]
pub struct DeleteForm {
    path: String,
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
