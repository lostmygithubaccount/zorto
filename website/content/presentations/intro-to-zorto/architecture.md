+++
title = "Architecture"
weight = 32

[extra]
layout = "wide"
+++

## Small core, wide surface

<div class="deck-card-grid deck-card-grid--four">
  <div class="deck-card">
    <strong>Rust core</strong>
    <span>Site model, rendering, search, checks, and output safety.</span>
  </div>
  <div class="deck-card">
    <strong>Rust CLI</strong>
    <span>Build, preview, clean, init, and developer workflow commands.</span>
  </div>
  <div class="deck-card">
    <strong>Python distribution</strong>
    <span>PyO3 bindings and `uv tool install zorto` for broad installation.</span>
  </div>
  <div class="deck-card">
    <strong>Static runtime</strong>
    <span>HTML, CSS, JavaScript, search tables, and public database files.</span>
  </div>
</div>

Code can grow behind stable boundaries. Content and config stay readable.
