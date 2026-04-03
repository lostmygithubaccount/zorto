# Add a blog

Set up a blog with posts, tags, pagination, and an Atom feed.

{% tree(caption="What your blog structure will look like after this guide.") %}
content/posts/
  _index.md  [section: blog config]
  my-first-post.md  [page]
  another-post.md  [page]
{% end %}

## Create the section

Create `content/posts/_index.md` with this [frontmatter](../concepts/glossary.md#frontmatter):

```toml
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++
```

This creates a paginated section at `/posts/` that sorts posts by date (newest first).

## Write a post

Create `content/posts/my-first-post.md`:

```markdown
+++
title = "My first post"
date = "2026-01-15"
description = "A short introduction."
tags = ["intro"]
+++

A short summary of the post goes here.

<!-- more -->

The full content continues after the "more" marker. Everything above it becomes the summary shown on listing pages.
```

## Enable tags

Add a [taxonomy](../concepts/glossary.md#taxonomy) to `config.toml`:

```toml
[[taxonomies]]
name = "tags"
```

Zorto automatically generates:
- `/tags/` — list of all tags
- `/tags/intro/` — all posts with the "intro" tag

## Add an Atom feed

Enable [feed generation](../concepts/glossary.md#atom-feed) in `config.toml`:

```toml
# config.toml
generate_feed = true
```

The feed is available at `/atom.xml`. Add a `<link>` tag in your base template's `<head>` so feed readers can discover it automatically:

```html
<link rel="alternate" type="application/atom+xml" title="{{ config.title }}" href="/atom.xml">
```

## Drafts

Set `draft = true` in a post's frontmatter to exclude it from production builds. To preview drafts locally, pass the `--drafts` flag:

```bash
zorto preview --drafts
```

## Related guides

- [Blog, events, and more](../concepts/blog.md) — how sections, pagination, and feeds work under the hood
- [Set up multiple authors](multiple-authors.md) — attribute posts to different authors
- [Organize content](organize-content.md) — nested sections and external content directories
