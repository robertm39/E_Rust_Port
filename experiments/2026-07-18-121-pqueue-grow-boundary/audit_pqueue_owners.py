#!/usr/bin/env python3
"""Audit PQueue growth reachability and C-to-Rust owner mappings."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


EXPECTED_C_OWNER_FILES = {
    "eprover/CLAUSES/ccl_derivation.c",
    "eprover/CLAUSES/ccl_sine.c",
    "eprover/CONTROL/cco_eserver.c",
    "eprover/INOUT/cio_multiplexer.c",
    "eprover/INOUT/cio_multiplexer.h",
    "eprover/TERMS/cte_ho_csu.c",
    "eprover/TERMS/cte_match_mgu_1-1.c",
    "eprover/TERMS/cte_pattern_match_mgu.c",
}
EXPECTED_RUST_PQUEUE_OWNERS = {
    "src/clauses/sine.rs",
    "src/terms/ho_csu.rs",
    "src/terms/match_mgu.rs",
    "src/terms/pattern_match_mgu.rs",
}
EXPECTED_RUST_VECDEQUE_MAPPINGS = {
    "src/clauses/proofstate.rs",
    "src/control/eserver.rs",
    "src/inout/multiplexer.rs",
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
    c_impl_path = repo / "eprover/BASICS/clb_pqueue.c"
    c_header_path = repo / "eprover/BASICS/clb_pqueue.h"
    rust_impl_path = repo / "src/basics/pqueue.rs"
    c_impl = c_impl_path.read_text(encoding="utf-8")
    c_header = c_header_path.read_text(encoding="utf-8")
    rust_impl = rust_impl_path.read_text(encoding="utf-8")

    c_owner_files = set()
    external_c_grow_calls = set()
    c_owner_pattern = re.compile(
        r"\bPQueue(?:Alloc|Free|Empty|Reset|StoreInt|StoreP|BuryInt|BuryP|"
        r"GetNext|GetLast|Look|LookLast|Cardinality|Element|TailIndex|IncIndex)\b"
    )
    for path in source_files(repo / "eprover", {".c", ".h"}):
        if path in {c_impl_path, c_header_path}:
            continue
        source = path.read_text(encoding="utf-8")
        if c_owner_pattern.search(source):
            c_owner_files.add(relative(repo, path))
        if re.search(r"\bPQueueGrow\s*\(", source):
            external_c_grow_calls.add(relative(repo, path))

    rust_pqueue_owners = set()
    external_rust_raw_grow_calls = set()
    for path in source_files(repo / "src", {".rs"}):
        if path == rust_impl_path:
            continue
        source = path.read_text(encoding="utf-8")
        if "crate::basics::pqueue" in source:
            rust_pqueue_owners.add(relative(repo, path))
        if re.search(r"\.grow_c_raw\s*\(", source):
            external_rust_raw_grow_calls.add(relative(repo, path))

    rust_vecdeque_mappings = set()
    for relative_path in EXPECTED_RUST_VECDEQUE_MAPPINGS:
        source = (repo / relative_path).read_text(encoding="utf-8")
        if "VecDeque" in source:
            rust_vecdeque_mappings.add(relative_path)

    production_rust = rust_impl.split("#[cfg(test)]", maxsplit=1)[0]
    checks = {
        "c_production_owner_inventory_matches": c_owner_files == EXPECTED_C_OWNER_FILES,
        "c_has_no_external_direct_grow_call": not external_c_grow_calls,
        "c_store_and_bury_are_the_only_grow_calls": c_header.count("PQueueGrow(queue);") == 2,
        "c_growth_copy_and_tail_shift_are_exact": all(
            marker in c_impl
            for marker in (
                "for(i=0; i<queue->head; i++)",
                "new_mem[i] = queue->queue[i];",
                "for(i=queue->head; i<queue->size; i++)",
                "new_mem[i+queue->size] = queue->queue[i];",
                "queue->tail+= queue->size;",
            )
        ),
        "rust_pqueue_owner_inventory_matches": rust_pqueue_owners
        == EXPECTED_RUST_PQUEUE_OWNERS,
        "rust_fifo_owner_mappings_use_vecdeque": rust_vecdeque_mappings
        == EXPECTED_RUST_VECDEQUE_MAPPINGS,
        "rust_has_no_external_direct_raw_grow_call": not external_rust_raw_grow_calls,
        "rust_store_and_bury_are_the_only_production_raw_grow_calls": (
            production_rust.count("self.grow_c_raw();") == 2
        ),
        "rust_raw_growth_is_public_but_memory_safe": (
            "pub fn grow_c_raw(&mut self)" in rust_impl
            and "Vec<Option<T>>" in rust_impl
            and "PQueueElement called on an uninitialized slot" in rust_impl
        ),
        "rust_pins_nonfull_holes_and_stale_slot_copy": (
            "direct_raw_nonfull_growth_copies_stale_slots_around_new_live_holes" in rust_impl
            and "direct_raw_grow_on_nonfull_queue_preserves_c_hazard_as_uninitialized_slot"
            in rust_impl
        ),
    }
    return {
        "schema_version": 1,
        "c_production_owner_files": sorted(c_owner_files),
        "c_external_direct_grow_calls": sorted(external_c_grow_calls),
        "rust_pqueue_owner_files": sorted(rust_pqueue_owners),
        "rust_vecdeque_owner_mappings": sorted(rust_vecdeque_mappings),
        "rust_external_direct_raw_grow_calls": sorted(external_rust_raw_grow_calls),
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
        print(f"PQueue owner mismatch: {args.output} != {args.expected}")
        return 1
    print(f"PQueue owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
