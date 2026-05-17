# Zorto data app next steps

Date: 2026-05-16

## Goal

Turn the zorto.dev analytics prototype into a good general Zorto data-app pattern before cutting a release or updating the live website.

The target shape is still the `back to Zorto` layering:

- Content: Markdown owns prose, context, and data notes.
- Config: TOML owns sources, pipeline steps, dashboard panels, saved queries, and knobs.
- Code: Python, SQL, HTML, CSS, and JavaScript are machinery behind stable boundaries.

## Current checkpoint

Committed work so far:

- `b3c7866` adds the zorto.dev analytics prototype.
- `051794b` moves dashboard intent into `website/data/analytics.toml` and emits `/data/analytics-dashboard.json`.
- `841715b` documents this generalization plan.

The prototype proves the important runtime shape:

- A static page shell loads first.
- `site.ddb` ships as a public static artifact.
- DuckDB-Wasm and Plotly load only after user intent.
- The dashboard queries local browser-attached DuckDB data.
- The checked-in database avoids private paths, emails, env vars, tokens, and untracked filenames.

Implemented after this checkpoint:

- `website/data/meta.toml` now owns the website-local metadata pipeline contract.
- `site.ddb` now includes `pipeline_steps` receipts for the generation path.
- `website/data/analytics.toml` owns dashboard queries, panel query bindings, and table columns.
- `website/static/js/data-app-runtime.js` holds reusable static data-app machinery, with `analytics-dashboard.js` acting as the analytics adapter.
- Plotly and DuckDB-Wasm are both pinned CDN-loaded runtime assets. Vendoring them should be a deliberate packaging decision later, not an accidental split policy.

## Why this is not general enough yet

The pattern is promising, but the reusable product has not been extracted.

- `website/pipelines/build_meta.py` still owns repo-specific collectors and table materialization logic.
- The dashboard manifest owns queries and table bindings, but chart renderers are still named JavaScript functions.
- There is no Zorto-native `[data]` config, pipeline runner, receipts model, or dashboard scaffold.
- Search still writes `search.db` separately instead of joining the `.ddb` direction.
- DuckDB-Wasm versioning and loading are still page-specific.
- The CMS has no data workbench yet.

## Next implementation phase

### 1. Move pipeline intent into TOML

Done for the website prototype: `website/data/meta.toml`.

It should describe:

- Output database path.
- Build output directory.
- Source groups: git, manifests, content files, generated site output.
- Tables to materialize.
- Privacy guards.
- Receipt expectations.

Keep the executor as the self-contained `uv` script for now. The point is to move the "what" into config while the script remains the "how".

### 2. Add build receipts

Done for the website prototype: `pipeline_steps`.

Useful fields:

- step name
- step kind
- started and finished timestamps
- duration
- status
- input count
- output count
- command or SQL file reference
- warning or error text

The dashboard should show these receipts so the data pipeline is inspectable.

### 3. Make dashboard rendering more generic

Partly done. `website/data/analytics.toml` now declares:

- query id
- renderer
- table columns
- empty states
- formatting hints

Still intentionally deferred: chart x/y/color fields. Keep named renderers for anything too custom. Do not invent a full chart grammar yet.

### 4. Extract a small reusable runtime

Done for the website prototype:

- `website/static/js/data-app-runtime.js`
- `website/static/js/analytics-dashboard.js` as a thin analytics-specific adapter

The reusable runtime should own:

- manifest loading
- DuckDB-Wasm loading
- read-only database attachment
- query execution
- tabs
- table rendering
- error/loading states
- Plotly loading when requested

### 5. Document the experimental pattern

Add docs before release:

- How to ship a `.ddb` beside a Zorto site.
- How to write a self-contained `uv` pipeline.
- How to define a dashboard manifest.
- What privacy guarantees the checked-in DB should satisfy.
- What is experimental and website-local versus supported Zorto API.

### 6. Decide what belongs in Zorto proper

Only promote the boring parts into core/CLI after the website prototype feels stable.

Likely candidates:

- A data-app scaffold/template.
- A generic dashboard template.
- A small static runtime asset.
- Optional `zorto data` or `zorto build --data` runner.
- Future `[data]` config.

Do not add a Rust DuckDB dependency until an external-tool runner has proven inadequate.

## Release gates

Before a Zorto release:

- `bin/check-rs`
- `bin/check-py`
- `bin/test-py`
- `website/bin/build-meta`
- `website/bin/build`
- Browser smoke for `/analytics/`
- DuckDB table count and representative query checks
- Privacy scan on `website/static/data/site.ddb`
- Confirm `git status` contains only intentional changes
- Confirm docs clearly mark the data-app pattern as experimental if it is not a public API yet

## Live website gates

Before updating live zorto.dev:

- Regenerate `website/static/data/site.ddb`.
- Regenerate `website/static/data/analytics-dashboard.json`.
- Confirm the pinned Plotly and DuckDB-Wasm CDN assets load.
- Confirm `/analytics/` works from local preview.
- Confirm the shipped `.ddb` contains no private data.
- Confirm CDN-loaded DuckDB-Wasm version can read the generated database.
- Decide whether `/analytics/` should be linked in nav for the first public push.

## Open decisions

- Should the generalized DB name always be `site.ddb`, or should projects configure it freely?
- Should saved SQL live inline in TOML or in `.sql` files referenced by TOML?
- Should dashboards be configured from section frontmatter, `data/*.toml`, or both?
- Should the first generalized runner invoke `duckdb` CLI, Python `duckdb`, or support either?
- How much chart configuration is useful before it becomes a bad chart DSL?
- What should the supported runtime-asset default be: CDN, vendored, or user-controlled per site?
- When should search move from SQLite `search.db` into DuckDB?
