use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static BODY_SHORTCODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)\{%\s*(\w+)\s*\(((?:[^)"']|"[^"]*"|'[^']*')*)\)\s*%\}(.*?)\{%\s*end\s*%\}"#)
        .unwrap()
});
static INLINE_SHORTCODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\{\{\s*(\w+)\s*\(((?:[^)"']|"[^"]*"|'[^']*')*)\)\s*\}\}"#).unwrap()
});
static ARGS_DOUBLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap());
static ARGS_SINGLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\s*=\s*'([^']*)'").unwrap());

/// Process shortcodes in raw markdown content before markdown rendering.
///
/// Inline shortcodes: {{ name(key="value", key2="value2") }}
/// Body shortcodes: {% name(key="value") %}...body...{% end %}
///
/// Built-in shortcodes (no template needed):
/// - `include(path="...")`: Read and inject file contents relative to site root
/// - `tabs(labels="A|B")`: Tabbed content panels, body split on `<!-- tab -->`
/// - `note(type="info|warning|danger|tip")`: Callout/admonition box
/// - `details(summary="...")`: Collapsible content section
/// - `figure(src="...")`: Image with optional caption
/// - `youtube(id="...")`: Responsive YouTube embed
/// - `gist(url="...")`: Embedded GitHub Gist
/// - `mermaid()`: Mermaid diagram
///
/// Process shortcodes in content.
///
/// `sandbox_root` is the outermost directory that file operations (like the
/// `include` shortcode) are allowed to access. Paths that resolve outside this
/// boundary are rejected. Pass `site_root` if no wider sandbox is needed.
pub fn process_shortcodes(
    content: &str,
    shortcode_dir: &Path,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    // Process body shortcodes first (they can contain inline shortcodes)
    let result = process_body_shortcodes(content, shortcode_dir, site_root, sandbox_root)?;

    // Then process inline shortcodes
    process_inline_shortcodes(&result, shortcode_dir, site_root, sandbox_root)
}

/// Process body shortcodes: {% name(args) %}...{% end %}
fn process_body_shortcodes(
    content: &str,
    shortcode_dir: &Path,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    let mut result = content.to_string();
    let mut iterations = 0;

    // Loop to handle nested shortcodes
    while BODY_SHORTCODE_RE.is_match(&result) && iterations < 10 {
        let mut first_error: Option<anyhow::Error> = None;
        let new_result = BODY_SHORTCODE_RE.replace_all(&result, |caps: &regex::Captures| {
            let name = &caps[1];
            let args_str = &caps[2];
            let body = &caps[3];

            match resolve_shortcode(
                name,
                args_str,
                Some(body.trim()),
                shortcode_dir,
                site_root,
                sandbox_root,
            ) {
                Ok(rendered) => rendered,
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(anyhow::anyhow!("shortcode error in {name}: {e}"));
                    }
                    caps[0].to_string()
                }
            }
        });
        if let Some(e) = first_error {
            return Err(e);
        }
        result = new_result.into_owned();
        iterations += 1;
    }

    Ok(result)
}

/// Process inline shortcodes: {{ name(args) }}
fn process_inline_shortcodes(
    content: &str,
    shortcode_dir: &Path,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    let mut first_error: Option<anyhow::Error> = None;
    let result = INLINE_SHORTCODE_RE.replace_all(content, |caps: &regex::Captures| {
        let name = &caps[1];
        let args_str = &caps[2];

        match resolve_shortcode(name, args_str, None, shortcode_dir, site_root, sandbox_root) {
            Ok(rendered) => rendered,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(anyhow::anyhow!("shortcode error in {name}: {e}"));
                }
                caps[0].to_string()
            }
        }
    });

    if let Some(e) = first_error {
        return Err(e);
    }

    Ok(result.into_owned())
}

/// Parse shortcode arguments: key="value", key2="value2"
fn parse_args(args_str: &str) -> HashMap<String, String> {
    let mut args = HashMap::new();

    for cap in ARGS_DOUBLE_RE.captures_iter(args_str) {
        args.insert(cap[1].to_string(), cap[2].to_string());
    }

    // Also handle single-quoted values
    for cap in ARGS_SINGLE_RE.captures_iter(args_str) {
        args.entry(cap[1].to_string())
            .or_insert_with(|| cap[2].to_string());
    }

    args
}

/// Dispatch a shortcode: handle built-ins first, fall back to template rendering.
fn resolve_shortcode(
    name: &str,
    args_str: &str,
    body: Option<&str>,
    shortcode_dir: &Path,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    match name {
        "include" => builtin_include(args_str, site_root, sandbox_root),
        "tabs" => builtin_tabs(args_str, body),
        "note" => builtin_note(args_str, body),
        "details" => builtin_details(args_str, body),
        "figure" => builtin_figure(args_str),
        "youtube" => builtin_youtube(args_str),
        "gist" => builtin_gist(args_str),
        "mermaid" => builtin_mermaid(body),
        _ => render_shortcode(name, args_str, body, shortcode_dir),
    }
}

/// Built-in `include` shortcode: read file contents from a local path or remote URL.
///
/// Arguments:
/// - `path` (required): file path relative to site root, or an `https://` URL
/// - `strip_frontmatter` (optional): "true" to strip `+++`-delimited TOML frontmatter
/// - `rewrite_links` (optional): "true" to rewrite relative `.md` links to clean URL paths.
///   This makes links work on both GitHub (as `.md` links) and the built site (as clean URLs).
fn builtin_include(
    args_str: &str,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let path = args
        .get("path")
        .ok_or_else(|| anyhow::anyhow!("include shortcode requires a `path` argument"))?;

    let content = if path.starts_with("https://") || path.starts_with("http://") {
        fetch_url(path)?
    } else {
        read_local_file(path, site_root, sandbox_root)?
    };

    let strip = args.get("strip_frontmatter").is_some_and(|v| v == "true");
    let mut content = if strip {
        strip_toml_frontmatter(&content)
    } else {
        content
    };

    let rewrite = args.get("rewrite_links").is_some_and(|v| v == "true");
    if rewrite && !path.starts_with("http") {
        content = rewrite_md_links(&content, path);
    }

    Ok(content)
}

/// Regex matching markdown links to local `.md` files: `[text](path.md)` or `[text](path.md#anchor)`.
static MD_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]*)\]\(([^)]+?\.md)(#[^)]*)?\)").unwrap());

/// Rewrite relative `.md` links in included content to clean URL paths.
///
/// Given the include `path` (relative to site root), resolves each relative `.md`
/// link against the included file's parent directory, then converts to a clean URL:
/// - Strip `.md` extension
/// - `README` at the end maps to the directory (e.g. `docs/concepts/README` → `/docs/concepts/`)
/// - Prepend `/`, append `/`
/// - Preserve `#anchor` fragments
pub(crate) fn rewrite_md_links(content: &str, include_path: &str) -> String {
    let include_dir = Path::new(include_path).parent().unwrap_or(Path::new(""));

    MD_LINK_RE
        .replace_all(content, |caps: &regex::Captures| {
            let text = &caps[1];
            let rel_path = &caps[2];
            let anchor = caps.get(3).map(|m| m.as_str()).unwrap_or("");

            // Skip absolute URLs and paths
            if rel_path.starts_with("http://")
                || rel_path.starts_with("https://")
                || rel_path.starts_with('/')
            {
                return caps[0].to_string();
            }

            // Resolve relative to the included file's directory
            let resolved = normalize_path(&include_dir.join(rel_path));
            let resolved_str = resolved.to_string_lossy();

            // Strip .md extension
            let without_ext = resolved_str.trim_end_matches(".md");

            // Handle README → directory
            let url_path = if without_ext.ends_with("/README") || without_ext == "README" {
                without_ext.trim_end_matches("README").to_string()
            } else {
                format!("{without_ext}/")
            };

            // Ensure leading slash
            let url_path = if url_path.starts_with('/') {
                url_path
            } else {
                format!("/{url_path}")
            };

            format!("[{text}]({url_path}{anchor})")
        })
        .into_owned()
}

/// Normalize a path by resolving `.` and `..` components without filesystem access.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => {
                components.push(other);
            }
        }
    }
    components.iter().collect()
}

/// Fetch content from a remote URL.
fn fetch_url(url: &str) -> anyhow::Result<String> {
    ureq::get(url)
        .call()
        .map_err(|e| anyhow::anyhow!("include shortcode: failed to fetch {url}: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| anyhow::anyhow!("include shortcode: failed to read response from {url}: {e}"))
}

/// Read a local file within the sandbox boundary.
fn read_local_file(path: &str, site_root: &Path, sandbox_root: &Path) -> anyhow::Result<String> {
    let file_path = site_root.join(path);

    // Ensure the resolved path stays within the sandbox boundary (allow
    // relative traversal like "../../shared/foo.md" as long as it doesn't
    // escape the sandbox root).
    let canonical = file_path.canonicalize().map_err(|e| {
        anyhow::anyhow!(
            "include shortcode: cannot resolve {}: {e}",
            file_path.display()
        )
    })?;
    let canonical_sandbox = sandbox_root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("include shortcode: cannot resolve sandbox root: {e}"))?;
    if !canonical.starts_with(&canonical_sandbox) {
        anyhow::bail!("include shortcode: path escapes sandbox boundary: {}", path);
    }

    std::fs::read_to_string(&canonical).map_err(|e| {
        anyhow::anyhow!(
            "include shortcode: cannot read {}: {e}",
            file_path.display()
        )
    })
}

/// Strip `+++`-delimited TOML frontmatter from content.
///
/// Matches the closing `+++` only at the start of a line (after a newline),
/// consistent with how [`parse_frontmatter`](crate::content::parse_frontmatter)
/// detects frontmatter boundaries.
fn strip_toml_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("+++")
        && let Some(after) = rest.find("\n+++")
    {
        return rest[after + 4..].to_string();
    }
    content.to_string()
}

/// Built-in `tabs` shortcode: tabbed content panels.
///
/// Arguments:
/// - `labels` (required): pipe-delimited tab labels, e.g. `labels="Python|Bash"`
///
/// Body is split on `<!-- tab -->` markers. Each part becomes a tab panel.
fn builtin_tabs(args_str: &str, body: Option<&str>) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let labels_str = args
        .get("labels")
        .ok_or_else(|| anyhow::anyhow!("tabs shortcode requires a `labels` argument"))?;
    let labels: Vec<&str> = labels_str.split('|').collect();
    let body = body.ok_or_else(|| anyhow::anyhow!("tabs shortcode requires a body"))?;
    let parts: Vec<&str> = body.split("<!-- tab -->").collect();

    if labels.len() != parts.len() {
        return Err(anyhow::anyhow!(
            "tabs shortcode: {} labels but {} tab panels",
            labels.len(),
            parts.len()
        ));
    }

    let mut html = String::from("<div class=\"tabs\" data-tabs>\n<div class=\"tabs__nav\">\n");
    for (i, label) in labels.iter().enumerate() {
        let active = if i == 0 { " tabs__btn--active" } else { "" };
        html.push_str(&format!(
            "<button class=\"tabs__btn{active}\" data-tab-idx=\"{i}\">{}</button>",
            label.trim()
        ));
    }
    html.push_str("\n</div>\n");

    for (i, part) in parts.iter().enumerate() {
        let active = if i == 0 { " tabs__panel--active" } else { "" };
        html.push_str(&format!(
            "<div class=\"tabs__panel{active}\" data-tab-idx=\"{i}\">\n\n{}\n\n</div>\n",
            part.trim()
        ));
    }

    html.push_str(concat!(
        "</div>\n",
        "<script>\n",
        "document.currentScript.previousElementSibling.querySelectorAll('.tabs__btn').forEach(btn => {\n",
        "  btn.addEventListener('click', () => {\n",
        "    const t = btn.closest('[data-tabs]'), i = btn.dataset.tabIdx;\n",
        "    t.querySelectorAll('.tabs__btn').forEach(b => b.classList.remove('tabs__btn--active'));\n",
        "    t.querySelectorAll('.tabs__panel').forEach(p => p.classList.remove('tabs__panel--active'));\n",
        "    btn.classList.add('tabs__btn--active');\n",
        "    t.querySelector('.tabs__panel[data-tab-idx=\"' + i + '\"]').classList.add('tabs__panel--active');\n",
        "  });\n",
        "});\n",
        "</script>\n",
    ));

    Ok(html)
}

/// Built-in `note` shortcode: callout/admonition box.
///
/// Arguments:
/// - `type` (required): one of `"info"`, `"warning"`, `"danger"`, `"tip"`
///
/// Body content is rendered inside a styled callout div.
fn builtin_note(args_str: &str, body: Option<&str>) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let note_type = args.get("type").ok_or_else(|| {
        anyhow::anyhow!("note shortcode requires a `type` argument (info, warning, danger, or tip)")
    })?;

    let (icon, title) = match note_type.as_str() {
        "info" => (CALLOUT_ICON_NOTE, "Note"),
        "warning" => (CALLOUT_ICON_WARNING, "Warning"),
        "danger" => (CALLOUT_ICON_CAUTION, "Danger"),
        "tip" => (CALLOUT_ICON_TIP, "Tip"),
        other => {
            return Err(anyhow::anyhow!(
                "note shortcode: unknown type '{other}', expected one of: info, warning, danger, tip"
            ));
        }
    };

    let body = body.ok_or_else(|| anyhow::anyhow!("note shortcode requires a body"))?;

    Ok(format!(
        "<div class=\"callout callout--{note_type}\">\
         <p class=\"callout__title\">{icon} {title}</p>\
         <div class=\"callout__body\">\n\n{body}\n\n</div></div>"
    ))
}

/// Built-in `details` shortcode: collapsible content section.
///
/// Arguments:
/// - `summary` (required): the clickable summary text
/// - `open` (optional): `"true"` to render expanded by default
fn builtin_details(args_str: &str, body: Option<&str>) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let summary = args
        .get("summary")
        .ok_or_else(|| anyhow::anyhow!("details shortcode requires a `summary` argument"))?;
    let body = body.ok_or_else(|| anyhow::anyhow!("details shortcode requires a body"))?;
    let open_attr = if args.get("open").is_some_and(|v| v == "true") {
        " open"
    } else {
        ""
    };

    Ok(format!(
        "<details class=\"details\"{open_attr}>\
         <summary>{}</summary>\
         <div class=\"details__body\">\n\n{body}\n\n</div></details>",
        escape_html(summary)
    ))
}

/// Built-in `figure` shortcode: image with optional caption.
///
/// Arguments:
/// - `src` (required): image URL or path
/// - `alt` (optional): alt text for accessibility
/// - `caption` (optional): caption text displayed below the image
/// - `width` (optional): CSS width value (e.g. `"80%"`, `"400px"`)
fn builtin_figure(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let src = args
        .get("src")
        .ok_or_else(|| anyhow::anyhow!("figure shortcode requires a `src` argument"))?;
    let alt = args.get("alt").map(|s| s.as_str()).unwrap_or("");
    let caption = args.get("caption");
    let width = args.get("width");

    let width_attr = match width {
        Some(w) => format!(r#" style="width: {}""#, escape_html(w)),
        None => String::new(),
    };

    let caption_html = match caption {
        Some(c) => format!("<figcaption>{}</figcaption>", escape_html(c)),
        None => String::new(),
    };

    Ok(format!(
        "<figure class=\"figure\"{width_attr}>\
         <img src=\"{}\" alt=\"{}\" loading=\"lazy\">\
         {caption_html}</figure>",
        escape_html(src),
        escape_html(alt),
    ))
}

/// Built-in `youtube` shortcode: responsive YouTube embed.
///
/// Arguments:
/// - `id` (required): YouTube video ID
///
/// Uses privacy-enhanced mode (`youtube-nocookie.com`).
fn builtin_youtube(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let id = args
        .get("id")
        .ok_or_else(|| anyhow::anyhow!("youtube shortcode requires an `id` argument"))?;

    Ok(format!(
        "<div class=\"youtube\">\
         <iframe src=\"https://www.youtube-nocookie.com/embed/{}\" \
         frameborder=\"0\" \
         allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture\" \
         allowfullscreen loading=\"lazy\"></iframe></div>",
        escape_html(id)
    ))
}

/// Built-in `gist` shortcode: embed a GitHub Gist.
///
/// Arguments:
/// - `url` (required): full GitHub Gist URL (e.g. `"https://gist.github.com/user/abc123"`)
/// - `file` (optional): specific file from the gist to embed
fn builtin_gist(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let url = args
        .get("url")
        .ok_or_else(|| anyhow::anyhow!("gist shortcode requires a `url` argument"))?;

    let file_param = match args.get("file") {
        Some(f) => format!("?file={}", escape_html(f)),
        None => String::new(),
    };

    Ok(format!(
        "<div class=\"gist\"><script src=\"{}.js{file_param}\"></script></div>",
        escape_html(url)
    ))
}

/// Built-in `mermaid` shortcode: render a Mermaid diagram.
///
/// Body content is the Mermaid diagram definition.
fn builtin_mermaid(body: Option<&str>) -> anyhow::Result<String> {
    let body = body.ok_or_else(|| anyhow::anyhow!("mermaid shortcode requires a body"))?;

    Ok(format!(
        "<pre class=\"mermaid\">{}</pre>",
        escape_html(body)
    ))
}

/// Escape HTML special characters for safe attribute/content insertion.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// SVG icons for callout types (used by both `note` shortcode and markdown callouts).
pub(crate) const CALLOUT_ICON_NOTE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"#;

pub(crate) const CALLOUT_ICON_TIP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863.5 8 .5s5.5 1.81 5.5 4.75c0 1.516-.701 2.5-1.328 3.259a10.56 10.56 0 0 0-.268.32c-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z"/></svg>"#;

pub(crate) const CALLOUT_ICON_WARNING: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"#;

pub(crate) const CALLOUT_ICON_CAUTION: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>"#;

pub(crate) const CALLOUT_ICON_IMPORTANT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="currentColor"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>"#;

/// Render a single shortcode
fn render_shortcode(
    name: &str,
    args_str: &str,
    body: Option<&str>,
    shortcode_dir: &Path,
) -> anyhow::Result<String> {
    let template_path = shortcode_dir.join(format!("{name}.html"));
    if !template_path.exists() {
        return Err(anyhow::anyhow!("shortcode template not found: {name}.html"));
    }

    let template_content = std::fs::read_to_string(&template_path)?;
    let args = parse_args(args_str);

    // Build Tera context
    let mut context = tera::Context::new();
    for (k, v) in &args {
        context.insert(k, v);
    }
    if let Some(body) = body {
        context.insert("body", body);
    }

    // Render the shortcode template
    let template_name = format!("shortcodes/{name}.html");
    let mut shortcode_tera = tera::Tera::default();
    shortcode_tera.add_raw_template(&template_name, &template_content)?;
    let rendered = shortcode_tera.render(&template_name, &context)?;

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_shortcode_dir(tmp: &TempDir, name: &str, template: &str) -> std::path::PathBuf {
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.html")), template).unwrap();
        dir
    }

    #[test]
    fn test_inline_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(&tmp, "greeting", "<b>Hello {{ name }}</b>");
        let result = process_shortcodes(
            r#"Before {{ greeting(name="World") }} after"#,
            &dir,
            tmp.path(),
            tmp.path(),
        )
        .unwrap();
        assert!(result.contains("<b>Hello World</b>"));
        assert!(result.starts_with("Before "));
        assert!(result.ends_with(" after"));
    }

    #[test]
    fn test_body_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(&tmp, "alert", r#"<div class="{{ kind }}">{{ body }}</div>"#);
        let result = process_shortcodes(
            r#"{% alert(kind="warning") %}Be careful!{% end %}"#,
            &dir,
            tmp.path(),
            tmp.path(),
        )
        .unwrap();
        assert!(result.contains(r#"<div class="warning">Be careful!</div>"#));
    }

    #[test]
    fn test_no_shortcodes() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = "Plain markdown with no shortcodes";
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_parse_args_double_quotes() {
        let args = parse_args(r#"key="value", other="test""#);
        assert_eq!(args.get("key").unwrap(), "value");
        assert_eq!(args.get("other").unwrap(), "test");
    }

    #[test]
    fn test_parse_args_single_quotes() {
        let args = parse_args("key='value'");
        assert_eq!(args.get("key").unwrap(), "value");
    }

    #[test]
    fn test_missing_shortcode_template_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ missing(key="value") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_include_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n\nWorld").unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="readme.md") }}"#,
            &dir,
            tmp.path(),
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result, "# Hello\n\nWorld");
    }

    #[test]
    fn test_include_missing_path_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="nope.md") }}"#,
            &dir,
            tmp.path(),
            tmp.path(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_include_missing_arg_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(r#"{{ include() }}"#, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tabs_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input =
            r#"{% tabs(labels="Python|Bash") %}print("hello")<!-- tab -->echo hello{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("data-tabs"));
        assert!(result.contains(r#"data-tab-idx="0""#));
        assert!(result.contains(r#"data-tab-idx="1""#));
        assert!(result.contains(">Python</button>"));
        assert!(result.contains(">Bash</button>"));
        assert!(result.contains("tabs__btn--active"));
        assert!(result.contains("tabs__panel--active"));
        assert!(result.contains("print(\"hello\")"));
        assert!(result.contains("echo hello"));
    }

    #[test]
    fn test_tabs_missing_labels_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% tabs() %}content{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tabs_mismatched_count_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% tabs(labels="A|B|C") %}only one{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_include_path_traversal_rejected() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        // Create a file outside the sandbox
        std::fs::write(tmp.path().join("secret.txt"), "top secret").unwrap();
        let result =
            process_shortcodes(r#"{{ include(path="../secret.txt") }}"#, &dir, &site, &site);
        assert!(result.is_err());
    }

    #[test]
    fn test_include_strip_frontmatter_with_plus_in_value() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        // The TOML value contains "+++" which must NOT be treated as a delimiter.
        std::fs::write(
            tmp.path().join("data.md"),
            "+++\ntitle = \"has +++ inside\"\n+++\nActual body",
        )
        .unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="data.md", strip_frontmatter="true") }}"#,
            &dir,
            tmp.path(),
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result.trim(), "Actual body");
    }

    #[test]
    fn test_note_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% note(type="info") %}This is important.{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("callout callout--info"));
        assert!(result.contains("callout__title"));
        assert!(result.contains("This is important."));
    }

    #[test]
    fn test_note_shortcode_invalid_type() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% note(type="invalid") %}text{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown type"));
    }

    #[test]
    fn test_note_shortcode_missing_type() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% note() %}text{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_details_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% details(summary="Click me") %}Hidden content{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("<details"));
        assert!(result.contains("<summary>Click me</summary>"));
        assert!(result.contains("Hidden content"));
        assert!(!result.contains("open"));
    }

    #[test]
    fn test_details_shortcode_open() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% details(summary="Expanded", open="true") %}Content{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains(" open"));
    }

    #[test]
    fn test_figure_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ figure(src="/img/photo.png", alt="A photo", caption="My photo") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("<figure"));
        assert!(result.contains(r#"src="/img/photo.png""#));
        assert!(result.contains(r#"alt="A photo""#));
        assert!(result.contains("<figcaption>My photo</figcaption>"));
    }

    #[test]
    fn test_figure_shortcode_xss_escape() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ figure(src="x\" onload=\"alert(1)", alt="<script>") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(!result.contains("onload"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_youtube_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ youtube(id="dQw4w9WgXcQ") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("youtube-nocookie.com/embed/dQw4w9WgXcQ"));
        assert!(result.contains("allowfullscreen"));
    }

    #[test]
    fn test_gist_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ gist(url="https://gist.github.com/user/abc123") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("https://gist.github.com/user/abc123.js"));
    }

    #[test]
    fn test_gist_shortcode_with_file() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ gist(url="https://gist.github.com/user/abc123", file="hello.py") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("?file=hello.py"));
    }

    #[test]
    fn test_mermaid_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% mermaid() %}graph LR; A-->B{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("<pre class=\"mermaid\">"));
        assert!(result.contains("graph LR; A--&gt;B"));
    }

    #[test]
    fn test_include_within_sandbox_allowed() {
        let tmp = TempDir::new().unwrap();
        // sandbox = tmp, site = tmp/site, shared file = tmp/shared/data.md
        let site = tmp.path().join("site");
        let shared = tmp.path().join("shared");
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("data.md"), "shared content").unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="../shared/data.md") }}"#,
            &dir,
            &site,
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result, "shared content");
    }

    #[test]
    fn test_rewrite_md_links_relative() {
        let content = "[Config](../reference/config.md)";
        let result = rewrite_md_links(content, "../docs/getting-started/installation.md");
        assert_eq!(result, "[Config](/docs/reference/config/)");
    }

    #[test]
    fn test_rewrite_md_links_with_anchor() {
        let content = "[Themes](../concepts/themes.md#built-in)";
        let result = rewrite_md_links(content, "../docs/getting-started/quick-start.md");
        assert_eq!(result, "[Themes](/docs/concepts/themes/#built-in)");
    }

    #[test]
    fn test_rewrite_md_links_readme() {
        let content = "[Concepts](../concepts/README.md)";
        let result = rewrite_md_links(content, "../docs/getting-started/installation.md");
        assert_eq!(result, "[Concepts](/docs/concepts/)");
    }

    #[test]
    fn test_rewrite_md_links_same_dir() {
        let content = "[Quick start](quick-start.md)";
        let result = rewrite_md_links(content, "../docs/getting-started/installation.md");
        assert_eq!(result, "[Quick start](/docs/getting-started/quick-start/)");
    }

    #[test]
    fn test_rewrite_md_links_skips_http() {
        let content = "[Zola](https://github.com/getzola/zola.md)";
        let result = rewrite_md_links(content, "../docs/concepts/themes.md");
        assert_eq!(result, "[Zola](https://github.com/getzola/zola.md)");
    }

    #[test]
    fn test_rewrite_md_links_preserves_non_md() {
        let content = "[Image](photo.jpg) and [Doc](other.md)";
        let result = rewrite_md_links(content, "../docs/concepts/themes.md");
        assert_eq!(
            result,
            "[Image](photo.jpg) and [Doc](/docs/concepts/other/)"
        );
    }

    #[test]
    fn test_rewrite_md_links_integration() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        let docs = tmp.path().join("docs").join("getting-started");
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(
            docs.join("install.md"),
            "See [config](../reference/config.md) for details.",
        )
        .unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="../docs/getting-started/install.md", rewrite_links="true") }}"#,
            &dir,
            &site,
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result, "See [config](/docs/reference/config/) for details.");
    }
}
