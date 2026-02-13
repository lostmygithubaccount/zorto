# Zorto

> **Under construction.** Zorto is in early development, check back soon.

A fast static site generator with executable code blocks inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

## Install

Recommended:

```bash
curl -LsSf https://dkdc.sh/zorto/install.sh | sh
```

uv:

```bash
uv tool install zorto
```

cargo:

```bash
cargo install zorto
```

You can use `uvx` to run it without installing:

```bash
uvx zorto
```

## Usage

```
zorto [OPTIONS] <COMMAND>
```

### Commands

| Command | Description |
|---------|-------------|
| `build` | Build the site |
| `preview` | Start preview server with live reload |
| `init` | Initialize a new site |
| `check` | Check site for errors without building |
| `clean` | Remove output directory |

### Quick start

```bash
# Create a new site
zorto init my-site
cd my-site

# Start dev server
zorto preview --open

# Build for production
zorto build
```

### `build`

```bash
zorto build [--output <dir>] [--drafts] [--base-url <url>]
```

### `preview`

```bash
zorto preview [--port <port>] [--drafts] [--open]
```

## Features

- TOML frontmatter (`+++` delimited)
- Syntax-highlighted code blocks
- Executable code blocks (`` ```{python} ``)
- Shortcodes (inline and block)
- File includes (`{{ include(path="...") }}`)
- SASS/SCSS compilation
- Live reload preview server
- Taxonomy and pagination support
- Sitemap and Atom feed generation
- Internal link validation
