#!/usr/bin/env python3
"""Audit the final BASICS Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [
    2,
    12,
    13,
    14,
    15,
    24,
    25,
    27,
    28,
    36,
    37,
    41,
    42,
    44,
    51,
    52,
    56,
    60,
    61,
    63,
    64,
    66,
    67,
    69,
    72,
    75,
    76,
    79,
    83,
    85,
    87,
    88,
    89,
    90,
    93,
    95,
    97,
    100,
    107,
]


def load_backlog_audit(repo: Path):
    path = (
        repo
        / "experiments/2026-07-25-029-post-compat-backlog-audit/audit_backlog.py"
    )
    spec = importlib.util.spec_from_file_location("post_compat_backlog_audit", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the post-compatibility audit module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    text = source(repo, relative)
    return all(needle in text for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    audit = load_backlog_audit(repo)
    issues = audit.load_children("E_Rust_Port-j76.4")
    records = [
        audit.issue_record("E_Rust_Port-j76.4", issue) for issue in issues
    ]
    audit.validate_parent("E_Rust_Port-j76.4", records)
    expected_ids = {f"E_Rust_Port-j76.4.{ordinal}" for ordinal in ORDINALS}
    selected = sorted(
        (record for record in records if record["id"] in expected_ids),
        key=lambda record: record["ordinal"],
    )
    issues_by_id = {issue["id"]: issue for issue in issues}
    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "id": record["id"],
            "legacy_text": record["legacy_text"],
            "ordinal": record["ordinal"],
            "source_file": record["source_file"],
        }
        for record in selected
    ]
    decision_digest = hashlib.sha256(
        json.dumps(
            stable_records, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()

    checks = {
        "assertion_shaped_array_and_stack_contracts_are_retained": contains(
            repo,
            "src/basics/ddarrays.rs",
            "fn element_ref_panics_on_negative_index_like_c_assertion()",
            "fn element_panics_on_negative_index_like_c_assertion()",
        )
        and contains(
            repo,
            "src/basics/dstacks.rs",
            "fn pop_panics_on_empty_like_c_assertion()",
            "fn element_panics_on_out_of_bounds_index_like_c_assertion()",
        )
        and contains(
            repo,
            "src/basics/fixdarrays.rs",
            "fn allocates_queryable_zeroed_storage_and_initializes()",
            "fn componentwise_arithmetic_panics_on_size_mismatch_like_c_assertion()",
        )
        and contains(
            repo,
            "src/basics/pstacks.rs",
            "fn top_panics_on_empty_like_c_assertion()",
            "fn discard_element_panics_on_position_at_stack_pointer_like_c_assertion()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "PDArray/DDArray nonnegative-index assertion behavior",
            "DStack assertion-shaped top/below-top/element/pop access",
        ),
        "dynamic_string_ownership_and_safe_indexing_are_explicit": contains(
            repo,
            "eprover/BASICS/clb_dstrings.h",
            "#define DStrAppendDStr(strdes, str)",
            "char*   DStrAddress(DStr_p strdes, int index);",
            "#define DStrGetRef(strdes)",
        )
        and contains(
            repo,
            "src/basics/dstrings.rs",
            "fn reference_helpers_preserve_c_counter_contract()",
            "fn releasing_after_final_reference_preserves_c_assertion()",
            "fn append_dstr_c_uses_source_c_string_view()",
            "fn address_exposes_allocated_c_nul_slot_at_len()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "final-release reporting",
            "allocated trailing-NUL address lookup",
        ),
        "resource_footer_elog_and_cpu_clock_surfaces_are_exact": contains(
            repo,
            "eprover/BASICS/clb_error.c",
            "void ELog(char* message, ...)",
            "getrusage(RUSAGE_CHILDREN, &cusage)",
            "usage.ru_maxrss",
        )
        and contains(
            repo,
            "src/basics/error.rs",
            "fn elog_helpers_preserve_c_record_and_stderr_newline_split()",
            "fn elog_in_dir_appends_to_pid_named_file()",
        )
        and contains(
            repo,
            "src/basics/os_wrapper.rs",
            "fn resource_usage_prints_c_shaped_footer()",
            "fn linux_getrusage_conversion_matches_c_print_rusage_shape()",
            "fn time_helpers_return_non_negative_c_shaped_units()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-107-resource-limit-ownership/FINDINGS.md",
            "one resource-footer case exact after host-value normalization",
        ),
        "allocator_and_registered_scratch_safety_boundaries_are_owned": contains(
            repo,
            "src/basics/memory.rs",
            "fn int_arrays_are_zero_initialized_and_stats_are_printable()",
            "MemoryBlock::zeroed",
            "MemoryPolicy",
        )
        and contains(
            repo,
            "src/basics/regmem.rs",
            "fn alloc_registers_zeroed_memory_and_free_unregisters_it()",
            "fn provide_doubles_capacity_and_zeroes_new_tail()",
            "fn free_panics_for_unknown_handle()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-123-memory-policy-boundary/FINDINGS.md",
            "Neither `MemoryBlock` nor",
            "true uninitialized bytes are excluded intentionally",
        )
        and contains(
            repo,
            "experiments/2026-07-18-111-regmem-typed-scratch/FINDINGS.md",
            "power-of-two growth, no-shrink, prefix, and zero-tail rules",
            "uninitialized allocation contents would require unsafe reads",
        ),
        "heap_compatibility_names_indices_and_empty_behavior_are_pinned": contains(
            repo,
            "src/basics/min_heap.rs",
            "fn pop_min_nonempty_panics_on_empty_like_c_sys_error()",
            "fn c_named_incr_and_decr_key_helpers_preserve_c_direction()",
            "fn signed_update_and_incr_negative_indices_match_c_noop()",
            "fn signed_remove_panics_on_negative_index_like_c_assertion()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "option-returning Rust drain pop plus C-shaped nonempty minimum pop",
            "signed negative-index wrappers",
        ),
        "numeric_and_object_splay_boundaries_match_unchanged_c": contains(
            repo,
            "src/basics/numtrees.rs",
            "fn operation_trace_matches_unchanged_c_splay_topology()",
            "fn traversal_and_limited_traversal_are_ascending()",
        )
        and contains(
            repo,
            "src/basics/numxtrees.rs",
            "fn operation_trace_matches_unchanged_c_splay_topology()",
            "fn traversal_and_limited_traversal_are_ascending()",
        )
        and contains(
            repo,
            "src/basics/objtrees.rs",
            "fn splayed_find_tracks_hits_and_nearest_misses_like_c()",
            "fn operation_trace_matches_unchanged_c_splay_topology()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-117-numtree-splay-topology/FINDINGS.md",
            "limited traversal initializes the path to the first key greater",
            "five direct generic Rust owner modules",
        )
        and contains(
            repo,
            "experiments/2026-07-18-116-numxtree-splay-topology/FINDINGS.md",
            "limited traversal initializes the path to the first key greater",
            "no direct Rust production owner",
        )
        and contains(
            repo,
            "experiments/2026-07-18-115-obj-splay-topology/FINDINGS.md",
            "nearest-node root after a miss",
            "same 378-byte stdout",
        ),
        "partial_ordering_and_property_masks_keep_the_c_tables": contains(
            repo,
            "eprover/BASICS/clb_properties.h",
            "#define IsAnyPropSet(obj, prop) ((obj)->properties & (prop))",
            "#define GiveProps(obj,prop) ((obj)->properties & (prop))",
        )
        and contains(
            repo,
            "src/basics/partial_orderings.rs",
            "fn inverse_c_panics_on_unknown_like_c_assertion()",
            "pub const PO_COMPARE_SYMBOLS",
        )
        and contains(
            repo,
            "src/basics/properties.rs",
            "pub const fn any_set(self, prop: Self) -> Self",
            "pub const fn give(self, prop: Self) -> Self",
            "fn give_and_equivalence_mask_properties()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-120-compare-symbol-boundary/FINDINGS.md",
            "values zero through four render identically",
            "is the exact safe encoding",
        ),
        "pointer_and_range_arrays_quarantine_raw_mutation_hazards": contains(
            repo,
            "eprover/BASICS/clb_pdarrays.h",
            "#define   PDArrayElementClear(arr, idx)",
        )
        and contains(
            repo,
            "src/basics/pdarrays.rs",
            "fn raw_element_clear_clears_existing_slot_without_growing()",
            "fn raw_element_clear_panics_on_uncovered_index_instead_of_c_out_of_bounds_write()",
            "fn element_ref_panics_on_negative_index_like_c_assertion()",
        )
        and contains(
            repo,
            "src/basics/pdrangearrays.rs",
            "fn exponential_growth_expands_up_and_down_with_c_offset_rule()",
            "fn delete_only_clears_covered_indices()",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "mutating element access that extends the range like the C macros",
        ),
        "permanent_strings_and_verbose_globals_have_safe_compatibility_owners": contains(
            repo,
            "eprover/BASICS/clb_verbose.h",
            "#define VERBOSE(arg) {if(Verbose){arg}}",
            "#define VERBOSE2(arg) {if(Verbose>=2){arg}}",
            "#define VERBOSE10(arg) {if(Verbose>=10){arg}}",
        )
        and contains(
            repo,
            "src/basics/permastrings.rs",
            "fn registry_reuses_existing_allocation_for_equal_strings()",
            "fn clear_drops_registry_references_without_invalidating_returned_arcs()",
        )
        and contains(
            repo,
            "src/basics/verbose.rs",
            "fn message_formatting_matches_c_macros()",
            "pub fn set_verbose_level(level: i32) -> i32",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "with no pointer-identity comparisons",
            "global `ProgName`/stderr-backed wrappers",
        ),
        "list_local_stack_queue_and_print_helpers_keep_narrow_raw_surfaces": contains(
            repo,
            "src/basics/plist.rs",
            "fn alloc_list_creates_empty_self_linked_anchor()",
            "fn delete_panics_on_invalid_handle_like_c_pointer_contract()",
        )
        and contains(
            repo,
            "src/basics/plocalstacks.rs",
            "fn push_does_not_grow_but_ensure_space_uses_c_equality_rule()",
            "fn tagged_wide_ensure_matches_portable_c_slot_growth()",
        )
        and contains(
            repo,
            "src/basics/pqueue.rs",
            "fn get_next_panics_on_empty_like_c_assertion()",
            "fn direct_raw_grow_on_nonfull_queue_preserves_c_hazard_as_uninitialized_slot()",
        )
        and contains(
            repo,
            "src/basics/pstacks.rs",
            "fn integer_average_and_deviation_match_population_formula()",
            "fn raw_pointer_printing_keeps_c_percent_p_shape_without_dereferencing()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-122-plocal-tagged-stack-boundary/FINDINGS.md",
            "`cto_kbolin.c` is",
            "only C tagged-stack owner",
        )
        and contains(
            repo,
            "experiments/2026-07-18-121-pqueue-grow-boundary/FINDINGS.md",
            "No C production owner calls exported `PQueueGrow` directly",
            "creates `None` holes",
        ),
        "pointer_and_quad_tree_topology_and_identity_are_explicit": contains(
            repo,
            "src/basics/ptrees.rs",
            "fn splayed_find_tracks_hits_and_nearest_miss_like_c()",
            "fn extract_root_delete_and_stack_conversion_follow_splay_shape()",
            "fn visit_in_order_and_debug_print_preserve_distinct_orders()",
        )
        and contains(
            repo,
            "src/basics/quadtrees.rs",
            "fn splayed_misses_move_nearest_boundary_key_to_root()",
            "fn operation_trace_matches_unchanged_c_splay_topology()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-113-ptree-identity-boundary/FINDINGS.md",
            "root-right-left stack traversal",
            "matches C's native `uintptr_t` pointer comparison policy",
        )
        and contains(
            repo,
            "experiments/2026-07-18-112-quadtree-splay-topology/FINDINGS.md",
            "top-down rotations, assembly, miss splaying",
            "exactly equal stdout, stderr, and exit",
        ),
        "performance_counter_single_slot_contract_is_current": contains(
            repo,
            "src/basics/perf_counters.rs",
            "fn counter_start_cell(counter: PerfCounter) -> &'static AtomicI64",
            "fn statistics_match_c_counter_names_and_order()",
        )
        and contains(
            repo,
            "experiments/2026-07-18-114-os-wrapper-perf-boundary/FINDINGS.md",
            "exact C names and order",
            "one overwriteable start slot per counter",
        ),
        "full_basics_and_port_compatibility_evidence_is_current": contains(
            repo,
            "docs/rust-port-status.md",
            "Dynamic `PDArray` and `DDArray` storage",
            "`PQueue` circular pointer/integer queue behavior",
            "Registered persistent-memory helpers",
            "Verbose-level helpers",
        )
        and contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
            '"rust_test_count": 4429',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }

    c_modules = [
        "clb_ddarrays",
        "clb_dstacks",
        "clb_dstrings",
        "clb_error",
        "clb_fixdarrays",
        "clb_memory",
        "clb_min_heap",
        "clb_numtrees",
        "clb_numxtrees",
        "clb_objtrees",
        "clb_os_wrapper",
        "clb_partial_orderings",
        "clb_pdarrays",
        "clb_pdrangearrays",
        "clb_permastrings",
        "clb_plist",
        "clb_plocalstacks",
        "clb_pqueue",
        "clb_pstacks",
        "clb_ptrees",
        "clb_quadtrees",
        "clb_regmem",
        "clb_verbose",
    ]
    source_files = [
        relative
        for module in c_modules
        for relative in (
            f"eprover/BASICS/{module}.c",
            f"eprover/BASICS/{module}.h",
        )
    ]
    source_files.append("eprover/BASICS/clb_properties.h")
    source_files.extend(
        [
            "src/basics/ddarrays.rs",
            "src/basics/dstacks.rs",
            "src/basics/dstrings.rs",
            "src/basics/error.rs",
            "src/basics/fixdarrays.rs",
            "src/basics/memory.rs",
            "src/basics/min_heap.rs",
            "src/basics/numtrees.rs",
            "src/basics/numxtrees.rs",
            "src/basics/objtrees.rs",
            "src/basics/os_wrapper.rs",
            "src/basics/partial_orderings.rs",
            "src/basics/pdarrays.rs",
            "src/basics/pdrangearrays.rs",
            "src/basics/perf_counters.rs",
            "src/basics/permastrings.rs",
            "src/basics/plist.rs",
            "src/basics/plocalstacks.rs",
            "src/basics/pqueue.rs",
            "src/basics/properties.rs",
            "src/basics/pstacks.rs",
            "src/basics/ptrees.rs",
            "src/basics/quadtrees.rs",
            "src/basics/regmem.rs",
            "src/basics/verbose.rs",
            "docs/rust-port-status.md",
            "experiments/2026-07-18-107-resource-limit-ownership/FINDINGS.md",
            "experiments/2026-07-18-111-regmem-typed-scratch/FINDINGS.md",
            "experiments/2026-07-18-112-quadtree-splay-topology/FINDINGS.md",
            "experiments/2026-07-18-113-ptree-identity-boundary/FINDINGS.md",
            "experiments/2026-07-18-114-os-wrapper-perf-boundary/FINDINGS.md",
            "experiments/2026-07-18-115-obj-splay-topology/FINDINGS.md",
            "experiments/2026-07-18-116-numxtree-splay-topology/FINDINGS.md",
            "experiments/2026-07-18-117-numtree-splay-topology/FINDINGS.md",
            "experiments/2026-07-18-120-compare-symbol-boundary/FINDINGS.md",
            "experiments/2026-07-18-121-pqueue-grow-boundary/FINDINGS.md",
            "experiments/2026-07-18-122-plocal-tagged-stack-boundary/FINDINGS.md",
            "experiments/2026-07-18-123-memory-policy-boundary/FINDINGS.md",
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
        ]
    )
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_basics = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "basics"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 39
        or report["content_hashes_verified"] != 39
        or not selected_are_basics
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("BASICS reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
