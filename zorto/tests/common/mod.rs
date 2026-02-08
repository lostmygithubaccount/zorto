use std::path::PathBuf;
use tempfile::TempDir;

/// Write minimal templates to a templates directory
fn write_templates(templates: &std::path::Path) {
    std::fs::create_dir_all(templates).unwrap();
    std::fs::write(
        templates.join("base.html"),
        "<!DOCTYPE html><html><body>{% block content %}{% endblock %}</body></html>",
    )
    .unwrap();
    std::fs::write(
        templates.join("index.html"),
        r#"{% extends "base.html" %}{% block content %}{{ section.title }}{% for page in section.pages %}<a href="{{ page.permalink }}">{{ page.title }}</a>{% endfor %}{% endblock %}"#,
    )
    .unwrap();
    std::fs::write(
        templates.join("section.html"),
        r#"{% extends "base.html" %}{% block content %}{{ section.title }}{% if paginator %}<nav>{% if paginator.previous %}<a href="{{ paginator.previous }}">prev</a>{% endif %}{% if paginator.next %}<a href="{{ paginator.next }}">next</a>{% endif %}</nav>{% endif %}{% for page in paginator.pages | default(value=section.pages) %}<a href="{{ page.permalink }}">{{ page.title }}</a>{% endfor %}{% endblock %}"#,
    )
    .unwrap();
    std::fs::write(
        templates.join("page.html"),
        r#"{% extends "base.html" %}{% block content %}<h1>{{ page.title }}</h1>{{ page.content | safe }}{% endblock %}"#,
    )
    .unwrap();
}

/// Create a minimal test site with config, one section, and one page
pub fn make_test_site(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().join("site");
    let content = root.join("content");
    std::fs::create_dir_all(content.join("posts")).unwrap();
    std::fs::create_dir_all(root.join("static")).unwrap();

    std::fs::write(
        root.join("config.toml"),
        r#"base_url = "https://example.com"
title = "Integration Test Site"
"#,
    )
    .unwrap();

    std::fs::write(
        content.join("_index.md"),
        "+++\ntitle = \"Home\"\n+++\nWelcome to the site.",
    )
    .unwrap();

    std::fs::write(
        content.join("posts/_index.md"),
        "+++\ntitle = \"Blog\"\nsort_by = \"date\"\n+++\n",
    )
    .unwrap();

    std::fs::write(
        content.join("posts/first.md"),
        "+++\ntitle = \"First Post\"\ndate = \"2025-01-01\"\n+++\nFirst post content.",
    )
    .unwrap();

    write_templates(&root.join("templates"));

    root
}

/// Create a test site with multiple sections and pages
pub fn make_test_site_with_sections(tmp: &TempDir) -> PathBuf {
    let root = make_test_site(tmp);
    let content = root.join("content");

    // Add more posts
    std::fs::write(
        content.join("posts/second.md"),
        "+++\ntitle = \"Second Post\"\ndate = \"2025-02-01\"\n+++\nSecond content.",
    )
    .unwrap();

    // Add a docs section
    std::fs::create_dir_all(content.join("docs")).unwrap();
    std::fs::write(
        content.join("docs/_index.md"),
        "+++\ntitle = \"Docs\"\n+++\n",
    )
    .unwrap();
    std::fs::write(
        content.join("docs/getting-started.md"),
        "+++\ntitle = \"Getting Started\"\n+++\nStart here.",
    )
    .unwrap();

    root
}

/// Create a test site with taxonomy templates and tagged posts
pub fn make_test_site_with_tags(tmp: &TempDir) -> PathBuf {
    let root = make_test_site(tmp);
    let content = root.join("content");
    let templates = root.join("templates");

    // Overwrite config with explicit taxonomy
    std::fs::write(
        root.join("config.toml"),
        r#"base_url = "https://example.com"
title = "Tag Test Site"

[[taxonomies]]
name = "tags"
"#,
    )
    .unwrap();

    // Tagged posts
    std::fs::write(
        content.join("posts/first.md"),
        "+++\ntitle = \"Rust Post\"\ndate = \"2025-01-01\"\ntags = [\"rust\"]\n+++\nRust content.",
    )
    .unwrap();
    std::fs::write(
        content.join("posts/second.md"),
        "+++\ntitle = \"Both Post\"\ndate = \"2025-02-01\"\ntags = [\"rust\", \"python\"]\n+++\nBoth.",
    )
    .unwrap();

    // Taxonomy templates
    std::fs::create_dir_all(templates.join("tags")).unwrap();
    std::fs::write(
        templates.join("tags/list.html"),
        r#"{% extends "base.html" %}{% block content %}{% for term in terms %}{{ term.name }}({{ term.pages | length }}){% endfor %}{% endblock %}"#,
    )
    .unwrap();
    std::fs::write(
        templates.join("tags/single.html"),
        r#"{% extends "base.html" %}{% block content %}{{ term.name }}{% for page in term.pages %}<a>{{ page.title }}</a>{% endfor %}{% endblock %}"#,
    )
    .unwrap();

    root
}

/// Create a test site with SASS
pub fn make_test_site_with_sass(tmp: &TempDir) -> PathBuf {
    let root = make_test_site(tmp);
    let sass_dir = root.join("sass");
    std::fs::create_dir_all(&sass_dir).unwrap();

    // Overwrite config with compile_sass = true
    std::fs::write(
        root.join("config.toml"),
        r#"base_url = "https://example.com"
title = "SASS Test Site"
compile_sass = true
"#,
    )
    .unwrap();

    std::fs::write(
        sass_dir.join("style.scss"),
        "body { color: red; .container { max-width: 800px; } }",
    )
    .unwrap();

    root
}

/// Create a test site with pagination configured
pub fn make_test_site_with_pagination(tmp: &TempDir) -> PathBuf {
    let root = make_test_site(tmp);
    let content = root.join("content");

    // Set paginate_by = 1 on the posts section
    std::fs::write(
        content.join("posts/_index.md"),
        "+++\ntitle = \"Blog\"\nsort_by = \"date\"\npaginate_by = 1\n+++\n",
    )
    .unwrap();

    // Add more posts to trigger pagination
    std::fs::write(
        content.join("posts/second.md"),
        "+++\ntitle = \"Second Post\"\ndate = \"2025-02-01\"\n+++\nSecond.",
    )
    .unwrap();
    std::fs::write(
        content.join("posts/third.md"),
        "+++\ntitle = \"Third Post\"\ndate = \"2025-03-01\"\n+++\nThird.",
    )
    .unwrap();

    root
}
