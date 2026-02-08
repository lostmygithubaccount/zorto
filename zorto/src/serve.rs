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

const LIVERELOAD_JS: &str = r#"
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
"#;

#[derive(Clone)]
struct AppState {
    reload_tx: broadcast::Sender<()>,
    output_dir: PathBuf,
}

pub async fn serve(
    root: &Path,
    output_dir: &Path,
    drafts: bool,
    interface: &str,
    port: u16,
    open_browser: bool,
) -> anyhow::Result<()> {
    let base_url = format!("http://{interface}:{port}");

    // Initial build
    println!("Building site...");
    let mut site = crate::site::Site::load(root, output_dir, drafts)?;
    site.set_base_url(base_url.clone());
    site.build()?;
    println!("Site built successfully.");

    // Set up broadcast channel for live reload
    let (reload_tx, _) = broadcast::channel::<()>(16);
    let state = AppState {
        reload_tx: reload_tx.clone(),
        output_dir: output_dir.to_path_buf(),
    };

    let app = Router::new()
        .route("/__livereload", get(ws_handler))
        .fallback(get(serve_file).head(serve_file))
        .with_state(state);

    let addr: SocketAddr = format!("{interface}:{port}").parse()?;
    println!("Serving at http://{addr}");

    if open_browser {
        let url = format!("http://{addr}");
        let _ = open::that(&url);
    }

    // Bridge notify events into a tokio channel so the watcher loop is fully async
    let (watch_tx, watch_rx) = tokio::sync::mpsc::channel(16);
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(300), notify_tx)?;
    let watch_dirs = ["content", "templates", "sass", "static"];
    for dir in &watch_dirs {
        let path = root.join(dir);
        if path.exists() {
            let _ = debouncer
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive);
        }
    }
    let config_path = root.join("config.toml");
    if config_path.exists() {
        let _ = debouncer
            .watcher()
            .watch(&config_path, notify::RecursiveMode::NonRecursive);
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
    let root_clone = root.to_path_buf();
    let output_clone = output_dir.to_path_buf();
    let watcher_handle = tokio::spawn(async move {
        watch_and_rebuild(
            root_clone,
            output_clone,
            drafts,
            reload_tx,
            base_url,
            watch_rx,
        )
        .await;
    });

    // Start server — ctrl+c cancels everything
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        eprintln!("\nShutting down...");
    };

    let listener = tokio::net::TcpListener::bind(addr).await?;
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

    // Determine file path
    let file_path = if path == "/" {
        output_dir.join("index.html")
    } else {
        let stripped = path.trim_start_matches('/');
        let candidate = output_dir.join(stripped);
        if candidate.is_dir() {
            candidate.join("index.html")
        } else if candidate.exists() {
            candidate
        } else {
            let with_index = output_dir.join(stripped).join("index.html");
            if with_index.exists() {
                with_index
            } else {
                // 404
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
                return (StatusCode::NOT_FOUND, "Not Found").into_response();
            }
        }
    };

    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
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

async fn watch_and_rebuild(
    root: PathBuf,
    output: PathBuf,
    drafts: bool,
    reload_tx: broadcast::Sender<()>,
    base_url: String,
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
                match crate::site::Site::load(&root, &output, drafts) {
                    Ok(mut site) => {
                        site.set_base_url(base_url.clone());
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
