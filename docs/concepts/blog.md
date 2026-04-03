# Blog, events, and more

Zorto does not have a special "blog" feature. Instead, its section system naturally handles any date-ordered content: blog posts, changelogs, event listings, release notes, newsletters. If it has a date and belongs in a list, it is a section.

## The pattern

A section is a directory with an `_index.md` file. Every markdown file in that directory becomes a page in the section. The section's frontmatter controls sorting and pagination, while each page's frontmatter provides its metadata.

{% tree(caption="A blog is just a section. No special blog feature — just the content model.") %}
content/posts/
  _index.md  [section: sort_by=date, paginate_by=10]
  first-post.md  [page with date, title, tags]
  second-post.md  [page]
  announcing-v2/
    index.md  [page with co-located assets]
    screenshot.png
{% end %}

This is the same content model used everywhere in Zorto — there is no blog-specific configuration or special directory name. A section called `posts/`, `blog/`, `news/`, or `changelog/` all work identically.

## Consequence: one model for everything

Because blogs are just sections, everything you learn about the content model applies directly. Sorting, pagination, taxonomies, co-located assets, internal links, custom templates — they all work the same way whether you are building a blog, a docs site, or both.

This also means you can have multiple blog-like sections on one site. A `/posts/` for articles and a `/changelog/` for release notes, each with their own sorting and pagination, coexist naturally.

## Taxonomies tie sections together

Tags and categories work across sections. Define taxonomies in your config, assign terms in page frontmatter, and Zorto generates listing pages for each term automatically. A tag page at `/tags/rust/` lists every page tagged "rust" regardless of which section it lives in.

## Summaries

Use `<!-- more -->` in a post to mark where the summary ends. Everything above it appears on listing pages; the full content appears on the post's own page.

## Feeds

When `generate_feed = true` in your config, Zorto generates an Atom feed. Pages need a `date` in their frontmatter to appear in the feed. Pages without dates are silently excluded.

## Further reading

- [Content model](content-model.md) — sections and pages in depth
- [Configuration](configuration.md) — how taxonomies and feed generation are configured
- [How to add a blog](../how-to/add-blog.md) — step-by-step setup
