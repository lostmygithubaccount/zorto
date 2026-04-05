//! Onboarding wizard for creating a new zorto site.
//!
//! When no `config.toml` exists in the site root, the webapp shows a
//! step-by-step wizard: Welcome -> Template -> Theme -> Configure -> Done.

use axum::extract::State;
use axum::response::{Html, IntoResponse, Redirect};
use std::sync::Arc;

use crate::{AppState, escape};

const DEFAULT_BASE_URL: &str = "http://localhost:1111";
const DEFAULT_SITE_TITLE: &str = "My Site";

/// Wizard step 1: Welcome page.
pub async fn welcome(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if state.root.join("config.toml").exists() {
        return Redirect::to("/").into_response();
    }
    Html(wizard_page(
        "Welcome",
        1,
        r#"<div class="wizard-hero">
  <h1>Create your site</h1>
  <p class="wizard-subtitle">Set up a new zorto site in seconds. Pick a template, choose a theme, and start writing.</p>
  <a href="/setup/template" class="btn btn-primary btn-lg">Get Started</a>
</div>"#,
    ))
    .into_response()
}

/// Wizard step 2: Template selection.
pub async fn template(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if state.root.join("config.toml").exists() {
        return Redirect::to("/").into_response();
    }

    let templates = [
        (
            "default",
            "Starter",
            "A clean starting point. Includes a homepage and posts section. Perfect for personal sites.",
        ),
        (
            "blog",
            "Blog",
            "Ready-to-go blog with example posts. Ideal for writers and content creators.",
        ),
        (
            "docs",
            "Documentation",
            "Structured documentation site with a guide section. Great for projects and APIs.",
        ),
        (
            "business",
            "Business",
            "Minimal landing page template. Best for company sites and portfolios.",
        ),
    ];

    let cards: String = templates
        .iter()
        .map(|(id, name, desc)| {
            format!(
                r#"<button type="submit" name="template" value="{id}" class="template-card">
  <div class="template-card-name">{name}</div>
  <div class="template-card-desc">{desc}</div>
</button>"#,
                id = escape(id),
                name = escape(name),
                desc = escape(desc),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let body = format!(
        r#"<div class="wizard-content">
  <h2>Choose a template</h2>
  <p class="wizard-hint">This determines the initial structure and sample content for your site.</p>
  <form method="POST" action="/setup/template">
    <div class="template-grid">
      {cards}
    </div>
  </form>
  <div class="wizard-nav">
    <a href="/setup" class="btn">Back</a>
  </div>
</div>"#
    );

    Html(wizard_page("Template", 2, &body)).into_response()
}

/// Handle template selection, redirect to theme step.
pub async fn template_submit(axum::Form(form): axum::Form<TemplateForm>) -> impl IntoResponse {
    let t = &form.template;
    let valid = ["default", "blog", "docs", "business"];
    let template = if valid.contains(&t.as_str()) {
        t
    } else {
        &"default".to_string()
    };
    Redirect::to(&format!("/setup/theme?template={template}"))
}

/// Wizard step 3: Theme selection.
pub async fn theme(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ThemeQuery>,
) -> impl IntoResponse {
    if state.root.join("config.toml").exists() {
        return Redirect::to("/").into_response();
    }

    let template = params.template.as_deref().unwrap_or("default");
    let themes = zorto_core::themes::Theme::available();

    let swatches: String = themes
        .iter()
        .map(|name| {
            let (color, desc) = theme_info(name);
            format!(
                r#"<button type="submit" name="theme" value="{name}" class="theme-swatch" style="--swatch-color: {color};">
  <div class="swatch-dot" style="background: {color};"></div>
  <div class="swatch-name">{name}</div>
  <div class="swatch-desc">{desc}</div>
</button>"#,
                name = escape(name),
                color = escape(color),
                desc = escape(desc),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let e_template = escape(template);

    let body = format!(
        r#"<div class="wizard-content">
  <h2>Pick a theme</h2>
  <p class="wizard-hint">Every theme supports light and dark mode. You can change this later in config.</p>
  <form method="POST" action="/setup/theme">
    <input type="hidden" name="template" value="{e_template}">
    <div class="theme-grid">
      {swatches}
    </div>
  </form>
  <div class="wizard-nav">
    <a href="/setup/template" class="btn">Back</a>
  </div>
</div>"#
    );

    Html(wizard_page("Theme", 3, &body)).into_response()
}

/// Handle theme selection, redirect to configure step.
pub async fn theme_submit(axum::Form(form): axum::Form<ThemeForm>) -> impl IntoResponse {
    let template = form.template.as_deref().unwrap_or("default");
    let theme = &form.theme;

    // Validate theme against available themes
    let available = zorto_core::themes::Theme::available();
    let theme = if available.contains(&theme.as_str()) {
        theme.as_str()
    } else {
        "default"
    };

    Redirect::to(&format!(
        "/setup/configure?template={}&theme={}",
        escape(template),
        escape(theme)
    ))
}

/// Wizard step 4: Configure basic settings.
pub async fn configure(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<ConfigureQuery>,
) -> impl IntoResponse {
    if state.root.join("config.toml").exists() {
        return Redirect::to("/").into_response();
    }

    let template = params.template.as_deref().unwrap_or("default");
    let theme = params.theme.as_deref().unwrap_or("default");

    let body = format!(
        r#"<div class="wizard-content">
  <h2>Configure your site</h2>
  <p class="wizard-hint">You can always change these in the config editor later.</p>
  <form method="POST" action="/setup/create">
    <input type="hidden" name="template" value="{e_template}">
    <input type="hidden" name="theme" value="{e_theme}">
    <div class="card" style="max-width: 480px; margin: 0 auto;">
      <div class="form-group">
        <label>Site Title</label>
        <input type="text" name="title" value="{default_title}" required autofocus>
      </div>
      <div class="form-group">
        <label>Author <span style="color: #666680; font-size: 0.7rem; text-transform: none;">(optional)</span></label>
        <input type="text" name="author" placeholder="Your name">
      </div>
      <div class="form-group">
        <label>Base URL</label>
        <input type="text" name="base_url" value="{default_base_url}">
      </div>
    </div>
    <div class="wizard-nav" style="justify-content: center; margin-top: 24px;">
      <a href="/setup/theme?template={e_template}" class="btn">Back</a>
      <button type="submit" class="btn btn-primary btn-lg">Create Site</button>
    </div>
  </form>
</div>"#,
        e_template = escape(template),
        e_theme = escape(theme),
        default_title = DEFAULT_SITE_TITLE,
        default_base_url = DEFAULT_BASE_URL,
    );

    Html(wizard_page("Configure", 4, &body)).into_response()
}

/// Handle site creation.
pub async fn create(
    State(state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<CreateForm>,
) -> impl IntoResponse {
    let template = form.template.as_deref().unwrap_or("default");
    let theme = form.theme.as_deref().unwrap_or("default");
    let title = if form.title.is_empty() {
        DEFAULT_SITE_TITLE
    } else {
        &form.title
    };
    let base_url = if form.base_url.is_empty() {
        DEFAULT_BASE_URL
    } else {
        &form.base_url
    };
    let author = if form.author.is_empty() {
        "Author"
    } else {
        &form.author
    };

    if let Err(e) = write_site(&state.root, template, theme, title, base_url, author) {
        return Html(wizard_page(
            "Error",
            5,
            &format!(
                r#"<div class="wizard-content">
  <div class="flash flash-error">Failed to create site: {}</div>
  <a href="/setup" class="btn">Start Over</a>
</div>"#,
                escape(&e.to_string())
            ),
        ))
        .into_response();
    }

    // Build the site
    let _ = crate::rebuild_site(&state);

    Redirect::to("/?welcome=1").into_response()
}

/// Write a new site to disk with the given template, theme, and config values.
fn write_site(
    root: &std::path::Path,
    template: &str,
    theme: &str,
    title: &str,
    base_url: &str,
    author: &str,
) -> Result<(), String> {
    let files: &[(&str, &str)] = match template {
        "blog" => &BLOG_FILES,
        "docs" => &DOCS_FILES,
        "business" => &BUSINESS_FILES,
        _ => &DEFAULT_FILES,
    };

    for (path, content) in files {
        let dest = root.join(path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&dest, content).map_err(|e| e.to_string())?;
    }

    // Ensure static/ directory exists
    let _ = std::fs::create_dir_all(root.join("static"));

    // Write customized config.toml
    // Escape backslashes and double quotes for TOML string context
    let escape_toml = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_title = escape_toml(title);
    let safe_base_url = escape_toml(base_url);
    let safe_theme = escape_toml(theme);
    let safe_author_toml = escape_toml(author);
    let safe_author = author.replace('\'', "&#39;");
    let safe_title_html = title.replace('\'', "&#39;");

    let config = match template {
        "blog" => format!(
            "base_url = \"{safe_base_url}\"\ntitle = \"{safe_title}\"\ntheme = \"{safe_theme}\"\ngenerate_feed = true\n\n[markdown]\nhighlight_code = true\n\n[extra]\ncopyright_html = '<a href=\"/\">{safe_title_html}</a> by {safe_author} via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'\n"
        ),
        "docs" => format!(
            "base_url = \"{safe_base_url}\"\ntitle = \"{safe_title}\"\ntheme = \"{safe_theme}\"\n\n[markdown]\nhighlight_code = true\ninsert_anchor_links = \"right\"\n\n[extra]\ncopyright_html = '<a href=\"/\">{safe_title_html}</a> by {safe_author} via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'\n"
        ),
        "business" => format!(
            "base_url = \"{safe_base_url}\"\ntitle = \"{safe_title}\"\ntheme = \"{safe_theme}\"\n\n[markdown]\nsmart_punctuation = true\nexternal_links_target_blank = true\n\n[extra]\nauthor = \"{safe_author_toml}\"\ncopyright_html = '<a href=\"/\">{safe_title_html}</a> by {safe_author} via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'\nhero_title = \"{safe_title}\"\nhero_subtitle = \"Welcome to our website.\"\n"
        ),
        _ => format!(
            "base_url = \"{safe_base_url}\"\ntitle = \"{safe_title}\"\ntheme = \"{safe_theme}\"\ngenerate_feed = true\n\n[extra]\ncopyright_html = '<a href=\"/\">{safe_title_html}</a> by {safe_author} via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'\n"
        ),
    };

    std::fs::write(root.join("config.toml"), config).map_err(|e| e.to_string())?;
    Ok(())
}

// Template content files (config.toml is generated dynamically above)

const DEFAULT_FILES: [(&str, &str); 7] = [
    (
        "content/_index.md",
        "+++\ntitle = \"Home\"\nsort_by = \"date\"\n+++\n",
    ),
    (
        "content/posts/_index.md",
        "+++\ntitle = \"Blog\"\nsort_by = \"date\"\n+++\n",
    ),
    (
        "content/posts/hello.md",
        "+++\ntitle = \"Hello World\"\ndate = \"2025-01-01\"\ndescription = \"My first post\"\ntags = [\"hello\"]\n+++\nWelcome to my new site built with [zorto](https://github.com/dkdc-io/zorto)!\n",
    ),
    (
        "templates/base.html",
        "<!DOCTYPE html>\n<html lang=\"{{ config.default_language }}\">\n<head>\n    <meta charset=\"utf-8\">\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n    <title>{% block title %}{{ config.title }}{% endblock %}</title>\n    {% if config.generate_feed %}<link rel=\"alternate\" type=\"application/atom+xml\" title=\"Feed\" href=\"{{ config.base_url }}/atom.xml\">{% endif %}\n</head>\n<body>\n    <nav><a href=\"{{ config.base_url }}/\">{{ config.title }}</a></nav>\n    <main>{% block content %}{% endblock %}</main>\n</body>\n</html>\n",
    ),
    (
        "templates/index.html",
        "{% extends \"base.html\" %}\n{% block content %}\n<h1>{{ section.title }}</h1>\n{{ section.content | safe }}\n{% for page in section.pages %}\n<article>\n    <h2><a href=\"{{ page.permalink }}\">{{ page.title }}</a></h2>\n    {% if page.date %}<time>{{ page.date }}</time>{% endif %}\n    {% if page.description %}<p>{{ page.description }}</p>{% endif %}\n</article>\n{% endfor %}\n{% endblock %}\n",
    ),
    (
        "templates/section.html",
        "{% extends \"base.html\" %}\n{% block content %}\n<h1>{{ section.title }}</h1>\n{{ section.content | safe }}\n{% for page in section.pages %}\n<article>\n    <h2><a href=\"{{ page.permalink }}\">{{ page.title }}</a></h2>\n    {% if page.date %}<time>{{ page.date }}</time>{% endif %}\n    {% if page.description %}<p>{{ page.description }}</p>{% endif %}\n</article>\n{% endfor %}\n{% endblock %}\n",
    ),
    (
        "templates/page.html",
        "{% extends \"base.html\" %}\n{% block title %}{{ page.title }} | {{ config.title }}{% endblock %}\n{% block content %}\n<article>\n    <h1>{{ page.title }}</h1>\n    {% if page.date %}<time>{{ page.date }}</time>{% endif %}\n    {{ page.content | safe }}\n</article>\n{% endblock %}\n",
    ),
];

const BLOG_FILES: [(&str, &str); 4] = [
    (
        "content/_index.md",
        "+++\ntitle = \"Home\"\nsort_by = \"date\"\npaginate_by = 10\n+++\n\nWelcome to my blog.\n",
    ),
    (
        "content/posts/_index.md",
        "+++\ntitle = \"Posts\"\nsort_by = \"date\"\n+++\n",
    ),
    (
        "content/posts/hello-world.md",
        "+++\ntitle = \"Hello, world!\"\ndate = \"2026-04-04\"\ndescription = \"Welcome to my new blog, built with Zorto.\"\n+++\nThis is your first blog post. Edit this file at `content/posts/hello-world.md` or create new posts in `content/posts/`.\n",
    ),
    (
        "content/posts/getting-started.md",
        "+++\ntitle = \"Getting started with Zorto\"\ndate = \"2026-04-03\"\ndescription = \"A quick guide to building your blog with Zorto.\"\n+++\nZorto is a fast, AI-native static site generator.\n\n## Useful commands\n\n| Command | Description |\n|---------|-------------|\n| `zorto build` | Build the site |\n| `zorto preview --open` | Live preview with hot reload |\n| `zorto check` | Validate without building |\n\nVisit [zorto.dev](https://zorto.dev) for full documentation.\n",
    ),
];

const DOCS_FILES: [(&str, &str); 5] = [
    (
        "content/_index.md",
        "+++\ntitle = \"Documentation\"\n+++\n\nWelcome to the documentation.\n\n- [Guide](/guide/) -- Get started\n",
    ),
    (
        "content/guide/_index.md",
        "+++\ntitle = \"Guide\"\nsort_by = \"title\"\n+++\n",
    ),
    (
        "content/guide/introduction.md",
        "+++\ntitle = \"Introduction\"\n+++\nWelcome to the documentation. This guide will help you get started.\n",
    ),
    (
        "content/guide/installation.md",
        "+++\ntitle = \"Installation\"\n+++\n## Install Zorto\n\n```bash\ncargo install zorto\n```\n\nVerify with `zorto --version`.\n",
    ),
    (
        "content/guide/configuration.md",
        "+++\ntitle = \"Configuration\"\n+++\nThe `config.toml` file controls all site settings.\n\n| Field | Description |\n|-------|-------------|\n| `base_url` | Deployment URL |\n| `title` | Site title |\n| `theme` | Built-in theme name |\n",
    ),
];

const BUSINESS_FILES: [(&str, &str); 1] = [(
    "content/_index.md",
    "+++\ntitle = \"Home\"\ndescription = \"Welcome to our website.\"\n+++\n",
)];

// --- Form types ---

#[derive(serde::Deserialize)]
pub struct TemplateForm {
    template: String,
}

#[derive(serde::Deserialize)]
pub struct ThemeQuery {
    template: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct ThemeForm {
    template: Option<String>,
    theme: String,
}

#[derive(serde::Deserialize)]
pub struct ConfigureQuery {
    template: Option<String>,
    theme: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct CreateForm {
    template: Option<String>,
    theme: Option<String>,
    title: String,
    author: String,
    base_url: String,
}

// --- Helpers ---

fn theme_info(name: &str) -> (&str, &str) {
    match name {
        "zorto" => ("#4ade80", "Blue/green with animations"),
        "dkdc" => ("#a78bfa", "Violet/cyan with animations"),
        "default" => ("#60a5fa", "Clean blue, no animations"),
        "ember" => ("#f59e0b", "Warm orange/amber"),
        "forest" => ("#84cc16", "Natural green/lime"),
        "ocean" => ("#14b8a6", "Calm teal/blue"),
        "rose" => ("#f472b6", "Soft pink/purple"),
        "slate" => ("#94a3b8", "Minimal monochrome"),
        "midnight" => ("#6366f1", "Navy/silver corporate"),
        "sunset" => ("#ef4444", "Bold red/orange"),
        "mint" => ("#34d399", "Modern green/cyan"),
        "plum" => ("#c084fc", "Rich purple/magenta"),
        "sand" => ("#d4a373", "Warm neutral/earth tones"),
        "arctic" => ("#7dd3fc", "Cool blue/white"),
        "lime" => ("#a3e635", "Bright green/yellow"),
        "charcoal" => ("#a1a1aa", "Dark grey/silver"),
        _ => ("#60a5fa", ""),
    }
}

/// Render a wizard page (no sidebar, centered layout with progress).
fn wizard_page(title: &str, step: u8, body: &str) -> String {
    let steps = ["Welcome", "Template", "Theme", "Configure", "Done"];
    let progress: String = steps
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let num = i as u8 + 1;
            let class = if num < step {
                "step-done"
            } else if num == step {
                "step-active"
            } else {
                "step-pending"
            };
            format!(
                r#"<div class="step {class}"><span class="step-num">{num}</span> {label}</div>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title} — zorto setup</title>
  <style>{CSS}</style>
</head>
<body class="wizard-body">
  <div class="wizard-container">
    <div class="wizard-header">
      <div class="wizard-logo">zorto</div>
      <div class="wizard-steps">
        {progress}
      </div>
    </div>
    {body}
  </div>
</body>
</html>"##,
        title = escape(title),
        CSS = WIZARD_CSS,
    )
}

const WIZARD_CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html { background: #111118; }
body.wizard-body {
  font-family: system-ui, -apple-system, sans-serif;
  background: #111118;
  color: #c8c8d8;
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
a { color: #60a5fa; text-decoration: none; }
a:hover { text-decoration: underline; }
.wizard-container { max-width: 720px; width: 100%; }
.wizard-header { text-align: center; margin-bottom: 40px; }
.wizard-logo { font-size: 1.4rem; font-weight: 700; color: #60a5fa; margin-bottom: 20px; }
.wizard-steps { display: flex; justify-content: center; gap: 8px; flex-wrap: wrap; }
.step { display: flex; align-items: center; gap: 6px; font-size: 0.8rem; color: #444460; padding: 4px 10px; border-radius: 20px; }
.step-num { display: inline-flex; width: 20px; height: 20px; align-items: center; justify-content: center; border-radius: 50%; font-size: 0.7rem; font-weight: 600; background: #1e1e2e; color: #444460; }
.step-done { color: #34d399; }
.step-done .step-num { background: #1a3a2a; color: #34d399; }
.step-active { color: #60a5fa; }
.step-active .step-num { background: #1e3a5f; color: #60a5fa; }
.wizard-hero { text-align: center; padding: 40px 0; }
.wizard-hero h1 { font-size: 2rem; color: #e0e0f0; margin-bottom: 12px; font-weight: 600; }
.wizard-subtitle { color: #8c8ca6; font-size: 1rem; margin-bottom: 32px; line-height: 1.6; max-width: 440px; margin-left: auto; margin-right: auto; }
.wizard-content { text-align: center; }
.wizard-content h2 { font-size: 1.4rem; color: #e0e0f0; margin-bottom: 8px; font-weight: 500; }
.wizard-hint { color: #666680; font-size: 0.85rem; margin-bottom: 24px; }
.wizard-nav { display: flex; gap: 12px; margin-top: 20px; justify-content: flex-start; }
.template-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; text-align: left; margin-bottom: 16px; }
.template-card {
  background: #16161f; border: 2px solid #2a2a3a; border-radius: 10px; padding: 20px;
  cursor: pointer; transition: border-color 0.15s, background 0.15s;
  display: block; width: 100%; font-family: inherit; color: inherit; text-align: left;
}
.template-card:hover { border-color: #60a5fa; background: #1a1a26; }
.template-card-name { font-size: 1rem; font-weight: 600; color: #e0e0f0; margin-bottom: 6px; }
.template-card-desc { font-size: 0.85rem; color: #8c8ca6; line-height: 1.4; }
.theme-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(140px, 1fr)); gap: 10px; text-align: left; margin-bottom: 16px; }
.theme-swatch {
  background: #16161f; border: 2px solid #2a2a3a; border-radius: 8px; padding: 14px;
  cursor: pointer; transition: border-color 0.15s; display: block; width: 100%;
  font-family: inherit; color: inherit; text-align: left;
}
.theme-swatch:hover { border-color: var(--swatch-color, #60a5fa); }
.swatch-dot { width: 28px; height: 28px; border-radius: 50%; margin-bottom: 8px; }
.swatch-name { font-size: 0.85rem; font-weight: 600; color: #e0e0f0; margin-bottom: 2px; }
.swatch-desc { font-size: 0.7rem; color: #666680; }
.card { background: #16161f; border: 1px solid #2a2a3a; border-radius: 8px; padding: 20px; }
.form-group { margin-bottom: 16px; }
label { display: block; font-size: 0.8rem; color: #8c8ca6; margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.05em; }
input[type="text"] { background: #111118; border: 1px solid #2a2a3a; border-radius: 6px; color: #c8c8d8; padding: 10px 12px; font-size: 0.9rem; width: 100%; }
input:focus { outline: none; border-color: #60a5fa; }
.btn { display: inline-block; background: #1e1e2e; border: 1px solid #2a2a3a; color: #c8c8d8; padding: 8px 16px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-family: inherit; text-decoration: none; }
.btn:hover { border-color: #60a5fa; color: #60a5fa; text-decoration: none; }
.btn-primary { background: #1e3a5f; border-color: #60a5fa; color: #60a5fa; }
.btn-primary:hover { background: #264d7a; }
.btn-lg { padding: 12px 28px; font-size: 1rem; }
.flash-error { background: #3a1a1a; border: 1px solid #f87171; color: #f87171; padding: 12px 16px; border-radius: 6px; margin-bottom: 16px; }
@media (max-width: 600px) {
  .wizard-hero h1 { font-size: 1.5rem; }
  .template-grid { grid-template-columns: 1fr; }
  .theme-grid { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
  .wizard-steps { gap: 4px; }
  .step { font-size: 0.7rem; padding: 3px 6px; }
}
"#;
