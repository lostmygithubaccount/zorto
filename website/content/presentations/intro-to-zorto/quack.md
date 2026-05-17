+++
title = "Quack"
weight = 61

[extra]
layout = "wide"
background_color = "#06151f"
+++

## Dynamic when needed

{% columns(widths="50%|50%") %}

### Static-first

- **Public data**: ship `site.ddb`
- **Local query**: DuckDB-Wasm in the browser
- **Deploy**: any static host
- **Cost**: no server process

<!-- column -->

### Live data

- **Quack**: DuckDB instances talk over HTTP
- **Attach**: remote catalogs through a `quack:` URI
- **Auth**: scoped secret or explicit `TOKEN` option
- **Interface**: HTMX or JS updates browser views
- **Security**: localhost default, reverse proxy/TLS for non-local

{% end %}

Zorto should not hide this. It should make the boundary obvious.
