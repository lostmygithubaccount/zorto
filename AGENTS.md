# zorto

The AI-native static site generator (SSG) with executable code blocks, inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

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

## website (zorto.dev)

The `website/` directory is the zorto.dev project site, built with zorto itself.

```bash
bin/preview        # Build + preview with --sandbox . (uses cargo run)
website/bin/preview # Same thing (delegates to bin/preview)
```

**Important**: The website uses `content_dirs` to pull in `../docs` and shortcodes like `configref` that reference files outside the website directory (e.g. `../crates/`). Always use `--sandbox .` (repo root) when building:

```bash
cargo run -p zorto -- --root website --sandbox . build
cargo run -p zorto -- --root website --sandbox . preview --open
```

Never run `zorto build` directly inside `website/` — it will fail with sandbox errors. Use `bin/preview` or pass `--sandbox .` from the repo root.

## Claude Code instructions

- DO NOT depend on any dkdc-* packages. This is a standalone open source project.
- do clone Zola & Quarto (and any other repos) into the .gitignored 'external' directory and use them as references for how to implement the features in this project.
- Python distribution (via PyO3/maturin) is a hard requirement. The `crates/zorto-py/` bindings and `py/zorto/` wrapper must be maintained.

## threat model

- **site authors** (config, templates, theme frontmatter): trusted. They own the build environment — Python code blocks execute with their credentials.
- **post authors** (markdown bodies, shortcode args): partly trusted in multi-author setups. Shortcode validators (`is_safe_css_length`, class allow-list, etc.) treat this as the untrusted boundary.
- **end readers** (browser): fully untrusted outputs — any path from author content to executed JS/CSS in their browser must be inspected (XSS through viz specs, CSS injection through shortcode args, theme URL injection, CDN supply chain).
- **Python code blocks** execute at build time in the author's venv — by design, NOT a sandbox.

## style conventions

- **Sentence case** for headings in documentation and on the website. Zorto is an independent open source project — it does NOT follow dkdc's all-lowercase brand style.
- "Zorto" (capitalized) in documentation prose; `zorto` for commands, code, URLs.
- built-in themes: `zorto`, `dkdc`, `default`, `ember`, `forest`, `ocean`, `rose`, `slate`, `midnight`, `sunset`, `mint`, `plum`, `sand`, `arctic`, `lime`, `charcoal`. All feature-gated Cargo features (default: all enabled). Python builds always include all themes. Every theme supports both light and dark mode via `[data-theme="light"]` CSS variable overrides.

## Zola reference

zorto references Zola's MIT-licensed code as a design guide. The reference copy in `external/zola/` **must stay at tag v0.21.0** — this is the last MIT-licensed release. Starting at v0.22.0, Zola switched to EUPL which is incompatible with our MIT license. Do NOT checkout, pull, or reference any Zola code beyond v0.21.0.
