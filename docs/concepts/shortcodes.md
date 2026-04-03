# Shortcodes

Shortcodes let you embed rich, structured content in markdown without writing raw HTML. They bridge the gap between markdown's simplicity and the flexibility of full templates.

## Syntax

**Inline shortcodes** (no body) use double curly braces:

<pre><code>&#123;&#123; figure(src="/images/photo.jpg", alt="A photo", caption="My caption") &#125;&#125;</code></pre>

**Body shortcodes** (wrap content) use curly-percent:

<pre><code>&#123;% note(type="warning") %&#125;
Be careful with this operation.
&#123;% end %&#125;</code></pre>

## Built-in shortcodes

| Shortcode | Type | Description |
|-----------|------|-------------|
| `include` | Inline | Include another file's content |
| `tabs` | Body | Tabbed content panels |
| `note` | Body | Styled callout box |
| `details` | Body | Collapsible `<details>` section |
| `figure` | Inline | Image with optional caption |
| `youtube` | Inline | Embedded YouTube video |
| `gist` | Inline | Embedded GitHub gist |
| `mermaid` | Body | Mermaid.js diagram |
| `pyref` | Inline | Python API reference (requires `python` feature) |
| `configref` | Inline | Config reference from Rust source doc comments |
| `flow` | Inline | Horizontal step flow diagram |
| `layers` | Inline | Vertical layered stack diagram |
| `tree` | Body | File tree visualization |
| `compare` | Inline | Side-by-side comparison cards |
| `cascade` | Inline | Priority/override cascade diagram |

The diagram shortcodes (`flow`, `layers`, `tree`, `compare`, `cascade`) render pure CSS/HTML visuals with no JavaScript. They are used throughout these docs — see [content model](content-model.md) and [AI-native](ai-native.md) for examples.

See the [shortcodes reference](../reference/shortcodes.md) for parameters and live examples.

## Custom shortcodes

Create a template in `templates/shortcodes/` to define your own:

```html
<!-- templates/shortcodes/greeting.html -->
<p class="greeting">Hello, {{ name }}!</p>
```

Use it in markdown:

<pre><code>&#123;&#123; greeting(name="world") &#125;&#125;</code></pre>

Body shortcodes receive the inner content as `body`:

```html
<!-- templates/shortcodes/card.html -->
<div class="card">
  <h3>{{ title }}</h3>
  {{ body }}
</div>
```

<pre><code>&#123;% card(title="My card") %&#125;
Card content goes here.
&#123;% end %&#125;</code></pre>

## Shortcodes vs. callouts vs. templates

| Tool | Best for |
|------|----------|
| Callouts | Inline alerts in prose (note, warning, tip) |
| Shortcodes | Reusable rich components (figures, tabs, embeds) |
| Templates | Full page layouts and structural HTML |

## Further reading

- [Shortcodes reference](../reference/shortcodes.md) — all 15 built-in shortcodes with live examples
- [Callouts](callouts.md) — GitHub-style alert boxes
- [Templates](templates.md) — the Tera template engine
- [How to customize your theme](../how-to/customize-theme.md) — create custom shortcodes
