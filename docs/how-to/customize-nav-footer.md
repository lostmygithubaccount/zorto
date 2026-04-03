# Customize navigation and footer

The built-in themes render navigation and footer from `config.toml` values. No template overrides needed for common cases.

## Navigation menu

Define `menu_items` under `[extra]` to populate the left-side nav links:

```toml
[extra]
menu_items = [
    { name = "Docs", url = "/docs/" },
    { name = "Blog", url = "/blog/" },
    { name = "GitHub", url = "https://github.com/you/repo", external = true },
]
```

Setting `external = true` opens the link in a new tab and adds an external-link icon.

## Right-side menu items

Use `menu_items_right` for call-to-action buttons on the right side of the navbar:

```toml
[extra]
menu_items_right = [
    { name = "Get started", url = "/docs/getting-started/" },
]
```

## Logo

```toml
[extra]
logo_text = "My Site"       # text next to the logo (defaults to config.title)
logo_tld = ".dev"           # optional TLD suffix rendered in accent color
logo_image = "/logo.svg"    # optional image (place in static/)
```

## Social links

```toml
[extra]
social_links = [
    { name = "GitHub", url = "https://github.com/you", icon = "github" },
]
```

Built-in icons: `github`, `linkedin`. To add other icons, override `base.html` and add SVG icons for your additional social links in the navbar section.

## Footer

The footer renders `config.extra.copyright_html` if set, otherwise falls back to `config.extra.author` with the current year:

```toml
[extra]
copyright_html = "Built with <a href=\"https://zorto.dev\">Zorto</a>"
# or simply:
author = "Your Name"
```

## [Template blocks](../concepts/glossary.md#template-block) to override

If you need deeper customization, override these blocks from `base.html` in your own templates (see [how to customize your theme](customize-theme.md)):

| Block | Purpose |
|-------|---------|
| `title` | Page `<title>` |
| `extra_head` | Inject CSS/JS into `<head>` |
| `content` | Main page content |
| `open_graph` | Open Graph meta tags |
| `extra_body` | Inject scripts before `</body>` |

See [Customize your theme](customize-theme.md) for examples of overriding these blocks.
