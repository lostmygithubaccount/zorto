# Add a sitemap

Zorto generates `sitemap.xml` automatically. It is enabled by default — no configuration needed.

## Verify

Build your site and confirm the sitemap exists:

```bash
zorto build
ls public/sitemap.xml
```

If you need to disable it for some reason: `generate_sitemap = false` in `config.toml`.

## Add a robots.txt

Create `static/robots.txt` to point crawlers to your sitemap:

```
User-agent: *
Allow: /

Sitemap: https://example.com/sitemap.xml
```

Replace `https://example.com` with your `base_url`. Note: `static/robots.txt` is a plain file, not a template — you must update the URL manually if your `base_url` changes.

## Submit to search engines

Submit `https://example.com/sitemap.xml` (using your `base_url`) to [Google Search Console](https://search.google.com/search-console) and [Bing Webmaster Tools](https://www.bing.com/webmasters). Both re-crawl automatically once submitted.

## Related guides

- [Optimize for SEO](seo.md) — meta tags, Open Graph, canonical URLs
- [Deploy your site](deploy.md) — hosting setup for each platform
