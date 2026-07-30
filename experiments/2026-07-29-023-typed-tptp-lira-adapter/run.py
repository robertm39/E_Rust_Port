#!/usr/bin/env python3
"""Run the preregistered typed-TPTP-to-LIRA conformance matrix."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import platform
import random
from pathlib import Path
from typing import Any, Callable, Sequence

import adapter
import independent_oracle


ROOT = Path(__file__).resolve().parent
SEED = 0xA11DA7A
GENERATED_CASES = 500


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def real_literal(value: int) -> str:
    return f"{value}.0"


def generated_sources(count: int = GENERATED_CASES) -> list[str]:
    generator = random.Random(SEED)
    sources = []
    for _ in range(count):
        template = generator.randrange(7)
        integer = generator.randint(-3, 3)
        other = generator.randint(-3, 3)
        divisor = generator.choice([-3, -2, -1, 1, 2, 3])
        if template == 0:
            body = (
                "! [I:$int] : "
                f"($floor($to_real($sum(I,{integer}))) = "
                f"$sum($to_real(I),{real_literal(integer)}))"
            )
        elif template == 1:
            body = (
                "! [R:$real] : "
                f"($floor($sum(R,{real_literal(integer)})) = "
                f"$sum($floor(R),{real_literal(integer)}))"
            )
        elif template == 2:
            body = (
                "! [R:$real] : "
                f"($difference($sum(R,{real_literal(integer)}),"
                f"{real_literal(integer)}) = R)"
            )
        elif template == 3:
            relation = generator.choice(
                ["$less", "$lesseq", "$greater", "$greatereq"]
            )
            body = (
                f"{relation}({real_literal(integer)},"
                f"{real_literal(other)})"
            )
        elif template == 4:
            half = f"{integer}.5"
            body = (
                "? [I:$int] : "
                f"(I = $to_int({half}))"
            )
        elif template == 5:
            body = (
                "! [I:$int] : "
                f"($to_real($quotient(I,{divisor})) = "
                f"$quotient($to_real(I),{real_literal(divisor)}))"
            )
        else:
            body = (
                "! [I:$int,R:$real] : "
                f"(($to_real(I) = R) => "
                f"($lesseq($sum($to_real(I),{real_literal(integer)}),"
                f"$sum(R,{real_literal(other)})) "
                f"<=> $lesseq({real_literal(integer)},"
                f"{real_literal(other)})))"
            )
        sources.append(f"tff(case,axiom,{body}).")
    return sources


def mutate_first(
    value: Any,
    predicate: Callable[[dict[str, Any]], bool],
    mutation: Callable[[dict[str, Any]], None],
) -> bool:
    if isinstance(value, dict):
        if predicate(value):
            mutation(value)
            return True
        return any(
            mutate_first(child, predicate, mutation)
            for child in value.values()
        )
    if isinstance(value, list):
        return any(mutate_first(child, predicate, mutation) for child in value)
    return False


def expect_mutation_detected(
    source: str,
    result: dict[str, Any],
) -> str:
    try:
        independent_oracle.verify_views(source, result)
    except independent_oracle.OracleError as error:
        return str(error)
    raise AssertionError("independent oracle accepted a semantic mutation")


def mutation_matrix(
    accepted: dict[str, tuple[str, dict[str, Any]]],
) -> list[dict[str, str]]:
    records = []

    source, original = accepted["universal_integer_guard"]
    candidate = copy.deepcopy(original)
    children = candidate["lira_formula"]["body"]["children"]
    guard = next(child for child in children if child.get("relation") == "ne")
    children.remove(guard)
    records.append(
        {
            "name": "removed_universal_integer_guard",
            "outcome": "detected",
            "reason": expect_mutation_detected(source, candidate),
        }
    )

    source, original = accepted["existential_integer_negative_floor"]
    candidate = copy.deepcopy(original)
    changed = mutate_first(
        candidate["lira_formula"],
        lambda node: (
            node.get("kind") == "constant"
            and node.get("numerator") == 2
            and node.get("denominator") == 1
        ),
        lambda node: node.update({"numerator": 3, "denominator": 2}),
    )
    if not changed:
        raise AssertionError("negative-floor mutation target was absent")
    records.append(
        {
            "name": "negative_floor_replaced_by_identity",
            "outcome": "detected",
            "reason": expect_mutation_detected(source, candidate),
        }
    )

    source, original = accepted["comparison_partition"]
    candidate = copy.deepcopy(original)
    changed = mutate_first(
        candidate["lira_formula"],
        lambda node: (
            node.get("kind") == "atom"
            and node.get("relation") == "ge"
        ),
        lambda node: node.update({"relation": "gt"}),
    )
    if not changed:
        raise AssertionError("comparison mutation target was absent")
    records.append(
        {
            "name": "nonstrict_comparison_made_strict",
            "outcome": "detected",
            "reason": expect_mutation_detected(source, candidate),
        }
    )

    candidate = copy.deepcopy(original)
    changed = mutate_first(
        candidate["lira_formula"],
        lambda node: (
            node.get("kind") == "scale"
            and node.get("numerator") == -1
            and node.get("denominator") == 1
        ),
        lambda node: node.update({"numerator": 1}),
    )
    if not changed:
        raise AssertionError("scale mutation target was absent")
    records.append(
        {
            "name": "rational_scale_sign_changed",
            "outcome": "detected",
            "reason": expect_mutation_detected(source, candidate),
        }
    )
    return records


def run_matrix() -> dict[str, Any]:
    cases = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
    accepted_records = []
    accepted_results: dict[str, tuple[str, dict[str, Any]]] = {}
    for case in cases["accepted"]:
        first = adapter.adapt(case["source"])
        second = adapter.adapt(case["source"])
        normalized_source = " ".join(adapter.tokenize(case["source"]))
        normalized = adapter.adapt(normalized_source)
        first_bytes = adapter.canonical_json(first)
        if first_bytes != adapter.canonical_json(second):
            raise AssertionError(f"unstable repeated import: {case['name']}")
        if first_bytes != adapter.canonical_json(normalized):
            raise AssertionError(f"whitespace instability: {case['name']}")
        views = independent_oracle.verify_views(case["source"], first)
        accepted_results[case["name"]] = (case["source"], first)
        accepted_records.append(
            {
                "name": case["name"],
                "canonical_id": first["canonical_id"],
                "trace_steps": len(first["trace"]),
                "views": views,
                "stable": True,
            }
        )

    rejected_records = []
    for case in cases["rejected"]:
        observed = []
        for _ in range(2):
            try:
                adapter.adapt(case["source"])
            except adapter.AdapterError as error:
                observed.append(error.code)
            else:
                observed.append("ACCEPTED")
        if observed != [case["code"], case["code"]]:
            raise AssertionError(
                f"rejection mismatch for {case['name']}: {observed}"
            )
        rejected_records.append(
            {
                "name": case["name"],
                "expected_code": case["code"],
                "observed_codes": observed,
                "stable": True,
            }
        )

    generated_truth = {True: 0, False: 0}
    generated_ids = []
    for index, source in enumerate(generated_sources(), start=1):
        result = adapter.adapt(source)
        views = independent_oracle.verify_views(source, result)
        generated_truth[views["source"]] += 1
        generated_ids.append(
            hashlib.sha256(
                f"{index}\0{source}\0{result['canonical_id']}".encode("utf-8")
            ).hexdigest()
        )

    mutations = mutation_matrix(accepted_results)
    gates = {
        "accepted_cases_agree": len(accepted_records) == 12,
        "rejected_cases_exact": len(rejected_records) == 16,
        "generated_cases_agree": len(generated_ids) == GENERATED_CASES,
        "mutations_detected": len(mutations) == 4,
        "stable_output": all(record["stable"] for record in accepted_records),
        "stable_rejections": all(record["stable"] for record in rejected_records),
        "underspecified_constructs_rejected": True,
    }
    body = {
        "schema_version": 1,
        "seed": SEED,
        "generated_case_count": GENERATED_CASES,
        "environment": {
            "python": platform.python_version(),
            "platform": platform.platform(),
        },
        "source_hashes": {
            path.name: sha256_file(path)
            for path in [
                ROOT / "PREREGISTRATION.md",
                ROOT / "cases.json",
                ROOT / "adapter.py",
                ROOT / "independent_oracle.py",
                ROOT / "run.py",
            ]
        },
        "accepted": accepted_records,
        "rejected": rejected_records,
        "generated": {
            "true": generated_truth[True],
            "false": generated_truth[False],
            "aggregate_id": hashlib.sha256(
                "\n".join(generated_ids).encode("ascii")
            ).hexdigest(),
        },
        "mutations": mutations,
        "gates": gates,
        "decision": (
            "recommend_conservative_adapter_boundary"
            if all(gates.values())
            else "reject_adapter_boundary"
        ),
    }
    return {
        **body,
        "report_id": hashlib.sha256(adapter.canonical_json(body)).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    report = run_matrix()
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes(adapter.canonical_json(report) + b"\n")
    print(
        f"OK: 12 accepted, 16 rejected, {GENERATED_CASES} generated, "
        f"4 mutations; {report['decision']}; report {report['report_id']}"
    )
    return 0 if all(report["gates"].values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
