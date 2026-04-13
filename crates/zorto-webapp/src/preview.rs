//! Embedded preview server. Serves built site output (`output_dir`) at
//! `/preview/*` so the webapp is self-contained — no separate `zorto preview`
//! process needed. HTML responses get the livereload script injected so the
//! same broadcast channel that powers CMS livereload also refreshes the
//! preview tab on rebuild.

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{AppState, LIVERELOAD_JS};

/// Handle a request under `/preview/*`. `request_path` is the tail after
/// the `/preview` prefix and may be empty, `/`, or any sub-path.
pub(crate) async fn serve(State(state): State<Arc<AppState>>, req: Request<Body>) -> Response {
    let full = req.uri().path();
    let tail = full.strip_prefix("/preview").unwrap_or(full);
    let file_path = match resolve_path(&state.output_dir, tail) {
        Some(p) => p,
        None => return not_found(&state.output_dir).await,
    };

    if !file_path.exists() {
        return not_found(&state.output_dir).await;
    }

    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext == "html" {
        match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => (
                [(header::CONTENT_TYPE, "text/html")],
                inject_livereload(&content),
            )
                .into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Read error").into_response(),
        }
    } else {
        let content_type = content_type_for(ext);
        match tokio::fs::read(&file_path).await {
            Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Read error").into_response(),
        }
    }
}

fn content_type_for(ext: &str) -> &'static str {
    match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "pdf" => "application/pdf",
        "xml" => "application/xml",
        "txt" | "sh" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Resolve a request path under `/preview` to a file inside `output_dir`.
/// Returns `None` on a traversal attempt or when the canonical path escapes
/// `output_dir` (e.g. via a symlink).
pub(crate) fn resolve_path(output_dir: &Path, tail: &str) -> Option<PathBuf> {
    let stripped = tail.trim_start_matches('/');
    if stripped.is_empty() {
        return Some(output_dir.join("index.html"));
    }

    if stripped.split('/').any(|seg| seg == ".." || seg == ".") {
        return None;
    }

    let candidate = output_dir.join(stripped);

    if candidate.exists() {
        let canonical = candidate.canonicalize().ok()?;
        let canonical_root = output_dir.canonicalize().ok()?;
        if !canonical.starts_with(&canonical_root) {
            return None;
        }
    }

    if candidate.is_dir() {
        Some(candidate.join("index.html"))
    } else if candidate.exists() {
        Some(candidate)
    } else {
        let with_index = candidate.join("index.html");
        with_index.exists().then_some(with_index)
    }
}

async fn not_found(output_dir: &Path) -> Response {
    let not_found = output_dir.join("404.html");
    if not_found.exists() {
        let content = tokio::fs::read_to_string(&not_found)
            .await
            .unwrap_or_default();
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html")],
            inject_livereload(&content),
        )
            .into_response();
    }
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/html")],
        format!(
            "<!doctype html><meta charset=\"utf-8\"><title>Not Found</title>\
            <p>No preview available yet. Build the site to populate <code>{}</code>.</p>{}",
            output_dir.display(),
            LIVERELOAD_JS
        ),
    )
        .into_response()
}

fn inject_livereload(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + LIVERELOAD_JS.len());
        out.push_str(&html[..pos]);
        out.push_str(LIVERELOAD_JS);
        out.push_str(&html[pos..]);
        out
    } else {
        format!("{html}{LIVERELOAD_JS}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn resolve_path_root_returns_index() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("index.html"), "<html></html>").unwrap();
        assert_eq!(resolve_path(&out, "").unwrap(), out.join("index.html"));
        assert_eq!(resolve_path(&out, "/").unwrap(), out.join("index.html"));
    }

    #[test]
    fn resolve_path_normal_file() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("style.css"), "body{}").unwrap();
        assert_eq!(
            resolve_path(&out, "/style.css").unwrap(),
            out.join("style.css")
        );
    }

    #[test]
    fn resolve_path_blocks_traversal() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        assert!(resolve_path(&out, "/../../../etc/passwd").is_none());
        assert!(resolve_path(&out, "/..").is_none());
        assert!(resolve_path(&out, "/foo/../../..").is_none());
        assert!(resolve_path(&out, "/./secret").is_none());
    }

    #[test]
    fn resolve_path_dir_returns_index_html() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        let sub = out.join("posts");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.html"), "posts").unwrap();
        assert_eq!(
            resolve_path(&out, "/posts").unwrap(),
            sub.join("index.html")
        );
    }

    #[test]
    fn resolve_path_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        assert!(resolve_path(&out, "/nope.html").is_none());
    }

    #[test]
    fn inject_livereload_before_body_close() {
        let html = "<html><body>hi</body></html>";
        let out = inject_livereload(html);
        assert!(out.contains("__livereload"));
        assert!(out.contains("hi"));
        // JS appears before </body>
        let body_close = out.find("</body>").unwrap();
        let script_pos = out.find("__livereload").unwrap();
        assert!(script_pos < body_close);
    }

    #[test]
    fn inject_livereload_appends_when_no_body_tag() {
        let html = "<p>no body</p>";
        let out = inject_livereload(html);
        assert!(out.starts_with("<p>no body</p>"));
        assert!(out.contains("__livereload"));
    }
}
