+++
title = "Shortcodes reference"
template = "docs.html"
+++

Complete reference for all built-in shortcodes.

## include

Include the contents of another file.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | *required* | Path to the file (relative to project root) |
| `strip_frontmatter` | bool | `false` | Remove TOML frontmatter from included content |

**Example:**

<pre><code>&#123;&#123; include(path="README.md", strip_frontmatter="true") &#125;&#125;</code></pre>

## tabs

Render tabbed content panels.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `labels` | string | *required* | Comma-separated tab labels |

Each tab's content is separated by `<!-- tab -->` in the body.

**Example:**

{% tabs(labels="Rust|Python|Bash") %}
`cargo install zorto`
<!-- tab -->
`uv tool install zorto`
<!-- tab -->
`curl -sSL https://dkdc.sh/zorto | bash`
{% end %}

## note

Styled callout box.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `type` | string | `"info"` | Style: `info`, `warning`, `tip`, `danger` |

**Examples:**

{% note(type="info") %}
This is an info note.
{% end %}

{% note(type="warning") %}
This is a warning note.
{% end %}

{% note(type="tip") %}
This is a tip note.
{% end %}

{% note(type="danger") %}
This is a danger note.
{% end %}

**Syntax:**

```
{% note(type="warning") %}
Be careful with this operation.
{% end %}
```

## details

Collapsible disclosure section.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `summary` | string | `"Details"` | Text shown in the clickable summary |
| `open` | bool | `false` | Whether the section starts expanded |

**Example:**

{% details(summary="Click to expand") %}
Hidden content revealed on click.

You can include any markdown here: **bold**, `code`, lists, etc.
{% end %}

**Syntax:**

```
{% details(summary="Click to expand", open=true) %}
This starts expanded.
{% end %}
```

## figure

Image with optional caption.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `src` | string | *required* | Image URL or path |
| `alt` | string | `""` | Alt text |
| `caption` | string | `""` | Caption displayed below the image |
| `width` | string | `""` | CSS width (e.g. `"80%"`, `"400px"`) |

**Syntax:**

```
{{ figure(src="/images/screenshot.png", alt="Screenshot", caption="The dashboard view", width="80%") }}
```

## youtube

Embed a YouTube video.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `id` | string | *required* | YouTube video ID |

**Syntax:**

```
{{ youtube(id="dQw4w9WgXcQ") }}
```

## gist

Embed a GitHub Gist.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | string | *required* | Full Gist URL |
| `file` | string | `""` | Specific file from the gist to embed |

**Syntax:**

```
{{ gist(url="https://gist.github.com/user/abc123", file="example.py") }}
```

## mermaid

Render a Mermaid diagram.

**Example:**

{% mermaid() %}
graph LR
    A[Markdown] --> B[Zorto]
    B --> C[HTML]
    B --> D[CSS]
    B --> E[Sitemap]
{% end %}

**Syntax:**

```
{% mermaid() %}
graph LR
    A[Markdown] --> B[Zorto]
    B --> C[HTML]
{% end %}
```
