# Configuration

Zorto is configured via `config.toml` in your project root.

{{ layers(items="Identity:Who is this site?:base_url, title|Build behavior:What outputs to produce?:feeds, sitemap|Content processing:How to parse and organize content?:markdown, taxonomies|Theme and custom data:How should the site look?:theme, extra", caption="Four conceptual layers, one file. Everything from identity to appearance in config.toml.") }}

## Minimal example

```toml
base_url = "https://example.com"
title = "My site"
```

## Full example

```toml
base_url = "https://example.com"
title = "My site"
description = "A site built with Zorto"
theme = "dkdc"

compile_sass = true
generate_feed = true
generate_sitemap = true
generate_llms_txt = true
generate_md_files = true

[markdown]
highlight_code = true
insert_anchor_links = "right"
external_links_target_blank = true
external_links_no_follow = true
external_links_no_referrer = true
smart_punctuation = true

[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"

[extra]
author = "Your Name"
# Any custom data: accessible as config.extra in templates
```

## Key sections

### Top-level settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_url` | string | *required* | Full URL of your site |
| `title` | string | `""` | Site title |
| `description` | string | `""` | Site description |
| `theme` | string | `""` | Theme name (`zorto`, `dkdc`, `light`, `dark`) |
| `compile_sass` | bool | `true` | Compile SCSS to CSS |
| `generate_feed` | bool | `false` | Generate Atom feed |
| `generate_sitemap` | bool | `true` | Generate sitemap.xml |
| `generate_llms_txt` | bool | `true` | Generate llms.txt and llms-full.txt |
| `generate_md_files` | bool | `false` | Generate .md versions of every page alongside HTML |

### `[markdown]`

Controls how Markdown is rendered to HTML. The most commonly used options:

- `highlight_code` — syntax highlighting for fenced code blocks
- `insert_anchor_links` — add `#` links to headings (`"right"`, `"left"`, or `"none"`)
- `external_links_target_blank` — open external links in a new tab
- `smart_punctuation` — convert `"quotes"` to "quotes" and `--` to —

See the [config reference](../reference/config.md) for all fields.

### `[[taxonomies]]`

Define taxonomies like tags and categories. Each entry creates listing pages automatically:

```toml
[[taxonomies]]
name = "tags"
```

This generates `/tags/` (all tags) and `/tags/<term>/` (pages with that tag). Add as many taxonomies as you need — tags, categories, authors, etc.

### `[extra]`

A free-form table for any custom data your templates need. Zorto passes it through as `config.extra` without interpreting it:

```toml
[extra]
author = "Your Name"
github = "https://github.com/you"
menu_items = [
  { name = "Docs", url = "/docs/" },
  { name = "Blog", url = "/posts/" },
]
```

Access in templates: `{{ config.extra.author }}`, `{% for item in config.extra.menu_items %}`.

## Further reading

- [Configuration reference](../reference/config.md) — complete field list with types and defaults
- [Themes](themes.md) — how the `theme` setting works
- [Content model](content-model.md) — how frontmatter relates to config
- [How to customize your theme](../how-to/customize-theme.md) — override styles and templates
- [How to deploy](../how-to/deploy.md) — build commands for each hosting provider
