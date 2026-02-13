use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
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
pub fn process_shortcodes(
    content: &str,
    shortcode_dir: &Path,
    site_root: &Path,
) -> anyhow::Result<String> {
    // Process body shortcodes first (they can contain inline shortcodes)
    let result = process_body_shortcodes(content, shortcode_dir, site_root)?;

    // Then process inline shortcodes
    process_inline_shortcodes(&result, shortcode_dir, site_root)
}

/// Process body shortcodes: {% name(args) %}...{% end %}
fn process_body_shortcodes(
    content: &str,
    shortcode_dir: &Path,
    site_root: &Path,
) -> anyhow::Result<String> {
    let mut result = content.to_string();
    let mut iterations = 0;

    // Loop to handle nested shortcodes
    while BODY_SHORTCODE_RE.is_match(&result) && iterations < 10 {
        let mut error: Option<anyhow::Error> = None;
        let new_result = BODY_SHORTCODE_RE.replace_all(&result, |caps: &regex::Captures| {
            let name = &caps[1];
            let args_str = &caps[2];
            let body = &caps[3];

            match resolve_shortcode(name, args_str, Some(body.trim()), shortcode_dir, site_root) {
                Ok(rendered) => rendered,
                Err(e) => {
                    error = Some(anyhow::anyhow!("shortcode error in {name}: {e}"));
                    caps[0].to_string()
                }
            }
        });
        if let Some(e) = error {
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
) -> anyhow::Result<String> {
    let mut error: Option<anyhow::Error> = None;
    let result = INLINE_SHORTCODE_RE.replace_all(content, |caps: &regex::Captures| {
        let name = &caps[1];
        let args_str = &caps[2];

        match resolve_shortcode(name, args_str, None, shortcode_dir, site_root) {
            Ok(rendered) => rendered,
            Err(e) => {
                error = Some(anyhow::anyhow!("shortcode error in {name}: {e}"));
                caps[0].to_string()
            }
        }
    });

    if let Some(e) = error {
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
) -> anyhow::Result<String> {
    match name {
        "include" => builtin_include(args_str, site_root),
        "tabs" => builtin_tabs(args_str, body),
        _ => render_shortcode(name, args_str, body, shortcode_dir),
    }
}

/// Built-in `include` shortcode: read file contents relative to site root.
///
/// Arguments:
/// - `path` (required): file path relative to site root
/// - `strip_frontmatter` (optional): "true" to strip `+++`-delimited TOML frontmatter
fn builtin_include(args_str: &str, site_root: &Path) -> anyhow::Result<String> {
    let args = parse_args(args_str);
    let path = args
        .get("path")
        .ok_or_else(|| anyhow::anyhow!("include shortcode requires a `path` argument"))?;
    let file_path = site_root.join(path);
    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        anyhow::anyhow!(
            "include shortcode: cannot read {}: {e}",
            file_path.display()
        )
    })?;

    let strip = args.get("strip_frontmatter").is_some_and(|v| v == "true");
    if strip {
        Ok(strip_toml_frontmatter(&content))
    } else {
        Ok(content)
    }
}

/// Strip `+++`-delimited TOML frontmatter from content.
fn strip_toml_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if let Some(rest) = trimmed.strip_prefix("+++")
        && let Some(after) = rest.find("+++")
    {
        return rest[after + 3..].to_string();
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
        )
        .unwrap();
        assert!(result.contains("<b>Hello World</b>"));
        assert!(result.starts_with("Before "));
        assert!(result.ends_with(" after"));
    }

    #[test]
    fn test_body_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(&tmp, "note", r#"<div class="{{ kind }}">{{ body }}</div>"#);
        let result = process_shortcodes(
            r#"{% note(kind="warning") %}Be careful!{% end %}"#,
            &dir,
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
        let result = process_shortcodes(input, &dir, tmp.path()).unwrap();
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
        let result = process_shortcodes(input, &dir, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_include_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(tmp.path().join("readme.md"), "# Hello\n\nWorld").unwrap();
        let result =
            process_shortcodes(r#"{{ include(path="readme.md") }}"#, &dir, tmp.path()).unwrap();
        assert_eq!(result, "# Hello\n\nWorld");
    }

    #[test]
    fn test_include_missing_path_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(r#"{{ include(path="nope.md") }}"#, &dir, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_include_missing_arg_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let result = process_shortcodes(r#"{{ include() }}"#, &dir, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tabs_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input =
            r#"{% tabs(labels="Python|Bash") %}print("hello")<!-- tab -->echo hello{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path()).unwrap();
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
        let result = process_shortcodes(input, &dir, tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tabs_mismatched_count_errors() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = r#"{% tabs(labels="A|B|C") %}only one{% end %}"#;
        let result = process_shortcodes(input, &dir, tmp.path());
        assert!(result.is_err());
    }
}
