#!/usr/bin/env python3
"""Run the frozen shared rewrite-cache evaluation on Linux."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PRIOR_RUN_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-29-013-inference-gap-audit"
    / "run.py"
)
CORPUS_PATH = EXPERIMENT_ROOT / "corpus.json"
BASE_WEIGHT = "Refinedweight(ConstPrio,2,1,1.5,1.1,1.1)"
FIFO = "FIFOWeight(ConstPrio)"
COMMON_ARGS = [
    f"--expert-heuristic=(5*{BASE_WEIGHT},1*{FIFO})",
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
]


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PRIOR = load_module("rewrite_cache_prior_run", PRIOR_RUN_PATH)
BASE = PRIOR.BASE
ExperimentError = BASE.ExperimentError
ABLATION_BINARY: Path | None = None
ABLATION_BINARY_SHA256: str | None = None


def strategy(kind: str, features: Sequence[str]) -> dict[str, Any]:
    return {
        "kind": kind,
        "features": list(features),
        "args": list(COMMON_ARGS),
        "prior_harness_sha256": BASE.sha256_file(PRIOR_RUN_PATH),
    }


STRATEGIES = {
    "cache": strategy(
        "production_cache",
        (
            "shared_top_rewrite_links",
            "shared_structural_rewrite_links",
            "rule_and_full_normal_form_dates",
        ),
    ),
    "recompute": strategy(
        "proof_preserving_ablation",
        (
            "ignore_persistent_rewrite_links",
            "ignore_normal_form_date_fast_return",
            "retain_intra_call_proof_chain",
        ),
    ),
}

PHASE_CONFIGS = {
    "casc": {
        "split": "test",
        "target_problems": 20,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    },
    "targeted": {
        "split": "rewrite_heavy",
        "target_problems": 5,
        "repetitions": 2,
        "budgets": {
            "targeted": {"soft_cpu_seconds": 30, "hard_cpu_seconds": 33}
        },
        "proof_objects": True,
    },
}


def load_frozen_corpus() -> dict[str, Any]:
    corpus = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    categories = corpus.get("categories")
    if not isinstance(categories, dict):
        raise ExperimentError("frozen corpus has no category map")
    return corpus


def frozen_casc_entries(corpus: dict[str, Any]) -> list[dict[str, str]]:
    entries: list[dict[str, str]] = []
    for category in ("FEQ", "FNE", "EPS", "UEQ"):
        category_entries = corpus["categories"].get(category)
        if not isinstance(category_entries, list):
            raise ExperimentError(f"missing frozen category: {category}")
        for entry in category_entries:
            entries.append(
                {
                    "category": category,
                    "problem_id": entry["problem_id"],
                    "sha256": entry["sha256"],
                }
            )
    return entries


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    corpus = load_frozen_corpus()
    if split == "test":
        desired = frozen_casc_entries(corpus)
        by_id = {record["problem_id"]: record for record in records}
        selected = []
        for entry in desired:
            record = by_id.get(entry["problem_id"])
            if record is None:
                raise ExperimentError(
                    f"manifest lacks frozen problem {entry['problem_id']}"
                )
            if (
                record["holdout_split"] != "test"
                or record["category"] != entry["category"]
                or record["sha256"] != entry["sha256"]
            ):
                raise ExperimentError(
                    f"frozen metadata changed for {entry['problem_id']}"
                )
            selected.append(record)
    elif split == "rewrite_heavy":
        desired_paths = corpus.get("rewrite_heavy")
        if not isinstance(desired_paths, list):
            raise ExperimentError("frozen corpus has no rewrite-heavy list")
        by_path = {record["path"]: record for record in records}
        selected = []
        for path in desired_paths:
            record = by_path.get(path)
            if record is None:
                raise ExperimentError(f"targeted manifest lacks {path}")
            if record["holdout_split"] != "rewrite_heavy":
                raise ExperimentError(f"targeted split changed for {path}")
            selected.append(record)
    else:
        raise ExperimentError(f"unsupported frozen split: {split}")

    if len(selected) != target:
        raise ExperimentError(
            f"selected {len(selected)} problems, expected {target}"
        )
    if len({record["problem_id"] for record in selected}) != target:
        raise ExperimentError("frozen problem IDs are not unique")
    return selected


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[dict[str, dict[str, Any]], None, None]:
    if phase not in PHASE_CONFIGS:
        raise ExperimentError(f"unsupported phase: {phase}")
    if selection_path is not None:
        raise ExperimentError("--selection is not accepted")
    return dict(STRATEGIES), None, None


_base_run_one = BASE.run_one


def expected_status_match(record: dict[str, Any], status: str | None) -> bool:
    return PRIOR.expected_status_match(record, status)


def run_one(**kwargs: Any) -> dict[str, Any]:
    if kwargs["strategy_name"] == "recompute":
        if ABLATION_BINARY is None or ABLATION_BINARY_SHA256 is None:
            raise ExperimentError("ablation binary was not configured")
        kwargs["binary"] = ABLATION_BINARY
        kwargs["binary_sha256"] = ABLATION_BINARY_SHA256

    outcome = _base_run_one(**kwargs)
    result_path = Path(outcome["result_path"])
    result = json.loads(result_path.read_text(encoding="utf-8"))
    record = kwargs["record"]
    corrected = expected_status_match(record, result["szs_status"])
    if (
        result.get("expected_status_match") != corrected
        or result.get("expected_class") != record["expected_class"]
    ):
        result["expected_class"] = record["expected_class"]
        result["expected_status_match"] = corrected
        BASE.atomic_json(result_path, result)
    return outcome


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.UEQ_CATEGORY = "rewrite-cache-evaluation"
    BASE.GENERAL_STRATEGIES = ("cache", "recompute")
    BASE.SPECIALIST_STRATEGIES = ()
    BASE.STRATEGIES = STRATEGIES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.select_family_balanced_records = select_records
    BASE.phase_strategies = phase_strategies
    BASE.run_one = run_one


def contract_preview() -> dict[str, Any]:
    body = {
        "schema_version": 1,
        "phase_configs": PHASE_CONFIGS,
        "strategies": STRATEGIES,
        "corpus_sha256": BASE.sha256_file(CORPUS_PATH),
        "targeted_manifest_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "targeted-manifest.jsonl"
        ),
        "preregistration_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "PREREGISTRATION.md"
        ),
        "protocol_amendment_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "PROTOCOL_AMENDMENT.md"
        ),
    }
    return {
        **body,
        "preview_id": hashlib.sha256(BASE.canonical_json(body)).hexdigest(),
    }


def parse_experiment_inputs(
    argv: Sequence[str],
) -> tuple[Path, str, list[str]]:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--ablation-binary", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    arguments, remaining = parser.parse_known_args(argv)
    binary = arguments.ablation_binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise ExperimentError(
            f"ablation binary is missing or not executable: {binary}"
        )
    revision = arguments.source_revision.lower()
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        raise ExperimentError("--source-revision must be a full Git SHA")
    return binary, revision, remaining


def verify_manifest_argument(argv: Sequence[str]) -> None:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--phase", choices=tuple(PHASE_CONFIGS), required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    arguments, _remaining = parser.parse_known_args(argv)
    manifest = arguments.manifest.resolve()
    observed = BASE.sha256_file(manifest)
    if arguments.phase == "casc":
        expected = load_frozen_corpus()["source_manifest_sha256"]
    else:
        expected = BASE.sha256_file(
            EXPERIMENT_ROOT / "targeted-manifest.jsonl"
        )
    if observed != expected:
        raise ExperimentError(
            f"{arguments.phase} manifest hash mismatch: "
            f"expected {expected}, observed {observed}"
        )


def main(argv: Sequence[str] | None = None) -> int:
    global ABLATION_BINARY, ABLATION_BINARY_SHA256

    actual_argv = list(sys.argv[1:] if argv is None else argv)
    if actual_argv == ["--contract-preview"]:
        print(json.dumps(contract_preview(), indent=2, sort_keys=True))
        return 0
    ABLATION_BINARY, source_revision, remaining = (
        parse_experiment_inputs(actual_argv)
    )
    ABLATION_BINARY_SHA256 = BASE.sha256_file(ABLATION_BINARY)
    STRATEGIES["recompute"]["binary_sha256"] = ABLATION_BINARY_SHA256
    for strategy_config in STRATEGIES.values():
        strategy_config["source_revision"] = source_revision
    verify_manifest_argument(remaining)
    configure_base()
    return BASE.main(remaining)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
