# AI-native

Zorto is a static site generator built for workflows where humans and AI agents collaborate on websites.

## Design principles

- **Explicit contracts.** One `config.toml` and markdown files with TOML frontmatter. No implicit file conventions or magic directories.
- **Markdown-first content.** AI models understand markdown natively. No proprietary content formats.
- **Predictable output.** Same input produces the same output. No runtime state, no database, no visitor-side side effects.
- **Strings in config, not templates.** Text content belongs in `config.toml` or frontmatter, not hard-coded in HTML.
- **Executable code blocks.** Dynamic output at build time, baked into static HTML.

## Build-time validation

Zorto validates your site during the build:

{{ flow(steps="Write:Content and config|Build:Compile the site|Check:Validate structure and links|Output:HTML or clear errors") }}

Current checks: internal link validation (broken `@/` references), template syntax, missing files, invalid configuration. Planned: mobile-friendliness, accessibility, semantic HTML validation.

## llms.txt

Zorto generates an [llms.txt](https://llmstxt.org/) file at build time by default — a machine-readable index of your entire site. An agent can read a single URL and understand the site's structure and content without crawling.

This creates two files:
- `/llms.txt` — links to markdown versions of every page
- `/llms-full.txt` — the full content of every page in a single file

This is for consumption, not editing. Agents that need to modify a site work directly on the filesystem.

## Markdown file generation

When enabled, Zorto generates a `.md` version of every page alongside the HTML, accessible at the same URL with a `.md` extension:

```toml
generate_md_files = true
```

## Further reading

- [Configuration](configuration.md) — the `config.toml` contract
- [Executable code blocks](executable-code.md) — dynamic output at build time
- [Content model](content-model.md) — sections, pages, and frontmatter
- [How to optimize for SEO](../how-to/seo.md) — llms.txt, Open Graph, and discoverability
