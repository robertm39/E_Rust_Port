#!/usr/bin/env python3
"""Independently replay a bounded connection-tableau certificate."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Sequence

import connection_common as common


class VerificationError(RuntimeError):
    """A connection certificate or artifact is invalid."""


def verifier_variable(term: common.Term) -> bool:
    if term.arguments or not term.symbol:
        return False
    return term.symbol.startswith("_") or term.symbol[0].isupper()


def verifier_deref(
    term: common.Term, bindings: dict[str, common.Term]
) -> common.Term:
    visited: set[str] = set()
    current = term
    while verifier_variable(current) and current.symbol in bindings:
        if current.symbol in visited:
            raise VerificationError("substitution contains a cycle")
        visited.add(current.symbol)
        current = bindings[current.symbol]
    return current


def verifier_apply_term(
    term: common.Term, bindings: dict[str, common.Term]
) -> common.Term:
    root = verifier_deref(term, bindings)
    if not root.arguments:
        return root
    return common.Term(
        root.symbol,
        tuple(
            verifier_apply_term(argument, bindings)
            for argument in root.arguments
        ),
    )


def verifier_apply_literal(
    literal: common.Literal, bindings: dict[str, common.Term]
) -> common.Literal:
    return common.Literal(
        literal.predicate,
        tuple(
            verifier_apply_term(argument, bindings)
            for argument in literal.arguments
        ),
        literal.positive,
    )


def verifier_occurs(
    variable: str,
    term: common.Term,
    bindings: dict[str, common.Term],
) -> bool:
    current = verifier_deref(term, bindings)
    if verifier_variable(current):
        return current.symbol == variable
    return any(
        verifier_occurs(variable, argument, bindings)
        for argument in current.arguments
    )


def verifier_unify_term(
    first: common.Term,
    second: common.Term,
    bindings: dict[str, common.Term],
) -> None:
    worklist: list[tuple[common.Term, common.Term]] = [(first, second)]
    while worklist:
        left, right = worklist.pop()
        left = verifier_deref(left, bindings)
        right = verifier_deref(right, bindings)
        if left == right:
            continue
        if verifier_variable(left):
            if verifier_occurs(left.symbol, right, bindings):
                raise VerificationError("unification fails the occurs check")
            bindings[left.symbol] = right
            continue
        if verifier_variable(right):
            if verifier_occurs(right.symbol, left, bindings):
                raise VerificationError("unification fails the occurs check")
            bindings[right.symbol] = left
            continue
        if left.symbol != right.symbol:
            raise VerificationError("unification has different term heads")
        if len(left.arguments) != len(right.arguments):
            raise VerificationError("unification has different term arities")
        worklist.extend(
            zip(left.arguments, right.arguments, strict=True)
        )


def verifier_unify_literals(
    first: common.Literal,
    second: common.Literal,
    bindings: dict[str, common.Term],
) -> None:
    if first.predicate != second.predicate:
        raise VerificationError("connection predicates differ")
    if len(first.arguments) != len(second.arguments):
        raise VerificationError("connection predicate arities differ")
    for left, right in zip(first.arguments, second.arguments, strict=True):
        verifier_unify_term(left, right, bindings)


def verifier_fresh_term(term: common.Term, instance_id: int) -> common.Term:
    if verifier_variable(term):
        return common.Term(f"V__{instance_id}__{term.symbol}")
    return common.Term(
        term.symbol,
        tuple(
            verifier_fresh_term(argument, instance_id)
            for argument in term.arguments
        ),
    )


def verifier_fresh_literal(
    literal: common.Literal, instance_id: int
) -> common.Literal:
    return common.Literal(
        literal.predicate,
        tuple(
            verifier_fresh_term(argument, instance_id)
            for argument in literal.arguments
        ),
        literal.positive,
    )


class Replay:
    """Stateful independent proof-tree replay."""

    def __init__(self, clauses: Sequence[common.Clause]) -> None:
        self.clauses = clauses
        self.used_instance_ids: set[int] = set()
        self.rule_counts = {"extension": 0, "reduction": 0}
        self.maximum_branch_depth = 0

    def claim_instance(self, value: Any) -> int:
        if not isinstance(value, int) or isinstance(value, bool) or value < 1:
            raise VerificationError("instance identifier must be a positive integer")
        if value in self.used_instance_ids:
            raise VerificationError(f"reused instance identifier: {value}")
        self.used_instance_ids.add(value)
        return value

    @staticmethod
    def require_int(node: dict[str, Any], name: str) -> int:
        value = node.get(name)
        if not isinstance(value, int) or isinstance(value, bool):
            raise VerificationError(f"{name} must be an integer")
        return value

    def replay(
        self,
        node: Any,
        goals: Sequence[common.Literal],
        path: tuple[common.Literal, ...],
        bindings: dict[str, common.Term],
        branch_depth: int,
    ) -> dict[str, common.Term]:
        if not isinstance(node, dict):
            raise VerificationError("proof node must be an object")
        kind = node.get("kind")
        if kind == "closed":
            if set(node) != {"kind"}:
                raise VerificationError("closed node has unexpected fields")
            if goals:
                raise VerificationError("closed node leaves open goals")
            return bindings
        if kind not in {"extension", "reduction"}:
            raise VerificationError(f"unknown proof node kind: {kind!r}")
        if not goals:
            raise VerificationError(f"{kind} node has no open goal")
        goal_index = self.require_int(node, "goal_index")
        if not 0 <= goal_index < len(goals):
            raise VerificationError("goal index is outside the open goal list")
        goal = goals[goal_index]
        remaining = tuple(goals[:goal_index]) + tuple(goals[goal_index + 1 :])
        applied_goal = verifier_apply_literal(goal, bindings)
        if node.get("goal") != applied_goal.canonical():
            raise VerificationError("goal diagnostic does not match replay state")
        if any(
            verifier_apply_literal(ancestor, bindings) == applied_goal
            for ancestor in path
        ):
            raise VerificationError("proof violates the frozen regularity rule")

        if kind == "reduction":
            allowed = {
                "kind",
                "goal_index",
                "goal",
                "path_index",
                "continuation",
            }
            if set(node) != allowed:
                raise VerificationError("reduction node has unexpected fields")
            path_index = self.require_int(node, "path_index")
            if not 0 <= path_index < len(path):
                raise VerificationError("reduction path index is invalid")
            ancestor = path[path_index]
            if ancestor.positive == goal.positive:
                raise VerificationError("reduction literals have the same polarity")
            next_bindings = bindings.copy()
            verifier_unify_literals(goal, ancestor, next_bindings)
            self.rule_counts["reduction"] += 1
            return self.replay(
                node["continuation"],
                remaining,
                path,
                next_bindings,
                branch_depth,
            )

        allowed = {
            "kind",
            "goal_index",
            "goal",
            "clause_index",
            "literal_index",
            "instance_id",
            "branch",
            "continuation",
        }
        if set(node) != allowed:
            raise VerificationError("extension node has unexpected fields")
        clause_index = self.require_int(node, "clause_index")
        if not 0 <= clause_index < len(self.clauses):
            raise VerificationError("extension clause index is invalid")
        source = self.clauses[clause_index]
        literal_index = self.require_int(node, "literal_index")
        if not 0 <= literal_index < len(source.literals):
            raise VerificationError("extension literal index is invalid")
        instance_id = self.claim_instance(node.get("instance_id"))
        fresh = tuple(
            verifier_fresh_literal(literal, instance_id)
            for literal in source.literals
        )
        connector = fresh[literal_index]
        if connector.positive == goal.positive:
            raise VerificationError("extension literals have the same polarity")
        next_bindings = bindings.copy()
        verifier_unify_literals(goal, connector, next_bindings)
        next_depth = branch_depth + 1
        if next_depth > common.MAX_BRANCH_DEPTH:
            raise VerificationError("proof exceeds the frozen branch-depth limit")
        self.maximum_branch_depth = max(self.maximum_branch_depth, next_depth)
        self.rule_counts["extension"] += 1
        if sum(self.rule_counts.values()) > common.MAX_SEARCH_NODES:
            raise VerificationError("proof exceeds the frozen node limit")
        branch_goals = fresh[:literal_index] + fresh[literal_index + 1 :]
        branch_bindings = self.replay(
            node["branch"],
            branch_goals,
            path + (goal,),
            next_bindings,
            next_depth,
        )
        return self.replay(
            node["continuation"],
            remaining,
            path,
            branch_bindings,
            branch_depth,
        )


def verify_artifacts(
    *,
    certificate: dict[str, Any],
    certificate_path: Path,
    transcript: Path,
    repo_root: Path,
    binary: Path,
    problem: Path,
    tptp_root: Path,
) -> tuple[list[common.Clause] | None, dict[str, Any]]:
    if certificate.get("schema") != common.SCHEMA:
        raise VerificationError("unknown certificate schema")
    if certificate.get("schema_version") != 1:
        raise VerificationError("unknown certificate schema version")
    if certificate.get("problem_sha256") != common.sha256_file(problem):
        raise VerificationError("problem hash does not match the certificate")
    if certificate.get("binary_sha256") != common.sha256_file(binary):
        raise VerificationError("binary hash does not match the certificate")

    transcript_exists = transcript.is_file()
    recorded_path = certificate.get("transcript_path")
    if transcript_exists:
        if recorded_path is None or Path(recorded_path).resolve() != transcript.resolve():
            raise VerificationError("transcript path does not match the certificate")
        observed_transcript_hash = common.sha256_file(transcript)
        if certificate.get("transcript_sha256") != observed_transcript_hash:
            raise VerificationError("transcript hash does not match the certificate")
        rerun = common.run_clausifier(
            binary=binary,
            problem=problem,
            tptp_root=tptp_root,
            timeout=30,
        )
        if rerun.stdout != transcript.read_bytes():
            raise VerificationError("fresh clausification differs byte-for-byte")
        try:
            transcript_text = transcript.read_text(encoding="utf-8")
            clauses = common.parse_cnf_transcript(
                transcript_text,
                repo_root=repo_root,
                module_name="connection_verifier_trace_parser",
            )
        except (common.ExperimentError, UnicodeError) as error:
            if certificate.get("status") == "Unknown":
                return None, {
                    "artifact_checked": True,
                    "transcript_replayed": True,
                    "parse_supported": False,
                    "parse_error": f"{type(error).__name__}:{error}",
                }
            raise VerificationError(f"cannot parse claimed proof matrix: {error}") from error
        if certificate.get("matrix_sha256") != common.matrix_digest(clauses):
            raise VerificationError("matrix digest does not match the certificate")
        if certificate.get("clause_count") != len(clauses):
            raise VerificationError("clause count does not match the certificate")
        return clauses, {
            "artifact_checked": True,
            "transcript_replayed": True,
            "parse_supported": True,
            "parse_error": None,
        }

    if certificate.get("status") != "Unknown":
        raise VerificationError("proof claim lacks a CNF transcript")
    if recorded_path is not None or certificate.get("transcript_sha256") is not None:
        raise VerificationError("missing transcript has recorded provenance")
    return None, {
        "artifact_checked": True,
        "transcript_replayed": False,
        "parse_supported": False,
        "parse_error": "clausification_produced_no_transcript",
    }


def verify_certificate(
    *,
    certificate_path: Path,
    transcript: Path,
    repo_root: Path,
    binary: Path,
    problem: Path,
    tptp_root: Path,
) -> dict[str, Any]:
    try:
        certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise VerificationError(f"cannot read certificate: {error}") from error
    if not isinstance(certificate, dict):
        raise VerificationError("certificate root must be an object")
    clauses, artifact = verify_artifacts(
        certificate=certificate,
        certificate_path=certificate_path,
        transcript=transcript,
        repo_root=repo_root,
        binary=binary,
        problem=problem,
        tptp_root=tptp_root,
    )
    status = certificate.get("status")
    if status == "Unknown":
        forbidden = {"proof", "start_clause_index", "start_instance_id"}
        if forbidden & set(certificate):
            raise VerificationError("Unknown certificate contains a theorem proof")
        return {
            "schema_version": 1,
            "valid": True,
            "status": "Unknown",
            "proof_checked": False,
            "rule_counts": {"extension": 0, "reduction": 0},
            **artifact,
        }
    if status != "Theorem":
        raise VerificationError(f"invalid connection status: {status!r}")
    if clauses is None:
        raise VerificationError("Theorem certificate has no parsed matrix")
    source_table = [
        {
            "index": clause.index,
            "name": clause.name,
            "role": clause.role,
            "statement_sha256": clause.statement_sha256,
        }
        for clause in clauses
    ]
    if certificate.get("source_clauses") != source_table:
        raise VerificationError("source-clause provenance table differs")
    start_index = certificate.get("start_clause_index")
    if not isinstance(start_index, int) or isinstance(start_index, bool):
        raise VerificationError("start clause index must be an integer")
    if not 0 <= start_index < len(clauses):
        raise VerificationError("start clause index is invalid")
    start = clauses[start_index]
    if start.role != "negated_conjecture":
        raise VerificationError("start clause is not a negated conjecture")
    replay = Replay(clauses)
    start_instance = replay.claim_instance(certificate.get("start_instance_id"))
    start_goals = tuple(
        verifier_fresh_literal(literal, start_instance)
        for literal in start.literals
    )
    replay.replay(certificate.get("proof"), start_goals, (), {}, 0)
    if certificate.get("proof_rule_counts") != replay.rule_counts:
        raise VerificationError("recorded proof rule counts differ")
    if certificate.get("proof_rule_nodes") != sum(replay.rule_counts.values()):
        raise VerificationError("recorded proof node total differs")
    return {
        "schema_version": 1,
        "valid": True,
        "status": "Theorem",
        "proof_checked": True,
        "rule_counts": replay.rule_counts,
        "maximum_branch_depth": replay.maximum_branch_depth,
        "fresh_instances": len(replay.used_instance_ids),
        **artifact,
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--certificate", type=Path, required=True)
    parser.add_argument("--transcript", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--tptp-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        result = verify_certificate(
            certificate_path=arguments.certificate.resolve(),
            transcript=arguments.transcript.resolve(),
            repo_root=arguments.repo_root.resolve(),
            binary=arguments.binary.resolve(),
            problem=arguments.problem.resolve(),
            tptp_root=arguments.tptp_root.resolve(),
        )
    except (
        VerificationError,
        common.ExperimentError,
        KeyError,
        RecursionError,
    ) as error:
        print(
            json.dumps(
                {
                    "schema_version": 1,
                    "valid": False,
                    "error": f"{type(error).__name__}:{error}",
                },
                sort_keys=True,
            )
        )
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())

