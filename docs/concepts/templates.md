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

## Template context

Each template receives context variables:

| Variable | Available in | Description |
|----------|-------------|-------------|
| `page` | `page.html` | Current page object |
| `section` | `section.html`, `index.html` | Current section object |
| `config` | All | Site configuration |
| `paginator` | Paginated sections | Pagination info |

## Custom functions

| Function | Description |
|----------|-------------|
| `get_url(path)` | Get the permalink for a path |
| `get_section(path)` | Load a section and its pages |
| `get_taxonomy_url(kind, name)` | URL for a taxonomy term |
| `now()` | Current timestamp |

Example:

```html
{% set posts = get_section(path="posts/_index.md") %}
{% for page in posts.pages %}
  <a href="{{ page.permalink }}">{{ page.title }}</a>
{% endfor %}
```

## Filters and tests

Common filters:

```html
{{ count | pluralize }}             <!-- "s" if count != 1, "" otherwise -->
{{ pages | slice(start=0, end=5) }} <!-- first 5 items -->
{{ page.date | date(format="%B %Y") }} <!-- January 2026 -->
```

Common tests:

```html
{% if path is starting_with("/docs") %}...{% endif %}
```

## Blocks and inheritance

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

## Macros

Define reusable template fragments:

<pre><code>&#123;% macro card(title, url) %&#125;
  &lt;a href="&#123;&#123; url &#125;&#125;" class="card"&gt;&#123;&#123; title &#125;&#125;&lt;/a&gt;
&#123;% endmacro %&#125;

&#123;&#123; self::card(title="Home", url="/") &#125;&#125;</code></pre>
