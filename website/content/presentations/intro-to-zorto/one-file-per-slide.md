+++
title = "One file per slide"
weight = 80
+++

## Presentations: one file per slide

```
presentations/intro-to-zorto/
  _index.md          # presentation config
  title.md           # slide 1
  what-is-zorto.md   # slide 2
  why-another-ssg.md # slide 3
  ...
```

- Each slide is a markdown file with its own frontmatter
- `weight` field controls slide order
- `[extra]` controls background, transitions, and layout
- Shortcodes for columns, fragments, speaker notes, and positioned images
- Powered by **reveal.js** -- keyboard navigation, fullscreen, speaker view
