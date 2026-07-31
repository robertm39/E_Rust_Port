#!/usr/bin/env python3
"""Shared immutable inputs and CNF parsing for the connection experiment."""

from __future__ import annotations

import dataclasses
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


SCHEMA = "umlaut.connection-tableau.v1"
SOURCE_REVISION = "b80150e336b8c2da7b2d5fcefbd01cf71f7001c5"
CONNECTION_BUDGET_SECONDS = 5.0
MAX_BRANCH_DEPTH = 12
MAX_SEARCH_NODES = 500_000
SATURATION_SOFT_SECONDS = 5
SATURATION_HARD_SECONDS = 7
MEMORY_MIB = 1_536
METHODS = ("connection", "global_aw", "goal_hard_priority")
REPETITIONS = {"train": 1, "validation": 2, "test": 2}
PROOF_STATUSES = frozenset({"Theorem", "Unsatisfiable", "ContradictoryAxioms"})
NO_CLAIM_STATUSES = frozenset(
    {
        "GaveUp",
        "Inappropriate",
        "InputError",
        "MemoryOut",
        "NoSuccess",
        "OutputError",
        "ResourceOut",
        "Timeout",
        "Unknown",
    }
)
SZS_RE = re.compile(
    r"(?:%|#)\s*SZS status\s+([A-Za-z_]+)", re.IGNORECASE
)
ANNOTATED_FORMULA_RE = re.compile(
    r"^\s*(?:cnf|fof|tff|tcf|thf)\s*\(", re.MULTILINE | re.IGNORECASE
)


class ExperimentError(RuntimeError):
    """A frozen experiment invariant was violated."""


@dataclasses.dataclass(frozen=True)
class Term:
    """One first-order term parsed from Umlaut's CNF output."""

    symbol: str
    arguments: tuple["Term", ...] = ()

    def canonical(self) -> str:
        if not self.arguments:
            return self.symbol
        return (
            f"{self.symbol}("
            + ",".join(argument.canonical() for argument in self.arguments)
            + ")"
        )


@dataclasses.dataclass(frozen=True)
class Literal:
    """One equality-free first-order literal."""

    predicate: str
    arguments: tuple[Term, ...]
    positive: bool

    def canonical(self) -> str:
        atom = self.predicate
        if self.arguments:
            atom += (
                "("
                + ",".join(argument.canonical() for argument in self.arguments)
                + ")"
            )
        return atom if self.positive else f"~{atom}"

    def index_key(self) -> tuple[str, int, bool]:
        return (self.predicate, len(self.arguments), self.positive)


@dataclasses.dataclass(frozen=True)
class Clause:
    """One parsed source clause with immutable transcript provenance."""

    index: int
    name: str
    role: str
    literals: tuple[Literal, ...]
    statement_sha256: str


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_bytes(canonical_json(value) + b"\n")
    temporary.replace(path)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"".join(canonical_json(row) + b"\n" for row in rows))


def load_corpus(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = read_jsonl(path)
    if (
        not rows
        or rows[0].get("kind") != "umlaut-complementary-connection-corpus"
        or rows[0].get("source_revision") != SOURCE_REVISION
    ):
        raise ExperimentError("corpus header violates the frozen contract")
    problems = rows[1:]
    if len(problems) != rows[0].get("problem_count") or len(problems) != 12:
        raise ExperimentError("corpus must contain exactly 12 problem records")
    expected = {"train": 4, "validation": 4, "test": 4}
    observed = {
        split: sum(record.get("experiment_split") == split for record in problems)
        for split in expected
    }
    if observed != expected:
        raise ExperimentError(f"corpus split mismatch: {observed}")
    families = {
        split: {
            str(record["family"])
            for record in problems
            if record["experiment_split"] == split
        }
        for split in expected
    }
    if any(
        families[left] & families[right]
        for left in families
        for right in families
        if left < right
    ):
        raise ExperimentError("corpus families are not split-disjoint")
    for record in problems:
        if (
            record.get("category") != "FNE"
            or record.get("division") != "FOF"
            or record.get("expected_class") != "theorem"
        ):
            raise ExperimentError(f"out-of-scope corpus row: {record}")
    return rows[0], problems


def verify_problem_record(
    problem_root: Path, record: dict[str, Any]
) -> tuple[Path, dict[str, str]]:
    problem = problem_root / str(record["path"])
    if not problem.is_file():
        raise ExperimentError(f"missing problem: {problem}")
    observed = sha256_file(problem)
    if observed != record["sha256"]:
        raise ExperimentError(
            f"problem hash mismatch for {record['problem_id']}: {observed}"
        )
    include_hashes: dict[str, str] = {}
    expected_includes = record.get("include_sha256", {})
    if set(expected_includes) != set(record.get("includes", [])):
        raise ExperimentError(
            f"include lock mismatch for {record['problem_id']}"
        )
    for include, expected_hash in expected_includes.items():
        include_path = problem_root / "problems" / "casc_2025" / include
        if not include_path.is_file():
            raise ExperimentError(f"missing include: {include_path}")
        include_hashes[include] = sha256_file(include_path)
        if include_hashes[include] != expected_hash:
            raise ExperimentError(
                f"include hash mismatch for {include}: {include_hashes[include]}"
            )
    return problem, include_hashes


def load_module(name: str, path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise ExperimentError(f"cannot import parser module: {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def load_trace_parser(repo_root: Path, module_name: str) -> ModuleType:
    return load_module(
        module_name,
        repo_root
        / "experiments"
        / "2026-07-30-002-real-ground-theory-traces"
        / "trace_model.py",
    )


def convert_term(value: Any) -> Term:
    return Term(
        str(value.symbol),
        tuple(convert_term(argument) for argument in value.arguments),
    )


def parse_cnf_transcript(
    text: str, *, repo_root: Path, module_name: str
) -> list[Clause]:
    parser = load_trace_parser(repo_root, module_name)
    clauses: list[Clause] = []
    for statement in parser.split_statements(text):
        prefix, fields = parser.statement_fields(statement)
        if prefix not in {"cnf", "tcf"}:
            continue
        if len(fields) < 3:
            raise ExperimentError("CNF statement has fewer than three fields")
        _sorts, literal_texts = parser.parse_quantified_clause(fields[2])
        literals: list[Literal] = []
        tautology = False
        for literal_text in literal_texts:
            parsed = parser.parse_literal(literal_text)
            if parsed.atom.relation == "eq":
                raise ExperimentError("equality is outside the frozen FNE calculus")
            literal = Literal(
                str(parsed.atom.relation),
                tuple(convert_term(term) for term in parsed.atom.arguments),
                bool(parsed.positive),
            )
            if (
                literal.predicate == "$true" and literal.positive
            ) or (
                literal.predicate == "$false" and not literal.positive
            ):
                tautology = True
                break
            if (
                literal.predicate == "$false" and literal.positive
            ) or (
                literal.predicate == "$true" and not literal.positive
            ):
                continue
            literals.append(literal)
        if tautology:
            continue
        clauses.append(
            Clause(
                index=len(clauses),
                name=fields[0].strip(),
                role=fields[1].strip(),
                literals=tuple(literals),
                statement_sha256=sha256_bytes(statement.encode("utf-8")),
            )
        )
    if not clauses:
        raise ExperimentError("clausifier emitted no supported clauses")
    if not any(clause.role == "negated_conjecture" for clause in clauses):
        raise ExperimentError("clausifier emitted no negated-conjecture clause")
    return clauses


def matrix_digest(clauses: Sequence[Clause]) -> str:
    rows = [
        {
            "index": clause.index,
            "name": clause.name,
            "role": clause.role,
            "literals": [literal.canonical() for literal in clause.literals],
            "statement_sha256": clause.statement_sha256,
        }
        for clause in clauses
    ]
    return sha256_bytes(canonical_json(rows))


def clausifier_command(binary: Path, problem: Path) -> list[str]:
    return [
        str(binary),
        "--cnf",
        "--no-preprocessing",
        "--tstp-format",
        str(problem),
    ]


def run_clausifier(
    *,
    binary: Path,
    problem: Path,
    tptp_root: Path,
    timeout: float,
) -> subprocess.CompletedProcess[bytes]:
    environment = os.environ.copy()
    environment["TPTP"] = str(tptp_root)
    try:
        completed = subprocess.run(
            clausifier_command(binary, problem),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        raise ExperimentError("clausification exceeded its enclosing deadline") from error
    if completed.returncode != 0:
        detail = (completed.stdout + completed.stderr)[-4_000:].decode(
            "utf-8", errors="replace"
        )
        raise ExperimentError(f"clausifier failed: {detail}")
    return completed


def count_annotated_formulas(text: str) -> int:
    return len(ANNOTATED_FORMULA_RE.findall(text))


def final_status(*texts: str) -> str | None:
    statuses: list[str] = []
    for text in texts:
        statuses.extend(SZS_RE.findall(text))
    return statuses[-1] if statuses else None
