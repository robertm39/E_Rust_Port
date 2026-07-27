#!/usr/bin/env python3
"""Audit the StrTree representation and direct Rust owner set."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


EXPECTED_RUST_OWNERS = {
    "src/control/batch_spec.rs",
    "src/inout/scanner.rs",
    "src/learn/examplerep.rs",
    "src/prover/eprover.rs",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def collect(repo: Path) -> dict[str, Any]:
    implementation_path = repo / "src/basics/stringtrees.rs"
    implementation = implementation_path.read_text(encoding="utf-8")
    rust_owners = []
    for path in (repo / "src").rglob("*.rs"):
        if path == implementation_path:
            continue
        source = path.read_text(encoding="utf-8")
        count = len(re.findall(r"\bStrTree\s*(?:<|::new|::<)", source))
        if count:
            rust_owners.append(
                {"path": path.relative_to(repo).as_posix(), "type_mentions": count}
            )

    c_owners = []
    for path in (repo / "eprover").rglob("*"):
        if path.suffix not in {".c", ".h"}:
            continue
        if path.name in {"clb_stringtrees.c", "clb_stringtrees.h"}:
            continue
        source = path.read_text(encoding="utf-8")
        count = len(
            re.findall(
                r"\bStrTree(?:_p|Find|Store|Insert|Extract|Delete|Traverse|Free)\b",
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
            "nodes: Vec<Option<StrTreeNode<V1, V2>>>" in implementation
            and "left: Option<usize>" in implementation
            and "right: Option<usize>" in implementation
            and "fn splay(&mut self" in implementation
        ),
        "standard_tree_and_root_marker_are_removed": (
            "BTreeMap" not in implementation and "root_key:" not in implementation
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
        "owned_c_string_prefix_and_byte_order_are_explicit": (
            "fn c_string_prefix" in implementation
            and "fn c_string_cmp" in implementation
            and ".as_bytes()" in implementation
            and "key.truncate(nul)" in implementation
        ),
        "direct_rust_owner_inventory_matches": (
            {owner["path"] for owner in rust_owners} == EXPECTED_RUST_OWNERS
        ),
        "c_owner_inventory_is_nonempty": len(c_owners) >= 20,
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
        print(f"StrTree owner audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"StrTree owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
