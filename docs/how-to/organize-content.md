# Organize content with sections

Structure your site for different content types, audiences, or topics using Zorto's [section](../concepts/glossary.md#section) system.

## Multiple content sections

A typical site might have:

{% tree(caption="Each section is independent — its own sort order, pagination, and template.") %}
content/
  _index.md  [section: homepage]
  posts/
    _index.md  [section: blog]
  docs/
    _index.md  [section: documentation]
  projects/
    _index.md  [section: portfolio]
  about.md  [page]
{% end %}

## Section-specific templates

Assign templates per section:

```toml
# content/docs/_index.md
+++
title = "Documentation"
template = "docs-section.html"
+++
```

Pages within the section can also use a custom template:

```toml
# content/docs/getting-started.md
+++
title = "Getting started"
template = "docs-page.html"
+++
```

## Nested sections

Sections can nest:

{% tree() %}
content/docs/
  _index.md  [section]
  getting-started/
    _index.md  [section]
    installation.md  [page]
  reference/
    _index.md  [section]
    cli.md  [page]
{% end %}

Access subsections in templates via `section.subsections`.

## External content directories

Pull content from outside the `content/` directory:

```toml
# config.toml
[[content_dirs]]
path = "../docs"
url_prefix = "docs"
template = "docs.html"
section_template = "docs-section.html"
sort_by = "title"
rewrite_links = true
```

This is useful for documentation that lives alongside source code in a separate directory or repository.

## Related guides

- [Content model](../concepts/content-model.md) — sections, pages, and frontmatter in depth
- [Configuration reference](../reference/config.md) — full `content_dirs` field reference
