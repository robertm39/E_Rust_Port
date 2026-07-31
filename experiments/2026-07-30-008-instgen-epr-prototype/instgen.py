#!/usr/bin/env python3
"""Bounded model-guided grounding for equality-free function-free CNF."""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import itertools
import json
import re
import subprocess
import time
from pathlib import Path
from typing import Any, Iterable, Iterator, Sequence

try:
    import resource
except ImportError:  # pragma: no cover - Windows development host
    resource = None  # type: ignore[assignment]


SCHEMA_VERSION = 1
MAX_BATCH = 64
TOKEN_RE = re.compile(
    r"""
    \s*
    (
        '(?:''|\\.|[^'])*'
      | "(?:""|\\.|[^"])*"
      | \$?[A-Za-z_][A-Za-z0-9_]*
      | [(),|~=]
    )
    """,
    re.VERBOSE,
)


class InstGenError(ValueError):
    """The input or a solver response violates the frozen contract."""


@dataclasses.dataclass(frozen=True, order=True)
class Atom:
    predicate: str
    arguments: tuple[str, ...]

    def canonical(self) -> str:
        return (
            f"{self.predicate}("
            + ",".join(self.arguments)
            + ")"
        )

    def render(self) -> str:
        if not self.arguments:
            return self.predicate
        return (
            f"{self.predicate}("
            + ",".join(self.arguments)
            + ")"
        )


@dataclasses.dataclass(frozen=True, order=True)
class Literal:
    atom: Atom
    positive: bool

    def render(self) -> str:
        return self.atom.render() if self.positive else f"~{self.atom.render()}"


@dataclasses.dataclass(frozen=True)
class Clause:
    index: int
    name: str
    role: str
    literals: tuple[Literal, ...]
    variables: tuple[str, ...]


@dataclasses.dataclass(frozen=True)
class Problem:
    source_text: str
    source_sha256: str
    clauses: tuple[Clause, ...]
    constants: tuple[str, ...]

    @property
    def ground_instance_count(self) -> int:
        return sum(
            len(self.constants) ** len(clause.variables)
            for clause in self.clauses
        )


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def strip_comments(text: str) -> str:
    output: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(text):
        character = text[index]
        if quote is not None:
            output.append(character)
            if character == quote:
                if index + 1 < len(text) and text[index + 1] == quote:
                    output.append(text[index + 1])
                    index += 2
                    continue
                quote = None
            elif character == "\\" and index + 1 < len(text):
                output.append(text[index + 1])
                index += 2
                continue
            index += 1
            continue
        if character in {"'", '"'}:
            quote = character
            output.append(character)
            index += 1
            continue
        if character == "%":
            while index < len(text) and text[index] not in "\r\n":
                output.append(" ")
                index += 1
            continue
        if text.startswith("/*", index):
            output.extend((" ", " "))
            index += 2
            while index < len(text) and not text.startswith("*/", index):
                output.append("\n" if text[index] == "\n" else " ")
                index += 1
            if index == len(text):
                raise InstGenError("unterminated block comment")
            output.extend((" ", " "))
            index += 2
            continue
        output.append(character)
        index += 1
    if quote is not None:
        raise InstGenError("unterminated quoted token")
    return "".join(output)


def split_top_level(value: str, separator: str) -> list[str]:
    pieces: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(value):
        character = value[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(value) and value[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in {"'", '"'}:
            quote = character
        elif character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
            if depth < 0:
                raise InstGenError("unbalanced expression")
        elif character == separator and depth == 0:
            pieces.append(value[start:index].strip())
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise InstGenError("unbalanced expression")
    pieces.append(value[start:].strip())
    return pieces


def split_statements(text: str) -> list[str]:
    clean = strip_comments(text)
    statements: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(clean):
        character = clean[index]
        if quote is not None:
            if character == quote:
                if index + 1 < len(clean) and clean[index + 1] == quote:
                    index += 2
                    continue
                quote = None
            elif character == "\\":
                index += 2
                continue
            index += 1
            continue
        if character in {"'", '"'}:
            quote = character
        elif character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth < 0:
                raise InstGenError("unbalanced closing parenthesis")
        elif character == "." and depth == 0:
            statement = clean[start : index + 1].strip()
            if statement:
                statements.append(statement)
            start = index + 1
        index += 1
    if quote is not None or depth != 0:
        raise InstGenError("unbalanced statement")
    if clean[start:].strip():
        raise InstGenError("trailing text without statement terminator")
    return statements


def strip_wrapping_parentheses(value: str) -> str:
    value = value.strip()
    while value.startswith("(") and value.endswith(")"):
        depth = 0
        quote: str | None = None
        closes_at_end = False
        for index, character in enumerate(value):
            if quote is not None:
                if character == quote:
                    quote = None
                elif character == "\\":
                    continue
                continue
            if character in {"'", '"'}:
                quote = character
            elif character == "(":
                depth += 1
            elif character == ")":
                depth -= 1
                if depth == 0:
                    closes_at_end = index == len(value) - 1
                    break
        if not closes_at_end:
            break
        value = value[1:-1].strip()
    return value


def tokenize(value: str) -> list[str]:
    tokens: list[str] = []
    position = 0
    while position < len(value):
        match = TOKEN_RE.match(value, position)
        if match is None:
            if value[position:].strip():
                raise InstGenError(
                    f"unsupported token near {value[position:position + 24]!r}"
                )
            break
        tokens.append(match.group(1))
        position = match.end()
    return tokens


def is_variable(symbol: str) -> bool:
    return (
        not symbol.startswith(("'", '"'))
        and bool(symbol)
        and (symbol[0].isupper() or symbol[0] == "_")
    )


def parse_atom(value: str) -> Atom:
    tokens = tokenize(value)
    if not tokens:
        raise InstGenError("empty atom")
    predicate = tokens[0]
    if predicate in {"(", ")", ",", "|", "~", "="}:
        raise InstGenError("invalid predicate")
    if len(tokens) == 1:
        return Atom(predicate, ())
    if len(tokens) < 4 or tokens[1] != "(" or tokens[-1] != ")":
        raise InstGenError("malformed predicate application")
    arguments: list[str] = []
    position = 2
    while position < len(tokens) - 1:
        argument = tokens[position]
        if argument in {"(", ")", ",", "|", "~", "="}:
            raise InstGenError("positive-arity function or malformed argument")
        arguments.append(argument)
        position += 1
        if position == len(tokens) - 1:
            break
        if tokens[position] != ",":
            raise InstGenError("positive-arity function or malformed argument")
        position += 1
    if not arguments:
        raise InstGenError("empty predicate argument list")
    return Atom(predicate, tuple(arguments))


def parse_literal(value: str) -> Literal:
    value = strip_wrapping_parentheses(value)
    positive = True
    while value.startswith("~"):
        positive = not positive
        value = strip_wrapping_parentheses(value[1:])
    tokens = tokenize(value)
    if "=" in tokens:
        raise InstGenError("equality is outside the fragment")
    atom = parse_atom(value)
    if atom.predicate.startswith("$") and atom.predicate not in {
        "$true",
        "$false",
    }:
        raise InstGenError("interpreted predicates are outside the fragment")
    return Literal(atom, positive)


def parse_problem(text: str) -> Problem:
    clauses: list[Clause] = []
    constants: set[str] = set()
    for index, statement in enumerate(split_statements(text)):
        prefix, separator, remainder = statement.partition("(")
        if not separator or prefix.strip().lower() != "cnf":
            raise InstGenError("only CNF statements are supported")
        if not remainder.rstrip().endswith(")."):
            raise InstGenError("malformed CNF statement")
        fields = split_top_level(remainder.rstrip()[:-2], ",")
        if len(fields) < 3:
            raise InstGenError("CNF statement has fewer than three fields")
        body = strip_wrapping_parentheses(fields[2])
        literals = tuple(
            parse_literal(piece) for piece in split_top_level(body, "|")
        )
        if not literals:
            raise InstGenError("empty source clause")
        variables = sorted(
            {
                argument
                for literal in literals
                for argument in literal.atom.arguments
                if is_variable(argument)
            }
        )
        constants.update(
            argument
            for literal in literals
            for argument in literal.atom.arguments
            if not is_variable(argument)
        )
        clauses.append(
            Clause(
                index=index,
                name=fields[0].strip(),
                role=fields[1].strip(),
                literals=literals,
                variables=tuple(variables),
            )
        )
    if not clauses:
        raise InstGenError("problem has no clauses")
    if not constants:
        occupied = {
            literal.atom.predicate
            for clause in clauses
            for literal in clause.literals
        }
        candidate = "instgen_default_constant"
        ordinal = 0
        while candidate in occupied:
            ordinal += 1
            candidate = f"instgen_default_constant_{ordinal}"
        constants.add(candidate)
    return Problem(
        source_text=text,
        source_sha256=sha256_bytes(text.encode("utf-8")),
        clauses=tuple(clauses),
        constants=tuple(sorted(constants)),
    )


def ground_literal(
    literal: Literal, substitution: dict[str, str]
) -> Literal:
    return Literal(
        Atom(
            literal.atom.predicate,
            tuple(substitution.get(value, value) for value in literal.atom.arguments),
        ),
        literal.positive,
    )


def normalize_ground_clause(
    literals: Iterable[Literal],
) -> tuple[Literal, ...] | None:
    retained: set[Literal] = set()
    for literal in literals:
        if literal.atom.predicate == "$true" and not literal.atom.arguments:
            if literal.positive:
                return None
            continue
        if literal.atom.predicate == "$false" and not literal.atom.arguments:
            if not literal.positive:
                return None
            continue
        complement = Literal(literal.atom, not literal.positive)
        if complement in retained:
            return None
        retained.add(literal)
    return tuple(
        sorted(
            retained,
            key=lambda literal: (
                literal.atom.canonical(),
                not literal.positive,
            ),
        )
    )


def ground_clause(
    clause: Clause, substitution: dict[str, str]
) -> tuple[Literal, ...] | None:
    return normalize_ground_clause(
        ground_literal(literal, substitution) for literal in clause.literals
    )


def clause_key(clause: tuple[Literal, ...]) -> tuple[tuple[str, bool], ...]:
    return tuple(
        (literal.atom.canonical(), literal.positive) for literal in clause
    )


def clause_record(clause: tuple[Literal, ...]) -> list[dict[str, Any]]:
    return [
        {
            "atom": literal.atom.canonical(),
            "positive": literal.positive,
        }
        for literal in clause
    ]


def substitutions(
    clause: Clause, constants: Sequence[str]
) -> Iterator[dict[str, str]]:
    for values in itertools.product(constants, repeat=len(clause.variables)):
        yield dict(zip(clause.variables, values, strict=True))


def ground_clause_is_false(
    clause: tuple[Literal, ...], model: dict[Atom, bool]
) -> bool:
    return all(
        model.get(literal.atom, False) != literal.positive
        for literal in clause
    )


def add_instance(
    *,
    clause: Clause,
    substitution: dict[str, str],
    ground: tuple[Literal, ...],
    known: set[tuple[tuple[str, bool], ...]],
    instances: list[dict[str, Any]],
    ground_clauses: list[tuple[Literal, ...]],
    phase: str,
    iteration: int,
) -> bool:
    key = clause_key(ground)
    if key in known:
        return False
    known.add(key)
    ground_clauses.append(ground)
    instances.append(
        {
            "source_index": clause.index,
            "source_name": clause.name,
            "substitution": dict(sorted(substitution.items())),
            "ground_clause": clause_record(ground),
            "phase": phase,
            "iteration": iteration,
        }
    )
    return True


def atom_map(
    clauses: Iterable[tuple[Literal, ...]]
) -> dict[Atom, int]:
    atoms = sorted(
        {literal.atom for clause in clauses for literal in clause},
        key=Atom.canonical,
    )
    return {atom: index + 1 for index, atom in enumerate(atoms)}


def write_dimacs(
    path: Path,
    clauses: Sequence[tuple[Literal, ...]],
    mapping: dict[Atom, int],
) -> None:
    lines = [f"p cnf {len(mapping)} {len(clauses)}"]
    for clause in clauses:
        literals = [
            mapping[literal.atom] if literal.positive else -mapping[literal.atom]
            for literal in clause
        ]
        lines.append(" ".join((*map(str, literals), "0")))
    path.write_text("\n".join(lines) + "\n", encoding="ascii", newline="\n")


def solve_dimacs(
    adapter: Path, dimacs: Path, timeout_seconds: float
) -> dict[str, Any]:
    try:
        completed = subprocess.run(
            [str(adapter), str(dimacs)],
            check=False,
            capture_output=True,
            text=True,
            timeout=max(0.001, timeout_seconds),
        )
    except subprocess.TimeoutExpired:
        return {"status": "unknown", "reason": "solver_timeout"}
    if completed.returncode != 0:
        raise InstGenError(
            "CaDiCaL adapter failed: "
            + (completed.stdout + completed.stderr)[-2000:]
        )
    try:
        result = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise InstGenError("CaDiCaL adapter returned invalid JSON") from error
    if result.get("status") not in {"sat", "unsat", "unknown"}:
        raise InstGenError("CaDiCaL adapter returned an invalid status")
    return result


def model_from_result(
    result: dict[str, Any], mapping: dict[Atom, int]
) -> dict[Atom, bool]:
    raw_model = result.get("model")
    if not isinstance(raw_model, list):
        raise InstGenError("SAT result has no model")
    by_variable = {abs(int(value)): int(value) > 0 for value in raw_model}
    if set(by_variable) != set(mapping.values()):
        raise InstGenError("SAT result is not a complete model")
    return {atom: by_variable[variable] for atom, variable in mapping.items()}


def render_instances(instances: Sequence[dict[str, Any]]) -> str:
    lines = [
        "% Replayable ground instances from the bounded Inst-Gen-style worker.",
    ]
    for index, instance in enumerate(instances):
        literals = [
            (
                literal["atom"][:-2]
                if literal["atom"].endswith("()")
                else literal["atom"]
            )
            for literal in instance["ground_clause"]
        ]
        literals = [
            value if literal["positive"] else f"~{value}"
            for value, literal in zip(
                literals, instance["ground_clause"], strict=True
            )
        ]
        body = " | ".join(literals) if literals else "$false"
        lines.append(f"cnf(instgen_{index}, plain, ({body})).")
    return "\n".join(lines) + "\n"


def drat_verified(completed: subprocess.CompletedProcess[str]) -> bool:
    combined = completed.stdout + completed.stderr
    return completed.returncode == 0 and "s VERIFIED" in combined


def certify_unsat(
    *,
    adapter: Path,
    drat_trim: Path,
    dimacs: Path,
    output_root: Path,
) -> dict[str, Any]:
    proof = output_root / "proof.drat"
    started = time.monotonic_ns()
    solved = subprocess.run(
        [str(adapter), str(dimacs), str(proof)],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if solved.returncode != 0:
        raise InstGenError("proof-producing CaDiCaL run failed")
    response = json.loads(solved.stdout)
    if response.get("status") != "unsat":
        raise InstGenError("proof-producing rerun did not reproduce UNSAT")
    checked = subprocess.run(
        [str(drat_trim), str(dimacs), str(proof)],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    checker_stdout = output_root / "drat-trim.stdout.txt"
    checker_stderr = output_root / "drat-trim.stderr.txt"
    checker_stdout.write_text(checked.stdout, encoding="utf-8")
    checker_stderr.write_text(checked.stderr, encoding="utf-8")
    if not drat_verified(checked):
        raise InstGenError("drat-trim rejected the final proof")
    return {
        "proof_path": proof.name,
        "proof_sha256": sha256_file(proof),
        "proof_bytes": proof.stat().st_size,
        "checker_ns": time.monotonic_ns() - started,
        "checker_stdout_sha256": sha256_file(checker_stdout),
        "checker_stderr_sha256": sha256_file(checker_stderr),
    }


def resource_snapshot() -> tuple[float, float, int]:
    if resource is None:
        return 0.0, 0.0, 0
    own = resource.getrusage(resource.RUSAGE_SELF)
    children = resource.getrusage(resource.RUSAGE_CHILDREN)
    return (
        own.ru_utime + children.ru_utime,
        own.ru_stime + children.ru_stime,
        max(int(own.ru_maxrss), int(children.ru_maxrss)),
    )


def run(
    *,
    problem_path: Path,
    adapter: Path,
    drat_trim: Path,
    output_root: Path,
    budget_seconds: float,
) -> dict[str, Any]:
    if budget_seconds <= 0:
        raise InstGenError("budget must be positive")
    output_root.mkdir(parents=True, exist_ok=True)
    source_bytes = problem_path.read_bytes()
    source_text = source_bytes.decode("utf-8")
    problem = parse_problem(source_text)
    if problem.source_sha256 != sha256_bytes(source_bytes):
        raise InstGenError("source must be UTF-8 without transcoding")

    known: set[tuple[tuple[str, bool], ...]] = set()
    instances: list[dict[str, Any]] = []
    ground_clauses: list[tuple[Literal, ...]] = []
    for clause in problem.clauses:
        substitution = {
            variable: problem.constants[0] for variable in clause.variables
        }
        ground = ground_clause(clause, substitution)
        if ground is not None:
            add_instance(
                clause=clause,
                substitution=substitution,
                ground=ground,
                known=known,
                instances=instances,
                ground_clauses=ground_clauses,
                phase="initial",
                iteration=0,
            )

    initial_user, initial_system, _ = resource_snapshot()
    started = time.monotonic()
    deadline = started + budget_seconds
    status = "unknown"
    reason = "wall_limit"
    sat_calls = 0
    sat_ns = 0
    refinements = 0
    enumerated_substitutions = 0
    final_model: dict[Atom, bool] = {}
    dimacs = output_root / "final.cnf"

    while time.monotonic() < deadline:
        mapping = atom_map(ground_clauses)
        write_dimacs(dimacs, ground_clauses, mapping)
        result = solve_dimacs(adapter, dimacs, deadline - time.monotonic())
        sat_calls += 1
        sat_ns += int(result.get("solve_ns", 0))
        if result["status"] == "unknown":
            reason = str(result.get("reason", "solver_unknown"))
            break
        if result["status"] == "unsat":
            status = "unsat"
            reason = "ground_abstraction_unsat"
            break

        model = model_from_result(result, mapping)
        final_model = model
        added = 0
        scan_complete = True
        solved_clause_keys = set(known)
        for clause in problem.clauses:
            for substitution in substitutions(clause, problem.constants):
                enumerated_substitutions += 1
                if (enumerated_substitutions & 255) == 0 and time.monotonic() >= deadline:
                    scan_complete = False
                    reason = "wall_limit_during_counterexample_scan"
                    break
                ground = ground_clause(clause, substitution)
                if ground is None:
                    continue
                if ground_clause_is_false(ground, model):
                    key = clause_key(ground)
                    if key in solved_clause_keys:
                        raise InstGenError(
                            "current SAT model falsifies a solved clause"
                        )
                    if key in known:
                        # A preceding source clause produced the same false
                        # instance in this refinement batch. The next SAT call
                        # will account for it; retaining duplicate ancestry is
                        # unnecessary.
                        break
                    if not add_instance(
                        clause=clause,
                        substitution=substitution,
                        ground=ground,
                        known=known,
                        instances=instances,
                        ground_clauses=ground_clauses,
                        phase="refinement",
                        iteration=sat_calls,
                    ):
                        raise InstGenError("new ground instance was not added")
                    added += 1
                    break
            if not scan_complete or added >= MAX_BATCH:
                break
        if not scan_complete:
            break
        if added == 0:
            status = "sat"
            reason = "complete_herbrand_model"
            break
        refinements += 1

    mapping = atom_map(ground_clauses)
    write_dimacs(dimacs, ground_clauses, mapping)
    instance_path = output_root / "instances.p"
    instance_path.write_text(
        render_instances(instances), encoding="utf-8", newline="\n"
    )
    search_wall = time.monotonic() - started
    final_user, final_system, final_rss = resource_snapshot()
    certificate: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "source_path": str(problem_path),
        "source_sha256": problem.source_sha256,
        "status": status,
        "termination_reason": reason,
        "budget_seconds": budget_seconds,
        "search_wall_seconds": search_wall,
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
            atom.canonical() for atom, value in final_model.items() if value
        ),
        "dimacs_path": dimacs.name,
        "dimacs_sha256": sha256_file(dimacs),
        "dimacs_bytes": dimacs.stat().st_size,
        "instances_path": instance_path.name,
        "instances_sha256": sha256_file(instance_path),
        "instances_bytes": instance_path.stat().st_size,
        "proof": None,
    }
    if status == "unsat":
        certificate["proof"] = certify_unsat(
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--budget-seconds", type=float, required=True)
    arguments = parser.parse_args()
    result = run(
        problem_path=arguments.problem.resolve(),
        adapter=arguments.cadical_driver.resolve(),
        drat_trim=arguments.drat_trim.resolve(),
        output_root=arguments.output_root.resolve(),
        budget_seconds=arguments.budget_seconds,
    )
    print(
        json.dumps(
            {
                key: result[key]
                for key in (
                    "status",
                    "termination_reason",
                    "sat_calls",
                    "refinement_iterations",
                    "generated_instances",
                    "enumerated_substitutions",
                    "search_wall_seconds",
                )
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
