# Customize your theme

Override templates, styles, and shortcodes without forking the theme.

{{ cascade(items="Fallback:Theme defaults — bundled templates, styles, shortcodes:default|Priority:Your project — templates/, sass/, templates/shortcodes/:wins", caption="Local files always take priority over theme defaults.") }}

## Override a template

Create the same file path in your local `templates/` directory:

```
templates/page.html
```

Your local file takes priority over the theme's version. Start by copying the theme's template and modifying it — or write one from scratch that extends `base.html`.

## Override styles

Creating `sass/style.scss` in your project replaces the theme's stylesheet entirely (local files overlay theme files by filename).

For lighter customization, create `sass/custom.scss` and load it via the `extra_head` [template block](../concepts/glossary.md#template-block) (see below). Zorto compiles [SCSS](../concepts/glossary.md#scss) to CSS at build time — `sass/custom.scss` becomes `/custom.css` in the output. All built-in themes use CSS custom properties you can override:

```scss
// sass/custom.scss
:root {
  --accent: #e74c3c;
  --background: #fafafa;
  --color: #1e293b;
  --max-width: 900px;
}
```

All themes support `--accent`, `--background`, `--background-raised`, `--color`, `--color-muted`, `--border-color`, and `--code-bg`.

## Add custom shortcodes

Create templates in `templates/shortcodes/`:

```html
{# templates/shortcodes/callout.html #}
<div class="callout callout-{{ type | default(value="info") }}">
  {{ body }}
</div>
```

Use in markdown:

<pre><code>&#123;% callout(type="warning") %&#125;
This is a custom callout.
&#123;% end %&#125;</code></pre>

## Inject into the base template

All built-in themes define an `extra_head` [template block](../concepts/glossary.md#template-block) you can fill without replacing the entire layout. Create `templates/base.html` in your project — it extends the theme's `base.html` (Zorto resolves the extends to the theme version, not to itself):

```html
{% extends "base.html" %}

{% block extra_head %}
  <link rel="stylesheet" href="/custom.css">
  <script defer src="/analytics.js"></script>
{% endblock %}
```

## Switch themes

Change the `theme` field in `config.toml`:

```toml
theme = "dark"
```

Available themes: `zorto`, `dkdc`, `light`, `dark`. All include light/dark mode toggling.

## Related guides

- [Themes](../concepts/themes.md) — how the theme system works
- [Customize navigation and footer](customize-nav-footer.md) — menus, logo, social links via config
- [Templates](../concepts/templates.md) — the Tera template engine and block inheritance
