#!/usr/bin/env python3
"""Audit that che_specsigfeatures exports no direct formula-set collector."""

from __future__ import annotations

import argparse
from pathlib import Path


EXPORTED_COLLECTORS = (
    "TermCollectSigFeatures",
    "ClauseCollectSigFeatures",
    "ClauseComputeSigFeatures",
    "ClauseSetCollectSigFeatures",
)


def require(source: str, needle: str, path: Path) -> None:
    if needle not in source:
        raise AssertionError(f"{path}: missing {needle}")


def reject(source: str, needle: str, path: Path) -> None:
    if needle in source:
        raise AssertionError(f"{path}: unexpected {needle}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    repo = args.repo.resolve()

    header_path = repo / "eprover" / "HEURISTICS" / "che_specsigfeatures.h"
    source_path = repo / "eprover" / "HEURISTICS" / "che_specsigfeatures.c"
    classifier_path = repo / "eprover" / "PROVER" / "classify_problem.c"
    rust_path = repo / "src" / "heuristics" / "specsigfeatures.rs"

    header = header_path.read_text(encoding="utf-8")
    source = source_path.read_text(encoding="utf-8")
    classifier = classifier_path.read_text(encoding="utf-8")
    rust = rust_path.read_text(encoding="utf-8")

    for collector in EXPORTED_COLLECTORS:
        require(header, collector, header_path)
    for forbidden in ("FormulaCollectSigFeatures", "FormulaSetCollectSigFeatures"):
        reject(header, forbidden, header_path)
        reject(source, forbidden, source_path)
    reject(rust, "FormulaSet", rust_path)
    reject(rust, "WrappedFormula", rust_path)

    cnf_position = classifier.index("FormulaSetCNF2(")
    collector_position = classifier.index("ClauseSetCollectSigFeatures(")
    if cnf_position >= collector_position:
        raise AssertionError("classify_problem must clausify before specsig collection")
    require(
        classifier,
        "ClauseSetCollectSigFeatures(fstate->signature, fstate->axioms,",
        classifier_path,
    )

    print("OK: specsig remains a term/clause/clause-set surface after formula CNF")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
