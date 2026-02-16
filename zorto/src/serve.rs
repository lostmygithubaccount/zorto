use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::broadcast;

const LIVERELOAD_JS: &str = "
<script>
(function() {
    var ws = new WebSocket('ws://' + location.host + '/__livereload');
    ws.onmessage = function(event) {
        if (event.data === 'reload') {
            location.reload();
        }
    };
    ws.onclose = function() {
        setTimeout(function() { location.reload(); }, 1000);
    };
})();
</script>
";

#[derive(Clone)]
struct AppState {
    reload_tx: broadcast::Sender<()>,
    output_dir: PathBuf,
}

/// Configuration for the preview server.
pub struct ServeConfig<'a> {
    pub root: &'a Path,
    pub output_dir: &'a Path,
    pub drafts: bool,
    pub no_exec: bool,
    pub sandbox: Option<&'a Path>,
    pub interface: &'a str,
    pub port: u16,
    pub open_browser: bool,
}

pub async fn serve(cfg: &ServeConfig<'_>) -> anyhow::Result<()> {
    // Bind listener first so we know the actual port before building
    let requested: SocketAddr = format!("{}:{}", cfg.interface, cfg.port).parse()?;
    let listener = match tokio::net::TcpListener::bind(requested).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "Port {} is in use, using a random available port...",
                cfg.port
            );
            let fallback: SocketAddr = format!("{}:0", cfg.interface).parse()?;
            tokio::net::TcpListener::bind(fallback).await?
        }
        Err(e) => return Err(e.into()),
    };
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");

    // Initial build
    println!("Building site...");
    let mut site = crate::site::Site::load(cfg.root, cfg.output_dir, cfg.drafts)?;
    site.no_exec = cfg.no_exec;
    site.sandbox = cfg.sandbox.map(|p| p.to_path_buf());
    site.set_base_url(base_url.clone());
    site.build()?;
    println!("Site built successfully.");

    // Set up broadcast channel for live reload
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = AppState {
        reload_tx: reload_tx.clone(),
        output_dir: cfg.output_dir.to_path_buf(),
    };

    let app = Router::new()
        .route("/__livereload", get(ws_handler))
        .fallback(get(serve_file).head(serve_file))
        .with_state(state);

    println!("Serving at http://{addr}");

    if cfg.open_browser {
        let url = format!("http://{addr}");
        let _ = open::that(&url);
    }

    // Bridge notify events into a tokio channel so the watcher loop is fully async
    let (watch_tx, watch_rx) = tokio::sync::mpsc::channel(16);
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(300), notify_tx)?;
    let watch_dirs = ["content", "templates", "sass", "static"];
    for dir in &watch_dirs {
        let path = cfg.root.join(dir);
        if path.exists() {
            debouncer
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive)?;
        }
    }
    let config_path = cfg.root.join("config.toml");
    if config_path.exists() {
        debouncer
            .watcher()
            .watch(&config_path, notify::RecursiveMode::NonRecursive)?;
    }

    // Blocking thread to bridge std::sync::mpsc -> tokio::sync::mpsc
    std::thread::spawn(move || {
        while let Ok(event) = notify_rx.recv() {
            if watch_tx.blocking_send(event).is_err() {
                break; // receiver dropped, shutting down
            }
        }
    });

    // Spawn the async watcher
    let rebuild_cfg = RebuildConfig {
        root: cfg.root.to_path_buf(),
        output: cfg.output_dir.to_path_buf(),
        drafts: cfg.drafts,
        no_exec: cfg.no_exec,
        sandbox: cfg.sandbox.map(|p| p.to_path_buf()),
        base_url,
    };
    let watcher_handle = tokio::spawn(async move {
        watch_and_rebuild(rebuild_cfg, reload_tx, watch_rx).await;
    });

    // Start server — ctrl+c cancels everything
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\nShutting down...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;

    // Server stopped — abort the watcher and let the bridge thread exit
    watcher_handle.abort();
    // Drop debouncer to close notify_tx, which unblocks the bridge thread
    drop(debouncer);

    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.reload_tx.subscribe();

    while let Ok(()) = rx.recv().await {
        if socket
            .send(Message::Text(String::from("reload").into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

async fn serve_file(State(state): State<AppState>, req: Request<Body>) -> Response {
    let path = req.uri().path();
    let output_dir = &state.output_dir;

    // Resolve the requested file path, guarding against directory traversal.
    let file_path = match resolve_serve_path(output_dir, path) {
        Some(p) => p,
        None => return serve_404(output_dir).await,
    };

    if !file_path.exists() {
        return serve_404(output_dir).await;
    }

    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let content_type = match ext {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "pdf" => "application/pdf",
        "xml" => "application/xml",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    };

    if ext == "html" {
        let content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Read error").into_response(),
        };
        let content = inject_livereload(&content);
        ([(header::CONTENT_TYPE, "text/html")], content).into_response()
    } else {
        match tokio::fs::read(&file_path).await {
            Ok(bytes) => ([(header::CONTENT_TYPE, content_type)], bytes).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Read error").into_response(),
        }
    }
}

/// Resolve a request path to a file inside `output_dir`, returning `None` if the
/// resolved path escapes the output directory (directory traversal guard).
fn resolve_serve_path(output_dir: &Path, request_path: &str) -> Option<PathBuf> {
    if request_path == "/" {
        return Some(output_dir.join("index.html"));
    }

    let stripped = request_path.trim_start_matches('/');
    // Percent-decode is already handled by axum/hyper, but reject obviously
    // suspicious components to be safe.
    if stripped.split('/').any(|seg| seg == ".." || seg == ".") {
        return None;
    }

    let candidate = output_dir.join(stripped);

    // Verify the resolved path stays within output_dir. This catches symlink
    // escapes and any edge cases the component check above might miss.
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
        if with_index.exists() {
            Some(with_index)
        } else {
            None
        }
    }
}

/// Serve a 404 response, using the custom 404.html template if available.
async fn serve_404(output_dir: &Path) -> Response {
    let not_found = output_dir.join("404.html");
    if not_found.exists() {
        let content = tokio::fs::read_to_string(&not_found)
            .await
            .unwrap_or_default();
        let content = inject_livereload(&content);
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/html")],
            content,
        )
            .into_response();
    }
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn inject_livereload(html: &str) -> String {
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + LIVERELOAD_JS.len());
        result.push_str(&html[..pos]);
        result.push_str(LIVERELOAD_JS);
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{html}{LIVERELOAD_JS}")
    }
}

/// Owned configuration for the file watcher rebuild loop.
struct RebuildConfig {
    root: PathBuf,
    output: PathBuf,
    drafts: bool,
    no_exec: bool,
    sandbox: Option<PathBuf>,
    base_url: String,
}

async fn watch_and_rebuild(
    cfg: RebuildConfig,
    reload_tx: broadcast::Sender<()>,
    mut watch_rx: tokio::sync::mpsc::Receiver<
        Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>,
    >,
) {
    while let Some(event) = watch_rx.recv().await {
        if let Ok(events) = event {
            let has_changes = events
                .iter()
                .any(|e| matches!(e.kind, DebouncedEventKind::Any));

            if has_changes {
                println!("Change detected, rebuilding...");
                match crate::site::Site::load(&cfg.root, &cfg.output, cfg.drafts) {
                    Ok(mut site) => {
                        site.no_exec = cfg.no_exec;
                        site.sandbox = cfg.sandbox.clone();
                        site.set_base_url(cfg.base_url.clone());
                        if let Err(e) = site.build() {
                            eprintln!("Build error: {e}");
                        } else {
                            println!("Rebuilt successfully.");
                            let _ = reload_tx.send(());
                        }
                    }
                    Err(e) => {
                        eprintln!("Load error: {e}");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_serve_path_root() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("index.html"), "home").unwrap();
        let result = resolve_serve_path(&out, "/");
        assert_eq!(result.unwrap(), out.join("index.html"));
    }

    #[test]
    fn test_resolve_serve_path_normal_file() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("style.css"), "body{}").unwrap();
        let result = resolve_serve_path(&out, "/style.css");
        assert_eq!(result.unwrap(), out.join("style.css"));
    }

    #[test]
    fn test_resolve_serve_path_directory_traversal_rejected() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        // Attempt to escape output directory
        assert!(resolve_serve_path(&out, "/../../../etc/passwd").is_none());
        assert!(resolve_serve_path(&out, "/..").is_none());
        assert!(resolve_serve_path(&out, "/foo/../../..").is_none());
    }

    #[test]
    fn test_resolve_serve_path_dir_index() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        let sub = out.join("posts");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.html"), "posts").unwrap();
        let result = resolve_serve_path(&out, "/posts");
        assert_eq!(result.unwrap(), sub.join("index.html"));
    }

    #[test]
    fn test_resolve_serve_path_nonexistent_returns_none() {
        let tmp = TempDir::new().unwrap();
        let out = tmp.path().join("public");
        std::fs::create_dir_all(&out).unwrap();
        assert!(resolve_serve_path(&out, "/nope.html").is_none());
    }
}
