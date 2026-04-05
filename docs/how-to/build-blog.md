# Build a blog

Create a blog from scratch with posts, tags, pagination, custom themes, and deployment.

{% tree(caption="What your blog will look like after this guide.") %}
my-blog/
  config.toml  [site config]
  content/
    _index.md  [section: homepage]
    posts/
      _index.md  [section: paginated post listing]
      hello-world.md  [page]
      getting-started.md  [page]
  static/  [images, favicon]
{% end %}

## Initialize the project

Scaffold a new blog with the `blog` template:

```bash
zorto init --template blog my-blog
cd my-blog
```

This creates a site with an Atom feed, code highlighting, and two example posts. Start the dev server:

```bash
zorto preview --open
```

Your blog is live at `http://localhost:1111`.

## Understand the structure

The generated `config.toml` looks like this:

```toml
base_url = "http://localhost:1111"
title = "My Blog"
theme = "default"
generate_feed = true

[markdown]
highlight_code = true

[extra]
copyright_html = '<a href="/">My Blog</a> by Author via <a href="https://zorto.dev" target="_blank" rel="noopener">Zorto</a>'
```

The homepage (`content/_index.md`) is a section that sorts by date and paginates:

```toml
+++
title = "Home"
sort_by = "date"
paginate_by = 10
+++
```

The posts section (`content/posts/_index.md`) lists all blog posts:

```toml
+++
title = "Posts"
sort_by = "date"
+++
```

## Write a new post

Create `content/posts/my-new-post.md`:

```markdown
+++
title = "My new post"
date = "2026-04-04"
description = "A short description for SEO and feeds."
tags = ["tutorial"]
+++

A short summary of the post appears here.

<!-- more -->

The full content continues after the "more" marker. Everything above it becomes the summary shown on listing pages and in the Atom feed.

## A heading in the post

Regular markdown works: **bold**, *italic*, `code`, [links](https://example.com), and images.
```

The `date` field determines sort order (newest first) and inclusion in the Atom feed. The `<!-- more -->` marker splits the summary from the full content.

## Enable tags

Add a taxonomy to `config.toml`:

```toml
[[taxonomies]]
name = "tags"
```

Then assign tags in each post's frontmatter:

```toml
+++
title = "My new post"
date = "2026-04-04"
tags = ["tutorial", "rust"]
+++
```

Zorto automatically generates:

- `/tags/` — list of all tags
- `/tags/tutorial/` — all posts tagged "tutorial"
- `/tags/rust/` — all posts tagged "rust"

### Add categories too

You can define multiple taxonomies:

```toml
[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"
```

Then use both in frontmatter:

```toml
+++
title = "My new post"
date = "2026-04-04"
tags = ["tutorial"]
categories = ["tech"]
+++
```

## Configure pagination

Control how many posts appear per page in the section's `_index.md`:

```toml
+++
title = "Posts"
sort_by = "date"
paginate_by = 5
+++
```

Zorto generates paginated pages: `/posts/`, `/posts/page/2/`, `/posts/page/3/`, and so on.

## Choose a theme

Change the theme in `config.toml`:

```toml
theme = "ocean"
```

Available themes: `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`, `midnight`, `sunset`, `mint`, `plum`, `sand`, `arctic`, `lime`, `charcoal`. All support light and dark mode.

Preview different themes by changing the value and checking the dev server — live reload picks up config changes.

## Customize colors

Override CSS variables without changing the theme. Create `sass/custom.scss`:

```scss
:root {
  --accent: #e74c3c;
  --background: #1a1a2e;
  --max-width: 800px;
}
```

Then load it via the `extra_head` block. Create `templates/base.html`:

```html
{% extends "base.html" %}

{% block extra_head %}
  <link rel="stylesheet" href="/custom.css">
{% endblock %}
```

Zorto compiles SCSS to CSS at build time — `sass/custom.scss` becomes `/custom.css` in the output.

## Add an about page

Create `content/about.md`:

```markdown
+++
title = "About"
+++

This is my blog about Rust, static sites, and building things.
```

This creates a standalone page at `/about/` — it is not part of any section.

## Set up multiple authors

If your blog has multiple authors, add an `authors` taxonomy:

```toml
# config.toml
[[taxonomies]]
name = "authors"
```

Then use it in each post:

```toml
+++
title = "Guest post"
date = "2026-04-04"
authors = ["Alice"]
+++
```

Zorto generates `/authors/` and `/authors/alice/` automatically.

## Use drafts

Mark a post as a draft to exclude it from production builds:

```toml
+++
title = "Work in progress"
date = "2026-04-04"
draft = true
+++
```

Preview drafts locally:

```bash
zorto preview --drafts
```

Drafts are excluded from `zorto build` output and the Atom feed.

## Add co-located images

For posts with images, use a directory instead of a single file:

```bash
mkdir -p content/posts/photo-gallery
```

Create `content/posts/photo-gallery/index.md`:

```markdown
+++
title = "Photo gallery"
date = "2026-04-04"
+++

![Sunset](sunset.jpg)
```

Place `sunset.jpg` alongside `index.md`. Zorto copies the image to the output directory, preserving the relative path.

## Set the base URL for production

Before deploying, update `base_url` in `config.toml`:

```toml
base_url = "https://myblog.com"
```

This affects permalinks, the Atom feed URL, sitemaps, and Open Graph tags. The dev server overrides it to `localhost` automatically.

## Deploy

Build the site:

```bash
zorto build
```

The output goes to `public/`. Deploy it to any static host. For Netlify, create `netlify.toml`:

```toml
[build]
command = "curl -LsSf https://dkdc.sh/zorto/install.sh | sh && zorto build"
publish = "public"
```

See [Deploy your site](deploy.md) for GitHub Pages, Vercel, and Cloudflare Pages instructions.

## Related guides

- [Add a blog](add-blog.md) — quick reference for blog sections, tags, feeds, and drafts
- [Blog, events, and more](../concepts/blog.md) — how sections and pagination work under the hood
- [Customize your theme](customize-theme.md) — override templates, styles, and shortcodes
- [Customize styles](custom-css.md) — CSS variables, light/dark mode, fonts
- [Set up multiple authors](multiple-authors.md) — taxonomies for author pages
