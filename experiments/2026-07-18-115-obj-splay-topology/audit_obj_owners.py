#!/usr/bin/env python3
"""Audit Rust ObjTree/ObjMap representations and production owners."""

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


def direct_type_owners(repo: Path, type_name: str) -> list[dict[str, Any]]:
    owners = []
    implementation = {"ObjTree": "objtrees.rs", "ObjMap": "objmaps.rs"}[type_name]
    for path in (repo / "src").rglob("*.rs"):
        if path.name == implementation:
            continue
        text = path.read_text(encoding="utf-8")
        count = len(re.findall(rf"\b{type_name}\s*<", text))
        if count:
            owners.append(
                {"path": path.relative_to(repo).as_posix(), "type_mentions": count}
            )
    return sorted(owners, key=lambda owner: owner["path"])


def collect(repo: Path) -> dict[str, Any]:
    objtrees = source(repo, "src/basics/objtrees.rs")
    objmaps = source(repo, "src/basics/objmaps.rs")
    fp_index = source(repo, "src/terms/fp_index.rs")
    overlap_index = source(repo, "src/clauses/overlap_index.rs")
    subterm_index = source(repo, "src/clauses/subterm_index.rs")
    objtree_owners = direct_type_owners(repo, "ObjTree")
    objmap_owners = direct_type_owners(repo, "ObjMap")
    checks = {
        "objtree_uses_safe_index_links": (
            "nodes: Vec<Option<ObjTreeNode<T>>>" in objtrees
            and "left: Option<usize>" in objtrees
            and "right: Option<usize>" in objtrees
        ),
        "objmap_uses_safe_index_links": (
            "nodes: Vec<Option<ObjMapNode<K, V>>>" in objmaps
            and "left: Option<usize>" in objmaps
            and "right: Option<usize>" in objmaps
        ),
        "marker_and_standard_tree_models_are_removed": not any(
            token in objtrees + objmaps
            for token in ("BTreeSet", "BTreeMap", "root_object:", "root_key:", "Rc<")
        ),
        "objtree_mutating_and_binary_lookup_are_distinct": (
            "pub fn find(&mut self, key: &T)" in objtrees
            and "let root = self.splay(self.root?, key);" in objtrees
            and "pub fn find_binary(&self, key: &T)" in objtrees
            and "self.find_index(key)" in objtrees
        ),
        "objmap_find_and_extract_splay_on_misses": (
            "pub fn find(&mut self, key: &K)" in objmaps
            and "pub fn extract_slot(&mut self, key: &K)" in objmaps
            and objmaps.count("let root = self.splay(self.root?, key);") >= 2
        ),
        "owned_payloads_do_not_require_clone": (
            "T: Ord + Clone" not in objtrees
            and "K: Ord + Clone" not in objmaps
            and "Rc::" not in objtrees
        ),
        "fp_leaf_is_the_direct_objtree_storage_owner": (
            "payload: Option<ObjTree<T>>" in fp_index
            and "self.ensure_payload().store(object)" in fp_index
            and objtree_owners
            == [
                {"path": "src/clauses/overlap_index.rs", "type_mentions": 2},
                {"path": "src/terms/fp_index.rs", "type_mentions": 18},
            ]
        ),
        "immutable_owner_queries_use_binary_lookup": (
            "payload.find_binary(&SubtermOcc::new(term))" in overlap_index
            and "payload.find_binary(&SubtermOcc::new(term))" in subterm_index
        ),
        "objmap_has_no_direct_rust_production_owner": objmap_owners == [],
        "c_stack_merge_and_postorder_free_are_explicit": (
            "add.into_c_stack_order()" in objtrees
            and "self.into_post_order()" in objtrees
            and "self.into_post_order()" in objmaps
        ),
    }
    return {
        "schema_version": 1,
        "direct_objtree_type_owners": objtree_owners,
        "direct_objmap_type_owners": objmap_owners,
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
        print(f"object owner audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"object owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
