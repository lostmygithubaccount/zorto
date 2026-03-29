+++
title = "Quick start"
template = "docs.html"
slug = "quick-start"
date = "2099-01-02"
+++

Get a site running in 30 seconds.

## 1. install

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

## 2. create

```bash
zorto init mysite
cd mysite
```

## 3. preview

```bash
zorto preview --open
```

That's it. You have a live site with hot reload at `http://127.0.0.1:1111`.

Edit `content/_index.md`, save, and watch it update instantly.

## next steps

- [installation](/docs/getting-started/installation/): other install methods (cargo, pip)
- [first site](/docs/getting-started/first-site/): project structure, adding pages, blogs
- [concepts](/docs/concepts/): how Zorto works under the hood
