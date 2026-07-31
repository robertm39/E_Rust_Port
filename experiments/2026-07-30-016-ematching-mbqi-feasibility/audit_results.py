#!/usr/bin/env python3
"""Produce validation totals and concrete repeat-difference diagnostics."""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--repo-commit", required=True)
    parser.add_argument("--z3-commit", required=True)
    arguments = parser.parse_args()
    output_root = arguments.output_root.resolve()
    records = [
        json.loads(line)
        for line in (output_root / "results.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
        if line
    ]
    here = Path(__file__).resolve().parent
    environment_path = output_root / "environment.json"
    environment = json.loads(environment_path.read_text(encoding="utf-8"))
    environment.update(
        {
            "repo_commit": arguments.repo_commit,
            "z3_commit": arguments.z3_commit,
            "analyzer_sha256": sha256_file(here / "analyze.py"),
            "audit_sha256": sha256_file(here / "audit_results.py"),
            "run_experiment_sha256": sha256_file(
                here / "run_experiment.py"
            ),
            "run_shard_sha256": sha256_file(here / "run_shard.py"),
            "merge_results_sha256": sha256_file(
                here / "merge_results.py"
            ),
            "parallel_resume_sha256": sha256_file(
                here / "resume_parallel.py"
            ),
        }
    )
    environment_path.write_text(
        json.dumps(environment, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if (
            record["kind"] == "corpus"
            and record["partition"] in {"validation", "test"}
        ):
            grouped[(record["problem_id"], record["method"])].append(record)

    repeat_differences: list[dict[str, Any]] = []
    status_differences: list[str] = []
    for (problem_id, method), repetitions in sorted(grouped.items()):
        if len(repetitions) != 2:
            status_differences.append(f"{problem_id}/{method}:count")
            continue
        repetitions.sort(key=lambda record: record["repetition"])
        if len({record["status"] for record in repetitions}) != 1:
            status_differences.append(f"{problem_id}/{method}:status")
        if len({record["semantic_sha256"] for record in repetitions}) != 1:
            repeat_differences.append(
                {
                    "problem_id": problem_id,
                    "method": method,
                    "status": [record["status"] for record in repetitions],
                    "termination_reason": [
                        record["termination_reason"] for record in repetitions
                    ],
                    "generated_instances": [
                        record["generated_instances"] for record in repetitions
                    ],
                    "enumerated_substitutions": [
                        record["enumerated_substitutions"]
                        for record in repetitions
                    ],
                    "semantic_sha256": [
                        record["semantic_sha256"] for record in repetitions
                    ],
                    "instances_sha256": [
                        record["instances_sha256"] for record in repetitions
                    ],
                }
            )

    validation = {
        "schema_version": 1,
        "runs_checked": len(records),
        "instances_checked": sum(
            record["validation"]["instances_checked"] for record in records
        ),
        "proofs_checked": sum(
            bool(record["validation"].get("proof_checked"))
            for record in records
        ),
        "models_checked": sum(
            bool(record["validation"].get("model_checked"))
            for record in records
        ),
        "trigger_records_checked": sum(
            record["validation"].get("trigger_records_checked", 0)
            for record in records
        ),
        "trigger_instances_checked": sum(
            record["validation"].get("trigger_instances_checked", 0)
            for record in records
        ),
        "trigger_rounds_checked": sum(
            record["validation"].get("rounds_checked", 0)
            for record in records
        ),
        "refinement_models_checked": sum(
            record["validation"].get("refinement_models_checked", 0)
            for record in records
        ),
        "counterexample_instances_checked": sum(
            record["validation"].get(
                "counterexample_instances_checked", 0
            )
            for record in records
        ),
        "validation_failures": [
            record["run_id"]
            for record in records
            if not record["validation_passed"]
        ],
        "terminal_polarity_disagreements": [
            record["run_id"]
            for record in records
            if record["status"] in {"sat", "unsat"}
            and record["status"] != record["expected_status"]
        ],
        "repeat_status_differences": status_differences,
        "repeat_semantic_differences": repeat_differences,
        "mutations_rejected": [
            "substitution",
            "ground_clause",
            "trigger_binding",
            "refinement_model",
            "dimacs",
            "drat",
        ],
    }
    (output_root / "validation.json").write_text(
        json.dumps(validation, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    lines = [
        "# Repeat diagnostics",
        "",
        (
            f"All terminal statuses were stable: "
            f"{not status_differences}. Semantic trace differences: "
            f"{len(repeat_differences)}."
        ),
        "",
        "| Problem | Method | Status | Reasons | Instances | Enumerated |",
        "| --- | --- | --- | --- | ---: | ---: |",
    ]
    for difference in repeat_differences:
        lines.append(
            f"| {difference['problem_id']} | {difference['method']} | "
            f"{'/'.join(difference['status'])} | "
            f"{'/'.join(difference['termination_reason'])} | "
            f"{'/'.join(map(str, difference['generated_instances']))} | "
            f"{'/'.join(map(str, difference['enumerated_substitutions']))} |"
        )
    (output_root / "REPEAT-DIAGNOSTICS.md").write_text(
        "\n".join(lines) + "\n", encoding="utf-8", newline="\n"
    )

    top_level = [
        "analysis.json",
        "RESULTS.md",
        "REPEAT-DIAGNOSTICS.md",
        "validation.json",
        "environment.json",
        "corpus-manifest.json",
        "results.jsonl",
        "clausify-shard.stdout.txt",
        "clausify-shard.stderr.txt",
        "ematch-shard.stdout.txt",
        "ematch-shard.stderr.txt",
        "mbqi-shard.stdout.txt",
        "mbqi-shard.stderr.txt",
        "unit-tests.stdout.txt",
        "unit-tests.stderr.txt",
        "integration.stdout.txt",
        "integration.stderr.txt",
    ]
    selected_paths = sorted(
        {record["output_path"] for record in records}
    )
    manifest = [
        path for path in top_level if (output_root / path).exists()
    ] + selected_paths + ["evidence-files.txt"]
    (output_root / "evidence-files.txt").write_text(
        "\n".join(manifest) + "\n", encoding="utf-8", newline="\n"
    )
    print(json.dumps(validation, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
