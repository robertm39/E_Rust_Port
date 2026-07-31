#!/usr/bin/env python3
"""Compare deterministic decision fields across two validation replications."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def decision_projection(result: dict[str, object]) -> dict[str, object]:
    return {
        "phase": result["phase"],
        "verdict": result["verdict"],
        "source_revision": result["source_revision"],
        "archive_sha256": result["archive"]["sha256"],
        "manifest_sha256": result["manifest_sha256"],
        "extraction": result["extraction"],
        "split_counts": result["split_counts"],
        "chronological_metrics": result["chronological"]["metrics"],
        "linear_metrics": result["linear"]["metrics"],
        "linear_model_sha256": result["linear"]["model_sha256"],
        "linear_score_checksum": result["linear"]["score_checksum"],
        "recursive_selected_seed": result["recursive"]["selected_seed"],
        "recursive_ap_range": result["recursive"]["ap_range"],
        "recursive_seeds": [
            {
                "seed": row["seed"],
                "metrics": row["metrics"],
                "score_checksum": row["score_checksum"],
                "model_sha256": row["model_sha256"],
            }
            for row in result["recursive"]["seeds"]
        ],
        "gate_checks": result["gate_checks"],
        "test": result["test"],
        "end_to_end": result["end_to_end"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("first", type=Path)
    parser.add_argument("second", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    first = json.loads(arguments.first.read_text(encoding="utf-8"))
    second = json.loads(arguments.second.read_text(encoding="utf-8"))
    first_projection = decision_projection(first)
    second_projection = decision_projection(second)
    if first_projection != second_projection:
        raise SystemExit("REPRODUCIBILITY FAILED: deterministic decision fields differ")
    report = {
        "reproducibility": "exact",
        "first_result_sha256": sha256_file(arguments.first),
        "second_result_sha256": sha256_file(arguments.second),
        "decision_projection_sha256": hashlib.sha256(
            json.dumps(
                first_projection, sort_keys=True, separators=(",", ":")
            ).encode("utf-8")
        ).hexdigest(),
        "dynamic_fields_excluded": [
            "training_seconds",
            "inference_timing",
            "process_cpu",
            "process_rss",
            "absolute_paths",
        ],
    }
    text = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if arguments.output is None:
        print(text, end="")
    else:
        arguments.output.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
