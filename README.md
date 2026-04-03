# Zorto

[![GitHub Release](https://img.shields.io/github/v/release/dkdc-io/zorto?color=blue)](https://github.com/dkdc-io/zorto/releases)
[![PyPI](https://img.shields.io/pypi/v/zorto?color=blue)](https://pypi.org/project/zorto/)
[![crates.io](https://img.shields.io/crates/v/zorto?color=blue)](https://crates.io/crates/zorto)
[![CI](https://img.shields.io/github/actions/workflow/status/dkdc-io/zorto/ci.yml?branch=main&label=CI)](https://github.com/dkdc-io/zorto/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-8A2BE2.svg)](https://github.com/dkdc-io/zorto/blob/main/LICENSE)

The AI-native static site generator (SSG) with executable code blocks, inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

**Warning**: While I use Zorto for many static websites including [zorto.dev](https://zorto.dev), I do not consider it production-ready for usage by others yet.

## Install

Recommended:

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

Pre-built binaries are available for Linux and macOS via Python (`uv`). Windows users should install via `cargo` or use macOS/Linux.

uv:

```bash
uv tool install zorto
```

cargo:

```bash
cargo install zorto
```

Verify installation:

```{bash}
zorto --version
```

You can use `uvx` to run it without installing:

```bash
uvx zorto
```

## Usage

```{bash}
zorto --help
```

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
