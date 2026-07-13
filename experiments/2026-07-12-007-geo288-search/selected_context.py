#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


def load_comparator(repo: Path):
    path = repo / "experiments/2026-07-12-005-swc078-selection/compare_selected.py"
    spec = importlib.util.spec_from_file_location("selected_compare", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def selected_lines(
    path: Path, comparator
) -> list[tuple[int, int, str, tuple[str, ...]]]:
    records = []
    after_presaturation = False
    selection_calls = 0
    for line_number, raw_line in enumerate(
        path.read_text(encoding="utf-8", errors="replace").splitlines(), start=1
    ):
        line = raw_line.strip()
        if "Presaturation interreduction done" in line:
            after_presaturation = True
            continue
        if not after_presaturation:
            continue
        if line.startswith("% Failure:") or line.startswith("%% Failure:"):
            break
        if "SZS status" in line:
            break

        selection_calls += len(line) - len(line.lstrip("%"))

        c_match = comparator.C_CLAUSE.fullmatch(line)
        normalized = (
            comparator.normalize_c_clause(c_match.group(1))
            if c_match is not None
            else comparator.normalize_lop_clause(line)
        )
        if normalized is not None:
            records.append((line_number, selection_calls, raw_line, normalized))
    return records


def main() -> int:
    if len(sys.argv) != 5:
        print("usage: selected_context.py TRACE START END REPO", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    start = int(sys.argv[2])
    end = int(sys.argv[3])
    comparator = load_comparator(Path(sys.argv[4]))
    records = selected_lines(path, comparator)
    for ordinal in range(start, min(end, len(records)) + 1):
        line_number, selection_call, raw_line, normalized = records[ordinal - 1]
        print(
            f"{ordinal} line={line_number} selection_call={selection_call}: {raw_line}"
        )
        print(f"  normalized: {' | '.join(normalized)}")
    print(f"selected clauses: {len(records)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
