Complete reference for `config.toml`.

## Top-level settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `base_url` | string | *required* | Full URL of the deployed site (e.g. `https://example.com`) |
| `title` | string | `""` | Site title, available as `config.title` in templates |
| `description` | string | `""` | Site description for meta tags and feeds |
| `default_language` | string | `"en"` | Default language for the site |
| `theme` | string | `""` | Theme to use (`dkdc`, `light`, `dark`) |
| `compile_sass` | bool | `true` | Compile `.scss` files from `sass/` to CSS |
| `generate_feed` | bool | `false` | Generate an Atom feed at `/atom.xml` |
| `generate_sitemap` | bool | `true` | Generate `sitemap.xml` |
| `generate_llms_txt` | bool | `true` | Generate `llms.txt` and `llms-full.txt` for LLM consumption |

## `[markdown]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `highlight_code` | bool | `true` | Enable syntax highlighting in code blocks |
| `highlight_theme` | string | `"base16-ocean.dark"` | Syntax highlighting theme |
| `insert_anchor_links` | string | `"none"` | Add anchor links to headings: `"none"`, `"right"` |
| `external_links_target_blank` | bool | `false` | Open external links in a new tab |
| `external_links_no_follow` | bool | `false` | Add `rel="nofollow"` to external links |
| `external_links_no_referrer` | bool | `false` | Add `rel="noreferrer"` to external links |
| `smart_punctuation` | bool | `false` | Convert `--` to em-dash, `"quotes"` to smart quotes, etc. |

## `[[taxonomies]]`

Each taxonomy is defined as an array entry:

```toml
[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | *required* | Taxonomy name (used in URLs and frontmatter) |

## `[extra]`

Arbitrary key-value pairs accessible as `config.extra` in templates. Common patterns:

```toml
[extra]
author = "Your Name"
favicon = "/favicon.png"
favicon_mimetype = "image/png"

menu_items = [
  { name = "Docs", url = "/docs/" },
  { name = "Blog", url = "/posts/" },
]

[[extra.social_links]]
name = "GitHub"
url = "https://github.com/you"
icon = "github"
```

## Section frontmatter

Used in `_index.md` files:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | string | `""` | Section title |
| `description` | string | `""` | Section description |
| `sort_by` | string | `"date"` | Sort pages by: `"date"`, `"title"` |
| `paginate_by` | int | `0` | Pages per pagination page (0 = no pagination) |
| `template` | string | `"section.html"` | Custom template for this section |

## Page frontmatter

Used in regular `.md` files:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | string | `""` | Page title |
| `date` | string | `""` | Publication date (YYYY-MM-DD) |
| `author` | string | `""` | Author name |
| `description` | string | `""` | Page description for SEO |
| `draft` | bool | `false` | Exclude from production builds |
| `slug` | string | filename | Override the URL slug |
| `template` | string | `"page.html"` | Custom template |
| `aliases` | array of strings | `[]` | Redirect old URLs to this page |
| taxonomy fields | array of strings | `[]` | Taxonomy values as top-level arrays (e.g. `tags = ["rust"]`) |
| `[extra]` | table | `{}` | Custom data for templates |
