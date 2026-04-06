# Search

Zorto includes built-in client-side search. At build time, it generates a SQLite database of all your pages. In the browser, [sql.js](https://sql.js.org/) (SQLite compiled to WASM) loads the database and runs queries entirely client-side — no server required.

## How it works

{{ flow(steps="Build:zorto indexes all pages into search.db|Load:Browser fetches search.db + sql.js WASM|Query:User types in Ctrl+K modal|Rank:SQL scores matches by field|Display:Top 10 results shown instantly", caption="Everything runs in the browser. No search server, no API calls.") }}

### Build time

When `generate_search = true`, Zorto creates a `search.db` file in your output directory containing every page and section:

| Column | Source |
|--------|--------|
| `title` | Page title |
| `url` | Relative URL |
| `description` | Frontmatter `description` field |
| `content` | Rendered HTML stripped to plain text |

Pre-computed lowercase columns (`title_lower`, `description_lower`, `content_lower`) enable case-insensitive matching without runtime overhead.

### Client side

The search modal dynamically loads sql.js from CDN and fetches `/search.db` as a binary blob. The database is created in memory and cached for the session — subsequent searches are instant.

### Ranking

Results are scored by where the match occurs:

| Match type | Points |
|------------|--------|
| Exact title match | 100 |
| Title starts with query | 80 |
| Title contains query | 60 |
| Description contains query | 20 |
| Content contains query | 10 |

Scores are additive — a match in both title and description scores higher than title alone. Results are ordered by total score, limited to the top 10.

### Why LIKE instead of FTS5

SQLite's FTS5 (full-text search) extension would be ideal, but the standard sql.js WASM build doesn't include it. Zorto uses `LIKE` queries with pre-computed lowercase columns instead. This trades some search sophistication (no stemming or phrase matching) for broad compatibility — it works in every browser that supports WASM, with no custom sql.js build required.

## The search UI

The search modal opens with **Ctrl+K** (or **Cmd+K** on macOS). It includes:

- A text input with 100ms debounce (minimum 3 characters to search)
- Keyboard navigation: arrow keys to move, Enter to open, Escape to close
- Result snippets from description or content with highlighted matches
- Mobile support via a search button in the header

## Further reading

- [Add search to your site](../how-to/add-search.md) — enable and configure search
- [Configuration](configuration.md) — the `generate_search` setting
