#!/usr/bin/env python3
"""Audit the production callers of the che_clausefeatures print helpers."""

from __future__ import annotations

import argparse
from pathlib import Path


def require_count(text: str, needle: str, count: int, source: Path) -> None:
    actual = text.count(needle)
    if actual != count:
        raise SystemExit(
            f"{source}: expected {count} occurrence(s) of {needle!r}, found {actual}"
        )


def require(text: str, needle: str, source: Path) -> None:
    if needle not in text:
        raise SystemExit(f"{source}: missing {needle!r}")


def read(root: Path, relative: str) -> tuple[Path, str]:
    path = root / relative
    return path, path.read_text(encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.repo.resolve()

    c_sets_path, c_sets = read(root, "eprover/HEURISTICS/che_clausesetfeatures.c")
    require_count(c_sets, "ClauseLinePrint(out, handle, printinfo);", 3, c_sets_path)
    require(c_sets, "void ProofStatePrintSelective(", c_sets_path)

    c_clause_path, c_clause = read(root, "eprover/HEURISTICS/che_clausefeatures.c")
    require_count(c_clause, "ClauseInfoPrint(out, clause);", 1, c_clause_path)

    c_prop_path, c_prop = read(root, "eprover/PCL2/pcl_propanalysis.c")
    require_count(
        c_prop,
        "ClausePropInfoPrint(out, data->max_standard_weight_clause->logic.clause);",
        1,
        c_prop_path,
    )

    c_eprover_path, c_eprover = read(root, "eprover/PROVER/eprover.c")
    require_count(
        c_eprover,
        "ProofStatePrintSelective(GlobalOut, proofstate, outdesc,",
        1,
        c_eprover_path,
    )
    c_epcl_path, c_epcl = read(root, "eprover/PROVER/epclanalyse.c")
    require_count(c_epcl, "PCLProtPropDataPrint(GlobalOut, &data);", 1, c_epcl_path)

    rust_sets_path, rust_sets = read(root, "src/heuristics/clausesetfeatures.rs")
    require(rust_sets, "pub fn proof_state_print_selective_string(", rust_sets_path)
    require_count(
        rust_sets,
        "clause_line_print_format_string_with_options(",
        1,
        rust_sets_path,
    )

    rust_eprover_path, rust_eprover = read(root, "src/prover/eprover.rs")
    require_count(
        rust_eprover,
        "let rendered = proof_state_print_selective_string(",
        1,
        rust_eprover_path,
    )
    require(rust_eprover, "section_output_format,", rust_eprover_path)
    require(rust_eprover, "section_problem_type,", rust_eprover_path)
    require(rust_eprover, "eqn_print_options,", rust_eprover_path)

    rust_prop_path, rust_prop = read(root, "src/pcl2/propanalysis.rs")
    require_count(
        rust_prop,
        "output.push_str(&clause_prop_info_print_string(protocol.term_bank(), clause));",
        1,
        rust_prop_path,
    )
    rust_epcl_path, rust_epcl = read(root, "src/prover/epclanalyse.rs")
    require_count(
        rust_epcl,
        "protocol_prop_data_print_string(&mut protocol, &data, ProblemType::FirstOrder)?",
        1,
        rust_epcl_path,
    )

    print("OK: clause-feature print callers are explicit and fully routed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
