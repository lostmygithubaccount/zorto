# Zorto

[![GitHub Release](https://img.shields.io/github/v/release/dkdc-io/zorto?color=blue)](https://github.com/dkdc-io/zorto/releases)
[![PyPI](https://img.shields.io/pypi/v/zorto?color=blue)](https://pypi.org/project/zorto/)
[![crates.io](https://img.shields.io/crates/v/zorto?color=blue)](https://crates.io/crates/zorto)
[![CI](https://img.shields.io/github/actions/workflow/status/dkdc-io/zorto/ci.yml?branch=main&label=CI)](https://github.com/dkdc-io/zorto/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/dkdc-io/zorto/blob/main/LICENSE)

Zorto turns plain Markdown into polished websites, blog posts, and slide decks — same files, same workflow, no HTML gymnastics. Point it at a folder of `.md` files and get a production-ready site with search, themes, and a reveal.js-powered presentation mode (arrow keys, speaker view, fullscreen). Designed so you — or an AI agent working alongside you — can draft a landing page, an engineering blog post, or your next all-hands deck in minutes, not days.

Prior art: inspired by [Zola](https://github.com/getzola/zola) (fast Rust SSG) and [Quarto](https://github.com/quarto-dev/quarto-cli) (executable documents). Zorto is MIT-licensed and free and open source forever.

## Install

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

Verify:

```{bash}
zorto --version
```

<details>
<summary>More install options</summary>

The `curl | sh` installer above wraps `uv tool install zorto`. On Linux and macOS it pulls a pre-built wheel from PyPI that bundles the Rust engine (no compile step). Windows is not covered by the installer — use `cargo` from source or run under WSL.

**uv (PyPI wheel, no compile):**

```bash
uv tool install zorto
```

**uvx (run once without installing):**

```bash
uvx zorto
```

**cargo (build from source, requires Rust 1.85+):**

```bash
cargo install zorto
```

</details>

## Usage

```{bash}
zorto --help
```

## Slide decks

Zorto ships a reveal.js-powered presentation mode: one markdown file per slide, with frontmatter for layout, background, and transitions. Arrow keys, speaker view, fullscreen, and overview mode are built in, so a deck is just a directory of `.md` files that a human or agent can draft, reorder, or hand off without touching HTML.

- [Live intro deck](https://zorto.dev/presentations/intro-to-zorto/) — what a Zorto-built deck looks like.
- [Create a presentation](https://zorto.dev/docs/how-to/create-presentation/) — step-by-step guide.
- [Presentations concept](https://zorto.dev/docs/concepts/presentations/) — the content model behind decks.

## Executable code blocks

Use `{bash}` or `{python}` in code blocks to execute them.

```{bash}
echo "hello"
```

```{bash}
echo "Built on $(uname -s) $(uname -m) at $(date -u '+%Y-%m-%d %H:%M UTC')"
```

```{python}
for i in range(5):
    print(i)
```

> [!TIP]
> If you're reading elsewhere, see [https://zorto.dev](https://zorto.dev/#executable-code-blocks) for the rendered results of the code blocks above.
