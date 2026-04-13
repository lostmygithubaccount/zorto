//! Built-in site templates for `zorto init`.

use std::path::Path;

/// A file to be written during site initialization.
struct TemplateFile {
    /// Relative path from the site root.
    path: &'static str,
    /// File contents.
    content: &'static str,
    /// Whether the file should be executable (bin/ scripts).
    executable: bool,
}

// ── Default (starter) template ──────────────────────────────────────────

const DEFAULT_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "config.toml",
        content: include_str!("../templates/default/config.toml"),
        executable: false,
    },
    TemplateFile {
        path: "content/_index.md",
        content: include_str!("../templates/default/content/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/posts/_index.md",
        content: include_str!("../templates/default/content/posts/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/posts/hello.md",
        content: include_str!("../templates/default/content/posts/hello.md"),
        executable: false,
    },
];

// ── Blog template ───────────────────────────────────────────────────────

const BLOG_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "config.toml",
        content: include_str!("../templates/blog/config.toml"),
        executable: false,
    },
    TemplateFile {
        path: "content/_index.md",
        content: include_str!("../templates/blog/content/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/posts/_index.md",
        content: include_str!("../templates/blog/content/posts/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/posts/hello-world.md",
        content: include_str!("../templates/blog/content/posts/hello-world.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/posts/getting-started.md",
        content: include_str!("../templates/blog/content/posts/getting-started.md"),
        executable: false,
    },
];

// ── Docs template ───────────────────────────────────────────────────────

const DOCS_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "config.toml",
        content: include_str!("../templates/docs/config.toml"),
        executable: false,
    },
    TemplateFile {
        path: "content/_index.md",
        content: include_str!("../templates/docs/content/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/guide/_index.md",
        content: include_str!("../templates/docs/content/guide/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/guide/introduction.md",
        content: include_str!("../templates/docs/content/guide/introduction.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/guide/installation.md",
        content: include_str!("../templates/docs/content/guide/installation.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/guide/configuration.md",
        content: include_str!("../templates/docs/content/guide/configuration.md"),
        executable: false,
    },
];

// ── Business template ────────────────────────────────────────────────────

const BUSINESS_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "config.toml",
        content: include_str!("../templates/business/config.toml"),
        executable: false,
    },
    TemplateFile {
        path: "content/_index.md",
        content: include_str!("../templates/business/content/_index.md"),
        executable: false,
    },
];

// ── Presentation template ────────────────────────────────────────────────

const PRESENTATION_FILES: &[TemplateFile] = &[
    TemplateFile {
        path: "config.toml",
        content: include_str!("../templates/presentation/config.toml"),
        executable: false,
    },
    TemplateFile {
        path: "content/_index.md",
        content: include_str!("../templates/presentation/content/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/_index.md",
        content: include_str!("../templates/presentation/content/intro/_index.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/title.md",
        content: include_str!("../templates/presentation/content/intro/title.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/bullets.md",
        content: include_str!("../templates/presentation/content/intro/bullets.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/code.md",
        content: include_str!("../templates/presentation/content/intro/code.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/fragments.md",
        content: include_str!("../templates/presentation/content/intro/fragments.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/columns.md",
        content: include_str!("../templates/presentation/content/intro/columns.md"),
        executable: false,
    },
    TemplateFile {
        path: "content/intro/thank-you.md",
        content: include_str!("../templates/presentation/content/intro/thank-you.md"),
        executable: false,
    },
    TemplateFile {
        path: "templates/base.html",
        content: include_str!("../templates/presentation/templates/base.html"),
        executable: false,
    },
    TemplateFile {
        path: "templates/index.html",
        content: include_str!("../templates/presentation/templates/index.html"),
        executable: false,
    },
    TemplateFile {
        path: "templates/section.html",
        content: include_str!("../templates/presentation/templates/section.html"),
        executable: false,
    },
    TemplateFile {
        path: "templates/page.html",
        content: include_str!("../templates/presentation/templates/page.html"),
        executable: false,
    },
    TemplateFile {
        path: "templates/presentation.html",
        content: include_str!("../templates/presentation/templates/presentation.html"),
        executable: false,
    },
];

/// Template metadata for interactive selection.
pub struct TemplateInfo {
    pub name: &'static str,
    pub description: &'static str,
}

/// Available templates with descriptions.
pub const TEMPLATES: &[TemplateInfo] = &[
    TemplateInfo {
        name: "default",
        description: "Clean starter site",
    },
    TemplateInfo {
        name: "blog",
        description: "Blog with example posts",
    },
    TemplateInfo {
        name: "docs",
        description: "Documentation site",
    },
    TemplateInfo {
        name: "business",
        description: "Business / landing page",
    },
    TemplateInfo {
        name: "presentation",
        description: "reveal.js slide deck (one file per slide)",
    },
];

/// Available template names.
pub const TEMPLATE_NAMES: &[&str] = &["default", "blog", "docs", "business", "presentation"];

/// Write all files for the given template into `target`.
pub fn write_template(target: &Path, template: &str) -> anyhow::Result<()> {
    let files = match template {
        "default" => DEFAULT_FILES,
        "blog" => BLOG_FILES,
        "docs" => DOCS_FILES,
        "business" => BUSINESS_FILES,
        "presentation" => PRESENTATION_FILES,
        _ => anyhow::bail!(
            "unknown template \"{template}\". Available templates: {}",
            TEMPLATE_NAMES.join(", ")
        ),
    };

    for file in files {
        let dest = target.join(file.path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, file.content)?;

        #[cfg(unix)]
        if file.executable {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    // Ensure static/ directory exists even if no static files are in the template.
    std::fs::create_dir_all(target.join("static"))?;

    Ok(())
}

/// Rewrite the config.toml at `target` with user-provided values.
///
/// This does a simple string replacement on the template-generated config.
pub fn customize_config(
    target: &Path,
    title: &str,
    base_url: &str,
    theme: Option<&str>,
    author: Option<&str>,
) -> anyhow::Result<()> {
    let config_path = target.join("config.toml");
    let content = std::fs::read_to_string(&config_path)?;

    let mut lines: Vec<String> = Vec::new();
    let mut has_theme = false;

    for line in content.lines() {
        if line.starts_with("base_url") {
            lines.push(format!("base_url = \"{base_url}\""));
        } else if line.starts_with("title") && !line.starts_with("title =")
            || line.starts_with("title =")
        {
            lines.push(format!("title = \"{}\"", title.replace('\"', "\\\"")));
        } else if line.starts_with("theme") {
            has_theme = true;
            if let Some(t) = theme {
                lines.push(format!("theme = \"{t}\""));
            } else {
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }

    // If theme was requested but not already in the config, insert it after base_url
    if let Some(t) = theme {
        if !has_theme {
            if let Some(pos) = lines.iter().position(|l| l.starts_with("base_url")) {
                lines.insert(pos + 1, format!("theme = \"{t}\""));
            }
        }
    }

    // If author was provided and template has an [extra] section, update it
    if let Some(author_name) = author {
        if let Some(pos) = lines.iter().position(|l| l.starts_with("author =")) {
            lines[pos] = format!("author = \"{}\"", author_name.replace('\"', "\\\""));
        }
    }

    // Update copyright_html with the actual site name and (optionally)
    // author. When no author is provided we drop the "by X" segment
    // entirely rather than baking in a literal "Author" placeholder.
    if let Some(pos) = lines
        .iter()
        .position(|l| l.trim_start().starts_with("copyright_html"))
    {
        let safe_title = title.replace('\'', "&#39;");
        let by_segment = match author {
            Some(a) => format!(" by {}", a.replace('\'', "&#39;")),
            None => String::new(),
        };
        lines[pos] = format!(
            "copyright_html = '<a href=\"/\">{safe_title}</a>{by_segment} via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'"
        );
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    std::fs::write(&config_path, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_minimal_config(target: &Path) {
        std::fs::create_dir_all(target).unwrap();
        std::fs::write(
            target.join("config.toml"),
            "base_url = \"https://example.com\"\n\
             title = \"X\"\n\
             [extra]\n\
             copyright_html = '<a href=\"/\">X</a> via <a href=\"https://zorto.dev\" target=\"_blank\" rel=\"noopener\">Zorto</a>'\n",
        )
        .unwrap();
    }

    #[test]
    fn customize_config_omits_by_when_no_author() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());
        customize_config(dir.path(), "My Site", "https://example.com", None, None).unwrap();
        let cfg = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(
            cfg.contains("My Site</a> via"),
            "expected `<title></a> via` with no `by` segment, got:\n{cfg}"
        );
        assert!(
            !cfg.contains(" by "),
            "expected no ` by ` segment when author is None, got:\n{cfg}"
        );
        assert!(
            !cfg.contains("Author"),
            "expected no literal `Author` placeholder, got:\n{cfg}"
        );
    }

    #[test]
    fn customize_config_includes_by_when_author_provided() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());
        customize_config(
            dir.path(),
            "My Site",
            "https://example.com",
            None,
            Some("Alice"),
        )
        .unwrap();
        let cfg = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(
            cfg.contains("by Alice via"),
            "expected `by Alice via`, got:\n{cfg}"
        );
    }

    #[test]
    fn customize_config_escapes_apostrophe_in_author() {
        let dir = tempfile::tempdir().unwrap();
        write_minimal_config(dir.path());
        customize_config(
            dir.path(),
            "My Site",
            "https://example.com",
            None,
            Some("O'Brien"),
        )
        .unwrap();
        let cfg = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        // Apostrophes in the author would otherwise close the TOML
        // literal-string copyright_html value.
        assert!(
            cfg.contains("O&#39;Brien"),
            "expected apostrophe to be HTML-encoded, got:\n{cfg}"
        );
    }
}
