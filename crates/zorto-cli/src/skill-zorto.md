---
name: zorto
description: Use the zorto CLI to build and manage static sites with executable code blocks.
---

# Zorto CLI

Build and manage static sites with the `zorto` CLI. Zorto is an AI-native static site generator (SSG) with executable code blocks, inspired by Zola and Quarto.

## Installation

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh   # recommended
uv tool install zorto                                # via uv
cargo install zorto                                  # via cargo
```

## Commands

### Build

```bash
zorto build                          # build site to public/
zorto build --output dist            # custom output directory
zorto build --drafts                 # include draft pages
zorto build --base-url https://...   # override base URL
zorto --no-exec build                # skip executable code blocks
zorto --sandbox . --root website build  # build with sandbox boundary
```

### Preview

```bash
zorto preview                        # dev server on http://127.0.0.1:1111
zorto preview --port 3000            # custom port
zorto preview --open                 # open browser automatically
zorto preview --drafts               # include draft pages
```

### Check

```bash
zorto check                          # validate site without building
zorto check --drafts                 # include drafts in check
zorto check --deny-warnings          # treat warnings as errors
```

### Init

```bash
zorto init                           # initialize site in current directory
zorto init my-site                   # create new site directory
zorto init --template business       # use business template
```

### Clean

```bash
zorto clean                          # remove public/ output directory
zorto clean --output dist            # remove custom output directory
```

### Global flags

| Flag | Description |
|------|-------------|
| `--root <DIR>` | Site root directory (default: `.`) |
| `--no-exec` / `-N` | Disable executable code blocks |
| `--sandbox <DIR>` | Sandbox boundary for file operations |

## Project structure

```
my-site/
  config.toml          # site configuration (required)
  content/             # markdown content
    _index.md          # homepage section
    about.md           # standalone page -> /about/
    posts/
      _index.md        # blog section -> /posts/
      my-post.md       # blog post -> /posts/my-post/
  templates/           # Tera template overrides
    shortcodes/        # custom shortcodes
  sass/                # SCSS style overrides
  static/              # static files copied as-is
  public/              # build output (gitignored)
```

## Configuration (config.toml)

Minimal:

```toml
base_url = "https://example.com"
title = "My site"
```

Full example:

```toml
base_url = "https://example.com"
title = "My site"
description = "A site built with Zorto"
theme = "dkdc"

compile_sass = true
generate_feed = true
generate_sitemap = true
generate_llms_txt = true
generate_md_files = false

[markdown]
highlight_code = true
insert_anchor_links = "right"
external_links_target_blank = true
external_links_no_follow = true
external_links_no_referrer = true
smart_punctuation = true

[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"

[extra]
author = "Your Name"
```

### External content directories

Pull content from outside `content/`:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "docs.html"
section_template = "docs-section.html"
sort_by = "title"
rewrite_links = true
```

## Content model

### Sections vs pages

- **Sections**: directories with `_index.md` — list child pages, support pagination and sorting
- **Pages**: individual `.md` files — render as standalone pages

### Frontmatter

TOML frontmatter between `+++` delimiters:

```toml
+++
title = "My page"
date = "2026-01-15"
author = "Name"
description = "Summary for SEO"
draft = false
slug = "custom-url"
template = "custom-page.html"
tags = ["rust", "ssg"]

[extra]
custom_field = "value"
+++
```

### Section frontmatter (_index.md)

| Field | Default | Description |
|-------|---------|-------------|
| `title` | `""` | Section title |
| `description` | `""` | Section description |
| `sort_by` | `"date"` | Sort by: `"date"`, `"title"` |
| `paginate_by` | `0` | Items per page (0 = no pagination) |
| `template` | `"section.html"` | Custom template |

### Summaries

Use `<!-- more -->` to mark where the summary ends. Everything above becomes `page.summary`.

### Internal links

Use `@/` prefix to link to content files: `[About](@/about.md)`. Resolved at build time.

## Executable code blocks

Use `{python}`, `{bash}`, or `{sh}` language tags to execute code at build time:

````markdown
```{python}
print("Hello from Python")
```

```{bash}
echo "Hello from Bash"
```

```{python file="scripts/analysis.py"}
```
````

- stdout is captured and rendered below the code block
- stderr shows as a warning block
- Non-zero exit codes produce an error block
- Use `--no-exec` flag to skip execution
- Python runs in-process via PyO3; `.venv` site-packages are auto-detected

## Templates (Tera)

Zorto uses Tera (Jinja2-like syntax). Template hierarchy:

| Template | Used for |
|----------|----------|
| `base.html` | Base layout |
| `index.html` | Homepage |
| `section.html` | Section listing pages |
| `page.html` | Individual pages |
| `404.html` | Not-found page |
| `taxonomy_list.html` | Taxonomy index |
| `taxonomy_single.html` | Single taxonomy term |

Template context variables: `page`, `section`, `config`, `paginator`.

Custom functions: `get_url(path)`, `get_section(path)`, `get_taxonomy_url(kind, name)`, `now()`.

## Themes

Built-in themes: `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`.

Set in config.toml: `theme = "zorto"`. Local files in `templates/` and `sass/` override theme defaults.

## Shortcodes

### Inline shortcodes (no body)

```
{{ figure(src="/img/photo.jpg", alt="Photo", caption="Caption") }}
{{ youtube(id="dQw4w9WgXcQ") }}
{{ gist(url="https://gist.github.com/user/abc123") }}
{{ include(path="README.md", strip_frontmatter="true") }}
```

### Body shortcodes (wrap content)

```
{% note(type="warning") %}
Warning text here.
{% end %}

{% tabs(labels="Rust|Python") %}
Rust content
<!-- tab -->
Python content
{% end %}

{% details(summary="Click to expand") %}
Hidden content.
{% end %}

{% mermaid() %}
graph LR
    A --> B
{% end %}
```

### Custom shortcodes

Create `templates/shortcodes/name.html`. Parameters become template variables. Body shortcodes receive `body`.

## Key constraints

- **Sandbox flag**: When building a site that uses `content_dirs` referencing paths outside the site root (e.g., `../docs`), you must pass `--sandbox <parent-dir>` to allow file access beyond `--root`. Without it, builds fail with sandbox errors.
- **Never build directly inside `website/`** in the zorto repo itself — use `--sandbox .` from the repo root: `cargo run -p zorto -- --root website --sandbox . build`
- Executable code blocks run with the same permissions as the zorto process. Use `--no-exec` for untrusted content.
- `uv` should be used for all Python operations (not `pip` or `python` directly).
