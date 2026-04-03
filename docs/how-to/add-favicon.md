# Add a favicon

Place your favicon file in the `static/` directory at your project root. Zorto copies everything in `static/` to `public/` at build time, so `static/favicon.svg` becomes `/favicon.svg`.

## Configure in config.toml

```toml
[extra]
favicon = "/favicon.svg"
favicon_mimetype = "image/svg+xml"
```

The built-in themes read `config.extra.favicon` and `config.extra.favicon_mimetype` in the `<head>` of `base.html`:

```html
<link rel="icon" type="{{ config.extra.favicon_mimetype | default(value="image/png") }}" href="{{ config.extra.favicon }}">
```

If `config.extra.favicon` is not set, no `<link rel="icon">` tag is rendered.

## Supported formats

Use any format your target browsers support. Common choices:

| File | Mimetype |
|------|----------|
| `favicon.png` | `image/png` |
| `favicon.ico` | `image/x-icon` |
| `favicon.svg` | `image/svg+xml` |

SVG is the most flexible option — it scales to any size and supports dark mode via CSS media queries.

## Related guides

- [Customize your theme](customize-theme.md) — override templates and styles
- [Optimize for SEO](seo.md) — Open Graph images and other meta tags
