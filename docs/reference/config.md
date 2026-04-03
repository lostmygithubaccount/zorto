# Configuration reference

Complete reference for `config.toml`. See [configuration concept](../concepts/configuration.md) for the mental model and examples. The tables below are auto-generated from the Zorto source code.

{{ configref(src="../crates/zorto-core/src/config.rs") }}

## Section frontmatter

Used in `_index.md` files:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | string | `""` | Section title |
| `description` | string | `""` | Section description |
| `sort_by` | string | `"date"` | Sort pages by: `"date"`, `"title"` |
| `paginate_by` | int | `0` | Pages per pagination page (0 = no pagination) |
| `template` | string | `"section.html"` | Custom template for this section |
| `[extra]` | table | `{}` | Custom data for templates |

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
