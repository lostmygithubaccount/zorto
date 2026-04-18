# Zorto

[![GitHub Release](https://img.shields.io/github/v/release/dkdc-io/zorto?color=blue)](https://github.com/dkdc-io/zorto/releases)
[![PyPI](https://img.shields.io/pypi/v/zorto?color=blue)](https://pypi.org/project/zorto/)
[![crates.io](https://img.shields.io/crates/v/zorto?color=blue)](https://crates.io/crates/zorto)
[![CI](https://img.shields.io/github/actions/workflow/status/dkdc-io/zorto/ci.yml?branch=main&label=CI)](https://github.com/dkdc-io/zorto/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/dkdc-io/zorto/blob/main/LICENSE)

The AI-native static site generator (SSG) with executable code blocks, inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

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

## What Zorto is not

- **Not a JavaScript SSG.** The engine is Rust, output is static HTML/CSS. There is no Node toolchain, bundler, or framework runtime.
- **Not a notebook.** Code blocks execute at build time; the output is static HTML rendered alongside the source. There is no in-browser kernel, no reactive cells, no stateful session.
- **Not a sandbox.** `{python}` and `{bash}` blocks run in the author's environment with the author's credentials. Treat zorto sites the way you treat a Makefile: trust the author, or pass `--no-exec`.
- **Not a headless CMS.** The `--webapp` mode is a local single-user editor for your own content tree. It is not multi-tenant and is not meant to be exposed on the public internet.
- **Not a general-purpose templating engine.** Tera templates are a build-time detail. Zorto renders sites, not arbitrary programs.
