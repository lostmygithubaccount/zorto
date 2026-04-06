+++
title = "zorto.dev"
+++

## Executable code blocks

Use `{bash}` or `{python}` in fenced code blocks to execute them at build time. Output is baked into static HTML.

```{bash}
zorto --version
```

```{bash}
echo "Built on $(uname -s) $(uname -m) at $(date -u '+%Y-%m-%d %H:%M UTC')"
```

```{python}
for i in range(5):
    print(i)
```

> [!TIP]
> See the [getting started guide](/docs/getting-started/) to build your own site, or browse the [source on GitHub](https://github.com/dkdc-io/zorto).
