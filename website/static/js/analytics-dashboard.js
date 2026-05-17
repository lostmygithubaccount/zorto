import {
  applyBasicManifest,
  attrSelector,
  defaultPalette as palette,
  escapeHtml,
  firstRow,
  formatBytes,
  formatDuration,
  formatNumber,
  labelFor,
  plotConfig,
  registerDataApp,
  renderConfiguredTable,
  resizeVisibleCharts,
  runManifestQueries,
  setupQueryExplorer,
  setupTabs,
  shortDate,
  shortSha,
  signedDuration,
  sum,
  withBaseLayout
} from './data-app-runtime.js';

const defaultQueryPresets = [
  {
    id: 'catalog',
    label: 'Catalog',
    sql: `
SELECT 'meta_info' AS table_name, count(*) AS rows FROM site.main.meta_info
UNION ALL SELECT 'repo_snapshot', count(*) FROM site.main.repo_snapshot
UNION ALL SELECT 'commits', count(*) FROM site.main.commits
UNION ALL SELECT 'commit_daily', count(*) FROM site.main.commit_daily
UNION ALL SELECT 'packages', count(*) FROM site.main.packages
UNION ALL SELECT 'content_files', count(*) FROM site.main.content_files
UNION ALL SELECT 'content_terms', count(*) FROM site.main.content_terms
UNION ALL SELECT 'content_links', count(*) FROM site.main.content_links
UNION ALL SELECT 'build_runs', count(*) FROM site.main.build_runs
UNION ALL SELECT 'build_outputs', count(*) FROM site.main.build_outputs
UNION ALL SELECT 'pipeline_steps', count(*) FROM site.main.pipeline_steps
ORDER BY rows DESC`
  }
];

const chartRenderers = {
  commitActivity: renderCommitActivity,
  buildDuration: renderBuildDuration,
  contentFreshness: renderContentFreshness,
  topTerms: renderTopTerms,
  outputFootprint: renderOutputFootprint,
  outputMix: renderOutputMix
};

registerDataApp({
  pageSelector: '[data-analytics-page]',
  loadSelector: '[data-analytics-load]',
  dashboardSelector: '[data-analytics-dashboard]',
  statusSelector: '[data-analytics-status]',
  defaultManifestUrl: '/data/analytics-dashboard.json',
  defaultDatabaseUrl: '/data/site.ddb',
  defaultDatabaseFile: 'site.ddb',
  defaultDatabaseSchema: 'site',
  defaultQueryPresets,
  loadedLabel: 'analytics loaded',
  applyManifest: applyBasicManifest,
  loadData: loadAnalyticsData,
  render: renderDashboard
});

async function loadAnalyticsData(conn, manifest) {
  const datasets = await runManifestQueries(conn, manifest);
  return {
    datasets,
    info: firstRow(datasets.info),
    snapshot: firstRow(datasets.snapshot),
    totals: firstRow(datasets.totals),
    health: firstRow(datasets.health),
    buildRuns: datasets.build_runs || []
  };
}

function renderDashboard(page, data, conn, manifest) {
  setupTabs(page);
  renderKpis(page.querySelector('[data-analytics-kpis]'), data, manifest);
  renderSignals(page.querySelector('[data-analytics-signals]'), data, manifest);
  renderCharts(page, data, manifest);
  renderTables(page, data, manifest);
  setupQueryExplorer(page, conn, manifest, defaultQueryPresets);
  resizeVisibleCharts(page);
}

function renderCharts(page, data, manifest) {
  (manifest.panels || []).forEach((panel) => {
    if (panel.kind !== 'chart') return;
    const renderer = chartRenderers[panel.renderer];
    const mount = page.querySelector('[data-chart="' + attrSelector(panel.id) + '"]');
    if (!renderer || !mount) return;
    renderer(mount, rowsForPanel(data, panel));
  });
}

function renderTables(page, data, manifest) {
  (manifest.panels || []).forEach((panel) => {
    if (panel.kind !== 'table') return;
    const head = page.querySelector('[data-table-head="' + attrSelector(panel.id) + '"]');
    const body = page.querySelector('[data-table="' + attrSelector(panel.id) + '"]');
    renderConfiguredTable(
      head,
      body,
      rowsForPanel(data, panel),
      panel.columns,
      panel.empty_state || 'No rows found.'
    );
  });
}

function rowsForPanel(data, panel) {
  return data.datasets[panel.query_id || panel.id] || [];
}

function renderKpis(mount, data, manifest) {
  const snapshot = data.snapshot || {};
  const totals = data.totals || {};
  const info = data.info || {};
  const label = (id, fallback) => labelFor(manifest.kpis, id, fallback);
  const cards = [
    [label('commits', 'Commits indexed'), formatNumber(totals.commits), 'latest local git history'],
    [label('content_files', 'Content files'), formatNumber(totals.content_files), formatNumber(totals.words) + ' words'],
    [label('build_output', 'Build output'), formatBytes(totals.output_bytes), formatDuration(totals.latest_build_ms) + ' latest build'],
    [label('repo_state', 'Repo state'), snapshot.dirty ? 'dirty' : 'clean', shortSha(snapshot.head_sha) + ' on ' + snapshot.branch],
    [label('terms', 'Terms indexed'), formatNumber(totals.terms), 'search-shaped content signals'],
    [label('generated', 'Generated'), shortDate(info.generated_at), 'schema v' + info.schema_version + ', DuckDB ' + info.duckdb_version]
  ];
  mount.innerHTML = cards.map(([labelText, value, note]) => (
    '<article class="analytics-kpi">' +
      '<span class="analytics-kpi__label">' + escapeHtml(labelText) + '</span>' +
      '<strong class="analytics-kpi__value">' + escapeHtml(value) + '</strong>' +
      '<span class="analytics-kpi__note">' + escapeHtml(note) + '</span>' +
    '</article>'
  )).join('');
}

function renderSignals(mount, data, manifest) {
  const snapshot = data.snapshot || {};
  const health = data.health || {};
  const builds = data.buildRuns || [];
  const label = (id, fallback) => labelFor(manifest.signals, id, fallback);
  const latest = builds[0] || {};
  const previous = builds[1] || {};
  const delta = Number(latest.duration_ms || 0) - Number(previous.duration_ms || 0);
  const deltaText = previous.duration_ms ? signedDuration(delta) + ' vs previous run' : 'first tracked run';
  const linkText = Number(health.broken_links || 0) === 0 ? 'all checked links pass' : formatNumber(health.broken_links) + ' need attention';
  const dirtyText = snapshot.dirty ? formatNumber(snapshot.untracked_count) + ' untracked files' : 'working tree is clean';
  const cards = [
    [label('build_pulse', 'Build pulse'), formatDuration(latest.duration_ms), deltaText, delta <= 0 ? 'good' : 'hot'],
    [label('docs_health', 'Docs health'), linkText, formatNumber(health.local_links) + ' local links checked', Number(health.broken_links || 0) ? 'watch' : 'good'],
    [label('working_tree', 'Working tree'), snapshot.dirty ? 'in motion' : 'clean', dirtyText, snapshot.dirty ? 'hot' : 'good']
  ];
  mount.innerHTML = cards.map(([labelText, value, note, tone]) => (
    '<article class="analytics-signal analytics-signal--' + tone + '">' +
      '<span class="analytics-signal__label">' + escapeHtml(labelText) + '</span>' +
      '<strong class="analytics-signal__value">' + escapeHtml(value) + '</strong>' +
      '<span class="analytics-signal__note">' + escapeHtml(note) + '</span>' +
    '</article>'
  )).join('');
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
