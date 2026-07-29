#!/usr/bin/env python3
"""Analyze TSM calibration, ranking cost, and held-out search evidence."""

from __future__ import annotations

import argparse
import json
import math
import re
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
BAD_STATUSES = {"Satisfiable", "CounterSatisfiable", "NonTheorem"}
CLASSIFIER_RE = re.compile(
    r"Evaluation:\s*([-+0-9.eE]+)\s+"
    r"Termeval:\s*([-+0-9.eE]+)\s+(OKOK|FAIL)"
)


class ExperimentError(RuntimeError):
    """Raised when raw evidence is incomplete or internally inconsistent."""


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def sigmoid(value: float) -> float:
    if value >= 0.0:
        inverse = math.exp(-value)
        return 1.0 / (1.0 + inverse)
    exponent = math.exp(value)
    return exponent / (1.0 + exponent)


def fit_logistic(
    scores: Sequence[float],
    labels: Sequence[float],
    weights: Sequence[float],
) -> dict[str, float]:
    total_weight = sum(weights)
    mean = sum(weight * score for score, weight in zip(scores, weights)) / total_weight
    variance = (
        sum(
            weight * (score - mean) ** 2
            for score, weight in zip(scores, weights)
        )
        / total_weight
    )
    scale = math.sqrt(variance) if variance > 1.0e-18 else 1.0
    normalized = [(score - mean) / scale for score in scores]
    positives = sum(
        weight for label, weight in zip(labels, weights) if label > 0.0
    )
    prior = min(max(positives / total_weight, 1.0e-9), 1.0 - 1.0e-9)
    intercept = math.log(prior / (1.0 - prior))
    slope = 0.0
    regularization = 1.0e-6
    for _iteration in range(100):
        g0 = 0.0
        g1 = regularization * slope
        h00 = 0.0
        h01 = 0.0
        h11 = regularization
        for value, label, weight in zip(normalized, labels, weights):
            target = 1.0 if label > 0.0 else 0.0
            probability = sigmoid(intercept + slope * value)
            residual = weight * (probability - target)
            curvature = weight * probability * (1.0 - probability)
            g0 += residual
            g1 += residual * value
            h00 += curvature
            h01 += curvature * value
            h11 += curvature * value * value
        determinant = h00 * h11 - h01 * h01
        if abs(determinant) < 1.0e-18:
            break
        delta0 = (h11 * g0 - h01 * g1) / determinant
        delta1 = (-h01 * g0 + h00 * g1) / determinant
        intercept -= delta0
        slope -= delta1
        if max(abs(delta0), abs(delta1)) < 1.0e-10:
            break
    return {
        "intercept": intercept,
        "slope": slope,
        "score_mean": mean,
        "score_scale": scale,
        "training_prior": prior,
    }


def calibrated_probability(score: float, model: dict[str, float]) -> float:
    normalized = (score - model["score_mean"]) / model["score_scale"]
    return sigmoid(model["intercept"] + model["slope"] * normalized)


def classifier_scores(path: Path) -> list[tuple[float, float, bool]]:
    parsed = []
    for line in path.read_text(encoding="utf-8").splitlines():
        match = CLASSIFIER_RE.search(line)
        if match:
            parsed.append(
                (
                    float(match.group(1)),
                    float(match.group(2)),
                    match.group(3) == "OKOK",
                )
            )
    if not parsed:
        raise ExperimentError(f"no classifier scores in {path}")
    return parsed


def weighted_classifier_metrics(
    parsed: Sequence[tuple[float, float, bool]],
    entries: Sequence[dict[str, Any]],
    model: dict[str, float],
) -> dict[str, Any]:
    if len(parsed) != len(entries):
        raise ExperimentError(
            f"classifier line count {len(parsed)} != metadata {len(entries)}"
        )
    total = sum(float(entry["sources"]) for entry in entries)
    positive_weight = sum(
        float(entry["sources"])
        for entry in entries
        if float(entry["label"]) > 0.0
    )
    negative_weight = total - positive_weight
    if positive_weight <= 0.0 or negative_weight <= 0.0:
        raise ExperimentError("classifier split does not contain both labels")

    correct = 0.0
    true_positive = 0.0
    true_negative = 0.0
    brier = 0.0
    bins = [
        {"weight": 0.0, "predicted": 0.0, "observed": 0.0}
        for _index in range(10)
    ]
    for (score, output_label, output_correct), entry in zip(parsed, entries):
        label = float(entry["label"])
        if output_label != label:
            raise ExperimentError("classifier output label order changed")
        weight = float(entry["sources"])
        if output_correct:
            correct += weight
            if label > 0.0:
                true_positive += weight
            else:
                true_negative += weight
        probability = calibrated_probability(score, model)
        target = 1.0 if label > 0.0 else 0.0
        brier += weight * (probability - target) ** 2
        bin_index = min(int(probability * 10.0), 9)
        bucket = bins[bin_index]
        bucket["weight"] += weight
        bucket["predicted"] += weight * probability
        bucket["observed"] += weight * target

    ece = 0.0
    rendered_bins = []
    for index, bucket in enumerate(bins):
        weight = bucket["weight"]
        if weight == 0.0:
            continue
        mean_prediction = bucket["predicted"] / weight
        observed_rate = bucket["observed"] / weight
        ece += weight / total * abs(mean_prediction - observed_rate)
        rendered_bins.append(
            {
                "bin": index,
                "weight": weight,
                "mean_probability": mean_prediction,
                "observed_positive_rate": observed_rate,
            }
        )
    prior = model["training_prior"]
    prior_brier = sum(
        float(entry["sources"])
        * (prior - (1.0 if float(entry["label"]) > 0.0 else 0.0)) ** 2
        for entry in entries
    ) / total
    return {
        "unique_patterns": len(entries),
        "weighted_patterns": total,
        "positive_weight": positive_weight,
        "negative_weight": negative_weight,
        "accuracy": correct / total,
        "balanced_accuracy": 0.5
        * (true_positive / positive_weight + true_negative / negative_weight),
        "brier_score": brier / total,
        "constant_prior_brier_score": prior_brier,
        "expected_calibration_error": ece,
        "calibration_bins": rendered_bins,
    }


def relative_range(values: Sequence[float]) -> float:
    median = statistics.median(values)
    return (max(values) - min(values)) / median if median else 0.0


def ranking_cost(
    timing_summary: dict[str, Any],
    metadata: dict[str, Any],
    split: str,
) -> dict[str, float]:
    workload = timing_summary["workloads"][split]
    timings = workload["timings"]
    cpu = [float(timing["cpu_seconds"]) for timing in timings]
    wall = [float(timing["wall_seconds"]) for timing in timings]
    weighted_patterns = sum(
        float(entry["sources"])
        for entry in metadata["heldout"][split]["entries"]
    )
    return {
        "median_cpu_seconds": statistics.median(cpu),
        "median_wall_seconds": statistics.median(wall),
        "cpu_microseconds_per_weighted_pattern": 1.0e6
        * statistics.median(cpu)
        / weighted_patterns,
        "wall_microseconds_per_weighted_pattern": 1.0e6
        * statistics.median(wall)
        / weighted_patterns,
        "cpu_relative_range": relative_range(cpu),
        "wall_relative_range": relative_range(wall),
    }


def unavailable_classifier_metrics(reason: str) -> dict[str, Any]:
    return {
        "status": "unavailable",
        "reason": reason,
        "unique_patterns": 0,
        "weighted_patterns": 0.0,
        "positive_weight": 0.0,
        "negative_weight": 0.0,
        "accuracy": None,
        "balanced_accuracy": None,
        "brier_score": None,
        "constant_prior_brier_score": None,
        "expected_calibration_error": None,
        "calibration_bins": [],
    }


def unavailable_ranking_cost(reason: str) -> dict[str, Any]:
    return {
        "status": "unavailable",
        "reason": reason,
        "median_cpu_seconds": None,
        "median_wall_seconds": None,
        "cpu_microseconds_per_weighted_pattern": None,
        "wall_microseconds_per_weighted_pattern": None,
        "cpu_relative_range": None,
        "wall_relative_range": None,
    }


def result_records(search_root: Path, split: str) -> list[dict[str, Any]]:
    records = []
    for result_path in sorted((search_root / split).rglob("result.json")):
        result = read_json(result_path)
        if result.get("phase") != split:
            continue
        telemetry_path = result_path.parent / "telemetry.json"
        telemetry = read_json(telemetry_path) if telemetry_path.is_file() else None
        result["_result_path"] = str(result_path)
        result["_telemetry"] = telemetry
        records.append(result)
    if len(records) != 32:
        raise ExperimentError(
            f"{split} has {len(records)} search results, expected 32"
        )
    return records


def median_or_none(values: Iterable[float]) -> float | None:
    materialized = list(values)
    return statistics.median(materialized) if materialized else None


def search_metrics(records: Sequence[dict[str, Any]]) -> dict[str, Any]:
    by_coordinate: dict[tuple[str, int], dict[str, dict[str, Any]]] = defaultdict(dict)
    telemetry_failures = 0
    bad_statuses = []
    load_failures = []
    for record in records:
        coordinate = (str(record["problem_id"]), int(record["repetition"]))
        by_coordinate[coordinate][str(record["strategy"])] = record
        telemetry = record["_telemetry"]
        if telemetry is None and record.get("szs_status") != "ResourceOut":
            telemetry_failures += 1
        if record.get("szs_status") in BAD_STATUSES:
            bad_statuses.append(
                {
                    "problem_id": record["problem_id"],
                    "strategy": record["strategy"],
                    "status": record["szs_status"],
                }
            )
        stderr = Path(record["_result_path"]).with_name("stderr.txt").read_text(
            encoding="utf-8", errors="replace"
        )
        if "panicked at" in stderr or "TSMWeight KB initialization" in stderr:
            load_failures.append(record["_result_path"])
    if any(set(pair) != {"control", "learned"} for pair in by_coordinate.values()):
        raise ExperimentError("search coordinate is not fully paired")

    ratios = []
    rss_ratios = []
    processed_ratios = []
    for pair in by_coordinate.values():
        control = pair["control"]
        learned = pair["learned"]
        if (
            control["szs_status"] in PROOF_STATUSES
            and learned["szs_status"] in PROOF_STATUSES
            and control["_telemetry"] is not None
            and learned["_telemetry"] is not None
        ):
            control_cpu = float(
                control["_telemetry"]["resources"]["total_cpu_seconds"]
            )
            learned_cpu = float(
                learned["_telemetry"]["resources"]["total_cpu_seconds"]
            )
            if control_cpu > 0.0:
                ratios.append(learned_cpu / control_cpu)
            control_rss = float(
                control["_telemetry"]["resources"]["maximum_resident_pages"]
            )
            learned_rss = float(
                learned["_telemetry"]["resources"]["maximum_resident_pages"]
            )
            if control_rss > 0.0:
                rss_ratios.append(learned_rss / control_rss)
            control_processed = float(
                control["_telemetry"]["search_funnel"][
                    "processed_non_trivial"
                ]
            )
            learned_processed = float(
                learned["_telemetry"]["search_funnel"][
                    "processed_non_trivial"
                ]
            )
            if control_processed > 0.0:
                processed_ratios.append(learned_processed / control_processed)

    reproducible = {}
    one_repeat_only = {}
    for strategy in ("control", "learned"):
        statuses: dict[str, list[bool]] = defaultdict(list)
        for record in records:
            if record["strategy"] == strategy:
                statuses[str(record["problem_id"])].append(
                    record["szs_status"] in PROOF_STATUSES
                )
        reproducible[strategy] = sorted(
            problem_id
            for problem_id, solved in statuses.items()
            if len(solved) == 2 and all(solved)
        )
        one_repeat_only[strategy] = sorted(
            problem_id
            for problem_id, solved in statuses.items()
            if any(solved) and not all(solved)
        )
    control_set = set(reproducible["control"])
    learned_set = set(reproducible["learned"])
    return {
        "run_count": len(records),
        "telemetry_failures": telemetry_failures,
        "bad_statuses": bad_statuses,
        "load_failures": load_failures,
        "reproducible_solves": reproducible,
        "one_repeat_only_solves": one_repeat_only,
        "common_reproducible_solves": sorted(control_set & learned_set),
        "learned_only_reproducible_solves": sorted(learned_set - control_set),
        "control_only_reproducible_solves": sorted(control_set - learned_set),
        "common_solve_coordinate_count": len(ratios),
        "median_common_solve_cpu_ratio": median_or_none(ratios),
        "median_common_solve_rss_ratio": median_or_none(rss_ratios),
        "median_common_solve_processed_ratio": median_or_none(
            processed_ratios
        ),
    }


def decide(summary: dict[str, Any]) -> dict[str, Any]:
    validation = summary["search"]["validation"]
    test = summary["search"]["test"]
    test_classifier = summary["classification"]["test"]
    test_cost = summary["ranking_cost"]["test"]
    correctness = (
        not validation["bad_statuses"]
        and not test["bad_statuses"]
        and not validation["load_failures"]
        and not test["load_failures"]
        and validation["telemetry_failures"] == 0
        and test["telemetry_failures"] == 0
    )
    sufficient_classifier = (
        test_classifier["weighted_patterns"] >= 20.0
        and test_classifier["positive_weight"] > 0.0
        and test_classifier["negative_weight"] > 0.0
    )
    no_test_loss = not test["control_only_reproducible_solves"]
    calibration_pass = (
        sufficient_classifier
        and test_classifier["balanced_accuracy"] > 0.55
        and test_classifier["brier_score"]
        < test_classifier["constant_prior_brier_score"]
        and test_classifier["expected_calibration_error"] <= 0.20
    )
    test_cost_per_pattern = test_cost[
        "cpu_microseconds_per_weighted_pattern"
    ]
    cost_pass = (
        test_cost_per_pattern is not None and test_cost_per_pattern < 50.0
    )
    solve_pass = bool(test["learned_only_reproducible_solves"]) or (
        validation["median_common_solve_cpu_ratio"] is not None
        and test["median_common_solve_cpu_ratio"] is not None
        and validation["median_common_solve_cpu_ratio"] <= 0.95
        and test["median_common_solve_cpu_ratio"] <= 0.95
    )
    uncertain = (
        not sufficient_classifier
        or test_cost_per_pattern is None
        or bool(validation["one_repeat_only_solves"]["control"])
        or bool(validation["one_repeat_only_solves"]["learned"])
        or bool(test["one_repeat_only_solves"]["control"])
        or bool(test["one_repeat_only_solves"]["learned"])
    )
    if (
        correctness
        and sufficient_classifier
        and no_test_loss
        and calibration_pass
        and cost_pass
        and solve_pass
    ):
        verdict = "continue"
    elif (
        not correctness
        or not no_test_loss
        or (
            test_cost_per_pattern is not None
            and test_cost_per_pattern > 100.0
        )
    ):
        verdict = "stop"
    elif uncertain:
        verdict = "uncertain"
    else:
        verdict = "stop"
    return {
        "verdict": verdict,
        "correctness_pass": correctness,
        "sufficient_classifier_coverage": sufficient_classifier,
        "no_reproducible_test_solve_lost": no_test_loss,
        "calibration_pass": calibration_pass,
        "ranking_cost_pass": cost_pass,
        "solve_or_common_speed_pass": solve_pass,
        "production_effect": "none_pending_followup"
        if verdict == "continue"
        else "leave_tsm_out_of_automatic_schedules",
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--classifier-input-root", type=Path, required=True)
    parser.add_argument("--classifier-output-root", type=Path, required=True)
    parser.add_argument("--search-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    input_root = arguments.classifier_input_root.resolve()
    output_root = arguments.classifier_output_root.resolve()
    search_root = arguments.search_root.resolve()
    metadata = read_json(input_root / "metadata.json")
    timings = read_json(output_root / "summary.json")

    training_parsed = classifier_scores(output_root / "train-self.stdout")
    training_entries = metadata["training"]
    if len(training_parsed) != len(training_entries):
        raise ExperimentError("training classifier output is misaligned")
    model = fit_logistic(
        [score for score, _label, _correct in training_parsed],
        [float(entry["label"]) for entry in training_entries],
        [float(entry["sources"]) for entry in training_entries],
    )
    classification = {}
    costs = {}
    for split in ("validation", "test"):
        split_metadata = metadata["heldout"][split]
        if split_metadata.get("status") == "unavailable":
            reason = str(split_metadata["reason"])
            classification[split] = unavailable_classifier_metrics(reason)
            costs[split] = unavailable_ranking_cost(reason)
            continue
        classification[split] = weighted_classifier_metrics(
            classifier_scores(output_root / f"{split}.stdout"),
            split_metadata["entries"],
            model,
        )
        costs[split] = ranking_cost(timings, metadata, split)
    summary = {
        "schema_version": 1,
        "calibration_model": model,
        "classification": classification,
        "ranking_cost": costs,
        "search": {
            split: search_metrics(result_records(search_root, split))
            for split in ("validation", "test")
        },
    }
    summary["decision"] = decide(summary)
    write_json(arguments.output, summary)
    print(json.dumps(summary["decision"], sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
