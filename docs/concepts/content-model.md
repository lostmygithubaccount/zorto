# Content model

Zorto organizes content into sections and pages, derives URLs from the file structure, supports internal linking with build-time validation, and lets you co-locate assets alongside your markdown.

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

## Permalinks

Every page and section gets a **permalink** — an absolute URL combining `base_url` from your config with the page's path. The path is derived from the file's location in the content directory:

| File path | URL path | Permalink (with `base_url = "https://example.com"`) |
|-----------|----------|------------------------------------------------------|
| `content/about.md` | `/about/` | `https://example.com/about/` |
| `content/posts/hello.md` | `/posts/hello/` | `https://example.com/posts/hello/` |
| `content/posts/_index.md` | `/posts/` | `https://example.com/posts/` |
| `content/posts/my-post/index.md` | `/posts/my-post/` | `https://example.com/posts/my-post/` |

Permalinks are available in templates as `page.permalink` and `section.permalink`. They are used for canonical URLs, Open Graph tags, feeds, and sitemaps.

## Slugs

The **slug** is the URL-safe name for a page, derived from the filename by default. Zorto uses the `slug` crate to convert filenames to lowercase, ASCII-only strings with hyphens:

| Source | Slug |
|--------|------|
| `My First Post.md` | `my-first-post` |
| `Héllo Wörld.md` | `hello-world` |
| `posts/my-post/index.md` | `my-post` (from the directory name) |

Override the slug in frontmatter to decouple the URL from the filename:

```toml
+++
title = "A very long title that you do not want in the URL"
slug = "short-url"
+++
```

This page renders at `/short-url/` regardless of its filename. Co-located pages (`index.md` inside a directory) derive their slug from the directory name, not the filename — the custom `slug` field overrides that too.

## Internal links with `@/`

Link to other content files using the `@/` prefix:

```text
[About](&#64;/about.md)
[First post](&#64;/posts/first-post.md)
[Blog section](&#64;/posts/_index.md)
```

Zorto resolves these to the correct URLs at build time. The path after `@/` is relative to the `content/` directory.

Anchor links work too:

```text
[Installation section](&#64;/getting-started.md#installation)
```

If the target file does not exist, Zorto emits a warning during the build:

```
unresolved internal link: posts/missing.md (no matching page or section found)
```

This gives you broken-link detection without an external tool. Use `zorto check` to validate all internal links without building the full site.

## Summaries

Use `<!-- more -->` in a page's body to mark where the summary ends:

```markdown
This is the summary shown on listing pages.

<!-- more -->

The full content continues here.
```

Everything above the marker becomes the page's `summary`, used in section listings and feeds. The summary is rendered as HTML — markdown formatting, links, and inline code all work.

If no `<!-- more -->` marker is present, the `summary` field is `None` in templates. Use the `description` frontmatter field as a fallback for feed entries and meta tags.

In templates, use the summary like this:

<pre><code>&#123;%- if page.summary %&#125;
  &#123;&#123; page.summary | safe &#125;&#125;
&#123;%- elif page.description %&#125;
  &#123;&#123; page.description &#125;&#125;
&#123;%- endif %&#125;</code></pre>

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

During the build, Zorto copies co-located assets to the page's output directory, preserving the relative path relationship. The output looks like:

{% tree(caption="Assets are copied alongside the rendered HTML.") %}
public/posts/my-post/
  index.html
  photo.jpg
  diagram.svg
{% end %}

Any non-markdown file inside a content directory is treated as a co-located asset. This includes images (`.jpg`, `.png`, `.svg`, `.gif`, `.webp`), PDFs, data files, and anything else.

For site-wide assets that are not tied to a specific page (favicons, global images, fonts), use the `static/` directory instead. See [Asset management](../how-to/assets.md).

## Further reading

- [Configuration reference](../reference/config.md) — complete frontmatter field list
- [Blog, events, and more](blog.md) — using sections for date-ordered content
- [Templates](templates.md) — how sections and pages map to templates
- [Organize content](../how-to/organize-content.md) — nested sections and external content directories
- [Asset management](../how-to/assets.md) — static files, co-located content, fonts, and images
