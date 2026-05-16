# Python API

Zorto's Python API lets you load, inspect, and build sites programmatically. Install the package:

```bash
uv add zorto
```

The Python package includes the same Rust engine compiled as a native extension — there is no performance difference from the CLI.

## Quick start

```python
import zorto

# Load a site — returns a Site object
site = zorto.load(root=".")  # Site

# Access site data
print(site.config.title)     # Config
for page in site.pages:      # list[Page]
    print(page.title, page.permalink)

# Build the site
zorto.build(root=".")
```

## CLI from Python

You can also invoke the full CLI from Python:

```python
import zorto
zorto.run_cli(["build"])              # same as `zorto build`
zorto.run_cli(["preview", "--open"])  # same as `zorto preview --open`
```

## API surface

The Python package exposes a small wrapper around the Rust engine:

| Name | Kind | Purpose |
| --- | --- | --- |
| `build(root=".")` | Function | Build a site from Python. |
| `load(root=".")` | Function | Load a site and inspect its config, sections, and pages. |
| `run_cli(argv=None)` | Function | Run the Zorto CLI from Python. |
| `version()` | Function | Return the installed Zorto version. |
| `Config` | Class | Site configuration. |
| `Site` | Class | Loaded site model. |
| `Page` | Class | Renderable content page. |
| `Section` | Class | Content section with child pages. |
