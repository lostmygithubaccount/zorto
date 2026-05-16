const DUCKDB_MODULE_URL = 'https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.33.1-dev45.0/+esm';
const PLOTLY_URL = '/vendor/plotly/plotly-3.4.0.min.js';
const META_DB_URL = '/data/meta.ddb';

const palette = [
  'rgb(167,139,250)',
  'rgb(34,211,238)',
  'rgb(52,211,153)',
  'rgb(251,191,36)',
  'rgb(248,113,113)',
  'rgb(96,165,250)',
  'rgb(244,114,182)'
];

const savedQueries = [
  {
    id: 'catalog',
    label: 'Catalog',
    sql: `
SELECT 'meta_info' AS table_name, count(*) AS rows FROM meta.main.meta_info
UNION ALL SELECT 'repo_snapshot', count(*) FROM meta.main.repo_snapshot
UNION ALL SELECT 'commits', count(*) FROM meta.main.commits
UNION ALL SELECT 'commit_daily', count(*) FROM meta.main.commit_daily
UNION ALL SELECT 'packages', count(*) FROM meta.main.packages
UNION ALL SELECT 'content_files', count(*) FROM meta.main.content_files
UNION ALL SELECT 'content_terms', count(*) FROM meta.main.content_terms
UNION ALL SELECT 'content_links', count(*) FROM meta.main.content_links
UNION ALL SELECT 'build_runs', count(*) FROM meta.main.build_runs
UNION ALL SELECT 'build_outputs', count(*) FROM meta.main.build_outputs
ORDER BY rows DESC`
  },
  {
    id: 'terms',
    label: 'Terms',
    sql: `
SELECT term, file_count, occurrence_count
FROM meta.main.content_terms
ORDER BY occurrence_count DESC, file_count DESC
LIMIT 30`
  },
  {
    id: 'freshness',
    label: 'Freshness',
    sql: `
WITH generated AS (
  SELECT CAST(generated_at AS DATE) AS generated_day
  FROM meta.main.meta_info
  LIMIT 1
)
SELECT path, kind, word_count, date_diff('day', CAST(last_commit_at AS DATE), generated_day) AS days_since_update
FROM meta.main.content_files, generated
WHERE kind IN ('docs', 'content')
ORDER BY days_since_update DESC NULLS LAST, word_count DESC
LIMIT 30`
  },
  {
    id: 'outputs',
    label: 'Outputs',
    sql: `
SELECT path, kind, extension, CAST(bytes AS DOUBLE) AS bytes
FROM meta.main.build_outputs
ORDER BY bytes DESC
LIMIT 30`
  },
  {
    id: 'links',
    label: 'Links',
    sql: `
SELECT source_path, target, target_path, link_kind, target_exists
FROM meta.main.content_links
ORDER BY target_exists ASC NULLS LAST, source_path
LIMIT 40`
  }
];

let plotlyPromise;
let duckPromise;

document.querySelectorAll('[data-analytics-load]').forEach((button) => {
  button.addEventListener('click', () => loadAnalytics(button));
});

async function loadAnalytics(button) {
  if (button.dataset.loading === 'true') return;

  const page = button.closest('[data-analytics-page]');
  const dashboard = page.querySelector('[data-analytics-dashboard]');
  const status = page.querySelector('[data-analytics-status]');
  const setStatus = (text) => { if (status) status.textContent = text; };

  button.dataset.loading = 'true';
  button.disabled = true;
  setStatus('loading Plotly');

  try {
    await loadPlotly();
    setStatus('loading DuckDB-Wasm');
    const duck = await loadDuckDB();
    setStatus('opening meta.ddb');
    await attachMetaDatabase(duck);
    setStatus('querying metadata');
    const data = await loadDashboardData(duck.conn);
    dashboard.hidden = false;
    renderDashboard(page, data, duck.conn);
    button.textContent = 'analytics loaded';
    setStatus('ready');
  } catch (error) {
    console.error(error);
    button.disabled = false;
    setStatus('failed');
    dashboard.hidden = false;
    dashboard.innerHTML = '<div class="analytics-error"><strong>Analytics failed to load.</strong><p>' +
      escapeHtml(error && error.message ? error.message : String(error)) +
      '</p></div>';
  } finally {
    button.dataset.loading = 'false';
  }
}

function loadPlotly() {
  if (window.Plotly) return Promise.resolve(window.Plotly);
  if (!plotlyPromise) {
    plotlyPromise = new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = PLOTLY_URL;
      script.async = true;
      script.charset = 'utf-8';
      script.onload = () => resolve(window.Plotly);
      script.onerror = () => reject(new Error('failed to load Plotly'));
      document.head.appendChild(script);
    });
  }
  return plotlyPromise;
}

async function loadDuckDB() {
  if (!duckPromise) {
    duckPromise = (async () => {
      const duckdb = await import(DUCKDB_MODULE_URL);
      const bundle = await duckdb.selectBundle(duckdb.getJsDelivrBundles());
      const worker = await duckdb.createWorker(bundle.mainWorker);
      const db = new duckdb.AsyncDuckDB(new duckdb.VoidLogger(), worker);
      await db.instantiate(bundle.mainModule, bundle.pthreadWorker || null);
      const conn = await db.connect();
      return { duckdb, db, conn, metaAttached: false };
    })();
  }
  return duckPromise;
}

async function attachMetaDatabase(duck) {
  if (duck.metaAttached) return;
  const response = await fetch(META_DB_URL);
  if (!response.ok) throw new Error('failed to fetch ' + META_DB_URL + ' (HTTP ' + response.status + ')');
  const bytes = new Uint8Array(await response.arrayBuffer());
  await duck.db.registerFileBuffer('meta.ddb', bytes);
  await duck.conn.query("ATTACH 'meta.ddb' AS meta (READ_ONLY)");
  duck.metaAttached = true;
}

async function loadDashboardData(conn) {
  const tableCountsSql = savedQueries.find((queryDef) => queryDef.id === 'catalog').sql;
  return {
    info: await first(conn, 'SELECT * FROM meta.main.meta_info LIMIT 1'),
    snapshot: await first(conn, 'SELECT * FROM meta.main.repo_snapshot LIMIT 1'),
    totals: await first(conn, `
      SELECT
        (SELECT count(*) FROM meta.main.commits) AS commits,
        (SELECT count(*) FROM meta.main.content_files) AS content_files,
        (SELECT coalesce(sum(word_count), 0) FROM meta.main.content_files) AS words,
        (SELECT CAST(coalesce(sum(bytes), 0) AS DOUBLE) FROM meta.main.build_outputs) AS output_bytes,
        (SELECT duration_ms FROM meta.main.build_runs ORDER BY finished_at DESC LIMIT 1) AS latest_build_ms,
        (SELECT count(*) FROM meta.main.content_links WHERE target_exists = false) AS broken_links,
        (SELECT count(*) FROM meta.main.content_terms) AS terms
    `),
    health: await first(conn, `
      SELECT
        count(*) FILTER (WHERE kind IN ('docs', 'content')) AS authored_files,
        count(*) FILTER (WHERE kind IN ('docs', 'content') AND title IS NULL) AS untitled_files,
        count(*) FILTER (WHERE kind IN ('docs', 'content') AND word_count = 0) AS empty_files,
        (SELECT count(*) FROM meta.main.content_links) AS local_links,
        (SELECT count(*) FROM meta.main.content_links WHERE target_exists = false) AS broken_links,
        (SELECT count(*) FROM meta.main.content_terms) AS indexed_terms
      FROM meta.main.content_files
    `),
    daily: await query(conn, `
      SELECT day, commit_count, additions, deletions
      FROM meta.main.commit_daily
      ORDER BY day
    `),
    freshness: await query(conn, `
      WITH generated AS (
        SELECT CAST(generated_at AS DATE) AS generated_day
        FROM meta.main.meta_info
        LIMIT 1
      ),
      aged AS (
        SELECT
          kind,
          CASE
            WHEN last_commit_at IS NULL THEN 'unknown'
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 7 THEN '0-7d'
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 30 THEN '8-30d'
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 90 THEN '31-90d'
            ELSE '90d+'
          END AS age_bucket,
          CASE
            WHEN last_commit_at IS NULL THEN 5
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 7 THEN 1
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 30 THEN 2
            WHEN date_diff('day', CAST(last_commit_at AS DATE), generated_day) <= 90 THEN 3
            ELSE 4
          END AS age_order,
          word_count
        FROM meta.main.content_files, generated
        WHERE kind IN ('docs', 'content')
      )
      SELECT kind, age_bucket, min(age_order) AS age_order, count(*) AS file_count, coalesce(sum(word_count), 0) AS words
      FROM aged
      GROUP BY kind, age_bucket
      ORDER BY age_order, kind
    `),
    terms: await query(conn, `
      SELECT term, file_count, occurrence_count
      FROM meta.main.content_terms
      ORDER BY occurrence_count DESC, file_count DESC
      LIMIT 20
    `),
    outputsByKind: await query(conn, `
      SELECT kind, count(*) AS file_count, CAST(sum(bytes) AS DOUBLE) AS bytes
      FROM meta.main.build_outputs
      GROUP BY kind
      ORDER BY bytes DESC
    `),
    outputsByExtension: await query(conn, `
      SELECT kind, extension, count(*) AS file_count, CAST(sum(bytes) AS DOUBLE) AS bytes
      FROM meta.main.build_outputs
      GROUP BY kind, extension
      ORDER BY kind, bytes DESC
    `),
    builds: await query(conn, `
      SELECT started_at, finished_at, duration_ms, status
      FROM meta.main.build_runs
      ORDER BY started_at
    `),
    buildRuns: await query(conn, `
      SELECT started_at, finished_at, duration_ms, status, zorto_version, command
      FROM meta.main.build_runs
      ORDER BY finished_at DESC
      LIMIT 2
    `),
    commits: await query(conn, `
      SELECT short_sha, committed_at, subject, file_count, additions, deletions
      FROM meta.main.commits
      ORDER BY committed_at DESC
      LIMIT 10
    `),
    content: await query(conn, `
      SELECT path, kind, title, word_count, bytes, last_commit_at
      FROM meta.main.content_files
      WHERE kind IN ('docs', 'content')
      ORDER BY word_count DESC, bytes DESC
      LIMIT 14
    `),
    links: await query(conn, `
      SELECT source_path, target, target_path, link_kind, target_exists
      FROM meta.main.content_links
      ORDER BY target_exists ASC NULLS LAST, source_path
      LIMIT 14
    `),
    outputs: await query(conn, `
      SELECT path, kind, extension, CAST(bytes AS DOUBLE) AS bytes
      FROM meta.main.build_outputs
      ORDER BY bytes DESC
      LIMIT 14
    `),
    packages: await query(conn, `
      SELECT ecosystem, name, version, manifest_path
      FROM meta.main.packages
      ORDER BY ecosystem, name
    `),
    dbTables: await query(conn, tableCountsSql)
  };
}

async function first(conn, sql) {
  const rows = await query(conn, sql);
  return rows[0] || {};
}

async function query(conn, sql) {
  const table = await conn.query(sql);
  const rows = [];
  for (const row of table) rows.push(coerceRow(row && row.toJSON ? row.toJSON() : row));
  return rows;
}

function renderDashboard(page, data, conn) {
  setupTabs(page);
  renderKpis(page.querySelector('[data-analytics-kpis]'), data);
  renderSignals(page.querySelector('[data-analytics-signals]'), data);
  renderCommitActivity(page.querySelector('[data-chart="commit-activity"]'), data.daily);
  renderBuildDuration(page.querySelector('[data-chart="build-duration"]'), data.builds);
  renderContentFreshness(page.querySelector('[data-chart="content-freshness"]'), data.freshness);
  renderTopTerms(page.querySelector('[data-chart="top-terms"]'), data.terms);
  renderOutputFootprint(page.querySelector('[data-chart="output-footprint"]'), data.outputsByExtension);
  renderOutputMix(page.querySelector('[data-chart="output-mix"]'), data.outputsByKind);
  renderCoreTables(page, data);
  setupQueryExplorer(page, conn);
  resizeVisibleCharts(page);
}

function setupTabs(page) {
  if (page.dataset.analyticsTabsReady === 'true') return;
  page.dataset.analyticsTabsReady = 'true';
  page.querySelectorAll('[data-view-toggle]').forEach((button) => {
    button.addEventListener('click', () => {
      const viewName = button.getAttribute('data-view-toggle');
      page.querySelectorAll('[data-view-toggle]').forEach((tab) => {
        tab.setAttribute('aria-pressed', String(tab === button));
      });
      page.querySelectorAll('[data-view]').forEach((view) => {
        view.hidden = view.getAttribute('data-view') !== viewName;
      });
      resizeVisibleCharts(page);
    });
  });
}

function setupQueryExplorer(page, conn) {
  const presets = page.querySelector('[data-query-presets]');
  const editor = page.querySelector('[data-query-editor]');
  const runButton = page.querySelector('[data-query-run]');
  const status = page.querySelector('[data-query-status]');

  if (presets.dataset.ready !== 'true') {
    presets.innerHTML = savedQueries.map((queryDef, index) => (
      '<button class="analytics-chip" type="button" data-query-id="' + escapeHtml(queryDef.id) +
      '" aria-pressed="' + String(index === 0) + '">' + escapeHtml(queryDef.label) + '</button>'
    )).join('');
    presets.addEventListener('click', (event) => {
      const button = event.target.closest('[data-query-id]');
      if (!button) return;
      const queryDef = savedQueries.find((candidate) => candidate.id === button.getAttribute('data-query-id'));
      if (!queryDef) return;
      editor.value = queryDef.sql.trim();
      presets.querySelectorAll('[data-query-id]').forEach((preset) => {
        preset.setAttribute('aria-pressed', String(preset === button));
      });
    });
    presets.dataset.ready = 'true';
  }

  if (!editor.value) editor.value = savedQueries[0].sql.trim();

  if (runButton.dataset.ready !== 'true') {
    runButton.addEventListener('click', async () => {
      status.textContent = 'running';
      runButton.disabled = true;
      try {
        const sql = validateReadOnlySql(editor.value);
        const rows = await query(conn, sql);
        renderQueryRows(page, rows.slice(0, 100));
        status.textContent = rows.length > 100 ? 'showing first 100 rows' : formatNumber(rows.length) + ' rows';
      } catch (error) {
        status.textContent = error && error.message ? error.message : String(error);
      } finally {
        runButton.disabled = false;
      }
    });
    runButton.dataset.ready = 'true';
  }

  runButton.click();
}

function renderKpis(mount, data) {
  const snapshot = data.snapshot || {};
  const totals = data.totals || {};
  const info = data.info || {};
  const cards = [
    ['Commits indexed', formatNumber(totals.commits), 'latest local git history'],
    ['Content files', formatNumber(totals.content_files), formatNumber(totals.words) + ' words'],
    ['Build output', formatBytes(totals.output_bytes), formatDuration(totals.latest_build_ms) + ' latest build'],
    ['Repo state', snapshot.dirty ? 'dirty' : 'clean', shortSha(snapshot.head_sha) + ' on ' + snapshot.branch],
    ['Terms indexed', formatNumber(totals.terms), 'search-shaped content signals'],
    ['Generated', shortDate(info.generated_at), 'schema v' + info.schema_version + ', DuckDB ' + info.duckdb_version]
  ];
  mount.innerHTML = cards.map(([label, value, note]) => (
    '<article class="analytics-kpi">' +
      '<span class="analytics-kpi__label">' + escapeHtml(label) + '</span>' +
      '<strong class="analytics-kpi__value">' + escapeHtml(value) + '</strong>' +
      '<span class="analytics-kpi__note">' + escapeHtml(note) + '</span>' +
    '</article>'
  )).join('');
}

function renderSignals(mount, data) {
  const snapshot = data.snapshot || {};
  const health = data.health || {};
  const builds = data.buildRuns || [];
  const latest = builds[0] || {};
  const previous = builds[1] || {};
  const delta = Number(latest.duration_ms || 0) - Number(previous.duration_ms || 0);
  const deltaText = previous.duration_ms ? signedDuration(delta) + ' vs previous run' : 'first tracked run';
  const linkText = Number(health.broken_links || 0) === 0 ? 'all checked links pass' : formatNumber(health.broken_links) + ' need attention';
  const dirtyText = snapshot.dirty ? formatNumber(snapshot.untracked_count) + ' untracked files' : 'working tree is clean';
  const cards = [
    ['Build pulse', formatDuration(latest.duration_ms), deltaText, delta <= 0 ? 'good' : 'hot'],
    ['Docs health', linkText, formatNumber(health.local_links) + ' local links checked', Number(health.broken_links || 0) ? 'watch' : 'good'],
    ['Working tree', snapshot.dirty ? 'in motion' : 'clean', dirtyText, snapshot.dirty ? 'hot' : 'good']
  ];
  mount.innerHTML = cards.map(([label, value, note, tone]) => (
    '<article class="analytics-signal analytics-signal--' + tone + '">' +
      '<span class="analytics-signal__label">' + escapeHtml(label) + '</span>' +
      '<strong class="analytics-signal__value">' + escapeHtml(value) + '</strong>' +
      '<span class="analytics-signal__note">' + escapeHtml(note) + '</span>' +
    '</article>'
  )).join('');
}

function renderCoreTables(page, data) {
  renderTable(page.querySelector('[data-table="db-tables"]'), data.dbTables, [
    (row) => '<code>' + escapeHtml(row.table_name) + '</code>',
    (row) => formatNumber(row.rows)
  ], 'No tables found.');
  renderTable(page.querySelector('[data-table="content-files"]'), data.content, [
    (row) => '<code>' + escapeHtml(row.path) + '</code>',
    (row) => escapeHtml(row.kind),
    (row) => formatNumber(row.word_count),
    (row) => escapeHtml(shortDate(row.last_commit_at))
  ], 'No content files found.');
  renderTable(page.querySelector('[data-table="links"]'), data.links, [
    (row) => '<code>' + escapeHtml(row.source_path) + '</code>',
    (row) => '<code>' + escapeHtml(row.target_path || row.target) + '</code>',
    (row) => row.target_exists === false ? 'missing' : 'ok'
  ], 'No local links found.');
  renderTable(page.querySelector('[data-table="outputs"]'), data.outputs, [
    (row) => '<code>' + escapeHtml(row.path) + '</code>',
    (row) => escapeHtml(row.kind),
    (row) => escapeHtml(row.extension),
    (row) => formatBytes(row.bytes)
  ], 'No generated outputs found.');
  renderTable(page.querySelector('[data-table="packages"]'), data.packages, [
    (row) => escapeHtml(row.name),
    (row) => '<code>' + escapeHtml(row.version) + '</code>',
    (row) => escapeHtml(row.ecosystem),
    (row) => '<code>' + escapeHtml(row.manifest_path) + '</code>'
  ], 'No packages found.');
}

function renderCommitActivity(el, rows) {
  Plotly.react(el, [
    {
      type: 'bar',
      name: 'commits',
      x: rows.map((row) => row.day),
      y: rows.map((row) => row.commit_count),
      marker: { color: palette[0] }
    },
    {
      type: 'scatter',
      mode: 'lines+markers',
      name: 'additions',
      x: rows.map((row) => row.day),
      y: rows.map((row) => row.additions),
      yaxis: 'y2',
      line: { color: palette[2], width: 2 }
    },
    {
      type: 'scatter',
      mode: 'lines',
      name: 'deletions',
      x: rows.map((row) => row.day),
      y: rows.map((row) => row.deletions),
      yaxis: 'y2',
      line: { color: palette[4], width: 2 }
    }
  ], withBaseLayout({
    yaxis: { title: 'commits' },
    yaxis2: { title: 'lines', overlaying: 'y', side: 'right', gridcolor: 'rgba(0,0,0,0)' }
  }), plotConfig());
}

function renderBuildDuration(el, rows) {
  Plotly.react(el, [{
    type: 'scatter',
    mode: 'lines+markers',
    name: 'duration',
    x: rows.map((row) => row.started_at),
    y: rows.map((row) => Number(row.duration_ms || 0) / 1000),
    line: { color: palette[1], width: 3 },
    marker: { size: 7 }
  }], withBaseLayout({
    yaxis: { title: 'seconds' }
  }), plotConfig());
}

function renderContentFreshness(el, rows) {
  const buckets = [...new Set(rows.map((row) => row.age_bucket))];
  const kinds = [...new Set(rows.map((row) => row.kind))];
  const traces = kinds.map((kind, index) => ({
    type: 'bar',
    name: kind,
    x: buckets,
    y: buckets.map((bucket) => {
      const row = rows.find((candidate) => candidate.kind === kind && candidate.age_bucket === bucket);
      return row ? row.file_count : 0;
    }),
    marker: { color: palette[index % palette.length] }
  }));
  Plotly.react(el, traces, withBaseLayout({
    barmode: 'stack',
    yaxis: { title: 'files' }
  }), plotConfig());
}

function renderTopTerms(el, rows) {
  const ordered = rows.slice().reverse();
  Plotly.react(el, [{
    type: 'bar',
    orientation: 'h',
    x: ordered.map((row) => row.occurrence_count),
    y: ordered.map((row) => row.term),
    customdata: ordered.map((row) => row.file_count),
    hovertemplate: '%{y}<br>%{x} occurrences<br>%{customdata} files<extra></extra>',
    marker: { color: palette[2] }
  }], withBaseLayout({
    margin: { t: 18, r: 24, b: 42, l: 120 },
    xaxis: { title: 'occurrences' }
  }), plotConfig());
}

function renderOutputFootprint(el, rows) {
  const totalsByKind = new Map();
  rows.forEach((row) => totalsByKind.set(row.kind, (totalsByKind.get(row.kind) || 0) + Number(row.bytes || 0)));
  const labels = ['site'];
  const parents = [''];
  const values = [sum(rows.map((row) => row.bytes))];
  for (const [kind, bytes] of totalsByKind.entries()) {
    labels.push(kind);
    parents.push('site');
    values.push(bytes);
  }
  rows.forEach((row) => {
    labels.push(row.kind + ' .' + row.extension);
    parents.push(row.kind);
    values.push(Number(row.bytes || 0));
  });
  Plotly.react(el, [{
    type: 'treemap',
    labels,
    parents,
    values,
    branchvalues: 'total',
    marker: { colors: labels.map((_, index) => palette[index % palette.length]) },
    hovertemplate: '%{label}<br>%{value:.2s} bytes<extra></extra>'
  }], withBaseLayout({
    margin: { t: 16, r: 8, b: 8, l: 8 }
  }), plotConfig());
}

function renderOutputMix(el, rows) {
  Plotly.react(el, [{
    type: 'pie',
    labels: rows.map((row) => row.kind),
    values: rows.map((row) => row.bytes),
    textinfo: 'label+percent',
    hole: 0.45,
    marker: { colors: palette }
  }], withBaseLayout({
    showlegend: false,
    margin: { t: 18, r: 10, b: 18, l: 10 }
  }), plotConfig());
}

function renderTable(tbody, rows, cells, emptyText) {
  if (!tbody) return;
  if (!rows.length) {
    tbody.innerHTML = '<tr><td class="analytics-table__empty" colspan="' + cells.length + '">' + escapeHtml(emptyText) + '</td></tr>';
    return;
  }
  tbody.innerHTML = rows.map((row) => (
    '<tr>' + cells.map((cell) => '<td>' + cell(row) + '</td>').join('') + '</tr>'
  )).join('');
}

function renderQueryRows(page, rows) {
  const head = page.querySelector('[data-query-head]');
  const body = page.querySelector('[data-query-body]');
  const columns = rows.length ? Object.keys(rows[0]) : [];
  head.innerHTML = columns.length ? '<tr>' + columns.map((column) => '<th>' + escapeHtml(column) + '</th>').join('') + '</tr>' : '';
  if (!rows.length) {
    body.innerHTML = '<tr><td class="analytics-table__empty">No rows returned.</td></tr>';
    return;
  }
  body.innerHTML = rows.map((row) => (
    '<tr>' + columns.map((column) => '<td>' + formatCell(row[column]) + '</td>').join('') + '</tr>'
  )).join('');
}

function validateReadOnlySql(sql) {
  const trimmed = sql.trim().replace(/;\s*$/, '');
  if (!trimmed) throw new Error('query is empty');
  if (/;\s*\S/.test(trimmed)) throw new Error('one statement at a time');
  if (!/^(select|with|show|describe|summarize|explain)\b/i.test(trimmed)) {
    throw new Error('readonly SELECT, WITH, SHOW, DESCRIBE, SUMMARIZE, or EXPLAIN only');
  }
  if (/\b(attach|copy|create|delete|detach|drop|export|import|insert|install|load|pragma|set|update)\b/i.test(trimmed)) {
    throw new Error('mutation and extension commands are disabled');
  }
  return trimmed;
}

function resizeVisibleCharts(page) {
  if (!window.Plotly) return;
  page.querySelectorAll('.analytics-view:not([hidden]) .analytics-chart').forEach((chart) => {
    if (chart.data) Plotly.Plots.resize(chart);
  });
}

function withBaseLayout(overrides) {
  const styles = getComputedStyle(document.documentElement);
  const color = styles.getPropertyValue('--color').trim() || '#e5e7eb';
  const muted = styles.getPropertyValue('--color-muted').trim() || '#94a3b8';
  const grid = 'rgba(167,139,250,0.16)';
  const base = {
    autosize: true,
    paper_bgcolor: 'rgba(0,0,0,0)',
    plot_bgcolor: 'rgba(0,0,0,0)',
    colorway: palette,
    font: { family: '"Public Sans", sans-serif', color, size: 12 },
    margin: { t: 28, r: 26, b: 54, l: 58 },
    xaxis: { gridcolor: grid, zerolinecolor: grid, tickfont: { color: muted } },
    yaxis: { gridcolor: grid, zerolinecolor: grid, tickfont: { color: muted } },
    legend: { orientation: 'h', x: 0, y: 1.16, bgcolor: 'rgba(0,0,0,0)' }
  };
  return mergeLayout(base, overrides || {});
}

function mergeLayout(base, overrides) {
  const out = Object.assign({}, base, overrides);
  out.font = Object.assign({}, base.font, overrides.font || {});
  out.margin = Object.assign({}, base.margin, overrides.margin || {});
  out.xaxis = Object.assign({}, base.xaxis, overrides.xaxis || {});
  out.yaxis = Object.assign({}, base.yaxis, overrides.yaxis || {});
  if (overrides.yaxis2) {
    out.yaxis2 = Object.assign({}, overrides.yaxis2);
  } else {
    delete out.yaxis2;
  }
  out.legend = Object.assign({}, base.legend, overrides.legend || {});
  return out;
}

function plotConfig() {
  return { responsive: true, displaylogo: false };
}

function coerceRow(row) {
  const out = {};
  Object.keys(row || {}).forEach((key) => {
    const value = row[key];
    if (isTemporalKey(key)) {
      out[key] = normalizeTemporal(value);
    } else {
      out[key] = typeof value === 'bigint' ? Number(value) : value;
    }
  });
  return out;
}

function isTemporalKey(key) {
  return key === 'day' || key.endsWith('_at');
}

function normalizeTemporal(value) {
  if (value == null || value === '') return '';
  if (value instanceof Date) return value.toISOString();
  if (typeof value === 'bigint') value = Number(value);
  if (typeof value === 'number' && Number.isFinite(value)) {
    const ms = value > 1e12 ? value : value > 1e9 ? value * 1000 : value * 86400000;
    return new Date(ms).toISOString();
  }
  return String(value);
}

function formatCell(value) {
  if (typeof value === 'number') return escapeHtml(Number.isInteger(value) ? formatNumber(value) : String(Number(value.toFixed(3))));
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  return '<code>' + escapeHtml(value) + '</code>';
}

function formatNumber(value) {
  return new Intl.NumberFormat('en-US').format(Number(value || 0));
}

function formatBytes(value) {
  const bytes = Number(value || 0);
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function formatDuration(ms) {
  const seconds = Number(ms || 0) / 1000;
  return seconds.toFixed(seconds >= 10 ? 0 : 1) + 's';
}

function signedDuration(ms) {
  const sign = Number(ms || 0) > 0 ? '+' : '';
  return sign + formatDuration(ms);
}

function shortDate(value) {
  if (!value) return '';
  return String(value).slice(0, 10);
}

function shortSha(value) {
  return value ? String(value).slice(0, 7) : '';
}

function sum(values) {
  return values.reduce((total, value) => total + Number(value || 0), 0);
}

function escapeHtml(value) {
  return String(value == null ? '' : value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
