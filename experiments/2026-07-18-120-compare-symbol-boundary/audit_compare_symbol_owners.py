#!/usr/bin/env python3
"""Audit the cache-only comparison values and printable OCB owner boundary."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


EXPECTED_C_CACHE_FILES = {
    "eprover/ORDERINGS/cto_cmpcache.c",
    "eprover/ORDERINGS/cto_lpo.c",
}
EXPECTED_RUST_CACHE_MENTION_FILES = {
    "src/clauses/clause.rs",
    "src/clauses/eqn.rs",
    "src/clauses/eqnlist.rs",
    "src/learn/indexfunctions.rs",
    "src/learn/patterns.rs",
    "src/orderings/cto_cmpcache.rs",
    "src/orderings/cto_kbolin.rs",
    "src/orderings/cto_lpo.rs",
    "src/orderings/ocb.rs",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def relative(repo: Path, path: Path) -> str:
    return path.relative_to(repo).as_posix()


def source_files(root: Path, suffixes: set[str]) -> list[Path]:
    return sorted(path for path in root.rglob("*") if path.suffix in suffixes)


def collect(repo: Path) -> dict[str, Any]:
    c_partial_path = repo / "eprover/BASICS/clb_partial_orderings.c"
    c_header_path = repo / "eprover/BASICS/clb_partial_orderings.h"
    c_ocb_path = repo / "eprover/ORDERINGS/cto_ocb.c"
    c_orderings_path = repo / "eprover/ORDERINGS/cto_orderings.c"
    rust_partial_path = repo / "src/basics/partial_orderings.rs"
    rust_ocb_path = repo / "src/orderings/ocb.rs"

    c_partial = c_partial_path.read_text(encoding="utf-8")
    c_header = c_header_path.read_text(encoding="utf-8")
    c_ocb = c_ocb_path.read_text(encoding="utf-8")
    c_orderings = c_orderings_path.read_text(encoding="utf-8")
    rust_partial = rust_partial_path.read_text(encoding="utf-8")
    rust_ocb = rust_ocb_path.read_text(encoding="utf-8")

    c_table_match = re.search(
        r"POCompareSymbol\s*\[\s*\]\s*=\s*\{(?P<body>.*?)\};",
        c_partial,
        re.DOTALL,
    )
    c_table_entries = (
        re.findall(r'"(?:[^"\\]|\\.)*"', c_table_match.group("body"))
        if c_table_match
        else []
    )

    c_symbol_index_sites = []
    for path in source_files(repo / "eprover", {".c", ".h"}):
        source = path.read_text(encoding="utf-8")
        count = len(re.findall(r"POCompareSymbol\s*\[\s*OCBFunCompare\s*\(", source))
        if count:
            c_symbol_index_sites.append({"path": relative(repo, path), "count": count})

    rust_symbol_call_sites = []
    for path in source_files(repo / "src", {".rs"}):
        if path == rust_partial_path:
            continue
        source = path.read_text(encoding="utf-8")
        count = len(re.findall(r"\.symbol\s*\(\s*\)", source))
        if count:
            rust_symbol_call_sites.append({"path": relative(repo, path), "count": count})

    c_cache_files = set()
    for path in source_files(repo / "eprover", {".c", ".h"}):
        if path in {c_partial_path, c_header_path}:
            continue
        source = path.read_text(encoding="utf-8")
        if re.search(r"\bto_not(?:gteq|leeq)\b", source):
            c_cache_files.add(relative(repo, path))

    rust_cache_mention_files = set()
    for path in source_files(repo / "src", {".rs"}):
        if path == rust_partial_path:
            continue
        source = path.read_text(encoding="utf-8")
        if re.search(r"\bNot(?:Greater|Less)Equal\b", source):
            rust_cache_mention_files.add(relative(repo, path))

    checks = {
        "c_enum_has_seven_ordered_values": bool(
            re.search(
                r"to_unknown\s*=\s*0\s*,\s*"
                r"to_uncomparable\s*=\s*1\s*,\s*"
                r"to_equal\s*=\s*2\s*,\s*"
                r"to_greater\s*=\s*3\s*,\s*"
                r"to_lesser\s*=\s*4\s*,\s*"
                r"to_notgteq\s*,.*?to_notleeq",
                c_header,
                re.DOTALL,
            )
        ),
        "c_symbol_table_has_exactly_five_entries": c_table_entries
        == ['"*u*"', '"=/="', '" = "', '" > "', '" < "'],
        "c_symbol_table_is_indexed_only_by_ocb_debug_print": c_symbol_index_sites
        == [{"path": "eprover/ORDERINGS/cto_ocb.c", "count": 1}],
        "c_cache_values_are_confined_to_lpo_and_cache_owners": c_cache_files
        == EXPECTED_C_CACHE_FILES,
        "c_ocb_matrix_initializes_only_printable_relations": (
            "((i==j) ? to_equal : to_uncomparable)" in c_ocb
        ),
        "c_parser_produces_only_concrete_relations": all(
            marker in c_orderings
            for marker in ("res = to_lesser;", "res = to_greater;", "res = to_equal;")
        ),
        "rust_symbol_table_is_checked_and_five_entries": (
            "PO_COMPARE_SYMBOLS: [&str; 5]" in rust_partial
            and "PO_COMPARE_SYMBOLS.get(usize::from(self.c_value())).copied()"
            in rust_partial
        ),
        "rust_symbol_rendering_has_one_production_owner": rust_symbol_call_sites
        == [{"path": "src/orderings/ocb.rs", "count": 1}],
        "rust_ocb_rejects_cache_only_relations": (
            "only concrete precedence relations are inserted" in rust_ocb
            and "CompareResult::NotGreaterEqual" in rust_ocb
            and "CompareResult::NotLessEqual" in rust_ocb
        ),
        "rust_cache_mention_inventory_matches": rust_cache_mention_files
        == EXPECTED_RUST_CACHE_MENTION_FILES,
    }
    return {
        "schema_version": 1,
        "c_symbol_table_entries": c_table_entries,
        "c_symbol_index_sites": c_symbol_index_sites,
        "rust_symbol_call_sites": rust_symbol_call_sites,
        "c_cache_value_files": sorted(c_cache_files),
        "rust_cache_value_mention_files": sorted(rust_cache_mention_files),
        "checks": checks,
        "accepted": all(checks.values()),
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[2]
    result = collect(repo)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(rendered, encoding="utf-8")
    if args.expected is not None and rendered != args.expected.read_text(encoding="utf-8"):
        print(f"comparison-symbol owner mismatch: {args.output} != {args.expected}")
        return 1
    print(f"comparison-symbol owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
