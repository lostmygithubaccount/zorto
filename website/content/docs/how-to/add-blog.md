+++
title = "Add a blog"
template = "docs.html"
+++

## Create the section

Create `content/posts/_index.md`:

```toml
+++
title = "Blog"
template = "docs.html"
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
template = "docs.html"
date = "2026-01-15"
author = "Cody"

[taxonomies]
tags = ["intro"]
+++

A short summary of the post goes here.

<!-- more -->

The full content continues after the "more" marker. Everything before it becomes the summary shown on listing pages.
```

## Enable tags

Add the taxonomy to `config.toml`:

```toml
[[taxonomies]]
name = "tags"
```

Zorto automatically generates:

- `/tags/` -- list of all tags
- `/tags/intro/` -- all posts with the "intro" tag

## Feed

To generate an Atom feed for your blog:

```toml
# config.toml
generate_feed = true
```

The feed is available at `/atom.xml`.
