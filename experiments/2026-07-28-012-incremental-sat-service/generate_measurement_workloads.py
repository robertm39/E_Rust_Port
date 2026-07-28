#!/usr/bin/env python3
"""Generate focused cancellation and proof-overhead workload sessions."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

from generate_workloads import clause_step, pigeonhole, query_step, write_session


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def generate(output: Path) -> list[dict[str, object]]:
    output.mkdir(parents=True, exist_ok=True)
    manifest: list[dict[str, object]] = []
    max_variable, clauses = pigeonhole(14, 13)
    for deadline_us in (100, 1_000, 10_000):
        path = output / f"cancel-pigeonhole-14-13-{deadline_us}us.isat"
        write_session(
            path,
            max_variable,
            [
                *(clause_step(clause) for clause in clauses),
                query_step("cancel", deadline_us=deadline_us),
            ],
        )
        manifest.append(
            {
                "kind": "cancellation",
                "deadline_us": deadline_us,
                "path": str(path),
                "sha256": digest(path),
            }
        )

    max_variable, clauses = pigeonhole(8, 7)
    path = output / "proof-pigeonhole-8-7.isat"
    write_session(
        path,
        max_variable,
        [
            *(clause_step(clause) for clause in clauses),
            query_step("solve"),
        ],
    )
    manifest.append(
        {
            "kind": "proof_overhead",
            "path": str(path),
            "sha256": digest(path),
        }
    )

    rendered = json.dumps(
        {"schema": 1, "sessions": manifest},
        indent=2,
        sort_keys=True,
    )
    (output / "manifest.json").write_text(rendered + "\n", encoding="utf-8")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    manifest = generate(arguments.output)
    rendered = json.dumps(
        {"schema": 1, "sessions": manifest},
        indent=2,
        sort_keys=True,
    )
    print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
