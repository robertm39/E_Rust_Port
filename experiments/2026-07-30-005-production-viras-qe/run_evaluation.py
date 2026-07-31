#!/usr/bin/env python3
"""Run the preregistered production VIRAS QE held-out evaluation."""

from __future__ import annotations

import argparse
import concurrent.futures
import copy
import hashlib
import json
import math
import random
import re
import statistics
import subprocess
import tempfile
import time
import tomllib
from collections import Counter
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path
from typing import Any, Iterable


SEED = 0x51A52026
CASES_PER_FAMILY = 20
EXPECTED_SCHEDULE_SHA256 = (
    "491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068"
)
SZS_RE = re.compile(r"% SZS status ([A-Za-z]+)")
TPTP_STATUS_RE = re.compile(r"(?m)^%\s*Status\s*:\s*([A-Za-z]+)")
NODE_TAGS = {
    "bool",
    "atom",
    "and",
    "or",
    "exists",
    "forall",
    "eq",
    "ne",
    "gt",
    "ge",
    "const",
    "var",
    "add",
    "scale",
    "floor",
}


@dataclass(frozen=True)
class AnalyticCase:
    case_id: str
    family: str
    source: str
    expected: bool


@dataclass
class Invocation:
    returncode: int
    stdout: str
    stderr: str
    elapsed_ms: float


class ValidationError(RuntimeError):
    """Raised when a production record violates the frozen validation rules."""


def run_process(
    command: list[str],
    *,
    source: str | None = None,
    timeout: float = 10.0,
) -> Invocation:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        input=source,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    return Invocation(
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
        elapsed_ms=elapsed_ms,
    )


def run_viras(
    binary: Path,
    source: str,
    *options: str,
) -> Invocation:
    return run_process(
        [str(binary), "--json", *options, "-"],
        source=source,
        timeout=30.0,
    )


def parse_fraction(text: str) -> Fraction:
    return Fraction(text)


def evaluate_term(term: list[Any]) -> Fraction:
    tag = term[0]
    if tag == "const":
        return parse_fraction(term[1])
    if tag == "add":
        return sum((evaluate_term(child) for child in term[1:]), Fraction())
    if tag == "scale":
        return parse_fraction(term[1]) * evaluate_term(term[2])
    if tag == "floor":
        value = evaluate_term(term[1])
        return Fraction(value.numerator // value.denominator)
    if tag == "var":
        raise ValidationError(f"result contains free variable {term[1]}")
    raise ValidationError(f"unknown term tag {tag!r}")


def evaluate_formula(formula: list[Any]) -> bool:
    tag = formula[0]
    if tag == "bool":
        return bool(formula[1])
    if tag == "atom":
        relation, term = formula[1]
        value = evaluate_term(term)
        return {
            "eq": value == 0,
            "ne": value != 0,
            "gt": value > 0,
            "ge": value >= 0,
        }[relation]
    if tag == "and":
        return all(evaluate_formula(child) for child in formula[1:])
    if tag == "or":
        return any(evaluate_formula(child) for child in formula[1:])
    if tag in {"exists", "forall"}:
        raise ValidationError(f"result retains quantifier {tag}")
    raise ValidationError(f"unknown formula tag {tag!r}")


def ast_nodes(value: Any) -> int:
    if not isinstance(value, list):
        return 0
    own = int(bool(value) and isinstance(value[0], str) and value[0] in NODE_TAGS)
    return own + sum(ast_nodes(child) for child in value[1:])


def contains_tag(value: Any, forbidden: set[str]) -> bool:
    if not isinstance(value, list):
        return False
    if value and value[0] in forbidden:
        return True
    return any(contains_tag(child, forbidden) for child in value[1:])


def validate_success(
    record: dict[str, Any],
    expected: bool,
    *,
    reference: dict[str, Any] | None = None,
) -> bool:
    if record.get("schema") != "umlaut-viras-qe-v1":
        raise ValidationError("wrong output schema")
    if record.get("status") != "success":
        raise ValidationError(f"non-success record: {record.get('status')}")
    derivation = record.get("derivation")
    if not isinstance(derivation, dict) or derivation.get("replay_validated") is not True:
        raise ValidationError("successful record lacks positive derivation replay")
    formula = record.get("result_formula")
    if not isinstance(formula, list):
        raise ValidationError("successful record has no result formula")
    if contains_tag(formula, {"exists", "forall", "var"}):
        raise ValidationError("successful result is not closed and quantifier-free")
    observed = evaluate_formula(formula)
    if observed != expected:
        raise ValidationError(f"exact result {observed} differs from expected {expected}")
    transformed = record.get("transformed_tff")
    if not isinstance(transformed, str) or not transformed.startswith("tff("):
        raise ValidationError("successful record has no TFF re-embedding")
    if formula[0] == "bool":
        required = "$true" if formula[1] else "$false"
        if required not in transformed:
            raise ValidationError("TFF re-embedding disagrees with Boolean result")
    if reference is not None and record != reference:
        raise ValidationError("record differs from byte-replayed canonical derivation")
    return observed


def real_literal(value: Fraction) -> str:
    sign = "-" if value < 0 else ""
    numerator = abs(value.numerator)
    denominator = value.denominator
    if denominator not in {1, 2, 4}:
        raise ValueError(f"nonterminating held-out literal {value}")
    whole, remainder = divmod(numerator, denominator)
    if denominator == 1:
        return f"{sign}{whole}.0"
    digits = {2: {0: "0", 1: "5"}, 4: {0: "00", 1: "25", 2: "50", 3: "75"}}
    return f"{sign}{whole}.{digits[denominator][remainder]}"


def document(case_id: str, body: str) -> str:
    return f"tff({case_id},conjecture,{body}).\n"


def analytic_cases(seed: int, cases_per_family: int) -> list[AnalyticCase]:
    rng = random.Random(seed)
    cases: list[AnalyticCase] = []
    for index in range(cases_per_family):
        lower = rng.randint(-6, 6)
        upper = lower + (-2, -1, 0, 1, 2)[index % 5]
        case_id = f"integer_interval_{index:02d}"
        body = (
            f"? [I:$int] : ($greatereq(I,{lower}) & $lesseq(I,{upper}))"
        )
        cases.append(
            AnalyticCase(case_id, "integer_interval", document(case_id, body), lower <= upper)
        )

        integer_part = rng.randint(-5, 5)
        offset = (Fraction(-1, 2), Fraction(0), Fraction(1, 2), Fraction(1), Fraction(3, 2))[
            index % 5
        ]
        threshold = Fraction(integer_part) + offset
        case_id = f"real_floor_band_{index:02d}"
        body = (
            "? [R:$real] : "
            f"(($floor(R) = {real_literal(Fraction(integer_part))}) & "
            f"$greater(R,{real_literal(threshold)}))"
        )
        cases.append(
            AnalyticCase(
                case_id,
                "real_floor_band",
                document(case_id, body),
                threshold < integer_part + 1,
            )
        )

        floor_value = rng.randint(-6, 6)
        cell_lower = Fraction(floor_value, 2)
        cell_upper = Fraction(floor_value + 1, 2)
        mode = index % 4
        if mode == 0:
            lower_real, upper_real = cell_lower, cell_upper
        elif mode == 1:
            lower_real, upper_real = cell_upper, cell_upper + 1
        elif mode == 2:
            lower_real, upper_real = cell_lower - 1, cell_lower
        else:
            lower_real, upper_real = cell_lower + 1, cell_upper + 2
        intersection_lower = max(cell_lower, lower_real)
        expected = (
            intersection_lower <= upper_real and intersection_lower < cell_upper
        )
        case_id = f"scaled_floor_interval_{index:02d}"
        body = (
            "? [R:$real] : "
            f"(($floor($product(2.0,R)) = {real_literal(Fraction(floor_value))}) & "
            f"($greatereq(R,{real_literal(lower_real)}) & "
            f"$lesseq(R,{real_literal(upper_real)})))"
        )
        cases.append(
            AnalyticCase(
                case_id,
                "scaled_floor_interval",
                document(case_id, body),
                expected,
            )
        )

        first = Fraction(rng.randint(-16, 16), 4)
        second = first + (Fraction(-1), Fraction(0), Fraction(1), Fraction(2))[
            index % 4
        ]
        case_id = f"universal_gap_{index:02d}"
        body = (
            "! [R:$real] : "
            f"($less(R,{real_literal(first)}) | "
            f"$greatereq(R,{real_literal(second)}))"
        )
        cases.append(
            AnalyticCase(
                case_id,
                "universal_gap",
                document(case_id, body),
                second <= first,
            )
        )

        left = Fraction(rng.randint(-16, 16), 4)
        right = Fraction(rng.randint(-16, 16), 4)
        maximum = max(left, right)
        threshold = maximum + (Fraction(-1, 2), Fraction(0), Fraction(1, 2), Fraction(1))[
            index % 4
        ]
        case_id = f"boolean_points_{index:02d}"
        body = (
            "? [R:$real] : "
            f"(((R = {real_literal(left)}) | (R = {real_literal(right)})) & "
            f"$greater(R,{real_literal(threshold)}))"
        )
        cases.append(
            AnalyticCase(
                case_id,
                "boolean_points",
                document(case_id, body),
                left > threshold or right > threshold,
            )
        )

        affine = Fraction(rng.randint(-12, 12), 4)
        bound = affine + (Fraction(-1), Fraction(0), Fraction(1), Fraction(2))[
            index % 4
        ]
        case_id = f"nested_affine_{index:02d}"
        body = (
            "! [R:$real] : ? [S:$real] : "
            f"((S = $sum(R,{real_literal(affine)})) & "
            f"$greatereq(S,$sum(R,{real_literal(bound)})))"
        )
        cases.append(
            AnalyticCase(
                case_id,
                "nested_affine",
                document(case_id, body),
                affine >= bound,
            )
        )
    return cases


def percentile(values: Iterable[float], percent: float) -> float:
    ordered = sorted(values)
    if not ordered:
        return 0.0
    index = max(0, math.ceil(percent * len(ordered)) - 1)
    return ordered[index]


def distribution(values: list[float]) -> dict[str, float]:
    if not values:
        return {"count": 0, "median": 0.0, "p95": 0.0, "max": 0.0}
    return {
        "count": len(values),
        "median": statistics.median(values),
        "p95": percentile(values, 0.95),
        "max": max(values),
    }


def expected_from_tptp(source: str) -> bool | None:
    match = TPTP_STATUS_RE.search(source)
    if match is None:
        return None
    status = match.group(1)
    if status in {"Theorem", "Satisfiable"}:
        return True
    if status in {"CounterSatisfiable", "Unsatisfiable"}:
        return False
    return None


def scan_tfi(binary: Path, corpus: Path) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    outcomes: Counter[str] = Counter()
    expected_checks = 0
    for path in sorted(corpus.glob("*.p")):
        source = path.read_text(encoding="utf-8", errors="replace")
        invocation = run_viras(binary, source)
        row: dict[str, Any] = {
            "file": path.name,
            "returncode": invocation.returncode,
            "elapsed_ms": invocation.elapsed_ms,
        }
        if invocation.stdout.startswith("{"):
            record = json.loads(invocation.stdout)
            outcome = record["status"]
            row["outcome"] = outcome
            row["unknown_kind"] = record.get("unknown_kind")
            if outcome == "success":
                expected = expected_from_tptp(source)
                observed = evaluate_formula(record["result_formula"])
                validate_success(record, observed if expected is None else expected)
                row["expected"] = expected
                row["observed_result"] = observed
                if expected is not None:
                    expected_checks += 1
        else:
            code = invocation.stderr.split(":", 1)[0].strip() or "NO_DIAGNOSTIC"
            outcome = code
            row["outcome"] = "rejected"
            row["rejection_code"] = code
        outcomes[outcome] += 1
        rows.append(row)
    return {
        "documents": len(rows),
        "outcomes": dict(sorted(outcomes.items())),
        "successful_record_validations": outcomes["success"],
        "expected_status_validations": expected_checks,
        "latency_ms": distribution([row["elapsed_ms"] for row in rows]),
        "rows": rows,
    }


def default_solve(binary: Path, path: Path) -> dict[str, Any]:
    try:
        invocation = run_process(
            [
                str(binary),
                "--auto",
                "--cpu-limit=1",
                "--memory-limit=2048",
                str(path),
            ],
            timeout=4.0,
        )
    except subprocess.TimeoutExpired:
        return {"status": "ControllerTimeout", "returncode": None}
    combined = f"{invocation.stdout}\n{invocation.stderr}"
    match = SZS_RE.search(combined)
    return {
        "status": match.group(1) if match else "NoSZS",
        "returncode": invocation.returncode,
    }


def status_is_correct(status: str, expected: bool) -> bool:
    return status == ("Theorem" if expected else "CounterSatisfiable")


def analytic_evaluation(
    viras_binary: Path,
    umlaut_binary: Path,
    cases: list[AnalyticCase],
) -> tuple[dict[str, Any], list[tuple[AnalyticCase, dict[str, Any], str]]]:
    validated: list[tuple[AnalyticCase, dict[str, Any], str]] = []
    rows: list[dict[str, Any]] = []
    true_count = 0
    false_count = 0
    with tempfile.TemporaryDirectory(prefix="umlaut-viras-heldout-") as temporary:
        temporary_root = Path(temporary)
        problem_paths: dict[str, Path] = {}
        for case in cases:
            path = temporary_root / f"{case.case_id}.p"
            path.write_text(case.source, encoding="utf-8")
            problem_paths[case.case_id] = path

        for case in cases:
            invocation = run_viras(viras_binary, case.source)
            if not invocation.stdout.startswith("{"):
                raise ValidationError(
                    f"{case.case_id} emitted no JSON: {invocation.stderr}"
                )
            record = json.loads(invocation.stdout)
            validate_success(record, case.expected)
            if invocation.returncode != 0:
                raise ValidationError(
                    f"{case.case_id} succeeded with exit {invocation.returncode}"
                )
            imported_nodes = ast_nodes(record["imported_formula"])
            result_nodes = ast_nodes(record["result_formula"])
            if imported_nodes == 0:
                raise ValidationError(f"{case.case_id} has empty imported AST")
            eliminations = record["derivation"]["eliminations"]
            candidates = sum(len(item["candidates"]) for item in eliminations)
            grids = sum(len(item["grid_flattening"]) for item in eliminations)
            row = {
                "case_id": case.case_id,
                "family": case.family,
                "expected": case.expected,
                "elapsed_ms": invocation.elapsed_ms,
                "imported_nodes": imported_nodes,
                "result_nodes": result_nodes,
                "growth_ratio": result_nodes / imported_nodes,
                "candidates": candidates,
                "grid_records": grids,
            }
            rows.append(row)
            validated.append((case, record, invocation.stdout))
            true_count += int(case.expected)
            false_count += int(not case.expected)

        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
            futures = {
                executor.submit(
                    default_solve,
                    umlaut_binary,
                    problem_paths[case.case_id],
                ): case
                for case in cases
            }
            default_results = {
                futures[future].case_id: future.result()
                for future in concurrent.futures.as_completed(futures)
            }

    default_statuses: Counter[str] = Counter()
    overlap = 0
    qe_only = 0
    default_only = 0
    for row in rows:
        result = default_results[row["case_id"]]
        row["default_status"] = result["status"]
        row["default_returncode"] = result["returncode"]
        correct = status_is_correct(result["status"], row["expected"])
        row["default_correct_solve"] = correct
        default_statuses[result["status"]] += 1
        if correct:
            overlap += 1
        else:
            qe_only += 1
    return (
        {
            "cases": len(rows),
            "true": true_count,
            "false": false_count,
            "families": dict(sorted(Counter(row["family"] for row in rows).items())),
            "latency_ms": distribution([row["elapsed_ms"] for row in rows]),
            "imported_nodes": distribution(
                [float(row["imported_nodes"]) for row in rows]
            ),
            "result_nodes": distribution([float(row["result_nodes"]) for row in rows]),
            "growth_ratio": distribution([row["growth_ratio"] for row in rows]),
            "max_candidates": max(row["candidates"] for row in rows),
            "max_grid_records": max(row["grid_records"] for row in rows),
            "default_statuses": dict(sorted(default_statuses.items())),
            "complementarity": {
                "both_correct": overlap,
                "qe_only_correct": qe_only,
                "default_only_correct": default_only,
                "qe_correct": len(rows),
                "default_correct": overlap,
            },
            "rows": rows,
        },
        validated,
    )


def determinism_checks(
    binary: Path,
    validated: list[tuple[AnalyticCase, dict[str, Any], str]],
) -> dict[str, Any]:
    selected: list[tuple[AnalyticCase, dict[str, Any], str]] = []
    counts: Counter[str] = Counter()
    for item in validated:
        family = item[0].family
        if counts[family] < 2:
            selected.append(item)
            counts[family] += 1
    digests: dict[str, str] = {}
    for case, _record, canonical in selected:
        repeated = run_viras(binary, case.source)
        if repeated.stdout != canonical:
            raise ValidationError(f"{case.case_id} output is not byte-deterministic")
        digests[case.case_id] = hashlib.sha256(canonical.encode()).hexdigest()
    return {"cases": len(selected), "sha256": digests}


def corruption_checks(
    validated: list[tuple[AnalyticCase, dict[str, Any], str]],
) -> dict[str, str]:
    case, authentic, _canonical = next(
        item
        for item in validated
        if any(
            elimination["candidates"]
            for elimination in item[1]["derivation"]["eliminations"]
        )
    )
    results: dict[str, str] = {}

    corrupted = copy.deepcopy(authentic)
    corrupted["result_formula"] = ["bool", not case.expected]
    try:
        validate_success(corrupted, case.expected)
    except ValidationError as error:
        results["result_flip"] = str(error)
    else:
        raise ValidationError("result-flip corruption was accepted")

    corrupted = copy.deepcopy(authentic)
    corrupted["transformed_tff"] = (
        "tff(corrupt,conjecture,$false).\n"
        if case.expected
        else "tff(corrupt,conjecture,$true).\n"
    )
    try:
        validate_success(corrupted, case.expected)
    except ValidationError as error:
        results["tff_flip"] = str(error)
    else:
        raise ValidationError("TFF-flip corruption was accepted")

    corrupted = copy.deepcopy(authentic)
    corrupted["derivation"]["replay_validated"] = False
    try:
        validate_success(corrupted, case.expected)
    except ValidationError as error:
        results["replay_flag"] = str(error)
    else:
        raise ValidationError("replay-flag corruption was accepted")

    corrupted = copy.deepcopy(authentic)
    for elimination in corrupted["derivation"]["eliminations"]:
        if elimination["candidates"]:
            elimination["candidates"].pop()
            break
    try:
        validate_success(corrupted, case.expected, reference=authentic)
    except ValidationError as error:
        results["candidate_deletion"] = str(error)
    else:
        raise ValidationError("candidate-deletion corruption was accepted")
    return results


def resource_checks(binary: Path) -> dict[str, dict[str, Any]]:
    periodic = document(
        "resource_periodic",
        "? [R:$real] : "
        "(($floor($product(2.0,R)) = 0.0) & "
        "($greatereq(R,0.0) & $lesseq(R,1.0)))",
    )
    simple = document("resource_simple", "? [R:$real] : (R = 0.0)")
    cases = {
        "steps": (periodic, "--max-steps=0"),
        "candidates": (periodic, "--max-candidates=0"),
        "grids": (periodic, "--max-grids=0"),
        "grid_points": (periodic, "--max-grid-points=0"),
        "dnf_branches": (simple, "--max-dnf-branches=0"),
        "formula_nodes": (simple, "--max-formula-nodes=0"),
        "rational_bits": (
            document("resource_rational", "256 = 256"),
            "--max-rational-bits=8",
        ),
    }
    results: dict[str, dict[str, Any]] = {}
    for name, (source, option) in cases.items():
        invocation = run_viras(binary, source, option)
        if not invocation.stdout.startswith("{"):
            raise ValidationError(f"{name} resource check emitted no JSON")
        record = json.loads(invocation.stdout)
        if (
            invocation.returncode != 2
            or record.get("status") != "unknown"
            or record.get("unknown_kind") != "ResourceLimit"
            or record.get("result_formula") is not None
        ):
            raise ValidationError(f"{name} did not fail closed: {record}")
        results[name] = {
            "returncode": invocation.returncode,
            "reason": record.get("reason"),
        }
    return results


def repository_boundary(source_root: Path) -> dict[str, Any]:
    schedule = source_root / "src" / "heuristics" / "schedule.vars"
    schedule_sha256 = hashlib.sha256(schedule.read_bytes()).hexdigest()
    if schedule_sha256 != EXPECTED_SCHEDULE_SHA256:
        raise ValidationError("automatic schedule input changed")
    with (source_root / "Cargo.toml").open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)
    default_features = manifest["features"]["default"]
    if default_features:
        raise ValidationError(f"default features are not empty: {default_features}")
    viras_bins = [
        binary
        for binary in manifest["bin"]
        if binary.get("name") == "umlaut-viras-qe"
    ]
    if len(viras_bins) != 1 or viras_bins[0].get("required-features") != ["viras-qe"]:
        raise ValidationError("VIRAS binary is not feature-required")
    return {
        "schedule_sha256": schedule_sha256,
        "default_features": default_features,
        "viras_required_features": viras_bins[0]["required-features"],
    }


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--viras-binary", type=Path, required=True)
    parser.add_argument("--umlaut-binary", type=Path, required=True)
    parser.add_argument("--tfi-corpus", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=SEED)
    parser.add_argument("--cases-per-family", type=int, default=CASES_PER_FAMILY)
    return parser.parse_args()


def main() -> int:
    args = parse_arguments()
    for binary in (args.viras_binary, args.umlaut_binary):
        if not binary.is_file():
            raise SystemExit(f"missing executable: {binary}")
    source_root = Path(__file__).resolve().parents[2]
    cases = analytic_cases(args.seed, args.cases_per_family)
    analytic, validated = analytic_evaluation(
        args.viras_binary,
        args.umlaut_binary,
        cases,
    )
    report = {
        "schema": "umlaut-production-viras-evaluation-v1",
        "seed": args.seed,
        "cases_per_family": args.cases_per_family,
        "repository_boundary": repository_boundary(source_root),
        "analytic": analytic,
        "determinism": determinism_checks(args.viras_binary, validated),
        "corruptions": corruption_checks(validated),
        "resource_limits": resource_checks(args.viras_binary),
        "casc_2025_tfi": scan_tfi(args.viras_binary, args.tfi_corpus),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({key: value for key, value in report.items() if key != "analytic"}, indent=2))
    print(f"report={args.output}")
    print(f"sha256={hashlib.sha256(args.output.read_bytes()).hexdigest()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
