+++
title = "First site"
template = "docs.html"
+++

This guide walks through the structure of a Zorto site and common tasks.

## Project structure

After `zorto init mysite`, you get:

```
mysite/
├── config.toml       # Site configuration
├── content/          # Markdown content
│   └── _index.md     # Homepage
├── templates/        # Tera HTML templates (optional with themes)
├── sass/             # SCSS stylesheets (optional with themes)
└── static/           # Static assets (copied as-is to public/)
```

## Adding a page

Create `content/about.md`:

```markdown
+++
title = "About"
template = "docs.html"
+++

This is my about page.
```

Visit `/about/` in your browser.

## Adding a blog section

Create the section index at `content/posts/_index.md`:

```toml
+++
title = "Blog"
template = "docs.html"
sort_by = "date"
paginate_by = 10
+++
```

Then add a post at `content/posts/hello-world.md`:

```markdown
+++
title = "Hello world"
template = "docs.html"
date = "2026-01-01"
+++

My first blog post.

<!-- more -->

Everything after the marker becomes the full content. Everything before it is the summary.
```

## Adding tags

Add a taxonomy in `config.toml`:

```toml
[[taxonomies]]
name = "tags"
```

Then tag your posts:

```toml
+++
title = "Hello world"
template = "docs.html"
date = "2026-01-01"
[taxonomies]
tags = ["intro", "hello"]
+++
```

## Deploying to Netlify

1. Build your site:

```bash
zorto build
```

2. Add a `netlify.toml` to your repo root:

```toml
[build]
command = "pip install zorto && zorto build"
publish = "public"
```

3. Connect your repository in the Netlify dashboard. Every push triggers a build.

See [deploy to Netlify](@/docs/how-to/deploy-netlify.md) for more details.
