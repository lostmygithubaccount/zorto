# Content directories

Pull content from outside the `content/` directory using `[[content_dirs]]` in `config.toml`. This is ideal for documentation that lives alongside source code, or content shared between multiple sites.

## Configuration

Add one or more `[[content_dirs]]` entries to your `config.toml`:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `path` | string | *required* | Path to the external directory (relative to site root) |
| `url_prefix` | string | *required* | URL prefix for generated pages (e.g. `"docs"` produces `/docs/...`) |
| `template` | string | `"page.html"` | Template for generated pages |
| `section_template` | string | `"section.html"` | Template for generated sections |
| `sort_by` | string | *none* | Sort pages: `"date"` or `"title"` |
| `rewrite_links` | bool | `false` | Rewrite relative `.md` links to clean URL paths |
| `exclude` | array of strings | `[]` | Files to skip (relative to the external directory) |

## How it works

Zorto walks the external directory and converts its files:

- **`README.md`** files become sections (equivalent to `_index.md` in `content/`)
- **Other `.md` files** become pages
- The `url_prefix` is prepended to all generated URL paths

{% tree(caption="External directory structure maps to site URLs.") %}
docs/
  README.md  [section -> /docs/]
  getting-started/
    README.md  [section -> /docs/getting-started/]
    installation.md  [page -> /docs/getting-started/installation/]
    quick-start.md  [page -> /docs/getting-started/quick-start/]
  reference/
    README.md  [section -> /docs/reference/]
    cli.md  [page -> /docs/reference/cli/]
{% end %}

### Title extraction

Since external markdown files typically lack TOML frontmatter, Zorto extracts:

- **Title** from the first `# Heading` line (or derived from the filename if absent)
- **Description** from the first paragraph of prose

The title heading is stripped from the rendered body to avoid duplication.

## Link rewriting

When `rewrite_links = true`, Zorto rewrites relative `.md` links in the external content to clean URL paths that work on the built site:

| Original link | Rewritten to |
|---------------|-------------|
| `[Install](installation.md)` | `[Install](/docs/getting-started/installation/)` |
| `[CLI](../reference/cli.md)` | `[CLI](/docs/reference/cli/)` |
| `[GitHub](https://github.com)` | unchanged |

This allows the same markdown files to work as documentation on both GitHub (where `.md` links are clickable) and the built site (where clean URLs are used).

## Excluding files

Use `exclude` to skip specific files. This is useful when you have manual content in `content/` that should take precedence:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
exclude = ["reference/cli.md", "internal/draft.md"]
```

Excluded files are expected to exist as manually authored content in the `content/` directory.

## Custom templates

Assign dedicated templates for external content:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "docs.html"
section_template = "docs-section.html"
sort_by = "title"
```

The `template` field is applied to all pages loaded from this directory. The `section_template` is applied to all sections (directories with `README.md`).

## Real-world example

The [zorto.dev](https://zorto.dev) website uses `content_dirs` to pull in its own documentation:

```toml
# website/config.toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "docs.html"
section_template = "docs-section.html"
sort_by = "title"
rewrite_links = true
```

The `docs/` directory lives at the repository root alongside the source code. The website in `website/` pulls it in as content under `/docs/`. This means:

- Documentation files are readable on GitHub with working relative links
- The same files render as pages on zorto.dev with clean URLs
- No content duplication between the repo and the website

## Multiple content directories

You can define multiple `[[content_dirs]]` entries:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
rewrite_links = true

[[content_dirs]]
path = "../api-docs"
url_prefix = "api"
template = "api-doc.html"
sort_by = "title"
```

Each directory is independent with its own URL prefix, templates, and settings.

## Further reading

- [Organize content](../how-to/organize-content.md) — sections, nested sections, and content structure
- [Configuration reference](config.md) — complete `config.toml` reference
- [Content model](../concepts/content-model.md) — sections and pages
