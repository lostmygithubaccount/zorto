use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

static BODY_SHORTCODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\{%\s*(\w+)\s*\(([^)]*)\)\s*%\}(.*?)\{%\s*end\s*%\}").unwrap()
});
static INLINE_SHORTCODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{\s*(\w+)\s*\(([^)]*)\)\s*\}\}").unwrap());
static ARGS_DOUBLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).unwrap());
static ARGS_SINGLE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\w+)\s*=\s*'([^']*)'").unwrap());

/// Process shortcodes in raw markdown content before markdown rendering.
///
/// Inline shortcodes: {{ name(key="value", key2="value2") }}
/// Body shortcodes: {% name(key="value") %}...body...{% end %}
pub fn process_shortcodes(content: &str, shortcode_dir: &Path) -> anyhow::Result<String> {
    // Process body shortcodes first (they can contain inline shortcodes)
    let result = process_body_shortcodes(content, shortcode_dir)?;

    // Then process inline shortcodes
    process_inline_shortcodes(&result, shortcode_dir)
}

/// Process body shortcodes: {% name(args) %}...{% end %}
fn process_body_shortcodes(content: &str, shortcode_dir: &Path) -> anyhow::Result<String> {
    let mut result = content.to_string();
    let mut iterations = 0;

    // Loop to handle nested shortcodes
    while BODY_SHORTCODE_RE.is_match(&result) && iterations < 10 {
        let mut error: Option<anyhow::Error> = None;
        let new_result = BODY_SHORTCODE_RE.replace_all(&result, |caps: &regex::Captures| {
            let name = &caps[1];
            let args_str = &caps[2];
            let body = &caps[3];

            match render_shortcode(name, args_str, Some(body.trim()), shortcode_dir) {
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
fn process_inline_shortcodes(content: &str, shortcode_dir: &Path) -> anyhow::Result<String> {
    let mut error: Option<anyhow::Error> = None;
    let result = INLINE_SHORTCODE_RE.replace_all(content, |caps: &regex::Captures| {
        let name = &caps[1];
        let args_str = &caps[2];

        match render_shortcode(name, args_str, None, shortcode_dir) {
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
        let result =
            process_shortcodes(r#"Before {{ greeting(name="World") }} after"#, &dir).unwrap();
        assert!(result.contains("<b>Hello World</b>"));
        assert!(result.starts_with("Before "));
        assert!(result.ends_with(" after"));
    }

    #[test]
    fn test_body_shortcode() {
        let tmp = TempDir::new().unwrap();
        let dir = setup_shortcode_dir(&tmp, "note", r#"<div class="{{ kind }}">{{ body }}</div>"#);
        let result =
            process_shortcodes(r#"{% note(kind="warning") %}Be careful!{% end %}"#, &dir).unwrap();
        assert!(result.contains(r#"<div class="warning">Be careful!</div>"#));
    }

    #[test]
    fn test_no_shortcodes() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("shortcodes");
        std::fs::create_dir_all(&dir).unwrap();
        let input = "Plain markdown with no shortcodes";
        let result = process_shortcodes(input, &dir).unwrap();
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
        let result = process_shortcodes(input, &dir);
        assert!(result.is_err());
    }
}
