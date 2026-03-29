+++
title = "Quick start"
template = "docs.html"
+++

## Create a new site

```bash
zorto init mysite
```

## Preview locally

```bash
cd mysite
zorto preview
```

Your browser opens automatically at `http://127.0.0.1:1111`. Edit any file in `content/` and the page reloads instantly.

## Build for production

```bash
zorto build
```

The output lands in `public/`. Deploy that directory to any static host.
