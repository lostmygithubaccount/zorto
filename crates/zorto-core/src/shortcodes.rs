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
/// - `pyref(module="...")`: Python API reference (requires `python` feature)
/// - `configref(src="...")`: Config reference from Rust source doc comments
/// - `flow(steps="Label:Desc|Label:Desc|...")`: Horizontal step flow diagram
/// - `layers(items="Title:Desc:badge|...")`: Vertical layered stack diagram
/// - `tree()`: File tree visualization (body content, one line per entry)
/// - `compare(left_title, left, right_title, right)`: Side-by-side comparison cards
/// - `cascade(items="Priority:Label:badge|...")`: Override/priority cascade diagram
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
        "pyref" => builtin_pyref(args_str, site_root),
        "configref" => builtin_configref(args_str, site_root, sandbox_root),
        "flow" => builtin_flow(args_str),
        "layers" => builtin_layers(args_str),
        "tree" => builtin_tree(args_str, body),
        "compare" => builtin_compare(args_str),
        "cascade" => builtin_cascade(args_str),
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
        if !path.starts_with("https://") {
            anyhow::bail!("include shortcode: only https:// URLs are allowed, got: {path}");
        }
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

/// Maximum response size for remote includes (10 MB).
const MAX_INCLUDE_RESPONSE_SIZE: u64 = 10 * 1024 * 1024;

/// Fetch content from a remote URL with size limit.
fn fetch_url(url: &str) -> anyhow::Result<String> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|e| anyhow::anyhow!("include shortcode: failed to fetch {url}: {e}"))?;

    // Check Content-Length header if present
    if let Some(len) = response.headers().get("content-length") {
        if let Ok(len_str) = len.to_str() {
            if let Ok(len) = len_str.parse::<u64>() {
                if len > MAX_INCLUDE_RESPONSE_SIZE {
                    anyhow::bail!(
                        "include shortcode: response from {url} too large ({len} bytes, max {MAX_INCLUDE_RESPONSE_SIZE})"
                    );
                }
            }
        }
    }

    let buf = response
        .body_mut()
        .with_config()
        .limit(MAX_INCLUDE_RESPONSE_SIZE)
        .read_to_vec()
        .map_err(|e| {
            anyhow::anyhow!("include shortcode: failed to read response from {url}: {e}")
        })?;

    String::from_utf8(buf).map_err(|e| {
        anyhow::anyhow!("include shortcode: response from {url} is not valid UTF-8: {e}")
    })
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

    // Only allow gist.github.com URLs to prevent script injection from arbitrary domains
    if !url.starts_with("https://gist.github.com/") {
        anyhow::bail!("gist shortcode: url must be a https://gist.github.com/ URL");
    }

    let file_param = match args.get("file") {
        Some(f) => format!("?file={}", escape_html(f)),
        None => String::new(),
    };

    Ok(format!(
        "<div class=\"gist\"><script src=\"{}.js{file_param}\"></script></div>",
        escape_html(url)
    ))
}

/// Built-in `flow` shortcode: horizontal step flow diagram.
///
/// Arguments:
/// - `steps` (required): pipe-delimited steps, each as "Label:Description" or just "Label"
/// - `caption` (optional): caption text below the diagram
///
/// Example: {{ flow(steps="Write:Markdown|Parse:Find code blocks|Execute:Run code|Render:HTML output") }}
fn builtin_flow(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let steps_str = args
        .get("steps")
        .ok_or_else(|| anyhow::anyhow!("flow shortcode requires a `steps` argument"))?;
    let caption = args.get("caption");

    let steps: Vec<(&str, &str)> = steps_str
        .split('|')
        .map(|s| {
            let s = s.trim();
            match s.split_once(':') {
                Some((label, desc)) => (label.trim(), desc.trim()),
                None => (s, ""),
            }
        })
        .collect();

    let mut html = String::from(
        "<div class=\"cv-visual cv-visual--wide cv-visual--center\">\n\
         <div class=\"cv-flow\">\n",
    );

    for (i, (label, desc)) in steps.iter().enumerate() {
        if i > 0 {
            html.push_str("<div class=\"cv-flow__arrow\">\u{2192}</div>\n");
        }
        let step_class = if i == steps.len() - 1 {
            "cv-flow__step cv-flow__step--green"
        } else if i > 0 {
            "cv-flow__step cv-flow__step--accent"
        } else {
            "cv-flow__step"
        };
        html.push_str(&format!("<div class=\"{step_class}\">"));
        html.push_str(&format!(
            "<div class=\"cv-flow__label\"><strong>{}</strong>",
            escape_html(label)
        ));
        if !desc.is_empty() {
            html.push_str(&escape_html(desc));
        }
        html.push_str("</div></div>\n");
    }

    html.push_str("</div>\n");

    if let Some(cap) = caption {
        html.push_str(&format!(
            "<p class=\"cv-caption\">{}</p>\n",
            escape_html(cap)
        ));
    }

    html.push_str("</div>");
    Ok(html)
}

/// Built-in `layers` shortcode: vertical layered stack diagram.
///
/// Arguments:
/// - `items` (required): pipe-delimited items, each as "Title:Description:badge" or "Title:Description"
/// - `caption` (optional): caption text below the diagram
///
/// Example: {{ layers(items="Identity:Who is this site?:base_url|Build:What outputs?:feeds") }}
fn builtin_layers(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let items_str = args
        .get("items")
        .ok_or_else(|| anyhow::anyhow!("layers shortcode requires an `items` argument"))?;
    let caption = args.get("caption");

    let items: Vec<(&str, &str, &str)> = items_str
        .split('|')
        .map(|s| {
            let s = s.trim();
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            match parts.len() {
                3 => (parts[0].trim(), parts[1].trim(), parts[2].trim()),
                2 => (parts[0].trim(), parts[1].trim(), ""),
                _ => (s, "", ""),
            }
        })
        .collect();

    let mut html = String::from(
        "<div class=\"cv-visual cv-visual--center\">\n\
         <div class=\"cv-layers\">\n",
    );

    for (i, (title, desc, badge)) in items.iter().enumerate() {
        let num = i + 1;
        html.push_str("<div class=\"cv-layers__item\">");
        html.push_str(&format!("<div class=\"cv-layers__num\">{num}</div>"));
        html.push_str("<div class=\"cv-layers__content\">");
        html.push_str(&format!(
            "<div class=\"cv-layers__title\">{}</div>",
            escape_html(title)
        ));
        if !desc.is_empty() {
            html.push_str(&format!(
                "<div class=\"cv-layers__desc\">{}</div>",
                escape_html(desc)
            ));
        }
        html.push_str("</div>");
        if !badge.is_empty() {
            html.push_str(&format!(
                "<span class=\"cv-layers__badge cv-layers__badge--blue\">{}</span>",
                escape_html(badge)
            ));
        }
        html.push_str("</div>\n");
    }

    html.push_str("</div>\n");

    if let Some(cap) = caption {
        html.push_str(&format!(
            "<p class=\"cv-caption\">{}</p>\n",
            escape_html(cap)
        ));
    }

    html.push_str("</div>");
    Ok(html)
}

/// Built-in `tree` shortcode: file tree visualization.
///
/// Arguments:
/// - `caption` (optional): caption text below the tree
///
/// Body: one entry per line, format: "path  # comment  [tag]"
/// Indent with spaces to show nesting. Lines starting with # are ignored.
///
/// Example:
/// {% tree(caption="Directory structure") %}
/// content/
///   _index.md        # root section  [section]
///   about.md         # standalone    [page → /about/]
///   posts/
///     _index.md      # blog section  [section]
///     first-post.md  # a blog post   [page]
/// {% end %}
fn builtin_tree(args_str: &str, body: Option<&str>) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let caption = args.get("caption");
    let body = body.ok_or_else(|| anyhow::anyhow!("tree shortcode requires a body"))?;

    let mut html = String::from(
        "<div class=\"cv-visual cv-visual--center\">\n\
         <div class=\"cv-tree\">\n",
    );

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Count leading spaces for indentation
        let indent = line.len() - line.trim_start().len();
        let depth = indent / 2;

        // Parse: "name  # comment  [tag]"
        let (name_part, comment, tag) = parse_tree_line(trimmed);

        let is_dir = name_part.ends_with('/');
        let is_section_file =
            name_part.contains("_index.md") || tag.to_lowercase().contains("section");
        let icon = if is_dir { "\u{1F4C2}" } else { "\u{1F4DD}" };

        let name_class = if is_dir || is_section_file {
            "cv-tree__name cv-tree__name--section"
        } else if tag.to_lowercase().contains("page") {
            "cv-tree__name cv-tree__name--page"
        } else {
            "cv-tree__name"
        };

        let tag_class = if tag.to_lowercase().contains("section") {
            "cv-tree__tag cv-tree__tag--section"
        } else if tag.to_lowercase().contains("page") {
            "cv-tree__tag cv-tree__tag--page"
        } else {
            "cv-tree__tag cv-tree__tag--url"
        };

        // Build tree prefix based on depth
        let prefix = if depth > 0 {
            let mut p = String::new();
            for _ in 0..depth - 1 {
                p.push_str("\u{00A0}\u{00A0}\u{00A0}\u{00A0}");
            }
            p.push_str("\u{251C}\u{2500}\u{2500} ");
            format!("<span class=\"cv-tree__prefix\">{}</span>", p)
        } else {
            String::new()
        };

        html.push_str("<div class=\"cv-tree__line\">");
        html.push_str(&prefix);
        html.push_str(&format!("<span class=\"cv-tree__icon\">{icon}</span>"));
        html.push_str(&format!(
            "<span class=\"{name_class}\">{}</span>",
            escape_html(&name_part)
        ));

        if !tag.is_empty() {
            html.push_str(&format!(
                "<span class=\"{tag_class}\">{}</span>",
                escape_html(&tag)
            ));
        }

        if !comment.is_empty() && tag.is_empty() {
            html.push_str(&format!(
                "<span class=\"cv-tree__tag cv-tree__tag--url\">{}</span>",
                escape_html(&comment)
            ));
        }

        html.push_str("</div>\n");
    }

    html.push_str("</div>\n");

    if let Some(cap) = caption {
        html.push_str(&format!(
            "<p class=\"cv-caption\">{}</p>\n",
            escape_html(cap)
        ));
    }

    html.push_str("</div>");
    Ok(html)
}

/// Parse a tree line into (name, comment, tag).
/// Format: "filename  # comment  [tag]" or "filename  [tag]" or just "filename"
fn parse_tree_line(line: &str) -> (String, String, String) {
    let mut name = line.to_string();
    let mut comment = String::new();
    let mut tag = String::new();

    // Extract [tag] if present
    if let Some(bracket_start) = name.rfind('[') {
        if let Some(bracket_end) = name.rfind(']') {
            if bracket_end > bracket_start {
                tag = name[bracket_start + 1..bracket_end].trim().to_string();
                name = name[..bracket_start].trim_end().to_string();
            }
        }
    }

    // Extract # comment if present
    if let Some(hash_pos) = name.find(" # ") {
        comment = name[hash_pos + 3..].trim().to_string();
        name = name[..hash_pos].trim_end().to_string();
    }

    (name, comment, tag)
}

/// Built-in `compare` shortcode: side-by-side comparison cards.
///
/// Arguments:
/// - `left_title` (required): title for the left card
/// - `left` (required): body text for the left card
/// - `right_title` (required): title for the right card
/// - `right` (required): body text for the right card
/// - `left_style` (optional): "accent" (blue) or "green" or "muted"
/// - `right_style` (optional): "accent" or "green" or "muted"
/// - `caption` (optional): caption text below
///
/// Example: {{ compare(left_title="Section", left="A directory with _index.md", right_title="Page", right="An individual .md file") }}
fn builtin_compare(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let left_title = args.get("left_title").map(|s| s.as_str()).unwrap_or("");
    let left = args
        .get("left")
        .ok_or_else(|| anyhow::anyhow!("compare shortcode requires a `left` argument"))?;
    let right_title = args.get("right_title").map(|s| s.as_str()).unwrap_or("");
    let right = args
        .get("right")
        .ok_or_else(|| anyhow::anyhow!("compare shortcode requires a `right` argument"))?;
    let left_style = args
        .get("left_style")
        .map(|s| s.as_str())
        .unwrap_or("accent");
    let right_style = args
        .get("right_style")
        .map(|s| s.as_str())
        .unwrap_or("green");
    let caption = args.get("caption");

    let left_class = match left_style {
        "green" => "cv-compare__card cv-compare__card--green",
        "muted" => "cv-compare__card cv-compare__card--muted",
        _ => "cv-compare__card cv-compare__card--accent",
    };
    let right_class = match right_style {
        "accent" => "cv-compare__card cv-compare__card--accent",
        "muted" => "cv-compare__card cv-compare__card--muted",
        _ => "cv-compare__card cv-compare__card--green",
    };

    let mut html = String::from(
        "<div class=\"cv-visual cv-visual--wide cv-visual--center\">\n\
         <div class=\"cv-compare\">\n",
    );

    html.push_str(&format!(
        "<div class=\"{left_class}\">\
         <div class=\"cv-compare__title\">{}</div>\
         <div class=\"cv-compare__body\">{}</div>\
         </div>\n",
        escape_html(left_title),
        escape_html(left)
    ));

    html.push_str(&format!(
        "<div class=\"{right_class}\">\
         <div class=\"cv-compare__title\">{}</div>\
         <div class=\"cv-compare__body\">{}</div>\
         </div>\n",
        escape_html(right_title),
        escape_html(right)
    ));

    html.push_str("</div>\n");

    if let Some(cap) = caption {
        html.push_str(&format!(
            "<p class=\"cv-caption\">{}</p>\n",
            escape_html(cap)
        ));
    }

    html.push_str("</div>");
    Ok(html)
}

/// Built-in `cascade` shortcode: override/priority cascade diagram.
///
/// Arguments:
/// - `items` (required): pipe-delimited items, each as "Priority:Label:badge"
///   The last item is highlighted as the "winner" (green).
/// - `caption` (optional): caption text below
///
/// Example: {{ cascade(items="Default:Theme templates:fallback|Override:Your templates/:wins") }}
fn builtin_cascade(args_str: &str) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let items_str = args
        .get("items")
        .ok_or_else(|| anyhow::anyhow!("cascade shortcode requires an `items` argument"))?;
    let caption = args.get("caption");

    let items: Vec<(&str, &str, &str)> = items_str
        .split('|')
        .map(|s| {
            let s = s.trim();
            let parts: Vec<&str> = s.splitn(3, ':').collect();
            match parts.len() {
                3 => (parts[0].trim(), parts[1].trim(), parts[2].trim()),
                2 => (parts[0].trim(), parts[1].trim(), ""),
                _ => (s, "", ""),
            }
        })
        .collect();

    let mut html = String::from(
        "<div class=\"cv-visual cv-visual--center\">\n\
         <div class=\"cv-cascade\">\n",
    );

    let last_idx = items.len().saturating_sub(1);
    for (i, (priority, label, badge)) in items.iter().enumerate() {
        let level_class = if i == last_idx {
            "cv-cascade__level cv-cascade__level--winner"
        } else {
            "cv-cascade__level"
        };
        let badge_class = if i == last_idx {
            "cv-cascade__badge cv-cascade__badge--wins"
        } else {
            "cv-cascade__badge cv-cascade__badge--default"
        };

        html.push_str(&format!("<div class=\"{level_class}\">"));
        html.push_str(&format!(
            "<span class=\"cv-cascade__priority\">{}</span>",
            escape_html(priority)
        ));
        html.push_str(&format!(
            "<span class=\"cv-cascade__label\">{}</span>",
            escape_html(label)
        ));
        if !badge.is_empty() {
            html.push_str(&format!(
                "<span class=\"{badge_class}\">{}</span>",
                escape_html(badge)
            ));
        }
        html.push_str("</div>\n");
    }

    html.push_str("</div>\n");

    if let Some(cap) = caption {
        html.push_str(&format!(
            "<p class=\"cv-caption\">{}</p>\n",
            escape_html(cap)
        ));
    }

    html.push_str("</div>");
    Ok(html)
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

/// Embedded Python script for introspecting a module's public API.
///
/// Returns a JSON array of items, each with:
/// - `kind`: "function" or "class"
/// - `name`: fully qualified name
/// - `signature`: function/method signature string
/// - `docstring`: first paragraph of the docstring (or empty)
/// - `methods`: (classes only) list of public methods with signature + docstring
#[cfg(feature = "python")]
const PYREF_SCRIPT: &str = r#"
import importlib
import inspect
import json

def _get_sig(obj):
    try:
        return str(inspect.signature(obj))
    except (ValueError, TypeError):
        return "()"

def _get_doc(obj):
    doc = inspect.getdoc(obj)
    if not doc:
        return "", []
    lines = doc.strip().split("\n")
    summary_lines = []
    examples = []
    current_example = []
    in_example = False

    past_summary = False
    for line in lines:
        stripped = line.strip()
        if stripped.startswith(">>> ") or stripped.startswith("... "):
            in_example = True
            past_summary = True
            current_example.append(stripped)
        elif in_example:
            if stripped and not stripped.startswith(">>> "):
                # This is expected output — include it
                current_example.append(stripped)
            else:
                if current_example:
                    examples.append(current_example)
                    current_example = []
                in_example = False
                if stripped.startswith(">>> "):
                    in_example = True
                    current_example.append(stripped)
        elif not past_summary:
            if stripped == "" and summary_lines:
                past_summary = True  # End of first paragraph, keep scanning for examples
            elif stripped:
                summary_lines.append(stripped)

    if current_example:
        examples.append(current_example)

    summary = " ".join(summary_lines).strip()
    return summary, examples

def _run_examples(examples, module_name):
    results = []
    # Shared namespace across all examples for this item
    ns = {}
    exec(f"import {module_name}", ns)

    for block in examples:
        code_lines = []
        expected_lines = []
        for line in block:
            if line.startswith(">>> ") or line.startswith("... "):
                code_lines.append(line[4:])
            elif line.startswith(">>>"):
                code_lines.append(line[3:])
            else:
                expected_lines.append(line)

        code = "\n".join(code_lines)

        import io, contextlib
        stdout = io.StringIO()
        actual = ""
        try:
            with contextlib.redirect_stdout(stdout):
                if len(code_lines) > 1:
                    # Exec all but last line, then eval last line
                    setup = "\n".join(code_lines[:-1])
                    exec(setup, ns)
                    try:
                        result = eval(code_lines[-1], ns)
                        if result is not None:
                            print(repr(result))
                    except SyntaxError:
                        exec(code_lines[-1], ns)
                else:
                    try:
                        result = eval(code, ns)
                        if result is not None:
                            print(repr(result))
                    except SyntaxError:
                        exec(code, ns)
            actual = stdout.getvalue().strip()
        except Exception as e:
            actual = f"Error: {e}"

        results.append({
            "code": code,
            "output": actual,
        })
    return results

def _introspect(module_name, recursive, exclude, include, private):
    try:
        mod = importlib.import_module(module_name)
    except ImportError as e:
        return json.dumps({"error": f"Cannot import module '{module_name}': {e}"})

    exclude_set = set(exclude) if exclude else set()
    include_set = set(include) if include else None

    items = []
    seen = set()

    def process_module(mod, prefix):
        for name in sorted(dir(mod)):
            if name.startswith("__") and name.endswith("__"):
                continue
            if not private and name.startswith("_"):
                continue
            if name in exclude_set:
                continue
            if include_set is not None and name not in include_set:
                continue

            obj = getattr(mod, name, None)
            if obj is None:
                continue

            full_name = f"{prefix}.{name}"
            obj_id = id(obj)
            if obj_id in seen:
                continue
            seen.add(obj_id)

            if inspect.isfunction(obj) or inspect.isbuiltin(obj) or callable(obj) and not inspect.isclass(obj):
                doc, examples = _get_doc(obj)
                example_results = _run_examples(examples, module_name) if examples else []
                items.append({
                    "kind": "function",
                    "name": full_name,
                    "signature": _get_sig(obj),
                    "docstring": doc,
                    "examples": example_results,
                })
            elif inspect.isclass(obj):
                methods = []
                for mname in sorted(dir(obj)):
                    if mname.startswith("__") and mname != "__init__":
                        continue
                    if not private and mname.startswith("_") and mname != "__init__":
                        continue
                    mobj = getattr(obj, mname, None)
                    if mobj is None:
                        continue
                    if not (inspect.isfunction(mobj) or inspect.ismethod(mobj) or inspect.ismethoddescriptor(mobj) or callable(mobj)):
                        continue
                    # Skip unhelpful default __init__
                    mdoc, mexamples = _get_doc(mobj)
                    if mname == "__init__" and ("See help(type(self))" in mdoc or not mdoc):
                        continue
                    mexample_results = _run_examples(mexamples, module_name) if mexamples else []
                    methods.append({
                        "name": mname,
                        "signature": _get_sig(mobj),
                        "docstring": mdoc,
                        "examples": mexample_results,
                    })
                class_doc, class_examples = _get_doc(obj)
                class_example_results = _run_examples(class_examples, module_name) if class_examples else []
                items.append({
                    "kind": "class",
                    "name": full_name,
                    "signature": _get_sig(obj),
                    "docstring": class_doc,
                    "examples": class_example_results,
                    "methods": methods,
                })

        if recursive:
            for name in sorted(dir(mod)):
                obj = getattr(mod, name, None)
                if inspect.ismodule(obj) and hasattr(obj, "__name__") and obj.__name__.startswith(module_name + "."):
                    sub_name = obj.__name__
                    if sub_name not in seen:
                        seen.add(sub_name)
                        process_module(obj, sub_name)

    process_module(mod, module_name)

    # Sort: functions first, then classes, alphabetically within each group
    functions = sorted([i for i in items if i["kind"] == "function"], key=lambda x: x["name"])
    classes = sorted([i for i in items if i["kind"] == "class"], key=lambda x: x["name"])

    return json.dumps(functions + classes)

_result = _introspect(_module_name, _recursive, _exclude, _include, _private)
"#;

/// Built-in `pyref` shortcode: generate Python API reference documentation.
///
/// Arguments:
/// - `module` (required): Python module name to introspect
/// - `recursive` (optional, default "true"): walk submodules
/// - `exclude` (optional): comma-separated names to exclude
/// - `include` (optional): comma-separated allowlist
/// - `private` (optional, default "false"): include _private members
#[cfg(feature = "python")]
fn builtin_pyref(args_str: &str, site_root: &Path) -> anyhow::Result<String> {
    use pyo3::prelude::*;

    let args = parse_args(args_str);
    let module = args
        .get("module")
        .ok_or_else(|| anyhow::anyhow!("pyref shortcode requires a `module` argument"))?;
    let recursive = args.get("recursive").map(|v| v == "true").unwrap_or(true);
    let exclude: Vec<String> = args
        .get("exclude")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let include: Vec<String> = args
        .get("include")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let private = args.get("private").map(|v| v == "true").unwrap_or(false);

    let module = module.clone();
    let site_root = site_root.to_path_buf();

    let json_str = Python::attach(|py: Python<'_>| -> PyResult<String> {
        crate::execute::activate_venv(py, &site_root)?;

        // Set up variables for the script
        let locals = pyo3::types::PyDict::new(py);
        locals.set_item("_module_name", &module)?;
        locals.set_item("_recursive", recursive)?;
        locals.set_item("_private", private)?;

        let exclude_list = pyo3::types::PyList::new(py, &exclude)?;
        locals.set_item("_exclude", exclude_list)?;

        let include_list = pyo3::types::PyList::new(py, &include)?;
        locals.set_item("_include", include_list)?;

        let code = std::ffi::CString::new(PYREF_SCRIPT)?;
        // Pass locals as both globals and locals so imports/functions share scope
        py.run(code.as_c_str(), Some(&locals), Some(&locals))?;

        let result: String = locals
            .get_item("_result")?
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err("No result from pyref script")
            })?
            .extract()?;
        Ok(result)
    })?;

    // Parse the JSON result
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| anyhow::anyhow!("pyref: failed to parse introspection result: {e}"))?;

    // Check for error — gracefully degrade if module can't be imported
    if let Some(obj) = parsed.as_object() {
        if let Some(err) = obj.get("error") {
            let msg = err.as_str().unwrap_or("unknown error");
            eprintln!("pyref warning: {msg}");
            return Ok(format!(
                "<div class=\"pyref\"><p><em>Python API reference unavailable: {msg}</em></p></div>"
            ));
        }
    }

    let items = parsed
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("pyref: expected array from introspection"))?;

    // Build HTML
    let mut html = String::from("<div class=\"pyref\">\n");

    for item in items {
        let kind = item["kind"].as_str().unwrap_or("");
        let name = item["name"].as_str().unwrap_or("");
        let signature = item["signature"].as_str().unwrap_or("()");
        let docstring = item["docstring"].as_str().unwrap_or("");

        // Short name (last component) for display in signatures
        let short_name = name.rsplit('.').next().unwrap_or(name);

        match kind {
            "function" => {
                html.push_str(&format!(
                    "  <div class=\"pyref-item\">\n    <h3 id=\"{name}\"><code>{name}{sig}</code></h3>\n",
                    name = escape_html(name),
                    sig = escape_html(signature),
                ));
                if !docstring.is_empty() {
                    html.push_str(&format!(
                        "    <p class=\"pyref-docstring\">{}</p>\n",
                        escape_html(docstring)
                    ));
                }
                if let Some(examples) = item["examples"].as_array() {
                    if !examples.is_empty() {
                        html.push_str("    <div class=\"pyref-examples\">\n");
                        for ex in examples {
                            let code = ex["code"].as_str().unwrap_or("");
                            let output = ex["output"].as_str().unwrap_or("");
                            html.push_str(&format!(
                                "      <pre class=\"pyref-example-code\"><code>{}</code></pre>\n",
                                escape_html(code)
                            ));
                            if !output.is_empty() {
                                html.push_str(&format!(
                                    "      <pre class=\"pyref-example-output\"><code>{}</code></pre>\n",
                                    escape_html(output)
                                ));
                            }
                        }
                        html.push_str("    </div>\n");
                    }
                }
                html.push_str("  </div>\n");
            }
            "class" => {
                html.push_str(&format!(
                    "  <div class=\"pyref-item pyref-class\">\n    <h3 id=\"{name}\"><code>class {name}{sig}</code></h3>\n",
                    name = escape_html(name),
                    sig = escape_html(signature),
                ));
                if !docstring.is_empty() {
                    html.push_str(&format!(
                        "    <p class=\"pyref-docstring\">{}</p>\n",
                        escape_html(docstring)
                    ));
                }
                if let Some(examples) = item["examples"].as_array() {
                    if !examples.is_empty() {
                        html.push_str("    <div class=\"pyref-examples\">\n");
                        for ex in examples {
                            let code = ex["code"].as_str().unwrap_or("");
                            let output = ex["output"].as_str().unwrap_or("");
                            html.push_str(&format!(
                                "      <pre class=\"pyref-example-code\"><code>{}</code></pre>\n",
                                escape_html(code)
                            ));
                            if !output.is_empty() {
                                html.push_str(&format!(
                                    "      <pre class=\"pyref-example-output\"><code>{}</code></pre>\n",
                                    escape_html(output)
                                ));
                            }
                        }
                        html.push_str("    </div>\n");
                    }
                }

                // Methods
                if let Some(methods) = item["methods"].as_array() {
                    if !methods.is_empty() {
                        html.push_str("    <div class=\"pyref-methods\">\n");
                        for method in methods {
                            let mname = method["name"].as_str().unwrap_or("");
                            let msig = method["signature"].as_str().unwrap_or("()");
                            let mdoc = method["docstring"].as_str().unwrap_or("");
                            let method_id = format!("{}.{}", name, mname);

                            html.push_str(&format!(
                                "      <div class=\"pyref-method\">\n        <h4 id=\"{mid}\"><code>{sn}.{mn}{ms}</code></h4>\n",
                                mid = escape_html(&method_id),
                                sn = escape_html(short_name),
                                mn = escape_html(mname),
                                ms = escape_html(msig),
                            ));
                            if !mdoc.is_empty() {
                                html.push_str(&format!(
                                    "        <p class=\"pyref-docstring\">{}</p>\n",
                                    escape_html(mdoc)
                                ));
                            }
                            if let Some(mexamples) = method["examples"].as_array() {
                                if !mexamples.is_empty() {
                                    html.push_str("        <div class=\"pyref-examples\">\n");
                                    for ex in mexamples {
                                        let code = ex["code"].as_str().unwrap_or("");
                                        let output = ex["output"].as_str().unwrap_or("");
                                        html.push_str(&format!(
                                            "          <pre class=\"pyref-example-code\"><code>{}</code></pre>\n",
                                            escape_html(code)
                                        ));
                                        if !output.is_empty() {
                                            html.push_str(&format!(
                                                "          <pre class=\"pyref-example-output\"><code>{}</code></pre>\n",
                                                escape_html(output)
                                            ));
                                        }
                                    }
                                    html.push_str("        </div>\n");
                                }
                            }
                            html.push_str("      </div>\n");
                        }
                        html.push_str("    </div>\n");
                    }
                }

                html.push_str("  </div>\n");
            }
            _ => {}
        }
    }

    html.push_str("</div>");
    Ok(html)
}

/// Built-in `pyref` shortcode fallback when Python feature is not available.
#[cfg(not(feature = "python"))]
fn builtin_pyref(_args_str: &str, _site_root: &Path) -> anyhow::Result<String> {
    Err(anyhow::anyhow!(
        "pyref shortcode requires the `python` feature (build with --features python)"
    ))
}

// ---------------------------------------------------------------------------
// configref shortcode — auto-generate config reference from Rust source
// ---------------------------------------------------------------------------

struct ConfigStruct {
    #[allow(dead_code)]
    name: String,
    display_name: String,
    anchor: String,
    doc: String,
    fields: Vec<ConfigField>,
}

struct ConfigField {
    name: String,
    ty: String,
    default: Option<String>,
    doc: String,
}

/// Built-in `configref` shortcode: generate configuration reference from Rust
/// source doc comments and serde attributes.
///
/// Arguments:
/// - `src` (required): path to the Rust source file relative to site root
fn builtin_configref(
    args_str: &str,
    site_root: &Path,
    sandbox_root: &Path,
) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let src = args
        .get("src")
        .ok_or_else(|| anyhow::anyhow!("configref shortcode requires a `src` argument"))?;

    let source = read_local_file(src, site_root, sandbox_root)?;
    let structs = parse_rust_config(&source);

    let mut html = String::new();
    for s in &structs {
        html.push_str(&format!(
            "<h2 id=\"{}\">{}</h2>\n",
            escape_html(&s.anchor),
            escape_html(&s.display_name)
        ));
        if !s.doc.is_empty() {
            html.push_str(&format!("<p>{}</p>\n", escape_html(&s.doc)));
        }
        html.push_str("<table>\n<thead><tr><th>Field</th><th>Type</th><th>Default</th><th>Description</th></tr></thead>\n<tbody>\n");
        for field in &s.fields {
            let default_cell = field
                .default
                .as_deref()
                .map(|d| format!("<code>{}</code>", escape_html(d)))
                .unwrap_or_else(|| "<em>required</em>".to_string());
            html.push_str(&format!(
                "<tr><td><code>{}</code></td><td><code>{}</code></td><td>{}</td><td>{}</td></tr>\n",
                escape_html(&field.name),
                escape_html(&field.ty),
                default_cell,
                escape_html(&field.doc),
            ));
        }
        html.push_str("</tbody></table>\n");
    }

    Ok(html)
}

/// Map a Rust struct name to its TOML display name and anchor.
fn config_display_name(name: &str) -> Option<(&'static str, &'static str)> {
    match name {
        "Config" => Some(("Top-level settings", "top-level-settings")),
        "MarkdownConfig" => Some(("[markdown]", "markdown")),
        "TaxonomyConfig" => Some(("[[taxonomies]]", "taxonomies")),
        "ContentDirConfig" => Some(("[[content_dirs]]", "content-dirs")),
        _ => None,
    }
}

/// Clean up a Rust type for display, returning `None` if the field should be skipped.
fn clean_type(ty: &str) -> Option<String> {
    let ty = ty.trim();
    match ty {
        "String" => Some("string".to_string()),
        "bool" => Some("bool".to_string()),
        "MarkdownConfig" | "Vec<TaxonomyConfig>" | "Vec<ContentDirConfig>" => None,
        _ if ty.starts_with("Option<") && ty.ends_with('>') => {
            let inner = &ty[7..ty.len() - 1];
            clean_type(inner)
        }
        _ if ty.starts_with("Vec<") && ty.ends_with('>') => {
            let inner = &ty[4..ty.len() - 1];
            clean_type(inner).map(|t| format!("{t}[]"))
        }
        "toml::Value" => Some("table".to_string()),
        "AnchorLinks" => Some("string".to_string()),
        "SortBy" => Some("string".to_string()),
        _ => Some(ty.to_lowercase()),
    }
}

/// Derive the default value from serde attributes and the field type.
fn derive_default(serde_attrs: &[String], ty: &str, is_option: bool) -> Option<String> {
    for attr in serde_attrs {
        if attr.contains("default = \"default_true\"") {
            return Some("true".to_string());
        }
        if attr.contains("default = \"default_en\"") {
            return Some("\"en\"".to_string());
        }
        if attr.contains("default = \"default_toml_table\"") {
            return Some("{}".to_string());
        }
        if attr.contains("default = \"default_page_html\"") {
            return Some("\"page.html\"".to_string());
        }
        if attr.contains("default = \"default_section_html\"") {
            return Some("\"section.html\"".to_string());
        }
        // Generic #[serde(default)] — derive from type
        if attr.contains("default") {
            let clean = ty.trim();
            if clean == "bool" {
                return Some("false".to_string());
            }
            if clean == "String" {
                return Some("\"\"".to_string());
            }
            if clean.starts_with("Option<") {
                return Some("null".to_string());
            }
            if clean.starts_with("Vec<") {
                return Some("[]".to_string());
            }
            if clean == "AnchorLinks" {
                return Some("\"none\"".to_string());
            }
            if clean == "SortBy" {
                return Some("\"date\"".to_string());
            }
            return Some("\"\"".to_string());
        }
    }
    if is_option {
        Some("null".to_string())
    } else {
        None // required
    }
}

/// Parse Rust source to extract config structs, fields, doc comments, and serde attributes.
fn parse_rust_config(source: &str) -> Vec<ConfigStruct> {
    let mut structs = Vec::new();
    let mut current_struct: Option<ConfigStruct> = None;
    let mut doc_lines: Vec<String> = Vec::new();
    let mut serde_attrs: Vec<String> = Vec::new();
    let mut in_struct = false;
    let mut brace_depth: i32 = 0;

    for line in source.lines() {
        let trimmed = line.trim();

        // Collect doc comments
        if trimmed.starts_with("///") {
            let comment = trimmed.strip_prefix("///").unwrap_or("").trim().to_string();
            doc_lines.push(comment);
            continue;
        }

        // Collect serde attributes
        if trimmed.starts_with("#[serde(") {
            serde_attrs.push(trimmed.to_string());
            continue;
        }

        // Skip other attributes
        if trimmed.starts_with("#[") {
            continue;
        }

        // Detect struct start
        if trimmed.starts_with("pub struct ") && trimmed.contains('{') {
            let name = trimmed
                .strip_prefix("pub struct ")
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            if let Some((display, anchor)) = config_display_name(&name) {
                let doc = doc_lines.join(" ");
                current_struct = Some(ConfigStruct {
                    name,
                    display_name: display.to_string(),
                    anchor: anchor.to_string(),
                    doc,
                    fields: Vec::new(),
                });
                in_struct = true;
                brace_depth = 1;
            }
            doc_lines.clear();
            serde_attrs.clear();
            continue;
        }

        if in_struct {
            // Track brace depth
            for ch in trimmed.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    _ => {}
                }
            }

            // Parse field: `pub field_name: Type,`
            if trimmed.starts_with("pub ") && trimmed.contains(':') {
                let without_pub = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
                if let Some((field_name, rest)) = without_pub.split_once(':') {
                    let field_name = field_name.trim().to_string();
                    let ty = rest.trim().trim_end_matches(',').trim().to_string();
                    let is_option = ty.starts_with("Option<");

                    if let Some(clean_ty) = clean_type(&ty) {
                        let default = derive_default(&serde_attrs, &ty, is_option);
                        let doc = doc_lines.join(" ");

                        if let Some(ref mut cs) = current_struct {
                            cs.fields.push(ConfigField {
                                name: field_name,
                                ty: clean_ty,
                                default,
                                doc,
                            });
                        }
                    }
                }
                doc_lines.clear();
                serde_attrs.clear();
            } else if !trimmed.is_empty() {
                // Non-field line inside struct — reset accumulators unless comment
                if !trimmed.starts_with("//") {
                    doc_lines.clear();
                    serde_attrs.clear();
                }
            }

            if brace_depth == 0 {
                if let Some(cs) = current_struct.take() {
                    structs.push(cs);
                }
                in_struct = false;
            }
        } else {
            // Outside struct — reset accumulators on non-doc/attr lines
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                doc_lines.clear();
                serde_attrs.clear();
            }
        }
    }

    structs
}

use crate::content::escape_html;

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
        return Err(anyhow::anyhow!(
            "shortcode template not found: {name}.html (expected at {})",
            template_path.display()
        ));
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

    // --- Security tests for include shortcode ---

    #[test]
    fn test_include_rejects_http_urls() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="http://evil.com/payload.txt") }}"#,
            &dir,
            tmp.path(),
            tmp.path(),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("only https://"),
            "expected https-only error, got: {err}"
        );
    }

    #[test]
    fn test_include_sandbox_escape_blocked() {
        let tmp = TempDir::new().unwrap();
        let sandbox = tmp.path().join("sandbox");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&sandbox).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), "top secret").unwrap();
        let dir = sandbox.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="../outside/secret.txt") }}"#,
            &dir,
            &sandbox,
            &sandbox,
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("sandbox boundary") || err.contains("cannot resolve"),
            "expected sandbox escape error, got: {err}"
        );
    }

    #[test]
    fn test_include_path_traversal_blocked() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(&site).unwrap();
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        // Try to escape via ../../etc/passwd
        let result = process_shortcodes(
            r#"{{ include(path="../../etc/passwd") }}"#,
            &dir,
            &site,
            &site,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_include_missing_path_arg() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(r#"{{ include() }}"#, &dir, tmp.path(), tmp.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `path` argument")
        );
    }

    #[test]
    fn test_include_valid_local_file() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(site.join("data.txt"), "hello world").unwrap();
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result =
            process_shortcodes(r#"{{ include(path="data.txt") }}"#, &dir, &site, tmp.path())
                .unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_include_strip_frontmatter() {
        let tmp = TempDir::new().unwrap();
        let site = tmp.path().join("site");
        std::fs::create_dir_all(&site).unwrap();
        std::fs::write(
            site.join("page.md"),
            "+++\ntitle = \"T\"\n+++\nContent here",
        )
        .unwrap();
        let dir = site.join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(
            r#"{{ include(path="page.md", strip_frontmatter="true") }}"#,
            &dir,
            &site,
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result.trim(), "Content here");
    }

    // --- Security tests for gist shortcode ---

    #[test]
    fn test_gist_valid_url() {
        let result = builtin_gist(r#"url="https://gist.github.com/user/abc123""#).unwrap();
        assert!(result.contains("gist.github.com/user/abc123.js"));
        assert!(result.starts_with("<div class=\"gist\">"));
    }

    #[test]
    fn test_gist_rejects_non_github_url() {
        let result = builtin_gist(r#"url="https://evil.com/malicious.js""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("gist.github.com"));
    }

    #[test]
    fn test_gist_rejects_http_url() {
        let result = builtin_gist(r#"url="http://gist.github.com/user/abc""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_gist_rejects_javascript_url() {
        let result = builtin_gist(r#"url="javascript:alert(1)""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_gist_missing_url() {
        let result = builtin_gist(r#""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `url` argument")
        );
    }

    #[test]
    fn test_gist_html_escapes_url() {
        let result =
            builtin_gist(r#"url="https://gist.github.com/user/abc"><script>alert(1)</script>""#);
        // Should either reject the URL or escape the HTML
        if let Ok(html) = result {
            assert!(!html.contains("<script>"));
        }
    }

    #[test]
    fn test_gist_with_file_param() {
        let result =
            builtin_gist(r#"url="https://gist.github.com/user/abc123", file="test.py""#).unwrap();
        assert!(result.contains("?file=test.py"));
    }

    #[test]
    fn test_gist_file_param_html_escaped() {
        let result = builtin_gist(
            r#"url="https://gist.github.com/user/abc", file="<script>alert(1)</script>""#,
        )
        .unwrap();
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }

    // --- strip_toml_frontmatter tests ---

    #[test]
    fn test_strip_toml_frontmatter_basic() {
        let input = "+++\ntitle = \"Test\"\n+++\nBody content";
        assert_eq!(strip_toml_frontmatter(input).trim(), "Body content");
    }

    #[test]
    fn test_strip_toml_frontmatter_no_frontmatter() {
        let input = "Just plain content";
        assert_eq!(strip_toml_frontmatter(input), "Just plain content");
    }

    #[test]
    fn test_strip_toml_frontmatter_unclosed() {
        let input = "+++\ntitle = \"Oops\"\nNo closing";
        assert_eq!(strip_toml_frontmatter(input), input);
    }

    #[test]
    fn test_strip_toml_frontmatter_special_chars() {
        let input = "+++\ntitle = \"<script>alert('xss')</script>\"\n+++\nSafe body";
        assert_eq!(strip_toml_frontmatter(input).trim(), "Safe body");
    }

    // --- Additional built-in shortcode tests ---

    #[test]
    fn test_flow_shortcode_basic() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ flow(steps="Write:Markdown|Parse:Find blocks|Render:HTML") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("cv-flow"));
        assert!(result.contains("Write"));
        assert!(result.contains("Markdown"));
        assert!(result.contains("Parse"));
        assert!(result.contains("Render"));
        assert!(result.contains("\u{2192}")); // arrow
    }

    #[test]
    fn test_flow_shortcode_missing_steps_errors() {
        let result = builtin_flow(r#""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `steps`")
        );
    }

    #[test]
    fn test_flow_shortcode_with_caption() {
        let result = builtin_flow(r#"steps="A|B", caption="Build pipeline""#).unwrap();
        assert!(result.contains("cv-caption"));
        assert!(result.contains("Build pipeline"));
    }

    #[test]
    fn test_layers_shortcode_basic() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{{ layers(items="Identity:Who?:base|Build:What?:feeds") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("cv-layers"));
        assert!(result.contains("Identity"));
        assert!(result.contains("Who?"));
        assert!(result.contains("base"));
        assert!(result.contains("Build"));
    }

    #[test]
    fn test_layers_shortcode_missing_items_errors() {
        let result = builtin_layers(r#""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires an `items`")
        );
    }

    #[test]
    fn test_compare_shortcode_basic() {
        let result = builtin_compare(
            r#"left_title="Section", left="A directory", right_title="Page", right="A file""#,
        )
        .unwrap();
        assert!(result.contains("cv-compare"));
        assert!(result.contains("Section"));
        assert!(result.contains("A directory"));
        assert!(result.contains("Page"));
        assert!(result.contains("A file"));
        // Default styles
        assert!(result.contains("cv-compare__card--accent"));
        assert!(result.contains("cv-compare__card--green"));
    }

    #[test]
    fn test_compare_shortcode_missing_left_errors() {
        let result = builtin_compare(r#"right="only right""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `left`")
        );
    }

    #[test]
    fn test_compare_shortcode_missing_right_errors() {
        let result = builtin_compare(r#"left="only left""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `right`")
        );
    }

    #[test]
    fn test_compare_shortcode_custom_styles() {
        let result = builtin_compare(
            r#"left_title="A", left="a", right_title="B", right="b", left_style="muted", right_style="accent""#,
        )
        .unwrap();
        assert!(result.contains("cv-compare__card--muted"));
        assert!(result.contains("cv-compare__card--accent"));
    }

    #[test]
    fn test_cascade_shortcode_basic() {
        let result = builtin_cascade(
            r#"items="Default:Theme templates:fallback|Override:Your templates:wins""#,
        )
        .unwrap();
        assert!(result.contains("cv-cascade"));
        assert!(result.contains("Default"));
        assert!(result.contains("Theme templates"));
        assert!(result.contains("Override"));
        // Last item is the winner
        assert!(result.contains("cv-cascade__level--winner"));
        assert!(result.contains("cv-cascade__badge--wins"));
    }

    #[test]
    fn test_cascade_shortcode_missing_items_errors() {
        let result = builtin_cascade(r#""#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires an `items`")
        );
    }

    #[test]
    fn test_tree_shortcode_basic() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = "{% tree() %}content/\n  _index.md\n  posts/\n    hello.md{% end %}";
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("cv-tree"));
        assert!(result.contains("content/"));
        assert!(result.contains("_index.md"));
        assert!(result.contains("hello.md"));
    }

    #[test]
    fn test_tree_shortcode_missing_body_errors() {
        let result = builtin_tree(r#""#, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a body"));
    }

    #[test]
    fn test_mermaid_missing_body_errors() {
        let result = builtin_mermaid(None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a body"));
    }

    #[test]
    fn test_mermaid_escapes_html() {
        let result = builtin_mermaid(Some("A --> B<script>")).unwrap();
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn test_details_missing_summary_errors() {
        let result = builtin_details(r#""#, Some("body"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires a `summary`")
        );
    }

    #[test]
    fn test_details_missing_body_errors() {
        let result = builtin_details(r#"summary="Click""#, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a body"));
    }

    #[test]
    fn test_details_escapes_summary_html() {
        let result =
            builtin_details(r#"summary="<script>alert(1)</script>""#, Some("safe body")).unwrap();
        assert!(!result.contains("<script>"));
        assert!(result.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_figure_missing_src_errors() {
        let result = builtin_figure(r#"alt="no src""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a `src`"));
    }

    #[test]
    fn test_figure_with_width() {
        let result = builtin_figure(r#"src="/img/pic.png", width="80%""#).unwrap();
        assert!(result.contains("width: 80%"));
    }

    #[test]
    fn test_figure_no_caption() {
        let result = builtin_figure(r#"src="/img/pic.png""#).unwrap();
        assert!(!result.contains("figcaption"));
    }

    #[test]
    fn test_youtube_missing_id_errors() {
        let result = builtin_youtube(r#""#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires an `id`"));
    }

    #[test]
    fn test_youtube_escapes_id() {
        let result = builtin_youtube(r#"id="x\" onload=\"alert(1)""#).unwrap();
        assert!(!result.contains("onload"));
    }

    #[test]
    fn test_note_all_types() {
        for note_type in &["info", "warning", "danger", "tip"] {
            let result = builtin_note(&format!(r#"type="{note_type}""#), Some("body")).unwrap();
            assert!(result.contains(&format!("callout--{note_type}")));
        }
    }

    #[test]
    fn test_note_missing_body_errors() {
        let result = builtin_note(r#"type="info""#, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a body"));
    }

    #[test]
    fn test_tabs_missing_body_errors() {
        let result = builtin_tabs(r#"labels="A|B""#, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires a body"));
    }

    // --- Nested shortcode test ---

    #[test]
    fn test_nested_body_shortcodes() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        // Nest a note inside a details
        let input =
            r#"{% details(summary="Expand") %}{% note(type="info") %}Inner note{% end %}{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains("<details"));
        assert!(result.contains("callout callout--info"));
        assert!(result.contains("Inner note"));
    }

    // --- Custom template shortcode tests ---

    #[test]
    fn test_custom_shortcode_with_body() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(
            &tmp,
            "wrapper",
            r#"<div class="{{ cls }}">{{ body }}</div>"#,
        );
        let input = r#"{% wrapper(cls="highlight") %}wrapped content{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains(r#"<div class="highlight">wrapped content</div>"#));
    }

    #[test]
    fn test_custom_shortcode_multiple_args() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(
            &tmp,
            "badge",
            r#"<span class="{{ color }}">{{ text }}</span>"#,
        );
        let input = r#"{{ badge(color="red", text="New") }}"#;
        let result = process_shortcodes(input, &dir, tmp.path(), tmp.path()).unwrap();
        assert!(result.contains(r#"<span class="red">New</span>"#));
    }

    // --- parse_args edge cases ---

    #[test]
    fn test_parse_args_empty() {
        let args = parse_args("");
        assert!(args.is_empty());
    }

    #[test]
    fn test_parse_args_mixed_quotes() {
        let args = parse_args(r#"a="double", b='single'"#);
        assert_eq!(args.get("a").unwrap(), "double");
        assert_eq!(args.get("b").unwrap(), "single");
    }

    #[test]
    fn test_parse_args_spaces_in_values() {
        let args = parse_args(r#"title="Hello World""#);
        assert_eq!(args.get("title").unwrap(), "Hello World");
    }
}
