# Add a 404 page

The built-in themes ship with a `404.html` template that renders automatically. If you want a custom one, create your own `templates/404.html` in your project root.

## Custom 404 template

Create `templates/404.html`:

```html
{% extends "base.html" %}
{% block title %}404 | {{ config.title }}{% endblock title %}
{% block content %}
<div style="text-align: center; padding: 4rem 1rem;">
    <h1>404</h1>
    <p>Page not found.</p>
    <a href="/">Go home</a>
</div>
{% endblock content %}
```

Your project-level template takes precedence over the theme's built-in `404.html`. To verify it looks correct, run `zorto build` and open `public/404.html` in your browser.

> [!NOTE]
> During `zorto preview`, the dev server may not route unknown paths to your 404 page. Test the final result by opening the built file directly or deploying to your hosting provider.

## Static host configuration

Zorto builds `404.html` into `public/404.html`. Most static hosts serve this automatically:

- **GitHub Pages** -- serves `404.html` by default
- **Netlify** -- serves `404.html` by default
- **Cloudflare Pages** -- serves `404.html` by default
- **Vercel** -- serves `404.html` by default

No additional Zorto configuration is needed.

## Related guides

- [Customize your theme](customize-theme.md) — how template overrides work
- [Deploy your site](deploy.md) — hosting setup for each platform
