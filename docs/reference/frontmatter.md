# Frontmatter reference

Complete specification for TOML frontmatter in Zorto content files. Frontmatter is enclosed in `+++` delimiters at the top of every `.md` file.

```toml
+++
title = "My page"
date = "2026-01-15"
+++
```

If no `+++` block is present, Zorto uses default values for all fields.

## Page frontmatter

Used in regular `.md` files (anything that is not `_index.md`).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | string | `""` | Page title, used in templates and feeds |
| `date` | string or datetime | *none* | Publication date (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SS`). Pages without dates sort last in date-ordered sections |
| `author` | string | *none* | Author name |
| `description` | string | *none* | Short summary for SEO, feeds, and `llms.txt` |
| `draft` | bool | `false` | If `true`, excluded from production builds |
| `slug` | string | filename | Override the URL slug. By default, derived from the filename (e.g. `my-post.md` becomes `my-post`) |
| `template` | string | `"page.html"` | Custom template for this page |
| `aliases` | array of strings | `[]` | Additional URL paths that redirect to this page |
| `[extra]` | table | `{}` | Arbitrary key-value data, accessible in templates as `page.extra` |
| taxonomy fields | array of strings | `[]` | Top-level arrays are interpreted as taxonomy values (e.g. `tags = ["rust"]`) |

### Computed fields

These fields are not set in frontmatter but are available in templates:

| Field | Type | Description |
|-------|------|-------------|
| `page.path` | string | URL path relative to site root (e.g. `"/posts/hello/"`) |
| `page.permalink` | string | Full URL including `base_url` |
| `page.content` | string | Rendered HTML content |
| `page.summary` | string or null | HTML content before `<!-- more -->` marker |
| `page.raw_content` | string | Raw markdown after frontmatter extraction |
| `page.taxonomies` | object | Taxonomy values keyed by name (e.g. `{"tags": ["rust", "web"]}`) |
| `page.word_count` | int | Approximate word count |
| `page.reading_time` | int | Estimated reading time in minutes (word_count / 200, minimum 1) |
| `page.relative_path` | string | Source file path relative to content directory |

## Section frontmatter

Used in `_index.md` files that define sections (directories).

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | string | `""` | Section title |
| `description` | string | *none* | Section description |
| `sort_by` | string | *none* | Sort pages: `"date"` (reverse chronological) or `"title"` (alphabetical) |
| `paginate_by` | int | *none* | Pages per pagination page. Omit or set to `0` to disable pagination |
| `template` | string | `"section.html"` | Custom template for this section |
| `[extra]` | table | `{}` | Arbitrary key-value data, accessible in templates as `section.extra` |

### Computed fields

| Field | Type | Description |
|-------|------|-------------|
| `section.path` | string | URL path relative to site root (e.g. `"/posts/"`) |
| `section.permalink` | string | Full URL including `base_url` |
| `section.content` | string | Rendered HTML from the `_index.md` body |
| `section.raw_content` | string | Raw markdown after frontmatter extraction |
| `section.pages` | array | Pages belonging to this section, sorted per `sort_by` |
| `section.relative_path` | string | Source file path relative to content directory |

## Taxonomy values

Taxonomy values are defined as top-level arrays in page frontmatter. Any top-level key whose value is an array of strings is treated as a taxonomy assignment:

```toml
+++
title = "Building a web app"
tags = ["rust", "web", "tutorial"]
categories = ["engineering"]
+++
```

The taxonomy names must match those defined in `config.toml` under `[[taxonomies]]`. See [taxonomies in depth](taxonomies.md) for the full workflow.

## Slug derivation

The URL slug determines the page's URL path. Zorto derives it in this order:

1. **Explicit `slug` field** in frontmatter, if set
2. **Directory name** for co-located content (`posts/my-post/index.md` uses slug `my-post`)
3. **Filename** for regular files (`my-post.md` uses slug `my-post`)

Slugs are always lowercased and URL-safe (spaces become hyphens, special characters are removed).

## Co-located content

Pages can use directory-based organization for co-located assets:

{% tree(caption="Co-located content uses index.md inside a named directory.") %}
content/posts/
  my-post/
    index.md  [page -> /posts/my-post/]
    photo.jpg
    diagram.svg
{% end %}

The `index.md` file becomes the page content, and sibling files are copied as assets. Reference them with relative paths in your markdown.

## Date formats

The `date` field accepts:

- **Date string**: `"2026-01-15"`
- **Datetime string**: `"2026-01-15T10:30:00"`
- **TOML datetime**: `2026-01-15T10:30:00` (unquoted)

All formats are normalized to a string for template rendering.

## Examples

### Minimal page

```toml
+++
title = "About"
+++
```

### Full page

```toml
+++
title = "Building a static site generator"
date = "2026-03-15"
author = "Cody"
description = "How and why I built Zorto."
draft = false
slug = "building-zorto"
template = "post.html"
aliases = ["/blog/old-url/"]
tags = ["rust", "ssg"]
categories = ["engineering"]

[extra]
featured = true
hero_image = "/images/zorto-hero.png"
+++
```

### Section with pagination

```toml
+++
title = "Blog"
description = "Posts about engineering and design."
sort_by = "date"
paginate_by = 10
template = "blog.html"

[extra]
show_sidebar = true
+++
```

## Further reading

- [Content model](../concepts/content-model.md) — sections, pages, and internal links
- [Configuration reference](config.md) — site-level settings
- [Templates](../concepts/templates.md) — how frontmatter fields are used in templates
- [Taxonomies in depth](taxonomies.md) — defining and assigning taxonomy terms
