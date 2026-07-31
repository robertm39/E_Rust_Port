#!/usr/bin/env python3
"""Independently replay quantifier-instance and method-specific certificates."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


class VerificationError(ValueError):
    """A certificate violates the experiment contract."""


def load_module(name: str, path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load module {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def stable_json_sha256(value: object) -> str:
    encoded = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("ascii")
    return hashlib.sha256(encoded).hexdigest()


def source_atom(atom: Any) -> tuple[str, tuple[str, ...]]:
    return (
        atom.relation,
        tuple(term.symbol for term in atom.arguments),
    )


def atom_variables(
    atom: tuple[str, tuple[str, ...]], variables: frozenset[str]
) -> frozenset[str]:
    return frozenset(argument for argument in atom[1] if argument in variables)


def canonical_atom(
    atom: tuple[str, tuple[str, ...]],
    substitution: dict[str, str] | None = None,
) -> str:
    substitution = substitution or {}
    arguments = [substitution.get(value, value) for value in atom[1]]
    return f"{atom[0]}(" + ",".join(arguments) + ")"


def infer_trigger(clause: dict[str, Any]) -> tuple[tuple[str, tuple[str, ...]], ...]:
    variables = frozenset(clause["variables"])
    if not variables:
        return ()
    atoms = [source_atom(literal.atom) for literal in clause["literals"]]
    for atom in atoms:
        if atom_variables(atom, variables) == variables:
            return (atom,)
    uncovered = set(variables)
    selected: list[tuple[str, tuple[str, ...]]] = []
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
            raise VerificationError("trigger inference cannot cover variables")
        atom = atoms[best_index]
        selected.append(atom)
        uncovered.difference_update(atom_variables(atom, variables))
        remaining = [
            item for item in remaining if item[0] != best_index
        ]
    return tuple(selected)


def record_clause(record: Any) -> tuple[tuple[str, bool], ...]:
    if not isinstance(record, list):
        raise VerificationError("ground clause is not a list")
    parsed: list[tuple[str, bool]] = []
    for literal in record:
        if (
            not isinstance(literal, dict)
            or not isinstance(literal.get("atom"), str)
            or not isinstance(literal.get("positive"), bool)
        ):
            raise VerificationError("malformed ground literal")
        parsed.append((literal["atom"], literal["positive"]))
    return tuple(parsed)


def clause_satisfied(
    clause: Iterable[tuple[str, bool]], true_atoms: frozenset[str]
) -> bool:
    return any((atom in true_atoms) == positive for atom, positive in clause)


def semantic_payload(certificate: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": certificate["schema_version"],
        "method": certificate["method"],
        "source_sha256": certificate["source_sha256"],
        "status": certificate["status"],
        "termination_reason": certificate["termination_reason"],
        "instances": [
            {
                "source_index": instance["source_index"],
                "source_name": instance["source_name"],
                "substitution": instance["substitution"],
                "ground_clause": instance["ground_clause"],
                "phase": instance["phase"],
                "iteration": instance["iteration"],
            }
            for instance in certificate["instances"]
        ],
        "method_data": certificate["method_data"],
    }


def verify_ematch(
    *,
    certificate: dict[str, Any],
    clauses: Sequence[dict[str, Any]],
) -> dict[str, Any]:
    method_data = certificate.get("method_data")
    if not isinstance(method_data, dict):
        raise VerificationError("E-matching method data is missing")
    trigger_records = method_data.get("triggers")
    if not isinstance(trigger_records, list) or len(trigger_records) != len(
        clauses
    ):
        raise VerificationError("E-matching trigger table is malformed")
    expected_triggers: dict[int, tuple[tuple[str, tuple[str, ...]], ...]] = {}
    for clause, record in zip(clauses, trigger_records, strict=True):
        trigger = infer_trigger(clause)
        expected_triggers[clause["index"]] = trigger
        expected_record = {
            "source_index": clause["index"],
            "source_name": clause["name"],
            "variables": list(clause["variables"]),
            "pattern": [canonical_atom(atom) for atom in trigger],
        }
        if record != expected_record:
            raise VerificationError("inferred trigger record mismatch")

    instances = certificate["instances"]
    initial = [
        instance for instance in instances if instance.get("phase") == "initial"
    ]
    if len(initial) > len(clauses):
        raise VerificationError("too many initial instances")
    ground_atoms = {
        atom
        for instance in initial
        for atom, _ in record_clause(instance["ground_clause"])
    }
    ematch_instances = [
        instance for instance in instances if instance.get("phase") == "ematch"
    ]
    if len(initial) + len(ematch_instances) != len(instances):
        raise VerificationError("unexpected E-matching instance phase")

    rounds = method_data.get("rounds")
    if not isinstance(rounds, list):
        raise VerificationError("E-matching round table is missing")
    replayed = 0
    for round_index, round_record in enumerate(rounds, start=1):
        if (
            not isinstance(round_record, dict)
            or round_record.get("round") != round_index
            or round_record.get("atoms_before") != len(ground_atoms)
        ):
            raise VerificationError("E-matching round prefix mismatch")
        this_round = [
            instance
            for instance in ematch_instances
            if instance.get("iteration") == round_index
        ]
        new_atoms: set[str] = set()
        for instance in this_round:
            source_index = instance["source_index"]
            trigger = expected_triggers[source_index]
            substitution = instance["substitution"]
            for atom in trigger:
                if canonical_atom(atom, substitution) not in ground_atoms:
                    raise VerificationError(
                        "E-matching instance lacks a prior trigger match"
                    )
            new_atoms.update(
                atom for atom, _ in record_clause(instance["ground_clause"])
            )
        replayed += len(this_round)
        ground_atoms.update(new_atoms)
        if (
            round_record.get("instances_before")
            != len(initial) + replayed - len(this_round)
            or round_record.get("instances_after")
            - round_record.get("instances_before")
            != len(this_round)
            or round_record.get("atoms_after") != len(ground_atoms)
        ):
            raise VerificationError("E-matching round delta mismatch")
        for counter in (
            "candidate_matches",
            "duplicate_substitutions",
            "duplicate_ground_clauses",
        ):
            if (
                not isinstance(round_record.get(counter), int)
                or round_record[counter] < 0
            ):
                raise VerificationError("invalid E-matching loop counter")
    if replayed != len(ematch_instances):
        raise VerificationError("E-matching instance has no round record")

    counterexample = method_data.get("first_ungenerated_counterexample")
    if counterexample is not None:
        if not isinstance(counterexample, dict):
            raise VerificationError("malformed ungenerated counterexample")
        index = counterexample.get("source_index")
        if not isinstance(index, int) or not 0 <= index < len(clauses):
            raise VerificationError("counterexample source index is invalid")
        if counterexample.get("source_name") != clauses[index]["name"]:
            raise VerificationError("counterexample source name mismatch")
        substitution = counterexample.get("substitution")
        if (
            not isinstance(substitution, dict)
            or set(substitution) != set(clauses[index]["variables"])
        ):
            raise VerificationError("counterexample substitution is invalid")
        true_atoms = frozenset(certificate.get("true_atoms", []))
        if clause_satisfied(
            record_clause(counterexample.get("ground_clause")), true_atoms
        ):
            raise VerificationError(
                "recorded E-matching counterexample is not false"
            )
        retained = {
            record_clause(instance["ground_clause"]) for instance in instances
        }
        if record_clause(counterexample["ground_clause"]) in retained:
            raise VerificationError(
                "recorded E-matching counterexample was generated"
            )
    return {
        "trigger_records_checked": len(trigger_records),
        "trigger_instances_checked": replayed,
        "rounds_checked": len(rounds),
    }


def verify_mbqi(certificate: dict[str, Any]) -> dict[str, Any]:
    method_data = certificate.get("method_data")
    if not isinstance(method_data, dict):
        raise VerificationError("MBQI method data is missing")
    log = method_data.get("refinement_log")
    if not isinstance(log, list):
        raise VerificationError("MBQI refinement log is missing")
    instances = certificate["instances"]
    initial_count = 0
    for instance in instances:
        if instance.get("phase") != "initial":
            break
        initial_count += 1
    next_index = initial_count
    for sat_call, record in enumerate(log, start=1):
        if (
            not isinstance(record, dict)
            or record.get("sat_call") != sat_call
            or record.get("known_before") != next_index
        ):
            raise VerificationError("MBQI refinement prefix mismatch")
        raw_true = record.get("true_atoms")
        if (
            not isinstance(raw_true, list)
            or any(not isinstance(atom, str) for atom in raw_true)
            or len(set(raw_true)) != len(raw_true)
        ):
            raise VerificationError("MBQI candidate model is malformed")
        true_atoms = frozenset(raw_true)
        prior_atoms = {
            atom
            for instance in instances[:next_index]
            for atom, _ in record_clause(instance["ground_clause"])
        }
        if not true_atoms <= prior_atoms:
            raise VerificationError("MBQI model assigns an unknown atom true")
        for instance in instances[:next_index]:
            if not clause_satisfied(
                record_clause(instance["ground_clause"]), true_atoms
            ):
                raise VerificationError(
                    "MBQI candidate model falsifies its prior abstraction"
                )
        added = record.get("added_instance_indices")
        if not isinstance(added, list) or added != list(
            range(next_index, next_index + len(added))
        ):
            raise VerificationError("MBQI added-index sequence is malformed")
        for index in added:
            if not 0 <= index < len(instances):
                raise VerificationError("MBQI added index is out of range")
            instance = instances[index]
            if (
                instance.get("phase") != "mbqi"
                or instance.get("iteration") != sat_call
                or clause_satisfied(
                    record_clause(instance["ground_clause"]), true_atoms
                )
            ):
                raise VerificationError(
                    "MBQI refinement is not a logged counterexample"
                )
        next_index += len(added)
    if next_index != len(instances):
        raise VerificationError("MBQI instance lacks a refinement log")
    return {
        "refinement_models_checked": len(log),
        "counterexample_instances_checked": len(instances) - initial_count,
    }


def verify_certificate(
    *,
    certificate_path: Path,
    problem_path: Path,
    repo_root: Path,
    drat_trim: Path,
) -> dict[str, Any]:
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    prior = load_module(
        "verify_instgen_008",
        repo_root
        / "experiments/2026-07-30-008-instgen-epr-prototype"
        / "verify_certificate.py",
    )
    base = prior.verify_certificate(
        certificate_path=certificate_path,
        problem_path=problem_path,
        repo_root=repo_root,
        drat_trim=drat_trim,
    )
    method = certificate.get("method")
    if method not in {"clausify", "ematch", "mbqi"}:
        raise VerificationError("unknown quantifier treatment")
    if certificate.get("semantic_sha256") != stable_json_sha256(
        semantic_payload(certificate)
    ):
        raise VerificationError("semantic hash mismatch")
    split_parser, term_parser = prior.load_parsers(repo_root)
    clauses, _ = prior.parse_source(
        problem_path.read_text(encoding="utf-8"),
        split_parser,
        term_parser,
    )
    method_result: dict[str, Any] = {}
    if method == "ematch":
        counterexample = certificate["method_data"].get(
            "first_ungenerated_counterexample"
        )
        if counterexample is not None:
            index = counterexample.get("source_index")
            substitution = counterexample.get("substitution")
            if not isinstance(index, int) or not isinstance(
                substitution, dict
            ):
                raise VerificationError("counterexample ancestry is malformed")
            expected = prior.ground_clause(clauses[index], substitution)
            if expected != record_clause(counterexample.get("ground_clause")):
                raise VerificationError(
                    "counterexample is not a source substitution"
                )
        method_result = verify_ematch(
            certificate=certificate, clauses=clauses
        )
    elif method == "mbqi":
        method_result = verify_mbqi(certificate)
    else:
        data = certificate.get("method_data")
        if (
            not isinstance(data, dict)
            or not isinstance(data.get("enumeration_complete"), bool)
        ):
            raise VerificationError("clausification metadata is malformed")
        if certificate["status"] == "sat" and not data["enumeration_complete"]:
            raise VerificationError("incomplete clausification claimed SAT")
    return {**base, "method": method, **method_result}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--certificate", type=Path, required=True)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    arguments = parser.parse_args()
    result = verify_certificate(
        certificate_path=arguments.certificate.resolve(),
        problem_path=arguments.problem.resolve(),
        repo_root=arguments.repo_root.resolve(),
        drat_trim=arguments.drat_trim.resolve(),
    )
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
