# Deploy your site

Zorto builds [static files](../concepts/glossary.md#static-site) — HTML, CSS, JS — that can be hosted on any [static hosting](../concepts/glossary.md#static-hosting) provider. No server-side runtime required.

{{ flow(steps="Build:zorto build|Upload:Push public/ to host|Live:Site served globally") }}

## Build for production

```bash
zorto build
```

This generates your site in `public/`. Upload that directory to any static hosting provider.

## Netlify

Create a `netlify.toml` in your project root:

```toml
[build]
command = "curl -LsSf https://dkdc.sh/zorto/install.sh | sh && zorto build"
publish = "public"
```

Push to GitHub and connect the repo in Netlify's dashboard. Every push triggers a build.

## GitHub Pages

Add a workflow at `.github/workflows/deploy.yml`:

```yaml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    permissions:
      pages: write
      id-token: write
    steps:
      - uses: actions/checkout@v4
      - run: curl -LsSf https://dkdc.sh/zorto/install.sh | sh && zorto build
      - uses: actions/upload-pages-artifact@v3
        with:
          path: public
      - id: deployment
        uses: actions/deploy-pages@v4
```

## Vercel

Create a `vercel.json`:

```json
{
  "buildCommand": "curl -LsSf https://dkdc.sh/zorto/install.sh | sh && zorto build",
  "outputDirectory": "public"
}
```

## Cloudflare Pages

Connect your Git repository in the Cloudflare Pages dashboard and configure:

- **Build command**: `curl -LsSf https://dkdc.sh/zorto/install.sh | sh && zorto build`
- **Build output directory**: `public`

Every push to your production branch triggers a build. Cloudflare Pages also creates preview deployments for pull requests automatically.

## Custom headers

Create a `static/_headers` file (Netlify) or configure headers in your platform's config. Zorto copies everything in `static/` to `public/` at build time.

## Next steps

- [Set up a custom domain](custom-domain.md) — DNS records for each platform
- [Optimize for SEO](seo.md) — meta tags, Open Graph, canonical URLs
- [Add a sitemap](add-sitemap.md) — submit your site to search engines
