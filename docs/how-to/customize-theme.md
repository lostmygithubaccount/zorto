Zorto themes are fully overridable. You don't need to fork a theme to customize it.

## Override a single template

Create the same file path in your local `templates/` directory. For example, to override the page template:

```
templates/page.html
```

Your local file takes priority over the theme's version. You can start by copying the theme's template and modifying it.

## Override SCSS variables

Create `sass/_variables.scss` in your project:

```scss
$primary-color: #e74c3c;
$accent-color: #3498db;
$font-family: "Inter", system-ui, sans-serif;
$max-width: 900px;
```

These override the theme's defaults while keeping the rest of the stylesheet intact.

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

## Extend base.html with extra blocks

Most themes define optional blocks you can fill. Create `templates/base.html`:

```html
{% extends "base.html" %}

{% block head_extra %}
  <link rel="stylesheet" href="/custom.css">
  <script defer src="/analytics.js"></script>
{% endblock %}
```

This injects content into the theme's base template without replacing it entirely.
