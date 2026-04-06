# Add search

Zorto has built-in client-side search powered by SQLite and WASM. One config line enables it.

## Enable search

Add to your `config.toml`:

```toml
generate_search = true
```

Build your site:

```bash
zorto build
ls public/search.db
```

That's it. Zorto generates `search.db` at build time and the theme's search modal handles the rest.

## What gets indexed

Every page and section in your site is indexed:

- **Title** — from the page title
- **Description** — from frontmatter `description` field
- **Content** — rendered HTML stripped to plain text

Pages without content still appear in search results via their title and description.

## Using search

Open the search modal with **Ctrl+K** (or **Cmd+K** on macOS). On mobile, tap the search icon in the header.

Type at least 3 characters to start searching. Results are ranked by match location — title matches score highest, followed by description, then content. Use arrow keys to navigate results and Enter to open.

## Customization

Search is part of the base template in Zorto's built-in themes. If you're using a [custom theme](customize-theme.md), you'll need to include the search modal and JavaScript in your own `base.html` template. See the built-in theme's `base.html` for the implementation.

The search database is a standard SQLite file. You can inspect it directly:

```bash
sqlite3 public/search.db "SELECT title, url FROM pages LIMIT 5;"
```

## Related guides

- [Search concepts](../concepts/search.md) — how search works under the hood
- [Configuration](../concepts/configuration.md) — all config.toml options
