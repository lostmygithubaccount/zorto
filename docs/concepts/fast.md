# Fast

Zorto's build pipeline typically completes in under 100ms. The build runs on every save during development and on every push in CI, so build speed directly affects iteration speed.

## Benchmark

Build the zorto.dev site (40 pages, shortcodes, executable code blocks):

```bash
time zorto build
```

Typical result: under 1 second total. The build pipeline itself takes ~50ms; the rest is executable code block runtime.

## Architecture

{{ layers(items="Rust:Compiled to native code. No garbage collector pauses.:language|Efficient pipeline:Minimal allocations, streaming I/O. Pages rendered in parallel.:build|Embedded themes:Templates and styles compiled into the binary. No filesystem reads to load them.:themes|Self-contained:No runtime dependencies. No Node.js, no Ruby. Python is optional for executable code blocks.:binary") }}

## In practice

Executable code blocks add time proportional to the code being run — a `{python}` block that queries an API takes as long as the API call. The build pipeline itself stays fast regardless of site size. Use `zorto --no-exec preview` during development to skip code execution while editing prose.

## Further reading

- [Live reload](live-reload.md) — the dev server's rebuild-on-save loop
- [Executable code blocks](executable-code.md) — `--no-exec` flag
- [How to deploy](../how-to/deploy.md) — `zorto build` in CI/CD pipelines
