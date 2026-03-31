Zorto ships with three built-in themes. Each supports automatic light/dark mode toggling.

## Built-in themes

| Theme | Description |
|-------|-------------|
| `dkdc` | Violet/cyan color scheme with binary rain animation and logo animation |
| `light` | Clean white default with blue accents |
| `dark` | Clean dark default with cyan accents |

## Setting a theme

In `config.toml`:

```toml
theme = "dkdc"
```

## Light/dark mode

All themes include a toggle button. The user's preference is saved in local storage.

## Overriding templates

To override a theme template, create the same file in your local `templates/` directory. For example, to customize the page layout:

```
templates/page.html
```

This takes priority over the theme's `page.html`. You can extend the theme's base and only override specific blocks:

```html
{% extends "base.html" %}
{% block content %}
  <div class="custom-wrapper">
    {{ page.content | safe }}
  </div>
{% endblock %}
```

## Overriding SCSS variables

Create `sass/_variables.scss` in your project to override theme variables:

```scss
$primary-color: #ff6600;
$font-family: "Inter", sans-serif;
```

## Theme features

Each theme is a Cargo feature. If you're building Zorto from source, you can disable unused themes for a smaller binary:

```bash
cargo install zorto --no-default-features --features theme-light
```
