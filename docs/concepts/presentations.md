# Presentations

Zorto builds slide decks from markdown: one file per slide, ordered by frontmatter, rendered by a presentation template. The template can be native HTML/CSS/JS, reveal.js, or a custom deck runtime. This follows Zorto's core principle: one markdown file per page.

## The model

A presentation is a section with `render_pages = false`. Each markdown file in the section becomes a slide. The section's `_index.md` controls presentation-level settings, and the slides are assembled into a single HTML file using a custom template.

{% tree(caption="Each slide is an independent markdown file. AI agents can create, edit, or reorder slides individually.") %}
content/presentations/intro/
  _index.md  [template=presentation.html, sort_by=weight]
  title.md  [weight=10, layout=center]
  overview.md  [weight=20]
  features.md  [weight=30, background_color=#16213e]
  demo.md  [weight=40]
  closing.md  [weight=50, layout=center]
{% end %}

## Why one file per slide

Traditional presentation tools (including Quarto) put all slides in a single file separated by headings or horizontal rules. Zorto takes a different approach:

- **AI-friendly**: an agent can create, modify, or reorder a single slide without parsing the entire presentation
- **Frontmatter per slide**: each slide declares layout, background, and any template-specific settings
- **Weight-based ordering**: the `weight` field controls slide order; increment by 10 to leave room for insertions
- **Git-friendly**: diffs show exactly which slides changed

## Key features

**Layouts**: predefined layouts via `[extra] layout`: `center`, `image-left`, `image-right`, `image-full`, `title`. Or use the default flow layout.

**Backgrounds**: set `background_color`, `background_image`, `background_size`, and `background_opacity` in `[extra]` to control per-slide backgrounds.

**Template control**: section and slide frontmatter are available to the template, so a site can define its own deck controls, visual treatment, and keyboard behavior.

**Progressive reveal**: use the `fragment` shortcode to reveal content incrementally on click.

**Speaker notes**: use the `speaker_notes` shortcode to keep notes beside the slide source. Whether notes render in a speaker view depends on the deck template.

**Multi-column layouts**: use the `columns` shortcode to split content side-by-side.

**Positioned images**: use the `slide_image` shortcode for absolute-positioned images anywhere on a slide.

## The `render_pages` field

When a section sets `render_pages = false`, its child pages are rendered to HTML (so their content is available in `section.pages`) but do not produce individual HTML output files. They are also excluded from the sitemap, feed, search data, and llms.txt. This is what makes presentations work.

## The `weight` field

Pages can have an optional `weight` field in frontmatter. When a section uses `sort_by = "weight"`, pages are sorted ascending by weight. Pages without a weight sort last; ties are broken by filename. This is useful beyond presentations — any section that needs explicit ordering.

## Further reading

- [How to create a presentation](../how-to/create-presentation.md): step-by-step guide
- [Shortcodes](shortcodes.md): all built-in shortcodes including presentation shortcodes
- [Content model](content-model.md): sections, pages, and frontmatter
