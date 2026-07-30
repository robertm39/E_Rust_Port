#!/usr/bin/env python3
"""Run the frozen structure-matched TSM search comparison on Linux."""

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
BASE_RUN_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "run.py"
)
CORPUS_PATH = EXPERIMENT_ROOT / "corpus.jsonl"
CATEGORIES = ("FNE", "FEQ", "EPU", "UEQ")
SOURCE_REVISION = "812323618aaa42d0f5e24bba8a0ef146ff1757cd"
SOURCE_BINARY_SHA256 = (
    "82db6c558f64d24b46e7b9eb5562b803874a3653d8a1ee99d0ec378d8449802d"
)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("tsm_learning_base_run", BASE_RUN_PATH)
ExperimentError = BASE.ExperimentError
STRATEGIES: dict[str, dict[str, Any]] = {}
PHASE_CONFIGS = {
    "validation": {
        "split": "validation",
        "target_problems": 8,
        "repetitions": 2,
        "budgets": {
            "heldout": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": True,
    },
    "test": {
        "split": "test",
        "target_problems": 8,
        "repetitions": 2,
        "budgets": {
            "heldout": {"soft_cpu_seconds": 8, "hard_cpu_seconds": 10}
        },
        "proof_objects": True,
    },
}


def sha256_tree(path: Path) -> str:
    digest = hashlib.sha256()
    for entry in sorted(candidate for candidate in path.rglob("*") if candidate.is_file()):
        digest.update(entry.relative_to(path).as_posix().encode())
        digest.update(b"\0")
        digest.update(BASE.sha256_file(entry).encode())
        digest.update(b"\n")
    return digest.hexdigest()


def configure_strategies(kb: Path) -> None:
    kb_literal = str(kb).replace("\\", "/")
    learned_weight = (
        "TSMLearned=TSMWeight(ConstPrio,1,1,2,flat,"
        f"{kb_literal},100000,1.0,1.0,Flat,IndexIdentity,100000,"
        "-20,20,-2,-1,0,2)"
    )
    control_weight = "TSMControl=Clauseweight(ConstPrio,1,1,2)"
    learned_heuristic = (
        "TSMLearnedSearch=(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
        "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
        "5*TSMLearned,1*FIFOWeight(PreferWatchlist))"
    )
    control_heuristic = (
        "TSMControlSearch=(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
        "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
        "5*TSMControl,1*FIFOWeight(PreferWatchlist))"
    )
    common = [
        "--term-ordering=KBO6",
        "--forward-demod-level=2",
        "--record-gcs",
        "--force-deriv=2",
    ]
    STRATEGIES.clear()
    STRATEGIES.update(
        {
            "control": {
                "kind": "structure_matched_non_learning",
                "args": [
                    f"--define-weight-function={control_weight}",
                    f"--define-heuristic={control_heuristic}",
                    "--expert-heuristic=TSMControlSearch",
                    *common,
                ],
                "queue_ratios": [10, 10, 5, 1],
                "learned_queue": False,
            },
            "learned": {
                "kind": "proof_derived_tsm",
                "args": [
                    f"--define-weight-function={learned_weight}",
                    f"--define-heuristic={learned_heuristic}",
                    "--expert-heuristic=TSMLearnedSearch",
                    *common,
                ],
                "queue_ratios": [10, 10, 5, 1],
                "learned_queue": True,
                "knowledge_base_sha256": sha256_tree(kb),
            },
        }
    )


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    selected = sorted(
        (
            record
            for record in records
            if record["experiment_split"] == split
        ),
        key=lambda record: (
            CATEGORIES.index(str(record["category"])),
            str(record["selection_rank"]),
        ),
    )
    if len(selected) != target:
        raise ExperimentError(
            f"{split} contains {len(selected)} records, expected {target}"
        )
    if {record["category"] for record in selected} != set(CATEGORIES):
        raise ExperimentError(f"{split} does not cover all categories")
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


def run_one(**kwargs: Any) -> dict[str, Any]:
    if kwargs["binary_sha256"] != SOURCE_BINARY_SHA256:
        raise ExperimentError("binary hash differs from the frozen search binary")
    outcome = _base_run_one(**kwargs)
    result_path = Path(outcome["result_path"])
    result = json.loads(result_path.read_text(encoding="utf-8"))
    expected = kwargs["record"]["expected_class"]
    if expected not in {"theorem", "unsatisfiable"}:
        raise ExperimentError(f"unsupported expected class: {expected}")
    expected_match = result["szs_status"] in PROOF_STATUSES
    if result.get("expected_status_match") != expected_match:
        result["expected_status_match"] = expected_match
        BASE.atomic_json(result_path, result)
    return outcome


def configure_base() -> None:
    BASE.__file__ = __file__
    BASE.UEQ_CATEGORY = "+".join(CATEGORIES)
    BASE.GENERAL_STRATEGIES = tuple(STRATEGIES)
    BASE.SPECIALIST_STRATEGIES = ()
    BASE.STRATEGIES = STRATEGIES
    BASE.PHASE_CONFIGS = PHASE_CONFIGS
    BASE.select_family_balanced_records = select_records
    BASE.phase_strategies = phase_strategies
    BASE.run_one = run_one


def parse_experiment_args(
    argv: Sequence[str],
) -> tuple[Path, str | None, list[str]]:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--knowledge-base", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    parser.add_argument("--strategy", choices=("control", "learned"))
    arguments, remaining = parser.parse_known_args(argv)
    if arguments.source_revision != SOURCE_REVISION:
        raise ExperimentError("source revision differs from preregistration")
    kb = arguments.knowledge_base.resolve()
    if not kb.is_dir():
        raise ExperimentError(f"knowledge base is missing: {kb}")
    required = {"description", "signature", "problems", "clausepatterns"}
    missing = sorted(name for name in required if not (kb / name).is_file())
    if missing:
        raise ExperimentError(f"knowledge base is incomplete: {missing}")
    return kb, arguments.strategy, remaining


def contract_preview() -> dict[str, Any]:
    body = {
        "schema_version": 1,
        "source_revision": SOURCE_REVISION,
        "phase_configs": PHASE_CONFIGS,
        "corpus_sha256": BASE.sha256_file(CORPUS_PATH),
        "preregistration_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "PREREGISTRATION.md"
        ),
    }
    return {
        **body,
        "preview_id": hashlib.sha256(BASE.canonical_json(body)).hexdigest(),
    }


def main(argv: Sequence[str] | None = None) -> int:
    actual_argv = list(sys.argv[1:] if argv is None else argv)
    if actual_argv == ["--contract-preview"]:
        print(json.dumps(contract_preview(), indent=2, sort_keys=True))
        return 0
    kb, strategy, remaining = parse_experiment_args(actual_argv)
    configure_strategies(kb)
    if strategy is not None:
        selected_strategy = STRATEGIES[strategy]
        STRATEGIES.clear()
        STRATEGIES[strategy] = selected_strategy
    configure_base()
    return BASE.main(remaining)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
