//! Build trigger and preview rendering.
//!
//! # Preview-render fidelity (closes agent3 H4 + #151)
//!
//! The editor's right-pane preview renders on every keystroke
//! (debounced 500ms). Three classes of fidelity gap exist between this
//! preview and what `zorto build` would actually produce:
//!
//! 1. **Markdown options.** The user's actual `MarkdownConfig` is loaded
//!    from `config.toml` and the site's `base_url` is passed through, so
//!    blockquote callouts, `target=_blank`/`rel=` rewriting, anchor
//!    links, smart punctuation all match the build.
//!
//! 2. **Shortcodes** (`{{ name(args) }}` / `{% name(args) %}…{% end %}`).
//!    Rendered through `zorto_core::shortcodes::process_shortcodes` using
//!    the site's `templates/shortcodes/` directory (if any) and the same
//!    sandbox boundary the build uses. `figure`, `note`, `tabs`,
//!    `details`, `include`, `youtube`, etc. produce identical output to
//!    `zorto build`. If processing fails (malformed args, sandbox
//!    violation, missing template) the raw content is forwarded to
//!    markdown rendering and the error surfaces in a disclaimer banner
//!    at the top of the preview.
//!
//! 3. **Executable code blocks** (`{python}` / `{bash}` / `{node}`).
//!    Running them on every keystroke would be slow and surprising
//!    (a Python block doing `subprocess.run(...)` inside a debounce
//!    loop is not what the user expects). Each captured exec block is
//!    rendered as its highlighted source plus a visible "code execution
//!    suppressed in preview — Save & Rebuild to run" pill. Deliberately
//!    NOT gated behind a toggle in this PR; follow-up.

use axum::extract::State;
use axum::response::Html;
use std::sync::Arc;

use crate::{AppState, escape};

pub async fn trigger(State(state): State<Arc<AppState>>) -> Html<String> {
    match crate::rebuild_site(&state) {
        Ok(()) => Html(r#"<span style="color: #34d399;">Built successfully.</span>"#.to_string()),
        Err(e) => Html(format!(
            r#"<span style="color: #f87171;">Build error: {}</span>"#,
            escape(&e)
        )),
    }
}

pub async fn render_preview(State(state): State<Arc<AppState>>, body: String) -> Html<String> {
    let content = strip_frontmatter(&body);
    let (md_config, base_url) = load_md_config_and_base(&state);

    // Run shortcodes before markdown rendering, the same order `zorto build`
    // uses. On error, forward the raw content and surface the error in the
    // disclaimer — the preview pane should never silently swallow shortcode
    // text.
    let shortcode_dir = state.root.join("templates/shortcodes");
    let sandbox_root = state.sandbox.as_deref().unwrap_or(&state.root);
    let (after_shortcodes, shortcode_error) = match zorto_core::shortcodes::process_shortcodes(
        &content,
        &shortcode_dir,
        &state.root,
        sandbox_root,
    ) {
        Ok(rendered) => (rendered, None),
        Err(e) => (content.clone(), Some(e.to_string())),
    };

    // `Vec<_>` element is `zorto_core::execute::ExecutableBlock`, which is
    // still `pub(crate)` — we deliberately never name it. `render_markdown`
    // infers the element type from its signature, so an empty `Vec::new()`
    // works; `blocks.len()` is the only thing we ever read off the vec.
    let mut blocks = Vec::new();
    let html = zorto_core::markdown::render_markdown(
        &after_shortcodes,
        &md_config,
        &mut blocks,
        &base_url,
    );
    let exec_count = blocks.len();
    drop(blocks);

    let html = stub_exec_placeholders(html, exec_count);

    let disclaimer = build_disclaimer(exec_count, shortcode_error.as_deref());
    if disclaimer.is_empty() {
        Html(html)
    } else {
        Html(format!("{disclaimer}{html}"))
    }
}

fn load_md_config_and_base(state: &AppState) -> (zorto_core::config::MarkdownConfig, String) {
    let config_path = state.root.join("config.toml");
    let raw = std::fs::read_to_string(&config_path).unwrap_or_default();
    match toml::from_str::<zorto_core::config::Config>(&raw) {
        Ok(cfg) => (cfg.markdown, cfg.base_url),
        Err(_) => (zorto_core::config::MarkdownConfig::default(), String::new()),
    }
}

/// Replace each `<!-- EXEC_BLOCK_N -->` placeholder with a visible
/// "suppressed in preview" affordance. We do not surface the source code
/// (the user is already looking at it in the editor pane), and we
/// deliberately do not run the code on every keystroke.
fn stub_exec_placeholders(html: String, count: usize) -> String {
    let mut result = html;
    for i in 0..count {
        let placeholder = format!("<!-- EXEC_BLOCK_{i} -->");
        if !result.contains(&placeholder) {
            continue;
        }
        let replacement = r#"<div class="code-block-preview-suppressed"><div class="preview-suppressed-pill">code execution suppressed in preview — Save &amp; Rebuild to run this block</div></div>"#;
        // replacen with limit 1 so an EXEC_BLOCK_M index that happens to be
        // a substring of a later block's identifier (e.g. EXEC_BLOCK_1 vs
        // EXEC_BLOCK_10) cannot mis-substitute.
        result = result.replacen(&placeholder, replacement, 1);
    }
    result
}

fn build_disclaimer(exec_count: usize, shortcode_error: Option<&str>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(err) = shortcode_error {
        parts.push(format!(
            r#"shortcode error — preview fell back to raw text: <code>{}</code>"#,
            escape(err)
        ));
    }

    if exec_count > 0 {
        parts.push(format!(
            "{exec_count} executable code block{plural} not run",
            plural = if exec_count == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        return String::new();
    }

    format!(
        r#"<div class="preview-disclaimer">Preview is fragment-only: {}. Save &amp; Rebuild to see the full output.</div>"#,
        parts.join("; ")
    )
}

fn strip_frontmatter(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(rest) = trimmed.strip_prefix("+++") {
        if let Some(end) = rest.find("\n+++") {
            return rest[end + 4..].to_string();
        }
    }
    content.to_string()
}
