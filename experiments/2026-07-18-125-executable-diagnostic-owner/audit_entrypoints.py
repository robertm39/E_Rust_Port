#!/usr/bin/env python3
"""Audit C/Rust executable diagnostic-name initialization and fatal routing."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any


ENTRYPOINTS = [
    ("eprover", "eprover.rs", "PROVER/eprover.c", "canonical"),
    ("CSSCPA_filter", "csscpa_filter.rs", "EXTERNAL/CSSCPA_filter.c", "canonical"),
    ("e_stratpar", "e_stratpar.rs", "PROVER/e_stratpar.c", "canonical"),
    ("e_ltb_runner", "e_ltb_runner.rs", "PROVER/e_ltb_runner.c", "canonical"),
    ("termprops", "termprops.rs", "PROVER/termprops.c", "invocation"),
    ("term2dag", "term2dag.rs", "SIMPLE_APPS/term2dag.c", "invocation"),
    ("ex_commandline", "ex_commandline.rs", "SIMPLE_APPS/ex_commandline.c", "invocation"),
    ("epclextract", "epclextract.rs", "PROVER/epclextract.c", "canonical"),
    ("epclanalyse", "epclanalyse.rs", "PROVER/epclanalyse.c", "canonical"),
    ("checkproof", "checkproof.rs", "PROVER/checkproof.c", "canonical"),
    ("epcllemma", "epcllemma.rs", "PROVER/epcllemma.c", "canonical"),
    ("edpll", "edpll.rs", "PROVER/edpll.c", "canonical"),
    ("eground", "eground.rs", "PROVER/eground.c", "canonical"),
    ("classify_problem", "classify_problem.rs", "PROVER/classify_problem.c", "canonical"),
    ("tsm_classify", "tsm_classify.rs", "PROVER/tsm_classify.c", "canonical"),
    ("direct_examples", "direct_examples.rs", "PROVER/direct_examples.c", "canonical"),
    ("e_client", "e_client.rs", "PROVER/e_client.c", "canonical"),
    (
        "e_deduction_server",
        "e_deduction_server.rs",
        "PROVER/e_deduction_server.c",
        "canonical",
    ),
    ("e_server", "e_server.rs", "PROVER/e_server.c", "canonical"),
    ("e_axfilter", "e_axfilter.rs", "PROVER/e_axfilter.c", "canonical"),
    ("enormalizer", "enormalizer.rs", "PROVER/enormalizer.c", "canonical"),
    ("epatternize", "epatternize.rs", "PROVER/epatternize.c", "canonical"),
    ("ekb_create", "ekb_create.rs", "PROVER/ekb_create.c", "canonical"),
    ("ekb_delete", "ekb_delete.rs", "PROVER/ekb_delete.c", "canonical"),
    ("ekb_insert", "ekb_insert.rs", "PROVER/ekb_insert.c", "canonical"),
    ("ekb_ginsert", "ekb_ginsert.rs", "PROVER/ekb_ginsert.c", "invocation"),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected", type=Path)
    return parser.parse_args()


def cargo_bins(cargo_toml: str) -> dict[str, str]:
    return dict(
        re.findall(
            r'\[\[bin\]\]\s+name = "([^"]+)"\s+path = "([^"]+)"',
            cargo_toml,
        )
    )


def collect(repo: Path) -> dict[str, Any]:
    expected_cargo = {
        cargo_name: f"src/bin/{rust_file}"
        for cargo_name, rust_file, _, _ in ENTRYPOINTS
    }
    actual_cargo = cargo_bins((repo / "Cargo.toml").read_text(encoding="utf-8"))
    records = []
    for cargo_name, rust_file, c_file, mode in ENTRYPOINTS:
        rust = (repo / "src/bin" / rust_file).read_text(encoding="utf-8")
        c_source = (repo / "eprover" / c_file).read_text(encoding="utf-8")
        if mode == "canonical":
            c_initialization_matches = "InitIO(NAME)" in c_source
        else:
            c_initialization_matches = bool(
                re.search(r"Init(?:IO|Error)\(argv\[0\]\)", c_source)
            )
        rust_marker = (
            "init_error(PROGRAM_NAME);"
            if mode == "canonical"
            else "init_error_from_invocation(PROGRAM_NAME);"
        )
        records.append(
            {
                "cargo_name": cargo_name,
                "rust_file": f"src/bin/{rust_file}",
                "c_file": f"eprover/{c_file}",
                "name_mode": mode,
                "c_initialization_matches": c_initialization_matches,
                "rust_initialization_matches": rust.count(rust_marker) == 1,
                "rust_uses_shared_fatal_reporter": (
                    rust.count("report_fatal_diagnostic") == 2
                    and "writeln!(stderr" not in rust
                    and "{PROGRAM_NAME}:" not in rust
                ),
            }
        )

    error_source = (repo / "src/basics/error.rs").read_text(encoding="utf-8")
    integration = (repo / "tests/executable_diagnostics.rs").read_text(encoding="utf-8")
    invocation_entries = sorted(
        record["cargo_name"]
        for record in records
        if record["name_mode"] == "invocation"
    )
    checks = {
        "cargo_declares_exactly_26_audited_bins": actual_cargo == expected_cargo,
        "all_c_initializers_match_expected_mode": all(
            record["c_initialization_matches"] for record in records
        ),
        "all_rust_initializers_match_c_mode": all(
            record["rust_initialization_matches"] for record in records
        ),
        "all_rust_bins_use_shared_fatal_reporter": all(
            record["rust_uses_shared_fatal_reporter"] for record in records
        ),
        "invocation_owned_entries_are_exactly_four": invocation_entries
        == ["ekb_ginsert", "ex_commandline", "term2dag", "termprops"],
        "global_error_owner_is_owned_and_poison_tolerant": all(
            marker in error_source
            for marker in (
                "static PROGRAM_NAME: OnceLock<Mutex<String>>",
                "unwrap_or_else(std::sync::PoisonError::into_inner)",
                "pub fn init_error_from_invocation(",
                "pub fn report_fatal_diagnostic(",
                "let program_name = program_name();",
            )
        ),
        "fatal_runtime_tests_cover_both_name_modes": all(
            marker in integration
            for marker in (
                "canonical_name_entrypoint_reports_through_global_fatal_owner",
                "argv0_entrypoint_reports_exact_invoked_name_through_global_fatal_owner",
                "CARGO_BIN_EXE_eprover",
                "CARGO_BIN_EXE_termprops",
            )
        ),
    }
    return {
        "schema_version": 1,
        "entrypoint_count": len(records),
        "canonical_name_entrypoint_count": sum(
            record["name_mode"] == "canonical" for record in records
        ),
        "invocation_name_entrypoint_count": sum(
            record["name_mode"] == "invocation" for record in records
        ),
        "records": records,
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
        print(f"entrypoint audit mismatch: {args.output} != {args.expected}")
        return 1
    print(f"executable diagnostic owner audit: accepted={result['accepted']}")
    return 0 if result["accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
