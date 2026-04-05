+++
title = "Getting started with Zorto"
date = "2026-04-03"
description = "A quick guide to building your blog with Zorto."
+++

Zorto is a fast, AI-native static site generator. Here's how to make the most of it.

## Project structure

```
your-site/
  config.toml          # Site configuration
  content/             # Markdown content
    _index.md          # Homepage
    posts/             # Blog posts
      _index.md        # Posts section
      hello-world.md   # A post
  templates/           # Tera HTML templates
    base.html          # Base layout
    index.html         # Homepage template
    section.html       # Section listing
    page.html          # Individual page
  static/              # Static assets (copied as-is)
  public/              # Build output (gitignore this)
```

## Useful commands

| Command | Description |
|---------|-------------|
| `zorto build` | Build the site to `public/` |
| `zorto preview --open` | Live preview with hot reload |
| `zorto check` | Validate the site without building |
| `zorto clean` | Remove build output |

## Learn more

Visit [zorto.dev](https://zorto.dev) for full documentation.
