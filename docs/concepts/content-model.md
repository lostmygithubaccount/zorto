# Content model

## Sections vs pages

Zorto content has two types:

- **Sections** are directories with an `_index.md` file. They can list their child pages and subsections.
- **Pages** are individual `.md` files. They render as standalone pages.

{{ compare(left_title="Section", left="A directory with _index.md. Lists child pages, supports pagination and sorting.", right_title="Page", right="An individual .md file. Renders as a standalone URL at its file path.") }}

A file at `content/posts/_index.md` creates a section at `/posts/`. A file at `content/about.md` creates a page at `/about/`.

{% tree(caption="Directories are sections, files are pages. The _index.md file turns a directory into a section.") %}
content/
  _index.md  [section → /]
  about.md  [page → /about/]
  posts/
    _index.md  [section → /posts/]
    first-post.md  [page → /posts/first-post/]
    second-post.md  [page → /posts/second-post/]
{% end %}

## Frontmatter

Every content file starts with TOML frontmatter between `+++` delimiters:

```toml
+++
title = "My page"
date = "2026-01-15"
author = "Cody"
description = "A short summary for SEO and feeds."
draft = true
slug = "custom-url"
template = "custom-page.html"
tags = ["rust", "ssg"]

[extra]
custom_field = "any value you want"
+++
```

| Field | Type | Description |
|-------|------|-------------|
| `title` | string | Page title (required) |
| `date` | string | Publication date (YYYY-MM-DD) |
| `author` | string | Author name |
| `description` | string | Summary for SEO and feeds |
| `draft` | bool | If true, excluded from builds (default: false) |
| `slug` | string | Override the URL slug |
| `template` | string | Use a custom template |
| `aliases` | array of strings | Redirect old URLs to this page |
| `sort_by` | string | Sort child pages: `"date"` (newest first) or `"title"` (sections only) |
| `paginate_by` | int | Number of items per page, 0 = no pagination (sections only) |
| taxonomy fields | array of strings | Taxonomy values as top-level arrays (e.g. `tags = ["rust", "ssg"]`) |
| `[extra]` | table | Arbitrary custom data, accessible in templates |

## Summaries

Use `<!-- more -->` in a page's body to mark where the summary ends:

```markdown
This is the summary shown on listing pages.

<!-- more -->

The full content continues here.
```

Everything above the marker becomes the page's `summary`, used in section listings and feeds.

## Co-located assets

Place images and other assets next to your markdown files:

{% tree(caption="Content and assets live together — no separate static/images/ directory needed.") %}
content/posts/my-post/
  index.md  [page]
  photo.jpg
  diagram.svg
{% end %}

Reference them with relative paths in your markdown:

```markdown
![A photo](photo.jpg)
```

## Internal links

Link to other content files using the `@/` prefix:

```text
[About](&#64;/about.md)
[First post](&#64;/posts/first-post.md)
[Blog section](&#64;/posts/_index.md)
```

Zorto resolves these to the correct URLs at build time and warns if the target doesn't exist.

## Further reading

- [Configuration reference](../reference/config.md) — complete frontmatter field list
- [Blog, events, and more](blog.md) — using sections for date-ordered content
- [Templates](templates.md) — how sections and pages map to templates
- [Organize content](../how-to/organize-content.md) — nested sections and external content directories
