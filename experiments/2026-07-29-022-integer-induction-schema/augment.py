#!/usr/bin/env python3
"""Add the restricted integer-induction schema to one TPTP problem."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Sequence

from schema import SchemaError, augment_problem


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    source = arguments.input.resolve().read_text(encoding="utf-8")
    augmented, schema = augment_problem(source)
    output = arguments.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(augmented, encoding="utf-8")
    metadata = {
        "schema_version": 1,
        "schema_name": schema.name,
        "schema_id": schema.schema_id,
        "conjecture_name": schema.target.conjecture_name,
        "source_form": schema.target.source_form,
        "variable": schema.target.variable,
        "bound": schema.target.bound,
        "property": schema.target.property,
    }
    if arguments.metadata is not None:
        metadata_path = arguments.metadata.resolve()
        metadata_path.parent.mkdir(parents=True, exist_ok=True)
        metadata_path.write_text(
            json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
    print(json.dumps(metadata, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, SchemaError, ValueError) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error

