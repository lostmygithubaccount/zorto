//! Zorto webapp — HTMX-based local CMS for managing zorto sites.

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::{any, get, post};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

mod assets;
mod build;
mod config;
mod dashboard;
mod html;
mod onboarding;
mod pages;
mod sections;

const DEFAULT_WEBAPP_PORT: u16 = 1112;
const DEFAULT_PREVIEW_URL: &str = "http://localhost:1111";
const RELOAD_CHANNEL_CAPACITY: usize = 16;

/// Client-side livereload snippet. Opens a WebSocket to `/__livereload`
/// and reloads the page on each `"reload"` message. Reconnects with backoff
/// so that webapp restarts do not leave the page dead.
pub(crate) const LIVERELOAD_JS: &str = r#"
<script>
(function() {
    var reconnectInterval = 1000;
    var maxReconnect = 30000;
    function connect() {
        var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        var ws = new WebSocket(proto + '//' + location.host + '/__livereload');
        ws.onmessage = function(event) {
            if (event.data === 'reload') { location.reload(); }
        };
        ws.onclose = function() {
            setTimeout(connect, reconnectInterval);
            reconnectInterval = Math.min(reconnectInterval * 1.5, maxReconnect);
        };
        ws.onopen = function() { reconnectInterval = 1000; };
    }
    connect();
})();
</script>
"#;

pub(crate) struct AppState {
    pub root: PathBuf,
    pub output_dir: PathBuf,
    pub sandbox: Option<PathBuf>,
    pub reload_tx: broadcast::Sender<()>,
}

impl AppState {
    fn site_title(&self) -> String {
        let config_path = self.root.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<toml::Value>(&content) {
                if let Some(title) = config.get("title").and_then(|v| v.as_str()) {
                    return title.to_string();
                }
            }
        }
        "Zorto Site".to_string()
    }

    fn site_exists(&self) -> bool {
        self.root.join("config.toml").exists()
    }

    fn site_base_url(&self) -> String {
        let config_path = self.root.join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<toml::Value>(&content) {
                if let Some(url) = config.get("base_url").and_then(|v| v.as_str()) {
                    if !url.is_empty() {
                        return url.trim_end_matches('/').to_string();
                    }
                }
            }
        }
        DEFAULT_PREVIEW_URL.to_string()
    }
}

/// Build the axum router with the given shared state.
pub(crate) fn app(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(dashboard::index))
        .route("/pages", get(pages::list))
        .route("/pages/new", get(pages::new_form).post(pages::create))
        .route("/pages/{*path}", get(pages::edit).post(pages::save))
        .route("/pages/delete/{*path}", post(pages::delete))
        .route("/sections", get(sections::list))
        .route(
            "/sections/{*path}",
            get(sections::edit).post(sections::save),
        )
        .route("/config", get(config::edit).post(config::save))
        .route("/assets", get(assets::list))
        .route("/assets/upload", post(assets::upload))
        .route("/assets/delete", post(assets::delete))
        .route("/build", post(build::trigger))
        .route("/preview/render", post(build::render_preview))
        .route("/static/htmx.min.js", get(serve_htmx))
        .route("/__livereload", any(livereload_ws))
        // Onboarding wizard routes
        .route("/setup", get(onboarding::welcome))
        .route(
            "/setup/template",
            get(onboarding::template).post(onboarding::template_submit),
        )
        .route(
            "/setup/theme",
            get(onboarding::theme).post(onboarding::theme_submit),
        )
        .route("/setup/configure", get(onboarding::configure))
        .route("/setup/create", post(onboarding::create))
        .with_state(state)
}

/// Run the zorto webapp server.
///
/// Starts an HTMX-based CMS webapp for managing the site at the given root directory.
/// The optional `sandbox` path allows file operations (like include shortcodes) to
/// access files outside the site root within the sandbox boundary.
pub fn run_webapp(root: &Path, output_dir: &Path, sandbox: Option<&Path>) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let port: u16 = DEFAULT_WEBAPP_PORT;
        let (reload_tx, _) = broadcast::channel::<()>(RELOAD_CHANNEL_CAPACITY);

        let state = Arc::new(AppState {
            root: root.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            sandbox: sandbox.map(|p| p.to_path_buf()),
            reload_tx,
        });

        // Determine start page based on whether a site exists
        let start_path = if state.site_exists() { "/" } else { "/setup" };

        let app = app(state);

        // Bind to 0.0.0.0 so the webapp is accessible on LAN
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                eprintln!("Port {port} is in use, using a random available port...");
                let fallback = SocketAddr::from(([0, 0, 0, 0], 0));
                tokio::net::TcpListener::bind(fallback).await?
            }
            Err(e) => return Err(e.into()),
        };
        let actual_addr = listener.local_addr()?;

        println!("zorto webapp: http://localhost:{}", actual_addr.port());
        let _ = open::that(format!(
            "http://localhost:{}{}",
            actual_addr.port(),
            start_path
        ));

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
                println!("\nshutting down...");
            })
            .await?;

        Ok(())
    })
}

/// Serve the embedded HTMX library.
async fn serve_htmx() -> impl axum::response::IntoResponse {
    (
        [("content-type", "application/javascript")],
        include_str!("htmx.min.js"),
    )
}

/// WebSocket upgrade handler for live reload. Subscribes to
/// [`AppState::reload_tx`] and emits `"reload"` on each broadcast.
async fn livereload_ws(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| handle_livereload(socket, state))
}

async fn handle_livereload(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.reload_tx.subscribe();
    while rx.recv().await.is_ok() {
        if socket
            .send(Message::Text(String::from("reload").into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

pub(crate) fn rebuild_site(state: &AppState) -> Result<(), String> {
    match zorto_core::site::Site::load(&state.root, &state.output_dir, true) {
        Ok(mut site) => {
            site.sandbox = state.sandbox.clone();
            site.build().map_err(|e| e.to_string())?;
            let _ = state.reload_tx.send(());
            Ok(())
        }
        Err(e) => Err(e.to_string()),
    }
}

pub(crate) fn escape(s: &str) -> String {
    zorto_core::content::escape_html(s)
}

/// Validate that a user-supplied path, when joined to a base directory, stays
/// within that directory. Returns the canonical path on success, or an error
/// message suitable for display.
pub(crate) fn validate_path(base: &Path, user_path: &str) -> Result<PathBuf, String> {
    let joined = base.join(user_path);

    // Canonicalize base (must exist)
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("Base directory does not exist: {e}"))?;

    // For existence-checking operations, canonicalize the joined path.
    // For creation, canonicalize the parent and verify.
    let canonical = if joined.exists() {
        joined
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path: {e}"))?
    } else {
        // File doesn't exist yet (creation). Canonicalize the parent dir.
        let parent = joined.parent().ok_or("Invalid path")?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|e| format!("Parent directory does not exist: {e}"))?;
        canonical_parent.join(joined.file_name().ok_or("Invalid filename")?)
    };

    if !canonical.starts_with(&canonical_base) {
        return Err("Path traversal detected".to_string());
    }

    Ok(canonical)
}

#[cfg(test)]
mod integration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_path_normal() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        std::fs::write(base.join("file.txt"), "hello").unwrap();
        let result = validate_path(base, "file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_traversal_blocked() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("subdir");
        std::fs::create_dir_all(&base).unwrap();
        let result = validate_path(&base, "../../../etc/passwd");
        let err = result.unwrap_err();
        assert!(err.contains("Path traversal detected") || err.contains("does not exist"));
    }

    #[test]
    fn test_validate_path_dotdot_traversal() {
        let tmp = TempDir::new().unwrap();
        let parent = tmp.path();
        let base = parent.join("site");
        let outside = parent.join("secret");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("data.txt"), "secret").unwrap();
        let result = validate_path(&base, "../secret/data.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_path_new_file_in_base() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        // File doesn't exist yet, but parent does — should succeed
        let result = validate_path(base, "new_file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_subdirectory() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();
        let sub = base.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("file.txt"), "data").unwrap();
        let result = validate_path(base, "sub/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_nonexistent_base() {
        let result = validate_path(Path::new("/nonexistent/base/dir"), "file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_validate_path_symlink_escape() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("site");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "secret data").unwrap();
        // Create a symlink inside base pointing outside
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, base.join("escape")).unwrap();
            let result = validate_path(&base, "escape/secret.txt");
            assert!(result.is_err(), "symlink escape should be blocked");
        }
    }
}
