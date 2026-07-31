#!/usr/bin/env python3
"""Bounded equality-free connection-tableau worker."""

from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence

import connection_common as common

try:
    import resource
except ModuleNotFoundError:  # pragma: no cover - worker executions are Linux-only.
    resource = None  # type: ignore[assignment]


class SearchStopped(RuntimeError):
    """The global deadline or search-node bound was reached."""


def is_variable(term: common.Term) -> bool:
    return (
        not term.arguments
        and bool(term.symbol)
        and (term.symbol[0].isupper() or term.symbol[0] == "_")
    )


def dereference(
    term: common.Term, substitution: dict[str, common.Term]
) -> common.Term:
    seen: set[str] = set()
    while is_variable(term) and term.symbol in substitution:
        if term.symbol in seen:
            raise common.ExperimentError("cyclic substitution")
        seen.add(term.symbol)
        term = substitution[term.symbol]
    return term


def apply_term(
    term: common.Term, substitution: dict[str, common.Term]
) -> common.Term:
    term = dereference(term, substitution)
    if not term.arguments:
        return term
    return common.Term(
        term.symbol,
        tuple(apply_term(argument, substitution) for argument in term.arguments),
    )


def apply_literal(
    literal: common.Literal, substitution: dict[str, common.Term]
) -> common.Literal:
    return common.Literal(
        literal.predicate,
        tuple(apply_term(argument, substitution) for argument in literal.arguments),
        literal.positive,
    )


def occurs(
    variable: str,
    term: common.Term,
    substitution: dict[str, common.Term],
) -> bool:
    term = dereference(term, substitution)
    if is_variable(term):
        return term.symbol == variable
    return any(occurs(variable, argument, substitution) for argument in term.arguments)


def unify_terms(
    left: common.Term,
    right: common.Term,
    substitution: dict[str, common.Term],
) -> bool:
    pending = [(left, right)]
    while pending:
        first, second = pending.pop()
        first = dereference(first, substitution)
        second = dereference(second, substitution)
        if first == second:
            continue
        if is_variable(first):
            if occurs(first.symbol, second, substitution):
                return False
            substitution[first.symbol] = second
            continue
        if is_variable(second):
            if occurs(second.symbol, first, substitution):
                return False
            substitution[second.symbol] = first
            continue
        if (
            first.symbol != second.symbol
            or len(first.arguments) != len(second.arguments)
        ):
            return False
        pending.extend(zip(first.arguments, second.arguments, strict=True))
    return True


def unify_atoms(
    left: common.Literal,
    right: common.Literal,
    substitution: dict[str, common.Term],
) -> bool:
    if (
        left.predicate != right.predicate
        or len(left.arguments) != len(right.arguments)
    ):
        return False
    return all(
        unify_terms(first, second, substitution)
        for first, second in zip(left.arguments, right.arguments, strict=True)
    )


def fresh_term(term: common.Term, instance_id: int) -> common.Term:
    if is_variable(term):
        return common.Term(f"V__{instance_id}__{term.symbol}")
    return common.Term(
        term.symbol,
        tuple(fresh_term(argument, instance_id) for argument in term.arguments),
    )


def fresh_literal(
    literal: common.Literal, instance_id: int
) -> common.Literal:
    return common.Literal(
        literal.predicate,
        tuple(fresh_term(argument, instance_id) for argument in literal.arguments),
        literal.positive,
    )


def proof_rule_counts(proof: dict[str, Any]) -> dict[str, int]:
    counts = {"extension": 0, "reduction": 0}

    def visit(node: dict[str, Any]) -> None:
        kind = node["kind"]
        if kind == "closed":
            return
        counts[kind] += 1
        if kind == "extension":
            visit(node["branch"])
        visit(node["continuation"])

    visit(proof)
    return counts


@dataclass
class SearchContext:
    clauses: Sequence[common.Clause]
    deadline: float
    maximum_nodes: int

    def __post_init__(self) -> None:
        self.search_nodes = 0
        self.unification_attempts = 0
        self.iterations_completed = 0
        self.deepest_iteration = -1
        self.next_instance = 1
        self.stop_reason: str | None = None
        self.extension_index: dict[
            tuple[str, int, bool], list[tuple[int, int]]
        ] = {}
        for clause in self.clauses:
            for literal_index, literal in enumerate(clause.literals):
                self.extension_index.setdefault(
                    literal.index_key(), []
                ).append((clause.index, literal_index))
        for candidates in self.extension_index.values():
            candidates.sort(
                key=lambda item: (
                    len(self.clauses[item[0]].literals) - 1,
                    self.clauses[item[0]].role == "negated_conjecture",
                    item[0],
                    item[1],
                )
            )

    def checkpoint(self) -> None:
        if time.monotonic() >= self.deadline:
            self.stop_reason = "deadline"
            raise SearchStopped("connection search reached its wall deadline")
        self.search_nodes += 1
        if self.search_nodes > self.maximum_nodes:
            self.stop_reason = "node_limit"
            raise SearchStopped("connection search reached its node limit")

    def choose_goal(
        self,
        goals: Sequence[common.Literal],
        substitution: dict[str, common.Term],
    ) -> int:
        def score(item: tuple[int, common.Literal]) -> tuple[int, int, int]:
            index, literal = item
            candidate_count = len(
                self.extension_index.get(
                    (
                        literal.predicate,
                        len(literal.arguments),
                        not literal.positive,
                    ),
                    (),
                )
            )
            size = len(apply_literal(literal, substitution).canonical())
            return (candidate_count, size, index)

        return min(enumerate(goals), key=score)[0]

    def prove(
        self,
        goals: Sequence[common.Literal],
        path: tuple[common.Literal, ...],
        substitution: dict[str, common.Term],
        depth_left: int,
    ) -> tuple[dict[str, Any], dict[str, common.Term]] | None:
        self.checkpoint()
        if not goals:
            return {"kind": "closed"}, substitution

        goal_index = self.choose_goal(goals, substitution)
        goal = goals[goal_index]
        remaining = tuple(goals[:goal_index]) + tuple(goals[goal_index + 1 :])
        applied_goal = apply_literal(goal, substitution)
        if any(apply_literal(item, substitution) == applied_goal for item in path):
            return None

        for path_index, ancestor in enumerate(path):
            if (
                ancestor.positive == goal.positive
                or ancestor.predicate != goal.predicate
                or len(ancestor.arguments) != len(goal.arguments)
            ):
                continue
            self.unification_attempts += 1
            candidate_substitution = substitution.copy()
            if not unify_atoms(goal, ancestor, candidate_substitution):
                continue
            continuation = self.prove(
                remaining, path, candidate_substitution, depth_left
            )
            if continuation is None:
                continue
            continuation_proof, final_substitution = continuation
            return (
                {
                    "kind": "reduction",
                    "goal_index": goal_index,
                    "goal": applied_goal.canonical(),
                    "path_index": path_index,
                    "continuation": continuation_proof,
                },
                final_substitution,
            )

        if depth_left <= 0:
            return None
        key = (goal.predicate, len(goal.arguments), not goal.positive)
        for clause_index, literal_index in self.extension_index.get(key, ()):
            self.checkpoint()
            instance_id = self.next_instance
            self.next_instance += 1
            source = self.clauses[clause_index]
            fresh = tuple(
                fresh_literal(literal, instance_id) for literal in source.literals
            )
            connector = fresh[literal_index]
            self.unification_attempts += 1
            candidate_substitution = substitution.copy()
            if not unify_atoms(goal, connector, candidate_substitution):
                continue
            branch_goals = (
                fresh[:literal_index] + fresh[literal_index + 1 :]
            )
            branch = self.prove(
                branch_goals,
                path + (goal,),
                candidate_substitution,
                depth_left - 1,
            )
            if branch is None:
                continue
            branch_proof, branch_substitution = branch
            continuation = self.prove(
                remaining, path, branch_substitution, depth_left
            )
            if continuation is None:
                continue
            continuation_proof, final_substitution = continuation
            return (
                {
                    "kind": "extension",
                    "goal_index": goal_index,
                    "goal": applied_goal.canonical(),
                    "clause_index": clause_index,
                    "literal_index": literal_index,
                    "instance_id": instance_id,
                    "branch": branch_proof,
                    "continuation": continuation_proof,
                },
                final_substitution,
            )
        return None

    def search(self) -> tuple[int, int, dict[str, Any]] | None:
        starts = [
            clause
            for clause in self.clauses
            if clause.role == "negated_conjecture"
        ]
        starts.sort(key=lambda clause: (len(clause.literals), clause.index))
        for depth in range(common.MAX_BRANCH_DEPTH + 1):
            self.deepest_iteration = depth
            for start in starts:
                self.checkpoint()
                instance_id = self.next_instance
                self.next_instance += 1
                goals = tuple(
                    fresh_literal(literal, instance_id)
                    for literal in start.literals
                )
                result = self.prove(goals, (), {}, depth)
                if result is not None:
                    proof, _substitution = result
                    return start.index, instance_id, proof
            self.iterations_completed += 1
        self.stop_reason = "depth_limit"
        return None


def make_unknown_certificate(
    *,
    problem: Path,
    binary: Path,
    started: float,
    failure: str,
    transcript_path: Path | None = None,
    transcript_sha256: str | None = None,
    matrix_sha256: str | None = None,
    clause_count: int | None = None,
    metrics: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "schema": common.SCHEMA,
        "schema_version": 1,
        "status": "Unknown",
        "problem_path": str(problem),
        "problem_sha256": common.sha256_file(problem),
        "binary_sha256": common.sha256_file(binary),
        "transcript_path": str(transcript_path) if transcript_path else None,
        "transcript_sha256": transcript_sha256,
        "matrix_sha256": matrix_sha256,
        "clause_count": clause_count,
        "failure": failure,
        "wall_seconds": time.monotonic() - started,
        "metrics": metrics or {},
        "limits": {
            "budget_seconds": common.CONNECTION_BUDGET_SECONDS,
            "maximum_branch_depth": common.MAX_BRANCH_DEPTH,
            "maximum_search_nodes": common.MAX_SEARCH_NODES,
        },
    }


def run_worker(
    *,
    repo_root: Path,
    binary: Path,
    problem: Path,
    tptp_root: Path,
    output_root: Path,
) -> dict[str, Any]:
    output_root.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    deadline = started + common.CONNECTION_BUDGET_SECONDS
    transcript_path = output_root / "cnf.tstp"
    stderr_path = output_root / "clausifier.stderr.txt"
    try:
        completed = common.run_clausifier(
            binary=binary,
            problem=problem,
            tptp_root=tptp_root,
            timeout=max(0.01, deadline - time.monotonic()),
        )
        transcript_path.write_bytes(completed.stdout)
        stderr_path.write_bytes(completed.stderr)
        transcript_text = completed.stdout.decode("utf-8")
        clauses = common.parse_cnf_transcript(
            transcript_text,
            repo_root=repo_root,
            module_name="connection_worker_trace_parser",
        )
        transcript_sha256 = common.sha256_file(transcript_path)
        matrix_sha256 = common.matrix_digest(clauses)
    except (common.ExperimentError, UnicodeError) as error:
        return make_unknown_certificate(
            problem=problem,
            binary=binary,
            started=started,
            failure=f"clausification_or_parse:{type(error).__name__}:{error}",
            transcript_path=transcript_path if transcript_path.exists() else None,
            transcript_sha256=(
                common.sha256_file(transcript_path)
                if transcript_path.exists()
                else None
            ),
        )

    context = SearchContext(
        clauses=clauses,
        deadline=deadline,
        maximum_nodes=common.MAX_SEARCH_NODES,
    )
    found: tuple[int, int, dict[str, Any]] | None = None
    failure = "exhausted"
    try:
        found = context.search()
        failure = context.stop_reason or "exhausted"
    except SearchStopped as error:
        failure = f"{context.stop_reason}:{error}"
    metrics = {
        "search_nodes": context.search_nodes,
        "unification_attempts": context.unification_attempts,
        "iterations_completed": context.iterations_completed,
        "deepest_iteration": context.deepest_iteration,
        "fresh_instances_allocated": context.next_instance - 1,
    }
    if found is None:
        return make_unknown_certificate(
            problem=problem,
            binary=binary,
            started=started,
            failure=failure,
            transcript_path=transcript_path,
            transcript_sha256=transcript_sha256,
            matrix_sha256=matrix_sha256,
            clause_count=len(clauses),
            metrics=metrics,
        )

    start_clause_index, start_instance_id, proof = found
    rule_counts = proof_rule_counts(proof)
    usage = (
        resource.getrusage(resource.RUSAGE_SELF)
        if resource is not None
        else None
    )
    return {
        "schema": common.SCHEMA,
        "schema_version": 1,
        "status": "Theorem",
        "problem_path": str(problem),
        "problem_sha256": common.sha256_file(problem),
        "binary_sha256": common.sha256_file(binary),
        "transcript_path": str(transcript_path),
        "transcript_sha256": transcript_sha256,
        "matrix_sha256": matrix_sha256,
        "clause_count": len(clauses),
        "source_clauses": [
            {
                "index": clause.index,
                "name": clause.name,
                "role": clause.role,
                "statement_sha256": clause.statement_sha256,
            }
            for clause in clauses
        ],
        "start_clause_index": start_clause_index,
        "start_instance_id": start_instance_id,
        "proof": proof,
        "proof_rule_counts": rule_counts,
        "proof_rule_nodes": sum(rule_counts.values()),
        "failure": None,
        "wall_seconds": time.monotonic() - started,
        "user_cpu_seconds": usage.ru_utime if usage is not None else None,
        "system_cpu_seconds": usage.ru_stime if usage is not None else None,
        "maximum_rss_kib": usage.ru_maxrss if usage is not None else None,
        "metrics": metrics,
        "limits": {
            "budget_seconds": common.CONNECTION_BUDGET_SECONDS,
            "maximum_branch_depth": common.MAX_BRANCH_DEPTH,
            "maximum_search_nodes": common.MAX_SEARCH_NODES,
        },
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--tptp-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    for path, label in (
        (arguments.repo_root, "repository"),
        (arguments.binary, "binary"),
        (arguments.problem, "problem"),
        (arguments.tptp_root, "TPTP root"),
    ):
        if not path.exists():
            raise common.ExperimentError(f"missing {label}: {path}")
    certificate = run_worker(
        repo_root=arguments.repo_root.resolve(),
        binary=arguments.binary.resolve(),
        problem=arguments.problem.resolve(),
        tptp_root=arguments.tptp_root.resolve(),
        output_root=arguments.output_root.resolve(),
    )
    common.atomic_json(arguments.output_root / "certificate.json", certificate)
    print(json.dumps(certificate, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
