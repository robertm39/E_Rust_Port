#!/usr/bin/env python3
"""Audit the safe MemoryBlock compatibility boundary and policy source."""

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
    memory_path = repo / "src/basics/memory.rs"
    newmem_path = repo / "src/basics/newmem.rs"
    memory = memory_path.read_text(encoding="utf-8")
    newmem = newmem_path.read_text(encoding="utf-8")
    c_memory = (repo / "eprover/BASICS/clb_memory.c").read_text(encoding="utf-8")
    c_memory_h = (repo / "eprover/BASICS/clb_memory.h").read_text(encoding="utf-8")
    c_newmem = (repo / "eprover/BASICS/clb_newmem.c").read_text(encoding="utf-8")
    makefile_vars = (repo / "eprover/Makefile.vars").read_text(encoding="utf-8")

    external_memory_block_owners = set()
    external_policy_owners = set()
    for path in sorted((repo / "src").rglob("*.rs")):
        if path in {memory_path, newmem_path}:
            continue
        source = path.read_text(encoding="utf-8")
        if re.search(r"\bMemoryBlock\b", source):
            external_memory_block_owners.add(relative(repo, path))
        if re.search(r"\bMemoryPolicy\b", source):
            external_policy_owners.add(relative(repo, path))

    checks = {
        "ordinary_c_build_uses_old_allocator": not re.search(
            r"(?m)^\s*-DUSE_NEWMEM\b", makefile_vars
        ),
        "c_old_policy_is_exact_size_and_flushes": all(
            marker in c_memory_h
            for marker in (
                "size>=MEM_ARR_MIN_INDEX && size<MEM_ARR_SIZE",
                "free_mem_list[size]",
            )
        )
        and all(
            marker in c_memory
            for marker in (
                "void MemFlushFreeList(void)",
                "FREE(handle);",
            )
        ),
        "c_new_policy_uses_byte_threshold_and_dummy_flush": all(
            marker in c_newmem
            for marker in (
                "mem_index = (size+MEM_ALIGN-1)/MEM_ALIGN;",
                "if(size < MEM_CHUNKLIMIT)",
                "MemAddNewChunk(mem_index);",
                "this is a dummy in the new memory handler",
            )
        ),
        "rust_has_no_external_memory_block_owner": not external_memory_block_owners,
        "rust_has_no_external_policy_owner": not external_policy_owners,
        "rust_uses_owned_initialized_byte_buffers": (
            "pub struct MemoryBlock" in memory
            and "bytes: Vec<u8>" in memory
            and "bytes.resize(allocation_size, 0)" in memory
        ),
        "rust_models_exact_and_aligned_policies": all(
            marker in memory
            for marker in (
                "OldExact",
                "NewAligned",
                "effective_size < MEM_CHUNKLIMIT",
                "state.add_newmem_chunk(mem_index)?",
            )
        ),
        "rust_newmem_flush_is_no_op": "pub fn mem_flush_free_list() -> (usize, usize) {\n    (0, 0)\n}"
        in newmem,
        "rust_pins_255_256_threshold": (
            "new_policy_chunk_threshold_uses_effective_bytes_not_bucket_index" in memory
        ),
        "rust_boundary_contains_no_unsafe_code": not re.search(
            r"\bunsafe\b", memory + newmem
        ),
    }
    return {
        "schema_version": 1,
        "external_memory_block_owner_files": sorted(external_memory_block_owners),
        "external_memory_policy_owner_files": sorted(external_policy_owners),
        "compatibility_owner_files": [
            "src/basics/memory.rs",
            "src/basics/newmem.rs",
        ],
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
        print(f"memory-owner mismatch: {args.output} != {args.expected}")
        return 1
    print(f"memory owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
