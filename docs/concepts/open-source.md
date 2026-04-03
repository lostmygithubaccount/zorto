# Free & open source

Zorto is [MIT-licensed](https://github.com/dkdc-io/zorto/blob/main/LICENSE). Use it for anything — personal sites, commercial projects, internal tools — with no restrictions beyond including the license and copyright notice.

## Why MIT?

MIT is one of the most permissive open source licenses. No copyleft requirements, no patent clauses, no usage restrictions.

Companies can embed Zorto in internal toolchains without legal review, and contributors can fork freely without license-compatibility concerns.

## Community

Zorto is developed in the open on [GitHub](https://github.com/dkdc-io/zorto). Issues, pull requests, and discussions are welcome. The project follows [semantic versioning](https://semver.org/) for both the Rust crate and Python package.

The codebase is Rust (core engine, CLI, themes) with Python bindings via PyO3. If you want to contribute, the [AGENTS.md](https://github.com/dkdc-io/zorto/blob/main/AGENTS.md) file in the repo root has the architecture overview and development commands.

## Distribution

| Channel | Command |
|---------|---------|
| Shell installer | `curl -LsSf https://dkdc.sh/zorto/install.sh \| sh` |
| PyPI | `uv tool install zorto` |
| crates.io | `cargo install zorto` |
| Source | `git clone https://github.com/dkdc-io/zorto` |

## Further reading

- [Installation](../getting-started/installation.md) — step-by-step install instructions
- [AI-native](ai-native.md) — Zorto's design philosophy
