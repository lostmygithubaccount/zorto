# Template functions and filters

Zorto registers custom template functions, filters, and tests on top of the [Tera template engine](https://keats.github.io/tera/docs/). This page is the complete reference.

## Functions

Functions are called with named arguments inside templates.

### get_url

Returns the full permalink for a content path or static file.

**Signature:** `get_url(path)`

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Content path (`@/` prefix), static file path, or external URL |

**Content paths** use the `@/` prefix to reference files in the `content/` directory:

<pre><code>&#123;&#123; get_url(path="posts/hello.md") &#125;&#125;
&lt;!-- https://example.com/posts/hello/ --&gt;

&#123;&#123; get_url(path="@/posts/_index.md") &#125;&#125;
&lt;!-- https://example.com/posts/ --&gt;</code></pre>

**Static file paths** are resolved relative to the site root:

<pre><code>&#123;&#123; get_url(path="/img/photo.png") &#125;&#125;
&lt;!-- https://example.com/img/photo.png --&gt;</code></pre>

**External URLs** are returned as-is:

<pre><code>&#123;&#123; get_url(path="https://github.com") &#125;&#125;
&lt;!-- https://github.com --&gt;</code></pre>

### get_section

Loads a section object by its `_index.md` path. Returns the full section with its pages, useful for cross-referencing content.

**Signature:** `get_section(path)`

| Argument | Type | Description |
|----------|------|-------------|
| `path` | string | Relative path to the section's `_index.md` |

<pre><code>&#123;% set posts = get_section(path="posts/_index.md") %&#125;
&#123;% for page in posts.pages %&#125;
  &lt;a href="&#123;&#123; page.permalink &#125;&#125;"&gt;&#123;&#123; page.title &#125;&#125;&lt;/a&gt;
&#123;% endfor %&#125;</code></pre>

The returned section object has all the fields documented in the [frontmatter reference](frontmatter.md) (title, pages, path, permalink, extra, etc.).

{% note(type="warning") %}
`get_section` raises an error if the path does not match any loaded section. Double-check the path matches the actual `_index.md` location.
{% end %}

### get_taxonomy_url

Returns the permalink for a specific taxonomy term page.

**Signature:** `get_taxonomy_url(kind, name)`

| Argument | Type | Description |
|----------|------|-------------|
| `kind` | string | Taxonomy name (e.g. `"tags"`, `"categories"`) |
| `name` | string | Term value (e.g. `"rust"`) |

<pre><code>&#123;&#123; get_taxonomy_url(kind="tags", name="rust") &#125;&#125;
&lt;!-- https://example.com/tags/rust/ --&gt;</code></pre>

The term name is slugified to form the URL (e.g. `"My Tag"` becomes `my-tag`).

### now

Returns the current local timestamp as a string in `YYYY-MM-DDTHH:MM:SS` format.

**Signature:** `now()`

<pre><code>&lt;footer&gt;Built at &#123;&#123; now() &#125;&#125;&lt;/footer&gt;
&lt;!-- Built at 2026-04-04T14:30:00 --&gt;</code></pre>

## Filters

Filters transform values using the pipe (`|`) syntax. Zorto provides these custom filters in addition to [Tera's built-in filters](https://keats.github.io/tera/docs/#built-in-filters).

### date

Formats a date string using [chrono format specifiers](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `format` | string | `"%Y-%m-%d"` | Output format string |

Accepts `YYYY-MM-DD` and `YYYY-MM-DDTHH:MM:SS` input formats. Returns the original string unchanged if parsing fails.

<pre><code>&#123;&#123; page.date | date(format="%B %d, %Y") &#125;&#125;
&lt;!-- June 15, 2025 --&gt;

&#123;&#123; page.date | date(format="%Y") &#125;&#125;
&lt;!-- 2025 --&gt;</code></pre>

### pluralize

Returns `"s"` when the value is not 1, empty string when it is 1. Useful for English pluralization.

<pre><code>&#123;&#123; count &#125;&#125; item&#123;&#123; count | pluralize &#125;&#125;
&lt;!-- "1 item" or "5 items" --&gt;</code></pre>

Works with integers and floats (floats are truncated to integers).

### slice

Extracts a sub-array from an array.

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `start` | int | `0` | Start index (inclusive) |
| `end` | int | array length | End index (exclusive) |

<pre><code>&#123;% for page in section.pages | slice(end=5) %&#125;
  &lt;!-- First 5 pages --&gt;
&#123;% endfor %&#125;

&#123;% for page in section.pages | slice(start=2, end=7) %&#125;
  &lt;!-- Pages 3 through 7 --&gt;
&#123;% endfor %&#125;</code></pre>

Out-of-bounds values are clamped to the array length.

## Tests

Tests check conditions using the `is` keyword.

### starting_with

Tests whether a string starts with the given prefix.

<pre><code>&#123;% if page.path is starting_with("/docs") %&#125;
  &lt;!-- Documentation page --&gt;
&#123;% endif %&#125;</code></pre>

## Tera built-in filters

Zorto inherits all of Tera's built-in filters. Commonly used ones:

| Filter | Description | Example |
|--------|-------------|---------|
| `safe` | Mark HTML as safe (no escaping) | `{{ page.content \| safe }}` |
| `length` | Array or string length | `{{ items \| length }}` |
| `upper` | Uppercase a string | `{{ title \| upper }}` |
| `lower` | Lowercase a string | `{{ title \| lower }}` |
| `replace` | Replace substring | `{{ title \| replace(from="old", to="new") }}` |
| `truncate` | Truncate string | `{{ desc \| truncate(length=100) }}` |
| `default` | Fallback value | `{{ author \| default(value="Anonymous") }}` |
| `join` | Join array to string | `{{ tags \| join(sep=", ") }}` |
| `first` | First element | `{{ items \| first }}` |
| `last` | Last element | `{{ items \| last }}` |
| `reverse` | Reverse array | `{{ items \| reverse }}` |
| `sort` | Sort array | `{{ items \| sort }}` |
| `json_encode` | Serialize to JSON | `{{ data \| json_encode }}` |

See the [Tera documentation](https://keats.github.io/tera/docs/#built-in-filters) for the complete list.

## Further reading

- [Templates concept](../concepts/templates.md) — template hierarchy and context variables
- [Advanced templating](../how-to/advanced-templating.md) — macros, block inheritance, config.extra access
- [Frontmatter reference](frontmatter.md) — all fields available on `page` and `section` objects
