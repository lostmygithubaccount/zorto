Get a site running in 30 seconds.

## 1. Install

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

## 2. Create

```bash
zorto init mysite
cd mysite
```

## 3. Preview

```bash
zorto preview --open
```

That's it. You have a live site with hot reload at `http://127.0.0.1:1111`.

Edit `content/_index.md`, save, and watch it update instantly.

## Next steps

- [Installation](installation.md): other install methods (cargo, pip)
- [First site](first-site.md): project structure, adding pages, blogs
- [Concepts](../concepts/README.md): how Zorto works under the hood
