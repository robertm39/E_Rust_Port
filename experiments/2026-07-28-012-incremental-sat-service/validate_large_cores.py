#!/usr/bin/env python3
"""Replay large returned assumption cores through independent SAT backends."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

from validate_results import QueryState, parse_session


@dataclass
class CoreCase:
    state: QueryState
    core: tuple[int, ...]
    origins: int = 0
    producers: set[str] = field(default_factory=set)


def resolve_session(workload_root: Path, raw: str) -> Path:
    raw_path = Path(raw)
    candidates = (raw_path, workload_root / raw_path, workload_root / raw_path.name)
    for candidate in candidates:
        if candidate.exists():
            return candidate
    matches = list(workload_root.rglob(raw_path.name))
    if len(matches) != 1:
        raise ValueError(f"cannot resolve session {raw!r}")
    return matches[0]


def case_key(state: QueryState, core: tuple[int, ...]) -> str:
    payload = json.dumps(
        [state.max_variable, state.clauses, core],
        separators=(",", ":"),
    ).encode()
    return hashlib.sha256(payload).hexdigest()


def collect_cases(workload_root: Path, results: list[Path]) -> dict[str, CoreCase]:
    sessions: dict[Path, dict[str, QueryState]] = {}
    cases: dict[str, CoreCase] = {}
    for result_path in results:
        for line in result_path.read_text(encoding="utf-8").splitlines():
            if not line:
                continue
            record = json.loads(line)
            if (
                record.get("record_type", "query") != "query"
                or record.get("status") != "unsat"
                or not record.get("assumptions")
            ):
                continue
            session_path = resolve_session(workload_root, str(record["session"]))
            query_states = sessions.setdefault(session_path, parse_session(session_path))
            state = query_states[str(record["query"])]
            if state.max_variable <= 16:
                continue
            core = tuple(int(literal) for literal in record.get("core", []))
            if not set(core).issubset(state.assumptions):
                raise ValueError(
                    f"{session_path}:{record['query']}: core is not a subset"
                )
            key = case_key(state, core)
            case = cases.setdefault(key, CoreCase(state=state, core=core))
            case.origins += 1
            case.producers.add(str(record["backend"]))
    return cases


def render_session(case: CoreCase) -> str:
    lines = [f"p isat {case.state.max_variable}"]
    lines.extend(
        "a " + " ".join(str(literal) for literal in clause) + " 0"
        for clause in case.state.clauses
    )
    suffix = " ".join(str(literal) for literal in case.core)
    lines.append(f"q core_check -1 0 {suffix} 0")
    return "\n".join(lines) + "\n"


def replay(checker: Path, session: Path, timeout: float) -> dict[str, object]:
    completed = subprocess.run(
        [str(checker), str(session)],
        check=False,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if completed.returncode != 0:
        return {
            "status": "process_failure",
            "returncode": completed.returncode,
            "stderr": completed.stderr[-500:],
        }
    records = [
        json.loads(line) for line in completed.stdout.splitlines() if line.strip()
    ]
    if len(records) != 1:
        return {"status": "malformed_output", "records": len(records)}
    return {
        "status": str(records[0].get("status")),
        "elapsed_ns": int(records[0].get("elapsed_ns", 0)),
    }


def checker_argument(raw: str) -> tuple[str, Path]:
    if "=" not in raw:
        raise argparse.ArgumentTypeError("checker must be NAME=EXECUTABLE")
    name, executable = raw.split("=", 1)
    return name, Path(executable)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("workload_root", type=Path)
    parser.add_argument("results", type=Path, nargs="+")
    parser.add_argument(
        "--checker",
        action="append",
        type=checker_argument,
        required=True,
    )
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()

    cases = collect_cases(arguments.workload_root, arguments.results)
    failures: list[dict[str, object]] = []
    elapsed: dict[str, list[int]] = {
        name: [] for name, _ in arguments.checker
    }
    with tempfile.TemporaryDirectory(prefix="sat-core-check-") as raw_temp:
        temp = Path(raw_temp)
        for key, case in sorted(cases.items()):
            session = temp / f"{key}.isat"
            session.write_text(render_session(case), encoding="utf-8")
            for name, executable in arguments.checker:
                result = replay(executable, session, arguments.timeout_seconds)
                if result["status"] != "unsat":
                    failures.append(
                        {
                            "case": key,
                            "checker": name,
                            "core": list(case.core),
                            "origins": case.origins,
                            "producers": sorted(case.producers),
                            "result": result,
                        }
                    )
                elif "elapsed_ns" in result:
                    elapsed[name].append(int(result["elapsed_ns"]))

    summary = {
        "schema": 1,
        "cases": len(cases),
        "origin_records": sum(case.origins for case in cases.values()),
        "checkers": [name for name, _ in arguments.checker],
        "checks": len(cases) * len(arguments.checker),
        "max_checker_elapsed_ns": {
            name: max(values, default=0) for name, values in elapsed.items()
        },
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
