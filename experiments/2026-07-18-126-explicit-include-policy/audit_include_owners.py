#!/usr/bin/env python3
"""Audit full formula-owner include parsing against unchanged C ownership."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


C_CALL_SITES = {
    "PROVER/eprover.c": 1,
    "PROVER/classify_problem.c": 1,
    "PROVER/eground.c": 1,
    "PROVER/epatternize.c": 1,
    "PROVER/enormalizer.c": 1,
    "CONTROL/cco_sine.c": 1,
    "CONTROL/cco_einteractive_mode.c": 2,
    "CONTROL/cco_batch_spec.c": 2,
}

RUST_GENERAL_CONSUMERS = {
    "src/prover/eprover.rs": (
        "parse_input_files_into_axioms(",
        "parse_input_files_into_formula_owners(",
        "parse_clause_scanner_into_destination_with_options(",
    ),
    "src/prover/classify_problem.rs": (
        "fn parse_real_input_scanner(",
        "parse_clause_scanner_into_formula_set_with_options(",
    ),
    "src/prover/eground.rs": (
        "fn parse_input_files_to_formula_set_with_progress(",
        "parse_clause_scanner_into_formula_set_with_options(",
    ),
    "src/prover/epatternize.rs": (
        "fn parse_input_file(",
        "parse_clause_scanner_into_formula_set_with_options(",
    ),
    "src/prover/enormalizer.rs": (
        "fn parse_rule_file(",
        "parse_clause_scanner_into_formula_set_with_options(",
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def read(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def collect(repo: Path) -> dict[str, Any]:
    c_records = []
    for relative, expected_calls in C_CALL_SITES.items():
        source = read(repo / "eprover", relative)
        actual_calls = source.count("FormulaAndClauseSetParse(")
        c_records.append(
            {
                "file": f"eprover/{relative}",
                "expected_calls": expected_calls,
                "actual_calls": actual_calls,
                "matches": actual_calls == expected_calls,
            }
        )

    rust_records = []
    for relative, markers in RUST_GENERAL_CONSUMERS.items():
        source = read(repo, relative)
        rust_records.append(
            {
                "file": relative,
                "policy_owner": "general_formula_owner_parser",
                "markers": list(markers),
                "matches": all(marker in source for marker in markers),
            }
        )

    scanner = read(repo, "src/inout/scanner.rs")
    eprover = read(repo, "src/prover/eprover.rs")
    batch = read(repo, "src/control/batch_spec.rs")
    interactive = read(repo, "src/control/einteractive_mode.rs")
    c_parser = read(repo / "eprover", "CLAUSES/ccl_formulafunc.c")
    rust_sources = {
        path.relative_to(repo).as_posix(): path.read_text(encoding="utf-8")
        for path in (repo / "src").rglob("*.rs")
    }
    automatic_refs = [
        relative
        for relative, source in rust_sources.items()
        for _ in range(source.count("from_file_following_includes("))
    ]
    scanner_test_start = scanner.index("#[cfg(test)]\nmod tests")
    automatic_call = scanner.rindex("from_file_following_includes(")

    checks = {
        "unchanged_c_has_exactly_ten_formula_owner_calls": (
            sum(record["actual_calls"] for record in c_records) == 10
            and all(record["matches"] for record in c_records)
        ),
        "c_formula_owner_parser_recurses_through_scanner_parse_include": all(
            marker in c_parser
            for marker in (
                "new_in = ScannerParseInclude(in, &new_limit, skip_includes);",
                "res += FormulaAndClauseSetParse(new_in,",
                "if (app_encode)",
                "ignore_include(in);",
            )
        ),
        "rust_general_consumers_share_explicit_parser": all(
            record["matches"] for record in rust_records
        ),
        "rust_general_parser_has_explicit_tptp_and_tstp_include_branches": (
            eprover.count("scanner.parse_include(&mut include_selectors, &skip_includes)?")
            == 2
            and "parse_tptp_entry_list(" in eprover
            and "parse_tstp_entry_list(" in eprover
        ),
        "rust_batch_and_interactive_paths_share_explicit_policy": all(
            marker in batch
            for marker in (
                "scanner.parse_include(&mut include_selectors, &skip_includes)?",
                "include_selector_stack.push(include_selectors);",
                "include_entry_selected_by_stack(",
                "fn parsed_include_skip_tree(",
            )
        )
        and all(
            marker in interactive
            for marker in (
                "fn parse_interactive_axioms(",
                "spec.load_problem_from_scanner(bank, ctrl, &mut scanner)",
            )
        ),
        "selector_filtering_is_one_shared_inner_to_outer_helper": all(
            marker in scanner
            for marker in (
                "pub(crate) fn include_entry_selected(",
                "pub(crate) fn include_entry_selected_by_stack(",
                "include_selector_stack.iter_mut().rev()",
            )
        )
        and "include_entry_selected_by_stack(" in eprover
        and "include_entry_selected_by_stack(" in batch,
        "automatic_splicing_constructor_is_scanner_test_only": (
            automatic_refs == ["src/inout/scanner.rs", "src/inout/scanner.rs"]
            and automatic_call > scanner_test_start
        ),
        "app_encode_deliberately_ignores_includes_like_c": all(
            marker in eprover
            for marker in (
                "fn parse_app_encode_ignored_include(",
                "include_echoes.push_str(&parse_app_encode_ignored_include(scanner)?)",
            )
        ),
        "batch_regressions_cover_nested_skip_and_repeat_policy": all(
            marker in batch
            for marker in (
                "batch_explicit_include_policy_applies_nested_selectors_inner_to_outer",
                "batch_explicit_include_policy_skips_registered_but_preserves_repeats",
            )
        ),
    }
    return {
        "schema_version": 1,
        "c_formula_owner_call_count": sum(
            record["actual_calls"] for record in c_records
        ),
        "c_call_sites": c_records,
        "rust_general_consumers": rust_records,
        "rust_explicit_policy_owners": [
            "src/prover/eprover.rs: general TPTP/TSTP formula-owner parser",
            "src/control/batch_spec.rs: batch and interactive TSTP formula-owner parser",
        ],
        "automatic_splicing_references": automatic_refs,
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
        print(f"include owner audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"explicit include owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
