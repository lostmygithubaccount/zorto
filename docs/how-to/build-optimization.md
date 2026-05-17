# Optimize builds

Speed up builds for large sites and control what Zorto generates.

## Skip executable code blocks with `--no-exec`

Executable code blocks run during every build by default. For sites with many code blocks, this can be slow. Skip execution with the `--no-exec` flag:

```bash
zorto --no-exec build
zorto --no-exec preview
```

`--no-exec` is a global flag (before the subcommand). Code blocks render as static syntax-highlighted code without execution output. This is useful for:

- Fast iteration on templates and styles
- Building on CI when code execution is not needed
- Working with untrusted content

## Work with drafts

Pages with `draft = true` in frontmatter are excluded from production builds:

```toml
+++
title = "Work in progress"
draft = true
+++
```

Include drafts during development with the `--drafts` flag:

```bash
zorto build --drafts
zorto preview --drafts
```

Drafts are excluded from sitemaps, feeds, `llms.txt`, and search data even when included in a preview build. Use drafts for content that is not ready for publication — remove the `draft = true` line when the page is ready.

## Compile all theme stylesheets

By default, Zorto compiles only the active theme's SCSS. Enable `compile_all_themes` to compile CSS for every built-in theme:

```toml
# config.toml
compile_all_themes = true
```

This generates `style-{name}.css` for each theme (e.g., `style-zorto.css`, `style-ember.css`) alongside the main `style.css`. Useful for:

- Theme preview or switcher pages
- Letting users choose a theme client-side

For most sites, leave this disabled — it adds build time without benefit.

## Cache executable code block results

If your site uses executable code blocks extensively, enable caching to avoid re-running unchanged blocks:

```toml
# config.toml
[cache]
enable = true
```

When enabled, Zorto caches code block output and reuses it if the code has not changed. This significantly speeds up rebuilds for sites with many executable code blocks.

## Large site considerations

### Content organization

For sites with hundreds of pages, structure content into sections. Zorto processes content in parallel where possible, so deeply nested structures do not create bottlenecks.

### SCSS compilation

Zorto compiles SCSS to CSS on every build. Keep stylesheets modular — import only what you need. Avoid importing large external CSS frameworks directly into your SCSS; place them in `static/` instead and load them via the `extra_head` block.

### External content directories

When using `content_dirs` to pull in external content, be aware that each directory is walked and parsed on every build. For very large external directories, use the `exclude` field to skip files that should not be processed:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
exclude = ["internal.md", "draft-notes.md"]
```

### Sitemap and feed generation

Sitemap and feed generation scale linearly with the number of pages. For very large sites (thousands of pages), you can disable them if not needed:

```toml
generate_sitemap = false
generate_feed = false
generate_llms_txt = false
```

### Search data

DuckDB-backed search ships a `.ddb` file containing searchable page content. For large sites, keep the `search_pages` table lean and avoid indexing private or generated content that visitors do not need.

## Build output summary

After a successful build, Zorto reports timing and output statistics. Use this to identify slow builds:

```bash
zorto build
# Built 142 pages in 0.8s
```

If builds are slow, try:

1. `--no-exec` to skip code execution (usually the biggest time cost)
2. `[cache] enable = true` if you must keep execution
3. Disable unused generators (`generate_feed`, `generate_sitemap`, etc.)
4. Use `exclude` in `content_dirs` to skip unnecessary files

## Related guides

- [CLI reference](../reference/cli.md) — all flags and subcommands
- [Configuration reference](../reference/config.md) — complete config.toml options
- [Executable code blocks](../concepts/executable-code.md) — how code execution works
- [Deploy your site](deploy.md) — production build and hosting
