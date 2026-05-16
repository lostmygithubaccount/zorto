# Zorto data app builder research

Date: 2026-05-16

## Direction

Make Zorto the static-first data app builder: authored as normal content/config/code, built into plain HTML/CSS/JS, and backed by DuckDB locally and in the browser. The default artifact should become a site/app-scoped DuckDB database, likely `/data/site.ddb`, shipped alongside the static output for search, dashboard data, query-backed pages, local apps, and future CMS workflows.

The mental model:

- Static pages stay fast and inspectable.
- Data apps hydrate only when a reader asks for them.
- DuckDB-Wasm handles browser-side SQL against shipped `.ddb`, CSV, JSON, Parquet, and remote assets.
- Build-time DuckDB and Python run locally as part of `zorto build`, using normal ecosystem tools instead of a hosted orchestrator.
- Quack becomes the optional remote database lane for large, live, private, or multi-writer data.
- Full applications are in scope. Once the data is local, queryable, and packaged with the UI, HTML, JS, and CSS are enough for many useful apps.

## Back to Zorto thesis

The `external/dkdc.dev/content/posts/back-to-zorto.md` post reframes Zorto as more than an SSG: Zorto should orchestrate the site and the data behind it.

The important product layering:

- Content: Markdown, prose, data notes, SQL receipts, mostly human-authored.
- Config: `config.toml`, routes, templates, styles, sources, queries, dashboards, and knobs, editable by humans or agents.
- Code: HTML, CSS, JavaScript, chart glue, database plumbing, browser edge cases, mostly machine-maintained.

That layering is the product constraint. Zorto should not ask people to hand-edit generated JavaScript or tangled pipeline glue just to change words, tune SQL, inspect data, or publish a dashboard. The human contract stays content above config above code.

DuckDB, DuckDB-Wasm, Quack, and `uv` make this practical without inventing a new platform. Zorto can run the pipeline locally, lean on the Python and SQL ecosystems, emit static artifacts, and still deploy anywhere. The output might be a website, but it can also be a local data app or a static data bundle where the primary artifact is the `.ddb` plus a thin HTML/JS UI.

The bolder version: Zorto can build full applications without becoming a traditional app framework. The app state that matters is data; the durable contract is the `.ddb` plus generated/static assets; the interaction layer is HTML, CSS, and JavaScript. AI agents can do the heavy lifting of generating chart glue, forms, controls, and app-specific code, while Zorto keeps strict architecture around inputs, outputs, schemas, receipts, sandboxing, and human-editable config. That gives agents room to move without turning the repo into unreviewable generated mud.

## Current Zorto state

### Search

Zorto currently has first-class client-side search, but it is SQLite-shaped rather than DuckDB-shaped.

- `crates/zorto-core/src/search.rs` writes `search.db` when `generate_search = true`.
- The database has one `pages` table with display fields plus lowercased title, description, and content columns.
- Search ranking is SQL `LIKE` based. It intentionally avoids SQLite FTS5 because the standard `sql.js` WASM build does not include FTS5.
- `crates/zorto-core/themes/zorto/templates/base.html` lazy-loads `sql.js` from CDN, fetches `search.db`, opens it in memory, and runs ranked queries in the browser.
- `docs/concepts/search.md` and `docs/how-to/add-search.md` describe the current `search.db` flow. Some docs still call it a full-text index, which is directionally understandable but technically loose now that the implementation is not FTS-backed.

Opportunity: search is already a proof that Zorto can ship a static database artifact and query it in the browser. Migrating search tables into `/data/site.ddb` would make search the first built-in data app instead of a separate SQLite path.

### Dashboarding

Core Zorto does not yet have dashboard primitives. The best prototype lives in `external/dkdc.dev`.

- `external/dkdc.dev/content/dashboards/` defines dashboard pages as normal markdown with `template = "dashboard.html"` and `extra.dashboard_module`.
- `external/dkdc.dev/templates/dashboard.html` renders a static article shell plus a button. The only eager script is `/js/dashboard-loader.js`.
- `external/dkdc.dev/static/js/dashboard-loader.js` waits for a click before importing `/js/dashboard-runtime.js`.
- `external/dkdc.dev/static/js/dashboard-runtime.js` lazy-loads Plotly and DuckDB-Wasm, creates a shared connection, and passes dashboard modules a small context: `query`, `exec`, `plot`, `registerUrl`, `registerJson`, `registerText`, `fetchJson`, formatting helpers, and escaping.
- The current dashboard modules fetch public CSV/API sources in-browser, register them with DuckDB-Wasm, create tables, run SQL, and render Plotly panels.
- `external/dkdc.dev/content/posts/how-the-dashboards-work.md` already sketches the next step: `zorto build` declares sources, SQL, outputs, dashboard bindings, and emits `/data/site.db` for dashboards to query locally.

The prototype has the right runtime boundary: static shell first, data runtime on demand. Its main limits are that data fetching and schema work happen in the reader's browser, so it is exposed to CORS, upstream drift, slow first runs, and harder reproducibility.

### CMS and webapp

`crates/zorto-webapp` is a local HTMX CMS, currently focused on content and site management.

- `crates/zorto-webapp/src/lib.rs` wires dashboard, pages, sections, config, assets, build, preview, and onboarding routes.
- `crates/zorto-webapp/src/dashboard.rs` summarizes filesystem state: pages, sections, drafts, assets, recent pages, and quick actions.
- There is no data source registry, schema browser, query runner, `.ddb` preview, dashboard scaffold, or build-time data pipeline UI yet.

Opportunity: the CMS can become the human-friendly data app workbench without changing the published static runtime. Add "Data" as a local authoring surface: sources, tables, views, query previews, dashboard scaffolds, and build receipts.

### Database dependencies

Current Zorto has no DuckDB dependency.

- `crates/zorto-core/Cargo.toml` gates SQLite search behind `search = ["dep:rusqlite"]`.
- `rusqlite` is optional and bundled.
- No Rust DuckDB crate, DuckDB CLI invocation, DuckDB-Wasm runtime asset, `.ddb` generator, or data pipeline config exists today.
- Python distribution is already a hard requirement for the project. Existing docs and agent instructions prefer `uv` for Python operations, but Zorto does not yet expose Python/`uv` as a first-class data pipeline runner.

That is good from a minimal-dependency standpoint. The first design pass should choose where DuckDB lives carefully: build-time engine, browser runtime, both, or staged behind a feature flag.

## DuckDB and Quack notes

DuckDB's Quack launch changes the long-term architecture, but it does not remove the need for a static `.ddb` artifact.

Verified from official DuckDB sources:

- Quack was announced on 2026-05-12 as a client-server protocol for DuckDB, letting DuckDB instances talk to each other with multiple concurrent writers.
- Quack is beta in DuckDB v1.5.2 and currently shipped through the `core_nightly` extension repository.
- Quack is HTTP-based, client-driven, and uses `application/duckdb` serialization based on DuckDB internals.
- After the connection handshake, small queries can complete in one request-response pair; large results stream back via follow-up fetches.
- DuckDB-Wasm can speak Quack, so a browser can connect to a remote DuckDB server when the remote endpoint is intentionally exposed.
- Quack defaults to localhost-oriented safety. Externally reachable deployments should sit behind TLS via a normal reverse proxy.
- Plain DuckDB database files can also be attached over HTTPS/S3 in read-only mode, which matters for static `.ddb` delivery.

Implication for Zorto:

- Static default: build `/data/site.ddb`, host it as a normal asset, and let DuckDB-Wasm attach/query it in-browser.
- Remote optional: allow dashboards to attach a Quack endpoint for live or large data, with explicit configuration, auth story, CORS/TLS guidance, and strong "not the default" posture.
- Hybrid future: build-time `.ddb` for public/static tables, Quack for private/live tables, same dashboard runtime context.

## Proposed architecture

### Site database

Introduce one canonical site database artifact:

```text
public/
  data/
    site.ddb
```

Candidate internal schema:

- `zorto_pages`: title, url, description, raw markdown, rendered text, dates, taxonomies, section, draft flag.
- `zorto_sections`: title, url, description, hierarchy, content text.
- `zorto_search`: pre-shaped search rows and ranking helper columns, initially matching today's SQLite behavior.
- `zorto_assets`: optional asset metadata for images, generated figures, downloads.
- `zorto_build`: build metadata, source hashes, Zorto version, generation timestamp.
- `app_*` or user schemas: materialized source tables, views, and dashboard-specific tables.

The first migration can preserve `search.db` for compatibility while also proving `site.ddb`. Once DuckDB search runtime is stable, `search.db` can become legacy output.

### Build-time data pipeline

Add a data pipeline in config, probably after an ADR/prototype:

```toml
[data]
database = "data/site.ddb"

[[data.sources]]
name = "storms_2024"
type = "csv"
url = "https://..."

[[data.transforms]]
name = "storm_summary"
sql = """
CREATE OR REPLACE TABLE dashboard_storm_summary AS
SELECT ...
FROM storms_2024;
"""
```

This should grow into a local orchestrator, not just a database exporter:

- Source steps fetch or read local files, HTTP URLs, APIs, git-tracked data, generated Python outputs, Parquet/CSV/JSON, and existing DuckDB files.
- SQL steps run in DuckDB and materialize tables/views into `site.ddb`.
- Python steps run through `uv`, so dependencies are isolated, locked, fast to install, and described in normal Python project or script metadata.
- Render steps can turn query results into Markdown/Tera context, static tables, cards, feeds, charts, JSON, or dashboard-ready tables.
- Receipt steps write build metadata: source URLs, hashes, row counts, schemas, query text, timings, dependency fingerprints, and errors.

The orchestrator should feel boring and local:

```toml
[data]
database = "data/site.ddb"

[[data.steps]]
name = "fetch_downloads"
run = "uv"
script = "pipelines/fetch_downloads.py"
outputs = ["data/npm_downloads.parquet"]

[[data.steps]]
name = "materialize_downloads"
run = "duckdb"
sql = "pipelines/materialize_downloads.sql"
depends_on = ["fetch_downloads"]
```

Open design choice: run DuckDB through a Rust crate, the DuckDB CLI, or a separately installed optional tool. For Zorto's minimal-dependency goal, the cleanest first implementation may be external-tool based: detect `duckdb` and `uv`, run them locally, produce excellent receipts and errors, and defer heavy embedded dependencies until the shape is proven.

This unlocks two usage modes:

- Website mode: `zorto build` emits HTML plus `.ddb` files for search, dashboards, and query-backed pages.
- Local app mode: Zorto runs the same local pipeline and serves a local data UI without needing the final artifact to be a public website.
- Full app mode: Zorto emits a static application bundle where the application logic is normal JS, the presentation is normal HTML/CSS, and the data model is a shipped or remote DuckDB database.

### Browser runtime

Lift the dkdc.dev prototype into Zorto as a small built-in runtime:

- No DuckDB-Wasm on normal page loads.
- Dashboard/search runtime loads only after intent: search open, dashboard button, query widget, or app route.
- Runtime can `ATTACH '/data/site.ddb'` or register/fetch it depending on DuckDB-Wasm's best supported path.
- Runtime exposes a stable `ctx` to dashboard modules: `query`, `exec`, `plot`, `table`, `setStatus`, `escapeHtml`, source registration helpers, and optional remote attach.
- Plotting should be pluggable. Plotly is excellent for the prototype, but Zorto should avoid making it unavoidable for users who only need tables/search.

For full applications, the runtime should stay boring:

- App modules are plain JavaScript modules.
- App shells are plain templates and generated HTML.
- App state is either in-memory browser state, local files selected by the user, shipped `.ddb` files, or explicit remote DuckDB/Quack connections.
- Zorto owns contracts and receipts, not a custom component framework.
- Generated code is allowed, but it should sit behind stable module boundaries so humans can inspect or replace it.

### Dashboard content model

Keep dashboard pages as content first:

```toml
+++
title = "Storm ledger"
template = "dashboard.html"

[extra.dashboard]
module = "/js/dashboards/storm-ledger.js"
database = "/data/site.ddb"
requires = ["duckdb-wasm", "plotly"]
+++
```

Later, allow lower-code forms:

```toml
[[extra.dashboard.panels]]
title = "Events by month"
query = "SELECT month, count(*) AS events FROM storms GROUP BY month"
chart = "bar"
```

The code-module model is enough for the first real feature because it matches the existing dkdc.dev prototype and keeps Zorto from inventing a chart grammar too early.

### CMS trajectory

The local CMS should become the data authoring surface:

- Data sources list: local files, URLs, APIs, generated tables.
- Table browser for `site.ddb`.
- Query runner using the same SQL as build/runtime.
- Dashboard scaffold generator.
- Build receipts: source URL, content hash, rows loaded, table schema, transform duration, error messages.
- Safety indicators for browser-loaded remote sources versus build-time materialized sources.
- Pipeline controls: run one step, run downstream, inspect logs, clear caches, and open generated artifacts.
- Python environment status: show `uv` project/script metadata, lock state, and dependency errors without making the user leave the CMS.
- App builder controls: scaffold app pages, bind UI controls to queries, inspect generated modules, and keep generated code separated from human-authored content/config.

This fits Zorto's maintainer story: a human and an AI can both edit content, config, SQL, and dashboard modules, while the CMS gives fast feedback.

## Risks and decisions

- File name: user-facing artifact should probably be `.ddb`, but DuckDB examples commonly use `.duckdb` or `.db`. Confirm whether `.ddb` is the brand choice and document that DuckDB accepts it as a normal database file path.
- Storage compatibility: pin the DuckDB version used to write `site.ddb` against the DuckDB-Wasm version used to read it. The site should not emit a database newer than the browser runtime can open.
- Dependency shape: adding DuckDB to Rust may be heavier than invoking `duckdb` when data features are enabled. Decide after a small spike.
- Local runner contract: using `uv` and `duckdb` as external tools keeps Rust dependencies light, but Zorto must define good discovery, version checks, lockfile expectations, and cross-platform error messages.
- CDN versus vendored runtime: current search loads sql.js from CDN; dkdc.dev loads DuckDB-Wasm from jsDelivr and Plotly from local vendor assets. Zorto should decide whether the default is CDN, vendored, or user-controlled.
- Security: post authors are only partly trusted. SQL and data source declarations likely belong to trusted site authors, while shortcode/dashboard args must keep validation and escaping boundaries.
- Remote data: Quack endpoints need TLS, auth, CORS, and explicit opt-in. Static `.ddb` should stay the default because it is cacheable, inspectable, and deploys anywhere.
- Docs drift: search docs should stop promising SQLite FTS5 if the implementation remains `LIKE` based.

## Near-term work candidates

1. Write an ADR for "site database and data app runtime" before code.
2. Prototype `/data/site.ddb` generation for search rows only.
3. Spike the local runner contract: detect `duckdb`, detect `uv`, run one SQL step, run one Python step, and write build receipts.
4. Add a DuckDB-Wasm search runtime behind a feature flag while preserving current `search.db`.
5. Extract the dkdc.dev dashboard loader/runtime shape into a Zorto example or experimental theme asset.
6. Add dashboard page docs using the module-based model.
7. Decide whether first DuckDB integration uses Rust crate, DuckDB CLI, or optional external tool.
8. Add CMS read-only data page that can inspect a generated `site.ddb`.
9. Add local data app mode as a thin wrapper around build, watch, pipeline receipts, and preview.

## Sources

Official DuckDB:

- [Quack project page](https://duckdb.org/quack/)
- [Quack announcement, 2026-05-12](https://duckdb.org/2026/05/12/quack-remote-protocol)
- [Quack docs overview](https://duckdb.org/docs/current/quack/overview)
- [Quack on WebAssembly](https://duckdb.org/docs/current/quack/setup/quack_wasm)
- [DuckDB-Wasm overview](https://duckdb.org/docs/current/clients/wasm/overview)
- [DuckDB-Wasm data ingestion](https://duckdb.org/docs/current/clients/wasm/data_ingestion)
- [ATTACH and DETACH statements](https://duckdb.org/docs/current/sql/statements/attach.html)
- [Directly read DuckDB databases](https://duckdb.org/docs/current/guides/file_formats/read_duckdb)

Official uv:

- [uv docs](https://docs.astral.sh/uv/)
- [uv CLI reference](https://docs.astral.sh/uv/reference/cli/)

Local code and prototypes:

- `crates/zorto-core/src/search.rs`
- `crates/zorto-core/themes/zorto/templates/base.html`
- `crates/zorto-core/Cargo.toml`
- `crates/zorto-webapp/src/lib.rs`
- `crates/zorto-webapp/src/dashboard.rs`
- `docs/concepts/search.md`
- `docs/how-to/add-search.md`
- `docs/reference/python-api.md`
- `crates/zorto-cli/src/skill-zorto.md`
- `external/dkdc.dev/content/posts/back-to-zorto.md`
- `external/dkdc.dev/content/dashboards/_index.md`
- `external/dkdc.dev/content/posts/how-the-dashboards-work.md`
- `external/dkdc.dev/templates/dashboard.html`
- `external/dkdc.dev/templates/dashboards.html`
- `external/dkdc.dev/static/js/dashboard-loader.js`
- `external/dkdc.dev/static/js/dashboard-runtime.js`
- `external/dkdc.dev/static/js/dashboards/*.js`
