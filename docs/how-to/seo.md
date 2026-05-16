# Optimize for SEO and discoverability

Configure your Zorto site for search engine visibility and AI agent discovery.

Three layers:

1. **Content** — `title` and `description` in every page's frontmatter (you write this)
2. **Meta tags** — Open Graph, Twitter Cards, canonical URLs (built-in themes handle this)
3. **Discovery** — sitemap.xml, robots.txt, llms.txt, search index (Zorto generates these automatically)

> [!TIP]
> If you use a built-in theme (`zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`, etc.), Open Graph tags, canonical URLs, and Twitter Card tags are already included. The sections below show how to set them up manually if you use custom templates.

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

For pages with a description, add the `og:description` tag:

<pre><code>&#123;%- if page.description %&#125;
  &lt;meta property="og:description" content="&#123;&#123; page.description &#125;&#125;" /&gt;
&#123;%- endif %&#125;</code></pre>

## Canonical URLs

Zorto generates absolute [canonical URLs](../concepts/glossary.md#canonical-urls) using `base_url` from `config.toml`. Set it to your production domain:

```toml
base_url = "https://example.com"
```

Add a canonical link in your base template (using the same `og_url` variable from the Open Graph section above):

<pre><code>&lt;link rel="canonical" href="&#123;&#123; og_url &#125;&#125;" /&gt;</code></pre>

## Sitemap

Zorto generates `sitemap.xml` automatically. It is enabled by default — no configuration needed. The sitemap includes all non-draft pages and sections with their permalinks.

Disable it if needed:

```toml
generate_sitemap = false
```

Submit your sitemap URL to [Google Search Console](https://search.google.com/search-console) and [Bing Webmaster Tools](https://www.bing.com/webmasters) for faster indexing.

## robots.txt

Create `static/robots.txt` to control crawler access and point to your sitemap:

```
User-agent: *
Allow: /

Sitemap: https://example.com/sitemap.xml
```

Replace `https://example.com` with your `base_url`. This is a plain file in `static/`, not a template — update the URL manually if your `base_url` changes.

## llms.txt

Zorto generates [`llms.txt`](../concepts/glossary.md#llms-txt) and `llms-full.txt` by default. These files help AI agents understand your site's content and structure. No configuration required.

Two files are generated:

| File | Contents | Use case |
|------|----------|----------|
| `/llms.txt` | Structured index with links to every page, organized by section | Agent reads one URL to understand the site |
| `/llms-full.txt` | Full raw markdown content of every page in a single file | Agent reads all content without crawling |

The `llms.txt` file includes the site title as an H1, the description as a blockquote, then each section as an H2 with its pages listed as links. Pages with descriptions include them inline.

Disable if needed:

```toml
generate_llms_txt = false
```

When `generate_md_files = true` is also set, `llms.txt` links point to the `.md` versions of each page instead of the HTML versions.

## Built-in search

Zorto can generate a SQLite search database for client-side search:

```toml
generate_search = true
```

This generates a `search.db` file in your output directory containing every page and section. The built-in themes include a search UI that queries this database using sql.js (SQLite compiled to WebAssembly).

The search index supports:

- Case-insensitive matching
- Ranked results (title matches score higher than body matches)
- Prefix matching for autocomplete-style queries

Search is disabled by default because the database file adds size to the output. Enable it for documentation sites, blogs, or any site where visitors need to find content quickly.

## Add custom headers for caching

Your [static hosting provider](../concepts/glossary.md#static-hosting) serves files through a [CDN](../concepts/glossary.md#cdn) with default caching. For custom cache-control policies, create a `static/_headers` file (Netlify) or configure headers in your provider's dashboard. See the [deploy guide](deploy.md) for platform-specific details.

## Related guides

- [Deploy your site](deploy.md) — hosting setup and headers
- [Set up a custom domain](custom-domain.md) — HTTPS and `base_url`
- [Add a sitemap](add-sitemap.md) — sitemap.xml submission details
- [AI-native](../concepts/ai-native.md) — llms.txt and the consumption model
- [Build optimization](build-optimization.md) — disabling generators for faster builds
