+++
title = "Installation"
template = "docs.html"
slug = "installation"
date = "2099-01-03"
+++

## Install script

The quickest way to install Zorto:

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

## From crates.io (Rust)

```bash
cargo install zorto
```

## From PyPI (Python)

```bash
uv tool install zorto
```

Or with pip:

```bash
pip install zorto
```

The Python package bundles the same Rust binary: no performance difference.

## Verify

```bash
zorto --version
```

You should see output like `zorto x.y.z`. You're ready to go.
