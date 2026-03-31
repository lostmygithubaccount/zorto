## Sections vs pages

Zorto content has two types:

- **Sections** are directories with an `_index.md` file. They can list their child pages and subsections.
- **Pages** are individual `.md` files. They render as standalone pages.

A file at `content/posts/_index.md` creates a section at `/posts/`. A file at `content/about.md` creates a page at `/about/`.

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
| `sort_by` | string | Sort child pages: `"date"` or `"title"` (sections only) |
| `paginate_by` | int | Number of items per page, 0 = no pagination (sections only) |
| taxonomy fields | array of strings | Taxonomy values as top-level arrays (e.g. `tags = ["rust", "ssg"]`) |
| `[extra]` | table | Arbitrary custom data, accessible in templates |

## Co-located assets

Place images and other assets next to your markdown files:

```
content/posts/my-post/
├── index.md
├── photo.jpg
└── diagram.svg
```

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
