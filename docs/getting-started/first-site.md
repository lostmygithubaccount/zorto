Now that you have Zorto running, here's how a site is structured and where to go next.

## Project structure

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

## Adding a page

Create `content/about.md`:

```markdown
+++
title = "About"
+++

This is my about page.
```

Visit `/about/` in your browser.

## Adding a blog

Create `content/posts/_index.md`:

```toml
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++
```

Add posts with dates. Use `<!-- more -->` to set the summary break. See [add a blog](../how-to/add-blog.md) for the full guide.

## Building

```bash
zorto build
```

Output goes to `public/`. Host it anywhere.

## Learn more

- [Content model](../concepts/content-model.md): sections, pages, frontmatter
- [Templates](../concepts/templates.md): Tera engine, context variables
- [Themes](../concepts/themes.md): built-in themes, overrides
- [Shortcodes](../concepts/shortcodes.md): note, figure, tabs, and more
- [Callouts](../concepts/callouts.md): GitHub-style alerts
- [Configuration](../concepts/configuration.md): all config.toml options
