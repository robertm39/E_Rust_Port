#!/usr/bin/env python3
"""Persistent newline-JSON inference boundary for the recursive candidate."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from neural_common import RecursiveEncoder, RecursiveModel, load_model, parse_clause


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True, type=Path)
    arguments = parser.parse_args()
    model = load_model(arguments.model)
    if not isinstance(model, RecursiveModel):
        raise SystemExit("inference worker requires a recursive model")
    encoder = RecursiveEncoder(model.seed)

    for line in sys.stdin:
        try:
            request = json.loads(line)
            clauses = request["clauses"]
            if not isinstance(clauses, list) or not all(
                isinstance(clause, str) for clause in clauses
            ):
                raise ValueError("'clauses' must be a list of strings")
            scores = [
                model.score_clause(parse_clause(clause), encoder) for clause in clauses
            ]
            response = {"scores": scores}
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            response = {"error": str(error)}
        sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
