# Taxonomies in depth

Taxonomies let you classify content with terms like tags, categories, or any custom grouping. Zorto auto-generates listing pages for each taxonomy and its terms.

## Defining taxonomies

Taxonomies are declared in `config.toml` with `[[taxonomies]]`:

```toml
[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"
```

Each entry requires a `name` field. The name determines the URL path (`/tags/`, `/categories/`) and the frontmatter field name.

{% note(type="info") %}
If you omit `[[taxonomies]]` entirely, Zorto creates a default `tags` taxonomy. Once you define any `[[taxonomies]]` entry, only the ones you list are active.
{% end %}

## Assigning terms

In page frontmatter, add a top-level array field matching the taxonomy name:

```toml
+++
title = "Building a web app with Rust"
date = "2026-03-15"
tags = ["rust", "web", "tutorial"]
categories = ["engineering"]
+++
```

Any top-level frontmatter key whose value is an array of strings is treated as a taxonomy assignment. The key must match a taxonomy `name` defined in `config.toml`.

## Auto-generated pages

For each taxonomy, Zorto generates two types of pages:

### List page

A page at `/<taxonomy>/` showing all terms. For `tags`, this renders at `/tags/`.

**Template:** `<taxonomy>/list.html` (e.g. `tags/list.html`)

**Context variables:**

| Variable | Type | Description |
|----------|------|-------------|
| `terms` | array of objects | All terms in this taxonomy, sorted alphabetically |
| `terms[].name` | string | Original term name (e.g. `"Rust"`) |
| `terms[].slug` | string | URL-safe slug (e.g. `"rust"`) |
| `terms[].permalink` | string | Full URL to the term page |
| `terms[].pages` | array | Pages with this term, sorted by date (newest first) |
| `config` | object | Site configuration |

### Term page

A page at `/<taxonomy>/<term>/` showing pages with that term. For the tag `"rust"`, this renders at `/tags/rust/`.

**Template:** `<taxonomy>/single.html` (e.g. `tags/single.html`)

**Context variables:**

| Variable | Type | Description |
|----------|------|-------------|
| `term` | object | The current taxonomy term |
| `term.name` | string | Original term name |
| `term.slug` | string | URL-safe slug |
| `term.permalink` | string | Full URL to this term page |
| `term.pages` | array | Pages with this term, sorted by date (newest first) |
| `config` | object | Site configuration |

## Template examples

### Taxonomy list template

Create `templates/tags/list.html` to render the tags index page:

<pre><code>&#123;% extends "base.html" %&#125;

&#123;% block content %&#125;
&lt;h1&gt;Tags&lt;/h1&gt;

&lt;ul&gt;
&#123;% for term in terms %&#125;
  &lt;li&gt;
    &lt;a href="&#123;&#123; term.permalink &#125;&#125;"&gt;&#123;&#123; term.name &#125;&#125;&lt;/a&gt;
    (&#123;&#123; term.pages | length &#125;&#125; post&#123;&#123; term.pages | length | pluralize &#125;&#125;)
  &lt;/li&gt;
&#123;% endfor %&#125;
&lt;/ul&gt;
&#123;% endblock %&#125;</code></pre>

### Taxonomy single template

Create `templates/tags/single.html` to render a page for each tag:

<pre><code>&#123;% extends "base.html" %&#125;

&#123;% block content %&#125;
&lt;h1&gt;Posts tagged "&#123;&#123; term.name &#125;&#125;"&lt;/h1&gt;

&#123;% for page in term.pages %&#125;
&lt;article&gt;
  &lt;h2&gt;&lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;&lt;/h2&gt;
  &#123;% if page.date %&#125;
    &lt;time&gt;&#123;&#123; page.date | date(format="%B %d, %Y") &#125;&#125;&lt;/time&gt;
  &#123;% endif %&#125;
  &#123;% if page.summary %&#125;
    &#123;&#123; page.summary | safe &#125;&#125;
  &#123;% endif %&#125;
&lt;/article&gt;
&#123;% endfor %&#125;
&#123;% endblock %&#125;</code></pre>

## Linking to taxonomy terms from pages

Use `get_taxonomy_url` in any template to generate a link to a term page:

<pre><code>&#123;% if page.taxonomies.tags %&#125;
&lt;div class="tags"&gt;
  &#123;% for tag in page.taxonomies.tags %&#125;
    &lt;a href="&#123;&#123; get_taxonomy_url(kind="tags", name=tag) &#125;&#125;"&gt;
      &#123;&#123; tag &#125;&#125;
    &lt;/a&gt;
  &#123;% endfor %&#125;
&lt;/div&gt;
&#123;% endif %&#125;</code></pre>

## Template directory structure

Taxonomy templates live in a subdirectory named after the taxonomy:

{% tree(caption="Each taxonomy gets its own template directory.") %}
templates/
  base.html
  page.html
  section.html
  tags/
    list.html  [renders /tags/]
    single.html  [renders /tags/rust/, etc.]
  categories/
    list.html  [renders /categories/]
    single.html  [renders /categories/engineering/, etc.]
{% end %}

{% note(type="warning") %}
Taxonomy pages are only rendered if the corresponding template exists. If there is no `tags/list.html`, the `/tags/` page will not be generated.
{% end %}

## Multiple taxonomies

You can define as many taxonomies as needed:

```toml
[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "categories"

[[taxonomies]]
name = "authors"
```

Each taxonomy is completely independent. A page can have terms in any or all of them:

```toml
+++
title = "Zorto 1.0 release"
tags = ["release", "rust"]
categories = ["announcements"]
authors = ["cody"]
+++
```

## Term slugification

Term names are slugified for URLs: spaces become hyphens, special characters are removed, and the result is lowercased. For example:

| Term name | Slug | URL |
|-----------|------|-----|
| `"Rust"` | `rust` | `/tags/rust/` |
| `"My Tag"` | `my-tag` | `/tags/my-tag/` |
| `"C++"` | `c` | `/tags/c/` |

Pages within each term are sorted by date in reverse chronological order (newest first). Terms in the list page are sorted alphabetically by name.

## Further reading

- [Frontmatter reference](frontmatter.md) — how to assign taxonomy values in frontmatter
- [Template functions and filters](template-functions.md) — `get_taxonomy_url` function reference
- [Templates concept](../concepts/templates.md) — template hierarchy and context variables
