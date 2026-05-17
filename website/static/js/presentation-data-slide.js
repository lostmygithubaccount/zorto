import {
  attachDatabase,
  escapeHtml,
  formatBytes,
  formatDuration,
  formatNumber,
  loadDuckDB,
  loadPlotly,
  plotConfig,
  query,
  sqlIdentifier,
  validateReadOnlySql,
  withBaseLayout
} from './data-app-runtime.js';

const states = new WeakMap();
const queryStates = new WeakMap();

function manifestFor(app) {
  return {
    dashboard: {
      database_url: app.dataset.databaseUrl || '/data/site.ddb',
      database_file: app.dataset.databaseFile || 'site.ddb',
      database_schema: app.dataset.databaseSchema || 'site'
    }
  };
}

function setStatus(app, text) {
  const status = app.querySelector('[data-deck-data-status]');
  if (status) status.textContent = text;
}

function setQueryStatus(app, text) {
  const status = app.querySelector('[data-deck-query-status]');
  if (status) status.textContent = text;
}

function setButton(app, text, disabled) {
  const button = app.querySelector('[data-deck-data-load]');
  if (!button) return;
  button.textContent = text;
  button.disabled = disabled;
}

async function loadDeckDataApp(app) {
  const existing = states.get(app);
  if (existing) return existing;

  const task = (async () => {
    const manifest = manifestFor(app);
    const schema = sqlIdentifier(manifest.dashboard.database_schema);

    setButton(app, 'Loading', true);
    setStatus(app, 'loading Plotly');
    const plotly = await loadPlotly(manifest);

    setStatus(app, 'loading DuckDB-Wasm');
    const duck = await loadDuckDB(manifest);

    setStatus(app, 'opening ' + manifest.dashboard.database_file);
    await attachDatabase(duck, manifest);

    setStatus(app, 'querying site data');
    const [totals] = await query(duck.conn, `
SELECT
  (SELECT count(*) FROM ${schema}.main.content_files) AS content_files,
  (SELECT CAST(coalesce(sum(bytes), 0) AS DOUBLE) FROM ${schema}.main.build_outputs) AS output_bytes,
  (SELECT duration_ms FROM ${schema}.main.build_runs ORDER BY finished_at DESC LIMIT 1) AS latest_build_ms
`);
    const outputs = await query(duck.conn, `
SELECT kind, count(*) AS file_count, CAST(coalesce(sum(bytes), 0) AS DOUBLE) AS bytes
FROM ${schema}.main.build_outputs
GROUP BY kind
ORDER BY bytes DESC
LIMIT 8
`);

    renderKpis(app, totals || {});
    renderChart(plotly, app, outputs);

    setButton(app, 'Loaded', true);
    setStatus(app, 'ready');
  })().catch((error) => {
    states.delete(app);
    console.error(error);
    setButton(app, 'Retry', false);
    setStatus(app, error && error.message ? error.message : String(error));
  });

  states.set(app, task);
  return task;
}

async function runDeckQueryApp(app) {
  const existing = queryStates.get(app);
  if (existing) return existing;

  const task = (async () => {
    const manifest = manifestFor(app);
    const editor = app.querySelector('[data-deck-query-editor]');
    const button = app.querySelector('[data-deck-query-run]');
    if (!editor) return;

    if (button) button.disabled = true;
    setQueryStatus(app, 'loading DuckDB-Wasm');
    const duck = await loadDuckDB(manifest);

    setQueryStatus(app, 'opening ' + manifest.dashboard.database_file);
    await attachDatabase(duck, manifest);

    setQueryStatus(app, 'running query');
    const rows = await query(duck.conn, validateReadOnlySql(editor.value));
    renderQueryTable(app, rows.slice(0, 12));
    setQueryStatus(app, rows.length > 12 ? 'showing first 12 rows' : rows.length + ' rows');
  })().catch((error) => {
    console.error(error);
    setQueryStatus(app, error && error.message ? error.message : String(error));
  }).finally(() => {
    const button = app.querySelector('[data-deck-query-run]');
    if (button) button.disabled = false;
    queryStates.delete(app);
  });

  queryStates.set(app, task);
  return task;
}

function renderQueryTable(app, rows) {
  const head = app.querySelector('[data-deck-query-head]');
  const body = app.querySelector('[data-deck-query-body]');
  if (!head || !body) return;

  const columns = rows.length ? Object.keys(rows[0]) : [];
  head.innerHTML = columns.length
    ? '<tr>' + columns.map((column) => '<th>' + escapeHtml(column) + '</th>').join('') + '</tr>'
    : '';
  if (!rows.length) {
    body.innerHTML = '<tr><td>No rows returned.</td></tr>';
    return;
  }
  body.innerHTML = rows.map((row) => (
    '<tr>' + columns.map((column) => '<td>' + escapeHtml(formatQueryCell(row[column])) + '</td>').join('') + '</tr>'
  )).join('');
}

function formatQueryCell(value) {
  if (value == null) return '';
  if (typeof value === 'number') return Number.isInteger(value) ? formatNumber(value) : String(Number(value.toFixed(2)));
  if (typeof value === 'bigint') return value.toString();
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  return String(value);
}

function renderKpis(app, totals) {
  const mount = app.querySelector('[data-deck-data-kpis]');
  if (!mount) return;
  const cells = [
    ['content files', formatNumber(totals.content_files)],
    ['build output', formatBytes(totals.output_bytes)],
    ['latest build', formatDuration(totals.latest_build_ms)]
  ];
  mount.innerHTML = cells.map(([label, value]) => (
    '<span>' + escapeHtml(label) + '</span><strong>' + escapeHtml(value) + '</strong>'
  )).join('');
}

function renderChart(plotly, app, rows) {
  const chart = app.querySelector('[data-deck-data-chart]');
  if (!chart) return;
  plotly.react(chart, [{
    type: 'bar',
    orientation: 'h',
    x: rows.map((row) => row.bytes),
    y: rows.map((row) => row.kind),
    customdata: rows.map((row) => row.file_count),
    marker: { color: 'rgb(61, 219, 217)' },
    hovertemplate: '%{y}<br>%{x:.2s} bytes<br>%{customdata} files<extra></extra>'
  }], withBaseLayout({
    title: { text: 'Generated output by kind', font: { size: 15 } },
    margin: { t: 42, r: 22, b: 42, l: 118 },
    xaxis: { title: 'bytes' },
    yaxis: { automargin: true }
  }), plotConfig());
}

function initDeckDataApps() {
  document.querySelectorAll('[data-deck-data-app]').forEach((app) => {
    const button = app.querySelector('[data-deck-data-load]');
    if (button && button.dataset.bound !== 'true') {
      button.dataset.bound = 'true';
      button.addEventListener('click', () => loadDeckDataApp(app));
    }
  });
  document.querySelectorAll('[data-deck-query-app]').forEach((app) => {
    const button = app.querySelector('[data-deck-query-run]');
    if (button && button.dataset.bound !== 'true') {
      button.dataset.bound = 'true';
      button.addEventListener('click', () => runDeckQueryApp(app));
    }
  });
  loadActiveDeckDataApps();
}

function loadActiveDeckDataApps() {
  document.querySelectorAll('.z-slide.is-active [data-deck-data-app]').forEach((app) => {
    loadDeckDataApp(app);
  });
  document.querySelectorAll('.z-slide.is-active [data-deck-query-app]').forEach((app) => {
    runDeckQueryApp(app);
  });
}

window.addEventListener('zorto:slidechange', loadActiveDeckDataApps);
window.addEventListener('resize', () => {
  if (!window.Plotly) return;
  document.querySelectorAll('.z-slide.is-active [data-deck-data-chart]').forEach((chart) => {
    if (chart.data) window.Plotly.Plots.resize(chart);
  });
});

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initDeckDataApps);
} else {
  initDeckDataApps();
}
