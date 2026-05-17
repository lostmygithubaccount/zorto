# Add search

Zorto search is a DuckDB data app. Generate a public `.ddb` file with a `search_pages` table, ship it as a static asset, and point the theme at it.

## Configure the theme

Add the database location to `config.toml`:

```toml
[extra]
search_database_url = "/data/site.ddb"
search_database_file = "site.ddb"
search_database_schema = "site"
```

When `search_database_url` is set, the built-in theme adds the search button and modal. The browser imports DuckDB-Wasm only when someone opens search.

## Create `search_pages`

Your `.ddb` should include:

| Column | Type |
|--------|------|
| `title` | text |
| `url` | text |
| `description` | text |
| `content` | text |
| `title_lower` | text |
| `description_lower` | text |
| `content_lower` | text |

zorto.dev builds this with `website/bin/build-meta`, a self-contained `uv` script that writes `website/static/data/site.ddb`.

## Inspect the data

Use DuckDB directly:

```bash
duckdb static/data/site.ddb "SELECT title, url FROM search_pages LIMIT 5;"
```

## Related guides

- [Search concepts](../concepts/search.md): how the browser search query works
- [Data apps](../concepts/data-apps.md): dashboards and `.ddb` files
