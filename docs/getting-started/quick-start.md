# Your first site

By the end of this tutorial you will have a working site with:

- A homepage and a standalone About page
- A blog section with two posts, tags, and an Atom feed
- An executable Python code block that generates output at build time
- A production build ready to deploy

Prerequisites: [install Zorto](installation.md) first.

## Create a new site

Run the following to scaffold a project:

```bash
zorto init my-site
cd my-site
```

Zorto creates the directory and populates it with starter files. Let's look at what it generated:

{% tree(caption="Don't worry about memorizing this — you will explore each piece as you work through the tutorial.") %}
my-site/
  config.toml
  content/
    _index.md  [section: homepage]
    posts/
      _index.md  [section: blog]
      hello.md  [page: first post]
  static/
  templates/
    base.html
    index.html
    page.html
    section.html
{% end %}

Here's what each part does:
- `config.toml` — site configuration (title, URL, features)
- `content/` — your Markdown content files. Files named `_index.md` define sections (collections); regular `.md` files are individual pages.
- `static/` — files copied as-is to the output (images, fonts, CSS)
- `templates/` — Tera HTML templates that control how content is rendered into web pages. `base.html` is the outer shell; `page.html` and `section.html` fill in the content.

## Start the preview server

Launch the dev server so you can see changes in real time:

```bash
zorto preview --open
```

Your browser opens `http://127.0.0.1:1111` and you should see a minimal homepage with a link to the "Hello World" post. If the browser doesn't open automatically, navigate to that URL manually. The server watches for file changes and reloads automatically.

Leave this running in the background for the rest of the tutorial.

## Understand the configuration

Open `config.toml` in your editor. It looks like this:

```toml
base_url = "https://example.com"
title = "My Site"
generate_feed = true
```

- `base_url` is where the site will eventually live. For local development it doesn't matter, but you will want to set it before deploying.
- `title` appears in templates and in the Atom feed.
- `generate_feed` tells Zorto to produce an Atom feed at `/atom.xml`.

You will come back to this file later to enable more features. For now, leave it as-is.

## Edit a page and see live reload

Open `content/_index.md`. This is the homepage. It contains frontmatter (the metadata between `+++` markers) and optional body content:

```markdown
+++
title = "Home"
sort_by = "date"
+++
```

Add some text below the closing `+++`:

```markdown
+++
title = "Home"
sort_by = "date"
+++

Welcome to my site! This is built with Zorto.
```

Save the file. Your browser reloads automatically and shows the new text on the homepage.

What you just learned: content files are Markdown with TOML frontmatter. The `sort_by = "date"` on a section page tells Zorto to list child pages by date, newest first.

## Explore the blog section

Zorto generated a blog section for you at `content/posts/`. Open `content/posts/_index.md`:

```markdown
+++
title = "Blog"
sort_by = "date"
+++
```

This is a section index — the underscore in `_index.md` marks it as the section's own page, not a child page. It renders as a listing page at `/posts/` showing all posts in the section. The frontmatter configures how the section behaves — `sort_by = "date"` means newest posts appear first. Any regular `.md` file you place in this directory becomes a blog post.

Now open the sample post at `content/posts/hello.md`:

```markdown
+++
title = "Hello World"
date = "2025-01-01"
description = "My first post"
tags = ["hello"]
+++
Welcome to my new site built with [zorto](https://github.com/dkdc-io/zorto)!
```

Notice the `date` field -- this is what `sort_by = "date"` uses for ordering. The `description` appears in post listings and the Atom feed. And `tags` assigns this post to the "hello" tag.

Visit `http://127.0.0.1:1111/posts/hello/` in your browser to see the rendered post.

## Add a new blog post

Create a file at `content/posts/learning-zorto.md`:

```markdown
+++
title = "Learning Zorto"
date = "2026-03-31"
description = "Notes from working through the Zorto tutorial."
+++

Today I built my first site with Zorto. Here are a few things I noticed:

- The preview server reloads instantly when I save a file.
- Content is just Markdown with TOML frontmatter.
- Sections are directories with an `_index.md` file.
```

Save the file. Your browser reloads and the homepage now lists two posts, with "Learning Zorto" on top because its date is more recent.

What you just learned: adding a page to a section is as simple as dropping a Markdown file into the directory. No configuration changes needed.

## Add tags

Your new post does not have tags yet. Let's add some. Open `content/posts/learning-zorto.md` and add a `tags` field to the frontmatter:

```markdown
+++
title = "Learning Zorto"
date = "2026-03-31"
description = "Notes from working through the Zorto tutorial."
tags = ["tutorial", "getting-started"]
+++
```

Save the file. Zorto automatically generates pages for each tag. Visit `http://127.0.0.1:1111/tags/` to see all tags, and `http://127.0.0.1:1111/tags/tutorial/` to see posts tagged "tutorial".

What you just learned: Zorto includes a `tags` taxonomy by default. You can add custom taxonomies in `config.toml`, but tags work without extra configuration.

## Check the Atom feed

Because `generate_feed = true` is already in your config, Zorto has been generating an Atom feed at `/atom.xml` this whole time. Visit `http://127.0.0.1:1111/atom.xml` in your browser to see it. It includes the title, description, and content of each post.

If you ever want to disable the feed, set `generate_feed = false` in `config.toml`.

## Add a standalone page

Not everything belongs in the blog. Create `content/about.md`:

```markdown
+++
title = "About"
+++

This site is built with [Zorto](https://zorto.dev), an AI-native static site
generator with executable code blocks.
```

Visit `http://127.0.0.1:1111/about/` to see it. Pages outside of a section directory render as standalone pages -- they don't appear in any listing unless you link to them from a template or another page.

## Try an executable code block

Zorto can run code blocks at build time. Blocks tagged with `{python}` or `{bash}` execute during the build and their output is rendered inline.

> [!NOTE]
> This requires Python to be available on your system. If you don't have Python installed, skip to the next section — everything else works without it.

Open `content/about.md` and add a Python code block:

````markdown
+++
title = "About"
+++

This site is built with [Zorto](https://zorto.dev).

```{python}
from datetime import datetime
print(f"Last built: {datetime.now():%Y-%m-%d %H:%M}")
```
````

Save the file. The preview reloads and you see the Python output rendered inline — the current date and time, generated fresh on every build. The code lives next to the prose it supports.

What you just learned: code blocks tagged with `{python}` or `{bash}` run at build time. This is how Zorto keeps documentation, data-driven pages, and CLI references always up to date.

## Build for production

When you are ready to publish, stop the preview server (Ctrl+C) and run:

```bash
zorto build
```

Zorto writes the complete site to `public/`. This directory contains plain HTML, CSS, and static assets -- you can host it anywhere (Netlify, Vercel, Cloudflare Pages, GitHub Pages, or your own server).

{% tree(caption="Plain HTML files you can host anywhere — no server runtime needed.") %}
public/
  atom.xml
  index.html
  llms.txt
  llms-full.txt
  sitemap.xml
  style.css
  about/
    index.html
  posts/
    index.html
    hello/
      index.html
    learning-zorto/
      index.html
  tags/
    index.html
    hello/
      index.html
    getting-started/
      index.html
    tutorial/
      index.html
{% end %}

Notice that Zorto generated `sitemap.xml`, `llms.txt`, and `llms-full.txt` automatically — no configuration needed. The `llms.txt` files make your content accessible to AI systems.

What you just learned: `zorto build` produces a fully static site. `zorto preview` is for development; `zorto build` is for production.

## What you built

In this tutorial you:

1. Scaffolded a new site with `zorto init`
2. Ran the live-reloading preview server
3. Edited the homepage and watched it update instantly
4. Created a blog post and saw it appear in the listing
5. Added tags and explored the auto-generated taxonomy pages
6. Verified the Atom feed
7. Added a standalone page with an executable code block
8. Built the site for production with auto-generated `llms.txt`

That covers the core workflow. Head to [next steps](first-site.md) to explore themes, deployment, and more advanced features.
