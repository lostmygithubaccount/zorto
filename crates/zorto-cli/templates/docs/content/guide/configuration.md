+++
title = "Configuration"
+++

## config.toml

The `config.toml` file at the root of your site controls all settings.

### Required fields

| Field | Description |
|-------|-------------|
| `base_url` | The URL where the site will be deployed |
| `title` | The site title |

### Optional fields

| Field | Default | Description |
|-------|---------|-------------|
| `theme` | none | Built-in theme name |
| `generate_feed` | `false` | Generate an Atom feed |
| `compile_sass` | `false` | Compile SCSS files |

## Themes

Zorto ships with 16 built-in themes. Set the `theme` field to use one:

```toml
theme = "ocean"
```

Available themes: `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`, `midnight`, `sunset`, `mint`, `plum`, `sand`, `arctic`, `lime`, `charcoal`.
