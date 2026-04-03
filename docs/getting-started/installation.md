# Installation

Zorto runs on macOS and Linux. Windows support is available via WSL.

The quickest way to install:

```bash
curl -LsSf https://dkdc.sh/zorto | sh
```

Alternatively, install from PyPI (requires Python 3.9+ and [uv](https://docs.astral.sh/uv/) or pip):

```bash
uv tool install zorto
```

The Python package includes the same Rust engine — there is no performance difference.

Or build from source (requires [Rust](https://www.rust-lang.org/tools/install) 1.75+):

```bash
cargo install zorto
```

Confirm it worked:

```bash
zorto --version
```

You should see something like `zorto 0.x.y`. If the command is not found, make sure the install location is in your `PATH`.

You're ready for the [tutorial](quick-start.md).
