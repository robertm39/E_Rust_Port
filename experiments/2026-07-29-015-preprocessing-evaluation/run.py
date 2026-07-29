#!/usr/bin/env python3
"""Run the frozen blocked-clause and definition preprocessing evaluation."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
PRIOR_RUN_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-29-014-rewrite-cache-evaluation"
    / "run.py"
)
CORPUS_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-29-014-rewrite-cache-evaluation"
    / "corpus.json"
)
TARGETED_MANIFEST_PATH = EXPERIMENT_ROOT / "targeted-manifest.jsonl"
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


PRIOR = load_module("preprocessing_prior_run", PRIOR_RUN_PATH)
BASE = PRIOR.BASE
ExperimentError = BASE.ExperimentError


def strategy(
    kind: str,
    extra_args: Sequence[str],
    transformation: str | None,
) -> dict[str, Any]:
    return {
        "kind": kind,
        "transformation": transformation,
        "args": [*COMMON_ARGS, *extra_args],
        "prior_harness_sha256": BASE.sha256_file(PRIOR_RUN_PATH),
    }


STRATEGIES = {
    "baseline": strategy("production_default", (), None),
    "bce": strategy(
        "default_off_candidate",
        ("--bce=true",),
        "blocked_clause_elimination",
    ),
    "predicate": strategy(
        "default_off_candidate",
        ("--pred-elim=true",),
        "predicate_elimination",
    ),
    "goal_defs": strategy(
        "default_off_candidate",
        ("--goal-defs=All",),
        "goal_definitions",
    ),
}

PHASE_CONFIGS = {
    "casc": {
        "split": "test",
        "target_problems": 20,
        "repetitions": 2,
        "budgets": {
            "heldout": {
                "soft_cpu_seconds": 10,
                "hard_cpu_seconds": 13,
            }
        },
        "proof_objects": True,
    },
    "differential": {
        "split": "differential",
        "target_problems": 3,
        "repetitions": 2,
        "budgets": {
            "differential": {
                "soft_cpu_seconds": 5,
                "hard_cpu_seconds": 8,
            }
        },
        "proof_objects": True,
    },
}


def load_frozen_corpus() -> dict[str, Any]:
    corpus = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    if not isinstance(corpus.get("categories"), dict):
        raise ExperimentError("frozen corpus has no category map")
    return corpus


def frozen_casc_entries() -> list[dict[str, str]]:
    corpus = load_frozen_corpus()
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
    if split == "test":
        desired = frozen_casc_entries()
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
    elif split == "differential":
        selected = sorted(
            (
                record
                for record in records
                if record["holdout_split"] == "differential"
            ),
            key=lambda record: record["official_category_order"],
        )
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


def run_one(**kwargs: Any) -> dict[str, Any]:
    outcome = _base_run_one(**kwargs)
    result_path = Path(outcome["result_path"])
    result = json.loads(result_path.read_text(encoding="utf-8"))
    record = kwargs["record"]
    corrected = PRIOR.expected_status_match(
        record, result["szs_status"]
    )
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
    BASE.UEQ_CATEGORY = "preprocessing-evaluation"
    BASE.GENERAL_STRATEGIES = tuple(STRATEGIES)
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
            TARGETED_MANIFEST_PATH
        ),
        "inventory_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "transformation-inventory.json"
        ),
        "preregistration_sha256": BASE.sha256_file(
            EXPERIMENT_ROOT / "PREREGISTRATION.md"
        ),
    }
    return {
        **body,
        "preview_id": hashlib.sha256(
            BASE.canonical_json(body)
        ).hexdigest(),
    }


def parse_experiment_inputs(
    argv: Sequence[str],
) -> tuple[str, list[str]]:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--source-revision", required=True)
    arguments, remaining = parser.parse_known_args(argv)
    revision = arguments.source_revision.lower()
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        raise ExperimentError("--source-revision must be a full Git SHA")
    return revision, remaining


def verify_manifest_argument(argv: Sequence[str]) -> None:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument(
        "--phase", choices=tuple(PHASE_CONFIGS), required=True
    )
    parser.add_argument("--manifest", type=Path, required=True)
    arguments, _remaining = parser.parse_known_args(argv)
    observed = BASE.sha256_file(arguments.manifest.resolve())
    expected = (
        load_frozen_corpus()["source_manifest_sha256"]
        if arguments.phase == "casc"
        else BASE.sha256_file(TARGETED_MANIFEST_PATH)
    )
    if observed != expected:
        raise ExperimentError(
            f"{arguments.phase} manifest hash mismatch: "
            f"expected {expected}, observed {observed}"
        )


def main(argv: Sequence[str] | None = None) -> int:
    actual_argv = list(sys.argv[1:] if argv is None else argv)
    if actual_argv == ["--contract-preview"]:
        print(json.dumps(contract_preview(), indent=2, sort_keys=True))
        return 0
    source_revision, remaining = parse_experiment_inputs(actual_argv)
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
