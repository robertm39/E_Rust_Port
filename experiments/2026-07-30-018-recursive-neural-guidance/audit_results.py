#!/usr/bin/env python3
"""Fail-closed audit of a validation artifact directory."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


SEEDS = [11, 23, 37, 53, 71]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def macro(record: dict[str, object], metric: str) -> float:
    return float(record["metrics"]["macro"][metric])


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"AUDIT FAILED: {message}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("result", type=Path)
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path(__file__).with_name("trace_manifest.jsonl"),
    )
    arguments = parser.parse_args()
    result_path = arguments.result.resolve()
    root = result_path.parent
    result = json.loads(result_path.read_text(encoding="utf-8"))
    manifest = [
        json.loads(line)
        for line in arguments.manifest.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]

    require(result["phase"] == "validation", "result is not validation phase")
    require(
        result["manifest_sha256"] == sha256_file(arguments.manifest),
        "manifest hash mismatch",
    )
    expected = {
        (row["problem"], row["split"]): row
        for row in manifest
        if row["split"] in {"train", "validation"}
    }
    actual = {
        (row["problem"], row["split"]): row for row in result["extraction"]
    }
    require(actual.keys() == expected.keys(), "extraction problem/split set differs")
    for key, expected_row in expected.items():
        actual_row = actual[key]
        for field in (
            "family",
            "archive_member",
            "sha256",
            "given_count",
            "positive_count",
            "proof_evalgc_count",
            "unmatched_evalgc_count",
        ):
            require(
                actual_row[field] == expected_row[field],
                f"{key}: extraction field {field} differs",
            )

    linear = result["linear"]
    linear_path = root / linear["model_file"]
    require(linear_path.is_file(), "linear model missing")
    require(linear_path.stat().st_size == linear["model_bytes"], "linear size differs")
    require(sha256_file(linear_path) == linear["model_sha256"], "linear hash differs")
    require(linear["repeat_exact"] is True, "linear repeat check failed")

    recursive = result["recursive"]["seeds"]
    require([row["seed"] for row in recursive] == SEEDS, "seed list differs")
    for row in recursive:
        model_path = root / row["model_file"]
        require(model_path.is_file(), f"seed {row['seed']}: model missing")
        require(
            model_path.stat().st_size == row["model_bytes"],
            f"seed {row['seed']}: model size differs",
        )
        require(
            sha256_file(model_path) == row["model_sha256"],
            f"seed {row['seed']}: model hash differs",
        )
        require(row["repeat_exact"] is True, f"seed {row['seed']}: repeat failed")
        for metric in row["metrics"]["macro"].values():
            require(0.0 <= float(metric) <= 1.0, "metric is outside [0, 1]")

    ordered = sorted(
        recursive, key=lambda row: (macro(row, "average_precision"), row["seed"])
    )
    selected = ordered[len(ordered) // 2]
    require(
        selected["seed"] == result["recursive"]["selected_seed"],
        "selected seed is not median validation AP",
    )
    aps = [macro(row, "average_precision") for row in recursive]
    require(
        abs((max(aps) - min(aps)) - result["recursive"]["ap_range"]) < 1e-12,
        "reported AP range differs",
    )

    linear_ap = macro(linear, "average_precision")
    linear_top10 = macro(linear, "top_10_percent_recall")
    linear_prefix = macro(linear, "all_positive_prefix_fraction")
    selected_ap = macro(selected, "average_precision")
    selected_top10 = macro(selected, "top_10_percent_recall")
    selected_prefix = macro(selected, "all_positive_prefix_fraction")
    better_seeds = sum(
        macro(row, "average_precision") > linear_ap
        and macro(row, "top_10_percent_recall") > linear_top10
        for row in recursive
    )
    independent_checks = {
        "ap_effect": selected_ap >= linear_ap + 0.03,
        "top10_effect": selected_top10 >= linear_top10 + 0.05,
        "prefix_effect": selected_prefix <= 0.80 * linear_prefix,
        "four_of_five_seeds": better_seeds >= 4,
        "ap_seed_range": max(aps) - min(aps) <= 0.10,
        "in_process_latency": float(
            result["recursive"]["in_process"]["microseconds_per_clause"]
        )
        <= 100.0,
        "external_latency": float(
            result["recursive"]["external_process"]["microseconds_per_clause"]
        )
        <= 500.0,
        "model_size": selected["model_bytes"] <= 1024 * 1024,
        "peak_rss": result["resources"]["peak_process_rss_bytes"]
        <= 256 * 1024 * 1024,
        "repeat_exact": result["recursive"]["in_process"]["repeat_exact"] is True
        and result["recursive"]["external_process"]["repeat_exact"] is True,
    }
    require(independent_checks == result["gate_checks"], "gate checks differ")
    expected_verdict = (
        "advance-test" if all(independent_checks.values()) else "stop-offline-validation"
    )
    require(result["verdict"] == expected_verdict, "verdict differs from gates")
    if expected_verdict == "stop-offline-validation":
        require(result["test"]["status"] == "not-run", "failed gate exposed test")
    require(result["end_to_end"]["status"] == "not-run", "unexpected online run")
    print(
        json.dumps(
            {
                "audit": "ok",
                "verdict": expected_verdict,
                "result_sha256": sha256_file(result_path),
                "models_checked": 1 + len(recursive),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
