# Manage assets

Serve images, fonts, and other static files alongside your Zorto site.

Zorto provides two mechanisms for assets: the `static/` directory for site-wide files, and co-located assets for page-specific files.

## Static files

Place site-wide assets in the `static/` directory at your project root. Everything inside is copied verbatim to the output directory during build:

{% tree(caption="Files in static/ are copied to the root of your output directory.") %}
my-site/
  static/
    favicon.ico  [→ /favicon.ico]
    images/
      logo.png  [→ /images/logo.png]
    fonts/
      custom.woff2  [→ /fonts/custom.woff2]
    robots.txt  [→ /robots.txt]
{% end %}

Reference them with absolute paths in templates or markdown:

```html
<link rel="icon" href="/favicon.ico">
<img src="/zorto-mark-transparent.png" alt="Logo">
```

```markdown
![Logo](/zorto-mark-transparent.png)
```

The directory structure is preserved. Nested directories work as expected.

## Co-located assets

Place images and files next to a page's markdown by using a directory with an `index.md` file:

{% tree(caption="Assets live alongside the page that uses them.") %}
content/posts/my-post/
  index.md  [page → /posts/my-post/]
  photo.jpg  [→ /posts/my-post/photo.jpg]
  diagram.svg  [→ /posts/my-post/diagram.svg]
  data.json  [→ /posts/my-post/data.json]
{% end %}

Reference co-located assets with relative paths in your markdown:

```markdown
![A photo](photo.jpg)
![A diagram](diagram.svg)
```

Any non-markdown file inside a content directory is treated as a co-located asset. During the build, Zorto copies these files to the page's output directory, preserving the relative path relationship.

### When to use each approach

| Approach | Use for | Reference with |
|----------|---------|----------------|
| `static/` | Favicons, global images, fonts, `robots.txt`, `_headers` | Absolute paths (`/zorto-mark-transparent.png`) |
| Co-located | Page-specific images, diagrams, downloads | Relative paths (`photo.jpg`) |

## Images

For images in markdown, use standard markdown syntax:

```markdown
![Alt text](photo.jpg)
![Alt text](/zorto-logo-dark.png)
```

> [!TIP]
> Always include alt text for accessibility. Keep image file sizes small — Zorto does not optimize images at build time. Use tools like `imagemagick`, `sharp`, or `squoosh` before adding images to your project.

Supported formats include `.jpg`, `.png`, `.svg`, `.gif`, `.webp`, and any other format browsers can display.

## Fonts

Host fonts locally by placing them in `static/fonts/`:

{% tree() %}
static/fonts/
  inter-regular.woff2
  inter-bold.woff2
{% end %}

Load them in your stylesheet:

```scss
// sass/custom.scss
@font-face {
  font-family: "Inter";
  src: url("/fonts/inter-regular.woff2") format("woff2");
  font-weight: 400;
  font-display: swap;
}

@font-face {
  font-family: "Inter";
  src: url("/fonts/inter-bold.woff2") format("woff2");
  font-weight: 700;
  font-display: swap;
}

body {
  font-family: "Inter", sans-serif;
}
```

Use `font-display: swap` to prevent invisible text while fonts load.

## Caching and cache busting

Static files are served with whatever caching policy your hosting provider applies. Zorto does not add cache-busting hashes to filenames.

For aggressive caching strategies, use your hosting provider's headers configuration. For example, with Netlify:

```
# static/_headers
/fonts/*
  Cache-Control: public, max-age=31536000, immutable
/images/*
  Cache-Control: public, max-age=86400
```

SCSS-compiled CSS files are regenerated on every build, so browsers fetch the latest version when the content changes and the hosting provider's cache expires.

## Related guides

- [Content model](../concepts/content-model.md) — co-located assets and page structure in depth
- [Customize styles](custom-css.md) — loading fonts and overriding styles
- [Deploy your site](deploy.md) — hosting setup and cache headers
- [Optimize for SEO](seo.md) — favicons, Open Graph images
