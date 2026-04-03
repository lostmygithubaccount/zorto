# Troubleshooting

Common issues and how to fix them.

## Build errors

### "shortcode template not found"

You referenced a shortcode that doesn't exist. Check the spelling — Zorto's built-in shortcodes are: `include`, `tabs`, `note`, `details`, `figure`, `youtube`, `gist`, `mermaid`, `pyref`, `configref`, `flow`, `layers`, `tree`, `compare`, `cascade`.

This also happens when Tera template syntax appears in your Markdown content and gets interpreted as a shortcode. Any pattern with double curly braces and parentheses is treated as a shortcode call — even inside fenced code blocks. To show template syntax safely in documentation, use HTML entities inside `<pre><code>` blocks.

### "path escapes sandbox boundary"

The `include` or `configref` shortcode tried to read a file outside the allowed directory. Zorto restricts file access for security. Use the `--sandbox` flag to widen the boundary:

```bash
zorto --root mysite --sandbox . build
```

### Executable code block errors

If a `{python}` block fails, check:
- **Python not found**: Zorto embeds Python via PyO3, but your system needs Python available. The shell installer and PyPI package include it.
- **Missing packages**: If your code imports third-party packages, create a virtual environment with `uv init --bare && uv add <package>`. Zorto automatically activates `.venv` at or above the site root.
- **Stderr output**: stderr renders as a warning block, not as an error. Check the rendered page for warning-styled output.

Use `zorto --no-exec build` to skip code execution entirely.

### Template errors

Tera template syntax errors produce messages like `Failed to render template`. Common causes:
- Missing `{% endblock %}` or `{% endif %}`
- Using `{{ page.title }}` in a section template (use `section.title` instead)
- Accessing a variable that doesn't exist (use `| default(value="")` to handle missing values)

## Preview server issues

### Port already in use

If port 1111 is occupied, specify a different one:

```bash
zorto preview --port 8080
```

### Not accessible from phone/tablet

The preview server binds to `127.0.0.1` by default (localhost only). To access from other devices on your network:

```bash
zorto preview --interface 0.0.0.0
```

### Changes not showing

The preview server watches for file changes and rebuilds automatically. If changes aren't appearing:
- Check that you saved the file
- Check the terminal for build errors
- Rust code changes (if building from source) require restarting the server

## Deployment issues

### Build succeeds locally but fails in CI

- Make sure the CI environment installs Zorto before building
- Check that executable code blocks have their dependencies available in CI (or use `--no-exec`)
- Verify `base_url` in `config.toml` matches your production domain

### Broken links after deploy

If internal links break after deployment, check that [`base_url`](../concepts/glossary.md#base-url) is set correctly. Links generated with the [`@/` prefix](../concepts/content-model.md#internal-links) are resolved at build time — they should work if the target file exists.

## Getting help

- [GitHub issues](https://github.com/dkdc-io/zorto/issues) — report bugs and request features
- [CLI reference](../reference/cli.md) — all available commands and flags
- [Configuration reference](../reference/config.md) — every config option
