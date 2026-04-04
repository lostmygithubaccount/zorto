# Advanced templating

Go beyond basic templates with macros, block inheritance, loops, pagination, and dynamic data from `config.extra`.

## Block inheritance

Every Zorto template system starts with a base layout. Child templates extend it and override specific blocks.

**base.html** defines the skeleton:

```html
<html>
<head>
  <title>{% block title %}{{ config.title }}{% endblock %}</title>
</head>
<body>
  <nav>{% block nav %}{% endblock %}</nav>
  <main>{% block content %}{% endblock %}</main>
  <footer>{% block footer %}{% endblock %}</footer>
</body>
</html>
```

**page.html** overrides only what it needs:

```html
{% extends "base.html" %}

{% block title %}{{ page.title }} | {{ config.title }}{% endblock %}

{% block content %}
<article>
  <h1>{{ page.title }}</h1>
  {{ page.content | safe }}
</article>
{% endblock %}
```

Blocks not overridden in the child template use the parent's default content. You can nest extends multiple levels deep (e.g. `page.html` extends `base.html`, `post.html` extends `page.html`).

### Calling the parent block

Use `super()` to include the parent block's content alongside your additions:

<pre><code>&#123;% block footer %&#125;
  &#123;&#123; super() &#125;&#125;
  &lt;p&gt;Extra footer content&lt;/p&gt;
&#123;% endblock %&#125;</code></pre>

## Macros

Macros are reusable template fragments with parameters. Define them in any template file and import them where needed.

### Define a macro

<pre><code>&#123;% macro card(title, url, description="") %&#125;
&lt;a href="&#123;&#123; url &#125;&#125;" class="card"&gt;
  &lt;h3&gt;&#123;&#123; title &#125;&#125;&lt;/h3&gt;
  &#123;% if description %&#125;
    &lt;p&gt;&#123;&#123; description &#125;&#125;&lt;/p&gt;
  &#123;% endif %&#125;
&lt;/a&gt;
&#123;% endmacro %&#125;</code></pre>

### Use a macro in the same file

<pre><code>&#123;&#123; self::card(title="Home", url="/") &#125;&#125;
&#123;&#123; self::card(title="About", url="/about/", description="Learn more") &#125;&#125;</code></pre>

### Import macros from another file

Create `templates/macros.html` with your macro definitions, then import:

<pre><code>&#123;% import "macros.html" as macros %&#125;

&#123;&#123; macros::card(title="Home", url="/") &#125;&#125;</code></pre>

## For loops

Iterate over arrays like `section.pages`, taxonomy terms, or any array value.

### Basic loop

<pre><code>&#123;% for page in section.pages %&#125;
  &lt;article&gt;
    &lt;h2&gt;&lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;&lt;/h2&gt;
    &#123;% if page.date %&#125;
      &lt;time&gt;&#123;&#123; page.date | date(format="%B %d, %Y") &#125;&#125;&lt;/time&gt;
    &#123;% endif %&#125;
  &lt;/article&gt;
&#123;% endfor %&#125;</code></pre>

### Loop variables

Tera provides these variables inside `for` loops:

| Variable | Description |
|----------|-------------|
| `loop.index` | Current iteration (1-based) |
| `loop.index0` | Current iteration (0-based) |
| `loop.first` | `true` on first iteration |
| `loop.last` | `true` on last iteration |

<pre><code>&#123;% for page in section.pages %&#125;
  &lt;div class="&#123;% if loop.first %&#125;featured&#123;% endif %&#125;"&gt;
    &#123;&#123; page.title &#125;&#125;
  &lt;/div&gt;
&#123;% endfor %&#125;</code></pre>

### Limiting results

Use the `slice` filter to show only a subset:

<pre><code>&#123;% for page in section.pages | slice(end=3) %&#125;
  &lt;!-- Only the first 3 pages --&gt;
&#123;% endfor %&#125;</code></pre>

## Pagination

Sections with `paginate_by` set in their frontmatter provide a `paginator` object in the template context.

### Section frontmatter

```toml
+++
title = "Blog"
sort_by = "date"
paginate_by = 10
+++
```

### Paginator fields

| Field | Type | Description |
|-------|------|-------------|
| `paginator.pages` | array | Pages for the current pagination page |
| `paginator.current_index` | int | Current page number (1-based) |
| `paginator.number_pagers` | int | Total number of pagination pages |
| `paginator.previous` | string or null | URL of the previous page |
| `paginator.next` | string or null | URL of the next page |
| `paginator.first` | string | URL of the first page |
| `paginator.last` | string | URL of the last page |

### Pagination template

<pre><code>&#123;% for page in paginator.pages %&#125;
  &lt;article&gt;
    &lt;h2&gt;&lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;&lt;/h2&gt;
  &lt;/article&gt;
&#123;% endfor %&#125;

&lt;nav class="pagination"&gt;
  &#123;% if paginator.previous %&#125;
    &lt;a href="&#123;&#123; paginator.previous &#125;&#125;"&gt;Previous&lt;/a&gt;
  &#123;% endif %&#125;

  &lt;span&gt;Page &#123;&#123; paginator.current_index &#125;&#125; of &#123;&#123; paginator.number_pagers &#125;&#125;&lt;/span&gt;

  &#123;% if paginator.next %&#125;
    &lt;a href="&#123;&#123; paginator.next &#125;&#125;"&gt;Next&lt;/a&gt;
  &#123;% endif %&#125;
&lt;/nav&gt;</code></pre>

## Accessing config.extra

The `config` object is available in every template. Custom values from the `[extra]` section of `config.toml` are accessible as `config.extra`.

### Define custom values

```toml
# config.toml
[extra]
author = "Cody"
year = 2026
social = { github = "dkdc-io", twitter = "dkdc_io" }
menu_items = [
  { name = "Home", url = "/" },
  { name = "Blog", url = "/posts/" },
]
```

### Use in templates

<pre><code>&lt;footer&gt;
  &lt;p&gt;&amp;copy; &#123;&#123; config.extra.year &#125;&#125; &#123;&#123; config.extra.author &#125;&#125;&lt;/p&gt;

  &#123;% if config.extra.social %&#125;
    &lt;a href="https://github.com/&#123;&#123; config.extra.social.github &#125;&#125;"&gt;GitHub&lt;/a&gt;
  &#123;% endif %&#125;
&lt;/footer&gt;

&lt;nav&gt;
  &#123;% for item in config.extra.menu_items %&#125;
    &lt;a href="&#123;&#123; item.url &#125;&#125;"&gt;&#123;&#123; item.name &#125;&#125;&lt;/a&gt;
  &#123;% endfor %&#125;
&lt;/nav&gt;</code></pre>

### Other config fields

Beyond `extra`, these top-level config fields are also available:

<pre><code>&#123;&#123; config.base_url &#125;&#125;       &lt;!-- "https://example.com" --&gt;
&#123;&#123; config.title &#125;&#125;          &lt;!-- "My Site" --&gt;
&#123;&#123; config.description &#125;&#125;    &lt;!-- "Site description" --&gt;</code></pre>

## Conditional taxonomy rendering

Render taxonomy values only when they exist on a page:

<pre><code>&#123;% if page.taxonomies.tags %&#125;
&lt;div class="tags"&gt;
  &#123;% for tag in page.taxonomies.tags %&#125;
    &lt;a href="&#123;&#123; get_taxonomy_url(kind="tags", name=tag) &#125;&#125;" class="tag"&gt;
      &#123;&#123; tag &#125;&#125;
    &lt;/a&gt;
  &#123;% endfor %&#125;
&lt;/div&gt;
&#123;% endif %&#125;

&#123;% if page.taxonomies.categories %&#125;
&lt;div class="categories"&gt;
  &#123;% for cat in page.taxonomies.categories %&#125;
    &lt;a href="&#123;&#123; get_taxonomy_url(kind="categories", name=cat) &#125;&#125;"&gt;
      &#123;&#123; cat &#125;&#125;
    &lt;/a&gt;
  &#123;% endfor %&#125;
&lt;/div&gt;
&#123;% endif %&#125;</code></pre>

## Conditional content with tests

Use Tera's `is` keyword with custom tests:

<pre><code>&#123;% if page.path is starting_with("/docs") %&#125;
  &lt;!-- Show docs sidebar --&gt;
&#123;% endif %&#125;

&#123;% if page.draft %&#125;
  &lt;div class="draft-banner"&gt;This is a draft&lt;/div&gt;
&#123;% endif %&#125;</code></pre>

## Loading sections dynamically

Use `get_section` to pull content from other sections into any template:

<pre><code>&#123;% set recent_posts = get_section(path="posts/_index.md") %&#125;

&lt;h2&gt;Latest posts&lt;/h2&gt;
&#123;% for page in recent_posts.pages | slice(end=3) %&#125;
  &lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;
&#123;% endfor %&#125;</code></pre>

This is commonly used on the homepage (`index.html`) to show recent posts from a section.

## Further reading

- [Templates concept](../concepts/templates.md) — template hierarchy and context variables
- [Template functions and filters](../reference/template-functions.md) — complete function and filter reference
- [Taxonomies in depth](../reference/taxonomies.md) — taxonomy template rendering
