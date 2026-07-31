#!/usr/bin/env python3
"""Aggregate the frozen E-matching/MBQI experiment and apply its gates."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


METHODS = ("clausify", "ematch", "mbqi")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def stable_sha256(value: object) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(encoded).hexdigest()


def ratio(numerator: int | float, denominator: int | float) -> float | None:
    if denominator == 0:
        return None
    return numerator / denominator


def status_counts(records: Iterable[dict[str, Any]]) -> dict[str, int]:
    return dict(sorted(Counter(record["status"] for record in records).items()))


def verified_solve(record: dict[str, Any]) -> bool:
    return bool(record["verified"]) and record["status"] in {"sat", "unsat"}


def reproducible_heldout_solves(
    records: list[dict[str, Any]], method: str
) -> set[str]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if (
            record["kind"] == "corpus"
            and record["partition"] in {"validation", "test"}
            and record["method"] == method
        ):
            grouped[record["problem_id"]].append(record)
    return {
        problem_id
        for problem_id, repetitions in grouped.items()
        if len(repetitions) == 2
        and all(verified_solve(record) for record in repetitions)
        and len({record["status"] for record in repetitions}) == 1
    }


def paired_records(
    records: list[dict[str, Any]],
    left: str,
    right: str,
    problems: set[str],
) -> list[tuple[dict[str, Any], dict[str, Any]]]:
    by_key = {
        (
            record["problem_id"],
            record["repetition"],
            record["method"],
        ): record
        for record in records
        if record["kind"] == "corpus"
    }
    pairs: list[tuple[dict[str, Any], dict[str, Any]]] = []
    for record in records:
        if (
            record["kind"] != "corpus"
            or record["method"] != left
            or record["problem_id"] not in problems
        ):
            continue
        counterpart = by_key.get(
            (record["problem_id"], record["repetition"], right)
        )
        if counterpart is not None:
            pairs.append((record, counterpart))
    return pairs


def markdown_report(analysis: dict[str, Any]) -> str:
    lines = [
        "# E-matching and MBQI comparison results",
        "",
        (
            f"Real coordinates: {analysis['corpus_coordinates']}; "
            f"hand coordinates: {analysis['hand_coordinates']}."
        ),
        "",
        "| Method | Verified terminal coordinates | SAT | UNSAT | UNKNOWN | Instances |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for method in METHODS:
        metrics = analysis["methods"][method]
        counts = metrics["status_counts"]
        lines.append(
            f"| {method} | {metrics['verified_terminal_coordinates']} | "
            f"{counts.get('sat', 0)} | {counts.get('unsat', 0)} | "
            f"{counts.get('unknown', 0)} | {metrics['instances']} |"
        )
    lines.extend(
        [
            "",
            "## Reproducible held-out solves",
            "",
        ]
    )
    for method in METHODS:
        solved = analysis["heldout_solves"][method]
        lines.append(f"- {method}: {len(solved)} — {solved}")
    comparison = analysis["comparison"]
    lines.extend(
        [
            "",
            "## E-matching mechanics",
            "",
            (
                f"- trigger rounds: {analysis['ematch']['rounds']}; "
                f"candidate matches: {analysis['ematch']['candidate_matches']}"
            ),
            (
                f"- duplicate substitutions / ground clauses: "
                f"{analysis['ematch']['duplicate_substitutions']} / "
                f"{analysis['ematch']['duplicate_ground_clauses']}"
            ),
            (
                f"- fixed points: {analysis['ematch']['fixed_points']}; "
                f"ungenerated model counterexamples: "
                f"{analysis['ematch']['ungenerated_counterexamples']}"
            ),
            "",
            "## Model-counterexample mechanics",
            "",
            (
                f"- SAT calls: {analysis['mbqi']['sat_calls']}; "
                f"refinements: {analysis['mbqi']['refinements']}; "
                f"enumerated substitutions: "
                f"{analysis['mbqi']['enumerated_substitutions']}"
            ),
            "",
            "## Comparison and decision",
            "",
            f"- E-matching unique over MBQI: {comparison['ematch_unique_over_mbqi']}",
            f"- E-matching losses versus MBQI: {comparison['ematch_lost_to_mbqi']}",
            (
                "- aggregate E-matching/MBQI instance ratio on common held-out "
                f"solves: {comparison['instance_ratio_on_common']}"
            ),
            (
                "- median E-matching/MBQI wall ratio on common held-out "
                f"coordinates: {comparison['median_wall_ratio_on_common']}"
            ),
            (
                "- clausification contains every held-out E-matching solve: "
                f"{comparison['clausification_contains_ematch']}"
            ),
            "",
            f"Decision: `{analysis['decision']['result']}`.",
            "",
        ]
    )
    return "\n".join(lines)


def analyze(output_root: Path) -> dict[str, Any]:
    results_path = output_root / "results.jsonl"
    records = [
        json.loads(line)
        for line in results_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    corpus = [record for record in records if record["kind"] == "corpus"]
    hand = [record for record in records if record["kind"] == "hand"]
    if len(corpus) % len(METHODS) != 0 or len(hand) % len(METHODS) != 0:
        raise ValueError("incomplete treatment matrix")

    expected_disagreements = [
        record["run_id"]
        for record in records
        if record["status"] in {"sat", "unsat"}
        and record["status"] != record["expected_status"]
    ]
    validation_failures = [
        record["run_id"] for record in records if not record["validation_passed"]
    ]
    hand_failures = [
        record["run_id"]
        for record in hand
        if record["status"] != record["expected_status"]
    ]

    repeat_groups: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in corpus:
        if record["partition"] in {"validation", "test"}:
            repeat_groups[(record["problem_id"], record["method"])].append(record)
    repeat_failures: list[str] = []
    for (problem_id, method), repetitions in sorted(repeat_groups.items()):
        if len(repetitions) != 2 or len(
            {record["semantic_sha256"] for record in repetitions}
        ) != 1:
            repeat_failures.append(f"{problem_id}/{method}")

    methods: dict[str, Any] = {}
    for method in METHODS:
        selected = [record for record in corpus if record["method"] == method]
        methods[method] = {
            "status_counts": status_counts(selected),
            "verified_terminal_coordinates": sum(
                verified_solve(record) for record in selected
            ),
            "instances": sum(record["generated_instances"] for record in selected),
            "wall_seconds": sum(record["search_wall_seconds"] for record in selected),
            "sat_calls": sum(record["sat_calls"] for record in selected),
            "refinements": sum(record["refinement_iterations"] for record in selected),
            "enumerated_substitutions": sum(
                record["enumerated_substitutions"] for record in selected
            ),
        }

    heldout = {
        method: reproducible_heldout_solves(corpus, method)
        for method in METHODS
    }
    common = heldout["ematch"] & heldout["mbqi"]
    pairs = paired_records(corpus, "ematch", "mbqi", common)
    ematch_instances = sum(left["generated_instances"] for left, _ in pairs)
    mbqi_instances = sum(right["generated_instances"] for _, right in pairs)
    wall_ratios = [
        left["search_wall_seconds"] / right["search_wall_seconds"]
        for left, right in pairs
        if right["search_wall_seconds"] > 0
    ]
    instance_ratio = ratio(ematch_instances, mbqi_instances)
    median_wall_ratio = (
        statistics.median(wall_ratios) if wall_ratios else None
    )
    ematch_unique = heldout["ematch"] - heldout["mbqi"]
    ematch_lost = heldout["mbqi"] - heldout["ematch"]
    retained_all = not ematch_lost
    reduction_alternative = (
        retained_all
        and instance_ratio is not None
        and instance_ratio <= 0.5
        and median_wall_ratio is not None
        and median_wall_ratio <= 1.5
    )
    clausification_contains = heldout["ematch"] <= heldout["clausify"]

    ematch_records = [
        record for record in corpus if record["method"] == "ematch"
    ]
    ematch_metrics = {
        "rounds": sum(record["method_data"]["round_count"] for record in ematch_records),
        "candidate_matches": sum(
            record["method_data"]["candidate_matches"] for record in ematch_records
        ),
        "duplicate_substitutions": sum(
            record["method_data"]["duplicate_substitutions"]
            for record in ematch_records
        ),
        "duplicate_ground_clauses": sum(
            record["method_data"]["duplicate_ground_clauses"]
            for record in ematch_records
        ),
        "fixed_points": sum(
            bool(record["method_data"]["fixed_point"])
            for record in ematch_records
        ),
        "ungenerated_counterexamples": sum(
            record["method_data"]["first_ungenerated_counterexample"] is not None
            for record in ematch_records
        ),
        "unary_patterns": sum(
            record["method_data"]["unary_patterns"] for record in ematch_records
        ),
        "multipatterns": sum(
            record["method_data"]["multipatterns"] for record in ematch_records
        ),
        "maximum_pattern_size": max(
            (
                record["method_data"]["maximum_pattern_size"]
                for record in ematch_records
            ),
            default=0,
        ),
    }
    mbqi_records = [record for record in corpus if record["method"] == "mbqi"]
    mbqi_metrics = {
        "sat_calls": sum(record["sat_calls"] for record in mbqi_records),
        "refinements": sum(
            record["refinement_iterations"] for record in mbqi_records
        ),
        "enumerated_substitutions": sum(
            record["enumerated_substitutions"] for record in mbqi_records
        ),
    }

    correctness_passed = not (
        expected_disagreements
        or validation_failures
        or hand_failures
        or repeat_failures
    )
    prototype_supported = correctness_passed and (
        bool(ematch_unique) or reduction_alternative
    )
    clausification_dominant = (
        correctness_passed
        and clausification_contains
        and not ematch_unique
        and not reduction_alternative
    )
    if not correctness_passed:
        decision = "stop"
    elif prototype_supported:
        decision = "prototype-supported"
    elif clausification_dominant:
        decision = "clausification-dominant"
    else:
        decision = "defer"

    trace_material = [
        {
            "run_id": record["run_id"],
            "semantic_sha256": record["semantic_sha256"],
        }
        for record in sorted(records, key=lambda item: item["run_id"])
    ]
    analysis: dict[str, Any] = {
        "schema_version": 1,
        "corpus_coordinates": len(corpus) // len(METHODS),
        "hand_coordinates": len(hand) // len(METHODS),
        "runs": len(records),
        "methods": methods,
        "heldout_solves": {
            method: sorted(problems) for method, problems in heldout.items()
        },
        "ematch": ematch_metrics,
        "mbqi": mbqi_metrics,
        "comparison": {
            "common_heldout_solves": sorted(common),
            "ematch_unique_over_mbqi": sorted(ematch_unique),
            "ematch_lost_to_mbqi": sorted(ematch_lost),
            "ematch_instances_on_common": ematch_instances,
            "mbqi_instances_on_common": mbqi_instances,
            "instance_ratio_on_common": instance_ratio,
            "median_wall_ratio_on_common": median_wall_ratio,
            "clausification_contains_ematch": clausification_contains,
            "reduction_alternative": reduction_alternative,
        },
        "correctness": {
            "passed": correctness_passed,
            "expected_disagreements": expected_disagreements,
            "validation_failures": validation_failures,
            "hand_failures": hand_failures,
            "repeat_semantic_failures": repeat_failures,
        },
        "decision": {
            "result": decision,
            "prototype_supported": prototype_supported,
            "clausification_dominant": clausification_dominant,
        },
        "semantic_trace_sha256": stable_sha256(trace_material),
        "results_sha256": sha256_file(results_path),
    }
    analysis["analysis_sha256"] = stable_sha256(analysis)
    (output_root / "analysis.json").write_text(
        json.dumps(analysis, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    (output_root / "RESULTS.md").write_text(
        markdown_report(analysis), encoding="utf-8", newline="\n"
    )
    return analysis


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", type=Path, required=True)
    arguments = parser.parse_args()
    analysis = analyze(arguments.output_root.resolve())
    print(json.dumps(analysis, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
