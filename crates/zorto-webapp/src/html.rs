//! Page shell, CSS, and navigation for the webapp.

use crate::escape;

const CSS: &str = r#"
* { margin: 0; padding: 0; box-sizing: border-box; }
html { background: #111118; }
body { font-family: system-ui, -apple-system, sans-serif; background: #111118; color: #c8c8d8; min-height: 100vh; display: flex; }
a { color: #60a5fa; text-decoration: none; }
a:hover { text-decoration: underline; }

/* Sidebar */
.sidebar { width: 220px; background: #16161f; border-right: 1px solid #2a2a3a; padding: 24px 16px; flex-shrink: 0; min-height: 100vh; display: flex; flex-direction: column; }
.sidebar h1 { font-size: 1.1rem; color: #60a5fa; margin-bottom: 4px; font-weight: 600; }
.sidebar .site-title { font-size: 0.8rem; color: #666680; margin-bottom: 24px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.sidebar nav a { display: block; padding: 8px 12px; border-radius: 6px; color: #8c8ca6; font-size: 0.9rem; margin-bottom: 2px; }
.sidebar nav a:hover { background: #1e1e2e; color: #c8c8d8; text-decoration: none; }
.sidebar nav a.active { background: #1e1e2e; color: #60a5fa; }
.sidebar-bottom { margin-top: auto; padding-top: 24px; }

/* Mobile header (hidden by default) */
.mobile-header { display: none; background: #16161f; border-bottom: 1px solid #2a2a3a; padding: 12px 16px; align-items: center; justify-content: space-between; position: sticky; top: 0; z-index: 100; }
.mobile-header h1 { font-size: 1rem; color: #60a5fa; font-weight: 600; }
.mobile-toggle { background: none; border: 1px solid #2a2a3a; color: #8c8ca6; padding: 6px 10px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; }

/* Main */
.main { flex: 1; padding: 32px 40px; max-width: 960px; min-width: 0; }
.main h2 { font-size: 1.3rem; color: #e0e0f0; margin-bottom: 20px; font-weight: 500; }

/* Cards */
.card { background: #16161f; border: 1px solid #2a2a3a; border-radius: 8px; padding: 20px; margin-bottom: 16px; }
.card h3 { font-size: 1rem; color: #c8c8d8; margin-bottom: 8px; }
.stat { display: inline-block; margin-right: 24px; }
.stat-num { font-size: 1.5rem; color: #60a5fa; font-weight: 600; }
.stat-label { font-size: 0.8rem; color: #666680; text-transform: uppercase; letter-spacing: 0.05em; }

/* Tables */
table { width: 100%; border-collapse: collapse; }
th { text-align: left; font-size: 0.75rem; color: #666680; text-transform: uppercase; letter-spacing: 0.05em; padding: 8px 12px; border-bottom: 1px solid #2a2a3a; }
td { padding: 8px 12px; border-bottom: 1px solid #1e1e2e; font-size: 0.9rem; }
tr:hover { background: #1a1a26; }

/* Buttons */
.btn { display: inline-block; background: #1e1e2e; border: 1px solid #2a2a3a; color: #c8c8d8; padding: 8px 16px; border-radius: 6px; cursor: pointer; font-size: 0.85rem; font-family: inherit; text-decoration: none; }
.btn:hover { border-color: #60a5fa; color: #60a5fa; text-decoration: none; }
.btn-primary { background: #1e3a5f; border-color: #60a5fa; color: #60a5fa; }
.btn-primary:hover { background: #264d7a; }
.btn-success { background: #1a3a2a; border-color: #34d399; color: #34d399; }
.btn-success:hover { background: #1f4a35; }
.btn-danger { background: #3a1a1a; border-color: #f87171; color: #f87171; }
.btn-danger:hover { background: #4a2020; }
.btn-sm { padding: 4px 10px; font-size: 0.8rem; }

/* Forms */
textarea { width: 100%; background: #111118; border: 1px solid #2a2a3a; border-radius: 6px; color: #c8c8d8; padding: 12px; font-family: 'SF Mono', 'Fira Code', monospace; font-size: 0.85rem; resize: vertical; }
textarea:focus { outline: none; border-color: #60a5fa; }
input[type="text"], input[type="date"], select { background: #111118; border: 1px solid #2a2a3a; border-radius: 6px; color: #c8c8d8; padding: 8px 12px; font-size: 0.85rem; width: 100%; font-family: inherit; }
input:focus, select:focus { outline: none; border-color: #60a5fa; }
label { display: block; font-size: 0.8rem; color: #8c8ca6; margin-bottom: 4px; text-transform: uppercase; letter-spacing: 0.05em; }
.form-group { margin-bottom: 16px; }
.form-row { display: flex; gap: 16px; }
.form-row > * { flex: 1; min-width: 0; }

/* Toolbar */
.toolbar { display: flex; gap: 8px; align-items: center; margin-bottom: 20px; flex-wrap: wrap; }
.toolbar-right { margin-left: auto; display: flex; gap: 8px; align-items: center; }

/* Badges */
.badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 0.75rem; }
.badge-draft { background: #3a2a1a; color: #fbbf24; }
.badge-section { background: #1a2a3a; color: #60a5fa; }

/* Flash messages */
.flash { padding: 12px 16px; border-radius: 6px; margin-bottom: 16px; font-size: 0.85rem; }
.flash-success { background: #1a3a2a; border: 1px solid #34d399; color: #34d399; }
.flash-error { background: #3a1a1a; border: 1px solid #f87171; color: #f87171; }

/* Inline form errors */
.field-error { color: #f87171; font-size: 0.8rem; margin-top: 4px; }
.form-group.has-error input, .form-group.has-error textarea, .form-group.has-error select { border-color: #f87171; }

/* Confirmation dialog */
.confirm-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: center; justify-content: center; z-index: 1000; }
.confirm-dialog { background: #16161f; border: 1px solid #2a2a3a; border-radius: 10px; padding: 24px; max-width: 400px; width: 90%; text-align: center; }
.confirm-dialog h3 { color: #e0e0f0; margin-bottom: 8px; font-size: 1.1rem; }
.confirm-dialog p { color: #8c8ca6; font-size: 0.85rem; margin-bottom: 20px; }
.confirm-dialog .confirm-actions { display: flex; gap: 8px; justify-content: center; }

/* Empty state */
.empty-state { text-align: center; padding: 40px 20px; color: #666680; }
.empty-state p { font-size: 0.95rem; margin-bottom: 16px; }

/* Preview panel */
.preview-panel { background: #1a1a26; border: 1px solid #2a2a3a; border-radius: 6px; padding: 16px; min-height: 200px; }
.preview-panel h1, .preview-panel h2, .preview-panel h3, .preview-panel h4 { color: #e0e0f0; margin: 1em 0 0.5em; }
.preview-panel h1 { font-size: 1.5rem; }
.preview-panel h2 { font-size: 1.25rem; }
.preview-panel h3 { font-size: 1.1rem; }
.preview-panel p { margin: 0.5em 0; line-height: 1.6; }
.preview-panel ul, .preview-panel ol { margin: 0.5em 0 0.5em 1.5em; }
.preview-panel li { margin: 0.25em 0; }
.preview-panel code { background: #242438; padding: 2px 6px; border-radius: 3px; font-size: 0.85em; }
.preview-panel pre { background: #0d0d14; border: 1px solid #2a2a3a; border-radius: 6px; padding: 12px; overflow-x: auto; margin: 0.5em 0; }
.preview-panel pre code { background: none; padding: 0; }
.preview-panel blockquote { border-left: 3px solid #60a5fa; padding-left: 12px; color: #8c8ca6; margin: 0.5em 0; }
.preview-panel img { max-width: 100%; border-radius: 6px; }
.preview-panel a { color: #60a5fa; }
.preview-panel table { width: 100%; border-collapse: collapse; margin: 0.5em 0; }
.preview-panel th, .preview-panel td { border: 1px solid #2a2a3a; padding: 6px 10px; text-align: left; }
.preview-panel th { background: #1e1e2e; }

/* Editor layout */
.editor-layout { display: flex; gap: 20px; }
.editor-layout > .editor-pane { flex: 1; min-width: 0; }

/* File list */
.file-list { list-style: none; }
.file-list li { padding: 6px 12px; border-bottom: 1px solid #1e1e2e; font-size: 0.85rem; display: flex; align-items: center; gap: 8px; }
.file-list li:hover { background: #1a1a26; }
.file-icon { color: #666680; }

/* Asset grid */
.asset-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 12px; }
.asset-card { background: #16161f; border: 1px solid #2a2a3a; border-radius: 8px; padding: 12px; text-align: center; position: relative; }
.asset-card:hover { border-color: #3a3a5a; }
.asset-thumb { width: 100%; height: 100px; object-fit: contain; border-radius: 4px; margin-bottom: 8px; background: #111118; }
.asset-thumb-placeholder { width: 100%; height: 100px; display: flex; align-items: center; justify-content: center; background: #111118; border-radius: 4px; margin-bottom: 8px; font-size: 2rem; color: #444460; }
.asset-name { font-size: 0.75rem; color: #8c8ca6; word-break: break-all; }
.asset-size { font-size: 0.7rem; color: #444460; margin-top: 4px; }
.asset-delete { position: absolute; top: 6px; right: 6px; background: #2a1a1a; border: 1px solid #5c2a2a; color: #f87171; width: 24px; height: 24px; border-radius: 4px; cursor: pointer; font-size: 0.75rem; display: flex; align-items: center; justify-content: center; opacity: 0; transition: opacity 0.15s; }
.asset-card:hover .asset-delete { opacity: 1; }

/* Upload drop zone */
.drop-zone { border: 2px dashed #2a2a3a; border-radius: 8px; padding: 32px; text-align: center; color: #666680; transition: border-color 0.15s, background 0.15s; cursor: pointer; }
.drop-zone:hover, .drop-zone.drag-over { border-color: #60a5fa; background: #1a1a26; color: #8c8ca6; }

/* Recent pages */
.recent-list { list-style: none; }
.recent-list li { padding: 8px 0; border-bottom: 1px solid #1e1e2e; display: flex; justify-content: space-between; align-items: center; }
.recent-list li:last-child { border-bottom: none; }

/* Responsive */
@media (max-width: 768px) {
  body { flex-direction: column; }
  .sidebar { display: none; width: 100%; min-height: auto; border-right: none; border-bottom: 1px solid #2a2a3a; }
  .sidebar.open { display: flex; }
  .mobile-header { display: flex; }
  .main { padding: 16px; }
  .main h2 { font-size: 1.1rem; }
  .form-row { flex-direction: column; gap: 0; }
  .editor-layout { flex-direction: column; }
  .toolbar { flex-direction: column; align-items: flex-start; }
  .toolbar-right { margin-left: 0; }
  .stat { margin-right: 16px; margin-bottom: 8px; }
  .stat-num { font-size: 1.2rem; }
  table { font-size: 0.8rem; }
  th, td { padding: 6px 8px; }
  .asset-grid { grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); }
}
"#;

pub fn page(title: &str, site_title: &str, active: &str, body: &str, base_url: &str) -> String {
    let nav_items = [
        ("dashboard", "/", "Dashboard"),
        ("pages", "/pages", "Pages"),
        ("sections", "/sections", "Sections"),
        ("config", "/config", "Config"),
        ("assets", "/assets", "Assets"),
    ];

    let nav_html: String = nav_items
        .iter()
        .map(|(id, href, label)| {
            let class = if *id == active { " active" } else { "" };
            format!(r#"<a href="{href}" class="{class}">{label}</a>"#)
        })
        .collect::<Vec<_>>()
        .join("\n      ");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title} — zorto</title>
  <script src="/static/htmx.min.js"></script>
  <style>{CSS}</style>
</head>
<body>
  <div class="mobile-header">
    <h1>zorto</h1>
    <button class="mobile-toggle" onclick="document.querySelector('.sidebar').classList.toggle('open')">Menu</button>
  </div>
  <div class="sidebar">
    <h1>zorto</h1>
    <div class="site-title">{site_title}</div>
    <nav>
      {nav_html}
    </nav>
    <div class="sidebar-bottom">
      <a href="{base_url}" target="_blank" class="btn" style="width: 100%; text-align: center; margin-bottom: 8px;">View Site</a>
      <form method="POST" action="/build">
        <button type="submit" class="btn btn-success" style="width: 100%;"
                hx-post="/build" hx-target="#build-status" hx-swap="innerHTML">
          Build Site
        </button>
      </form>
      <div id="build-status" style="margin-top: 8px; font-size: 0.8rem; color: #666680;"></div>
    </div>
  </div>
  <div class="main">
    {body}
  </div>
  <script>
  document.querySelectorAll('.sidebar nav a').forEach(function(a) {{
    a.addEventListener('click', function() {{
      document.querySelector('.sidebar').classList.remove('open');
    }});
  }});
  </script>
</body>
</html>"##,
        title = escape(title),
        site_title = escape(site_title),
        base_url = escape(base_url),
    )
}
