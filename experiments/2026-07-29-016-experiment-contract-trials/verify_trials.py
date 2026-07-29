#!/usr/bin/env python3
"""Recompute the two version-1 contract trials from raw archives."""

from __future__ import annotations

import argparse
import importlib.util
import json
import statistics
import sys
import tarfile
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
VALIDATOR_PATH = REPO_ROOT / "tools/experiment_contract/validate.py"
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
MODEL_STATUSES = {"CounterSatisfiable", "Satisfiable"}


class TrialError(RuntimeError):
    """Raised when preserved evidence disagrees with a trial record."""


@dataclass(frozen=True)
class TrialSpec:
    record_name: str
    archive_path: str
    summary_path: str
    phase: str
    budget: str
    baseline: str
    candidate: str
    source_decision: str
    expected_outcome: str


TRIALS = (
    TrialSpec(
        record_name="rewrite-cache-result.json",
        archive_path=(
            ".artifacts/experiments/"
            "2026-07-29-014-rewrite-cache-evaluation/"
            "rewrite-cache-results-final.tar.gz"
        ),
        summary_path=(
            "experiments/2026-07-29-014-rewrite-cache-evaluation/"
            "results-summary.json"
        ),
        phase="casc",
        budget="larger",
        baseline="recompute",
        candidate="cache",
        source_decision="retain_full_shared_rewrite_cache",
        expected_outcome="continue",
    ),
    TrialSpec(
        record_name="bce-toggle-result.json",
        archive_path=(
            ".artifacts/experiments/"
            "2026-07-29-015-preprocessing-evaluation/"
            "preprocessing-results.tar.gz"
        ),
        summary_path=(
            "experiments/2026-07-29-015-preprocessing-evaluation/"
            "results-summary.json"
        ),
        phase="casc",
        budget="heldout",
        baseline="baseline",
        candidate="bce",
        source_decision="retain_explicit_default_off",
        expected_outcome="stop",
    ),
)


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


VALIDATE = load_module("experiment_contract_validator", VALIDATOR_PATH)


def rounded(value: float) -> float:
    return round(value, 6)


def relative_range(values: Sequence[float]) -> float:
    if not values:
        raise TrialError("cannot compute variation from no values")
    center = statistics.median(values)
    if center == 0:
        if min(values) == max(values):
            return 0.0
        raise TrialError("relative range is undefined around zero")
    return (max(values) - min(values)) / center


def summarized_ranges(
    values_by_coordinate: dict[str, Sequence[float]],
) -> tuple[float, float]:
    if not values_by_coordinate:
        raise TrialError("cannot summarize variation without coordinates")
    ranges = [
        relative_range(values)
        for _coordinate, values in sorted(values_by_coordinate.items())
    ]
    return rounded(statistics.median(ranges)), rounded(max(ranges))


def _read_json_member(
    archive: tarfile.TarFile,
    member: tarfile.TarInfo,
) -> dict[str, Any]:
    stream = archive.extractfile(member)
    if stream is None:
        raise TrialError(f"cannot read archive member: {member.name}")
    value = json.load(stream)
    if not isinstance(value, dict):
        raise TrialError(f"archive JSON is not an object: {member.name}")
    return value


def load_archive_results(path: Path) -> list[dict[str, Any]]:
    if not path.is_file():
        raise TrialError(f"raw evidence archive is missing: {path}")
    results: list[dict[str, Any]] = []
    with tarfile.open(path, "r:gz") as archive:
        members = {
            member.name: member
            for member in archive.getmembers()
            if member.isfile()
        }
        result_names = sorted(
            name for name in members if name.endswith("/result.json")
        )
        for result_name in result_names:
            result = _read_json_member(archive, members[result_name])
            telemetry_name = (
                result_name[: -len("result.json")] + "telemetry.json"
            )
            telemetry = (
                _read_json_member(archive, members[telemetry_name])
                if telemetry_name in members
                else None
            )
            if bool(result.get("telemetry_present")) != (
                telemetry is not None
            ):
                raise TrialError(
                    f"telemetry presence mismatch: {result_name}"
                )
            result["_telemetry"] = telemetry
            results.append(result)
    if not results:
        raise TrialError(f"archive contains no result records: {path}")
    return results


def status_polarity(status: str | None) -> str | None:
    if status in PROOF_STATUSES:
        return "proof"
    if status in MODEL_STATUSES:
        return "model"
    return None


def index_results(
    results: Iterable[dict[str, Any]],
) -> dict[tuple[str, str, str, int, str], dict[str, Any]]:
    indexed: dict[
        tuple[str, str, str, int, str],
        dict[str, Any],
    ] = {}
    for result in results:
        key = (
            result["phase"],
            result["budget"],
            result["problem_id"],
            result["repetition"],
            result["strategy"],
        )
        if key in indexed:
            raise TrialError(f"duplicate result coordinate: {key}")
        indexed[key] = result
    return indexed


def audit_status_pairs(
    results: Sequence[dict[str, Any]],
    baseline: str,
    candidate: str,
) -> dict[str, int]:
    indexed = index_results(results)
    paired = 0
    exact = 0
    disagreements = 0
    for key, candidate_result in sorted(indexed.items()):
        phase, budget, problem_id, repetition, strategy = key
        if strategy != candidate:
            continue
        baseline_key = (
            phase,
            budget,
            problem_id,
            repetition,
            baseline,
        )
        if baseline_key not in indexed:
            continue
        paired += 1
        baseline_result = indexed[baseline_key]
        candidate_status = candidate_result.get("szs_status")
        baseline_status = baseline_result.get("szs_status")
        if candidate_status == baseline_status:
            exact += 1
        candidate_polarity = status_polarity(candidate_status)
        baseline_polarity = status_polarity(baseline_status)
        if (
            candidate_polarity is not None
            and baseline_polarity is not None
            and candidate_polarity != baseline_polarity
        ):
            disagreements += 1
    return {
        "paired_coordinates": paired,
        "exact_matches": exact,
        "polarity_disagreements": disagreements,
    }


def _proof_solves_in_every_repetition(
    indexed: dict[
        tuple[str, str, str, int, str],
        dict[str, Any],
    ],
    *,
    phase: str,
    budget: str,
    strategy: str,
    problem_id: str,
    repetitions: Sequence[int],
) -> bool:
    return all(
        indexed.get(
            (phase, budget, problem_id, repetition, strategy),
            {},
        ).get("szs_status")
        in PROOF_STATUSES
        for repetition in repetitions
    )


def common_solve_evidence(
    results: Sequence[dict[str, Any]],
    spec: TrialSpec,
) -> dict[str, Any]:
    indexed = index_results(results)
    selected = [
        result
        for result in results
        if result["phase"] == spec.phase
        and result["budget"] == spec.budget
        and result["strategy"] in {spec.baseline, spec.candidate}
    ]
    repetitions = sorted(
        {int(result["repetition"]) for result in selected}
    )
    problem_ids = sorted(
        {str(result["problem_id"]) for result in selected}
    )
    baseline_solved = {
        problem_id
        for problem_id in problem_ids
        if _proof_solves_in_every_repetition(
            indexed,
            phase=spec.phase,
            budget=spec.budget,
            strategy=spec.baseline,
            problem_id=problem_id,
            repetitions=repetitions,
        )
    }
    candidate_solved = {
        problem_id
        for problem_id in problem_ids
        if _proof_solves_in_every_repetition(
            indexed,
            phase=spec.phase,
            budget=spec.budget,
            strategy=spec.candidate,
            problem_id=problem_id,
            repetitions=repetitions,
        )
    }
    common = sorted(baseline_solved & candidate_solved)
    values: dict[str, dict[str, list[float]]] = {
        spec.baseline: {},
        spec.candidate: {},
    }
    ratios: dict[str, list[float]] = {}
    for problem_id in common:
        for strategy in (spec.baseline, spec.candidate):
            samples: list[float] = []
            for repetition in repetitions:
                result = indexed[
                    (
                        spec.phase,
                        spec.budget,
                        problem_id,
                        repetition,
                        strategy,
                    )
                ]
                telemetry = result.get("_telemetry")
                if not isinstance(telemetry, dict):
                    raise TrialError(
                        "common solve lacks telemetry: "
                        f"{problem_id}/{strategy}/rep-{repetition}"
                    )
                samples.append(
                    float(
                        telemetry["resources"]["total_cpu_seconds"]
                    )
                )
            values[strategy][problem_id] = samples
        ratios[problem_id] = [
            candidate_value / baseline_value
            for candidate_value, baseline_value in zip(
                values[spec.candidate][problem_id],
                values[spec.baseline][problem_id],
                strict=True,
            )
        ]
    all_ratios = [
        ratio
        for problem_ratios in ratios.values()
        for ratio in problem_ratios
    ]
    if not all_ratios:
        raise TrialError("trial has no common-solve CPU coordinates")
    baseline_median, baseline_maximum = summarized_ranges(
        values[spec.baseline]
    )
    candidate_median, candidate_maximum = summarized_ranges(
        values[spec.candidate]
    )
    ratio_median, ratio_maximum = summarized_ranges(ratios)
    return {
        "repetitions": repetitions,
        "common_ids": common,
        "coverage": {
            "baseline_reproducible_solves": len(baseline_solved),
            "candidate_reproducible_solves": len(candidate_solved),
            "common_reproducible_solves": len(common),
            "candidate_only": sorted(candidate_solved - baseline_solved),
            "baseline_only": sorted(baseline_solved - candidate_solved),
        },
        "observation": {
            "paired_coordinates": len(all_ratios),
            "candidate_over_baseline_median": rounded(
                statistics.median(all_ratios)
            ),
            "noise": {
                "coordinate_count": len(common),
                "baseline_median_relative_range": baseline_median,
                "baseline_max_relative_range": baseline_maximum,
                "candidate_median_relative_range": candidate_median,
                "candidate_max_relative_range": candidate_maximum,
                "paired_ratio_median_relative_range": ratio_median,
                "paired_ratio_max_relative_range": ratio_maximum,
            },
        },
    }


def _secondary_metric_map(record: dict[str, Any]) -> dict[str, float]:
    return {
        metric["name"]: float(metric["value"])
        for metric in record["performance"]["secondary_metrics"]
    }


def _source_performance(
    summary: dict[str, Any],
    spec: TrialSpec,
) -> dict[str, float]:
    if spec.candidate == "cache":
        larger = summary["phases"]["casc"]["budgets"]["larger"]
        targeted = summary["phases"]["targeted"]["budgets"]["targeted"]
        return {
            "generated_clause_ratio": larger[
                "common_solved_ratios"
            ]["median_generated_ratio"],
            "high_water_total_ratio": larger[
                "common_solved_ratios"
            ]["median_high_water_total_ratio"],
            "maximum_resident_pages_ratio": larger[
                "maximum_rss"
            ]["cache_over_recompute"],
            "targeted_total_cpu_ratio": targeted[
                "common_solved_ratios"
            ]["median_cpu_ratio"],
        }
    candidate = summary["phases"]["casc"]["budgets"]["heldout"][
        spec.candidate
    ]
    return {
        "generated_clause_ratio": candidate[
            "common_solved_ratios"
        ]["median_generated_ratio"],
        "high_water_total_ratio": candidate[
            "common_solved_ratios"
        ]["median_high_water_total_ratio"],
        "maximum_resident_pages_ratio": candidate[
            "maximum_rss"
        ]["candidate_over_baseline"],
        "blocked_clauses_removed": candidate[
            "transformation_activity"
        ]["removed_total"],
    }


def _source_decision(
    summary: dict[str, Any],
    spec: TrialSpec,
) -> str:
    if spec.candidate == "cache":
        return str(summary["decision"]["result"])
    return str(summary["decisions"][spec.candidate]["result"])


def _proof_counts(
    summary: dict[str, Any],
) -> dict[str, int]:
    proof = summary["proof_validation"]
    verified = int(proof["verified_cases"])
    expected = int(proof["expected_cases"])
    coverage_gaps = int(
        proof.get("coverage_gap_cases", expected - verified)
    )
    rejected = int(proof.get("rejected_cases", 0))
    return {
        "verified_claims": verified,
        "coverage_gaps": coverage_gaps,
        "rejected_claims": rejected,
    }


def _verified_candidate_witnesses(
    summary: dict[str, Any],
    candidate: str,
) -> set[str]:
    witnesses = set()
    for case in summary["proof_validation"]["cases"]:
        if case.get("strategy") != candidate:
            continue
        if case.get("gate_verdict") != "verified":
            continue
        if (
            "transformation_active" in case
            and not case["transformation_active"]
        ):
            continue
        witnesses.add(str(case["problem_id"]))
    return witnesses


def require_equal(
    observed: Any,
    expected: Any,
    location: str,
) -> None:
    if observed != expected:
        raise TrialError(
            f"{location} mismatch: expected {expected!r}, "
            f"observed {observed!r}"
        )


def verify_trial(
    repo_root: Path,
    spec: TrialSpec,
    *,
    verify_artifacts: bool,
) -> dict[str, Any]:
    record_path = EXPERIMENT_ROOT / spec.record_name
    record = VALIDATE.load_json(record_path)
    errors = VALIDATE.validate_record(
        record,
        repo_root=repo_root,
        verify_artifacts=verify_artifacts,
    )
    if errors:
        raise TrialError("; ".join(errors))
    archive_path = repo_root / spec.archive_path
    summary_path = repo_root / spec.summary_path
    summary = VALIDATE.load_json(summary_path)
    results = load_archive_results(archive_path)
    require_equal(
        len(results),
        record["reproduction"]["run_count"],
        "reproduction.run_count",
    )

    pairing = audit_status_pairs(
        results,
        spec.baseline,
        spec.candidate,
    )
    require_equal(
        pairing,
        record["correctness"]["status_pairing"],
        "correctness.status_pairing",
    )

    evidence = common_solve_evidence(results, spec)
    require_equal(
        evidence["coverage"],
        record["coverage"],
        "coverage",
    )
    record_observation = record["performance"]["observations"][0]
    for key, value in evidence["observation"].items():
        if key == "noise":
            for noise_key, noise_value in value.items():
                require_equal(
                    record_observation["noise"][noise_key],
                    noise_value,
                    f"performance.observations[0].noise.{noise_key}",
                )
        else:
            require_equal(
                record_observation[key],
                value,
                f"performance.observations[0].{key}",
            )

    require_equal(
        _secondary_metric_map(record),
        {
            name: float(value)
            for name, value in _source_performance(
                summary,
                spec,
            ).items()
        },
        "performance.secondary_metrics",
    )
    require_equal(
        _source_decision(summary, spec),
        spec.source_decision,
        "source decision",
    )
    require_equal(
        record["decision"]["source_result"],
        spec.source_decision,
        "decision.source_result",
    )
    require_equal(
        record["decision"]["outcome"],
        spec.expected_outcome,
        "decision.outcome",
    )

    independent = record["correctness"]["independent_validation"]
    for key, value in _proof_counts(summary).items():
        require_equal(
            independent[key],
            value,
            f"correctness.independent_validation.{key}",
        )
    available_witnesses = _verified_candidate_witnesses(
        summary,
        spec.candidate,
    )
    declared_witnesses = set(independent["candidate_witnesses"])
    if not declared_witnesses:
        raise TrialError("candidate witness list must not be empty")
    if not declared_witnesses <= available_witnesses:
        raise TrialError(
            "declared candidate witnesses are not independently verified: "
            + ", ".join(sorted(declared_witnesses - available_witnesses))
        )
    if spec.candidate != "cache":
        require_equal(
            bool(
                summary["proof_validation"]["candidate_validity"][
                    spec.candidate
                ]
            ),
            True,
            "proof candidate validity",
        )
    return {
        "record": spec.record_name,
        "runs": len(results),
        "paired_statuses": pairing["paired_coordinates"],
        "common_solves": evidence["coverage"][
            "common_reproducible_solves"
        ],
        "cpu_ratio": evidence["observation"][
            "candidate_over_baseline_median"
        ],
        "paired_ratio_max_relative_range": evidence["observation"][
            "noise"
        ]["paired_ratio_max_relative_range"],
        "decision": spec.expected_outcome,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=REPO_ROOT,
    )
    parser.add_argument(
        "--verify-artifacts",
        action="store_true",
        help="verify sizes and SHA-256 before recomputing trial evidence",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        reports = [
            verify_trial(
                arguments.repo_root.resolve(),
                spec,
                verify_artifacts=arguments.verify_artifacts,
            )
            for spec in TRIALS
        ]
    except (
        TrialError,
        OSError,
        ValueError,
        json.JSONDecodeError,
        tarfile.TarError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    for report in reports:
        print(
            f"OK: {report['record']}; {report['runs']} runs; "
            f"{report['paired_statuses']} status pairs; "
            f"{report['common_solves']} common solves; "
            f"CPU ratio {report['cpu_ratio']}; "
            "max paired-ratio repeat variation "
            f"{report['paired_ratio_max_relative_range']}; "
            f"decision {report['decision']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
