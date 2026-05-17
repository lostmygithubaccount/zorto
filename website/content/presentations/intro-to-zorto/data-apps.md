+++
title = "Data apps"
weight = 58

[extra]
slide_theme = "ink"
layout = "wide"
+++

## The slide can query `site.ddb`

<div class="deck-data-app" data-deck-data-app data-database-url="/data/site.ddb" data-database-file="site.ddb" data-database-schema="site">
  <div class="deck-data-app__copy">
    <p>Zorto can ship a public DuckDB database beside the site, then let a page query it in the browser.</p>
    <ul>
      <li><code>site.ddb</code>: generated locally</li>
      <li>DuckDB-Wasm attaches it read-only</li>
      <li>Plotly renders the result inside this deck</li>
    </ul>
    <button class="deck-data-app__button" type="button" data-deck-data-load>Load site.ddb</button>
    <p class="deck-data-app__status" data-deck-data-status aria-live="polite">waiting for slide</p>
  </div>
  <div class="deck-data-app__viz">
    <div class="deck-data-app__kpis" data-deck-data-kpis>
      <span>content files</span>
      <strong>pending</strong>
      <span>build output</span>
      <strong>pending</strong>
      <span>latest build</span>
      <strong>pending</strong>
    </div>
    <div class="deck-data-app__chart" data-deck-data-chart></div>
  </div>
</div>
