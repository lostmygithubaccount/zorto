"""Zorto — the AI-native static site generator."""

import sys

from zorto.core import (
    Config,
    Page,
    Section,
    Site,
    build,
    load,
    run_cli as _run_cli,
    version,
)

__all__ = [
    "build",
    "Config",
    "load",
    "main",
    "Page",
    "run_cli",
    "Section",
    "Site",
    "version",
]


def run_cli(argv: list[str] | None = None) -> None:
    """Run the zorto CLI with the given arguments."""
    if argv is None:
        argv = sys.argv
    try:
        _run_cli(argv)
    except KeyboardInterrupt:
        sys.exit(130)


def main() -> None:
    """CLI entry point."""
    run_cli()
