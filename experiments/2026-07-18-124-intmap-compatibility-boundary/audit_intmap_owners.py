#!/usr/bin/env python3
"""Audit IntMap compatibility behavior and its production Rust owners."""

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


def relative(repo: Path, path: Path) -> str:
    return path.relative_to(repo).as_posix()


def collect(repo: Path) -> dict[str, Any]:
    intmap_path = repo / "src/basics/intmap.rs"
    pdrange_path = repo / "src/basics/pdrangearrays.rs"
    intmap = intmap_path.read_text(encoding="utf-8")
    pdrange = pdrange_path.read_text(encoding="utf-8")
    c_header = (repo / "eprover/BASICS/clb_intmap.h").read_text(encoding="utf-8")
    c_source = (repo / "eprover/BASICS/clb_intmap.c").read_text(encoding="utf-8")

    owner_files: set[str] = set()
    get_val_files: set[str] = set()
    mutating_iterator_files: set[str] = set()
    get_ref_files: set[str] = set()
    delete_files: set[str] = set()
    constant_storage_files: set[str] = set()
    for path in sorted((repo / "src").rglob("*.rs")):
        if path == intmap_path:
            continue
        source = path.read_text(encoding="utf-8")
        rel = relative(repo, path)
        if re.search(r"\bIntMap(?:<|::|\b)", source):
            owner_files.add(rel)
        if ".get_val(" in source:
            get_val_files.add(rel)
        if ".iter_range_c_mut(" in source:
            mutating_iterator_files.add(rel)
        if ".get_ref(" in source and "intmap" in source.lower():
            get_ref_files.add(rel)
        if ".del_key(" in source:
            delete_files.add(rel)
        if "constant_mem_storage_estimate" in source and "IntMap" in source:
            constant_storage_files.add(rel)

    expected_owners = {
        "src/clauses/fcvindexing.rs",
        "src/clauses/pdtrees.rs",
    }
    fcv = (repo / "src/clauses/fcvindexing.rs").read_text(encoding="utf-8")
    pdt = (repo / "src/clauses/pdtrees.rs").read_text(encoding="utf-8")
    checks = {
        "c_single_transition_has_argument_order_asymmetry": (
            "switch_to_array(key, map->min_key, map->max_key, 2)" in c_source
        ),
        "c_array_lookup_and_delete_use_growing_element_access": (
            c_source.count("PDRangeArrElementP(map->values.array, key)") >= 2
        ),
        "c_array_iterator_starts_at_raw_lower_and_grows": all(
            marker in c_source + c_header
            for marker in (
                "handle->admin_data.current = lower_key;",
                "res = PDRangeArrElementP(iter->map->values.array, i);",
            )
        ),
        "c_null_array_slots_inflate_entry_count": all(
            marker in c_source
            for marker in (
                "res = &(PDRangeArrElementP(map->values.array, key));",
                "if(!(*res))",
                "map->entry_no++;",
            )
        ),
        "c_constant_storage_shapes_are_present": all(
            marker in c_header
            for marker in (
                "#define INTMAPCELL_MEM 20",
                "PDArrayStorage((map)->values.array)",
                "((map)->entry_no*NUMTREECELL_MEM)",
            )
        ),
        "rust_preserves_all_four_representations": all(
            marker in intmap for marker in ("Empty", "Single", "Array", "Tree")
        ),
        "rust_preserves_single_argument_order": (
            "switch_to_array(key, self.min_key, self.max_key, 2)" in intmap
        ),
        "rust_preserves_mutating_miss_boundary": all(
            marker in intmap
            for marker in (
                "array.element(key).as_ref()",
                "let result = array.element(key).clone();",
                "pub fn iter_range_c_mut(",
            )
        ),
        "rust_exposes_side_effect_free_iteration": (
            "array.existing_element(key)" in intmap
        ),
        "rust_preserves_null_count_inflation": all(
            marker in intmap
            for marker in (
                "array.element_ref(key).is_none()",
                "self.entry_no += 1;",
            )
        ),
        "rust_pins_same_keys_different_representation": (
            "sparse_second_key_preserves_c_single_insertion_order_asymmetry" in intmap
        ),
        "production_owners_are_exactly_fv_and_pdt": owner_files == expected_owners,
        "production_has_no_mutating_lookup_owner": not get_val_files,
        "production_has_no_mutating_iterator_owner": not mutating_iterator_files,
        "constant_storage_owners_are_exactly_fv_and_pdt": (
            constant_storage_files == expected_owners
        ),
        "fv_owner_uses_map_only_for_storage_compatibility": all(
            marker in fcv
            for marker in (
                "successor_storage: Option<IntMap<()>>",
                "IntMap::constant_mem_storage_estimate",
                ".assign(key, ());",
            )
        ),
        "pdt_owner_guards_created_slot_and_deletes_known_child": all(
            marker in pdt
            for marker in (
                "select_alt_ref_for_insert",
                ".constant_mem_storage_estimate()",
                ".get_ref(fun_code_key(code));",
                "if slot.is_none()",
                ".del_key(fun_code_key(code));",
            )
        ),
        "compatibility_boundary_contains_no_unsafe_code": not re.search(
            r"\bunsafe\b", intmap + pdrange
        ),
    }
    return {
        "schema_version": 1,
        "production_owner_files": sorted(owner_files),
        "mutating_lookup_owner_files": sorted(get_val_files),
        "mutating_iterator_owner_files": sorted(mutating_iterator_files),
        "get_ref_owner_files": sorted(get_ref_files),
        "delete_owner_files": sorted(delete_files),
        "constant_storage_owner_files": sorted(constant_storage_files),
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
        print(f"IntMap owner mismatch: {args.output} != {args.expected}")
        return 1
    print(f"IntMap owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
