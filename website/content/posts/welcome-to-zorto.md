+++
title = "introducing zorto v0.12"
date = "2026-03-28"
author = "Cody"
description = "built-in themes, shortcodes, GitHub-style callouts, and comprehensive documentation."
tags = ["release"]
+++

Zorto v0.12 is a big step toward a production-ready static site generator. here's what's new.

<!-- more -->

## built-in themes

Zorto now ships with three built-in themes: `dkdc`, `light`, and `dark`. set `theme = "dkdc"` in your `config.toml` and you get a complete site with navbar, footer, theme toggle, and responsive design -- no local templates or SCSS needed.

all themes support light/dark mode toggling. the theme name picks the aesthetic and default mode. override any template or SCSS variable locally; your files always win.

themes are [Cargo features](/docs/concepts/themes/) -- you can disable ones you don't need for smaller binaries. Python builds include all themes.

## built-in shortcodes

six new shortcodes ship with Zorto:

- **note** -- callout/admonition boxes (info, warning, danger, tip)
- **details** -- collapsible sections
- **figure** -- images with captions
- **youtube** -- responsive video embeds
- **gist** -- GitHub gist embeds
- **mermaid** -- diagrams

see the full [shortcode reference](/docs/reference/shortcodes/) for examples.

## GitHub-style callouts

write callouts with standard GitHub alert syntax:

> [!TIP]
> Zorto renders these natively using pulldown-cmark's GFM support. no shortcode needed.

five types: `NOTE`, `TIP`, `WARNING`, `CAUTION`, `IMPORTANT`. see the [callouts reference](/docs/reference/callouts/).

## template linting

`zorto check` now warns about hardcoded strings in templates. the idea: user-facing text belongs in `config.toml` or content files, not in HTML templates. this makes themes reusable and sites maintainable.

use `--deny-warnings` in CI to enforce it.

## documentation

the [docs](/docs/getting-started/) are organized into four sections: getting started, concepts, how-to guides, and reference. the [CLI reference](/docs/reference/cli/) uses executable code blocks -- it updates itself every build.

## install

```bash
curl -sSL https://dkdc.sh/zorto | bash
```

or via [crates.io](https://crates.io/crates/zorto) / [PyPI](https://pypi.org/project/zorto/).

## roadmap to v1.0.0

v0.12 is the v1-ready release. what remains:

- polish the light and dark themes
- search (full-text index)
- i18n support
- more shortcodes based on community feedback
- sentence-case convention shift (like Go)

follow along on [GitHub](https://github.com/dkdc-io/zorto).
