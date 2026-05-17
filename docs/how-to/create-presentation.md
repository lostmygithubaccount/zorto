# Create a presentation

This guide walks through creating a slide deck with Zorto. The example uses a reveal.js template, but the content model also works with native HTML/CSS/JS deck templates.

## 1. Create the presentation template

Add a `presentation.html` template to your site's `templates/` directory. This template assembles slides into a deck. See the [zorto.dev source](https://github.com/dkdc-io/zorto/tree/main/website/templates) for working template examples.

The template iterates `section.pages` and wraps each page's content in a `<section>` element. It can map `page.extra` fields to CSS classes, data attributes, backgrounds, keyboard behavior, or whatever the deck runtime expects.

## 2. Create the section

Create a directory for your presentation with an `_index.md`:

```
content/presentations/my-talk/_index.md
```

```toml
+++
title = "My Talk"
description = "A presentation about something interesting."
template = "presentation.html"
sort_by = "weight"
render_pages = false

[extra]
width = 1050
height = 700
transition = "slide"
reveal_theme = "black"
+++
```

Key settings:
- `template = "presentation.html"`: uses your presentation template
- `sort_by = "weight"`: orders slides by their weight field
- `render_pages = false`: slides only exist in the assembled deck
- `[extra]`: passes deck-level settings to the template

## 3. Add slides

Each slide is a markdown file in the presentation directory. Use `weight` to control order:

```toml
+++
title = "Welcome"
weight = 10

[extra]
layout = "center"
background_color = "#1a1a2e"
+++

# Welcome to my talk

*A subtitle goes here*
```

Increment weights by 10 to leave room for inserting slides later.

## 4. Use layouts and backgrounds

Control slide appearance via `[extra]` frontmatter:

| Field | Effect |
|-------|--------|
| `layout` | CSS class: `center`, `image-left`, `image-right`, `image-full`, `title` |
| `background_color` | Solid background color (e.g. `"#1a1a2e"`) |
| `background_image` | Background image path or URL |
| `background_size` | CSS background-size (e.g. `"cover"`, `"contain"`) |
| `background_opacity` | Background opacity (e.g. `"0.3"`) |
| `transition` | Template-specific transition setting |

## 5. Use presentation shortcodes

**Progressive reveal**: content appears on each click:

<pre><code>&#123;% fragment(style="fade-in") %&#125;
This appears first.
&#123;% end %&#125;

&#123;% fragment(style="fade-in") %&#125;
This appears second.
&#123;% end %&#125;</code></pre>

**Multi-column layout**:

<pre><code>&#123;% columns() %&#125;
Left column content

&lt;!-- column --&gt;

Right column content
&#123;% end %&#125;</code></pre>

With custom widths: `columns(widths="60%|40%")`.

**Speaker notes**: keep notes beside the slide source. Speaker-view behavior depends on the template:

<pre><code>&#123;% speaker_notes() %&#125;
Remember to mention the key point here.
&#123;% end %&#125;</code></pre>

**Positioned images**: place images at arbitrary coordinates:

<pre><code>&#123;&#123; slide_image(src="logo.png", top="10%", right="5%", width="200px") &#125;&#125;</code></pre>

## 6. Build and preview

```bash
zorto preview --open
```

Navigation depends on the template. The zorto.dev native deck supports arrow keys, space, and fullscreen.
