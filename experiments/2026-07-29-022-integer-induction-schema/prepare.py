#!/usr/bin/env python3
"""Prepare one baseline with redundant standard integer symbol types."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Sequence

from schema import SchemaError, prepare_problem


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    prepared = prepare_problem(
        arguments.input.resolve().read_text(encoding="utf-8")
    )
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(prepared, encoding="utf-8")
    print(f"OK: prepared {output}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, SchemaError, ValueError) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
