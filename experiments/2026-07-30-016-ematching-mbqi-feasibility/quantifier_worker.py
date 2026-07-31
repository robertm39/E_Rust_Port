#!/usr/bin/env python3
"""Bounded clausification, E-matching, and MBQI for function-free EPR."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import itertools
import json
import sys
import time
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Iterator, Sequence


SCHEMA_VERSION = 1
MAX_BATCH = 64
SOLVER_RESERVE_SECONDS = 0.25


def load_instgen(repo_root: Path) -> ModuleType:
    path = (
        repo_root
        / "experiments/2026-07-30-008-instgen-epr-prototype/instgen.py"
    )
    specification = importlib.util.spec_from_file_location("instgen_008", path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load InstGen support module {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[specification.name] = module
    specification.loader.exec_module(module)
    return module


def stable_json_sha256(value: object) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(encoded).hexdigest()


def initial_abstraction(
    instgen: ModuleType, problem: Any
) -> tuple[
    set[tuple[tuple[str, bool], ...]],
    list[dict[str, Any]],
    list[tuple[Any, ...]],
]:
    known: set[tuple[tuple[str, bool], ...]] = set()
    instances: list[dict[str, Any]] = []
    ground_clauses: list[tuple[Any, ...]] = []
    for clause in problem.clauses:
        substitution = {
            variable: problem.constants[0] for variable in clause.variables
        }
        ground = instgen.ground_clause(clause, substitution)
        if ground is None:
            continue
        instgen.add_instance(
            clause=clause,
            substitution=substitution,
            ground=ground,
            known=known,
            instances=instances,
            ground_clauses=ground_clauses,
            phase="initial",
            iteration=0,
        )
    return known, instances, ground_clauses


def source_substitutions(
    clause: Any, constants: Sequence[str]
) -> Iterator[dict[str, str]]:
    if not clause.variables:
        yield {}
        return
    for values in itertools.product(constants, repeat=len(clause.variables)):
        yield dict(zip(clause.variables, values, strict=True))


def atom_variables(atom: Any, variables: frozenset[str]) -> frozenset[str]:
    return frozenset(argument for argument in atom.arguments if argument in variables)


def infer_trigger(clause: Any) -> tuple[Any, ...]:
    """Infer the preregistered unary pattern or greedy multipattern."""

    variables = frozenset(clause.variables)
    if not variables:
        return ()
    atoms = [literal.atom for literal in clause.literals]
    for atom in atoms:
        if atom_variables(atom, variables) == variables:
            return (atom,)

    uncovered = set(variables)
    selected: list[Any] = []
    remaining = list(enumerate(atoms))
    while uncovered:
        best_index = -1
        best_gain = 0
        for position, atom in remaining:
            gain = len(atom_variables(atom, variables) & uncovered)
            if gain > best_gain:
                best_index = position
                best_gain = gain
        if best_index < 0:
            raise ValueError(
                f"clause {clause.name} has a variable absent from every atom"
            )
        atom = atoms[best_index]
        selected.append(atom)
        uncovered.difference_update(atom_variables(atom, variables))
        remaining = [
            item for item in remaining if item[0] != best_index
        ]
    return tuple(selected)


def trigger_record(trigger: Sequence[Any]) -> list[str]:
    return [atom.canonical() for atom in trigger]


def match_atom(
    pattern: Any,
    ground: Any,
    variables: frozenset[str],
    binding: dict[str, str],
) -> dict[str, str] | None:
    if (
        pattern.predicate != ground.predicate
        or len(pattern.arguments) != len(ground.arguments)
    ):
        return None
    result = dict(binding)
    for template, value in zip(
        pattern.arguments, ground.arguments, strict=True
    ):
        if template in variables:
            previous = result.get(template)
            if previous is not None and previous != value:
                return None
            result[template] = value
        elif template != value:
            return None
    return result


def matching_substitutions(
    trigger: Sequence[Any],
    variables: Sequence[str],
    atoms: Sequence[Any],
) -> Iterator[dict[str, str]]:
    variable_set = frozenset(variables)

    def visit(
        pattern_index: int, binding: dict[str, str]
    ) -> Iterator[dict[str, str]]:
        if pattern_index == len(trigger):
            if set(binding) == variable_set:
                yield dict(binding)
            return
        pattern = trigger[pattern_index]
        for atom in atoms:
            merged = match_atom(pattern, atom, variable_set, binding)
            if merged is not None:
                yield from visit(pattern_index + 1, merged)

    yield from visit(0, {})


def sorted_atoms(ground_clauses: Iterable[tuple[Any, ...]]) -> list[Any]:
    atoms = {
        literal.atom for clause in ground_clauses for literal in clause
    }
    return sorted(atoms, key=lambda atom: atom.canonical())


def solve_abstraction(
    instgen: ModuleType,
    *,
    adapter: Path,
    dimacs: Path,
    ground_clauses: Sequence[tuple[Any, ...]],
    deadline: float,
) -> tuple[dict[str, Any], dict[Any, int]]:
    mapping = instgen.atom_map(ground_clauses)
    instgen.write_dimacs(dimacs, ground_clauses, mapping)
    remaining = max(0.001, deadline - time.monotonic())
    return instgen.solve_dimacs(adapter, dimacs, remaining), mapping


def all_clauses_satisfied(
    ground_clauses: Sequence[tuple[Any, ...]], model: dict[Any, bool]
) -> bool:
    return all(
        not all(
            model.get(literal.atom, False) != literal.positive
            for literal in clause
        )
        for clause in ground_clauses
    )


def scan_complete_model(
    instgen: ModuleType,
    *,
    problem: Any,
    model: dict[Any, bool],
    deadline: float,
    initial_count: int,
) -> tuple[bool, int, dict[str, Any] | None, str]:
    enumerated = initial_count
    for clause in problem.clauses:
        for substitution in source_substitutions(clause, problem.constants):
            enumerated += 1
            if (enumerated & 255) == 0 and time.monotonic() >= deadline:
                return False, enumerated, None, "wall_limit_during_model_scan"
            ground = instgen.ground_clause(clause, substitution)
            if ground is None:
                continue
            if instgen.ground_clause_is_false(ground, model):
                return (
                    False,
                    enumerated,
                    {
                        "source_index": clause.index,
                        "source_name": clause.name,
                        "substitution": dict(sorted(substitution.items())),
                        "ground_clause": instgen.clause_record(ground),
                    },
                    "ungenerated_model_counterexample",
                )
    return True, enumerated, None, "complete_herbrand_model"


def semantic_payload(
    *,
    method: str,
    problem: Any,
    status: str,
    reason: str,
    instances: Sequence[dict[str, Any]],
    method_data: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "method": method,
        "source_sha256": problem.source_sha256,
        "status": status,
        "termination_reason": reason,
        "instances": [
            {
                "source_index": instance["source_index"],
                "source_name": instance["source_name"],
                "substitution": instance["substitution"],
                "ground_clause": instance["ground_clause"],
                "phase": instance["phase"],
                "iteration": instance["iteration"],
            }
            for instance in instances
        ],
        "method_data": method_data,
    }


def finalize(
    instgen: ModuleType,
    *,
    method: str,
    problem_path: Path,
    problem: Any,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
    started: float,
    initial_resources: tuple[float, float, int],
    status: str,
    reason: str,
    sat_calls: int,
    sat_ns: int,
    refinements: int,
    enumerated_substitutions: int,
    instances: list[dict[str, Any]],
    ground_clauses: list[tuple[Any, ...]],
    final_model: dict[Any, bool],
    method_data: dict[str, Any],
) -> dict[str, Any]:
    mapping = instgen.atom_map(ground_clauses)
    dimacs = output_root / "final.cnf"
    instgen.write_dimacs(dimacs, ground_clauses, mapping)
    rendered = output_root / "instances.p"
    rendered.write_text(
        instgen.render_instances(instances), encoding="utf-8", newline="\n"
    )
    final_user, final_system, final_rss = instgen.resource_snapshot()
    initial_user, initial_system, _ = initial_resources
    payload = semantic_payload(
        method=method,
        problem=problem,
        status=status,
        reason=reason,
        instances=instances,
        method_data=method_data,
    )
    certificate: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "method": method,
        "source_path": str(problem_path),
        "source_sha256": problem.source_sha256,
        "status": status,
        "termination_reason": reason,
        "budget_seconds": budget_seconds,
        "search_wall_seconds": time.monotonic() - started,
        "search_user_seconds": max(0.0, final_user - initial_user),
        "search_system_seconds": max(0.0, final_system - initial_system),
        "search_max_rss_kib": final_rss,
        "sat_calls": sat_calls,
        "sat_ns": sat_ns,
        "refinement_iterations": refinements,
        "generated_instances": len(instances),
        "unique_ground_clauses": len(ground_clauses),
        "enumerated_substitutions": enumerated_substitutions,
        "ground_instance_count": str(problem.ground_instance_count),
        "grounding_fraction": {
            "numerator": len(ground_clauses),
            "denominator": str(problem.ground_instance_count),
        },
        "source_clauses": len(problem.clauses),
        "domain_constants": list(problem.constants),
        "instances": instances,
        "atom_map": {
            atom.canonical(): variable
            for atom, variable in sorted(
                mapping.items(), key=lambda item: item[1]
            )
        },
        "true_atoms": sorted(
            atom.canonical()
            for atom, value in final_model.items()
            if value
        ),
        "method_data": method_data,
        "semantic_sha256": stable_json_sha256(payload),
        "dimacs_path": dimacs.name,
        "dimacs_sha256": instgen.sha256_file(dimacs),
        "dimacs_bytes": dimacs.stat().st_size,
        "instances_path": rendered.name,
        "instances_sha256": instgen.sha256_file(rendered),
        "instances_bytes": rendered.stat().st_size,
        "proof": None,
    }
    if status == "unsat":
        certificate["proof"] = instgen.certify_unsat(
            adapter=adapter,
            drat_trim=drat_trim,
            dimacs=dimacs,
            output_root=output_root,
        )
    certificate_path = output_root / "certificate.json"
    certificate_path.write_text(
        json.dumps(certificate, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return certificate


def run_clausify(
    instgen: ModuleType,
    *,
    problem_path: Path,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
    max_instances: int,
    max_steps: int,
) -> dict[str, Any]:
    problem = instgen.parse_problem(problem_path.read_text(encoding="utf-8"))
    known, instances, ground_clauses = initial_abstraction(instgen, problem)
    initial_resources = instgen.resource_snapshot()
    started = time.monotonic()
    deadline = started + budget_seconds
    enumeration_deadline = max(
        started, deadline - min(SOLVER_RESERVE_SECONDS, budget_seconds / 4)
    )
    enumerated = 0
    complete = True
    stop_reason = "complete_grounding"
    for clause in problem.clauses:
        for substitution in source_substitutions(clause, problem.constants):
            enumerated += 1
            if (
                (enumerated & 255) == 0
                and time.monotonic() >= enumeration_deadline
            ):
                complete = False
                stop_reason = "wall_limit_during_clausification"
                break
            if enumerated >= max_steps:
                complete = False
                stop_reason = "step_limit"
                break
            ground = instgen.ground_clause(clause, substitution)
            if ground is not None:
                instgen.add_instance(
                    clause=clause,
                    substitution=substitution,
                    ground=ground,
                    known=known,
                    instances=instances,
                    ground_clauses=ground_clauses,
                    phase="clausify",
                    iteration=0,
                )
            if len(ground_clauses) >= max_instances:
                complete = False
                stop_reason = "instance_limit"
                break
        if not complete:
            break

    dimacs = output_root / "final.cnf"
    result, mapping = solve_abstraction(
        instgen,
        adapter=adapter,
        dimacs=dimacs,
        ground_clauses=ground_clauses,
        deadline=deadline,
    )
    sat_calls = 1
    sat_ns = int(result.get("solve_ns", 0))
    final_model: dict[Any, bool] = {}
    if result["status"] == "unsat":
        status = "unsat"
        reason = "ground_abstraction_unsat"
    elif result["status"] == "sat" and complete:
        status = "sat"
        reason = "complete_grounding"
        final_model = instgen.model_from_result(result, mapping)
    else:
        status = "unknown"
        reason = (
            str(result.get("reason", "solver_unknown"))
            if result["status"] == "unknown"
            else stop_reason
        )
    method_data = {
        "enumeration_complete": complete,
        "enumeration_stop_reason": stop_reason,
        "instance_limit": max_instances,
        "step_limit": max_steps,
    }
    return finalize(
        instgen,
        method="clausify",
        problem_path=problem_path,
        problem=problem,
        adapter=adapter,
        drat_trim=drat_trim,
        output_root=output_root,
        budget_seconds=budget_seconds,
        started=started,
        initial_resources=initial_resources,
        status=status,
        reason=reason,
        sat_calls=sat_calls,
        sat_ns=sat_ns,
        refinements=0,
        enumerated_substitutions=enumerated,
        instances=instances,
        ground_clauses=ground_clauses,
        final_model=final_model,
        method_data=method_data,
    )


def run_ematch(
    instgen: ModuleType,
    *,
    problem_path: Path,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
    max_instances: int,
    max_steps: int,
) -> dict[str, Any]:
    problem = instgen.parse_problem(problem_path.read_text(encoding="utf-8"))
    known, instances, ground_clauses = initial_abstraction(instgen, problem)
    triggers = {
        clause.index: infer_trigger(clause) for clause in problem.clauses
    }
    initial_resources = instgen.resource_snapshot()
    started = time.monotonic()
    deadline = started + budget_seconds
    matching_deadline = max(
        started, deadline - min(SOLVER_RESERVE_SECONDS, budget_seconds / 4)
    )
    rounds: list[dict[str, Any]] = []
    candidate_matches = 0
    duplicate_substitutions = 0
    duplicate_ground_clauses = 0
    fixed_point = False
    limit_reason: str | None = None

    round_index = 0
    while time.monotonic() < matching_deadline:
        round_index += 1
        atoms_before = sorted_atoms(ground_clauses)
        instances_before = len(instances)
        candidates_before = candidate_matches
        duplicate_substitutions_before = duplicate_substitutions
        duplicate_ground_before = duplicate_ground_clauses
        for clause in problem.clauses:
            trigger = triggers[clause.index]
            if not trigger:
                continue
            seen_bindings: set[tuple[tuple[str, str], ...]] = set()
            for substitution in matching_substitutions(
                trigger, clause.variables, atoms_before
            ):
                candidate_matches += 1
                key = tuple(sorted(substitution.items()))
                if key in seen_bindings:
                    duplicate_substitutions += 1
                    continue
                seen_bindings.add(key)
                ground = instgen.ground_clause(clause, substitution)
                if ground is None:
                    continue
                if not instgen.add_instance(
                    clause=clause,
                    substitution=substitution,
                    ground=ground,
                    known=known,
                    instances=instances,
                    ground_clauses=ground_clauses,
                    phase="ematch",
                    iteration=round_index,
                ):
                    duplicate_ground_clauses += 1
                if len(ground_clauses) >= max_instances:
                    limit_reason = "instance_limit"
                    break
                if candidate_matches >= max_steps:
                    limit_reason = "step_limit"
                    break
                if (
                    candidate_matches & 255
                ) == 0 and time.monotonic() >= matching_deadline:
                    limit_reason = "wall_limit_during_matching"
                    break
            if limit_reason is not None:
                break
        atoms_after = sorted_atoms(ground_clauses)
        rounds.append(
            {
                "round": round_index,
                "atoms_before": len(atoms_before),
                "atoms_after": len(atoms_after),
                "instances_before": instances_before,
                "instances_after": len(instances),
                "candidate_matches": candidate_matches - candidates_before,
                "duplicate_substitutions": (
                    duplicate_substitutions
                    - duplicate_substitutions_before
                ),
                "duplicate_ground_clauses": (
                    duplicate_ground_clauses - duplicate_ground_before
                ),
            }
        )
        if limit_reason is not None:
            break
        if len(instances) == instances_before:
            fixed_point = True
            break

    if not fixed_point and limit_reason is None:
        limit_reason = "wall_limit_during_matching"

    dimacs = output_root / "final.cnf"
    result, mapping = solve_abstraction(
        instgen,
        adapter=adapter,
        dimacs=dimacs,
        ground_clauses=ground_clauses,
        deadline=deadline,
    )
    sat_calls = 1
    sat_ns = int(result.get("solve_ns", 0))
    final_model: dict[Any, bool] = {}
    enumerated = 0
    first_counterexample: dict[str, Any] | None = None
    if result["status"] == "unsat":
        status = "unsat"
        reason = "ground_abstraction_unsat"
    elif result["status"] == "unknown":
        status = "unknown"
        reason = str(result.get("reason", "solver_unknown"))
    elif not fixed_point:
        status = "unknown"
        reason = limit_reason or "matching_incomplete"
    else:
        final_model = instgen.model_from_result(result, mapping)
        (
            model_complete,
            enumerated,
            first_counterexample,
            scan_reason,
        ) = scan_complete_model(
            instgen,
            problem=problem,
            model=final_model,
            deadline=deadline,
            initial_count=0,
        )
        if model_complete:
            status = "sat"
            reason = "complete_herbrand_model"
        else:
            status = "unknown"
            reason = scan_reason

    trigger_records = [
        {
            "source_index": clause.index,
            "source_name": clause.name,
            "variables": list(clause.variables),
            "pattern": trigger_record(triggers[clause.index]),
        }
        for clause in problem.clauses
    ]
    method_data = {
        "triggers": trigger_records,
        "unary_patterns": sum(
            1 for trigger in triggers.values() if len(trigger) == 1
        ),
        "multipatterns": sum(
            1 for trigger in triggers.values() if len(trigger) > 1
        ),
        "maximum_pattern_size": max(map(len, triggers.values()), default=0),
        "rounds": rounds,
        "round_count": len(rounds),
        "candidate_matches": candidate_matches,
        "duplicate_substitutions": duplicate_substitutions,
        "duplicate_ground_clauses": duplicate_ground_clauses,
        "fixed_point": fixed_point,
        "limit_reason": limit_reason,
        "first_ungenerated_counterexample": first_counterexample,
        "instance_limit": max_instances,
        "step_limit": max_steps,
    }
    return finalize(
        instgen,
        method="ematch",
        problem_path=problem_path,
        problem=problem,
        adapter=adapter,
        drat_trim=drat_trim,
        output_root=output_root,
        budget_seconds=budget_seconds,
        started=started,
        initial_resources=initial_resources,
        status=status,
        reason=reason,
        sat_calls=sat_calls,
        sat_ns=sat_ns,
        refinements=0,
        enumerated_substitutions=enumerated,
        instances=instances,
        ground_clauses=ground_clauses,
        final_model=final_model,
        method_data=method_data,
    )


def run_mbqi(
    instgen: ModuleType,
    *,
    problem_path: Path,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
    max_instances: int,
    max_steps: int,
) -> dict[str, Any]:
    problem = instgen.parse_problem(problem_path.read_text(encoding="utf-8"))
    known, instances, ground_clauses = initial_abstraction(instgen, problem)
    initial_resources = instgen.resource_snapshot()
    started = time.monotonic()
    deadline = started + budget_seconds
    status = "unknown"
    reason = "wall_limit"
    sat_calls = 0
    sat_ns = 0
    refinements = 0
    enumerated = 0
    final_model: dict[Any, bool] = {}
    refinement_log: list[dict[str, Any]] = []
    dimacs = output_root / "final.cnf"

    while time.monotonic() < deadline:
        result, mapping = solve_abstraction(
            instgen,
            adapter=adapter,
            dimacs=dimacs,
            ground_clauses=ground_clauses,
            deadline=deadline,
        )
        sat_calls += 1
        sat_ns += int(result.get("solve_ns", 0))
        if result["status"] == "unknown":
            reason = str(result.get("reason", "solver_unknown"))
            break
        if result["status"] == "unsat":
            status = "unsat"
            reason = "ground_abstraction_unsat"
            break

        model = instgen.model_from_result(result, mapping)
        final_model = model
        known_before = len(ground_clauses)
        log_entry: dict[str, Any] = {
            "sat_call": sat_calls,
            "known_before": known_before,
            "true_atoms": sorted(
                atom.canonical() for atom, value in model.items() if value
            ),
            "added_instance_indices": [],
        }
        added = 0
        scan_complete = True
        solved_clause_keys = set(known)
        for clause in problem.clauses:
            for substitution in source_substitutions(
                clause, problem.constants
            ):
                enumerated += 1
                if (
                    enumerated & 255
                ) == 0 and time.monotonic() >= deadline:
                    scan_complete = False
                    reason = "wall_limit_during_counterexample_scan"
                    break
                if enumerated >= max_steps:
                    scan_complete = False
                    reason = "step_limit"
                    break
                ground = instgen.ground_clause(clause, substitution)
                if ground is None:
                    continue
                if instgen.ground_clause_is_false(ground, model):
                    key = instgen.clause_key(ground)
                    if key in solved_clause_keys:
                        raise ValueError(
                            "candidate model falsifies its prior abstraction"
                        )
                    if key in known:
                        break
                    if not instgen.add_instance(
                        clause=clause,
                        substitution=substitution,
                        ground=ground,
                        known=known,
                        instances=instances,
                        ground_clauses=ground_clauses,
                        phase="mbqi",
                        iteration=sat_calls,
                    ):
                        raise ValueError("new MBQI instance was not added")
                    log_entry["added_instance_indices"].append(
                        len(instances) - 1
                    )
                    added += 1
                    break
            if (
                not scan_complete
                or added >= MAX_BATCH
                or len(ground_clauses) >= max_instances
            ):
                break
        refinement_log.append(log_entry)
        if not scan_complete:
            break
        if len(ground_clauses) >= max_instances and added:
            reason = "instance_limit"
            break
        if added == 0:
            status = "sat"
            reason = "complete_herbrand_model"
            break
        refinements += 1

    method_data = {
        "batch_limit": MAX_BATCH,
        "instance_limit": max_instances,
        "step_limit": max_steps,
        "refinement_log": refinement_log,
    }
    return finalize(
        instgen,
        method="mbqi",
        problem_path=problem_path,
        problem=problem,
        adapter=adapter,
        drat_trim=drat_trim,
        output_root=output_root,
        budget_seconds=budget_seconds,
        started=started,
        initial_resources=initial_resources,
        status=status,
        reason=reason,
        sat_calls=sat_calls,
        sat_ns=sat_ns,
        refinements=refinements,
        enumerated_substitutions=enumerated,
        instances=instances,
        ground_clauses=ground_clauses,
        final_model=final_model,
        method_data=method_data,
    )


def run(
    *,
    method: str,
    repo_root: Path,
    problem_path: Path,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
    max_instances: int,
    max_steps: int,
) -> dict[str, Any]:
    if budget_seconds <= 0:
        raise ValueError("budget must be positive")
    if max_instances <= 0:
        raise ValueError("instance limit must be positive")
    if max_steps <= 0:
        raise ValueError("step limit must be positive")
    output_root.mkdir(parents=True, exist_ok=True)
    instgen = load_instgen(repo_root)
    callback = {
        "clausify": run_clausify,
        "ematch": run_ematch,
        "mbqi": run_mbqi,
    }[method]
    return callback(
        instgen,
        problem_path=problem_path,
        adapter=adapter,
        drat_trim=drat_trim,
        output_root=output_root,
        budget_seconds=budget_seconds,
        max_instances=max_instances,
        max_steps=max_steps,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--method", choices=("clausify", "ematch", "mbqi"), required=True
    )
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--budget-seconds", type=float, default=4.0)
    parser.add_argument("--max-instances", type=int, default=100_000)
    parser.add_argument("--max-steps", type=int, default=250_000)
    arguments = parser.parse_args()
    result = run(
        method=arguments.method,
        repo_root=arguments.repo_root.resolve(),
        problem_path=arguments.problem.resolve(),
        adapter=arguments.cadical_driver.resolve(),
        drat_trim=arguments.drat_trim.resolve(),
        output_root=arguments.output_root.resolve(),
        budget_seconds=arguments.budget_seconds,
        max_instances=arguments.max_instances,
        max_steps=arguments.max_steps,
    )
    print(
        json.dumps(
            {
                "method": result["method"],
                "status": result["status"],
                "termination_reason": result["termination_reason"],
                "generated_instances": result["generated_instances"],
                "semantic_sha256": result["semantic_sha256"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
