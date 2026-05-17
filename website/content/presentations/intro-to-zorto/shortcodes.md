+++
title = "Shortcodes"
weight = 85

[extra]
slide_theme = "paper"
+++

## Shortcodes: rich content without HTML

{% columns(widths="52%|48%") %}

### Built-in

- **`slide_image`**: positioned or inline images
- **`fragment`**: progressive reveal
- **`columns`**: side-by-side layout
- **`speaker_notes`**: source-level presenter notes

<!-- column -->

### Custom

- Drop a `.tera` template in `shortcodes/`
- Call it from any markdown page
- Keep repeated HTML out of prose
- Validate author-facing inputs at shortcode boundaries
- Test the rendered result with `zorto check`

{% end %}

{% fragment(style="highlight-green") %}
**The whole presentation you're watching** is rendered from these shortcodes.
{% end %}
