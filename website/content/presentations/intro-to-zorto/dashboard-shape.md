+++
title = "Data app shape"
weight = 57

[extra]
layout = "wide"
+++

## Data app shape

<div class="deck-flow">
  <div class="deck-flow__step">
    <strong>Sources</strong>
    <span>Git, content, manifests, generated output, local files.</span>
  </div>
  <div class="deck-flow__step">
    <strong>Pipeline</strong>
    <span>Self-contained `uv` script plus DuckDB transforms.</span>
  </div>
  <div class="deck-flow__step">
    <strong>Artifact</strong>
    <span>Checked-in `site.ddb` today. Remote DuckDB and DuckLake-backed data next.</span>
  </div>
  <div class="deck-flow__step">
    <strong>Config</strong>
    <span>Site-local TOML declares views, SQL, tables, and chart bindings.</span>
  </div>
  <div class="deck-flow__step">
    <strong>Browser</strong>
    <span>DuckDB-Wasm queries data. Plotly renders charts.</span>
  </div>
</div>

Static-first does not mean static-only. The interface is HTML, CSS, and JavaScript.
