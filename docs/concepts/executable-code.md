# Executable code blocks

Zorto can execute Python and Bash code blocks at build time and render their output inline.

{{ flow(steps="Write:Markdown with code blocks|Parse:Find executable blocks|Execute:Run via PyO3 or shell|Render:Output inlined in HTML", caption="Code runs at build time. The output is baked into static HTML — no JavaScript, no runtime.") }}

## Python blocks

Use the `{python}` language tag:

````markdown
```{python}
import math
print(f"Pi is approximately {math.pi:.4f}")
```
````

At build time, Zorto runs the code and inserts the output below the block.

## Bash blocks

Use the `{bash}` (or `{sh}`) language tag:

````markdown
```{bash}
echo "Hello from $(uname)"
date +%Y-%m-%d
```
````

## File attribute

Run a script file instead of inline code:

````markdown
```{python file="scripts/analysis.py"}
```
````

The file path is relative to the content file's directory.

## Disabling execution

To build without executing code blocks:

```bash
zorto --no-exec build
```

This renders the code blocks as plain syntax-highlighted code.

## Output rendering

- **stdout** is captured and displayed as a code block below the source
- **stderr** is displayed as a warning block
- **Non-zero exit codes** produce an error block with the return code

> [!TIP]
> Executable code blocks are great for keeping CLI references up to date. Use `zorto --help` in a `{bash}` block and the docs always match the current version.

## Python runtime

Zorto embeds Python via [PyO3](https://pyo3.rs/) — code blocks run in-process, not by shelling out. If a `.venv` directory exists at or above the site root (or `VIRTUAL_ENV` is set), Zorto activates its site-packages automatically, giving code blocks access to installed packages.

## Security considerations

Executable code blocks run with the same permissions as the `zorto` process. In CI environments, treat executable code blocks like any other build script — review content before building untrusted markdown. Use `zorto --no-exec build` to skip execution when building untrusted content.

## Further reading

- [How to use executable code blocks](../how-to/executable-code-blocks.md) — setup, file attribute, error handling
- [CLI reference](../reference/cli.md) — `--no-exec` flag and other options
- [AI-native](ai-native.md) — how executable code blocks fit into Zorto's design philosophy
