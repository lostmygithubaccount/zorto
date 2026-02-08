use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

use crate::config::MarkdownConfig;
use crate::execute::ExecutableBlock;

/// Render markdown to HTML with all processing steps.
pub fn render_markdown(
    content: &str,
    config: &MarkdownConfig,
    executable_blocks: &mut Vec<ExecutableBlock>,
    base_url: &str,
) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    if config.smart_punctuation {
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
    }

    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let parser = Parser::new_ext(content, options);
    let mut events: Vec<Event> = Vec::new();

    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut _heading_level = 0u8;
    let mut heading_text = String::new();
    let mut in_heading = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_content.clear();
                code_lang = match &kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;

                // Check if this is an executable code block
                if code_lang.starts_with('{') && code_lang.ends_with('}') {
                    let lang = &code_lang[1..code_lang.len() - 1];
                    // Parse potential attributes like file="..."
                    let (actual_lang, file_ref) = parse_code_attrs(lang);

                    let block_idx = executable_blocks.len();
                    executable_blocks.push(ExecutableBlock {
                        language: actual_lang.to_string(),
                        source: code_content.clone(),
                        file_ref,
                        output: None,
                        error: None,
                    });

                    // Insert placeholder that will be replaced after execution
                    let placeholder = format!("<!-- EXEC_BLOCK_{block_idx} -->");
                    events.push(Event::Html(CowStr::from(placeholder)));
                } else {
                    // Regular code block with syntax highlighting
                    let html = highlight_code(&code_content, &code_lang, config, &ss, &ts);
                    events.push(Event::Html(CowStr::from(html)));
                }
            }
            Event::Text(text) if in_code_block => {
                code_content.push_str(&text);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                _heading_level = level as u8;
                heading_text.clear();
                events.push(event);
            }
            Event::End(TagEnd::Heading(_level)) => {
                in_heading = false;

                // Insert anchor link if configured
                if config.insert_anchor_links != "none" {
                    let id = slug::slugify(&heading_text);
                    let anchor_html = format!(
                        "<a class=\"zola-anchor\" href=\"#{}\" aria-label=\"Anchor link for: {}\">#</a>",
                        id, heading_text
                    );

                    if config.insert_anchor_links == "right" {
                        // Insert anchor after heading text
                        events.push(Event::Html(CowStr::from(format!(" {anchor_html}"))));
                    }
                }
                events.push(event);
            }
            Event::Text(ref text) if in_heading => {
                heading_text.push_str(text);
                events.push(event);
            }
            Event::Start(Tag::Link {
                dest_url, title, ..
            }) => {
                // Rewrite external links
                if is_external_url(&dest_url, base_url) && config.external_links_target_blank {
                    let mut attrs = Vec::new();
                    attrs.push(r#"target="_blank""#.to_string());
                    if config.external_links_no_follow {
                        attrs.push(r#"rel="nofollow"#.to_string());
                    }
                    if config.external_links_no_referrer {
                        if attrs.last().is_some_and(|a| a.starts_with("rel=")) {
                            let last = attrs.last_mut().unwrap();
                            *last = format!("{} noreferrer\"", &last[..last.len() - 1]);
                        } else {
                            attrs.push(r#"rel="noreferrer""#.to_string());
                        }
                    } else if config.external_links_no_follow {
                        // Close the rel attribute
                        let last = attrs.last_mut().unwrap();
                        last.push('"');
                    }
                    let attrs_str = attrs.join(" ");
                    let html = format!(r#"<a href="{dest_url}" title="{title}" {attrs_str}>"#);
                    events.push(Event::Html(CowStr::from(html)));
                } else {
                    events.push(Event::Start(Tag::Link {
                        link_type: pulldown_cmark::LinkType::Inline,
                        dest_url,
                        title,
                        id: CowStr::from(""),
                    }));
                }
            }
            _ => {
                events.push(event);
            }
        }
    }

    // Render to HTML
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, events.into_iter());

    // Render emoji if configured
    if config.render_emoji {
        html = render_emoji(&html);
    }

    html
}

/// Extract summary from content at <!-- more --> marker
pub fn extract_summary(content: &str) -> Option<(String, String)> {
    let marker = "<!-- more -->";
    if let Some(pos) = content.find(marker) {
        let before = &content[..pos];
        Some((before.to_string(), content.to_string()))
    } else {
        None
    }
}

/// Replace executable block placeholders with rendered output
pub fn replace_exec_placeholders(
    html: &str,
    blocks: &[ExecutableBlock],
    config: &MarkdownConfig,
) -> String {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let mut result = html.to_string();

    for (i, block) in blocks.iter().enumerate() {
        let placeholder = format!("<!-- EXEC_BLOCK_{i} -->");
        if result.contains(&placeholder) {
            let source_html = highlight_code(&block.source, &block.language, config, &ss, &ts);
            let mut block_html = format!(r#"<div class="code-block-executed">{source_html}"#,);

            if let Some(ref output) = block.output
                && !output.is_empty()
            {
                block_html.push_str(&format!(
                    r#"<div class="code-output"><pre><code>{}</code></pre></div>"#,
                    html_escape(output)
                ));
            }
            if let Some(ref error) = block.error
                && !error.is_empty()
            {
                block_html.push_str(&format!(
                    r#"<div class="code-error"><pre><code>{}</code></pre></div>"#,
                    html_escape(error)
                ));
            }
            block_html.push_str("</div>");
            result = result.replace(&placeholder, &block_html);
        }
    }

    result
}

/// Highlight a code block with syntect
fn highlight_code(
    code: &str,
    lang: &str,
    config: &MarkdownConfig,
    ss: &SyntaxSet,
    ts: &ThemeSet,
) -> String {
    if !config.highlight_code || lang.is_empty() {
        return format!(
            "<pre><code class=\"language-{lang}\">{}</code></pre>",
            html_escape(code)
        );
    }

    // CSS-based highlighting
    if config.highlight_theme.as_deref() == Some("css") {
        let syntax = ss
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        // Use a base theme for class-based highlighting
        let theme = &ts.themes["base16-ocean.dark"];
        match highlighted_html_for_string(code, ss, syntax, theme) {
            Ok(html) => html,
            Err(_) => format!(
                "<pre><code class=\"language-{lang}\">{}</code></pre>",
                html_escape(code)
            ),
        }
    } else {
        let theme_name = config
            .highlight_theme
            .as_deref()
            .unwrap_or("base16-ocean.dark");
        let syntax = ss
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        let theme = ts
            .themes
            .get(theme_name)
            .unwrap_or(&ts.themes["base16-ocean.dark"]);

        match highlighted_html_for_string(code, ss, syntax, theme) {
            Ok(html) => html,
            Err(_) => format!(
                "<pre><code class=\"language-{lang}\">{}</code></pre>",
                html_escape(code)
            ),
        }
    }
}

/// Parse code block attributes like {python file="script.py"}
fn parse_code_attrs(lang: &str) -> (&str, Option<String>) {
    let parts: Vec<&str> = lang.splitn(2, ' ').collect();
    let actual_lang = parts[0];

    let file_ref = if parts.len() > 1 {
        let re = Regex::new(r#"file="([^"]+)""#).unwrap();
        re.captures(parts[1]).map(|c| c[1].to_string())
    } else {
        None
    };

    (actual_lang, file_ref)
}

fn is_external_url(url: &str, base_url: &str) -> bool {
    (url.starts_with("http://") || url.starts_with("https://")) && !url.starts_with(base_url)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_emoji(html: &str) -> String {
    // Simple emoji rendering for common shortcodes
    let re = Regex::new(r":(\w+):").unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let name = &caps[1];
        match emojify(name) {
            Some(emoji) => emoji.to_string(),
            None => caps[0].to_string(),
        }
    })
    .to_string()
}

fn emojify(name: &str) -> Option<&'static str> {
    match name {
        "smile" => Some("\u{1f604}"),
        "laughing" => Some("\u{1f606}"),
        "heart" => Some("\u{2764}\u{fe0f}"),
        "thumbsup" | "+1" => Some("\u{1f44d}"),
        "thumbsdown" | "-1" => Some("\u{1f44e}"),
        "rocket" => Some("\u{1f680}"),
        "fire" => Some("\u{1f525}"),
        "star" => Some("\u{2b50}"),
        "warning" => Some("\u{26a0}\u{fe0f}"),
        "check" | "white_check_mark" => Some("\u{2705}"),
        "x" => Some("\u{274c}"),
        "wave" => Some("\u{1f44b}"),
        "tada" => Some("\u{1f389}"),
        "thinking" => Some("\u{1f914}"),
        "eyes" => Some("\u{1f440}"),
        "100" => Some("\u{1f4af}"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MarkdownConfig;
    use crate::execute::ExecutableBlock;

    fn default_config() -> MarkdownConfig {
        MarkdownConfig::default()
    }

    #[test]
    fn test_render_basic_paragraph() {
        let mut blocks = Vec::new();
        let html = render_markdown(
            "Hello world",
            &default_config(),
            &mut blocks,
            "https://example.com",
        );
        assert!(html.contains("<p>Hello world</p>"));
    }

    #[test]
    fn test_render_code_block_highlighted() {
        let config = default_config();
        let mut blocks = Vec::new();
        let input = "```rust\nfn main() {}\n```";
        let html = render_markdown(input, &config, &mut blocks, "https://example.com");
        // Syntax highlighting produces <pre style="..."> tags from syntect
        assert!(html.contains("<pre"));
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_render_executable_block_detected() {
        let mut blocks = Vec::new();
        let input = "```{python}\nprint('hello')\n```";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "python");
        assert!(blocks[0].source.contains("print('hello')"));
        assert!(html.contains("<!-- EXEC_BLOCK_0 -->"));
    }

    #[test]
    fn test_render_table() {
        let mut blocks = Vec::new();
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn test_render_heading_anchor_right() {
        let mut config = default_config();
        config.insert_anchor_links = "right".to_string();
        let mut blocks = Vec::new();
        let html = render_markdown(
            "## Hello World",
            &config,
            &mut blocks,
            "https://example.com",
        );
        assert!(html.contains("zola-anchor"));
        assert!(html.contains("href=\"#hello-world\""));
    }

    #[test]
    fn test_render_heading_anchor_none() {
        let config = default_config(); // insert_anchor_links = "none"
        let mut blocks = Vec::new();
        let html = render_markdown(
            "## Hello World",
            &config,
            &mut blocks,
            "https://example.com",
        );
        assert!(!html.contains("zola-anchor"));
    }

    #[test]
    fn test_render_external_link_target_blank() {
        let mut config = default_config();
        config.external_links_target_blank = true;
        let mut blocks = Vec::new();
        let input = "[link](https://other.com)";
        let html = render_markdown(input, &config, &mut blocks, "https://example.com");
        assert!(html.contains(r#"target="_blank""#));
    }

    #[test]
    fn test_render_internal_link_no_target_blank() {
        let mut config = default_config();
        config.external_links_target_blank = true;
        let mut blocks = Vec::new();
        let input = "[link](https://example.com/page)";
        let html = render_markdown(input, &config, &mut blocks, "https://example.com");
        assert!(!html.contains("target="));
    }

    #[test]
    fn test_render_emoji() {
        let mut config = default_config();
        config.render_emoji = true;
        let mut blocks = Vec::new();
        let html = render_markdown(
            "Hello :rocket:",
            &config,
            &mut blocks,
            "https://example.com",
        );
        assert!(html.contains("\u{1f680}"));
        assert!(!html.contains(":rocket:"));
    }

    #[test]
    fn test_extract_summary_present() {
        let content = "First part\n<!-- more -->\nRest of content";
        let result = extract_summary(content);
        assert!(result.is_some());
        let (summary, full) = result.unwrap();
        assert_eq!(summary, "First part\n");
        assert_eq!(full, content);
    }

    #[test]
    fn test_extract_summary_absent() {
        let content = "No summary marker here";
        assert!(extract_summary(content).is_none());
    }

    #[test]
    fn test_replace_exec_with_output() {
        let html = "before <!-- EXEC_BLOCK_0 --> after";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "print('hi')".into(),
            file_ref: None,
            output: Some("hi\n".into()),
            error: None,
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(result.contains("code-block-executed"));
        assert!(result.contains("code-output"));
        assert!(result.contains("hi\n"));
        assert!(!result.contains("EXEC_BLOCK_0"));
    }

    #[test]
    fn test_replace_exec_with_error() {
        let html = "<!-- EXEC_BLOCK_0 -->";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "bad".into(),
            file_ref: None,
            output: None,
            error: Some("NameError".into()),
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(result.contains("code-error"));
        assert!(result.contains("NameError"));
    }

    #[test]
    fn test_is_external_url() {
        assert!(is_external_url("https://other.com", "https://example.com"));
        assert!(is_external_url("http://other.com", "https://example.com"));
        assert!(!is_external_url(
            "https://example.com/page",
            "https://example.com"
        ));
        assert!(!is_external_url("/relative/path", "https://example.com"));
        assert!(!is_external_url("#anchor", "https://example.com"));
    }
}
