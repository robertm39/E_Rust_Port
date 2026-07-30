#!/usr/bin/env python3
"""Independent exact reference search for frozen ground abstractions."""

from __future__ import annotations

import argparse
import hashlib
import json
import time
from fractions import Fraction
from pathlib import Path
from typing import Any, Sequence

from trace_model import clauses_satisfied, theory_context, unit_propagate


class SearchError(RuntimeError):
    """The frozen abstraction or exact reference decision is invalid."""


def canonical_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=True, separators=(",", ":"), sort_keys=True)


def fraction(value: str) -> Fraction:
    try:
        return Fraction(value)
    except (ValueError, ZeroDivisionError) as error:
        raise SearchError(f"invalid exact rational {value!r}") from error


def decide_difference(
    constraints: Sequence[dict[str, Any]],
    sort: str,
) -> dict[str, Any]:
    if sort not in {"Int", "Real"}:
        raise SearchError(f"unsupported sort {sort!r}")
    vertices = {"zero"}
    for constraint in constraints:
        if constraint["kind"] != "difference":
            raise SearchError("reference received a non-difference constraint")
        vertices.add(constraint["lhs"])
        vertices.add(constraint["rhs"])
        bound = fraction(constraint["bound"])
        if sort == "Int" and bound.denominator != 1:
            raise SearchError("integer query contains a fractional bound")
    distances = {vertex: Fraction(0) for vertex in vertices}
    for iteration in range(len(vertices)):
        changed = False
        for constraint in constraints:
            candidate = (
                distances[constraint["rhs"]] + fraction(constraint["bound"])
            )
            if candidate < distances[constraint["lhs"]]:
                distances[constraint["lhs"]] = candidate
                changed = True
                if iteration == len(vertices) - 1:
                    return {
                        "status": "unsat",
                        "core": [constraint["label"] for constraint in constraints],
                        "model": {},
                        "reason": "exact negative cycle",
                    }
        if not changed:
            zero = distances["zero"]
            model = {
                variable: str(distances[variable] - zero)
                for variable in sorted(vertices - {"zero"})
            }
            if not verify_model(constraints, model):
                raise SearchError("reference constructed an invalid model")
            return {
                "status": "sat",
                "core": [],
                "model": model,
                "reason": "exact Bellman-Ford potential",
            }
    raise SearchError("Bellman-Ford terminated without a verdict")


def verify_model(
    constraints: Sequence[dict[str, Any]],
    model: dict[str, str],
) -> bool:
    values = {variable: fraction(value) for variable, value in model.items()}
    values["zero"] = Fraction(0)
    for constraint in constraints:
        if constraint["lhs"] not in values or constraint["rhs"] not in values:
            return False
        if (
            values[constraint["lhs"]] - values[constraint["rhs"]]
            > fraction(constraint["bound"])
        ):
            return False
    return True


def verify_negative_cycle(
    constraints: Sequence[dict[str, Any]],
    core: Sequence[str],
) -> bool:
    if not core or len(core) != len(set(core)):
        return False
    by_label = {constraint["label"]: constraint for constraint in constraints}
    if any(label not in by_label for label in core):
        return False
    selected = [by_label[label] for label in core]
    vertices = {"zero"}
    for constraint in selected:
        vertices.add(constraint["lhs"])
        vertices.add(constraint["rhs"])
    distances = {vertex: Fraction(0) for vertex in vertices}
    for iteration in range(len(vertices)):
        changed = False
        for constraint in selected:
            candidate = (
                distances[constraint["rhs"]] + fraction(constraint["bound"])
            )
            if candidate < distances[constraint["lhs"]]:
                distances[constraint["lhs"]] = candidate
                changed = True
                if iteration == len(vertices) - 1:
                    return True
        if not changed:
            return False
    return False


def run_reference_search(
    abstraction: dict[str, Any],
    *,
    max_nodes: int = 4096,
    max_leaves: int = 1024,
) -> dict[str, Any]:
    if abstraction["bounds_crossed"]:
        return {
            "schema": "umlaut-real-ground-reference-search-v1",
            "source_id": abstraction["source_id"],
            "status": "bound",
            "bounds_crossed": abstraction["bounds_crossed"],
            "nodes": 0,
            "open_leaves": 0,
            "propositional_conflicts": 0,
            "theory_prunes": 0,
            "theory_cache_hits": 0,
            "queries": [],
            "events": [],
            "closed": False,
        }
    clauses = [tuple(clause["literals"]) for clause in abstraction["clauses"]]
    queries: list[dict[str, Any]] = []
    events: list[dict[str, Any]] = []
    node_count = 0
    open_leaves = 0
    conflicts = 0
    theory_prunes = 0
    theory_cache_hits = 0
    next_node = 1
    bound_hit: str | None = None
    decision_cache: dict[str, tuple[str, dict[str, Any]]] = {}

    def visit(
        assignment: dict[int, bool],
        decisions: list[dict[str, Any]],
        parent: int | None,
        previous_fingerprint: str | None,
    ) -> None:
        nonlocal node_count, open_leaves, conflicts, theory_prunes
        nonlocal theory_cache_hits
        nonlocal next_node, bound_hit
        if bound_hit is not None:
            return
        if node_count >= max_nodes:
            bound_hit = "nodes"
            return
        node_id = next_node
        next_node += 1
        node_count += 1
        propagated, units, conflict = unit_propagate(clauses, assignment)
        event: dict[str, Any] = {
            "node": node_id,
            "parent": parent,
            "depth": len(decisions),
            "decisions": list(decisions),
            "unit_steps": units,
            "assignment": [
                {"atom": atom, "value": value}
                for atom, value in sorted(propagated.items())
            ],
        }
        if conflict is not None:
            conflicts += 1
            event.update({"outcome": "propositional_conflict", "conflict": conflict})
            events.append(event)
            return

        context = theory_context(abstraction, propagated)
        if context["eligible"] and context["fingerprint"] != previous_fingerprint:
            cached = decision_cache.get(context["fingerprint"])
            if cached is None:
                started = time.perf_counter_ns()
                decision = decide_difference(
                    context["constraints"], context["sort"]
                )
                elapsed_ns = time.perf_counter_ns() - started
                query_id = (
                    f"{abstraction['source_id']}_q_{len(queries) + 1:05d}"
                )
                query = {
                    "id": query_id,
                    "node": node_id,
                    "parent": parent,
                    "depth": len(decisions),
                    "sort": context["sort"],
                    "fingerprint": context["fingerprint"],
                    "constraints": context["constraints"],
                    "excluded_unsupported": context["unsupported"],
                    "assignment": event["assignment"],
                    "decisions": list(decisions),
                    "unit_steps": units,
                    "reference": {**decision, "elapsed_ns": elapsed_ns},
                }
                queries.append(query)
                decision_cache[context["fingerprint"]] = (query_id, decision)
            else:
                query_id, decision = cached
                theory_cache_hits += 1
                event["theory_cache_hit"] = True
            event["theory_query"] = query_id
            event["theory_status"] = decision["status"]
            previous_fingerprint = context["fingerprint"]
            if decision["status"] == "unsat":
                if not verify_negative_cycle(
                    context["constraints"], decision["core"]
                ):
                    raise SearchError("reference UNSAT core failed replay")
                theory_prunes += 1
                event["outcome"] = "theory_pruned"
                events.append(event)
                return
        elif context["unsupported"]:
            event["theory_unknown"] = context["unsupported"]

        if clauses_satisfied(clauses, propagated):
            if open_leaves >= max_leaves:
                bound_hit = "leaves"
                return
            open_leaves += 1
            event["outcome"] = "open_leaf"
            events.append(event)
            return

        undecided = sorted(
            {
                abs(literal)
                for clause in clauses
                for literal in clause
                if abs(literal) not in propagated
            }
        )
        if not undecided:
            raise SearchError("unsatisfied clause set has no undecided atom")
        chosen = undecided[0]
        event.update({"outcome": "branch", "decision_atom": chosen})
        events.append(event)
        for value in (False, True):
            visit(
                {**propagated, chosen: value},
                [
                    *decisions,
                    {
                        "atom": chosen,
                        "value": value,
                        "parent_node": node_id,
                    },
                ],
                node_id,
                previous_fingerprint,
            )

    visit({}, [], None, None)
    complete = bound_hit is None
    return {
        "schema": "umlaut-real-ground-reference-search-v1",
        "source_id": abstraction["source_id"],
        "status": "complete" if complete else "bound",
        "bounds_crossed": [] if complete else [bound_hit],
        "nodes": node_count,
        "open_leaves": open_leaves,
        "propositional_conflicts": conflicts,
        "theory_prunes": theory_prunes,
        "theory_cache_hits": theory_cache_hits,
        "queries": queries,
        "events": events,
        "closed": complete and open_leaves == 0,
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace-build-root", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    build = json.loads(
        (arguments.trace_build_root / "trace-build.json").read_text(encoding="utf-8")
    )
    arguments.output_root.mkdir(parents=True, exist_ok=True)
    records = []
    for record in build["records"]:
        common = {
            "problem_id": record["problem_id"],
            "family": record["family"],
            "partition": record["partition"],
        }
        if record["status"] != "traced":
            records.append({**common, "status": record["status"]})
            continue
        abstraction_path = (
            arguments.trace_build_root
            / record["problem_id"]
            / "abstraction.json"
        )
        if sha256_file(abstraction_path) != record["abstraction_sha256"]:
            raise SearchError(
                f"abstraction hash mismatch for {record['problem_id']}"
            )
        abstraction = json.loads(abstraction_path.read_text(encoding="utf-8"))
        search = run_reference_search(abstraction)
        problem_root = arguments.output_root / record["problem_id"]
        problem_root.mkdir(parents=True, exist_ok=True)
        search_path = problem_root / "reference-search.json"
        search_path.write_text(canonical_json(search) + "\n", encoding="utf-8")
        records.append(
            {
                **common,
                "status": "searched",
                "search_status": search["status"],
                "nodes": search["nodes"],
                "open_leaves": search["open_leaves"],
                "propositional_conflicts": search["propositional_conflicts"],
                "theory_prunes": search["theory_prunes"],
                "theory_cache_hits": search["theory_cache_hits"],
                "query_count": len(search["queries"]),
                "closed": search["closed"],
                "search_sha256": sha256_file(search_path),
            }
        )
    report = {
        "schema": "umlaut-real-ground-reference-batch-v1",
        "trace_build_sha256": sha256_file(
            arguments.trace_build_root / "trace-build.json"
        ),
        "records": records,
        "totals": {
            "searched": sum(record["status"] == "searched" for record in records),
            "queries": sum(record.get("query_count", 0) for record in records),
            "theory_prunes": sum(
                record.get("theory_prunes", 0) for record in records
            ),
            "theory_cache_hits": sum(
                record.get("theory_cache_hits", 0) for record in records
            ),
            "closed": sum(record.get("closed", False) for record in records),
            "eligible_families": sorted(
                {
                    record["family"]
                    for record in records
                    if record.get("query_count", 0)
                }
            ),
        },
    }
    report_path = arguments.output_root / "reference-batch.json"
    report_path.write_text(canonical_json(report) + "\n", encoding="utf-8")
    print(json.dumps(report["totals"], sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
