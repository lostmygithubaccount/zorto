# Templates

Zorto uses the [Tera](https://keats.github.io/tera/) template engine, which has Jinja2-like syntax.

## Template hierarchy

Zorto looks for these templates (themes provide defaults for all of them):

| Template | Used for |
|----------|----------|
| `base.html` | Base layout all others extend |
| `index.html` | Site homepage (`content/_index.md`) |
| `section.html` | Section pages (`content/*/_index.md`) |
| `page.html` | Individual pages |
| `404.html` | Not-found page |
| `taxonomy_list.html` | Taxonomy index (e.g., `/tags/`) |
| `taxonomy_single.html` | Single taxonomy term (e.g., `/tags/rust/`) |

## Template context

Each template receives context variables:

| Variable | Available in | Description |
|----------|-------------|-------------|
| `page` | `page.html` | Current page object (title, content, date, permalink, extra, etc.) |
| `section` | `section.html`, `index.html` | Current section object (title, pages, subsections, etc.) |
| `config` | All | Site configuration (`config.title`, `config.extra`, etc.) |
| `paginator` | Paginated sections | Pagination info (pages, current_index, number_pagers) |

Use `page.permalink` or `section.permalink` to get the current page's full URL.

## Custom functions

| Function | Description |
|----------|-------------|
| `get_url(path)` | Get the permalink for a path |
| `get_section(path)` | Load a section and its pages |
| `get_taxonomy_url(kind, name)` | URL for a taxonomy term |
| `now()` | Current timestamp |

Example:

<pre><code>&#123;% set posts = get_section(path="posts/_index.md") %&#125;
&#123;% for page in posts.pages %&#125;
  &lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;
&#123;% endfor %&#125;</code></pre>

## Worked example

Here's a complete `page.html` showing how template variables, filters, and blocks work together:

<pre><code>&#123;% extends "base.html" %&#125;

&#123;% block content %&#125;
&lt;article&gt;
  &lt;h1&gt;&#123;&#123; page.title &#125;&#125;&lt;/h1&gt;

  &#123;% if page.date %&#125;
  &lt;time&gt;&#123;&#123; page.date | date(format="%B %d, %Y") &#125;&#125;&lt;/time&gt;
  &#123;% endif %&#125;

  &#123;% if page.taxonomies.tags %&#125;
  &lt;div class="tags"&gt;
    &#123;% for tag in page.taxonomies.tags %&#125;
    &lt;a href="&#123;&#123; get_taxonomy_url(kind="tags", name=tag) &#125;&#125;"&gt;&#123;&#123; tag &#125;&#125;&lt;/a&gt;
    &#123;% endfor %&#125;
  &lt;/div&gt;
  &#123;% endif %&#125;

  &#123;&#123; page.content | safe &#125;&#125;
&lt;/article&gt;
&#123;% endblock %&#125;</code></pre>

## Filters and tests

Tera provides filters (transform values) and tests (check conditions). Common ones:

<pre><code>&#123;&#123; page.date | date(format="%B %Y") &#125;&#125;  &lt;!-- "January 2026" --&gt;
&#123;&#123; pages | slice(start=0, end=5) &#125;&#125;     &lt;!-- first 5 items --&gt;
&#123;&#123; count | pluralize &#125;&#125;                  &lt;!-- "s" if count != 1 --&gt;
&#123;% if path is starting_with("/docs") %&#125;...&#123;% endif %&#125;</code></pre>

See [Tera's documentation](https://keats.github.io/tera/docs/) for the full list of filters and tests.

## Blocks and inheritance

{{ cascade(items="Parent:base.html — shared layout (nav, footer, HTML skeleton):extends|Child:index.html, section.html, page.html — fill in specific blocks:overrides", caption="Child templates extend base.html and override only the blocks they need.") }}

Templates use block inheritance:

```html
{# base.html #}
<html>
<body>
  {% block content %}{% endblock %}
</body>
</html>
```

```html
{# page.html #}
{% extends "base.html" %}
{% block content %}
  <h1>{{ page.title }}</h1>
  {{ page.content | safe }}
{% endblock %}
```

## Discovering theme templates

To see what templates and blocks a theme provides, check the theme source in the Zorto repository under `crates/zorto-core/themes/<theme-name>/templates/`. Each theme defines the same set of templates with different styling.

## Macros

Define reusable template fragments:

<pre><code>&#123;% macro card(title, url) %&#125;
  &lt;a href="&#123;&#123; url &#125;&#125;" class="card"&gt;&#123;&#123; title &#125;&#125;&lt;/a&gt;
&#123;% endmacro %&#125;

&#123;&#123; self::card(title="Home", url="/") &#125;&#125;</code></pre>

## Further reading

- [Themes](themes.md) — how the theme system provides and layers templates
- [Content model](content-model.md) — how sections and pages map to templates
- [How to customize your theme](../how-to/customize-theme.md) — step-by-step template overrides
- [Tera documentation](https://keats.github.io/tera/docs/) — full syntax reference
