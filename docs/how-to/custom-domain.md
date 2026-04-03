# Set up a custom domain

Connect your own [domain](../concepts/glossary.md#domain-name) to a Zorto site hosted on any [static hosting](../concepts/glossary.md#static-hosting) provider.

{{ flow(steps="Update:Set base_url in config|DNS:Add records at your registrar|Wait:DNS propagates (minutes to an hour)|Live:HTTPS works automatically", caption="The entire process takes minutes of work, then waiting for DNS propagation.") }}

## Update base_url

Set your production domain in `config.toml`:

```toml
base_url = "https://yourdomain.com"
```

This ensures all generated URLs (sitemap, feeds, canonical links) point to the correct domain.

## Configure [DNS](../concepts/glossary.md#dns)

Add [DNS records](../concepts/glossary.md#cname-record) for your domain. The exact records depend on your hosting provider.

**Netlify:**

Add a CNAME record pointing your domain to your Netlify site:

| Type  | Name | Value                        |
|-------|------|------------------------------|
| CNAME | www  | your-site.netlify.app        |
| A     | @    | 75.2.60.5                    |

Then add your domain under Site settings > Domain management in Netlify.

**GitHub Pages:**

Add these records and configure the domain in your repository's Pages settings:

| Type  | Name | Value                        |
|-------|------|------------------------------|
| CNAME | www  | username.github.io           |
| A     | @    | 185.199.108.153              |
| A     | @    | 185.199.109.153              |
| A     | @    | 185.199.110.153              |
| A     | @    | 185.199.111.153              |

Create a `static/CNAME` file containing your domain (e.g. `yourdomain.com`). Zorto copies it to `public/` at build time.

**Cloudflare Pages:**

Add a CNAME record and configure the custom domain in the Cloudflare Pages dashboard:

| Type  | Name | Value                        |
|-------|------|------------------------------|
| CNAME | www  | your-site.pages.dev          |
| CNAME | @    | your-site.pages.dev          |

**Vercel:**

Add a CNAME record and configure the domain in Vercel's project settings:

| Type  | Name | Value                        |
|-------|------|------------------------------|
| CNAME | www  | cname.vercel-dns.com         |
| A     | @    | 76.76.21.21                  |

## [HTTPS](../concepts/glossary.md#https--ssl)

All major static hosting providers provision [TLS certificates](../concepts/glossary.md#https--ssl) automatically for custom domains. No additional configuration is needed — HTTPS works once DNS propagation completes, typically within a few minutes to an hour.

## Related guides

- [Deploy your site](deploy.md) — build commands and hosting setup for each platform
- [Optimize for SEO](seo.md) — ensure `base_url` is correct for canonical URLs
