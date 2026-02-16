# zorto

A fast static site generator inspired by [Zola](https://github.com/getzola/zola) and [Quarto](https://github.com/quarto-dev/quarto-cli).

## Claude Code instructions

- DO NOT depend on any dkdc-* packages. This is a separate open source-ish project that we will eventually move to its own repository.
- do clone Zola & Quarto (and any other repos) into the .gitignored 'external' directory and use them as references for how to implement the features in this project.
- Python distribution (via PyO3/maturin) is a hard requirement. The `zorto-py/` bindings and `src/zorto/` wrapper must be maintained.

## Zola reference

zorto references Zola's MIT-licensed code as a design guide. The reference copy in `external/zola/` **must stay at tag v0.21.0** — this is the last MIT-licensed release. Starting at v0.22.0, Zola switched to EUPL which is incompatible with our MIT license. Do NOT checkout, pull, or reference any Zola code beyond v0.21.0.

