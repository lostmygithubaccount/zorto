+++
title = "Presentation control"
weight = 75

[extra]
background_image = "/zorto-mark-transparent.png"
background_size = "420px"
background_opacity = "0.14"
slide_theme = "ink"
+++

## Config drives the deck

```toml
+++
title = "Data apps"
weight = 58

[extra]
slide_theme = "ink"
layout = "wide"
+++
```

Slide source stays readable. The template owns the runtime and visual system.

{% speaker_notes() %}
This slide demonstrates per-slide styling through frontmatter. The template
applies the shell; the markdown stays focused on content.
{% end %}
