#!/usr/bin/env python3
"""Audit the FloatTree representation and direct production owner set."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def collect(repo: Path) -> dict[str, Any]:
    implementation_path = repo / "src/basics/floattrees.rs"
    implementation = implementation_path.read_text(encoding="utf-8")
    rust_owners = []
    for path in (repo / "src").rglob("*.rs"):
        if path == implementation_path:
            continue
        source = path.read_text(encoding="utf-8")
        count = len(re.findall(r"\bFloatTree\s*(?:<|::new|::<)", source))
        if count:
            rust_owners.append(
                {"path": path.relative_to(repo).as_posix(), "type_mentions": count}
            )

    c_owners = []
    for path in (repo / "eprover").rglob("*"):
        if path.suffix not in {".c", ".h"}:
            continue
        if path.name in {"clb_floattrees.c", "clb_floattrees.h"}:
            continue
        source = path.read_text(encoding="utf-8")
        count = len(
            re.findall(
                r"\bFloatTree(?:_p|Find|Store|Insert|Extract|Delete|Traverse|Free|Nodes)\b",
                source,
            )
        )
        if count:
            c_owners.append(
                {"path": path.relative_to(repo).as_posix(), "mentions": count}
            )

    rust_owners.sort(key=lambda owner: owner["path"])
    c_owners.sort(key=lambda owner: owner["path"])
    checks = {
        "safe_index_linked_splay_representation": (
            "nodes: Vec<Option<FloatTreeNode<V1, V2>>>" in implementation
            and "left: Option<usize>" in implementation
            and "right: Option<usize>" in implementation
            and "fn splay(&mut self" in implementation
        ),
        "standard_total_order_tree_and_root_marker_are_removed": (
            "BTreeMap" not in implementation
            and "FloatTreeKey" not in implementation
            and "root_key:" not in implementation
        ),
        "primary_find_and_extract_splay_on_misses": (
            "pub fn find(&mut self" in implementation
            and "pub fn extract_entry(&mut self" in implementation
            and implementation.count("let root = self.splay(self.root?, key);") >= 3
        ),
        "read_only_binary_query_is_explicit": (
            "pub fn find_binary(&self" in implementation
            and "self.find_index(key)" in implementation
        ),
        "nan_structural_stop_and_ieee_match_are_separate": (
            "fn c_splay_cmp" in implementation
            and "unwrap_or(Ordering::Equal)" in implementation
            and "fn c_keys_equal" in implementation
            and "matches!(left.partial_cmp(&right), Some(Ordering::Equal))" in implementation
        ),
        "rust_has_no_direct_production_owner": rust_owners == [],
        "c_has_no_direct_production_owner": c_owners == [],
    }
    return {
        "schema_version": 1,
        "direct_rust_owners": rust_owners,
        "c_owner_files": c_owners,
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
        print(f"FloatTree owner audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"FloatTree owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
