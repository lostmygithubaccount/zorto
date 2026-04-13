# Shortcodes reference

Complete reference for all built-in shortcodes. See [shortcodes concept](../concepts/shortcodes.md) for an overview and [how to customize your theme](../how-to/customize-theme.md) for creating custom shortcodes.

## include

Include the contents of another file.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | string | *required* | Path to the file (relative to project root), or an `https://` URL |
| `strip_frontmatter` | bool | `false` | Remove `+++`-delimited TOML frontmatter from included content |
| `rewrite_links` | bool | `false` | Rewrite relative `.md` links to clean URL paths |

**Example:**

<pre><code>&#123;&#123; include(path="README.md", strip_frontmatter="true") &#125;&#125;</code></pre>

## tabs

Render tabbed content panels.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `labels` | string | *required* | Pipe-separated tab labels |

Each tab's content is separated by `<!-- tab -->` in the body.

**Example:**

{% tabs(labels="Rust|Python|Bash") %}
`cargo install zorto`
<!-- tab -->
`uv tool install zorto`
<!-- tab -->
`curl -LsSf https://dkdc.sh/zorto/install.sh | sh`
{% end %}

## note

Styled callout box.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `type` | string | *required* | Style: `info`, `warning`, `tip`, `danger` |

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
| `summary` | string | *required* | Text shown in the clickable summary |
| `open` | bool | `false` | Whether the section starts expanded |

**Example:**

{% details(summary="Click to expand") %}
Hidden content revealed on click.

You can include any markdown here: **bold**, `code`, lists, etc.
{% end %}

**Syntax:**

```
{% details(summary="Click to expand", open="true") %}
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

## pyref

Auto-generate Python API reference documentation by introspecting a module at build time. Requires the `python` feature.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `module` | string | *required* | Python module name to document |
| `recursive` | bool | `true` | Walk submodules |
| `exclude` | string | `""` | Comma-separated names to exclude |
| `include` | string | `""` | Comma-separated allowlist (only document these) |
| `private` | bool | `false` | Include `_private` members |

**Example:**

<pre><code>&#123;&#123; pyref(module="zorto", exclude="main,core", recursive="false") &#125;&#125;</code></pre>

Generates HTML with function signatures, class methods, and docstrings. Doctest examples (`>>>` lines in docstrings) are executed at build time and their output is rendered inline.

## configref

Auto-generate configuration reference from a Rust source file's doc comments and serde attributes.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `src` | string | *required* | Path to Rust source file (relative to site root) |

**Example:**

<pre><code>&#123;&#123; configref(src="../crates/zorto-core/src/config.rs") &#125;&#125;</code></pre>

Parses struct definitions, field types, `///` doc comments, and `#[serde(...)]` attributes to generate HTML tables.

## flow

Horizontal step flow diagram with arrows between steps.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `steps` | string | *required* | Pipe-delimited steps, each as `Label:Description` or just `Label` |
| `caption` | string | `""` | Caption text below the diagram |

**Example:**

{{ flow(steps="Write:Markdown content|Build:Compile site|Deploy:Push to production", caption="A typical workflow.") }}

**Syntax:**

<pre><code>&#123;&#123; flow(steps="Write:Content|Build:Compile|Deploy:Ship", caption="Development workflow.") &#125;&#125;</code></pre>

## layers

Vertical layered stack diagram with numbered items.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `items` | string | *required* | Pipe-delimited items, each as `Title:Description:badge` |
| `caption` | string | `""` | Caption text below the diagram |

**Example:**

{{ layers(items="Identity:Site name and URL:base_url|Build:Output toggles:feeds, sitemap|Theme:Visual appearance:theme", caption="Configuration layers.") }}

**Syntax:**

<pre><code>&#123;&#123; layers(items="Layer 1:Description:badge|Layer 2:Description:badge") &#125;&#125;</code></pre>

## tree

File tree visualization. Body content defines the tree structure, one entry per line.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `caption` | string | `""` | Caption text below the tree |

Lines use indentation (2 spaces per level) for nesting. Append `[tag]` for labels. Directories end with `/`.

**Example:**

{% tree(caption="A typical Zorto project.") %}
my-site/
  config.toml
  content/
    _index.md  [section]
    about.md  [page]
  templates/
  sass/
{% end %}

**Syntax:**

<pre><code>&#123;% tree(caption="Project structure.") %&#125;
content/
  _index.md  [section]
  about.md  [page]
&#123;% end %&#125;</code></pre>

## compare

Side-by-side comparison cards.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `left_title` | string | `""` | Title for the left card |
| `left` | string | *required* | Body text for the left card |
| `right_title` | string | `""` | Title for the right card |
| `right` | string | *required* | Body text for the right card |
| `left_style` | string | `"accent"` | Style: `accent` (blue), `green`, or `muted` |
| `right_style` | string | `"green"` | Style: `accent`, `green`, or `muted` |
| `caption` | string | `""` | Caption text below |

**Example:**

{{ compare(left_title="Before", left="Manual process, error-prone, slow.", right_title="After", right="Automated, validated, fast.") }}

**Syntax:**

<pre><code>&#123;&#123; compare(left_title="Option A", left="Description A", right_title="Option B", right="Description B") &#125;&#125;</code></pre>

## cascade

Override/priority cascade diagram. The last item is highlighted as the winner.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `items` | string | *required* | Pipe-delimited items, each as `Priority:Label:badge` |
| `caption` | string | `""` | Caption text below |

**Example:**

{{ cascade(items="Default:Built-in theme templates:fallback|Override:Your local templates/:wins", caption="Local files always take priority.") }}

**Syntax:**

<pre><code>&#123;&#123; cascade(items="Low:Default value:default|High:Your override:wins") &#125;&#125;</code></pre>

## Presentation shortcodes

The following shortcodes are designed for use in [presentations](../concepts/presentations.md) but work in any page.

## slide_image

Absolutely positioned image for slide layouts.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `src` | string | *required* | Image path or URL |
| `alt` | string | `""` | Alt text |
| `top` | string | | CSS top position (e.g. `"10%"`, `"50px"`) |
| `left` | string | | CSS left position |
| `right` | string | | CSS right position |
| `bottom` | string | | CSS bottom position |
| `width` | string | | CSS width |
| `height` | string | | CSS height |

**Syntax:**

<pre><code>&#123;&#123; slide_image(src="logo.png", top="10%", right="5%", width="200px") &#125;&#125;</code></pre>

## speaker_notes

Speaker notes for reveal.js presentations. Press `S` in a presentation to open the speaker view.

**Syntax:**

<pre><code>&#123;% speaker_notes() %&#125;
Remember to mention the key point about performance.
&#123;% end %&#125;</code></pre>

## fragment

Progressive reveal — content appears on each click/advance within a slide.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `style` | string | `"fade-in"` | Animation style. Validated against an allowlist; an unknown value errors at build time rather than rendering a no-op fragment. |

Allowed styles: `fade-in`, `fade-out`, `fade-up`, `fade-down`, `fade-left`, `fade-right`, `grow`, `shrink`, `strike`, `highlight-red`, `highlight-blue`, `highlight-green`, `highlight-current-red`, `highlight-current-blue`, `highlight-current-green`.

**Syntax:**

<pre><code>&#123;% fragment(style="fade-in") %&#125;
This appears on click.
&#123;% end %&#125;</code></pre>

## columns

Multi-column layout. Body content is split on `<!-- column -->` markers.

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `widths` | string | equal | Pipe-separated column widths (e.g. `"60%|40%"`) |

**Syntax:**

<pre><code>&#123;% columns(widths="60%|40%") %&#125;
Left column content

&lt;!-- column --&gt;

Right column content
&#123;% end %&#125;</code></pre>
