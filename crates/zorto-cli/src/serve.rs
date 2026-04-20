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
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::broadcast;
use zorto_core::content::Section;

const RELOAD_CHANNEL_CAPACITY: usize = 16;
const DEBOUNCE_MS: u64 = 300;

/// Initial delay (ms) before attempting to reconnect the livereload WebSocket.
const LIVERELOAD_RECONNECT_INTERVAL_MS: u64 = 1000;
/// Maximum backoff delay (ms) for livereload WebSocket reconnections.
const LIVERELOAD_MAX_RECONNECT_MS: u64 = 30000;

static LIVERELOAD_JS: LazyLock<String> = LazyLock::new(|| {
    format!(
        r#"
<script>
(function() {{
    var reconnectInterval = {LIVERELOAD_RECONNECT_INTERVAL_MS};
    var maxReconnect = {LIVERELOAD_MAX_RECONNECT_MS};
    var toastId = '__zorto_build_error';

    function clearToast() {{
        var el = document.getElementById(toastId);
        if (el) el.remove();
    }}

    function showToast(msg) {{
        clearToast();
        var el = document.createElement('div');
        el.id = toastId;
        el.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#b00020;color:#fff;padding:0.75em 1em;font:14px/1.4 ui-monospace,SFMono-Regular,Menlo,monospace;white-space:pre-wrap;box-shadow:0 2px 8px rgba(0,0,0,0.25);';
        el.textContent = 'Build error:\n' + msg;
        document.body.appendChild(el);
    }}

    function connect() {{
        var ws = new WebSocket('ws://' + location.host + '/__livereload');
        ws.onmessage = function(event) {{
            if (event.data === 'reload') {{
                location.reload();
                return;
            }}
            try {{
                var payload = JSON.parse(event.data);
                if (payload.kind === 'error') showToast(payload.msg);
                else if (payload.kind === 'clear') clearToast();
            }} catch (e) {{}}
        }};
        ws.onclose = function() {{
            setTimeout(function() {{ connect(); }}, reconnectInterval);
            reconnectInterval = Math.min(reconnectInterval * 1.5, maxReconnect);
        }};
        ws.onopen = function() {{
            reconnectInterval = {LIVERELOAD_RECONNECT_INTERVAL_MS};
        }};
    }}
    connect();
}})();
</script>
"#
    )
});

/// Messages broadcast from the rebuild loop to every connected browser tab.
#[derive(Clone, Debug)]
enum LivereloadMsg {
    Reload,
    Error(String),
    Clear,
}

impl LivereloadMsg {
    fn to_ws_text(&self) -> String {
        match self {
            LivereloadMsg::Reload => String::from("reload"),
            LivereloadMsg::Error(msg) => {
                let escaped = msg
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "");
                format!(r#"{{"kind":"error","msg":"{escaped}"}}"#)
            }
            LivereloadMsg::Clear => String::from(r#"{"kind":"clear"}"#),
        }
    }
}

#[derive(Clone)]
struct AppState {
    reload_tx: broadcast::Sender<LivereloadMsg>,
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
    let requested_port = cfg.port;
    let requested: SocketAddr = format!("{}:{}", cfg.interface, requested_port).parse()?;
    let listener = match tokio::net::TcpListener::bind(requested).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            let fallback: SocketAddr = format!("{}:0", cfg.interface).parse()?;
            let l = tokio::net::TcpListener::bind(fallback).await?;
            let fallback_port = l.local_addr().map(|a| a.port()).unwrap_or(0);
            println!("Port {requested_port} busy — using {fallback_port} instead.");
            l
        }
        Err(e) => return Err(e.into()),
    };
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");

    // Initial build (timed, with draft-count surfacing).
    let build_start = std::time::Instant::now();
    let mut site = zorto_core::site::Site::load(cfg.root, cfg.output_dir, cfg.drafts)?;
    site.no_exec = cfg.no_exec;
    site.sandbox = cfg.sandbox.map(|p| p.to_path_buf());
    site.set_base_url(base_url.clone());
    let draft_total = site.pages.values().filter(|p| p.draft).count();
    site.build()?;
    let build_ms = build_start.elapsed().as_millis();
    if draft_total > 0 {
        if cfg.drafts {
            println!("Including {draft_total} draft page(s). Pass --no-drafts to hide them.");
        } else {
            println!("Hiding {draft_total} draft page(s). Remove --no-drafts to include them.");
        }
    }
    let open_url = preview_open_url(&base_url, site.sections.values());

    // Set up broadcast channel for live reload
    let (reload_tx, _) = broadcast::channel::<LivereloadMsg>(RELOAD_CHANNEL_CAPACITY);
    let state = AppState {
        reload_tx: reload_tx.clone(),
        output_dir: cfg.output_dir.to_path_buf(),
    };

    let app = Router::new()
        .route("/__livereload", get(ws_handler))
        .fallback(get(serve_file).head(serve_file))
        .with_state(state);

    let url = format!("http://{addr}");
    println!("Ready at {url} (build {build_ms}ms). Ctrl-C to stop.");
    if !cfg.open_browser {
        println!("Tip: pass --open to launch your browser automatically.");
    }

    if cfg.open_browser {
        println!("Opening {open_url}");
        if let Err(e) = open::that(&open_url) {
            eprintln!("Could not open browser ({e}). Visit {open_url} manually.");
        }
    }

    // Bridge notify events into a tokio channel so the watcher loop is fully async
    let (watch_tx, watch_rx) = tokio::sync::mpsc::channel(RELOAD_CHANNEL_CAPACITY);
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(DEBOUNCE_MS), notify_tx)?;
    let watch_dirs = ["content", "templates", "sass", "static"];
    for dir in &watch_dirs {
        let path = cfg.root.join(dir);
        if path.exists() {
            debouncer
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive)?;
        }
    }
    // Also watch any external content_dirs declared in config.toml so edits
    // under e.g. `../docs` trigger a rebuild. Silently skip unreadable paths —
    // a stale config shouldn't kill the preview server.
    for dir_config in &site.config.content_dirs {
        let external = cfg.root.join(&dir_config.path);
        if external.exists() {
            if let Err(e) = debouncer
                .watcher()
                .watch(&external, notify::RecursiveMode::Recursive)
            {
                eprintln!(
                    "Warning: cannot watch content_dir {}: {e}",
                    external.display()
                );
            }
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

fn preview_open_url<'a>(base_url: &str, sections: impl Iterator<Item = &'a Section>) -> String {
    let presentation_path = sections
        .filter(|section| section.path != "/")
        .filter(|section| section.template.as_deref() == Some("presentation.html"))
        .map(|section| section.path.as_str())
        .collect::<Vec<_>>();
    if presentation_path.len() == 1 {
        return join_preview_url(base_url, presentation_path[0]);
    }
    base_url.to_string()
}

fn join_preview_url(base_url: &str, path: &str) -> String {
    if path == "/" {
        return base_url.to_string();
    }
    format!("{}{}", base_url.trim_end_matches('/'), path)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.reload_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        if socket
            .send(Message::Text(msg.to_ws_text().into()))
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
        "txt" | "sh" => "text/plain",
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
    let js: &str = &LIVERELOAD_JS;
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + js.len());
        result.push_str(&html[..pos]);
        result.push_str(js);
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{html}{js}")
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
    reload_tx: broadcast::Sender<LivereloadMsg>,
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
                let rebuild_start = std::time::Instant::now();
                println!("Change detected, rebuilding...");
                match zorto_core::site::Site::load(&cfg.root, &cfg.output, cfg.drafts) {
                    Ok(mut site) => {
                        site.no_exec = cfg.no_exec;
                        site.sandbox = cfg.sandbox.clone();
                        site.set_base_url(cfg.base_url.clone());
                        match site.build() {
                            Ok(()) => {
                                let ms = rebuild_start.elapsed().as_millis();
                                println!("Rebuilt in {ms}ms.");
                                let _ = reload_tx.send(LivereloadMsg::Clear);
                                let _ = reload_tx.send(LivereloadMsg::Reload);
                            }
                            Err(e) => {
                                eprintln!("Build error: {e}");
                                let _ = reload_tx.send(LivereloadMsg::Error(format!("{e:#}")));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Load error: {e}");
                        let _ = reload_tx.send(LivereloadMsg::Error(format!("{e:#}")));
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

    #[test]
    fn livereload_reload_serializes_to_plain_text() {
        assert_eq!(LivereloadMsg::Reload.to_ws_text(), "reload");
    }

    #[test]
    fn livereload_clear_serializes_to_kind_tagged_json() {
        assert_eq!(LivereloadMsg::Clear.to_ws_text(), r#"{"kind":"clear"}"#);
    }

    #[test]
    fn livereload_error_escapes_control_chars() {
        let msg = LivereloadMsg::Error("line1\nline2 \"quoted\" \\ path".to_string());
        let json = msg.to_ws_text();
        assert!(json.starts_with(r#"{"kind":"error","msg":"#), "got: {json}");
        // Control chars + quotes + backslash are JSON-escaped (not raw).
        assert!(!json.contains('\n'), "got: {json}");
        assert!(json.contains(r#"\""#), "got: {json}");
        assert!(json.contains(r"\\"), "got: {json}");
        assert!(json.contains(r"\n"), "got: {json}");
    }

    #[test]
    fn preview_open_url_prefers_single_presentation_section() {
        let sections = [
            Section {
                title: "Home".into(),
                description: None,
                path: "/".into(),
                permalink: "http://127.0.0.1:1111/".into(),
                content: String::new(),
                raw_content: String::new(),
                pages: vec![],
                sort_by: None,
                paginate_by: None,
                template: None,
                render_pages: true,
                extra: Default::default(),
                relative_path: "_index.md".into(),
            },
            Section {
                title: "Deck".into(),
                description: None,
                path: "/intro/".into(),
                permalink: "http://127.0.0.1:1111/intro/".into(),
                content: String::new(),
                raw_content: String::new(),
                pages: vec![],
                sort_by: None,
                paginate_by: None,
                template: Some("presentation.html".into()),
                render_pages: false,
                extra: Default::default(),
                relative_path: "intro/_index.md".into(),
            },
        ];
        let url = preview_open_url("http://127.0.0.1:1111", sections.iter());
        assert_eq!(url, "http://127.0.0.1:1111/intro/");
    }

    #[test]
    fn preview_open_url_falls_back_to_root_for_multiple_presentations() {
        let sections = [
            Section {
                title: "Deck One".into(),
                description: None,
                path: "/intro/".into(),
                permalink: String::new(),
                content: String::new(),
                raw_content: String::new(),
                pages: vec![],
                sort_by: None,
                paginate_by: None,
                template: Some("presentation.html".into()),
                render_pages: false,
                extra: Default::default(),
                relative_path: "intro/_index.md".into(),
            },
            Section {
                title: "Deck Two".into(),
                description: None,
                path: "/deep-dive/".into(),
                permalink: String::new(),
                content: String::new(),
                raw_content: String::new(),
                pages: vec![],
                sort_by: None,
                paginate_by: None,
                template: Some("presentation.html".into()),
                render_pages: false,
                extra: Default::default(),
                relative_path: "deep-dive/_index.md".into(),
            },
        ];
        let url = preview_open_url("http://127.0.0.1:1111", sections.iter());
        assert_eq!(url, "http://127.0.0.1:1111");
    }
}
