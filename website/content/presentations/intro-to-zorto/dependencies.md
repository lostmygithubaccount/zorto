+++
title = "Dependencies"
weight = 52

[extra]
layout = "wide"
slide_theme = "paper"
+++

## Dependencies are explicit

{% columns(widths="48%|52%") %}

```toml
# website/pyproject.toml
[project]
dependencies = [
  "matplotlib",
  "plotly",
  "pandas",
  "numpy",
]

# build_meta.py
# /// script
# dependencies = [
#   "duckdb==1.5.2",
]
```

<!-- column -->

- **Executable blocks**: Zorto activates the site `.venv`
- **Website builds**: `uv sync` installs chart/data deps
- **Pipelines**: `uv run --locked --script` owns script deps
- **Distribution**: `uv tool install zorto` installs the Rust-backed CLI

{% end %}

No hidden global Python. No generated dependency soup.
