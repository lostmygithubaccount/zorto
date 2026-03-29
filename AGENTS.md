# zorto

A fast static site generator inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

## architecture

```
crates/
  zorto-core/       # Pure library: site model, build pipeline, rendering
  zorto-cli/        # CLI binary + preview server (published as "zorto")
  zorto-webapp/     # HTMX CMS webapp (feature-gated, under development)
  zorto-app/        # Desktop app (future, stubbed)
  zorto-py/         # PyO3 bindings (own workspace, built by maturin)
py/zorto/           # Python package wrapper
```

- `zorto-core` has no axum/tokio/clap deps — pure library with only filesystem I/O
- `zorto-cli` crate name is `zorto` (the published binary) with optional features: `webapp`, `app`
- `zorto-py` is excluded from the workspace (cdylib, built by maturin)

Crates.io: `zorto-core`, `zorto`. PyPI: `zorto`. Installed binary: `zorto`.

## development

```bash
bin/build          # Build all (Rust + Python)
bin/build-rs       # Build Rust workspace
bin/build-py       # Build Python bindings (maturin develop)
bin/check          # Run all checks (format, lint, test)
bin/check-rs       # Rust checks (fmt, clippy, test)
bin/check-py       # Python checks (ruff, ty)
bin/format         # Format all code
bin/test           # Run all tests
bin/install        # Install CLI (Rust + Python)
bin/bump-version   # Bump version (--patch, --minor (default), --major)
```

Rust checks: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
Python checks: `ruff check .`, `ruff format --check .`, `ty check`

## Claude Code instructions

- DO NOT depend on any dkdc-* packages. This is a standalone open source project.
- do clone Zola & Quarto (and any other repos) into the .gitignored 'external' directory and use them as references for how to implement the features in this project.
- Python distribution (via PyO3/maturin) is a hard requirement. The `crates/zorto-py/` bindings and `py/zorto/` wrapper must be maintained.

## style conventions

- **all lowercase** for nav items, menu text, UI labels, and headings on the website (dkdc brand style). Sentence case is planned for v1.0.0 (like Go's convention shift).
- "Zorto" (capitalized) in documentation prose; `zorto` for commands, code, URLs.
- built-in themes: `dkdc`, `light`, `dark`. All feature-gated Cargo features (default: all enabled). Python builds always include all themes.

## Zola reference

zorto references Zola's MIT-licensed code as a design guide. The reference copy in `external/zola/` **must stay at tag v0.21.0** — this is the last MIT-licensed release. Starting at v0.22.0, Zola switched to EUPL which is incompatible with our MIT license. Do NOT checkout, pull, or reference any Zola code beyond v0.21.0.
