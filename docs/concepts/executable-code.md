Zorto can execute Python and Bash code blocks at build time and render their output inline.

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
zorto build --no-exec
```

This renders the code blocks as plain syntax-highlighted code.

## Output rendering

- **stdout** is captured and displayed as a code block below the source
- **stderr** is displayed as a warning block
- **Non-zero exit codes** produce an error block with the return code

> [!TIP]
> Executable code blocks are great for keeping CLI references up to date. Use `zorto --help` in a `{bash}` block and the docs always match the current version.
