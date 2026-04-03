# Optimize for SEO

Configure your Zorto site for search engine visibility.

Three layers:

1. **Content** — `title` and `description` in every page's frontmatter (you write this)
2. **Meta tags** — Open Graph, Twitter Cards, canonical URLs (built-in themes handle this)
3. **Discovery** — sitemap.xml, robots.txt, llms.txt (Zorto generates these automatically)

> [!TIP]
> If you use a built-in theme (`zorto`, `dkdc`, `light`, `dark`), Open Graph tags, canonical URLs, and Twitter Card tags are already included. The sections below show how to set them up manually if you use custom templates.

## Set title and description on every page

Search engines use `title` and `description` directly in results.

```markdown
+++
title = "Getting started with Zorto"
description = "Install Zorto and build your first static site in under five minutes."
date = "2025-01-15"
+++
```

Keep titles under 60 characters and descriptions under 160 characters.

## Add Open Graph meta tags

Add [Open Graph](../concepts/glossary.md#open-graph) tags in your `templates/base.html` `<head>`:

The built-in themes handle OG tags using `page.permalink` and `section.permalink` with conditional logic. Here's the pattern for custom templates:

<pre><code>&#123;%- if page %&#125;
  &#123;%- set og_url = page.permalink %&#125;
  &#123;%- set og_title = page.title %&#125;
&#123;%- elif section %&#125;
  &#123;%- set og_url = section.permalink %&#125;
  &#123;%- set og_title = section.title %&#125;
&#123;%- endif %&#125;
&lt;meta property="og:title" content="&#123;&#123; og_title &#125;&#125;" /&gt;
&lt;meta property="og:url" content="&#123;&#123; og_url &#125;&#125;" /&gt;
&lt;meta property="og:type" content="article" /&gt;
&lt;meta name="twitter:card" content="summary_large_image" /&gt;</code></pre>

## Canonical URLs

Zorto generates absolute [canonical URLs](../concepts/glossary.md#canonical-urls) using `base_url` from `config.toml`. Set it to your production domain:

```toml
base_url = "https://example.com"
```

Add a canonical link in your base template (using the same `og_url` variable from the Open Graph section above):

<pre><code>&lt;link rel="canonical" href="&#123;&#123; og_url &#125;&#125;" /&gt;</code></pre>

## llms.txt

Zorto generates [`llms.txt`](../concepts/glossary.md#llms-txt) and `llms-full.txt` by default. These files help AI agents understand your site's content and structure. No configuration required.

## Add custom headers for caching

Your [static hosting provider](../concepts/glossary.md#static-hosting) serves files through a [CDN](../concepts/glossary.md#cdn) with default caching. For custom cache-control policies, create a `static/_headers` file (Netlify) or configure headers in your provider's dashboard. See the [deploy guide](deploy.md) for platform-specific details.

## Related guides

- [Deploy your site](deploy.md) — hosting setup and headers
- [Set up a custom domain](custom-domain.md) — HTTPS and `base_url`
- [Add a sitemap](add-sitemap.md) — sitemap.xml and robots.txt
- [AI-native](../concepts/ai-native.md) — llms.txt and the consumption model
