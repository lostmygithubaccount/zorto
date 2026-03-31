Zorto is configured via `config.toml` in your project root.

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
| `theme` | string | `""` | Theme name (`dkdc`, `light`, `dark`) |
| `compile_sass` | bool | `true` | Compile SCSS to CSS |
| `generate_feed` | bool | `false` | Generate Atom feed |
| `generate_sitemap` | bool | `true` | Generate sitemap.xml |
| `generate_llms_txt` | bool | `true` | Generate llms.txt and llms-full.txt |

### `[markdown]`

Controls markdown rendering. See the [config reference](../reference/config.md) for all fields.

### `[[taxonomies]]`

Define taxonomies like tags and categories. Each entry needs at least a `name` field.

### `[extra]`

Arbitrary key-value data accessible in templates as `config.extra`. Use this for author info, social links, navigation menus, and anything else your templates need.
