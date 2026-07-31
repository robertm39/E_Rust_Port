#!/usr/bin/env python3
"""Run the frozen validation or conditional-test neural ranking study."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Sequence

from neural_common import (
    SEEDS,
    IntegrityError,
    LinearModel,
    Observation,
    RecursiveEncoder,
    RecursiveModel,
    chronological_scores,
    evaluate_scores,
    load_manifest,
    load_model,
    read_split_from_archive,
    save_model,
    score_observations,
    scores_checksum,
    sha256_file,
    train_linear,
    train_recursive,
)


ARCHIVE_SHA256 = "8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156"


def write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def _macro(metrics: dict[str, object], key: str) -> float:
    return float(metrics["macro"][key])


def _split_observations(
    observations: Sequence[Observation], split: str
) -> list[Observation]:
    return [observation for observation in observations if observation.split == split]


def _peak_rss_bytes() -> int:
    import resource

    maximum = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    # Linux reports KiB; macOS reports bytes. The experiment runs on Ubuntu.
    return int(maximum * 1024)


def _cpu_seconds() -> tuple[float, float]:
    import resource

    own = resource.getrusage(resource.RUSAGE_SELF)
    children = resource.getrusage(resource.RUSAGE_CHILDREN)
    return own.ru_utime + children.ru_utime, own.ru_stime + children.ru_stime


def _linux_peak_rss_bytes(process_id: int) -> int | None:
    status = Path(f"/proc/{process_id}/status")
    if not status.is_file():
        return None
    for line in status.read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            fields = line.split()
            if len(fields) >= 2:
                return int(fields[1]) * 1024
    return None


def benchmark_in_process(
    model: RecursiveModel, observations: Sequence[Observation], repetitions: int = 7
) -> dict[str, object]:
    encoder = RecursiveEncoder(model.seed)

    def score_all() -> list[float]:
        return [
            model.score_clause(observation.literals, encoder)
            for observation in observations
        ]

    first = score_all()
    second = score_all()
    if first != second:
        raise IntegrityError("recursive in-process repeat scores differ")
    start = time.perf_counter_ns()
    final = first
    for _ in range(repetitions):
        final = score_all()
    elapsed = time.perf_counter_ns() - start
    return {
        "repetitions": repetitions,
        "clause_evaluations": repetitions * len(observations),
        "microseconds_per_clause": elapsed
        / (1000.0 * repetitions * len(observations)),
        "score_checksum": scores_checksum(final),
        "repeat_exact": True,
    }


def _worker_request(
    process: subprocess.Popen[str], clauses: Sequence[str]
) -> list[float]:
    assert process.stdin is not None
    assert process.stdout is not None
    process.stdin.write(json.dumps({"clauses": clauses}, separators=(",", ":")) + "\n")
    process.stdin.flush()
    line = process.stdout.readline()
    if not line:
        stderr = process.stderr.read() if process.stderr is not None else ""
        raise RuntimeError(f"inference worker exited without a response: {stderr}")
    response = json.loads(line)
    if "error" in response:
        raise RuntimeError(f"inference worker error: {response['error']}")
    return [float(value) for value in response["scores"]]


def benchmark_external(
    model_path: Path,
    observations: Sequence[Observation],
    repetitions: int = 5,
    batch_size: int = 64,
) -> dict[str, object]:
    worker_path = Path(__file__).with_name("inference_worker.py")
    process = subprocess.Popen(
        [sys.executable, str(worker_path), "--model", str(model_path)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    batches = [
        [observation.raw_clause for observation in observations[index : index + batch_size]]
        for index in range(0, len(observations), batch_size)
    ]
    try:
        warmup = [
            score
            for batch in batches
            for score in _worker_request(process, batch)
        ]
        start = time.perf_counter_ns()
        final = warmup
        for _ in range(repetitions):
            final = [
                score
                for batch in batches
                for score in _worker_request(process, batch)
            ]
        elapsed = time.perf_counter_ns() - start
        repeat = [
            score
            for batch in batches
            for score in _worker_request(process, batch)
        ]
        if final != repeat:
            raise IntegrityError("external-process repeat scores differ")
        worker_peak_rss = _linux_peak_rss_bytes(process.pid)
    finally:
        if process.stdin is not None:
            process.stdin.close()
        return_code = process.wait(timeout=10)
        if return_code != 0:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise RuntimeError(
                f"inference worker failed with exit {return_code}: {stderr}"
            )
    return {
        "persistent_process": True,
        "batch_size": batch_size,
        "repetitions": repetitions,
        "clause_evaluations": repetitions * len(observations),
        "microseconds_per_clause": elapsed
        / (1000.0 * repetitions * len(observations)),
        "score_checksum": scores_checksum(final),
        "repeat_exact": True,
        "worker_peak_rss_bytes": worker_peak_rss,
    }


def validation_gate(
    linear_metrics: dict[str, object],
    recursive_results: Sequence[dict[str, object]],
    selected: dict[str, object],
    in_process: dict[str, object],
    external: dict[str, object],
    model_size: int,
    peak_rss_bytes: int,
) -> dict[str, bool]:
    linear_ap = _macro(linear_metrics, "average_precision")
    linear_top10 = _macro(linear_metrics, "top_10_percent_recall")
    linear_prefix = _macro(linear_metrics, "all_positive_prefix_fraction")
    selected_metrics = selected["metrics"]
    selected_ap = _macro(selected_metrics, "average_precision")
    selected_top10 = _macro(selected_metrics, "top_10_percent_recall")
    selected_prefix = _macro(selected_metrics, "all_positive_prefix_fraction")
    better_seeds = sum(
        _macro(result["metrics"], "average_precision") > linear_ap
        and _macro(result["metrics"], "top_10_percent_recall") > linear_top10
        for result in recursive_results
    )
    aps = [
        _macro(result["metrics"], "average_precision")
        for result in recursive_results
    ]
    return {
        "ap_effect": selected_ap >= linear_ap + 0.03,
        "top10_effect": selected_top10 >= linear_top10 + 0.05,
        "prefix_effect": selected_prefix <= 0.80 * linear_prefix,
        "four_of_five_seeds": better_seeds >= 4,
        "ap_seed_range": max(aps) - min(aps) <= 0.10,
        "in_process_latency": float(in_process["microseconds_per_clause"]) <= 100.0,
        "external_latency": float(external["microseconds_per_clause"]) <= 500.0,
        "model_size": model_size <= 1024 * 1024,
        "peak_rss": peak_rss_bytes <= 256 * 1024 * 1024,
        "repeat_exact": bool(in_process["repeat_exact"])
        and bool(external["repeat_exact"]),
    }


def test_gate(
    linear_metrics: dict[str, object],
    recursive_results: Sequence[dict[str, object]],
    selected_seed: int,
) -> dict[str, bool]:
    linear_ap = _macro(linear_metrics, "average_precision")
    linear_top10 = _macro(linear_metrics, "top_10_percent_recall")
    linear_prefix = _macro(linear_metrics, "all_positive_prefix_fraction")
    selected = next(
        result for result in recursive_results if int(result["seed"]) == selected_seed
    )
    selected_metrics = selected["metrics"]
    aps = [
        _macro(result["metrics"], "average_precision")
        for result in recursive_results
    ]
    better_seeds = sum(
        _macro(result["metrics"], "average_precision") > linear_ap
        and _macro(result["metrics"], "top_10_percent_recall") > linear_top10
        for result in recursive_results
    )
    return {
        "ap_effect": _macro(selected_metrics, "average_precision")
        >= linear_ap + 0.03,
        "top10_effect": _macro(selected_metrics, "top_10_percent_recall")
        >= linear_top10 + 0.05,
        "prefix_effect": _macro(
            selected_metrics, "all_positive_prefix_fraction"
        )
        <= 0.80 * linear_prefix,
        "four_of_five_seeds": better_seeds >= 4,
        "ap_seed_range": max(aps) - min(aps) <= 0.10,
    }


def run_validation(arguments: argparse.Namespace) -> dict[str, object]:
    phase_wall_start = time.perf_counter()
    cpu_user_start, cpu_system_start = _cpu_seconds()
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    manifest = load_manifest(arguments.manifest)
    observations, extraction = read_split_from_archive(
        arguments.archive,
        manifest,
        {"train", "validation"},
        ARCHIVE_SHA256,
    )
    train = _split_observations(observations, "train")
    validation = _split_observations(observations, "validation")
    if not train or not validation:
        raise IntegrityError("train or validation split is empty")

    start = time.perf_counter()
    linear = train_linear(train)
    linear_training_seconds = time.perf_counter() - start
    linear_path = output / "linear-model.json"
    linear_size = save_model(linear, linear_path)
    linear_scores = score_observations(linear, validation)
    if linear_scores != score_observations(load_model(linear_path), validation):
        raise IntegrityError("linear serialized-model repeat scores differ")
    linear_metrics = evaluate_scores(validation, linear_scores)

    recursive_results: list[dict[str, object]] = []
    for seed in SEEDS:
        start = time.perf_counter()
        model = train_recursive(train, seed)
        training_seconds = time.perf_counter() - start
        model_path = output / f"recursive-seed-{seed}.json"
        size = save_model(model, model_path)
        scores = score_observations(model, validation)
        loaded_scores = score_observations(load_model(model_path), validation)
        if scores != loaded_scores:
            raise IntegrityError(f"seed {seed}: serialized-model repeat scores differ")
        recursive_results.append(
            {
                "seed": seed,
                "metrics": evaluate_scores(validation, scores),
                "score_checksum": scores_checksum(scores),
                "training_seconds": training_seconds,
                "model_file": model_path.name,
                "model_sha256": sha256_file(model_path),
                "model_bytes": size,
                "repeat_exact": True,
            }
        )

    ordered = sorted(
        recursive_results,
        key=lambda result: (
            _macro(result["metrics"], "average_precision"),
            int(result["seed"]),
        ),
    )
    selected = ordered[len(ordered) // 2]
    selected_seed = int(selected["seed"])
    selected_path = output / str(selected["model_file"])
    selected_model = load_model(selected_path)
    if not isinstance(selected_model, RecursiveModel):
        raise IntegrityError("selected model is not recursive")
    in_process = benchmark_in_process(selected_model, validation)
    external = benchmark_external(selected_path, validation)
    peak_rss = _peak_rss_bytes()
    checks = validation_gate(
        linear_metrics,
        recursive_results,
        selected,
        in_process,
        external,
        int(selected["model_bytes"]),
        peak_rss,
    )
    verdict = "advance-test" if all(checks.values()) else "stop-offline-validation"
    cpu_user_end, cpu_system_end = _cpu_seconds()
    result: dict[str, object] = {
        "schema_version": 1,
        "phase": "validation",
        "verdict": verdict,
        "source_revision": arguments.source_revision,
        "archive": {
            "path": str(arguments.archive.resolve()),
            "sha256": ARCHIVE_SHA256,
        },
        "manifest_sha256": sha256_file(arguments.manifest),
        "extraction": extraction,
        "split_counts": {
            "train": {"rows": len(train), "positives": sum(row.label for row in train)},
            "validation": {
                "rows": len(validation),
                "positives": sum(row.label for row in validation),
            },
        },
        "chronological": {
            "metrics": evaluate_scores(validation, chronological_scores(validation))
        },
        "linear": {
            "metrics": linear_metrics,
            "training_seconds": linear_training_seconds,
            "model_file": linear_path.name,
            "model_sha256": sha256_file(linear_path),
            "model_bytes": linear_size,
            "score_checksum": scores_checksum(linear_scores),
            "repeat_exact": True,
        },
        "recursive": {
            "seeds": recursive_results,
            "selected_seed": selected_seed,
            "selection_rule": "median validation macro average precision; seed tie-break",
            "ap_range": max(
                _macro(result["metrics"], "average_precision")
                for result in recursive_results
            )
            - min(
                _macro(result["metrics"], "average_precision")
                for result in recursive_results
            ),
            "in_process": in_process,
            "external_process": external,
        },
        "resources": {
            "phase_wall_seconds": time.perf_counter() - phase_wall_start,
            "process_user_cpu_seconds": cpu_user_end - cpu_user_start,
            "process_system_cpu_seconds": cpu_system_end - cpu_system_start,
            "peak_process_rss_bytes": peak_rss,
            "runner_cpu_count": os.cpu_count(),
        },
        "packaging": {
            "custom_runtime": "Python standard library only",
            "onnx_runtime": "not-evaluated: unavailable and forbidden before quality gate",
            "repository_dependencies_added": 0,
        },
        "gate_checks": checks,
        "test": {
            "status": "authorized-but-not-run"
            if verdict == "advance-test"
            else "not-run",
            "reason": None
            if verdict == "advance-test"
            else "preregistered validation gate failed",
        },
        "end_to_end": {
            "status": "not-run",
            "solve_count": "not-run",
            "cpu": "not-run",
            "memory": "not-run",
            "reason": "offline gate precedes a separately preregistered online experiment",
        },
    }
    result_path = output / "validation-result.json"
    write_json(result_path, result)
    return result


def run_test(arguments: argparse.Namespace) -> dict[str, object]:
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    validation_path = arguments.validation_result.resolve()
    validation = json.loads(validation_path.read_text(encoding="utf-8"))
    if validation.get("verdict") != "advance-test":
        raise IntegrityError("validation result does not authorize test evaluation")
    model_root = validation_path.parent
    selected_seed = int(validation["recursive"]["selected_seed"])

    manifest = load_manifest(arguments.manifest)
    test, extraction = read_split_from_archive(
        arguments.archive, manifest, {"test"}, ARCHIVE_SHA256
    )
    if not test or any(observation.split != "test" for observation in test):
        raise IntegrityError("test split is empty or contaminated")

    linear_record = validation["linear"]
    linear_path = model_root / linear_record["model_file"]
    if sha256_file(linear_path) != linear_record["model_sha256"]:
        raise IntegrityError("linear model hash differs from validation result")
    linear = load_model(linear_path)
    if not isinstance(linear, LinearModel):
        raise IntegrityError("linear model has wrong kind")
    linear_scores = score_observations(linear, test)
    linear_metrics = evaluate_scores(test, linear_scores)

    recursive_results: list[dict[str, object]] = []
    for record in validation["recursive"]["seeds"]:
        model_path = model_root / record["model_file"]
        if sha256_file(model_path) != record["model_sha256"]:
            raise IntegrityError(f"seed {record['seed']}: model hash differs")
        model = load_model(model_path)
        if not isinstance(model, RecursiveModel):
            raise IntegrityError(f"seed {record['seed']}: wrong model kind")
        scores = score_observations(model, test)
        recursive_results.append(
            {
                "seed": int(record["seed"]),
                "metrics": evaluate_scores(test, scores),
                "score_checksum": scores_checksum(scores),
            }
        )

    checks = test_gate(linear_metrics, recursive_results, selected_seed)
    verdict = (
        "advance-online-experiment"
        if all(checks.values())
        else "stop-offline-test"
    )
    result = {
        "schema_version": 1,
        "phase": "test",
        "verdict": verdict,
        "source_revision": arguments.source_revision,
        "validation_result": {
            "path": str(validation_path),
            "sha256": sha256_file(validation_path),
            "selected_seed": selected_seed,
        },
        "archive": {
            "path": str(arguments.archive.resolve()),
            "sha256": ARCHIVE_SHA256,
        },
        "manifest_sha256": sha256_file(arguments.manifest),
        "extraction": extraction,
        "split_counts": {
            "test": {"rows": len(test), "positives": sum(row.label for row in test)}
        },
        "chronological": {
            "metrics": evaluate_scores(test, chronological_scores(test))
        },
        "linear": {
            "metrics": linear_metrics,
            "score_checksum": scores_checksum(linear_scores),
        },
        "recursive": {
            "seeds": recursive_results,
            "selected_seed": selected_seed,
            "ap_range": max(
                _macro(result["metrics"], "average_precision")
                for result in recursive_results
            )
            - min(
                _macro(result["metrics"], "average_precision")
                for result in recursive_results
            ),
        },
        "gate_checks": checks,
        "end_to_end": {
            "status": "not-run",
            "solve_count": "not-run",
            "cpu": "not-run",
            "memory": "not-run",
            "reason": "requires separately preregistered online experiment",
        },
    }
    write_json(output / "test-result.json", result)
    return result


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=("validation", "test"))
    parser.add_argument("--archive", required=True, type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).with_name("trace_manifest.jsonl"),
    )
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument(
        "--source-revision",
        default="a9a5acabdf2e7d7db6ef6b520c63e5debf39097f",
    )
    parser.add_argument("--validation-result", type=Path)
    arguments = parser.parse_args()
    if arguments.phase == "test" and arguments.validation_result is None:
        parser.error("test phase requires --validation-result")
    return arguments


def main() -> int:
    arguments = parse_arguments()
    result = (
        run_validation(arguments)
        if arguments.phase == "validation"
        else run_test(arguments)
    )
    print(json.dumps({"phase": result["phase"], "verdict": result["verdict"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
