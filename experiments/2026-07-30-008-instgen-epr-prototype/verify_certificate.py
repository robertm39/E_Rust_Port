#!/usr/bin/env python3
"""Independent certificate replay for the bounded Inst-Gen-style worker."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import itertools
import json
import subprocess
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Iterator


class VerificationError(ValueError):
    """A candidate artifact fails the independent semantic contract."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_module(name: str, path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise VerificationError(f"cannot load parser {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def load_parsers(repo_root: Path) -> tuple[ModuleType, ModuleType]:
    split_parser = load_module(
        "instgen_verify_split",
        repo_root
        / "experiments/2026-07-29-008-avatar-restart-prototype/tptp_split.py",
    )
    term_parser = load_module(
        "instgen_verify_terms",
        repo_root
        / "experiments/2026-07-30-002-real-ground-theory-traces/trace_model.py",
    )
    return split_parser, term_parser


def is_variable(symbol: str) -> bool:
    return (
        not symbol.startswith(("'", '"'))
        and bool(symbol)
        and (symbol[0].isupper() or symbol[0] == "_")
    )


def atom_canonical(predicate: str, arguments: Iterable[str]) -> str:
    return f"{predicate}(" + ",".join(arguments) + ")"


def parse_source(
    text: str, split_parser: ModuleType, term_parser: ModuleType
) -> tuple[list[dict[str, Any]], list[str]]:
    clauses: list[dict[str, Any]] = []
    constants: set[str] = set()
    for index, statement in enumerate(split_parser.split_statements(text)):
        if statement.partition("(")[0].strip().lower() != "cnf":
            raise VerificationError("source is not pure CNF")
        parsed = split_parser.parse_cnf_statement(statement, index)
        literals = [
            term_parser.parse_literal(value) for value in parsed["literals"]
        ]
        variables: set[str] = set()
        for literal in literals:
            if literal.atom.relation == "eq":
                raise VerificationError("source contains equality")
            if literal.atom.relation.startswith("$") and literal.atom.relation not in {
                "$true",
                "$false",
            }:
                raise VerificationError("source contains an interpreted predicate")
            for term in literal.atom.arguments:
                if term.arguments:
                    raise VerificationError("source contains a function")
                if is_variable(term.symbol):
                    variables.add(term.symbol)
                else:
                    constants.add(term.symbol)
        clauses.append(
            {
                "index": index,
                "name": parsed["name"],
                "literals": literals,
                "variables": tuple(sorted(variables)),
            }
        )
    if not clauses:
        raise VerificationError("source has no clauses")
    if not constants:
        occupied = {
            literal.atom.relation
            for clause in clauses
            for literal in clause["literals"]
        }
        candidate = "instgen_default_constant"
        ordinal = 0
        while candidate in occupied:
            ordinal += 1
            candidate = f"instgen_default_constant_{ordinal}"
        constants.add(candidate)
    return clauses, sorted(constants)


def normalize_clause(
    literals: Iterable[tuple[str, bool]]
) -> tuple[tuple[str, bool], ...] | None:
    retained: set[tuple[str, bool]] = set()
    for atom, positive in literals:
        if atom == "$true()":
            if positive:
                return None
            continue
        if atom == "$false()":
            if not positive:
                return None
            continue
        if (atom, not positive) in retained:
            return None
        retained.add((atom, positive))
    return tuple(sorted(retained, key=lambda value: (value[0], not value[1])))


def ground_clause(
    clause: dict[str, Any],
    substitution: dict[str, str],
) -> tuple[tuple[str, bool], ...] | None:
    literals: list[tuple[str, bool]] = []
    for literal in clause["literals"]:
        arguments = [
            substitution.get(term.symbol, term.symbol)
            for term in literal.atom.arguments
        ]
        literals.append(
            (
                atom_canonical(literal.atom.relation, arguments),
                bool(literal.positive),
            )
        )
    return normalize_clause(literals)


def substitutions(
    variables: tuple[str, ...], constants: list[str]
) -> Iterator[dict[str, str]]:
    for values in itertools.product(constants, repeat=len(variables)):
        yield dict(zip(variables, values, strict=True))


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
    normalized = tuple(sorted(parsed, key=lambda value: (value[0], not value[1])))
    if tuple(parsed) != normalized or len(set(parsed)) != len(parsed):
        raise VerificationError("ground clause is not canonical")
    return normalized


def parse_dimacs(path: Path) -> tuple[int, list[tuple[int, ...]]]:
    variables = None
    declared_clauses = None
    clauses: list[tuple[int, ...]] = []
    pending: list[int] = []
    for raw_line in path.read_text(encoding="ascii").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("c"):
            continue
        if line.startswith("p "):
            fields = line.split()
            if len(fields) != 4 or fields[:2] != ["p", "cnf"]:
                raise VerificationError("malformed DIMACS header")
            variables = int(fields[2])
            declared_clauses = int(fields[3])
            continue
        for field in line.split():
            literal = int(field)
            if literal == 0:
                clauses.append(tuple(pending))
                pending.clear()
            else:
                pending.append(literal)
    if variables is None or declared_clauses is None or pending:
        raise VerificationError("incomplete DIMACS")
    if declared_clauses != len(clauses):
        raise VerificationError("DIMACS clause count mismatch")
    if any(abs(literal) > variables for clause in clauses for literal in clause):
        raise VerificationError("DIMACS literal exceeds variable bound")
    return variables, clauses


def verify_rendered_instances(
    path: Path,
    expected: list[tuple[tuple[str, bool], ...]],
    split_parser: ModuleType,
    term_parser: ModuleType,
) -> None:
    statements = split_parser.split_statements(path.read_text(encoding="utf-8"))
    if len(statements) != len(expected):
        raise VerificationError("rendered instance count mismatch")
    for index, (statement, expected_clause) in enumerate(
        zip(statements, expected, strict=True)
    ):
        parsed = split_parser.parse_cnf_statement(statement, index)
        if parsed["name"] != f"instgen_{index}":
            raise VerificationError("rendered instance name mismatch")
        literals = []
        for text in parsed["literals"]:
            literal = term_parser.parse_literal(text)
            if any(term.arguments or is_variable(term.symbol)
                   for term in literal.atom.arguments):
                raise VerificationError("rendered instance is not ground")
            literals.append(
                (
                    literal.atom.canonical(),
                    bool(literal.positive),
                )
            )
        if normalize_clause(literals) != expected_clause:
            raise VerificationError("rendered ground clause mismatch")


def drat_verified(completed: subprocess.CompletedProcess[str]) -> bool:
    return (
        completed.returncode == 0
        and "s VERIFIED" in completed.stdout + completed.stderr
    )


def verify_certificate(
    *,
    certificate_path: Path,
    problem_path: Path,
    repo_root: Path,
    drat_trim: Path,
) -> dict[str, Any]:
    certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    if certificate.get("schema_version") != 1:
        raise VerificationError("unsupported certificate schema")
    if certificate.get("source_sha256") != sha256_file(problem_path):
        raise VerificationError("source hash mismatch")
    split_parser, term_parser = load_parsers(repo_root)
    clauses, constants = parse_source(
        problem_path.read_text(encoding="utf-8"),
        split_parser,
        term_parser,
    )
    if certificate.get("domain_constants") != constants:
        raise VerificationError("domain constant mismatch")
    expected_ground_count = sum(
        len(constants) ** len(clause["variables"]) for clause in clauses
    )
    if certificate.get("ground_instance_count") != str(expected_ground_count):
        raise VerificationError("ground-instance count mismatch")
    if certificate.get("source_clauses") != len(clauses):
        raise VerificationError("source-clause count mismatch")

    instances = certificate.get("instances")
    if not isinstance(instances, list):
        raise VerificationError("instances are missing")
    replayed: list[tuple[tuple[str, bool], ...]] = []
    seen: set[tuple[tuple[str, bool], ...]] = set()
    for instance in instances:
        if not isinstance(instance, dict):
            raise VerificationError("malformed instance")
        index = instance.get("source_index")
        if not isinstance(index, int) or not 0 <= index < len(clauses):
            raise VerificationError("instance source index is invalid")
        source = clauses[index]
        if instance.get("source_name") != source["name"]:
            raise VerificationError("instance source name mismatch")
        substitution = instance.get("substitution")
        if (
            not isinstance(substitution, dict)
            or set(substitution) != set(source["variables"])
            or any(value not in constants for value in substitution.values())
        ):
            raise VerificationError("instance substitution is invalid")
        expected = ground_clause(source, substitution)
        if expected is None:
            raise VerificationError("tautological instance was retained")
        actual = record_clause(instance.get("ground_clause"))
        if actual != expected:
            raise VerificationError("ground instance is not a source substitution")
        if actual in seen:
            raise VerificationError("duplicate ground clause")
        seen.add(actual)
        replayed.append(actual)

    for source in clauses:
        initial = {
            variable: constants[0] for variable in source["variables"]
        }
        expected = ground_clause(source, initial)
        if expected is not None and expected not in seen:
            raise VerificationError("initial abstraction is incomplete")
    if certificate.get("generated_instances") != len(replayed):
        raise VerificationError("generated-instance count mismatch")
    if certificate.get("unique_ground_clauses") != len(replayed):
        raise VerificationError("unique-ground-clause count mismatch")

    output_root = certificate_path.parent
    rendered = output_root / str(certificate.get("instances_path"))
    dimacs = output_root / str(certificate.get("dimacs_path"))
    if (
        not rendered.is_file()
        or sha256_file(rendered) != certificate.get("instances_sha256")
    ):
        raise VerificationError("rendered-instance hash mismatch")
    if not dimacs.is_file() or sha256_file(dimacs) != certificate.get(
        "dimacs_sha256"
    ):
        raise VerificationError("DIMACS hash mismatch")
    verify_rendered_instances(
        rendered, replayed, split_parser, term_parser
    )

    raw_atom_map = certificate.get("atom_map")
    if not isinstance(raw_atom_map, dict):
        raise VerificationError("atom map is missing")
    atoms = sorted({atom for clause in replayed for atom, _ in clause})
    expected_atom_map = {atom: index + 1 for index, atom in enumerate(atoms)}
    if raw_atom_map != expected_atom_map:
        raise VerificationError("atom map mismatch")
    variables, dimacs_clauses = parse_dimacs(dimacs)
    expected_dimacs = [
        tuple(
            expected_atom_map[atom] if positive else -expected_atom_map[atom]
            for atom, positive in clause
        )
        for clause in replayed
    ]
    if variables != len(expected_atom_map) or dimacs_clauses != expected_dimacs:
        raise VerificationError("DIMACS does not encode the replayed instances")

    status = certificate.get("status")
    if status not in {"sat", "unsat", "unknown"}:
        raise VerificationError("invalid status")
    proof_checked = False
    model_checked = False
    if status == "unsat":
        proof = certificate.get("proof")
        if not isinstance(proof, dict):
            raise VerificationError("UNSAT certificate has no proof")
        proof_path = output_root / str(proof.get("proof_path"))
        if not proof_path.is_file() or sha256_file(proof_path) != proof.get(
            "proof_sha256"
        ):
            raise VerificationError("proof hash mismatch")
        checked = subprocess.run(
            [str(drat_trim), str(dimacs), str(proof_path)],
            check=False,
            capture_output=True,
            text=True,
            timeout=120,
        )
        if not drat_verified(checked):
            raise VerificationError("drat-trim rejected the proof")
        proof_checked = True
    elif status == "sat":
        true_atoms = certificate.get("true_atoms")
        if (
            not isinstance(true_atoms, list)
            or any(not isinstance(atom, str) for atom in true_atoms)
            or len(set(true_atoms)) != len(true_atoms)
            or any(atom not in expected_atom_map for atom in true_atoms)
        ):
            raise VerificationError("SAT model is malformed")
        true = set(true_atoms)
        enumerated = 0
        for source in clauses:
            for substitution in substitutions(source["variables"], constants):
                enumerated += 1
                ground = ground_clause(source, substitution)
                if ground is None:
                    continue
                if not any(
                    (atom in true) == positive for atom, positive in ground
                ):
                    raise VerificationError("SAT model falsifies a ground instance")
        if certificate.get("enumerated_substitutions", 0) < enumerated:
            raise VerificationError("SAT scan count is incomplete")
        model_checked = True
    elif certificate.get("proof") is not None:
        raise VerificationError("UNKNOWN certificate unexpectedly has a proof")

    return {
        "status": status,
        "instances_checked": len(replayed),
        "proof_checked": proof_checked,
        "model_checked": model_checked,
        "ground_instance_count": str(expected_ground_count),
    }


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
