# Glossary

Definitions for web and SSG terms used throughout the Zorto docs. Skip to the section you need:

- [Web fundamentals](#web-fundamentals) — domains, DNS, HTTPS, hosting
- [Content and structure](#content-and-structure) — markdown, frontmatter, sections, taxonomies
- [Build and deploy](#build-and-deploy) — templates, SCSS, executable code, CI/CD
- [SEO and discovery](#seo-and-discovery) — sitemap, Open Graph, llms.txt

## Web fundamentals

### URLs and domain names

A URL (Uniform Resource Locator) is the full address of any page on the web. Every URL has the same anatomy:

`https://` `app.` `zorto` `.dev` `/docs/concepts/`

| Part | Name | What it is |
|------|------|-----------|
| `https://` | Protocol | Secure connection |
| `app.` | Subdomain | Optional prefix pointing to a different server |
| `zorto` | Second-level domain | The name you buy and own |
| `.dev` | Top-level domain (TLD) | The extension (.com, .dev, .org, etc.) |
| `/docs/concepts/` | Path | Which page on the site |

When you set `base_url = "https://zorto.dev"` in Zorto's `config.toml`, you're setting the protocol, domain, and TLD. Zorto generates the paths from your content file structure.

### Domain name

Your website's identity on the internet. A domain name has two parts you choose: the **second-level domain** (the name itself, like `zorto`) and the **top-level domain** (the extension, like `.dev`). Together they form the address people type in their browser: `zorto.dev`.

You buy a domain from a registrar (Namecheap, Cloudflare, Squarespace, etc.), typically for $10–20/year. Once you own it, nobody else in the world can have the same one. Think of it like property — you own the address.

{{ flow(steps="Register:Buy from a registrar ($10-20/yr)|Configure:Point DNS at your host|Live:Visitors type your domain", caption="From registration to a live website.") }}

### Top-level domains (TLDs)

The last part of a domain — `.com`, `.dev`, `.org`, etc. Different TLDs have different associations:

{{ compare(left_title="The classics", left=".com — most recognized worldwide. .org — traditionally organizations. .net — traditionally networks.", right_title="Newer options", right=".dev — developers and tech. .io — startups and tech. .ai — AI companies. .shop, .blog, .design — industry-specific.") }}

`.com` is the most common and widely recognized. Newer TLDs like `.dev` and `.io` are popular in tech. The right choice depends on your audience and brand.

### Subdomains

A prefix before your domain name, separated by a dot. `app.zorto.dev` is a subdomain of `zorto.dev`. Each subdomain can point to a completely different server:

| Subdomain | Points to |
|-----------|-----------|
| `app.zorto.dev` | A VPS at 164.90.252.58 |
| `www.zorto.dev` | Redirects to `zorto.dev` |
| `zorto.dev` | Netlify |

Each subdomain can point to a completely different server.

The old `www.` prefix is actually a subdomain too — it was common in the early web but most modern sites redirect it to the bare domain.

### DNS

The Domain Name System — the internet's phone book. When someone types your domain, DNS translates it to the IP address where your site actually lives:

{{ flow(steps="Type:zorto.dev|Lookup:DNS finds the IP|Connect:Browser reaches the server|Load:Page appears", caption="This happens in milliseconds, every time anyone visits any website.") }}

When you deploy a Zorto site, you configure DNS records to point your domain at your hosting provider. There are two main types:

### CNAME record

A DNS record that maps one domain name to another — an alias. Instead of saying "my site lives at IP 75.2.60.5," a CNAME says "my site is the same as `my-site.netlify.app`."

{{ compare(left_title="CNAME record", left="www.example.com → my-site.netlify.app. Points to another domain name. Used for subdomains (www, app, etc.).", right_title="A record", right="example.com → 75.2.60.5. Points directly to an IP address. Used for the root domain (no www).") }}

Your hosting provider tells you which to create. CNAME for subdomains, A records for root domains. See the [custom domain guide](../how-to/custom-domain.md) for exact records per provider.

### A record

A DNS record that maps a domain name directly to an IP address. Used for the root domain (`example.com` without `www`) because CNAME records technically can't be used there.

When you deploy, your hosting provider gives you one or more IP addresses. Create A records pointing your root domain to those IPs. The [deploy guide](../how-to/deploy.md) shows the exact IPs for Netlify, GitHub Pages, Vercel, and Cloudflare.

### HTTPS / SSL

HTTPS encrypts the connection between your visitor's browser and your server. Look for the padlock icon in your browser's address bar right now — that's HTTPS in action.

{{ compare(left_title="HTTPS (secure)", left="Encrypted connection. Padlock icon in browser. Required by modern browsers and search engines.", right_title="HTTP (insecure)", right="Unencrypted. Browser shows a 'Not secure' warning. Bad for trust and SEO.", left_style="green", right_style="muted") }}

Every static hosting provider provisions HTTPS certificates automatically for your domain. No configuration needed. The underlying technology is TLS (Transport Layer Security), sometimes still called SSL (its predecessor's name).

### IP address

A numeric address that identifies a device on the internet. You encounter them when setting up DNS A records:

{{ compare(left_title="IPv4", left="75.2.60.5 — four numbers separated by dots. The format you will see most often when configuring DNS.", right_title="IPv6", right="2604:a880:4:1d0::5a:c000 — longer, with hexadecimal and colons. Created because the world ran out of IPv4 addresses.") }}

Domain names exist so humans don't have to remember these numbers. DNS translates between the two automatically.

### Static site

A website made entirely of pre-built files — HTML, CSS, JavaScript, images — served directly to browsers. The server just sends files; it doesn't run code, query a database, or generate pages on the fly.

{{ compare(left_title="Static site", left="Pre-built files. Served from CDN. No server-side code. Zorto is a static site generator.", right_title="Dynamic site", right="Generated per request. Requires a running server and database. WordPress, Rails, Django.") }}

### Static hosting

A service optimized for serving pre-built static files to visitors worldwide:

| Provider | Highlights |
|----------|-----------|
| [Netlify](https://netlify.com) | Auto-deploy from Git, instant rollbacks |
| [Vercel](https://vercel.com) | Fast edge network, preview deployments |
| [Cloudflare Pages](https://pages.cloudflare.com) | Global CDN, unlimited bandwidth |
| [GitHub Pages](https://pages.github.com) | Free for public repos |

All handle HTTPS, CDN, and continuous deployment automatically. See [how to deploy](../how-to/deploy.md).

Push to your Git repo and your site updates within seconds.

### CDN

A Content Delivery Network — a global network of servers that cache copies of your site close to your visitors:

{{ flow(steps="Deploy:Your site in Virginia|Cache:CDN copies files to 200+ locations|Serve:Visitor in Tokyo gets the nearby copy", caption="Static hosting providers include CDN automatically.") }}

### 404 page

The error page visitors see when they request a URL that doesn't exist. "404" is the HTTP status code for "not found." Zorto generates `public/404.html` from your `templates/404.html` — customize it with a friendly message and a link back to your homepage. Static hosting providers serve it automatically.

## Content and structure

### Markdown

A text format that uses simple symbols for formatting. You're probably already familiar with it from GitHub, Slack, or Discord:

- `**bold**` → **bold**
- `*italic*` → *italic*
- `# Heading` → a heading
- `[link text](url)` → a clickable link
- `` `code` `` → inline code

Zorto uses Markdown as its content format. Your content files are `.md` files with TOML metadata at the top. See [content model](content-model.md) for how Zorto organizes Markdown files into sections and pages.

### TOML

A configuration file format designed for humans to read and write. Zorto uses TOML for `config.toml` and for frontmatter in content files. It looks like this:

```toml
title = "My site"
base_url = "https://example.com"

[markdown]
highlight_code = true
```

Zorto uses TOML rather than YAML or JSON. TOML supports comments, doesn't rely on indentation for structure, and handles nested tables cleanly. See [configuration](configuration.md) for how `config.toml` is structured.

### Frontmatter

The metadata block at the top of every Markdown content file, enclosed between `+++` markers:

```markdown
+++
title = "My page"
date = "2026-01-15"
tags = ["rust", "tutorial"]
+++

Your content starts here.
```

Frontmatter controls how Zorto processes the page — its title, date, template, URL slug, draft status, and any custom data you need in templates. The format is TOML. See [content model](content-model.md) for the complete list of frontmatter fields.

### Section

A directory inside `content/` that contains an `_index.md` file. Sections are how Zorto organizes content into groups — a blog, a docs area, a portfolio. Each section:

- Lists its child pages at its URL (e.g., `/posts/` shows all blog posts)
- Can sort pages by date or title
- Can paginate (show 10 per page)
- Can use a custom template

Without an `_index.md`, a directory is just a namespace — it doesn't generate a listing page. See [content model](content-model.md) for the full explanation and [how to organize content](../how-to/organize-content.md) for nested sections.

### Taxonomy

A way to classify content across sections. Tags and categories are the most common taxonomies. When you add `tags = ["rust", "tutorial"]` to a page's frontmatter, Zorto automatically generates:

- `/tags/` — a page listing all tags
- `/tags/rust/` — a page listing all pages tagged "rust"
- `/tags/tutorial/` — a page listing all pages tagged "tutorial"

You can define any taxonomy — not just tags. Authors, categories, topics, or anything else that groups pages by shared attributes. See [how to add a blog](../how-to/add-blog.md) for tags setup and [how to set up multiple authors](../how-to/multiple-authors.md) for custom taxonomies.

### Atom feed

An XML file that lets readers subscribe to your site using feed readers (like Feedly or NetNewsWire). When you publish new content, subscribers see it automatically without visiting your site.

Zorto generates an Atom feed at `/atom.xml` when you set `generate_feed = true` in config. Pages need a `date` in frontmatter to appear in the feed. If you're searching for "RSS" — Atom and RSS serve the same purpose; Atom is what Zorto generates. See [how to add a blog](../how-to/add-blog.md) for setup.

### Shortcode

A named, reusable content component you embed in Markdown instead of writing raw HTML. For example, instead of writing `<figure>` tags manually, you write:

<pre><code>&#123;&#123; figure(src="/photo.jpg", caption="My photo") &#125;&#125;</code></pre>

Zorto replaces this with properly structured HTML at build time. There are two types: **inline** shortcodes (single line, double curly braces) and **body** shortcodes (wrap content, curly-percent delimiters). Zorto includes 15 built-in shortcodes for tabs, figures, diagrams, embeds, and more. See [shortcodes](shortcodes.md) for the full list.

### Co-located assets

Images, scripts, and other files placed in the same directory as a content page. Instead of managing a separate `static/images/` folder, you keep assets next to the content that uses them:

{% tree(caption="Assets live next to content. Reference with relative paths.") %}
content/posts/my-post/
  index.md  [page]
  photo.jpg
  chart.svg
{% end %}

Reference them in Markdown with relative paths: `![Photo](photo.jpg)`. Zorto copies them to the output alongside the HTML. See [content model](content-model.md) for details.

### Pagination

Splitting a long list of pages across multiple pages — showing 10 posts per page instead of all 200 at once. Controlled by the `paginate_by` field in a section's frontmatter:

```toml
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++
```

Zorto generates `/posts/`, `/posts/page/2/`, `/posts/page/3/`, etc. Templates receive a `paginator` variable for building navigation. See [blog](blog.md) for the full pattern.

### Draft

A page with `draft = true` in its frontmatter. Drafts are excluded from production builds (`zorto build`) but can be previewed locally with `zorto preview --drafts`. Use drafts for work-in-progress content you're not ready to publish.

### Theme

A bundled set of templates and styles that controls your site's appearance. Zorto ships four built-in themes (`zorto`, `dkdc`, `light`, `dark`) embedded directly in the binary. Set one in config with `theme = "zorto"` and you get a complete, working site without creating any templates or stylesheets. Override any template or style file locally to customize. See [themes](themes.md).

### Slug

The URL-friendly version of a page's filename. `content/posts/my-first-post.md` becomes `/posts/my-first-post/` in the URL. Override it with the `slug` frontmatter field if you want a different URL than what the filename suggests.

### Base URL

The root URL of your site, set as `base_url` in `config.toml`. Every generated URL — feeds, sitemaps, canonical links, Open Graph tags — is built from this. Set it to your production domain before deploying:

```toml
base_url = "https://example.com"
```

## Build and deploy

### Tera

The template engine Zorto uses for HTML rendering. Templates are HTML files with special syntax for dynamic content:

- `{{ page.title }}` — insert a value
- `{% for page in section.pages %}` — loop over pages
- `{% block content %}{% endblock %}` — define overridable regions

If you've used Jinja2 (Python), Twig (PHP), or Liquid (Jekyll), Tera will feel familiar. See [templates](templates.md).

### PyO3

The technology that lets Zorto run Python code blocks. When you write a `{python}` code block in your Markdown, PyO3 runs it inside the Zorto process — no separate Python shell needed. You never interact with PyO3 directly. See [executable code blocks](executable-code.md) and the [how-to guide](../how-to/executable-code-blocks.md).

### Build command

The command that generates your final site files. For Zorto, it's `zorto build`, which reads your content, templates, and config, then writes the complete static site to `public/`. In CI/CD, the build command also typically includes installing Zorto first:

```bash
curl -LsSf https://dkdc.sh/zorto | sh && zorto build
```

### SCSS

A superset of CSS that adds features plain CSS lacks — variables, nesting, and reusable mixins. Instead of repeating the same color value in 20 places, you define it once:

```scss
$accent: #3b82f6;
.button { background: $accent; }
.link { color: $accent; }
```

Zorto compiles `.scss` files from your `sass/` directory into plain CSS at build time. `sass/style.scss` becomes `public/style.css`. See [how to customize your theme](../how-to/customize-theme.md) for overriding styles.

### Template block

A named region in a template that child templates can override. Here's how it works in practice:

```html
{# base.html defines the skeleton with named blocks #}
<nav>...</nav>
{% block content %}{% endblock %}
<footer>...</footer>
```

```html
{# page.html fills in just the content block #}
{% extends "base.html" %}
{% block content %}
  <h1>{{ page.title }}</h1>
{% endblock %}
```

Navigation and footer come from the base. Each page only writes the part that differs. This is how all Zorto themes work — you can override individual blocks without rewriting the entire layout.

### Executable code blocks

Fenced code blocks tagged with `{python}` or `{bash}` that Zorto runs at build time. The output is captured and rendered inline in the HTML:

````markdown
```{python}
print(f"Built on: {datetime.now():%Y-%m-%d}")
```
````

This keeps documentation always up to date — output is regenerated on every build, so it stays in sync with the code. Use it for CLI help text, data tables, generated charts, or anything that should match the current state of your code. See [executable code blocks](executable-code.md).

### Continuous deployment

A workflow where pushing code to a Git repository automatically triggers a build and deploy. All major static hosting providers support this. The typical flow:

{{ flow(steps="Push:Commit to main|Build:Host runs zorto build|Deploy:New files go live|Done:Site updated in seconds") }}

## SEO and discovery

### Favicon

The small icon next to your site's name in browser tabs, bookmarks, and history. Place your favicon file in `static/` and configure it in `config.toml`:

```toml
[extra]
favicon = "/favicon.svg"
favicon_mimetype = "image/svg+xml"
```

SVG scales to any size and can adapt to dark mode via CSS. PNG (32x32) and ICO are also supported. See [Add a favicon](../how-to/add-favicon.md).

### Sitemap

An XML file listing every page on your site so search engines can discover and index them efficiently. Zorto generates `sitemap.xml` automatically. See [how to add a sitemap](../how-to/add-sitemap.md) for robots.txt setup and search engine submission.

### robots.txt

A plain-text file at `/robots.txt` that tells search engine crawlers what to index. Create it as `static/robots.txt` in your Zorto project:

```
User-agent: *
Allow: /
Sitemap: https://example.com/sitemap.xml
```

Replace `https://example.com` with your `base_url`. See [how to add a sitemap](../how-to/add-sitemap.md).

### SEO

Search engine optimization — how people find your site through Google (and increasingly, through AI tools like ChatGPT and Claude). Good SEO means structuring your site so search engines understand what each page is about and rank it for relevant searches.

The basics: every page needs a clear `title` and `description` in frontmatter. Beyond that, clean URL structure, fast load times, HTTPS, a sitemap, and Open Graph tags all contribute. Zorto generates sitemaps and llms.txt automatically; built-in themes include Open Graph and canonical URL tags. See the [SEO guide](../how-to/seo.md) for specifics.

### Open Graph

A protocol that controls how your pages appear when shared on social media — the title, description, and preview image in those link cards on Twitter, Facebook, LinkedIn, etc. Implemented via `<meta>` tags in your template's `<head>`. The built-in Zorto themes include Open Graph tags automatically.

### Canonical URLs

When the same page is reachable at multiple URLs (e.g., with and without `www`, or with and without a trailing slash), search engines need to know which version is the "official" one. A canonical URL tag in the `<head>` tells them, preventing duplicate-content penalties. Zorto generates absolute URLs from your `base_url` configuration.

### llms.txt

A proposed standard file (like `robots.txt` but for AI) that gives large language models a structured index of your site's content. Instead of crawling and parsing HTML, an agent can read a single URL and understand your entire site.

Zorto generates two files automatically:
- `/llms.txt` — links to Markdown versions of every page
- `/llms-full.txt` — the full content of every page in one file

This is for **consumption**, not editing. Agents that need to modify your site work directly on the filesystem. `llms.txt` is for agents that need to understand and explain your content.

### Structured data

Machine-readable metadata embedded in your pages that tells search engines what your content represents — not just the words on the page, but that this page is a *recipe*, or an *event*, or a *FAQ*. This powers rich results in search: recipe cards with cooking times, event listings with dates, FAQ dropdowns, product ratings with stars.

The most common format is JSON-LD, a block of JSON in a `<script>` tag in your page's `<head>`. Add it to Zorto pages by overriding the `extra_head` block in your template (see [Customize your theme](../how-to/customize-theme.md)):

<pre><code>&#123;% block extra_head %&#125;
&lt;script type="application/ld+json"&gt;
&#123; "@context": "https://schema.org", "@type": "Article", "headline": "&#123;&#123; page.title &#125;&#125;" &#125;
&lt;/script&gt;
&#123;% endblock %&#125;</code></pre>

Zorto's built-in themes don't include structured data by default — add it for pages where rich results would help.
