#!/usr/bin/env python3
"""Audit optional fingerprint-index statistics ownership contracts."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    repo = args.repo.resolve()
    cargo = (repo / "Cargo.toml").read_text(encoding="utf-8")
    prover = (repo / "src/prover/eprover.rs").read_text(encoding="utf-8")
    global_indices = (repo / "src/clauses/global_indices.rs").read_text(encoding="utf-8")
    fp_index = (repo / "src/terms/fp_index.rs").read_text(encoding="utf-8")
    subterm_tree = (repo / "src/clauses/subterm_tree.rs").read_text(encoding="utf-8")
    c_prover = (repo / "eprover/PROVER/eprover.c").read_text(encoding="utf-8")
    c_fp_index = (repo / "eprover/TERMS/cte_fp_index.c").read_text(encoding="utf-8")
    c_subterm_tree = (repo / "eprover/CLAUSES/ccl_subterm_tree.c").read_text(
        encoding="utf-8"
    )

    contracts = {
        "c_optional_block_feature_gate": (
            "#ifdef PRINT_INDEX_STATS" in c_prover
            and "FPIndexPrintDot(GlobalOut, \"pm_from_index\"" in c_prover
        ),
        "c_distribution_handles_null_index": (
            "void FPIndexDistribDataPrint(FILE* out, FPIndex_p index)" in c_fp_index
            and "if(index)" in c_fp_index
        ),
        "c_dot_prints_nodes_edges_then_payloads": (
            c_fp_index.index("fp_index_tree_print_nodes(out, index->index, stack, sig);")
            < c_fp_index.index("fp_index_tree_print_edges(out, index->index, stack, sig);")
            < c_fp_index.index("prt_leaf(out, leaf->payload, sig);")
        ),
        "c_flattened_payload_is_default": (
            "#ifdef PRT_SUBTERM_SET_AS_TREE" in c_subterm_tree
            and "PTreeToPStack(terms, root);" in c_subterm_tree
        ),
        "cargo_exposes_optional_feature": ("print-index-stats = []" in cargo),
        "prover_feature_dispatches_final_statistics": (
            '#[cfg(feature = "print-index-stats")]' in prover
            and "indices.write_index_statistics_io(output, state.terms())?;" in prover
        ),
        "global_writer_preserves_four_line_order": (
            global_indices.index("Backwards rewriting index :")
            < global_indices.index("Paramod-from index        :")
            < global_indices.index("Paramod-into index        :")
            < global_indices.index("Paramod-neg-atom index    :")
        ),
        "global_writer_prints_only_pm_from_graph": (
            global_indices.count('"pm_from_index"') == 1
            and "index.dot_string(" in global_indices
        ),
        "global_writer_handles_disabled_indexes": (
            "fn write_null_fp_index_distrib_data(" in global_indices
        ),
        "fp_tree_generic_payload_renderer": (
            "pub fn write_print_with<W, F>" in fp_index
            and "FnMut(&[FunCode], &Self, &mut W)" in fp_index
        ),
        "fp_tree_distribution_shape": (
            "pub fn collect_distrib(&self) -> FPIndexDistrib" in fp_index
            and '"{:5} nodes, {:5} leaves, {:6.2}+/-{:4.3} terms/leaf"'
            in fp_index
        ),
        "fp_tree_dot_nodes_edges_payloads": (
            fp_index.index("self.write_dot_nodes(sig, &mut path, &mut output);")
            < fp_index.index("self.write_dot_edges(sig, &mut output);")
            < fp_index.index("self.collect_leaves(&mut leaves);")
        ),
        "flattened_subterm_payload_renderer": (
            "pub fn write_subterm_occurrences_dot_record" in subterm_tree
            and 'write!(output, "     t{dot_id} [label=\\"{{|{{")?' in subterm_tree
        ),
        "distribution_regression": (
            "fn distribution_prints_payload_paths_and_c_summary_shape()" in fp_index
        ),
        "dot_graph_regression": (
            "fn dot_prints_c_pointer_ids_and_only_structural_leaf_payload_edges()"
            in fp_index
        ),
        "flattened_payload_regression": (
            "fn dot_record_output_matches_c_flattened_payload_shape()" in subterm_tree
        ),
        "global_optional_block_regression": (
            "fn index_statistics_string_prints_c_optional_index_stats_block()"
            in global_indices
        ),
        "disabled_index_regression": (
            "fn index_statistics_string_prints_c_null_distribution_for_disabled_indexes()"
            in global_indices
        ),
    }
    report = {
        "schema_version": 1,
        "contracts": contracts,
        "passed": sum(contracts.values()),
        "total": len(contracts),
        "all_passed": all(contracts.values()),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("audit does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_passed"]:
        failed = [name for name, passed in contracts.items() if not passed]
        print(f"fingerprint-index statistics contracts failed: {failed}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
