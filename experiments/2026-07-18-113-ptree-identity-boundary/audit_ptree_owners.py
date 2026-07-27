#!/usr/bin/env python3
"""Audit direct Rust PTree owners and their identity-key boundary."""

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


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def collect(repo: Path) -> dict[str, Any]:
    rust_sources = list((repo / "src").rglob("*.rs"))
    direct_owners = []
    for path in rust_sources:
        text = path.read_text(encoding="utf-8")
        count = len(re.findall(r"\bPTree\s*<", text))
        if count and path.name != "ptrees.rs":
            direct_owners.append(
                {"path": path.relative_to(repo).as_posix(), "type_mentions": count}
            )
    direct_owners.sort(key=lambda owner: owner["path"])

    termtypes = source(repo, "src/terms/termtypes.rs")
    clause = source(repo, "src/clauses/clause.rs")
    clausefunc = source(repo, "src/clauses/clausefunc.rs")
    ptrees = source(repo, "src/basics/ptrees.rs")
    checks = {
        "direct_owners_are_the_two_formula_collectors": direct_owners
        == [
            {"path": "src/clauses/clause.rs", "type_mentions": 1},
            {"path": "src/clauses/clausefunc.rs", "type_mentions": 1},
        ],
        "identity_is_live_rc_allocation_address": (
            "pub fn term_identity_id(term: &Term) -> usize" in termtypes
            and "Rc::as_ptr(&term.0).cast::<()>() as usize" in termtypes
        ),
        "clause_print_collector_uses_identity_and_c_stack_order": (
            "let identity = term_identity_id(term);" in clause
            and "order.store(identity);" in clause
            and "let mut variables = order" in clause
            and ".to_stack()" in clause
        ),
        "formula_collector_uses_identity_and_c_stack_order": (
            "let identity = term_identity_id(&term);" in clausefunc
            and "self.order.store(identity);" in clausefunc
            and "self.order" in clausefunc
            and ".to_stack()" in clausefunc
        ),
        "ptree_uses_safe_index_links": (
            "nodes: Vec<Option<PTreeNode<K>>>" in ptrees
            and "left: Option<usize>" in ptrees
            and "right: Option<usize>" in ptrees
        ),
        "c_shaped_find_and_binary_find_are_distinct": (
            "pub fn find(&mut self, key: &K)" in ptrees
            and "let root = self.splay(self.root?, key);" in ptrees
            and "pub fn find_binary(&self, key: &K)" in ptrees
            and "self.find_index(key)" in ptrees
        ),
    }
    return {
        "schema_version": 1,
        "direct_production_owners": direct_owners,
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
        print(f"owner audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"PTree owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
