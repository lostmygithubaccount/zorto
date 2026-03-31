Shortcodes let you embed rich content in markdown without writing HTML.

## Syntax

**Inline shortcodes** (no body) use double curly braces:

<pre><code>&#123;&#123; figure(src="/images/photo.jpg", alt="A photo") &#125;&#125;</code></pre>

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
| `details` | Body | Collapsible section |
| `figure` | Inline | Image with caption |
| `youtube` | Inline | Embedded YouTube video |
| `gist` | Inline | Embedded GitHub gist |
| `mermaid` | Body | Mermaid diagram |

See the [shortcodes reference](../reference/shortcodes.md) for full details and examples. The live reference is available at [zorto.dev/docs/reference/shortcodes](https://zorto.dev/docs/reference/shortcodes/).

## Custom shortcodes

Create a template at `templates/shortcodes/name.html`:

```html
<!-- templates/shortcodes/greeting.html -->
<p class="greeting">Hello, {{ name }}!</p>
```

Then use it in markdown:

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
