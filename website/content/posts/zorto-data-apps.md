+++
title = "Zorto as a data app builder"
date = "2026-05-16"
author = "Cody"
description = "The zorto.dev analytics page is the first prototype of static data apps backed by DuckDB."
tags = ["zorto", "duckdb", "data"]
+++

Zorto should build the site and the data behind it.

<!-- more -->

The first proof is live in this repo now: zorto.dev has a local analytics dashboard powered by a checked-in DuckDB database.

This is not visitor analytics. No tracking, no cookies, no third-party event stream. It is metadata about the site itself: commits, packages, content files, local links, build outputs, and the pipeline steps that produced the database.

## Content above config above code

The point is still layers:

- **Content**: Markdown owns the page title, description, and explanation.
- **Config**: TOML owns sources, queries, panels, table columns, runtime assets, and knobs.
- **Code**: Python, SQL, HTML, CSS, and JavaScript do the machinery work.

That boundary matters because it gives humans a stable surface to edit and gives agents a strict architecture to work inside.

## What changed

The website now has an `/analytics/` page. The shell is static HTML. When you click the load button, it lazy-loads DuckDB-Wasm and Plotly, fetches `/data/meta.ddb`, attaches it read-only in the browser, and renders the dashboard locally.

The metadata pipeline lives in `website/bin/build-meta`. It runs as a self-contained `uv` script, uses DuckDB, runs a timed current-code Zorto build through the Rust CLI, and writes `website/static/data/meta.ddb` only after generation succeeds.

The pipeline now has a manifest too: `website/data/meta.toml`. That file owns the database path, build output path, collection limits, content include/exclude rules, privacy checks, and the build command. The script is the executor; the manifest is the intent.

## Receipts, not magic

The database includes a `pipeline_steps` table. Every generation records what ran, when it started, when it finished, how long it took, what kind of step it was, and how many rows or files it produced.

That is the shape I want for Zorto data work: run locally, write durable artifacts, leave receipts.

## Static still means static

Shipping a `.ddb` file does not make the site dynamic in the server sense. Static hosting still serves files. The server does not run application code, query a database, or need a private process.

The browser can do more now because DuckDB-Wasm is good enough to make local querying boring. That is the unlock.

## What is deliberately not done

This is a zorto.dev prototype, not a public Zorto API yet.

Search still uses `search.db` and sql.js. There is no `[data]` config in Zorto core. There is no automatic data pipeline hook in `zorto build`. There is no remote database story in this pass. DuckDB-Wasm and Plotly are pinned CDN-loaded runtime assets for now; vendoring and offline packaging need an explicit decision later.

That restraint is the important part. Prove the pattern on the website, make it boring, then promote the stable pieces into Zorto.

## Where this is going

Zorto can become a static data app builder without becoming a traditional app framework.

Run pipelines locally. Use Python through `uv`, SQL through DuckDB, Rust for the site generator, HTML/CSS/JavaScript for the interface. Ship the data beside the site. Let agents help with the code, but keep the human-editable contract in Markdown and TOML.

That is the next version of Zorto I want to build toward.
