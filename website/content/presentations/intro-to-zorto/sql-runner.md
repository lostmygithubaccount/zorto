+++
title = "SQL runner"
weight = 59

[extra]
layout = "wide"
slide_theme = "ink"
+++

## Query the deck data

<div class="deck-query-app" data-deck-query-app data-database-url="/data/site.ddb" data-database-file="site.ddb" data-database-schema="site">
  <div class="deck-query-app__editor">
    <textarea data-deck-query-editor spellcheck="false">SELECT kind, count(*) AS files, round(sum(bytes) / 1024.0, 1) AS kb
FROM site.main.build_outputs
GROUP BY kind
ORDER BY kb DESC
LIMIT 8;</textarea>
    <div class="deck-query-app__bar">
      <button class="deck-data-app__button" type="button" data-deck-query-run>Run SQL</button>
      <p class="deck-data-app__status" data-deck-query-status aria-live="polite">ready</p>
    </div>
  </div>
  <div class="deck-query-app__result">
    <table>
      <thead data-deck-query-head></thead>
      <tbody data-deck-query-body></tbody>
    </table>
  </div>
</div>

This deployment is static: the browser is doing the database work.
