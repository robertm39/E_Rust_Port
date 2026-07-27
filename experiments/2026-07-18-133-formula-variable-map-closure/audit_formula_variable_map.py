#!/usr/bin/env python3
"""Audit shared formula variable-name state and retain exact live cases."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


def ordered(text: str, *tokens: str) -> bool:
    position = 0
    for token in tokens:
        position = text.find(token, position)
        if position < 0:
            return False
        position += len(token)
    return True


def main_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": case["name"],
        "reference_status": case["reference_status"],
        "candidate_status": case["candidate_status"],
        "reference_exit_code": case["reference_exit_code"],
        "candidate_exit_code": case["candidate_exit_code"],
        "normalized_output_equal": case["normalized_output_equal"],
        "mismatches": case["mismatches"],
        "expected_mismatches": case["expected_mismatches"],
    }


def tool_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": case["name"],
        "reference_mode": case["reference_mode"],
        "reference_exit_code": case["reference_exit_code"],
        "candidate_exit_code": case["candidate_exit_code"],
        "normalized_stdout_equal": case["normalized_stdout_equal"],
        "normalized_stderr_equal": case["normalized_stderr_equal"],
        "mismatches": case["mismatches"],
        "expected_mismatches": case["expected_mismatches"],
    }


def summarize(
    repo: Path, main_report: dict[str, Any], tool_report: dict[str, Any]
) -> dict[str, Any]:
    clause = (repo / "src" / "clauses" / "clause.rs").read_text(encoding="utf-8")
    eprover = (repo / "src" / "prover" / "eprover.rs").read_text(encoding="utf-8")
    batch = (repo / "src" / "control" / "batch_spec.rs").read_text(encoding="utf-8")
    enormalizer = (repo / "src" / "prover" / "enormalizer.rs").read_text(
        encoding="utf-8"
    )
    all_rules = next(
        case for case in main_report["cases"] if case["name"] == "ALL_RULES.p"
    )
    enormalizer_case = next(
        case
        for case in tool_report["cases"]
        if case["name"] == "enormalizer/tstp-formula-target"
    )
    return {
        "schema_version": 1,
        "upstream_commit": main_report["reference_manifest"]["upstream_commit"],
        "manifests_equal": (
            main_report["reference_manifest"] == tool_report["reference_manifest"]
        ),
        "contracts": {
            "default_clause_scope_is_local_not_disjoint": ordered(
                clause,
                "impl Default for ClauseParseOptions",
                "clauses_have_local_variables: true,",
                "clauses_have_disjoint_variables: false,",
            ),
            "clause_scope_clears_external_names": ordered(
                clause,
                "fn apply_clause_parse_var_scope(",
                "if options.clauses_have_local_variables {",
                "bank.vars().clear_ext_names();",
                "if options.clauses_have_disjoint_variables {",
                "bank.vars().clear_ext_names_no_reset();",
            ),
            "main_app_encode_regression_present": (
                "fn run_app_encode_reuses_last_clause_variable_name_map_like_c()"
                in eprover
            ),
            "main_print_regression_present": (
                "fn run_print_formulas_reuses_last_clause_variable_name_map_like_c()"
                in eprover
            ),
            "main_all_rules_proof_regression_present": (
                "fn run_all_rules_proof_records_ac_resolution_ancestry()" in eprover
            ),
            "batch_regression_present": (
                "fn load_problem_from_file_reuses_formula_variable_name_map_like_c()"
                in batch
            ),
            "enormalizer_regression_present": (
                "fn tstp_formula_targets_reuse_external_name_map_like_c()"
                in enormalizer
            ),
            "c_binder_permutation_pinned_in_owner_tests": all(
                "X3, X4, X1, X2, X5" in source
                for source in (eprover, batch, enormalizer)
            ),
        },
        "main_case": main_case(all_rules),
        "tool_case": tool_case(enormalizer_case),
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("variable-map reports used the wrong archived C commit")
    if not summary["manifests_equal"]:
        raise SystemExit("main and enormalizer cases used different C manifests")
    failed = [name for name, passed in summary["contracts"].items() if not passed]
    if failed:
        raise SystemExit("variable-map contracts failed: " + ", ".join(failed))
    main = summary["main_case"]
    if main["reference_status"] != "Theorem" or main["candidate_status"] != "Theorem":
        raise SystemExit("ALL_RULES theorem outcome changed")
    if main["reference_exit_code"] != 0 or main["candidate_exit_code"] != 0:
        raise SystemExit("ALL_RULES did not exit successfully")
    if not main["normalized_output_equal"]:
        raise SystemExit("ALL_RULES normalized output differs")
    if main["mismatches"] or main["expected_mismatches"]:
        raise SystemExit("ALL_RULES must remain exact")
    tool = summary["tool_case"]
    if tool["reference_exit_code"] != 0 or tool["candidate_exit_code"] != 0:
        raise SystemExit("enormalizer formula-target case did not exit successfully")
    if not tool["normalized_stdout_equal"] or not tool["normalized_stderr_equal"]:
        raise SystemExit("enormalizer formula-target output differs")
    if tool["mismatches"] or tool["expected_mismatches"]:
        raise SystemExit("enormalizer formula-target case must remain exact")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--main-report", type=Path, required=True)
    parser.add_argument("--tool-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    main_report = json.loads(args.main_report.read_text(encoding="utf-8"))
    tool_report = json.loads(args.tool_report.read_text(encoding="utf-8"))
    summary = summarize(args.repo.resolve(), main_report, tool_report)
    validate(summary)

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if summary != expected:
            raise SystemExit("stable variable-map summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
