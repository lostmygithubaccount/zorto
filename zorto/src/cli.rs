use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{serve, site};

#[derive(Parser)]
#[command(
    name = "zorto",
    version,
    about = "A fast static site generator with executable code blocks"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Site root directory
    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Disable execution of code blocks ({python}, {bash}, {sh})
    #[arg(short = 'N', long)]
    no_exec: bool,

    /// Sandbox boundary for file operations (include shortcode, etc.).
    /// Paths cannot escape this directory. Defaults to --root.
    #[arg(long)]
    sandbox: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the site
    Build {
        /// Output directory
        #[arg(short, long, default_value = "public")]
        output: PathBuf,

        /// Include draft pages
        #[arg(long)]
        drafts: bool,

        /// Base URL override
        #[arg(long)]
        base_url: Option<String>,
    },

    /// Start preview server with live reload
    Preview {
        /// Output directory
        #[arg(short, long, default_value = "public")]
        output: PathBuf,

        /// Port number
        #[arg(short, long, default_value = "1111")]
        port: u16,

        /// Include draft pages
        #[arg(long)]
        drafts: bool,

        /// Open browser
        #[arg(short = 'O', long)]
        open: bool,

        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        interface: String,
    },

    /// Remove output directory
    Clean {
        /// Output directory to remove
        #[arg(short, long, default_value = "public")]
        output: PathBuf,
    },

    /// Initialize a new site
    Init {
        /// Site directory name (defaults to current --root)
        name: Option<String>,
    },

    /// Check site for errors without building
    Check {
        /// Include draft pages
        #[arg(long)]
        drafts: bool,
    },
}

/// Run the zorto CLI with the given arguments.
///
/// This is the main entry point, equivalent to calling `zorto` on the command line.
/// Pass `std::env::args()` for normal use, or synthetic args for testing.
pub fn run<I, T>(args: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    let root = std::fs::canonicalize(&cli.root)?;
    let sandbox = resolve_sandbox(&cli.sandbox)?;

    match cli.command {
        Commands::Build {
            output,
            drafts,
            base_url,
        } => {
            let output = resolve_output(&root, output);
            let mut site = site::Site::load(&root, &output, drafts)?;
            site.no_exec = cli.no_exec;
            site.sandbox = sandbox;
            if let Some(url) = base_url {
                site.set_base_url(url);
            }
            site.build()?;
            println!("Site built to {}", output.display());
        }
        Commands::Preview {
            output,
            port,
            drafts,
            open,
            interface,
        } => {
            let output = resolve_output(&root, output);
            let cfg = serve::ServeConfig {
                root: &root,
                output_dir: &output,
                drafts,
                no_exec: cli.no_exec,
                sandbox: sandbox.as_deref(),
                interface: &interface,
                port,
                open_browser: open,
            };
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(serve::serve(&cfg))?;
        }
        Commands::Clean { output } => {
            let output = resolve_output(&root, output);
            if output.exists() {
                std::fs::remove_dir_all(&output)?;
                println!("Removed {}", output.display());
            }
        }
        Commands::Init { name } => {
            let target = match name {
                Some(n) => root.join(n),
                None => root.clone(),
            };
            init_site(&target)?;
        }
        Commands::Check { drafts } => {
            let output = root.join("public");
            let mut site = site::Site::load(&root, &output, drafts)?;
            site.no_exec = cli.no_exec;
            site.sandbox = sandbox;
            site.check()?;
            println!("Site check passed.");
        }
    }

    Ok(())
}

/// Resolve an output path relative to the site root.
fn resolve_output(root: &std::path::Path, output: PathBuf) -> PathBuf {
    if output.is_relative() {
        root.join(output)
    } else {
        output
    }
}

/// Canonicalize the sandbox path, returning an error if it doesn't exist.
fn resolve_sandbox(sandbox: &Option<PathBuf>) -> anyhow::Result<Option<PathBuf>> {
    match sandbox {
        Some(p) => {
            let canonical = std::fs::canonicalize(p)
                .map_err(|e| anyhow::anyhow!("cannot resolve sandbox path {}: {e}", p.display()))?;
            Ok(Some(canonical))
        }
        None => Ok(None),
    }
}

fn init_site(target: &std::path::Path) -> anyhow::Result<()> {
    if target.join("config.toml").exists() {
        anyhow::bail!("config.toml already exists in {}", target.display());
    }

    let content = target.join("content");
    let posts = content.join("posts");
    let templates = target.join("templates");
    let static_dir = target.join("static");

    std::fs::create_dir_all(&posts)?;
    std::fs::create_dir_all(&templates)?;
    std::fs::create_dir_all(&static_dir)?;

    std::fs::write(
        target.join("config.toml"),
        r#"base_url = "https://example.com"
title = "My Site"
generate_feed = true
"#,
    )?;

    std::fs::write(
        content.join("_index.md"),
        r#"+++
title = "Home"
sort_by = "date"
+++
"#,
    )?;

    std::fs::write(
        posts.join("_index.md"),
        r#"+++
title = "Blog"
sort_by = "date"
+++
"#,
    )?;

    std::fs::write(
        posts.join("hello.md"),
        r#"+++
title = "Hello World"
date = "2025-01-01"
description = "My first post"
tags = ["hello"]
+++
Welcome to my new site built with [zorto](https://github.com/lostmygithubaccount/zorto)!
"#,
    )?;

    std::fs::write(
        templates.join("base.html"),
        r#"<!DOCTYPE html>
<html lang="{{ config.default_language }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{% block title %}{{ config.title }}{% endblock %}</title>
    {% if config.generate_feed %}<link rel="alternate" type="application/atom+xml" title="Feed" href="{{ config.base_url }}/atom.xml">{% endif %}
</head>
<body>
    <nav><a href="{{ config.base_url }}/">{{ config.title }}</a></nav>
    <main>{% block content %}{% endblock %}</main>
</body>
</html>
"#,
    )?;

    std::fs::write(
        templates.join("index.html"),
        r#"{% extends "base.html" %}
{% block content %}
<h1>{{ section.title }}</h1>
{{ section.content | safe }}
{% for page in section.pages %}
<article>
    <h2><a href="{{ page.permalink }}">{{ page.title }}</a></h2>
    {% if page.date %}<time>{{ page.date }}</time>{% endif %}
    {% if page.description %}<p>{{ page.description }}</p>{% endif %}
</article>
{% endfor %}
{% endblock %}
"#,
    )?;

    std::fs::write(
        templates.join("section.html"),
        r#"{% extends "base.html" %}
{% block content %}
<h1>{{ section.title }}</h1>
{{ section.content | safe }}
{% for page in section.pages %}
<article>
    <h2><a href="{{ page.permalink }}">{{ page.title }}</a></h2>
    {% if page.date %}<time>{{ page.date }}</time>{% endif %}
    {% if page.description %}<p>{{ page.description }}</p>{% endif %}
</article>
{% endfor %}
{% endblock %}
"#,
    )?;

    std::fs::write(
        templates.join("page.html"),
        r#"{% extends "base.html" %}
{% block title %}{{ page.title }} | {{ config.title }}{% endblock %}
{% block content %}
<article>
    <h1>{{ page.title }}</h1>
    {% if page.date %}<time>{{ page.date }}</time>{% endif %}
    {{ page.content | safe }}
</article>
{% endblock %}
"#,
    )?;

    println!("Initialized new site at {}", target.display());
    Ok(())
}
