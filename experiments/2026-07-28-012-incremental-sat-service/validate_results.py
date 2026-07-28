#!/usr/bin/env python3
"""Validate adapter JSON records against sessions and exact small-CNF oracles."""

from __future__ import annotations

import argparse
import itertools
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


@dataclass(frozen=True)
class QueryState:
    clauses: tuple[tuple[int, ...], ...]
    assumptions: tuple[int, ...]
    max_variable: int


def parse_session(path: Path) -> dict[str, QueryState]:
    clauses: list[tuple[int, ...]] = []
    queries: dict[str, QueryState] = {}
    max_variable: int | None = None
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line or line.startswith("c"):
            continue
        fields = line.split()
        if fields[0] == "p":
            if len(fields) != 3 or fields[1] != "isat":
                raise ValueError(f"{path}:{line_number}: malformed header")
            max_variable = int(fields[2])
        elif fields[0] == "a":
            if fields[-1] != "0":
                raise ValueError(f"{path}:{line_number}: unterminated clause")
            clauses.append(tuple(int(field) for field in fields[1:-1]))
        elif fields[0] == "q":
            if max_variable is None or len(fields) < 5 or fields[-1] != "0":
                raise ValueError(f"{path}:{line_number}: malformed query")
            identifier = fields[1]
            if identifier in queries:
                raise ValueError(f"{path}:{line_number}: duplicate query")
            queries[identifier] = QueryState(
                tuple(clauses),
                tuple(int(field) for field in fields[4:-1]),
                max_variable,
            )
        else:
            raise ValueError(f"{path}:{line_number}: unknown opcode")
    return queries


def clause_satisfied(clause: Iterable[int], true_variables: set[int]) -> bool:
    return any(
        (literal > 0 and literal in true_variables)
        or (literal < 0 and -literal not in true_variables)
        for literal in clause
    )


def formula_satisfied(
    clauses: Iterable[Iterable[int]],
    assumptions: Iterable[int],
    true_variables: set[int],
) -> bool:
    return all(clause_satisfied(clause, true_variables) for clause in clauses) and all(
        clause_satisfied((literal,), true_variables) for literal in assumptions
    )


def brute_force(state: QueryState, assumptions: tuple[int, ...] | None = None) -> bool:
    active_assumptions = state.assumptions if assumptions is None else assumptions
    for values in itertools.product((False, True), repeat=state.max_variable):
        true_variables = {
            index + 1 for index, value in enumerate(values) if value
        }
        if formula_satisfied(state.clauses, active_assumptions, true_variables):
            return True
    return False


def validate_record(record: dict[str, object], state: QueryState) -> list[str]:
    failures: list[str] = []
    status = str(record.get("status"))
    if status not in {"sat", "unsat", "unknown", "error"}:
        return [f"invalid status {status!r}"]

    exact_status: str | None = None
    if state.max_variable <= 16:
        exact_status = "sat" if brute_force(state) else "unsat"
        if status not in {exact_status, "unknown"}:
            failures.append(f"status {status} disagrees with exact {exact_status}")

    model = [int(value) for value in record.get("model", [])]
    if status == "sat":
        if not model and state.max_variable != 0:
            # The current internal DPLL does not expose a model. This is a
            # declared capability gap, not a fabricated certificate.
            if record.get("backend") != "internal-dpll":
                failures.append("SAT result has no model")
        else:
            assigned = {abs(literal) for literal in model}
            expected = set(range(1, state.max_variable + 1))
            if assigned != expected or len(model) != len(expected):
                failures.append("model is not a complete one-value assignment")
            true_variables = {literal for literal in model if literal > 0}
            if not formula_satisfied(
                state.clauses, state.assumptions, true_variables
            ):
                failures.append("model does not satisfy active formula")

    core = tuple(int(value) for value in record.get("core", []))
    if status == "unsat" and state.assumptions:
        if not set(core).issubset(state.assumptions):
            failures.append("core contains a literal outside assumptions")
        elif exact_status == "unsat" and brute_force(state, core):
            failures.append("returned assumption core is satisfiable")
    elif core:
        failures.append("core returned outside assumption-dependent UNSAT")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workload_root", type=Path)
    parser.add_argument("results", type=Path, nargs="+")
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    sessions: dict[str, dict[str, QueryState]] = {}
    records = 0
    failures: list[dict[str, object]] = []
    statuses: dict[str, int] = {}
    for result_path in arguments.results:
        for line_number, line in enumerate(
            result_path.read_text(encoding="utf-8").splitlines(), 1
        ):
            if not line:
                continue
            record = json.loads(line)
            if record.get("record_type", "query") != "query":
                continue
            raw_session = Path(str(record["session"]))
            candidates = [
                raw_session,
                arguments.workload_root / raw_session,
                arguments.workload_root / raw_session.name,
            ]
            session_path = next((path for path in candidates if path.exists()), None)
            if session_path is None:
                matches = list(arguments.workload_root.rglob(raw_session.name))
                if len(matches) != 1:
                    raise ValueError(
                        f"{result_path}:{line_number}: cannot resolve {raw_session}"
                    )
                session_path = matches[0]
            key = str(session_path.resolve())
            query_states = sessions.setdefault(key, parse_session(session_path))
            query = str(record["query"])
            if query not in query_states:
                raise ValueError(
                    f"{result_path}:{line_number}: unknown query {query!r}"
                )
            record_failures = validate_record(record, query_states[query])
            if record_failures:
                failures.append(
                    {
                        "file": str(result_path),
                        "line": line_number,
                        "backend": record.get("backend"),
                        "session": str(session_path),
                        "query": query,
                        "failures": record_failures,
                    }
                )
            status = str(record["status"])
            statuses[status] = statuses.get(status, 0) + 1
            records += 1

    summary = {
        "schema": 1,
        "records": records,
        "statuses": dict(sorted(statuses.items())),
        "failures": failures,
        "valid": not failures,
    }
    rendered = json.dumps(summary, indent=2, sort_keys=True) + "\n"
    if arguments.output:
        arguments.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
