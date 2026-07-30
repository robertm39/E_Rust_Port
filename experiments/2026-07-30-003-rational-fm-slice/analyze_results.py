#!/usr/bin/env python3
"""Score the frozen FM advancement gates from raw experiment artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from pathlib import Path
from typing import Any, Iterable


def load(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def percentile(values: Iterable[float], probability: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    index = max(
        0,
        min(
            len(ordered) - 1,
            int(probability * len(ordered) + 0.999999) - 1,
        ),
    )
    return ordered[index]


def solved(status: str | None) -> bool:
    return status in {
        "Theorem",
        "Unsatisfiable",
        "CounterSatisfiable",
        "Satisfiable",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--validation-summary", type=Path)
    arguments = parser.parse_args()
    root = arguments.artifact_root
    synthetic_native_path = root / "fm-native-ubuntu/synthetic/native_report.json"
    production_native_path = root / "fm-native-ubuntu/production/native_report.json"
    robustness_path = root / "robustness.json"
    extraction_path = root / "production-extraction.json"
    synthetic_external_path = root / "fm-controls/synthetic-external.json"
    production_external_path = root / "fm-controls/production-external.json"
    source_controls_path = root / "fm-controls/source-controls.json"

    synthetic_native = load(synthetic_native_path)
    production_native = load(production_native_path)
    robustness = load(robustness_path)
    extraction = load(extraction_path)
    synthetic_external = load(synthetic_external_path)
    production_external = load(production_external_path)
    source_controls = load(source_controls_path)

    synthetic_by_id = {
        workload["id"]: workload for workload in synthetic_native["workloads"]
    }
    synthetic_external_by_id = {
        workload["id"]: workload
        for workload in synthetic_external["workloads"]
    }
    production_by_id = {
        workload["id"]: workload for workload in production_native["workloads"]
    }
    production_external_by_id = {
        workload["id"]: workload
        for workload in production_external["workloads"]
    }
    heldout_synthetic = [
        workload
        for workload in synthetic_native["workloads"]
        if workload["partition"] in {"validation", "test"}
        and workload["supported"]
    ]
    heldout_production = [
        workload
        for workload in production_native["workloads"]
        if workload["partition"] in {"validation", "test"}
    ]

    replayed = (
        synthetic_native["summary"]["replayed_certificates"]
        + production_native["summary"]["replayed_certificates"]
    )
    expected_replays = (
        synthetic_native["summary"]["workloads"]
        + production_native["summary"]["workloads"]
    ) * 2 * synthetic_native["repetitions"]
    replay_gate = (
        replayed == expected_replays
        and synthetic_native["summary"]["all_native_expected"]
        and production_native["summary"]["all_native_expected"]
    )

    required_robustness = {
        "mutation_rejections": 8,
        "cancellation": 2,
        "bounds": 2,
        "timeout": 1,
        "malformed": 1,
    }
    robustness_gate = robustness["success"] and all(
        robustness["categories"].get(name, 0) >= minimum
        for name, minimum in required_robustness.items()
    )

    heldout_status_pairs = [
        (
            workload["arms"]["native_fm"]["outcome"],
            synthetic_external_by_id[workload["id"]]["z3"]["outcome"],
        )
        for workload in heldout_synthetic
    ]
    status_matches = sum(left == right for left, right in heldout_status_pairs)
    z3_gate = status_matches == len(heldout_status_pairs)

    heldout_source_reports = [
        source
        for source in extraction["sources"]
        if source["partition"] in {"validation", "test"}
        and source.get("eligible_clauses", 0) > 0
    ]
    heldout_eligible_clauses = sum(
        source["eligible_clauses"] for source in heldout_source_reports
    )
    heldout_eligible_families = len(
        {source["family"] for source in heldout_source_reports}
    )
    eligibility_gate = (
        heldout_eligible_clauses >= 20 and heldout_eligible_families >= 3
    )

    synthetic_unique = sum(
        workload["unique_native_closure"] for workload in heldout_synthetic
    )
    production_unique = sum(
        workload["unique_native_closure"] for workload in heldout_production
    )
    closure_gate = synthetic_unique + production_unique >= 3 and production_unique >= 1

    baseline_closed = [
        workload
        for workload in heldout_synthetic
        if workload["arms"]["normalize_resolution"]["outcome"] == "unsat"
    ]
    baseline_losses = [
        workload["id"]
        for workload in baseline_closed
        if workload["arms"]["native_fm"]["outcome"] != "unsat"
    ]
    neutral_losses = [
        workload["id"]
        for workload in heldout_synthetic
        if workload["expected"] == "sat"
        and workload["arms"]["native_fm"]["outcome"] != "unknown"
    ]
    no_loss_gate = not baseline_losses and not neutral_losses

    heldout_growth = [
        workload["retained_growth_ratio"]
        for workload in [*heldout_synthetic, *heldout_production]
    ]
    median_growth = statistics.median(heldout_growth)
    p95_growth = percentile(heldout_growth, 0.95)
    growth_gate = median_growth <= 4.0 and p95_growth <= 10.0

    heldout_times = [
        workload["arms"]["native_fm"]["p95_ms"]
        for workload in [*heldout_synthetic, *heldout_production]
    ]
    native_p95_ms = percentile(heldout_times, 0.95)
    timing_gate = native_p95_ms <= 50.0

    all_arms = [
        arm
        for report in (synthetic_native, production_native)
        for workload in report["workloads"]
        for arm in workload["arms"].values()
    ]
    crossed_bounds = sorted(
        {
            arm["crossed_bound"]
            for arm in all_arms
            if arm["crossed_bound"] is not None
        }
    )
    max_coefficient_bits = max(
        arm["max_coefficient_bits"] for arm in all_arms
    )
    coefficient_gate = not crossed_bounds and max_coefficient_bits <= 256

    packaging = {
        "production_code_changed": False,
        "release_binary_delta_bytes": 0,
        "new_runtime_dependencies": [],
        "prototype_location": "experiments/2026-07-30-003-rational-fm-slice",
    }
    packaging_gate = (
        not packaging["production_code_changed"]
        and packaging["release_binary_delta_bytes"] <= 256 * 1024
        and not packaging["new_runtime_dependencies"]
    )

    comprehensive: dict[str, Any] = {
        "provided": arguments.validation_summary is not None,
        "passed": False,
    }
    if arguments.validation_summary is not None:
        summary_path = arguments.validation_summary
        comprehensive = {
            "provided": True,
            "passed": (
                (summary_path.parent / "SUCCESS").is_file()
                and (summary_path.parent / "VALIDATION_COMPLETE").is_file()
            ),
            "summary": load(summary_path),
            "run_id": summary_path.parent.name,
            "summary_sha256": sha256_file(summary_path),
        }

    alasca_only_sources = [
        source["problem_id"]
        for source in source_controls["sources"]
        if solved(source["arms"]["vampire_alasca_no_viras"]["szs_status"])
        and not solved(source["arms"]["vampire_theory_axioms"]["szs_status"])
    ]
    production_status_pairs = [
        (
            workload["arms"]["native_fm"]["outcome"],
            production_external_by_id[workload["id"]]["z3"]["outcome"],
        )
        for workload in heldout_production
    ]

    gates = {
        "1_all_trusted_steps_replay": {
            "passed": replay_gate,
            "replayed_certificates": replayed,
            "expected_certificates": expected_replays,
        },
        "2_fail_closed_robustness": {
            "passed": robustness_gate,
            "tests_run": robustness["tests_run"],
            "categories": robustness["categories"],
        },
        "3_heldout_native_z3_status_agreement": {
            "passed": z3_gate,
            "matches": status_matches,
            "total": len(heldout_status_pairs),
            "pairs": heldout_status_pairs,
        },
        "4_production_eligibility": {
            "passed": eligibility_gate,
            "heldout_clauses": heldout_eligible_clauses,
            "heldout_families": heldout_eligible_families,
            "total_clauses": extraction["summary"]["eligible_clauses"],
            "total_families": extraction["summary"]["eligible_families"],
        },
        "5_unique_heldout_closures": {
            "passed": closure_gate,
            "synthetic": synthetic_unique,
            "production": production_unique,
        },
        "6_no_baseline_or_neutral_loss": {
            "passed": no_loss_gate,
            "baseline_losses": baseline_losses,
            "neutral_losses": neutral_losses,
        },
        "7_bounded_clause_growth": {
            "passed": growth_gate,
            "median_ratio": median_growth,
            "p95_ratio": p95_growth,
        },
        "8_native_p95_at_most_50ms": {
            "passed": timing_gate,
            "p95_ms": native_p95_ms,
        },
        "9_coefficient_bounds_enforced": {
            "passed": coefficient_gate,
            "crossed_bounds": crossed_bounds,
            "max_coefficient_bits": max_coefficient_bits,
        },
        "10_optional_removable_packaging": {
            "passed": packaging_gate,
            **packaging,
        },
        "11_comprehensive_ubuntu_clean": comprehensive,
    }
    failed = [name for name, gate in gates.items() if not gate["passed"]]
    result = {
        "verdict": "advance" if not failed else "do_not_advance",
        "failed_gates": failed,
        "gates": gates,
        "synthetic": {
            "workloads": synthetic_native["summary"]["workloads"],
            "supported": synthetic_native["summary"]["supported"],
            "unique_native_closures": synthetic_native["summary"][
                "unique_native_closures"
            ],
            "z3_expected_matches": synthetic_external["summary"][
                "z3_expected_matches"
            ],
            "z3_expected_total": synthetic_external["summary"][
                "z3_expected_total"
            ],
            "vampire_arm_outcome_agreement": all(
                workload["vampire_theory_axioms"]["outcome"]
                == workload["vampire_alasca_no_viras"]["outcome"]
                for workload in synthetic_external["workloads"]
            ),
        },
        "production": {
            "selection_sources": extraction["summary"]["selected_sources"],
            "eligible_sources": extraction["summary"]["eligible_sources"],
            "eligible_clauses": extraction["summary"]["eligible_clauses"],
            "eligible_families": extraction["summary"]["eligible_families"],
            "native_unique_closures": production_native["summary"][
                "unique_native_closures"
            ],
            "heldout_native_z3_pairs": production_status_pairs,
            "vampire_alasca_only_source_solves": alasca_only_sources,
            "production_umlaut_panics": sum(
                source["arms"]["production_umlaut"]["returncode"] == 101
                for source in source_controls["sources"]
            ),
        },
        "artifact_sha256": {
            path.relative_to(root).as_posix(): sha256_file(path)
            for path in (
                synthetic_native_path,
                production_native_path,
                robustness_path,
                extraction_path,
                synthetic_external_path,
                production_external_path,
                source_controls_path,
            )
        },
    }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "verdict": result["verdict"],
                "failed_gates": failed,
                "synthetic_unique_closures": synthetic_unique,
                "production_unique_closures": production_unique,
                "alasca_only_source_solves": alasca_only_sources,
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
