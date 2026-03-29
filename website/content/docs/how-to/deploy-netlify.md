+++
title = "Deploy your site"
template = "docs.html"
slug = "deploy"
+++

Zorto builds a static `public/` directory that can be hosted anywhere.

## Build

```bash
zorto build
```

This outputs everything to `public/`. Upload that directory to any static hosting provider.

## GitHub Pages

Add a GitHub Actions workflow at `.github/workflows/deploy.yml`:

```yaml
name: Deploy
on:
  push:
    branches: [main]
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: pip install zorto && zorto build
      - uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./public
```

## Any static host

Zorto outputs plain HTML, CSS, and assets. Point any static hosting provider at the `public/` directory:

- **Cloudflare Pages**: build command `pip install zorto && zorto build`, output dir `public`
- **Vercel**: same build command and output dir
- **S3 / GCS**: upload `public/` to a bucket with static hosting enabled

> [!TIP]
> The Python install (`pip install zorto`) is fastest on CI since it downloads a prebuilt wheel. `cargo install zorto` compiles from source and takes longer.
