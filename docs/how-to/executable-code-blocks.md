# Use executable code blocks

Run Python and Bash at build time to generate dynamic content in your static site.

{{ flow(steps="Write:Code in markdown|Build:Zorto executes blocks|Render:Output baked into HTML", caption="Code runs at build time — the output becomes part of the static site.") }}

## Setup

No setup required for Bash blocks. For Python blocks, Zorto uses an embedded interpreter. If you need third-party packages, create a virtual environment:

```bash
uv init --bare
uv add pandas matplotlib
```

Zorto automatically activates the `.venv` in your project root.

## Write a Python block

Use the `{python}` language tag in your markdown:

````markdown
```{python}
from datetime import datetime
print(f"Last built: {datetime.now():%Y-%m-%d %H:%M}")
```
````

The output appears below the code block in the rendered page.

## Write a Bash block

````markdown
```{bash}
echo "Running on $(uname -s) $(uname -m)"
```
````

## Run a script file

For longer scripts, use the `file` attribute:

````markdown
```{python file="scripts/generate_chart.py"}
```
````

The path is relative to the content file's directory.

## Keep CLI docs up to date

Use executable blocks to ensure documentation always matches the current version:

````markdown
```{bash}
zorto --help
```
````

Every build regenerates the output, keeping the docs in sync with the current version.

## Preview without execution

```bash
zorto --no-exec preview
```

Code blocks render as static syntax-highlighted code. Useful for faster iteration when you're editing prose, not code.

## Error handling

- **stdout** renders as a code block below the source
- **stderr** renders as a warning block
- **Non-zero exit codes** render as an error block with the return code

Errors don't stop the build — other pages continue rendering.

## Related guides

- [Executable code blocks](../concepts/executable-code.md) — concept overview, Python runtime, security
- [CLI reference](../reference/cli.md) — `--no-exec` flag and other options
