#!/usr/bin/env python3
"""Run the frozen local/inner-rewriting audit on the CASC-30 test split."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PRIOR_ROOT = EXPERIMENT_ROOT.parent / "2026-07-28-008-stronger-redundancy"
PRIOR_RUN_PATH = PRIOR_ROOT / "run.py"
CATEGORIES = ("FEQ", "FNE", "EPS", "UEQ")
QUOTAS = {"FEQ": 6, "FNE": 6, "EPS": 2, "UEQ": 6}
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


PRIOR = load_module("inference_gap_prior_run", PRIOR_RUN_PATH)
BASE = PRIOR.BASE
ExperimentError = BASE.ExperimentError


def strategy(kind: str, features: Sequence[str], *extra: str) -> dict[str, Any]:
    return {
        "kind": kind,
        "features": list(features),
        "args": [*COMMON_ARGS, *extra],
        "prior_harness_sha256": BASE.sha256_file(PRIOR_RUN_PATH),
    }


STRATEGIES = {
    "baseline": strategy(
        "baseline",
        ("indexed_subsumption", "full_forward_demodulation", "local_rw_off"),
    ),
    "local_rw": strategy(
        "inference_candidate",
        ("indexed_subsumption", "full_forward_demodulation", "local_rw_on"),
        "--local-rw=true",
    ),
}

PHASE_CONFIGS = {
    "audit": {
        "split": "test",
        "target_problems": 20,
        "repetitions": 2,
        "budgets": {
            "short": {"soft_cpu_seconds": 5, "hard_cpu_seconds": 7},
            "larger": {"soft_cpu_seconds": 20, "hard_cpu_seconds": 23},
        },
        "proof_objects": True,
    }
}


def select_records(
    records: list[dict[str, Any]], split: str, target: int
) -> list[dict[str, Any]]:
    selected = PRIOR.select_records(records, split, target)
    observed = {
        category: sum(record["category"] == category for record in selected)
        for category in CATEGORIES
    }
    if observed != QUOTAS:
        raise ExperimentError(
            f"selection quotas changed: expected {QUOTAS}, observed {observed}"
        )
    return selected


def phase_strategies(
    phase: str, selection_path: Path | None
) -> tuple[dict[str, dict[str, Any]], None, None]:
    if phase != "audit":
        raise ExperimentError(f"unsupported phase: {phase}")
    if selection_path is not None:
        raise ExperimentError("--selection is not accepted for the audit phase")
    return dict(STRATEGIES), None, None


_base_run_one = BASE.run_one


def expected_status_match(record: dict[str, Any], status: str | None) -> bool:
    return PRIOR.expected_status_match(record, status)


def run_one(**kwargs: Any) -> dict[str, Any]:
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
    BASE.UEQ_CATEGORY = "+".join(CATEGORIES)
    BASE.GENERAL_STRATEGIES = ("baseline",)
    BASE.SPECIALIST_STRATEGIES = ("local_rw",)
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
        "categories": CATEGORIES,
        "quotas": QUOTAS,
        "matrix_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "capability-matrix.json"
        ),
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
    configure_base()
    return BASE.main(actual_argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
