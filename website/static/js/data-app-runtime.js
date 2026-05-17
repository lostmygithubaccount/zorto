export const defaultPalette = [
  'rgb(167,139,250)',
  'rgb(34,211,238)',
  'rgb(52,211,153)',
  'rgb(251,191,36)',
  'rgb(248,113,113)',
  'rgb(96,165,250)',
  'rgb(244,114,182)'
];

const defaultOptions = {
  pageSelector: '[data-data-app-page]',
  loadSelector: '[data-data-app-load]',
  dashboardSelector: '[data-data-app-dashboard]',
  statusSelector: '[data-data-app-status]',
  defaultManifestUrl: '/data/app.json',
  defaultDuckdbModuleUrl: 'https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.33.1-dev45.0/+esm',
  defaultPlotlyUrl: 'https://cdn.jsdelivr.net/npm/plotly.js-dist-min@3.4.0/plotly.min.js',
  defaultDatabaseUrl: '/data/site.ddb',
  defaultDatabaseFile: 'site.ddb',
  defaultDatabaseSchema: 'site',
  loadedLabel: 'loaded',
  defaultQueryPresets: []
};

const manifestPromises = new Map();
const plotlyPromises = new Map();
const duckPromises = new Map();

export function registerDataApp(options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  document.querySelectorAll(merged.loadSelector).forEach((button) => {
    button.addEventListener('click', () => loadDataApp(button, merged));
  });
}

export async function loadDataApp(button, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  if (button.dataset.loading === 'true') return;

  const page = button.closest(merged.pageSelector);
  if (!page) throw new Error('data app page not found');

  const dashboard = page.querySelector(merged.dashboardSelector);
  const status = page.querySelector(merged.statusSelector);
  const setStatus = (text) => {
    if (status) status.textContent = text;
  };

  button.dataset.loading = 'true';
  button.disabled = true;
  setStatus('loading dashboard config');

  try {
    const manifest = await loadManifest(page, merged);
    if (merged.applyManifest) merged.applyManifest(page, manifest);

    setStatus('loading Plotly');
    await loadPlotly(manifest, merged);

    setStatus('loading DuckDB-Wasm');
    const duck = await loadDuckDB(manifest, merged);

    setStatus('opening ' + databaseFile(manifest, merged));
    await attachDatabase(duck, manifest, merged);

    setStatus('querying metadata');
    const data = merged.loadData
      ? await merged.loadData(duck.conn, manifest)
      : await runManifestQueries(duck.conn, manifest);

    if (dashboard) dashboard.hidden = false;
    if (merged.render) await merged.render(page, data, duck.conn, manifest);

    button.textContent = dashboardConfig(manifest).loaded_label || merged.loadedLabel;
    setStatus('ready');
  } catch (error) {
    console.error(error);
    button.disabled = false;
    setStatus('failed');
    if (dashboard) {
      dashboard.hidden = false;
      dashboard.innerHTML = '<div class="analytics-error"><strong>Analytics failed to load.</strong><p>' +
        escapeHtml(error && error.message ? error.message : String(error)) +
        '</p></div>';
    }
  } finally {
    button.dataset.loading = 'false';
  }
}

export async function loadManifest(page, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  const url = page.dataset.manifestUrl || merged.defaultManifestUrl;
  if (!manifestPromises.has(url)) {
    manifestPromises.set(url, fetch(url).then(async (response) => {
      if (!response.ok) {
        throw new Error('failed to fetch dashboard manifest (HTTP ' + response.status + ')');
      }
      return normalizeManifest(await response.json(), merged.defaultQueryPresets);
    }));
  }
  return manifestPromises.get(url);
}

export function normalizeManifest(manifest, defaultQueryPresets = []) {
  const normalized = manifest && typeof manifest === 'object' ? manifest : {};
  normalized.dashboard = Object.assign({}, normalized.dashboard || {});
  normalized.views = Array.isArray(normalized.views) ? normalized.views : [];
  normalized.panels = Array.isArray(normalized.panels) ? normalized.panels : [];
  normalized.kpis = Array.isArray(normalized.kpis) ? normalized.kpis : [];
  normalized.signals = Array.isArray(normalized.signals) ? normalized.signals : [];
  normalized.queries = Array.isArray(normalized.queries) ? normalized.queries : [];
  normalized.query_presets = Array.isArray(normalized.query_presets) && normalized.query_presets.length
    ? normalized.query_presets
    : defaultQueryPresets;
  return normalized;
}

export function applyBasicManifest(page, manifest) {
  (manifest.views || []).forEach((view) => {
    const button = page.querySelector('[data-view-toggle="' + attrSelector(view.id) + '"]');
    if (button && view.label) button.textContent = view.label;
  });

  (manifest.panels || []).forEach((panel) => {
    const section = page.querySelector('[data-panel="' + attrSelector(panel.id) + '"]');
    if (!section) return;
    const title = section.querySelector('.analytics-panel__header h2');
    const description = section.querySelector('.analytics-panel__header p');
    if (title && panel.title) title.textContent = panel.title;
    if (description) {
      description.textContent = panel.description || '';
      description.hidden = !panel.description;
    }
    section.classList.toggle('analytics-panel--wide', Boolean(panel.wide));
  });
}

export function dashboardConfig(manifest) {
  return manifest && manifest.dashboard ? manifest.dashboard : {};
}

export function queryPresets(manifest, defaultQueryPresets = []) {
  return manifest && Array.isArray(manifest.query_presets) && manifest.query_presets.length
    ? manifest.query_presets
    : defaultQueryPresets;
}

export function databaseUrl(manifest, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  return dashboardConfig(manifest).database_url || merged.defaultDatabaseUrl;
}

export function databaseFile(manifest, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  return dashboardConfig(manifest).database_file || merged.defaultDatabaseFile;
}

export function databaseSchema(manifest, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  return dashboardConfig(manifest).database_schema || merged.defaultDatabaseSchema;
}

export function loadPlotly(manifest, options = {}) {
  if (window.Plotly) return Promise.resolve(window.Plotly);
  const merged = Object.assign({}, defaultOptions, options);
  const url = dashboardConfig(manifest).plotly_url || merged.defaultPlotlyUrl;
  if (!plotlyPromises.has(url)) {
    plotlyPromises.set(url, new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = url;
      script.async = true;
      script.charset = 'utf-8';
      script.onload = () => {
        if (window.Plotly) {
          resolve(window.Plotly);
        } else {
          reject(new Error('Plotly loaded without exposing window.Plotly'));
        }
      };
      script.onerror = () => reject(new Error('failed to load Plotly from ' + url));
      document.head.appendChild(script);
    }));
  }
  return plotlyPromises.get(url);
}

export async function loadDuckDB(manifest, options = {}) {
  const merged = Object.assign({}, defaultOptions, options);
  const url = dashboardConfig(manifest).duckdb_module_url || merged.defaultDuckdbModuleUrl;
  if (!duckPromises.has(url)) {
    duckPromises.set(url, (async () => {
      const duckdb = await import(url);
      const bundle = await duckdb.selectBundle(duckdb.getJsDelivrBundles());
      const worker = await duckdb.createWorker(bundle.mainWorker);
      const db = new duckdb.AsyncDuckDB(new duckdb.VoidLogger(), worker);
      await db.instantiate(bundle.mainModule, bundle.pthreadWorker || null);
      const conn = await db.connect();
      return { duckdb, db, conn, attachedSchemas: new Set() };
    })());
  }
  return duckPromises.get(url);
}

export async function attachDatabase(duck, manifest, options = {}) {
  const dbUrl = databaseUrl(manifest, options);
  const dbFile = databaseFile(manifest, options);
  const dbSchema = databaseSchema(manifest, options);
  if (duck.attachedSchemas.has(dbSchema)) return;

  const response = await fetch(dbUrl);
  if (!response.ok) throw new Error('failed to fetch ' + dbUrl + ' (HTTP ' + response.status + ')');
  const bytes = new Uint8Array(await response.arrayBuffer());
  await duck.db.registerFileBuffer(dbFile, bytes);
  try {
    await duck.conn.query("ATTACH '" + dbFile.replace(/'/g, "''") + "' AS " + sqlIdentifier(dbSchema) + " (READ_ONLY)");
  } catch (error) {
    if (!/already exists/i.test(error && error.message ? error.message : String(error))) {
      throw error;
    }
  }
  duck.attachedSchemas.add(dbSchema);
}

export async function runManifestQueries(conn, manifest) {
  const results = {};
  for (const queryDef of manifest.queries || []) {
    if (!queryDef.id || !queryDef.sql) continue;
    results[queryDef.id] = await query(conn, queryDef.sql);
  }
  return results;
}

export async function first(conn, sql) {
  const rows = await query(conn, sql);
  return firstRow(rows);
}

export async function query(conn, sql) {
  const table = await conn.query(sql);
  const rows = [];
  for (const row of table) rows.push(coerceRow(row && row.toJSON ? row.toJSON() : row));
  return rows;
}

export function firstRow(rows) {
  return Array.isArray(rows) && rows.length ? rows[0] : {};
}

export function setupTabs(page) {
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

export function setupQueryExplorer(page, conn, manifest, defaultQueryPresets = []) {
  const presets = page.querySelector('[data-query-presets]');
  const editor = page.querySelector('[data-query-editor]');
  const runButton = page.querySelector('[data-query-run]');
  const status = page.querySelector('[data-query-status]');
  const savedQueries = queryPresets(manifest, defaultQueryPresets);
  if (!presets || !editor || !runButton || !status || !savedQueries.length) return;

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

export function renderConfiguredTable(head, body, rows, columns, emptyText = 'No rows found.') {
  if (!body) return;
  const safeColumns = Array.isArray(columns) && columns.length
    ? columns
    : inferColumns(rows);
  if (head) {
    head.innerHTML = '<tr>' + safeColumns.map((column) => (
      '<th>' + escapeHtml(column.label || column.field || '') + '</th>'
    )).join('') + '</tr>';
  }
  if (!rows.length) {
    body.innerHTML = '<tr><td class="analytics-table__empty" colspan="' + safeColumns.length + '">' +
      escapeHtml(emptyText) + '</td></tr>';
    return;
  }
  body.innerHTML = rows.map((row) => (
    '<tr>' + safeColumns.map((column) => (
      '<td>' + formatColumnValue(valueForColumn(row, column), column) + '</td>'
    )).join('') + '</tr>'
  )).join('');
}

export function renderQueryRows(page, rows) {
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

export function validateReadOnlySql(sql) {
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

export function resizeVisibleCharts(page) {
  if (!window.Plotly) return;
  page.querySelectorAll('.analytics-view:not([hidden]) .analytics-chart').forEach((chart) => {
    if (chart.data) Plotly.Plots.resize(chart);
  });
}

export function withBaseLayout(overrides) {
  const styles = getComputedStyle(document.documentElement);
  const color = styles.getPropertyValue('--color').trim() || '#e5e7eb';
  const muted = styles.getPropertyValue('--color-muted').trim() || '#94a3b8';
  const grid = 'rgba(167,139,250,0.16)';
  const base = {
    autosize: true,
    paper_bgcolor: 'rgba(0,0,0,0)',
    plot_bgcolor: 'rgba(0,0,0,0)',
    colorway: defaultPalette,
    font: { family: '"Public Sans", sans-serif', color, size: 12 },
    margin: { t: 28, r: 26, b: 54, l: 58 },
    xaxis: { gridcolor: grid, zerolinecolor: grid, tickfont: { color: muted } },
    yaxis: { gridcolor: grid, zerolinecolor: grid, tickfont: { color: muted } },
    legend: { orientation: 'h', x: 0, y: 1.16, bgcolor: 'rgba(0,0,0,0)' }
  };
  return mergeLayout(base, overrides || {});
}

export function plotConfig() {
  return { responsive: true, displaylogo: false };
}

export function coerceRow(row) {
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

export function formatCell(value) {
  if (typeof value === 'number') {
    return escapeHtml(Number.isInteger(value) ? formatNumber(value) : String(Number(value.toFixed(3))));
  }
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  return '<code>' + escapeHtml(value) + '</code>';
}

export function formatNumber(value) {
  return new Intl.NumberFormat('en-US').format(Number(value || 0));
}

export function formatBytes(value) {
  const bytes = Number(value || 0);
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

export function formatDuration(ms) {
  const seconds = Number(ms || 0) / 1000;
  return seconds.toFixed(seconds >= 10 ? 0 : 1) + 's';
}

export function signedDuration(ms) {
  const sign = Number(ms || 0) > 0 ? '+' : '';
  return sign + formatDuration(ms);
}

export function shortDate(value) {
  if (!value) return '';
  return String(value).slice(0, 10);
}

export function shortSha(value) {
  return value ? String(value).slice(0, 7) : '';
}

export function sum(values) {
  return values.reduce((total, value) => total + Number(value || 0), 0);
}

export function labelFor(items, id, fallback) {
  const item = (items || []).find((candidate) => candidate.id === id);
  return item && item.label ? item.label : fallback;
}

export function attrSelector(value) {
  return String(value == null ? '' : value).replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

export function sqlIdentifier(value) {
  const name = String(value || '');
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) throw new Error('invalid DuckDB schema name in dashboard manifest');
  return name;
}

export function escapeHtml(value) {
  return String(value == null ? '' : value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function inferColumns(rows) {
  if (!Array.isArray(rows) || !rows.length) return [];
  return Object.keys(rows[0]).map((field) => ({ field, label: field }));
}

function valueForColumn(row, column) {
  const value = row[column.field];
  if ((value == null || value === '') && column.fallback_field) {
    return row[column.fallback_field];
  }
  return value;
}

function formatColumnValue(value, column) {
  switch (column.format) {
    case 'bytes':
      return escapeHtml(formatBytes(value));
    case 'code':
      return '<code>' + escapeHtml(value) + '</code>';
    case 'date':
      return escapeHtml(shortDate(value));
    case 'duration':
      return escapeHtml(formatDuration(value));
    case 'number':
      return escapeHtml(formatNumber(value));
    case 'ok_missing':
      return value === false ? 'missing' : 'ok';
    default:
      if (typeof value === 'number') return escapeHtml(formatNumber(value));
      if (typeof value === 'boolean') return value ? 'true' : 'false';
      return escapeHtml(value);
  }
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
