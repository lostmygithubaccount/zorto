# Build a docs site

Create a documentation site with organized sections, sidebar navigation, and optional external content sources using `content_dirs`.

{% tree(caption="What your docs site will look like after this guide.") %}
my-docs/
  config.toml  [site config]
  content/
    _index.md  [section: homepage]
    guide/
      _index.md  [section: getting started]
      installation.md  [page]
      configuration.md  [page]
    reference/
      _index.md  [section: API reference]
      cli.md  [page]
      config.md  [page]
  templates/  [optional overrides]
  static/  [images, fonts]
{% end %}

## Initialize the project

Scaffold a new docs site with the `docs` template:

```bash
zorto init --template docs my-docs
cd my-docs
```

This creates a project with a `guide/` section and example pages sorted by title. Start the dev server to see it:

```bash
zorto preview --open
```

## Add sections

Each section is a directory with an `_index.md` file. Add a reference section:

```bash
mkdir -p content/reference
```

Create `content/reference/_index.md`:

```toml
+++
title = "Reference"
sort_by = "title"
+++
```

Then add pages inside it. Create `content/reference/cli.md`:

```toml
+++
title = "CLI reference"
+++
```

```markdown
## Commands

| Command | Description |
|---------|-------------|
| `build` | Build the site to `public/` |
| `preview` | Start dev server with live reload |
| `check` | Validate links and config |
| `clean` | Remove build output |
```

Repeat for each page in the section. Zorto generates URLs from the file structure: `content/reference/cli.md` becomes `/reference/cli/`.

## Nest sections

Sections can nest to any depth. Create subsections by adding directories with their own `_index.md`:

```bash
mkdir -p content/guide/advanced
```

Create `content/guide/advanced/_index.md`:

```toml
+++
title = "Advanced"
sort_by = "title"
+++
```

Pages inside `content/guide/advanced/` appear under `/guide/advanced/`. Access subsections in templates via `section.subsections`.

{% tree(caption="Nested sections create a hierarchical URL structure.") %}
content/guide/
  _index.md  [section: /guide/]
  installation.md  [page: /guide/installation/]
  advanced/
    _index.md  [section: /guide/advanced/]
    caching.md  [page: /guide/advanced/caching/]
{% end %}

## Sort pages

Control page order with `sort_by` in the section's `_index.md`:

- `sort_by = "title"` — alphabetical by title (good for reference docs)
- `sort_by = "date"` — newest first (good for changelogs)

For manual ordering, prefix filenames with numbers: `01-installation.md`, `02-configuration.md`. Zorto includes the full filename in the title slug, so the numbers appear in URLs too. To keep clean URLs, set a custom `slug` in each page's frontmatter:

```toml
+++
title = "Installation"
slug = "installation"
+++
```

## Use custom templates

Assign a custom template to your docs sections for a different layout than the rest of your site:

```toml
# content/guide/_index.md
+++
title = "Guide"
sort_by = "title"
template = "docs-section.html"
+++
```

Individual pages can also use custom templates:

```toml
# content/reference/cli.md
+++
title = "CLI reference"
template = "docs-page.html"
+++
```

Create the corresponding template files in your `templates/` directory. They extend `base.html` like any other template.

## Pull in external docs with content_dirs

If your documentation lives outside the site directory (for example, alongside source code in a library repo), use `content_dirs` to pull it in without copying files:

```toml
# config.toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "docs.html"
section_template = "docs-section.html"
sort_by = "title"
rewrite_links = true
```

| Field | Description |
|-------|-------------|
| `path` | Relative path to the external directory |
| `url_prefix` | URL prefix for all generated pages (e.g. `"docs"` produces `/docs/...`) |
| `template` | Template for pages from this directory |
| `section_template` | Template for sections from this directory |
| `sort_by` | Sort order: `"title"` or `"date"` |
| `rewrite_links` | Rewrite relative `.md` links to clean URL paths |

With this config, a file at `../docs/guide/installation.md` becomes available at `/docs/guide/installation/`. Zorto treats the external directory as if it were inside `content/` — sections, pages, frontmatter, and links all work normally.

### Exclude files

If some external files overlap with manually written content, exclude them:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
exclude = ["reference/cli.md"]
```

The excluded file is skipped during content loading. You can then provide your own version at `content/docs/reference/cli.md`.

### Multiple content directories

You can define multiple `content_dirs` entries to pull from several sources:

```toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
sort_by = "title"
rewrite_links = true

[[content_dirs]]
path = "../api-docs"
url_prefix = "api"
sort_by = "title"
rewrite_links = true
```

Each entry is independent — different URL prefixes, templates, and sort orders.

## Add navigation

Configure top-level navigation in `config.toml` using `menu_items` under `[extra]`:

```toml
[extra]
menu_items = [
  { name = "Guide", url = "/guide/" },
  { name = "Reference", url = "/reference/" },
]
```

If using `content_dirs` with a URL prefix, adjust the URLs accordingly:

```toml
[extra]
menu_items = [
  { name = "Guide", url = "/docs/guide/" },
  { name = "Reference", url = "/docs/reference/" },
]
```

Built-in themes render `menu_items` as a navigation bar. For sidebar navigation within a section, theme templates use `section.subsections` and `section.pages` to build the sidebar automatically.

## Enable anchor links

Add clickable anchor links to headings so readers can link to specific sections:

```toml
# config.toml
[markdown]
insert_anchor_links = "right"
```

Options: `"right"`, `"left"`, or `"none"` (default).

## Build and deploy

Build the site for production:

```bash
zorto build
```

The output goes to `public/`. Deploy it to any static hosting provider — see [Deploy your site](deploy.md) for platform-specific instructions.

> [!TIP]
> Run `zorto check` before deploying to catch broken internal links and configuration issues.

## Related guides

- [Content model](../concepts/content-model.md) — sections, pages, and frontmatter in depth
- [Configuration reference](../reference/config.md) — full `content_dirs` and config field reference
- [Organize content](organize-content.md) — section nesting and external content directories
- [Customize navigation and footer](customize-nav-footer.md) — menus, logo, social links
