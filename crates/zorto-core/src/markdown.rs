use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::LazyLock;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::{SyntaxDefinition, SyntaxSet};

use crate::config::{AnchorLinks, MarkdownConfig};
use crate::content::escape_xml;
use crate::execute::{ExecutableBlock, VizOutput};
use crate::shortcodes::{
    CALLOUT_ICON_CAUTION, CALLOUT_ICON_IMPORTANT, CALLOUT_ICON_NOTE, CALLOUT_ICON_TIP,
    CALLOUT_ICON_WARNING,
};

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
    // Add TOML syntax (not in syntect's defaults)
    if let Ok(toml_syn) = SyntaxDefinition::load_from_str(
        include_str!("../syntaxes/TOML.sublime-syntax"),
        true,
        Some("TOML"),
    ) {
        builder.add(toml_syn);
    }
    builder.build()
});
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);
static FILE_ATTR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"file="([^"]+)""#).unwrap());

const DEFAULT_HIGHLIGHT_THEME: &str = "base16-ocean.dark";

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
    options.insert(Options::ENABLE_GFM);
    if config.smart_punctuation {
        options.insert(Options::ENABLE_SMART_PUNCTUATION);
    }

    let parser = Parser::new_ext(content, options);
    let mut events: Vec<Event> = Vec::new();

    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_content = String::new();
    let mut heading_text = String::new();
    let mut in_heading = false;

    for event in parser {
        match event {
            // --- GitHub-style callout / alert blockquotes ---
            Event::Start(Tag::BlockQuote(Some(kind))) => {
                let (css_class, icon, title) = callout_info(&kind);
                let html = format!(
                    "<div class=\"callout callout--{css_class}\">\
                     <p class=\"callout__title\">{icon} {title}</p>"
                );
                events.push(Event::Html(CowStr::from(html)));
            }
            Event::End(TagEnd::BlockQuote(Some(_))) => {
                events.push(Event::Html(CowStr::from("</div>")));
            }

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
                        viz: Vec::new(),
                    });

                    // Insert placeholder that will be replaced after execution
                    let placeholder = format!("<!-- EXEC_BLOCK_{block_idx} -->");
                    events.push(Event::Html(CowStr::from(placeholder)));
                } else {
                    // Regular code block with syntax highlighting
                    let html = highlight_code(&code_content, &code_lang, config);
                    events.push(Event::Html(CowStr::from(html)));
                }
            }
            Event::Text(text) if in_code_block => {
                code_content.push_str(&text);
            }
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_text.clear();
                // Remember where we'll patch in the id later
                events.push(Event::Start(Tag::Heading {
                    level,
                    id: None,
                    classes: vec![],
                    attrs: vec![],
                }));
            }
            Event::End(TagEnd::Heading(_level)) => {
                in_heading = false;
                let id = slug::slugify(&heading_text);

                // Patch the heading start event with the computed id
                for ev in events.iter_mut().rev() {
                    if let Event::Start(Tag::Heading { id: h_id, .. }) = ev {
                        *h_id = Some(CowStr::from(id.clone()));
                        break;
                    }
                }

                // Insert anchor link if configured
                if config.insert_anchor_links == AnchorLinks::Right {
                    let anchor_html = format!(
                        " <a class=\"zorto-anchor\" href=\"#{}\" aria-label=\"Anchor link for: {}\">#</a>",
                        id, heading_text
                    );
                    events.push(Event::Html(CowStr::from(anchor_html)));
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
                    let mut attrs = vec![r#"target="_blank""#.to_string()];
                    let mut rel_parts = Vec::new();
                    if config.external_links_no_follow {
                        rel_parts.push("nofollow");
                    }
                    if config.external_links_no_referrer {
                        rel_parts.push("noreferrer");
                    }
                    if !rel_parts.is_empty() {
                        attrs.push(format!(r#"rel="{}""#, rel_parts.join(" ")));
                    }
                    let attrs_str = attrs.join(" ");
                    let html = format!(
                        r#"<a href="{}" title="{}" {attrs_str}>"#,
                        escape_xml(&dest_url),
                        escape_xml(&title),
                    );
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

    html
}

/// Get CSS class, SVG icon, and display title for a GitHub-style callout.
fn callout_info(kind: &BlockQuoteKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        BlockQuoteKind::Note => ("note", CALLOUT_ICON_NOTE, "Note"),
        BlockQuoteKind::Tip => ("tip", CALLOUT_ICON_TIP, "Tip"),
        BlockQuoteKind::Warning => ("warning", CALLOUT_ICON_WARNING, "Warning"),
        BlockQuoteKind::Caution => ("caution", CALLOUT_ICON_CAUTION, "Caution"),
        BlockQuoteKind::Important => ("important", CALLOUT_ICON_IMPORTANT, "Important"),
    }
}

/// Extract summary from content at <!-- more --> marker
pub fn extract_summary(content: &str) -> Option<String> {
    let marker = "<!-- more -->";
    content.find(marker).map(|pos| content[..pos].to_string())
}

/// Replace executable block placeholders with rendered output
pub fn replace_exec_placeholders(
    html: &str,
    blocks: &[ExecutableBlock],
    config: &MarkdownConfig,
) -> String {
    let mut result = html.to_string();

    for (i, block) in blocks.iter().enumerate() {
        let placeholder = format!("<!-- EXEC_BLOCK_{i} -->");
        if result.contains(&placeholder) {
            let source_html = highlight_code(&block.source, &block.language, config);
            let mut block_html = format!(r#"<div class="code-block-executed">{source_html}"#,);

            if let Some(ref output) = block.output
                && !output.is_empty()
            {
                block_html.push_str(&format!(
                    r#"<div class="code-output"><pre><code>{}</code></pre></div>"#,
                    escape_xml(output)
                ));
            }
            if let Some(ref error) = block.error
                && !error.is_empty()
            {
                block_html.push_str(&format!(
                    r#"<div class="code-error"><pre><code>{}</code></pre></div>"#,
                    escape_xml(error)
                ));
            }
            // Render visualization output (after text output, before closing div)
            for v in &block.viz {
                block_html.push_str(&render_viz_output(v));
            }
            block_html.push_str("</div>");
            // replacen with limit 1 so that an executed block whose output text
            // happens to contain another block's placeholder string does not
            // cause that later substitution to land in the wrong place.
            result = result.replacen(&placeholder, &block_html, 1);
        }
    }

    result
}

/// Render a single visualization output into HTML.
fn render_viz_output(viz: &VizOutput) -> String {
    match viz.kind.as_str() {
        "img" => {
            format!(
                r#"<div class="code-viz"><img src="{}" alt="Plot output"></div>"#,
                escape_xml(&viz.data)
            )
        }
        "html" => {
            // SAFETY: This raw HTML injection is intentional. Plotly/altair HTML contains
            // <script> tags required for interactive visualizations. Since executable code
            // blocks already run arbitrary Python, XSS via visualization output is not a
            // meaningful additional risk — the user already has full code execution.
            format!(r#"<div class="code-viz">{}</div>"#, viz.data)
        }
        _ => String::new(),
    }
}

/// Highlight a code block with syntect
fn highlight_code(code: &str, lang: &str, config: &MarkdownConfig) -> String {
    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;
    let fallback = || {
        format!(
            "<pre><code class=\"language-{lang}\">{}</code></pre>",
            escape_xml(code)
        )
    };

    if !config.highlight_code || lang.is_empty() {
        return fallback();
    }

    let theme_name = config
        .highlight_theme
        .as_deref()
        .unwrap_or(DEFAULT_HIGHLIGHT_THEME);
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let theme = ts
        .themes
        .get(theme_name)
        .unwrap_or(&ts.themes[DEFAULT_HIGHLIGHT_THEME]);

    highlighted_html_for_string(code, ss, syntax, theme).unwrap_or_else(|_| fallback())
}

/// Parse code block attributes like {python file="script.py"}
fn parse_code_attrs(lang: &str) -> (&str, Option<String>) {
    let parts: Vec<&str> = lang.splitn(2, ' ').collect();
    let actual_lang = parts[0];

    let file_ref = if parts.len() > 1 {
        FILE_ATTR_RE.captures(parts[1]).map(|c| c[1].to_string())
    } else {
        None
    };

    (actual_lang, file_ref)
}

fn is_external_url(url: &str, base_url: &str) -> bool {
    (url.starts_with("http://") || url.starts_with("https://")) && !url.starts_with(base_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MarkdownConfig;
    use crate::execute::{ExecutableBlock, VizOutput};

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
    fn test_toml_syntax_available() {
        let ss = &*SYNTAX_SET;
        assert!(
            ss.find_syntax_by_token("toml").is_some(),
            "TOML syntax should be available for highlighting"
        );
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
        config.insert_anchor_links = AnchorLinks::Right;
        let mut blocks = Vec::new();
        let html = render_markdown(
            "## Hello World",
            &config,
            &mut blocks,
            "https://example.com",
        );
        assert!(html.contains("zorto-anchor"));
        assert!(html.contains("href=\"#hello-world\""));
        assert!(html.contains("id=\"hello-world\""));
    }

    #[test]
    fn test_render_heading_id_always_present() {
        let config = default_config(); // insert_anchor_links = "none"
        let mut blocks = Vec::new();
        let html = render_markdown(
            "## Hello World",
            &config,
            &mut blocks,
            "https://example.com",
        );
        assert!(!html.contains("zorto-anchor"));
        // Heading id should always be present even without anchor links
        assert!(html.contains("id=\"hello-world\""));
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
    fn test_extract_summary_present() {
        let content = "First part\n<!-- more -->\nRest of content";
        let result = extract_summary(content);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "First part\n");
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
            viz: Vec::new(),
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
            viz: Vec::new(),
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(result.contains("code-error"));
        assert!(result.contains("NameError"));
    }

    #[test]
    fn test_render_external_link_escapes_attributes() {
        let mut config = default_config();
        config.external_links_target_blank = true;
        let mut blocks = Vec::new();
        let input = r#"[xss](https://evil.com/"><script>alert(1)</script>)"#;
        let html = render_markdown(input, &config, &mut blocks, "https://example.com");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_render_callout_note() {
        let mut blocks = Vec::new();
        let input = "> [!NOTE]\n> This is a note.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout callout--note"));
        assert!(html.contains("callout__title"));
        assert!(html.contains("Note"));
        assert!(html.contains("This is a note."));
    }

    #[test]
    fn test_render_callout_tip() {
        let mut blocks = Vec::new();
        let input = "> [!TIP]\n> Helpful advice here.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout--tip"));
        assert!(html.contains("Tip"));
        assert!(html.contains("Helpful advice here."));
    }

    #[test]
    fn test_render_callout_warning() {
        let mut blocks = Vec::new();
        let input = "> [!WARNING]\n> Be careful.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout--warning"));
        assert!(html.contains("Warning"));
    }

    #[test]
    fn test_render_callout_caution() {
        let mut blocks = Vec::new();
        let input = "> [!CAUTION]\n> Danger zone.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout--caution"));
        assert!(html.contains("Caution"));
    }

    #[test]
    fn test_render_callout_important() {
        let mut blocks = Vec::new();
        let input = "> [!IMPORTANT]\n> Key info.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout--important"));
        assert!(html.contains("Important"));
    }

    #[test]
    fn test_render_regular_blockquote_unchanged() {
        let mut blocks = Vec::new();
        let input = "> This is a regular blockquote.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("<blockquote>"));
        assert!(!html.contains("callout"));
    }

    #[test]
    fn test_render_callout_with_multiple_paragraphs() {
        let mut blocks = Vec::new();
        let input = "> [!NOTE]\n> First paragraph.\n>\n> Second paragraph.";
        let html = render_markdown(input, &default_config(), &mut blocks, "https://example.com");
        assert!(html.contains("callout--note"));
        assert!(html.contains("First paragraph."));
        assert!(html.contains("Second paragraph."));
    }

    #[test]
    fn test_replace_exec_with_viz_img() {
        let html = "<!-- EXEC_BLOCK_0 -->";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "import matplotlib".into(),
            file_ref: None,
            output: Some(String::new()),
            error: None,
            viz: vec![VizOutput {
                kind: "img".into(),
                data: "data:image/png;base64,abc123".into(),
            }],
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(result.contains("code-viz"));
        assert!(result.contains(r#"<img src="data:image/png;base64,abc123""#));
        assert!(result.contains(r#"alt="Plot output""#));
    }

    #[test]
    fn test_replace_exec_with_viz_html() {
        let html = "<!-- EXEC_BLOCK_0 -->";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "import plotly".into(),
            file_ref: None,
            output: Some(String::new()),
            error: None,
            viz: vec![VizOutput {
                kind: "html".into(),
                data: "<div id=\"plotly\">chart</div>".into(),
            }],
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(result.contains("code-viz"));
        assert!(result.contains("<div id=\"plotly\">chart</div>"));
    }

    #[test]
    fn test_replace_exec_with_output_and_viz() {
        let html = "<!-- EXEC_BLOCK_0 -->";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "print('hello')".into(),
            file_ref: None,
            output: Some("hello\n".into()),
            error: None,
            viz: vec![VizOutput {
                kind: "img".into(),
                data: "data:image/png;base64,xyz".into(),
            }],
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        // Both text output and viz should appear
        assert!(result.contains("code-output"));
        assert!(result.contains("code-viz"));
        // Viz should come after text output
        let output_pos = result.find("code-output").unwrap();
        let viz_pos = result.find("code-viz").unwrap();
        assert!(viz_pos > output_pos, "viz should appear after text output");
    }

    #[test]
    fn test_replace_exec_no_viz_no_overhead() {
        let html = "<!-- EXEC_BLOCK_0 -->";
        let blocks = vec![ExecutableBlock {
            language: "python".into(),
            source: "print('hi')".into(),
            file_ref: None,
            output: Some("hi\n".into()),
            error: None,
            viz: Vec::new(),
        }];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(!result.contains("code-viz"));
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

    #[test]
    fn test_replace_exec_placeholder_collision() {
        // Regression: block 0's output happens to contain block 1's placeholder
        // string. Without the replacen-with-limit-1 fix, the second pass would
        // substitute that copy and corrupt block 0's output.
        let html = "<!-- EXEC_BLOCK_0 -->\n<!-- EXEC_BLOCK_1 -->";
        let blocks = vec![
            ExecutableBlock {
                language: "python".into(),
                source: "print('a')".into(),
                file_ref: None,
                output: Some("<!-- EXEC_BLOCK_1 -->".into()),
                error: None,
                viz: Vec::new(),
            },
            ExecutableBlock {
                language: "python".into(),
                source: "print('b')".into(),
                file_ref: None,
                output: Some("BLOCK_ONE_OUTPUT".into()),
                error: None,
                viz: Vec::new(),
            },
        ];
        let result = replace_exec_placeholders(html, &blocks, &default_config());
        assert!(
            result.contains("BLOCK_ONE_OUTPUT"),
            "block 1 must still be replaced at its original position"
        );
        // Block 0's literal placeholder text in its output is HTML-escaped, so
        // the comment form is gone — but the escaped form must survive,
        // proving block 1's substitution did not consume it.
        assert!(
            result.contains("&lt;!-- EXEC_BLOCK_1 --&gt;"),
            "block 0's output (containing the escaped placeholder text) must survive"
        );
    }
}
