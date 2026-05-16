# Data apps

Zorto can publish static data apps: HTML, CSS, JavaScript, and public database files served as ordinary static assets.

This is experimental today. The zorto.dev analytics page is the first prototype, not a stable public Zorto API.

## Current shape

The zorto.dev analytics prototype uses three layers:

- **Content** lives in Markdown and owns the page title, description, and explanatory prose.
- **Config** lives in TOML and owns pipeline paths, runtime asset URLs, dashboard views, SQL queries, panel bindings, and table columns.
- **Code** is machinery: a self-contained `uv` script builds the database, a template defines the page shell, and JavaScript loads DuckDB-Wasm and Plotly after user intent.

The public artifact is `meta.ddb`, a DuckDB database shipped beside the site. The browser fetches it, attaches it read-only with DuckDB-Wasm, and runs dashboard queries locally.

## What ships

The prototype emits:

- `website/static/data/meta.ddb` — public repository, site, build, content, package, and pipeline metadata
- `website/static/data/analytics-dashboard.json` — generated dashboard manifest compiled from TOML
- `website/static/js/data-app-runtime.js` — reusable static data-app loading and query machinery
- `website/static/js/analytics-dashboard.js` — analytics-specific renderers

The metadata generator intentionally avoids private data: no author emails, no absolute repo paths, no environment variables, no tokens, and no untracked filenames.

## Runtime assets

The analytics page lazy-loads pinned CDN assets for DuckDB-Wasm and Plotly after the visitor clicks the load control. Normal pages do not load those assets.

Vendored runtime assets, offline builds, and a supported asset policy are future design work.

## What is not stable yet

These pieces are still website-local:

- The `website/data/meta.toml` pipeline manifest shape
- The `website/data/analytics.toml` dashboard manifest shape
- The `pipeline_steps` receipt schema
- The browser data-app runtime
- The decision to use `meta.ddb` versus a future `site.ddb`

Zorto core still builds static sites. It does not yet provide `[data]` config, `zorto data`, automatic pipeline hooks, or a general dashboard scaffold.

## Why this fits static sites

Static hosting can serve database files the same way it serves images or JavaScript. The server still does not run application code or query a server-side database. The visitor's browser fetches public data files and executes local queries.

That keeps deployment simple while opening a path to richer docs, dashboards, search, catalogs, and local data apps.

## Further reading

- [Search](search.md) — the current SQLite-powered client-side search system
- [AI-native](ai-native.md) — content, config, and code boundaries for agent-friendly sites
- [Executable code blocks](executable-code.md) — build-time code execution
