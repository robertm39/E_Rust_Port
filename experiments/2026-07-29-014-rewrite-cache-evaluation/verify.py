#!/usr/bin/env python3
"""Independently verify representative shared rewrite-cache proofs."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
ANALYZE_PATH = EXPERIMENT_ROOT / "analyze.py"
PRIOR_VERIFY_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-008-stronger-redundancy"
    / "verify.py"
)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
PHASE_BUDGETS = {"casc": "larger", "targeted": "targeted"}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


ANALYZE = load_module("rewrite_cache_verify_analyze", ANALYZE_PATH)
PRIOR = load_module("rewrite_cache_prior_verify", PRIOR_VERIFY_PATH)


class VerificationError(RuntimeError):
    """A proof-selection or merged-report contract failure."""


def representative_claims(
    contract: dict[str, Any],
    results: Sequence[dict[str, Any]],
    budget: str,
) -> tuple[
    list[tuple[str, str, dict[str, Any]]],
    list[dict[str, str]],
]:
    """Select one proof per reproducibly solved category and build."""

    claims: list[tuple[str, str, dict[str, Any]]] = []
    boundaries: list[dict[str, str]] = []
    for strategy in contract["strategies"]:
        coverage = ANALYZE.BASE.reproducible_coverage(
            results, strategy, budget, contract["repetitions"]
        )
        categories = sorted(
            {
                result["category"]
                for result in results
                if result["strategy"] == strategy
                and result["budget"] == budget
                and result["problem_id"] in coverage
            }
        )
        for category in categories:
            candidates = sorted(
                (
                    result
                    for result in results
                    if result["strategy"] == strategy
                    and result["budget"] == budget
                    and result["repetition"] == 1
                    and result["problem_id"] in coverage
                    and result["category"] == category
                    and result["szs_status"] in PROOF_STATUSES
                ),
                key=lambda result: result["problem_id"],
            )
            if not candidates:
                boundaries.append(
                    {
                        "strategy": strategy,
                        "category": category,
                        "reason": "no_reproducible_proof_in_frozen_sample",
                    }
                )
                continue
            representative = candidates[0]
            claims.append(
                (strategy, representative["problem_id"], representative)
            )
    return claims, boundaries


def verify_phase(
    *,
    phase: str,
    repo: Path,
    experiment_root: Path,
    problem_root: Path,
    output_root: Path,
    proofcheck: Path,
) -> tuple[dict[str, Any], list[dict[str, str]]]:
    contract, results = ANALYZE.BASE.load_phase(experiment_root, phase)
    claims, boundaries = representative_claims(
        contract, results, PHASE_BUDGETS[phase]
    )

    def load_phase(
        _experiment_root: Path, requested_phase: str
    ) -> tuple[dict[str, Any], list[dict[str, Any]]]:
        if requested_phase != "test":
            raise VerificationError(
                f"proof verifier expected test alias, got {requested_phase}"
            )
        return contract, results

    def proof_claims(
        _contract: dict[str, Any],
        _results: Sequence[dict[str, Any]],
    ) -> list[tuple[str, str, dict[str, Any]]]:
        return claims

    original_load_phase = PRIOR.ANALYZE.load_phase
    original_proof_claims = PRIOR.proof_claims
    try:
        PRIOR.ANALYZE.load_phase = load_phase
        PRIOR.proof_claims = proof_claims
        report = PRIOR.verify_claims(
            repo=repo,
            experiment_root=experiment_root,
            problem_root=problem_root,
            output_root=output_root,
            proofcheck=proofcheck,
        )
    finally:
        PRIOR.ANALYZE.load_phase = original_load_phase
        PRIOR.proof_claims = original_proof_claims
    for case in report["cases"]:
        case["phase"] = phase
        case["budget"] = PHASE_BUDGETS[phase]
    for boundary in boundaries:
        boundary["phase"] = phase
        boundary["budget"] = PHASE_BUDGETS[phase]
    return report, boundaries


def verify(
    *,
    repo: Path,
    experiment_root: Path,
    casc_problem_root: Path,
    targeted_problem_root: Path,
    output_root: Path,
    explicit_proofcheck: Path | None,
) -> dict[str, Any]:
    contracts = {
        phase: ANALYZE.BASE.load_phase(experiment_root, phase)[0]
        for phase in PHASE_BUDGETS
    }
    output_root.mkdir(parents=True, exist_ok=True)
    proofcheck = PRIOR.find_or_download_proofcheck(
        output_root, explicit_proofcheck
    )
    phase_reports = {}
    boundaries = []
    for phase, problem_root in (
        ("casc", casc_problem_root),
        ("targeted", targeted_problem_root),
    ):
        report, phase_boundaries = verify_phase(
            phase=phase,
            repo=repo,
            experiment_root=experiment_root,
            problem_root=problem_root,
            output_root=output_root / phase,
            proofcheck=proofcheck,
        )
        phase_reports[phase] = report
        boundaries.extend(phase_boundaries)

    cases = [
        case
        for phase in PHASE_BUDGETS
        for case in phase_reports[phase]["cases"]
    ]
    expected_cases = sum(
        report["expected_cases"] for report in phase_reports.values()
    )
    verified_cases = sum(
        report["verified_cases"] for report in phase_reports.values()
    )
    proofcheck_metadata = phase_reports["casc"]["proofcheck"]
    if any(
        report["proofcheck"] != proofcheck_metadata
        for report in phase_reports.values()
    ):
        raise VerificationError("phase ProofCheck metadata differs")
    body = {
        "schema_version": 1,
        "contracts": {
            phase: contract["contract_id"]
            for phase, contract in contracts.items()
        },
        "binaries": {
            "cache": contracts["casc"]["binary_sha256"],
            "recompute": contracts["casc"]["strategies"]["recompute"][
                "binary_sha256"
            ],
        },
        "proofcheck": proofcheck_metadata,
        "checker": phase_reports["casc"]["checker"],
        "ueq_adapter": phase_reports["casc"]["ueq_adapter"],
        "skolem_metadata_adapter": phase_reports["casc"][
            "skolem_metadata_adapter"
        ],
        "expected_cases": expected_cases,
        "verified_cases": verified_cases,
        "all_verified": verified_cases == expected_cases,
        "focused_ancestry_boundaries": boundaries,
        "cases": cases,
    }
    return {
        **body,
        "report_id": hashlib.sha256(
            ANALYZE.BASE.canonical_json(body)
        ).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--experiment-root", type=Path, required=True)
    parser.add_argument("--casc-problem-root", type=Path, required=True)
    parser.add_argument("--targeted-problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise VerificationError("independent proof checking requires Linux")
    output_root = arguments.output_root.resolve()
    report = verify(
        repo=arguments.repo.resolve(),
        experiment_root=arguments.experiment_root.resolve(),
        casc_problem_root=arguments.casc_problem_root.resolve(),
        targeted_problem_root=arguments.targeted_problem_root.resolve(),
        output_root=output_root,
        explicit_proofcheck=(
            arguments.proofcheck.resolve()
            if arguments.proofcheck is not None
            else None
        ),
    )
    report_path = output_root / "proof-validation.json"
    report_path.write_bytes(
        ANALYZE.BASE.canonical_json(report) + b"\n"
    )
    print(
        f"OK: {report['verified_cases']}/{report['expected_cases']} "
        f"representative proof claims verified; "
        f"report {report['report_id']}"
    )
    return 0 if report["all_verified"] else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        VerificationError,
        PRIOR.VerificationError,
        ANALYZE.AnalysisError,
        OSError,
        ValueError,
        json.JSONDecodeError,
        RuntimeError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
