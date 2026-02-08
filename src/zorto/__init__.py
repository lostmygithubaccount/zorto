import sys

from zorto.core import run as _run


def run(argv: list[str] | None = None) -> None:
    """Run the zorto CLI with the given arguments."""
    if argv is None:
        argv = sys.argv
    try:
        _run(argv)
    except KeyboardInterrupt:
        sys.exit(0)


def main() -> None:
    """CLI entry point."""
    try:
        run()
    except KeyboardInterrupt:
        sys.exit(0)
