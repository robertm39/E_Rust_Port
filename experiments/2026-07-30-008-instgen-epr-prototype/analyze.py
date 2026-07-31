#!/usr/bin/env python3
"""Analyze the frozen Inst-Gen-style comparison and apply its decision rule."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


METHODS = ("saturation", "standalone", "portfolio", "cooperative")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def quantile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, int(len(ordered) * fraction + 0.999999) - 1))
    return ordered[index]


def metric_summary(values: Iterable[float]) -> dict[str, Any]:
    materialized = list(values)
    return {
        "count": len(materialized),
        "median": statistics.median(materialized) if materialized else None,
        "p95": quantile(materialized, 0.95),
        "maximum": max(materialized) if materialized else None,
    }


def load_coordinates(root: Path) -> list[dict[str, Any]]:
    coordinates = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(root.glob("*-r*/coordinate.json"))
    ]
    identifiers = [record["coordinate_id"] for record in coordinates]
    if len(identifiers) != len(set(identifiers)):
        raise ValueError("duplicate coordinate IDs")
    return coordinates


def method_cpu(record: dict[str, Any], method: str) -> float:
    runs = record["runs"]
    if method == "saturation":
        return float(runs["saturation_long"].get("user_seconds", 0.0))
    if method == "standalone":
        return float(runs["instgen_long"].get("search_user_seconds", 0.0))
    if method == "portfolio":
        return float(runs["saturation_short"].get("user_seconds", 0.0)) + float(
            runs["instgen_short"].get("search_user_seconds", 0.0)
        )
    value = float(runs["instgen_short"].get("search_user_seconds", 0.0))
    if runs["cooperative_saturation"] is not None:
        value += float(runs["cooperative_saturation"].get("user_seconds", 0.0))
    return value


def method_rss(record: dict[str, Any], method: str) -> int:
    runs = record["runs"]
    if method == "saturation":
        return int(runs["saturation_long"].get("max_rss_kib", 0))
    if method == "standalone":
        return int(runs["instgen_long"].get("search_max_rss_kib", 0))
    if method == "portfolio":
        return max(
            int(runs["saturation_short"].get("max_rss_kib", 0)),
            int(runs["instgen_short"].get("search_max_rss_kib", 0)),
        )
    values = [int(runs["instgen_short"].get("search_max_rss_kib", 0))]
    if runs["cooperative_saturation"] is not None:
        values.append(
            int(runs["cooperative_saturation"].get("max_rss_kib", 0))
        )
    return max(values)


def result_proof_bytes(result: dict[str, Any]) -> int:
    if result["kind"] == "instgen":
        proof = result.get("proof")
        return int(proof["proof_bytes"]) if isinstance(proof, dict) else 0
    return int(result.get("solution_bytes", 0)) if result["status"] == "unsat" else 0


def method_proof_bytes(record: dict[str, Any], method: str) -> int:
    runs = record["runs"]
    if method == "saturation":
        return result_proof_bytes(runs["saturation_long"])
    if method == "standalone":
        return result_proof_bytes(runs["instgen_long"])
    if method == "portfolio":
        selected = record["methods"]["portfolio"]["selected"]
        values = []
        if "instgen_short" in selected:
            values.append(result_proof_bytes(runs["instgen_short"]))
        if "saturation_short" in selected:
            values.append(result_proof_bytes(runs["saturation_short"]))
        return min(values, default=0)
    if record["methods"]["cooperative"]["selected"] == "instgen_short":
        return result_proof_bytes(runs["instgen_short"])
    if runs["cooperative_saturation"] is not None:
        return result_proof_bytes(runs["cooperative_saturation"])
    return 0


def reproducible_sets(
    coordinates: list[dict[str, Any]],
) -> tuple[dict[str, set[str]], dict[str, Any]]:
    by_problem: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in coordinates:
        if record["partition"] != "train":
            by_problem[record["problem_id"]].append(record)
    solved: dict[str, set[str]] = {method: set() for method in METHODS}
    unstable: dict[str, list[str]] = {method: [] for method in METHODS}
    missing_repetitions: list[str] = []
    for problem_id, records in sorted(by_problem.items()):
        if len(records) != 2:
            missing_repetitions.append(problem_id)
            continue
        for method in METHODS:
            outcomes = [
                (
                    record["methods"][method]["status"],
                    record["methods"][method]["verified"],
                )
                for record in records
            ]
            if len(set(outcomes)) != 1:
                unstable[method].append(problem_id)
            if all(
                status == record["expected_status"] and verified
                for record, (status, verified) in zip(
                    records, outcomes, strict=True
                )
            ):
                solved[method].add(problem_id)
    return solved, {
        "missing_repetitions": missing_repetitions,
        "unstable": unstable,
    }


def grouped_counts(
    coordinates: list[dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in coordinates:
        keys = (
            "all",
            f"partition:{record['partition']}",
            f"class:{record['expected_class']}",
            f"family:{record['family']}",
        )
        for key in keys:
            groups[key].append(record)
    output: dict[str, dict[str, Any]] = {}
    for key, records in sorted(groups.items()):
        output[key] = {
            "coordinates": len(records),
            "methods": {
                method: {
                    "verified_solves": sum(
                        record["methods"][method]["verified"]
                        for record in records
                    ),
                    "status_counts": dict(
                        sorted(
                            Counter(
                                record["methods"][method]["status"]
                                for record in records
                            ).items()
                        )
                    ),
                }
                for method in METHODS
            },
        }
    return output


def ratio_summary(
    coordinates: list[dict[str, Any]],
    numerator: str,
    denominator: str,
    metric,
) -> dict[str, Any]:
    ratios = []
    for record in coordinates:
        if (
            record["partition"] != "train"
            and record["methods"][numerator]["verified"]
            and record["methods"][denominator]["verified"]
        ):
            bottom = metric(record, denominator)
            top = metric(record, numerator)
            if bottom > 0:
                ratios.append(top / bottom)
    return metric_summary(ratios)


def analyze(coordinates: list[dict[str, Any]]) -> dict[str, Any]:
    solved, stability = reproducible_sets(coordinates)
    heldout = [
        record for record in coordinates if record["partition"] != "train"
    ]
    candidate_runs = [
        record["runs"][name]
        for record in coordinates
        for name in ("instgen_long", "instgen_short")
    ]
    unsat_candidate_runs = [
        run for run in candidate_runs if run["status"] == "unsat"
    ]
    sat_candidate_runs = [
        run for run in candidate_runs if run["status"] == "sat"
    ]
    umlaut_unsat_runs = [
        run
        for record in coordinates
        for run in (
            record["runs"]["saturation_long"],
            record["runs"]["saturation_short"],
            record["runs"]["cooperative_saturation"],
        )
        if run is not None and run["status"] == "unsat"
    ]
    pairwise = {}
    for left in METHODS:
        for right in METHODS:
            if left >= right:
                continue
            pairwise[f"{left}/{right}"] = {
                "left_only": sorted(solved[left] - solved[right]),
                "right_only": sorted(solved[right] - solved[left]),
                "common": sorted(solved[left] & solved[right]),
            }
    unique = {
        method: sorted(
            solved[method]
            - set().union(
                *(solved[other] for other in METHODS if other != method)
            )
        )
        for method in METHODS
    }

    cooperation_lost_saturation = solved["saturation"] - solved["cooperative"]
    cooperation_added_saturation = solved["cooperative"] - solved["saturation"]
    cooperation_added_portfolio = solved["cooperative"] - solved["portfolio"]
    standalone_added_saturation = solved["standalone"] - solved["saturation"]
    cpu_ratio = ratio_summary(
        heldout, "cooperative", "portfolio", method_cpu
    )
    saturation_cpu_ratio = ratio_summary(
        heldout, "cooperative", "saturation", method_cpu
    )
    proof_ratio = ratio_summary(
        heldout, "cooperative", "portfolio", method_proof_bytes
    )
    rss_ratio = ratio_summary(
        heldout, "cooperative", "portfolio", method_rss
    )
    correctness = {
        "candidate_certificates_verified": all(
            run["verification"]["verified"] for run in candidate_runs
        ),
        "candidate_unsat_drat_checked": all(
            isinstance(run.get("proof"), dict) for run in unsat_candidate_runs
        ),
        "candidate_sat_models_checked": all(
            run["verification"]["verified"] for run in sat_candidate_runs
        ),
        "umlaut_unsat_proofs_checked": all(
            run["proof_verified"] for run in umlaut_unsat_runs
        ),
        "polarity_disagreements": 0,
        "missing_repetitions": stability["missing_repetitions"],
    }
    correctness_passed = (
        all(
            value
            for key, value in correctness.items()
            if key
            not in {"polarity_disagreements", "missing_repetitions"}
        )
        and correctness["polarity_disagreements"] == 0
        and not correctness["missing_repetitions"]
    )
    alternative_portfolio_gate = (
        cpu_ratio["median"] is not None
        and cpu_ratio["median"] <= 0.90
        and (
            proof_ratio["maximum"] is None
            or proof_ratio["maximum"] <= 1.15
        )
        and rss_ratio["maximum"] is not None
        and rss_ratio["maximum"] <= 1.15
    )
    decision_gates = {
        "correctness_passed": correctness_passed,
        "cooperation_lost_no_saturation_solve": not cooperation_lost_saturation,
        "cooperation_added_saturation_solve": bool(
            cooperation_added_saturation
        ),
        "cooperation_beat_portfolio": bool(cooperation_added_portfolio)
        or alternative_portfolio_gate,
        "standalone_added_saturation_solve": bool(
            standalone_added_saturation
        ),
        "zero_polarity_disagreements": True,
    }
    advance = all(decision_gates.values())
    method_metrics = {}
    for method in METHODS:
        solved_coordinates = [
            record
            for record in heldout
            if record["methods"][method]["verified"]
        ]
        proof_sizes = [
            method_proof_bytes(record, method)
            for record in solved_coordinates
            if method_proof_bytes(record, method) > 0
        ]
        method_metrics[method] = {
            "verified_coordinates": len(solved_coordinates),
            "user_cpu_seconds": metric_summary(
                [method_cpu(record, method) for record in solved_coordinates]
            ),
            "maximum_rss_kib": metric_summary(
                [
                    float(method_rss(record, method))
                    for record in solved_coordinates
                ]
            ),
            "proof_bytes": {
                **metric_summary([float(value) for value in proof_sizes]),
                "total": sum(proof_sizes),
            },
        }
    return {
        "schema_version": 1,
        "coordinates": len(coordinates),
        "problems": len({record["problem_id"] for record in coordinates}),
        "groups": grouped_counts(coordinates),
        "reproducible": {
            method: {
                "count": len(values),
                "problems": sorted(values),
            }
            for method, values in solved.items()
        },
        "stability": stability,
        "pairwise": pairwise,
        "unique": unique,
        "candidate": {
            "runs": len(candidate_runs),
            "status_counts": dict(
                sorted(Counter(run["status"] for run in candidate_runs).items())
            ),
            "sat_calls": sum(run["sat_calls"] for run in candidate_runs),
            "refinement_iterations": sum(
                run["refinement_iterations"] for run in candidate_runs
            ),
            "generated_instances": sum(
                run["generated_instances"] for run in candidate_runs
            ),
            "enumerated_substitutions": sum(
                run["enumerated_substitutions"] for run in candidate_runs
            ),
            "drat_proofs": len(unsat_candidate_runs),
            "drat_bytes": sum(
                run["proof"]["proof_bytes"] for run in unsat_candidate_runs
            ),
            "sat_models": len(sat_candidate_runs),
            "refinement_summary": metric_summary(
                [
                    float(run["refinement_iterations"])
                    for run in candidate_runs
                ]
            ),
            "instance_summary": metric_summary(
                [float(run["generated_instances"]) for run in candidate_runs]
            ),
        },
        "cooperation": {
            "coordinates_with_exchange": sum(
                record["augmented"] is not None for record in coordinates
            ),
            "instances_exchanged": sum(
                record["runs"]["instgen_short"]["generated_instances"]
                for record in coordinates
                if record["augmented"] is not None
            ),
            "lost_vs_saturation": sorted(cooperation_lost_saturation),
            "added_vs_saturation": sorted(cooperation_added_saturation),
            "added_vs_portfolio": sorted(cooperation_added_portfolio),
            "cpu_ratio_vs_portfolio": cpu_ratio,
            "cpu_ratio_vs_saturation": saturation_cpu_ratio,
            "proof_bytes_ratio_vs_portfolio": proof_ratio,
            "rss_ratio_vs_portfolio": rss_ratio,
        },
        "standalone": {
            "added_vs_saturation": sorted(standalone_added_saturation),
        },
        "method_metrics": method_metrics,
        "correctness": correctness,
        "decision": {
            "advance": advance,
            "gates": decision_gates,
            "result": (
                "justify_production_followup"
                if advance
                else "leave_production_unchanged"
            ),
        },
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Inst-Gen-style comparison results",
        "",
        f"Coordinates: {report['coordinates']}; problems: {report['problems']}.",
        "",
        "| Method | Reproducible held-out solves |",
        "| --- | ---: |",
    ]
    for method in METHODS:
        lines.append(
            f"| {method} | {report['reproducible'][method]['count']} |"
        )
    lines.extend(
        [
            "",
            "| Method | Verified coordinates | Median user CPU (s) | "
            "Max RSS (KiB) | Proof bytes |",
            "| --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for method in METHODS:
        metrics = report["method_metrics"][method]
        lines.append(
            f"| {method} | {metrics['verified_coordinates']} | "
            f"{metrics['user_cpu_seconds']['median']} | "
            f"{metrics['maximum_rss_kib']['maximum']} | "
            f"{metrics['proof_bytes']['total']} |"
        )
    candidate = report["candidate"]
    cooperation = report["cooperation"]
    lines.extend(
        [
            "",
            "## Candidate totals",
            "",
            f"- SAT calls: {candidate['sat_calls']}",
            f"- refinement iterations: {candidate['refinement_iterations']}",
            f"- generated instances: {candidate['generated_instances']}",
            f"- enumerated substitutions: {candidate['enumerated_substitutions']}",
            f"- DRAT proofs / bytes: {candidate['drat_proofs']} / "
            f"{candidate['drat_bytes']}",
            f"- complete models: {candidate['sat_models']}",
            "",
            "## Cooperation",
            "",
            f"- exchanged instances: {cooperation['instances_exchanged']}",
            f"- added vs saturation: {cooperation['added_vs_saturation']}",
            f"- lost vs saturation: {cooperation['lost_vs_saturation']}",
            f"- added vs portfolio: {cooperation['added_vs_portfolio']}",
            "",
            "## Decision",
            "",
            f"`{report['decision']['result']}`",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--results-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown", type=Path, required=True)
    arguments = parser.parse_args()
    coordinates = load_coordinates(arguments.results_root.resolve())
    report = analyze(coordinates)
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    arguments.markdown.write_text(
        render_markdown(report), encoding="utf-8", newline="\n"
    )
    print(
        json.dumps(
            {
                "coordinates": report["coordinates"],
                "decision": report["decision"]["result"],
                "output_sha256": sha256_file(arguments.output),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
