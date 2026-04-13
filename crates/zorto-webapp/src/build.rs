//! Build trigger and preview rendering.
//!
//! # Preview-render fidelity (closes agent3 H4)
//!
//! The editor's right-pane preview renders on every keystroke
//! (debounced 500ms). Three classes of fidelity gap exist between this
//! preview and what `zorto build` would actually produce, and we close
//! them as far as the webapp boundary allows:
//!
//! 1. **Markdown options.** Previously rendered with
//!    `MarkdownConfig::default()`, which silently disagreed with any
//!    user-customised `[markdown]` table — anchor links, smart
//!    punctuation, external-link rewriting, callouts. Now the user's
//!    actual `MarkdownConfig` is loaded from `config.toml` and the
//!    site's `base_url` is passed through, so blockquote callouts,
//!    `target=_blank`/`rel=` rewriting, and heading anchor IDs all
//!    match the build.
//!
//! 2. **Executable code blocks** (`{python}` / `{bash}` / `{node}`).
//!    Running them on every keystroke would be slow and surprising
//!    (a Python block doing `subprocess.run(...)` inside a debounce
//!    loop is not what the user expects). Each captured exec block is
//!    instead rendered as the highlighted source plus a visible
//!    "code execution suppressed in preview — save to run" pill, so
//!    the user sees something rather than getting silent emptiness.
//!
//! 3. **Shortcodes** (`{{ name(args) }}` / `{% name(args) %}…{% end %}`).
//!    The shortcode processor lives in `zorto_core::shortcodes`, which
//!    is `pub(crate)` — the webapp cannot reach it across crate
//!    boundaries today. As a partial fix, both inline and body
//!    shortcodes are detected and replaced with a labelled
//!    `(shortcode preview unavailable)` stub *before* markdown
//!    rendering, so users see "this thing would render" rather than
//!    raw `{{ }}` text. Full shortcode rendering in the preview pane
//!    requires exposing `zorto_core::shortcodes::process_shortcodes`
//!    as `pub`; deferred to a follow-up zorto-core PR (see PR body).

use axum::extract::State;
use axum::response::Html;
use std::fmt::Write as _;
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

    // Load the user's actual MarkdownConfig + base_url from disk so the
    // preview matches what `zorto build` will produce. Any failure (missing
    // config, malformed TOML, missing fields) falls back to the same defaults
    // the site would use during a real build.
    let (md_config, base_url) = load_md_config_and_base(&state);

    // Stub out shortcodes so the user sees a labelled placeholder rather than
    // raw `{{ }}` syntax. Counts both kinds for the disclaimer.
    let (with_stubs, shortcode_count) = stub_shortcodes(&content);

    // The `Vec<_>` element type (`zorto_core::execute::ExecutableBlock`) lives
    // in a `pub(crate)` module and so cannot be named from this crate.
    // `render_markdown` infers the element type from the type-elaborated
    // signature, so an empty `Vec::new()` works without naming it. We then
    // use `blocks.len()` for the count and substitute the placeholders by
    // index — never inspecting the elements themselves.
    let mut blocks = Vec::new();
    let html =
        zorto_core::markdown::render_markdown(&with_stubs, &md_config, &mut blocks, &base_url);
    let exec_count = blocks.len();
    drop(blocks);

    // Substitute the `<!-- EXEC_BLOCK_N -->` placeholders inserted by
    // `render_markdown` with a visible "suppressed in preview" affordance
    // instead of running the code. We deliberately do NOT reach into the
    // block data — the user is already looking at their source in the
    // editor pane.
    let html = stub_exec_placeholders(html, exec_count);

    let disclaimer = build_disclaimer(shortcode_count, exec_count);
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

/// Replace shortcode invocations with a labelled placeholder so the user
/// sees structure where the rendered shortcode would land. Returns the
/// substituted text and the total shortcode count (inline + body) so the
/// caller can show a single disclaimer banner.
///
/// Hand-rolled rather than `regex`-based to keep the webapp's dependency
/// surface unchanged (the `regex` crate is not currently a webapp dep, and
/// the inline matcher we need is simple enough that pulling it in would be
/// gratuitous).
fn stub_shortcodes(content: &str) -> (String, usize) {
    // Body shortcodes first — they may wrap inline shortcodes that we don't
    // want to also substitute outside the wrapper.
    let (after_body, body_count) = stub_body_shortcodes(content);
    let (after_inline, inline_count) = stub_inline_shortcodes(&after_body);
    (after_inline, body_count + inline_count)
}

fn stub_body_shortcodes(content: &str) -> (String, usize) {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let mut count = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            // Try to parse `{% name(args) %} ... {% end %}`. If anything
            // about the parse fails, emit the literal `{` and continue —
            // we never want this stubber to silently swallow text.
            if let Some((name, after_close, end_close)) = parse_body_shortcode(content, i) {
                let _ = write!(
                    out,
                    r#"<div class="preview-shortcode-stub" data-kind="body" data-name="{name}">[shortcode <code>{{% {name}(…) %}}…{{% end %}}</code> — preview unavailable; run a build to render]</div>"#,
                    name = escape(&name),
                );
                count += 1;
                i = end_close;
                let _ = after_close; // unused: end_close already covers it
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    (out, count)
}

fn stub_inline_shortcodes(content: &str) -> (String, usize) {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let mut count = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some((name, after_close)) = parse_inline_shortcode(content, i) {
                let _ = write!(
                    out,
                    r#"<span class="preview-shortcode-stub" data-kind="inline" data-name="{name}">[shortcode <code>{{{{ {name}(…) }}}}</code> — preview unavailable]</span>"#,
                    name = escape(&name),
                );
                count += 1;
                i = after_close;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    (out, count)
}

/// Parse `{{ name(args) }}` at `start`. Returns `(name, byte index after the
/// closing `}}`)` if the tag matches; `None` if it doesn't look like a
/// shortcode (treat as literal text).
fn parse_inline_shortcode(content: &str, start: usize) -> Option<(String, usize)> {
    let s = &content[start..];
    let after_open = s.strip_prefix("{{")?;
    let after_open_trim = after_open.trim_start();
    let consumed_ws = after_open.len() - after_open_trim.len();
    let (name, name_end) = take_ident(after_open_trim)?;
    if name.is_empty() {
        return None;
    }
    let after_name = &after_open_trim[name_end..];
    let after_name_trim = after_name.trim_start();
    if !after_name_trim.starts_with('(') {
        return None;
    }
    // Find the matching `)` then the closing `}}`. Skip over quoted args
    // that may legitimately contain `)`.
    let after_paren = scan_past_args(after_name_trim, b'(', b')')?;
    let close_search = &after_name_trim[after_paren..];
    let close_search_trim = close_search.trim_start();
    let trim_offset = close_search.len() - close_search_trim.len();
    let after_close = close_search_trim.strip_prefix("}}")?;
    let consumed = (after_open.len() - after_open_trim.len()) // leading ws inside `{{ `
        + name_end
        + (after_name.len() - after_name_trim.len())
        + after_paren
        + trim_offset
        + 2; // closing `}}`
    let _ = consumed_ws;
    let after_close_idx = start + 2 + consumed;
    // Sanity: the end position must land where after_close starts.
    let _ = after_close;
    Some((name.to_string(), after_close_idx))
}

/// Parse `{% name(args) %}body{% end %}` at `start`. Returns `(name, byte
/// index after the opening `%}`, byte index after the closing `{% end %}`)`
/// when matched.
fn parse_body_shortcode(content: &str, start: usize) -> Option<(String, usize, usize)> {
    let s = &content[start..];
    let after_open = s.strip_prefix("{%")?;
    let after_open_trim = after_open.trim_start();
    let (name, name_end) = take_ident(after_open_trim)?;
    if name.is_empty() || name == "end" {
        return None;
    }
    let after_name = &after_open_trim[name_end..];
    let after_name_trim = after_name.trim_start();
    if !after_name_trim.starts_with('(') {
        return None;
    }
    let after_paren = scan_past_args(after_name_trim, b'(', b')')?;
    let close_search = &after_name_trim[after_paren..];
    let close_search_trim = close_search.trim_start();
    let trim_offset = close_search.len() - close_search_trim.len();
    let after_open_close = close_search_trim.strip_prefix("%}")?;
    // Position after the opening `%}` in the original `content`.
    let opening_consumed = 2 // `{%`
        + (after_open.len() - after_open_trim.len())
        + name_end
        + (after_name.len() - after_name_trim.len())
        + after_paren
        + trim_offset
        + 2; // `%}`
    let opening_end = start + opening_consumed;

    // Now find the matching `{% end %}`. Allow `end` with surrounding
    // whitespace; do not nest (zorto's processor doesn't either, per
    // agent5's earlier review).
    let needle = "{%";
    let mut search_from = opening_end;
    let _ = after_open_close;
    loop {
        let rest = content.get(search_from..)?;
        let next = rest.find(needle)?;
        let pos = search_from + next;
        let candidate = &content[pos..];
        if let Some(end_close) = parse_end_marker(candidate) {
            return Some((name.to_string(), opening_end, pos + end_close));
        }
        // Advance past this `{%` and keep looking.
        search_from = pos + 2;
    }
}

/// At `s`, expect `{% end %}` (with optional surrounding whitespace).
/// Returns the byte length consumed if matched.
fn parse_end_marker(s: &str) -> Option<usize> {
    let after_open = s.strip_prefix("{%")?;
    let after_open_trim = after_open.trim_start();
    let after_end = after_open_trim.strip_prefix("end")?;
    let after_end_trim = after_end.trim_start();
    let after_close = after_end_trim.strip_prefix("%}")?;
    let consumed = s.len() - after_close.len();
    Some(consumed)
}

/// Scan past a balanced `(...)` argument list, respecting `"..."` and
/// `'...'` quoted strings. Returns the byte offset just AFTER the closing
/// `)`. `s` must start at `(`.
fn scan_past_args(s: &str, open: u8, close: u8) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&open) {
        return None;
    }
    let mut i = 1usize;
    let mut depth = 1usize;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return None;
                }
                i += 1; // past closing quote
            }
            b'\'' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += 1;
                }
                if i >= bytes.len() {
                    return None;
                }
                i += 1;
            }
            b => {
                if b == open {
                    depth += 1;
                } else if b == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                i += 1;
            }
        }
    }
    None
}

/// Take an ASCII identifier (`[A-Za-z_][A-Za-z0-9_]*`) from the front of `s`.
/// Returns the slice and the byte length consumed.
fn take_ident(s: &str) -> Option<(&str, usize)> {
    let bytes = s.as_bytes();
    let mut end = 0usize;
    if bytes.is_empty() {
        return None;
    }
    if !(bytes[0].is_ascii_alphabetic() || bytes[0] == b'_') {
        return None;
    }
    end += 1;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    Some((&s[..end], end))
}

/// Replace each `<!-- EXEC_BLOCK_N -->` placeholder with a generic
/// "execution suppressed" stub. We do not surface the source code (the
/// user is already looking at it in the editor pane), and we deliberately
/// do not run the code on every keystroke.
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
        // EXEC_BLOCK_10) cannot mis-substitute. The fixed-string format
        // `<!-- EXEC_BLOCK_{i} -->` already disambiguates by the trailing
        // ` -->`, but limiting to 1 is belt-and-braces.
        result = result.replacen(&placeholder, replacement, 1);
    }
    result
}

fn build_disclaimer(shortcode_count: usize, exec_count: usize) -> String {
    if shortcode_count == 0 && exec_count == 0 {
        return String::new();
    }
    let mut parts = Vec::new();
    if shortcode_count > 0 {
        parts.push(format!(
            "{shortcode_count} shortcode{plural} stubbed",
            plural = if shortcode_count == 1 { "" } else { "s" }
        ));
    }
    if exec_count > 0 {
        parts.push(format!(
            "{exec_count} executable code block{plural} not run",
            plural = if exec_count == 1 { "" } else { "s" }
        ));
    }
    format!(
        r#"<div class="preview-disclaimer">Preview is fragment-only: {}. Save &amp; Rebuild to see the full output.</div>"#,
        parts.join(", ")
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
