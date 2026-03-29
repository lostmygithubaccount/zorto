+++
title = "First site"
template = "docs.html"
slug = "first-site"
date = "2099-01-01"
+++

Now that you have Zorto running, here's how a site is structured and where to go next.

## project structure

```
mysite/
├── config.toml       # site configuration
├── content/          # markdown content
│   └── _index.md     # homepage
├── templates/        # tera HTML templates (optional with themes)
├── sass/             # SCSS stylesheets (optional with themes)
└── static/           # static assets (copied as-is to public/)
```

With `theme = "dkdc"` in `config.toml`, you don't need `templates/` or `sass/`: the theme provides them.

## adding a page

Create `content/about.md`:

```markdown
+++
title = "About"
+++

This is my about page.
```

Visit `/about/` in your browser.

## adding a blog

Create `content/posts/_index.md`:

```toml
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++
```

Add posts with dates. Use `<!-- more -->` to set the summary break. See [add a blog](/docs/how-to/add-blog/) for the full guide.

## building

```bash
zorto build
```

Output goes to `public/`. Host it anywhere.

## learn more

- [content model](/docs/concepts/content-model/): sections, pages, frontmatter
- [templates](/docs/concepts/templates/): Tera engine, context variables
- [themes](/docs/concepts/themes/): built-in themes, overrides
- [shortcodes](/docs/concepts/shortcodes/): note, figure, tabs, and more
- [callouts](/docs/concepts/callouts/): GitHub-style alerts
- [configuration](/docs/concepts/configuration/): all config.toml options
