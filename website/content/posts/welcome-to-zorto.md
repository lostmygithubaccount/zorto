+++
title = "Introducing Zorto!"
date = "2026-03-28"
author = "Cody"
description = "The AI-native static site generator (SSG) built for AI and agents. MIT-licensed, free and open source forever."
tags = ["zorto"]
+++

Zorto is the AI-native static site generator (SSG) inspired by [Zola](https://www.getzola.org/) and [Quarto](https://quarto.org/).

<!-- more -->

## Built for AI and agents

Zorto is designed from the ground up for agentic software engineering. AI can create a full website in minutes and maintain it with ease: the config-driven architecture, built-in themes, and opinionated linting make it natural for both humans and agents to work with.

More on AI-native workflows coming soon as we approach v1.

## Executable code blocks

Zorto's defining feature. Fenced code blocks marked with `{python}` or `{bash}` run at build time:

```{bash}
echo "This ran at $(date +%Y-%m-%d)"
```

The output is rendered inline. This powers self-updating documentation: our [CLI reference](/docs/reference/cli/) runs `zorto --help` at build time, so the docs are always current.

## Built-in themes

Zorto ships with 16 built-in themes including `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`, and more. Set `theme = "dkdc"` in `config.toml` and you get a complete site: navbar, footer, theme toggle, responsive design.

All themes support light/dark mode toggling. Override any template or SCSS variable locally; your files always win.

## GitHub-style callouts

Write callouts with standard GitHub alert syntax:

> [!TIP]
> Zorto renders these natively using pulldown-cmark's GFM support. No shortcode needed.

Five types: `NOTE`, `TIP`, `WARNING`, `CAUTION`, `IMPORTANT`.

## Shortcodes

Nineteen built-in shortcodes, spanning content (`include`, `tabs`, `note`, `details`, `figure`, `youtube`, `gist`, `mermaid`), diagrams (`flow`, `layers`, `tree`, `compare`, `cascade`), API references (`pyref`, `configref`), and presentations (`slide_image`, `speaker_notes`, `fragment`, `columns`). Plus you can create your own with Tera templates.

## Template linting

`zorto check` warns about hardcoded strings in templates, inspired by clippy. User-facing text belongs in `config.toml` or content files, not in HTML templates. This keeps themes reusable.

## llms.txt

Zorto generates [llms.txt](/llms.txt) and [llms-full.txt](/llms-full.txt) by default: structured text files that help LLMs understand your site. This is enabled out of the box; set `generate_llms_txt = false` in `config.toml` to disable.

## What's next

We're working toward v1 with more built-in themes, support for additional languages in executable code blocks (beyond Python and Bash), and broader visualization support. More on all of that soon.

## Free and open source

Zorto is [MIT-licensed](https://github.com/dkdc-io/zorto/blob/main/LICENSE): free and open source forever. We may consider dual-licensing with Apache 2.0 in the future; [open an issue](https://github.com/dkdc-io/zorto/issues) if that would be useful for your project.

## Install

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

Or via [crates.io](https://crates.io/crates/zorto) / [PyPI](https://pypi.org/project/zorto/).

Check out the [getting started](/docs/getting-started/) guide, or browse the [source on GitHub](https://github.com/dkdc-io/zorto).
