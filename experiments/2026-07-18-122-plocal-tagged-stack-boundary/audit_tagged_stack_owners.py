#!/usr/bin/env python3
"""Audit tagged-local-stack build mode, owners, and safe Rust mapping."""

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


def source_files(root: Path, suffixes: set[str]) -> list[Path]:
    return sorted(path for path in root.rglob("*") if path.suffix in suffixes)


def collect(repo: Path) -> dict[str, Any]:
    c_header_path = repo / "eprover/BASICS/clb_plocalstacks.h"
    c_header = c_header_path.read_text(encoding="utf-8")
    makefile_vars = (repo / "eprover/Makefile.vars").read_text(encoding="utf-8")
    rust_stack_path = repo / "src/basics/plocalstacks.rs"
    rust_stack = rust_stack_path.read_text(encoding="utf-8")
    rust_kbo = (repo / "src/orderings/cto_kbolin.rs").read_text(encoding="utf-8")
    profile_path = repo / "experiments/2026-07-16-066-main-eprover-profile/FINDINGS.md"
    profile = profile_path.read_text(encoding="utf-8")

    c_owner_files = set()
    for path in source_files(repo / "eprover", {".c", ".h"}):
        if path == c_header_path:
            continue
        source = path.read_text(encoding="utf-8")
        if "PLocalTaggedStack" in source:
            c_owner_files.add(relative(repo, path))

    rust_external_compatibility_owners = set()
    for path in source_files(repo / "src", {".rs"}):
        if path == rust_stack_path:
            continue
        source = path.read_text(encoding="utf-8")
        if "PLocalTaggedStack" in source:
            rust_external_compatibility_owners.add(relative(repo, path))

    checks = {
        "ordinary_c_build_enables_tagged_pointers": "-DTAGGED_POINTERS" in makefile_vars,
        "c_has_both_one_and_two_slot_macro_variants": all(
            marker in c_header
            for marker in (
                "#ifdef TAGGED_POINTERS",
                "PLocalStackEnsureSpace(stack, space)",
                "PLocalStackEnsureSpace(stack, (2*(space)))",
                "((uintptr_t)val) | tag",
                "stack##_##data[stack##_##current++] = (void*)tag",
            )
        ),
        "c_tagged_stack_has_one_production_owner": c_owner_files
        == {"eprover/ORDERINGS/cto_kbolin.c"},
        "rust_generic_tagged_stack_has_no_production_owner": (
            not rust_external_compatibility_owners
        ),
        "rust_kbo_uses_typed_term_deref_frames": (
            rust_kbo.count("let mut stack = vec![(term.clone(), deref)];") == 4
            and "while let Some((candidate, mut current_deref)) = stack.pop()" in rust_kbo
            and "while let Some((candidate, current_deref)) = stack.pop()" in rust_kbo
        ),
        "rust_boundary_contains_no_unsafe_code": not re.search(
            r"\bunsafe\b", rust_stack + rust_kbo
        ),
        "rust_pins_two_slot_constants_frame_and_growth": all(
            marker in rust_stack
            for marker in (
                "PLOCALSTACK_TAG_BITS: usize = 2",
                "PLOCALSTACK_TAG_MASK",
                "tagged_stack_constants_and_frame_size_match_portable_c_slots",
                "tagged_wide_ensure_matches_portable_c_slot_growth",
                "assert_eq!(stack.allocated_slots(), 256)",
            )
        ),
        "existing_profile_measured_live_kbo_traversal": all(
            marker in profile
            for marker in (
                "1,306,910",
                "iterative `mfy_vwb` KBO6",
                "0.87%",
                "temporary argument vector",
            )
        ),
    }
    return {
        "schema_version": 1,
        "c_tagged_stack_owner_files": sorted(c_owner_files),
        "rust_external_compatibility_owner_files": sorted(
            rust_external_compatibility_owners
        ),
        "rust_live_owner_files": ["src/orderings/cto_kbolin.rs"],
        "profile_evidence": {
            "path": relative(repo, profile_path),
            "mfy_vwb_argument_helper_calls": 1_306_910,
            "accepted_instruction_reduction_percent": 0.87,
        },
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
        print(f"tagged-stack owner mismatch: {args.output} != {args.expected}")
        return 1
    print(f"tagged-stack owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
