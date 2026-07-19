#!/usr/bin/env python3
"""Audit pre-CNF formula ownership and retain exact executable cases."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


EXPECTED_UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
EXPECTED_CASES = ("ALL_RULES.p", "ans_test06.p", "socrates.p")


def ordered(text: str, *tokens: str) -> bool:
    position = 0
    for token in tokens:
        position = text.find(token, position)
        if position < 0:
            return False
        position += len(token)
    return True


def exact_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "name": case["name"],
        "mode": case["mode"],
        "reference_status": case["reference_status"],
        "candidate_status": case["candidate_status"],
        "reference_exit_code": case["reference_exit_code"],
        "candidate_exit_code": case["candidate_exit_code"],
        "normalized_output_equal": case["normalized_output_equal"],
        "mismatches": case["mismatches"],
        "expected_mismatches": case["expected_mismatches"],
    }


def summarize(repo: Path, report: dict[str, Any]) -> dict[str, Any]:
    eprover = (repo / "src" / "prover" / "eprover.rs").read_text(encoding="utf-8")
    formulasets = (repo / "src" / "clauses" / "formulasets.rs").read_text(
        encoding="utf-8"
    )
    schedule_tests = (repo / "tests" / "eprover_schedule.rs").read_text(
        encoding="utf-8"
    )
    by_name = {case["name"]: case for case in report["cases"]}
    return {
        "schema_version": 1,
        "upstream_commit": report["reference_manifest"]["upstream_commit"],
        "candidate_kind": report["candidate_kind"],
        "contracts": {
            "main_parser_keeps_parsed_formulas_in_f_axioms": ordered(
                eprover,
                "fn parse_input_files_into_axioms(",
                "let mut parsed_formulas = FormulaSet::new();",
                "parse_clause_formula_file_with_verbose_output(",
                "state.f_axioms_mut().insert_set(&mut parsed_formulas);",
            ),
            "auto_classification_precedes_formula_clausification": ordered(
                eprover,
                "fn run_proof_search<",
                "apply_auto_mode_preprocessing_selection(",
                "clausify_formula_axioms_with_docs(",
            ),
            "clause_wrapper_marks_clause_and_moves_info": ordered(
                formulasets,
                "pub fn form_clause_alloc(",
                "wrapped.is_clause = true;",
                "wrapped.set_info(clause.take_info());",
            ),
            "clause_wrapper_cnf_pushes_fof_quote": ordered(
                formulasets,
                "pub fn cnf2_into(",
                "if self.is_clause {",
                "clause_push_formula_derivation(&mut clause, DC_FOF_QUOTE, Some(source), None);",
            ),
            "formula_archive_copy_pushes_fof_quote": ordered(
                formulasets,
                "pub fn archive_into(&mut self, archive: &mut Self)",
                "newform.push_formula_derivation(DC_FOF_QUOTE, Some(source), None);",
            ),
            "auto_owner_classification_regression_present": (
                "fn auto_mode_classifies_cnf_inputs_as_pre_cnf_formula_owners()"
                in schedule_tests
            ),
            "formula_owner_and_proof_regressions_present": all(
                token in eprover
                for token in (
                    "fn parse_input_files_into_formula_owners_routes_print_input_through_f_axioms()",
                    "fn run_answer_proof_object_preserves_formula_copy_ancestry()",
                    "fn run_all_rules_proof_records_ac_resolution_ancestry()",
                )
            ),
        },
        "exact_cases": [exact_case(by_name[name]) for name in EXPECTED_CASES],
    }


def validate(summary: dict[str, Any]) -> None:
    if summary["upstream_commit"] != EXPECTED_UPSTREAM_COMMIT:
        raise SystemExit("pre-CNF owner report used the wrong archived C commit")
    if summary["candidate_kind"] != "windows-rust":
        raise SystemExit("pre-CNF owner report did not use native Windows Rust")
    failed = [name for name, passed in summary["contracts"].items() if not passed]
    if failed:
        raise SystemExit("pre-CNF owner contracts failed: " + ", ".join(failed))
    if [case["name"] for case in summary["exact_cases"]] != list(EXPECTED_CASES):
        raise SystemExit("pre-CNF executable case inventory changed")
    for case in summary["exact_cases"]:
        if case["mode"] != "fol":
            raise SystemExit(f"{case['name']} did not use the FOL reference")
        if case["reference_status"] != "Theorem":
            raise SystemExit(f"archived C {case['name']} outcome changed")
        if case["candidate_status"] != case["reference_status"]:
            raise SystemExit(f"Rust {case['name']} outcome differs from C")
        if case["reference_exit_code"] != 0 or case["candidate_exit_code"] != 0:
            raise SystemExit(f"{case['name']} did not exit successfully")
        if not case["normalized_output_equal"]:
            raise SystemExit(f"{case['name']} normalized output differs")
        if case["mismatches"] or case["expected_mismatches"]:
            raise SystemExit(f"{case['name']} must remain exact")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()

    report = json.loads(args.report.read_text(encoding="utf-8"))
    summary = summarize(args.repo.resolve(), report)
    validate(summary)

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if summary != expected:
            raise SystemExit("stable pre-CNF owner summary differs from retained evidence")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
