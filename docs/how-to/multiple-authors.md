# Set up multiple authors

Use [taxonomies](../concepts/glossary.md#taxonomy) and page frontmatter to attribute content to different authors.

## Add an authors taxonomy

Define an `authors` taxonomy in `config.toml`:

```toml
base_url = "https://example.com"

[[taxonomies]]
name = "tags"

[[taxonomies]]
name = "authors"
```

This generates an author listing page at `/authors/` and individual pages for each author (e.g. `/authors/jane-doe/`).

## Tag pages with authors

Use the `authors` taxonomy in your page frontmatter:

```markdown
+++
title = "Building a blog with Zorto"
date = "2025-03-15"
authors = ["Jane Doe"]
+++

Your content here.
```

Multiple authors on a single page:

```markdown
+++
title = "Collaborative guide"
date = "2025-03-20"
authors = ["Jane Doe", "Alex Kim"]
+++
```

## Add author metadata

Use the `[extra]` table for additional author information like bio or avatar:

```markdown
+++
title = "Why static sites win"
date = "2025-04-01"
authors = ["Jane Doe"]

[extra]
author_url = "https://janedoe.com"
author_image = "/images/jane.jpg"
+++
```

Access these in templates:

```html
{% if page.extra.author_url %}
  <a href="{{ page.extra.author_url }}">
    <img src="{{ page.extra.author_image }}" alt="{{ page.authors[0] }}" />
  </a>
{% endif %}
```

## Create author templates

Add `templates/authors/single.html` for individual author pages (e.g., `/authors/jane-doe/`):

```html
<h1>{{ term.name }}</h1>
<ul>
{% for page in term.pages %}
  <li><a href="{{ page.permalink }}">{{ page.title }}</a></li>
{% endfor %}
</ul>
```

Add `templates/authors/list.html` for the authors index at `/authors/`:

```html
<h1>Authors</h1>
<ul>
{% for term in terms %}
  <li><a href="{{ term.permalink }}">{{ term.name }}</a> ({{ term.pages | length }} posts)</li>
{% endfor %}
</ul>
```

## Related guides

- [Add a blog](add-blog.md) — posts, tags, pagination
- [Templates](../concepts/templates.md) — template context variables and custom functions
