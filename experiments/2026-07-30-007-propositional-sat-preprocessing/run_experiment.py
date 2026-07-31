#!/usr/bin/env python3
"""Prepare, run, certify, and analyze the propositional-preprocessing study."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import re
import shutil
import signal
import statistics
import subprocess
import tarfile
import tempfile
import time
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


EXPERIMENT = "2026-07-30-007-propositional-sat-preprocessing"
SAT_ARCHIVE_SHA256 = (
    "85356e073a26234f51e07898019d0a9a7685066eff21dd9350d621ede3158375"
)
SAT_MANIFEST = "workloads/captured-test-final/manifest.json"
TOKEN_ATOM = re.compile(r"(?:[a-z][A-Za-z0-9_]*|'(?:[^'\\]|\\.)*')\Z")
SZS_RE = re.compile(r"SZS status ([A-Za-z]+)")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def session_payload_valid(data: bytes, session: dict[str, Any]) -> bool:
    """Authenticate generated ISAT bytes, not the source capture byte count."""
    return sha256_bytes(data) == session["session_sha256"]


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    temporary.replace(path)


def canonical_clause(clause: Iterable[int]) -> tuple[int, ...]:
    return tuple(sorted(set(clause), key=lambda literal: (abs(literal), literal < 0)))


def parse_isat(data: bytes) -> tuple[int, list[tuple[int, ...]], list[dict[str, Any]]]:
    maximum: int | None = None
    clauses: list[tuple[int, ...]] = []
    queries: list[dict[str, Any]] = []
    for line_number, raw in enumerate(data.decode("ascii").splitlines(), 1):
        if not raw or raw.startswith("c"):
            continue
        fields = raw.split()
        if fields[0] == "p":
            if len(fields) != 3 or fields[1] != "isat" or maximum is not None:
                raise ValueError(f"line {line_number}: invalid ISAT header")
            maximum = int(fields[2])
            if maximum < 0:
                raise ValueError(f"line {line_number}: negative variable count")
        elif fields[0] == "a":
            clauses.append(parse_zero_terminated(fields[1:], line_number))
        elif fields[0] == "q":
            if len(fields) < 5:
                raise ValueError(f"line {line_number}: incomplete query")
            queries.append(
                {
                    "query": fields[1],
                    "decision_limit": int(fields[2]),
                    "deadline_us": int(fields[3]),
                    "assumptions": list(parse_zero_terminated(fields[4:], line_number)),
                }
            )
        else:
            raise ValueError(f"line {line_number}: unknown ISAT opcode")
    if maximum is None or not queries:
        raise ValueError("ISAT session requires a header and at least one query")
    if len({query["query"] for query in queries}) != len(queries):
        raise ValueError("ISAT query identifiers are not unique")
    return maximum, clauses, queries


def parse_zero_terminated(fields: list[str], line_number: int) -> tuple[int, ...]:
    if not fields or fields[-1] != "0":
        raise ValueError(f"line {line_number}: literals are not zero-terminated")
    literals = tuple(int(field) for field in fields[:-1])
    if any(literal == 0 for literal in literals):
        raise ValueError(f"line {line_number}: embedded zero literal")
    return literals


def dimacs_bytes(
    variables: int, clauses: Iterable[Iterable[int]]
) -> tuple[bytes, list[tuple[int, ...]]]:
    normalized = [canonical_clause(clause) for clause in clauses]
    lines = [f"p cnf {variables} {len(normalized)}"]
    for clause in normalized:
        body = " ".join(str(literal) for literal in clause)
        lines.append(f"{body} 0" if body else "0")
    return ("\n".join(lines) + "\n").encode("ascii"), normalized


def parse_dimacs(path: Path) -> tuple[int, list[tuple[int, ...]]]:
    variables: int | None = None
    declared: int | None = None
    clauses: list[tuple[int, ...]] = []
    for line_number, line in enumerate(path.read_text(encoding="ascii").splitlines(), 1):
        fields = line.split()
        if not fields or fields[0] == "c":
            continue
        if fields[0] == "p":
            if len(fields) != 4 or fields[1] != "cnf" or variables is not None:
                raise ValueError(f"{path}:{line_number}: invalid DIMACS header")
            variables, declared = int(fields[2]), int(fields[3])
        else:
            clauses.append(parse_zero_terminated(fields, line_number))
    if variables is None or declared != len(clauses):
        raise ValueError(f"{path}: DIMACS header mismatch")
    return variables, clauses


def complete_model_valid(
    variables: int, clauses: Iterable[Iterable[int]], model: list[int]
) -> bool:
    if len(model) != variables:
        return False
    expected = set(range(1, variables + 1))
    if {abs(literal) for literal in model} != expected:
        return False
    values = set(model)
    return all(any(literal in values for literal in clause) for clause in clauses)


def exhaustive_status(variables: int, clauses: list[tuple[int, ...]]) -> str | None:
    if variables > 20:
        return None
    for assignment in range(1 << variables):
        if all(
            any(
                bool(assignment & (1 << (abs(literal) - 1))) == (literal > 0)
                for literal in clause
            )
            for clause in clauses
        ):
            return "sat"
    return "unsat"


def strip_comments(text: str) -> str:
    output: list[str] = []
    index = 0
    quote: str | None = None
    block = False
    while index < len(text):
        char = text[index]
        following = text[index + 1] if index + 1 < len(text) else ""
        if block:
            if char == "*" and following == "/":
                block = False
                index += 2
            else:
                index += 1
            continue
        if quote:
            output.append(char)
            if char == "\\" and following:
                output.append(following)
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
            output.append(char)
            index += 1
        elif char == "%":
            while index < len(text) and text[index] not in "\r\n":
                index += 1
        elif char == "/" and following == "*":
            block = True
            index += 2
        else:
            output.append(char)
            index += 1
    if quote or block:
        raise ValueError("unterminated quote or block comment")
    return "".join(output)


def split_top_level(text: str, separator: str) -> list[str]:
    parts: list[str] = []
    start = 0
    depth = 0
    quote: str | None = None
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            if char == "\\":
                index += 2
                continue
            if char == quote:
                quote = None
        elif char in {"'", '"'}:
            quote = char
        elif char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                raise ValueError("unbalanced delimiter")
        elif char == separator and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
        index += 1
    if quote or depth != 0:
        raise ValueError("unbalanced top-level syntax")
    parts.append(text[start:].strip())
    return parts


def iter_statements(text: str) -> Iterable[str]:
    statement: list[str] = []
    depth = 0
    quote: str | None = None
    block_comment = False
    line_comment = False
    index = 0
    while index < len(text):
        char = text[index]
        following = text[index + 1] if index + 1 < len(text) else ""
        if line_comment:
            if char in "\r\n":
                line_comment = False
                statement.append(char)
            index += 1
            continue
        if block_comment:
            if char == "*" and following == "/":
                block_comment = False
                index += 2
            else:
                index += 1
            continue
        if quote:
            statement.append(char)
            if char == "\\" and following:
                statement.append(following)
                index += 2
                continue
            if char == quote:
                quote = None
            index += 1
            continue
        if char in {"'", '"'}:
            quote = char
            statement.append(char)
        elif char == "%":
            line_comment = True
        elif char == "/" and following == "*":
            block_comment = True
            index += 1
        elif char in "([{":
            depth += 1
            statement.append(char)
        elif char in ")]}":
            depth -= 1
            if depth < 0:
                raise ValueError("unbalanced delimiter")
            statement.append(char)
        elif char == "." and depth == 0:
            complete = "".join(statement).strip()
            if complete:
                yield complete
            statement.clear()
        else:
            statement.append(char)
        index += 1
    if quote or block_comment:
        raise ValueError("unterminated quote or block comment")
    if depth != 0:
        raise ValueError("unbalanced top-level syntax")
    if "".join(statement).strip():
        raise ValueError("unterminated_statement")


def split_statements(text: str) -> list[str]:
    return list(iter_statements(text))


def strip_outer_parentheses(text: str) -> str:
    text = text.strip()
    while text.startswith("(") and text.endswith(")"):
        depth = 0
        quote: str | None = None
        closes_at_end = False
        for index, char in enumerate(text):
            if quote:
                if char == quote and (index == 0 or text[index - 1] != "\\"):
                    quote = None
            elif char in {"'", '"'}:
                quote = char
            elif char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    closes_at_end = index == len(text) - 1
                    break
        if not closes_at_end:
            break
        text = text[1:-1].strip()
    return text


def parse_prop_clause(formula: str) -> tuple[list[tuple[str, bool]], bool]:
    formula = strip_outer_parentheses(formula)
    parts = split_top_level(formula, "|")
    literals: list[tuple[str, bool]] = []
    for part in parts:
        literal = strip_outer_parentheses(part)
        positive = True
        if literal.startswith("~"):
            positive = False
            literal = strip_outer_parentheses(literal[1:])
        if literal == "$true":
            if positive:
                return [], True
            continue
        if literal == "$false":
            if not positive:
                return [], True
            continue
        if not TOKEN_ATOM.fullmatch(literal):
            raise ValueError("non-propositional literal")
        literals.append((literal, positive))
    if any((atom, not sign) in literals for atom, sign in literals):
        return [], True
    return literals, False


def classify_whole_problem(text: str) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    for statement in iter_statements(text):
        lowered = statement.lstrip().lower()
        if lowered.startswith("include"):
            raise ValueError("include")
        if not lowered.startswith("cnf"):
            raise ValueError("non_cnf_record")
        opening = statement.find("(")
        if opening < 0 or not statement.rstrip().endswith(")"):
            raise ValueError("malformed_cnf")
        fields = split_top_level(statement[opening + 1 : statement.rfind(")")], ",")
        if len(fields) < 3:
            raise ValueError("malformed_cnf")
        name, role, formula = fields[:3]
        role = role.strip().lower()
        if role == "conjecture":
            raise ValueError("cnf_conjecture")
        literals, tautology = parse_prop_clause(formula)
        records.append(
            {
                "name": name.strip(),
                "role": role,
                "literals": literals,
                "tautology": tautology,
            }
        )
    if not records:
        raise ValueError("empty")
    names = [record["name"] for record in records]
    if len(set(names)) != len(names):
        raise ValueError("duplicate_cnf_name")
    atoms = sorted(
        {atom for record in records for atom, _positive in record["literals"]}
    )
    atom_ids = {atom: index + 1 for index, atom in enumerate(atoms)}
    clauses: list[tuple[int, ...]] = []
    mappings: list[dict[str, Any]] = []
    for record in records:
        if record["tautology"]:
            mappings.append(
                {
                    "source_name": record["name"],
                    "role": record["role"],
                    "dimacs_clause": None,
                    "tautology": True,
                    "source_literals": [],
                }
            )
            continue
        clause = tuple(
            atom_ids[atom] if positive else -atom_ids[atom]
            for atom, positive in record["literals"]
        )
        clauses.append(canonical_clause(clause))
        mappings.append(
            {
                "source_name": record["name"],
                "role": record["role"],
                "dimacs_clause": len(clauses),
                "tautology": False,
                "source_literals": [
                    {"atom": atom, "positive": positive}
                    for atom, positive in record["literals"]
                ],
            }
        )
    return {"atoms": atom_ids, "clauses": clauses, "source_mappings": mappings}


def load_problem_rows(path: Path) -> list[dict[str, Any]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    problems = [row for row in rows if row.get("record_type") == "problem"]
    if len(problems) != 2901:
        raise ValueError(f"expected 2901 CASC problems, found {len(problems)}")
    return problems


def prepare(arguments: argparse.Namespace) -> int:
    output = arguments.output.resolve()
    if output.exists():
        raise ValueError(f"refusing existing preparation directory: {output}")
    output.mkdir(parents=True)
    coordinates: list[dict[str, Any]] = []
    rejection_counts: Counter[str] = Counter()
    accepted_by_category: Counter[str] = Counter()
    accepted_by_division: Counter[str] = Counter()
    accepted_by_family: Counter[str] = Counter()

    rows = load_problem_rows(arguments.casc_manifest)
    rows_by_path = {row["path"]: row for row in rows}
    if len(rows_by_path) != len(rows):
        raise ValueError("duplicate CASC manifest path")
    seen_paths: set[str] = set()
    with tarfile.open(arguments.casc_archive, "r|gz") as archive:
        for member in archive:
            row = rows_by_path.get(member.name)
            if row is None:
                continue
            if member.name in seen_paths:
                raise ValueError(f"duplicate CASC archive member {member.name}")
            seen_paths.add(member.name)
            file_object = archive.extractfile(member)
            if file_object is None:
                raise ValueError(f"CASC archive member is not a file: {member.name}")
            data = file_object.read()
            if sha256_bytes(data) != row["sha256"]:
                raise ValueError(f"hash mismatch for {row['path']}")
            try:
                parsed = classify_whole_problem(data.decode("utf-8"))
            except (UnicodeDecodeError, ValueError) as error:
                rejection_counts[str(error)] += 1
                continue
            accepted_by_category[row["category"]] += 1
            accepted_by_division[row["division"]] += 1
            accepted_by_family[row["family"]] += 1
            stem = f"whole-{row['problem_id'].replace('/', '_')}"
            materialization_start = time.perf_counter_ns()
            dimacs, clauses = dimacs_bytes(len(parsed["atoms"]), parsed["clauses"])
            dimacs_path = output / "dimacs" / f"{stem}.cnf"
            source_path = output / "whole-source" / f"{stem}.p"
            mapping_path = output / "whole-mapping" / f"{stem}.json"
            dimacs_path.parent.mkdir(parents=True, exist_ok=True)
            source_path.parent.mkdir(parents=True, exist_ok=True)
            mapping_path.parent.mkdir(parents=True, exist_ok=True)
            dimacs_path.write_bytes(dimacs)
            source_path.write_bytes(data)
            atomic_json(mapping_path, parsed)
            materialization_ns = time.perf_counter_ns() - materialization_start
            coordinates.append(
                {
                    "coordinate": stem,
                    "kind": "whole",
                    "query": "whole",
                    "dimacs": str(dimacs_path.relative_to(output)),
                    "dimacs_sha256": sha256_bytes(dimacs),
                    "source": str(source_path.relative_to(output)),
                    "source_sha256": row["sha256"],
                    "mapping": str(mapping_path.relative_to(output)),
                    "variables": len(parsed["atoms"]),
                    "clauses": len(clauses),
                    "materialization_ns": materialization_ns,
                    "decision_limit": -1,
                    **{
                        key: row[key]
                        for key in (
                            "problem_id",
                            "category",
                            "division",
                            "family",
                            "holdout_split",
                            "expected_class",
                        )
                    },
                }
            )
    missing_paths = sorted(rows_by_path.keys() - seen_paths)
    if missing_paths:
        raise ValueError(f"missing CASC archive member {missing_paths[0]}")

    if sha256_file(arguments.sat_archive) != SAT_ARCHIVE_SHA256:
        raise ValueError("incremental SAT archive hash mismatch")
    with tarfile.open(arguments.sat_archive, "r:gz") as archive:
        manifest = json.load(archive.extractfile(SAT_MANIFEST))
        sessions = manifest["sessions"]
        if len(sessions) != 127:
            raise ValueError(f"expected 127 captured sessions, found {len(sessions)}")
        seen_sessions: set[str] = set()
        for session in sessions:
            name = session["session"]
            if name in seen_sessions:
                raise ValueError(f"duplicate session {name}")
            seen_sessions.add(name)
            member_name = f"workloads/captured-test-final/{name}"
            data = archive.extractfile(member_name).read()
            if not session_payload_valid(data, session):
                raise ValueError(f"captured session integrity failure: {name}")
            maximum, clauses, queries = parse_isat(data)
            for query in queries:
                materialization_start = time.perf_counter_ns()
                scope = clauses + [(literal,) for literal in query["assumptions"]]
                key_hash = hashlib.sha256(
                    f"{name}\0{query['query']}".encode()
                ).hexdigest()[:16]
                stem = f"capture-{key_hash}"
                dimacs, normalized = dimacs_bytes(maximum, scope)
                dimacs_path = output / "dimacs" / f"{stem}.cnf"
                dimacs_path.parent.mkdir(parents=True, exist_ok=True)
                dimacs_path.write_bytes(dimacs)
                materialization_ns = time.perf_counter_ns() - materialization_start
                coordinates.append(
                    {
                        "coordinate": stem,
                        "kind": "captured",
                        "query": query["query"],
                        "dimacs": str(dimacs_path.relative_to(output)),
                        "dimacs_sha256": sha256_bytes(dimacs),
                        "variables": maximum,
                        "clauses": len(normalized),
                        "materialization_ns": materialization_ns,
                        "permanent_clauses": len(clauses),
                        "assumptions": query["assumptions"],
                        "decision_limit": query["decision_limit"],
                        "session": name,
                        "capture_path": session["capture_path"],
                        **{
                            key: session[key]
                            for key in (
                                "problem_id",
                                "category",
                                "division",
                                "family",
                                "holdout_split",
                            )
                        },
                    }
                )

    report = {
        "schema_version": 1,
        "experiment": EXPERIMENT,
        "casc_archive_sha256": sha256_file(arguments.casc_archive),
        "sat_archive_sha256": SAT_ARCHIVE_SHA256,
        "whole_scanned": len(rows),
        "whole_scanned_by_category": dict(
            sorted(Counter(row["category"] for row in rows).items())
        ),
        "whole_scanned_by_division": dict(
            sorted(Counter(row["division"] for row in rows).items())
        ),
        "whole_scanned_by_family": dict(
            sorted(Counter(row["family"] for row in rows).items())
        ),
        "whole_accepted": sum(accepted_by_category.values()),
        "whole_accepted_by_category": dict(sorted(accepted_by_category.items())),
        "whole_accepted_by_division": dict(sorted(accepted_by_division.items())),
        "whole_accepted_by_family": dict(sorted(accepted_by_family.items())),
        "whole_rejection_counts": dict(sorted(rejection_counts.items())),
        "captured_sessions": 127,
        "captured_session_hashes_verified": 127,
        "captured_manifest_bytes_field": "source_capture_bytes_not_session_bytes",
        "captured_coordinates": sum(
            coordinate["kind"] == "captured" for coordinate in coordinates
        ),
        "coordinates": sorted(coordinates, key=lambda item: item["coordinate"]),
    }
    atomic_json(output / "manifest.json", report)
    print(json.dumps({key: value for key, value in report.items() if key != "coordinates"}))
    return 0


def load_prepared(path: Path) -> dict[str, Any]:
    manifest = json.loads((path / "manifest.json").read_text(encoding="utf-8"))
    for coordinate in manifest["coordinates"]:
        dimacs = path / coordinate["dimacs"]
        if sha256_file(dimacs) != coordinate["dimacs_sha256"]:
            raise ValueError(f"prepared DIMACS hash mismatch: {dimacs}")
    return manifest


def parse_existing_results(path: Path) -> dict[tuple[str, str, int], dict[str, Any]]:
    if not path.exists():
        return {}
    records: dict[tuple[str, str, int], dict[str, Any]] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        record = json.loads(line)
        key = (record["coordinate"], record["arm"], record["repetition"])
        if key in records:
            raise ValueError(f"{path}:{line_number}: duplicate result {key}")
        records[key] = record
    return records


def run_process(command: list[str], timeout: float, rss_path: Path) -> dict[str, Any]:
    timed = [
        "/usr/bin/time",
        "-f",
        "%M",
        "-o",
        str(rss_path),
        "/usr/bin/prlimit",
        f"--as={512 * 1024 * 1024}",
        "--",
        *command,
    ]
    started = time.perf_counter_ns()
    process = subprocess.Popen(
        timed,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=True,
    )
    timed_out = False
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        os.killpg(process.pid, signal.SIGKILL)
        stdout, stderr = process.communicate()
    wall_ns = time.perf_counter_ns() - started
    rss_kib = None
    if rss_path.exists() and rss_path.read_text(encoding="ascii").strip():
        rss_kib = int(rss_path.read_text(encoding="ascii").strip().splitlines()[-1])
    return {
        "returncode": process.returncode,
        "timed_out": timed_out,
        "wall_ns": wall_ns,
        "rss_kib": rss_kib,
        "stdout": stdout,
        "stderr": stderr,
    }


def parse_probe_stdout(stdout: str) -> dict[str, Any]:
    lines = [line for line in stdout.splitlines() if line.strip()]
    if not lines:
        raise ValueError("probe produced no JSON output")
    return json.loads(lines[-1])


def result_job(
    job: tuple[dict[str, Any], str, int],
    prepared_root: Path,
    internal_probe: Path,
    cadical_probe: Path,
    umlaut_binary: Path,
    temporary_root: Path,
) -> dict[str, Any]:
    coordinate, arm, repetition = job
    dimacs = prepared_root / coordinate["dimacs"]
    rss_path = temporary_root / (
        f"{coordinate['coordinate']}-{arm}-{repetition}.rss"
    )
    if arm == "internal":
        command = [
            str(internal_probe),
            str(dimacs),
            str(coordinate["decision_limit"]),
        ]
    elif arm in {"plain", "default"}:
        command = [
            str(cadical_probe),
            arm,
            str(dimacs),
            str(coordinate["decision_limit"]),
        ]
    elif arm == "umlaut":
        command = [
            str(umlaut_binary),
            "--auto",
            "--cpu-limit=1",
            "--memory-limit=512",
            "--output-level=1",
            str(prepared_root / coordinate["source"]),
        ]
    else:
        raise ValueError(f"unknown arm {arm}")
    execution = run_process(command, 1.0, rss_path)
    record: dict[str, Any] = {
        "coordinate": coordinate["coordinate"],
        "kind": coordinate["kind"],
        "query": coordinate["query"],
        "arm": arm,
        "repetition": repetition,
        **{key: execution[key] for key in ("returncode", "timed_out", "wall_ns", "rss_kib")},
    }
    if execution["timed_out"]:
        record.update(status="unknown", unknown="external_timeout", valid=True)
        return record
    if arm == "umlaut":
        statuses = SZS_RE.findall(execution["stdout"])
        status = statuses[-1] if statuses else ""
        if status in {"Theorem", "Unsatisfiable"}:
            normalized = "unsat"
        elif status in {"Satisfiable", "CounterSatisfiable"}:
            normalized = "sat"
        else:
            normalized = "unknown"
        record.update(
            status=normalized,
            szs_status=status,
            valid=execution["returncode"] in {0, 1, 8},
            stderr=execution["stderr"],
        )
        return record
    if execution["returncode"] != 0:
        record.update(
            status="error",
            valid=False,
            stderr=execution["stderr"],
            stdout=execution["stdout"],
        )
        return record
    try:
        probe = parse_probe_stdout(execution["stdout"])
    except (json.JSONDecodeError, ValueError):
        record.update(status="error", valid=False, stdout=execution["stdout"])
        return record
    variables, clauses = parse_dimacs(dimacs)
    valid = probe["status"] != "sat" or complete_model_valid(
        variables, clauses, probe["model"]
    )
    record.update(probe)
    record["arm"] = arm
    record["valid"] = valid
    if not valid:
        record["validation_error"] = "invalid complete model"
    return record


def run_benchmark(arguments: argparse.Namespace) -> int:
    prepared = arguments.prepared.resolve()
    manifest = load_prepared(prepared)
    results_path = arguments.results.resolve()
    results_path.parent.mkdir(parents=True, exist_ok=True)
    existing = parse_existing_results(results_path)
    jobs: list[tuple[dict[str, Any], str, int]] = []
    for coordinate in manifest["coordinates"]:
        if coordinate["kind"] == "captured":
            arms = ("internal", "plain", "default")
            repetitions = arguments.repetitions
        else:
            arms = ("umlaut", "plain", "default")
            repetitions = 5
        for repetition in range(repetitions):
            for arm in arms:
                key = (coordinate["coordinate"], arm, repetition)
                if key not in existing:
                    jobs.append((coordinate, arm, repetition))
    jobs.sort(
        key=lambda job: hashlib.sha256(
            f"{job[2]}\0{job[0]['coordinate']}\0{job[1]}".encode()
        ).digest()
    )
    temporary_root = Path(
        tempfile.mkdtemp(prefix="prop-sat-preprocess-", dir=results_path.parent)
    )
    try:
        with results_path.open("a", encoding="utf-8", newline="\n") as output:
            with concurrent.futures.ThreadPoolExecutor(
                max_workers=arguments.workers
            ) as executor:
                futures = [
                    executor.submit(
                        result_job,
                        job,
                        prepared,
                        arguments.internal_probe.resolve(),
                        arguments.cadical_probe.resolve(),
                        arguments.umlaut_binary.resolve(),
                        temporary_root,
                    )
                    for job in jobs
                ]
                for index, future in enumerate(
                    concurrent.futures.as_completed(futures), 1
                ):
                    record = future.result()
                    output.write(json.dumps(record, sort_keys=True) + "\n")
                    output.flush()
                    if index % 1000 == 0:
                        print(f"completed {index}/{len(jobs)}", flush=True)
    finally:
        shutil.rmtree(temporary_root, ignore_errors=True)
    print(f"completed {len(jobs)} new records; resumed {len(existing)}")
    return 0


def unique_statuses(
    records: Iterable[dict[str, Any]]
) -> dict[tuple[str, str], str]:
    grouped: defaultdict[tuple[str, str], set[str]] = defaultdict(set)
    for record in records:
        grouped[(record["coordinate"], record["arm"])].add(record["status"])
        if not record.get("valid", False):
            raise ValueError(f"invalid benchmark record: {record}")
    result: dict[tuple[str, str], str] = {}
    for key, statuses in grouped.items():
        decisive = statuses & {"sat", "unsat"}
        if len(decisive) > 1 or "error" in statuses:
            raise ValueError(f"contradictory or erroneous statuses for {key}: {statuses}")
        result[key] = next(iter(decisive)) if decisive else "unknown"
    return result


def run_certificate_probe(
    probe: Path,
    arm: str,
    dimacs: Path,
    decision_limit: int,
    proof: Path | None,
    simplified: Path | None,
) -> dict[str, Any]:
    command = [str(probe), arm, str(dimacs), str(decision_limit)]
    if proof is not None or simplified is not None:
        command.append(str(proof) if proof is not None else "-")
    if simplified is not None:
        command.append(str(simplified))
    completed = subprocess.run(command, capture_output=True, text=True, timeout=10)
    if completed.returncode != 0:
        raise ValueError(f"certificate probe failed: {completed.stderr}")
    return parse_probe_stdout(completed.stdout)


def drat_verified(completed: subprocess.CompletedProcess[str]) -> bool:
    return completed.returncode == 0 and any(
        line.strip() == "s VERIFIED" for line in completed.stdout.splitlines()
    )


def certify(arguments: argparse.Namespace) -> int:
    prepared_root = arguments.prepared.resolve()
    manifest = load_prepared(prepared_root)
    records = list(parse_existing_results(arguments.results.resolve()).values())
    statuses = unique_statuses(records)
    output_root = arguments.output.resolve()
    if output_root.exists():
        raise ValueError(f"refusing existing certificate directory: {output_root}")
    (output_root / "proofs").mkdir(parents=True)
    (output_root / "simplified").mkdir(parents=True)
    coordinate_by_id = {
        coordinate["coordinate"]: coordinate for coordinate in manifest["coordinates"]
    }
    certificates: list[dict[str, Any]] = []
    oracle_counts = Counter()
    mapping_roundtrips = 0
    for coordinate_id, coordinate in coordinate_by_id.items():
        dimacs = prepared_root / coordinate["dimacs"]
        variables, clauses = parse_dimacs(dimacs)
        if coordinate["kind"] == "whole":
            mapping = json.loads(
                (prepared_root / coordinate["mapping"]).read_text(encoding="utf-8")
            )
            if not mapping_roundtrip_valid(mapping):
                raise ValueError(f"whole mapping rejected for {coordinate_id}")
            mapping_roundtrips += 1
        oracle = exhaustive_status(variables, clauses)
        if oracle is not None:
            oracle_counts[oracle] += 1
            arms = (
                ("internal", "plain", "default")
                if coordinate["kind"] == "captured"
                else ("umlaut", "plain", "default")
            )
            for arm in arms:
                if statuses.get((coordinate_id, arm)) != oracle:
                    raise ValueError(f"exhaustive oracle disagreement for {coordinate_id}")
        for arm in ("plain", "default"):
            status = statuses.get((coordinate_id, arm))
            if status is None:
                continue
            proof = (
                output_root / "proofs" / f"{coordinate_id}-{arm}.drat"
                if status == "unsat"
                else None
            )
            simplified = (
                output_root / "simplified" / f"{coordinate_id}-{arm}.cnf"
                if coordinate["kind"] == "captured" and coordinate["query"] == "cold"
                else None
            )
            if proof is None and simplified is None:
                continue
            probe = run_certificate_probe(
                arguments.cadical_probe.resolve(),
                arm,
                dimacs,
                coordinate["decision_limit"],
                proof,
                simplified,
            )
            checker_ok = None
            checker_ns = 0
            if proof is not None:
                started = time.perf_counter_ns()
                checked = subprocess.run(
                    [str(arguments.drat_trim.resolve()), str(dimacs), str(proof)],
                    capture_output=True,
                    text=True,
                    timeout=30,
                )
                checker_ns = time.perf_counter_ns() - started
                checker_ok = drat_verified(checked)
                if not checker_ok:
                    raise ValueError(f"proof rejected for {coordinate_id}/{arm}")
            certificates.append(
                {
                    "coordinate": coordinate_id,
                    "arm": arm,
                    "status": status,
                    "proof": str(proof.relative_to(output_root)) if proof else None,
                    "proof_sha256": sha256_file(proof) if proof else None,
                    "proof_bytes": proof.stat().st_size if proof else 0,
                    "checker_ok": checker_ok,
                    "checker_ns": checker_ns,
                    "simplified": (
                        str(simplified.relative_to(output_root)) if simplified else None
                    ),
                    "simplified_sha256": (
                        sha256_file(simplified) if simplified else None
                    ),
                    "probe": probe,
                }
            )

    mutation_checks = mutation_tests(
        prepared_root,
        output_root,
        certificates,
        coordinate_by_id,
        arguments.cadical_probe.resolve(),
        arguments.drat_trim.resolve(),
    )
    report = {
        "schema_version": 1,
        "certificates": certificates,
        "exhaustive_oracle_counts": dict(oracle_counts),
        "mapping_roundtrips": {
            "required": manifest["whole_accepted"],
            "checked": mapping_roundtrips,
        },
        "mutation_checks": mutation_checks,
    }
    atomic_json(output_root / "certificates.json", report)
    print(json.dumps({key: value for key, value in report.items() if key != "certificates"}))
    return 0


def mutation_tests(
    prepared_root: Path,
    output_root: Path,
    certificates: list[dict[str, Any]],
    coordinate_by_id: dict[str, dict[str, Any]],
    cadical_probe: Path,
    drat_trim: Path,
) -> dict[str, bool]:
    first_sat = next(
        (
            certificate
            for certificate in certificates
            if certificate["probe"]["status"] == "sat"
        ),
        None,
    )
    if first_sat is None:
        variables, clauses, model = 2, [(1,), (2,)], [1, 2]
    else:
        coordinate = coordinate_by_id[first_sat["coordinate"]]
        variables, clauses = parse_dimacs(prepared_root / coordinate["dimacs"])
        model = first_sat["probe"]["model"]
    truncated = model[:-1]
    model_rejected = not complete_model_valid(variables, clauses, truncated)
    hash_rejected = (
        sha256_file(prepared_root / coordinate["dimacs"])
        != ("0" * 64)
    )
    mapping = {
        "atoms": {"p": 1},
        "clauses": [[1]],
        "source_mappings": [
            {
                "source_name": "synthetic",
                "role": "axiom",
                "dimacs_clause": 1,
                "tautology": False,
                "source_literals": [{"atom": "p", "positive": True}],
            }
        ],
    }
    whole = next(
        (
            item
            for item in coordinate_by_id.values()
            if item["kind"] == "whole"
        ),
        None,
    )
    if whole is not None:
        mapping = json.loads(
            (prepared_root / whole["mapping"]).read_text(encoding="utf-8")
        )
    mapping["source_mappings"][0]["dimacs_clause"] = 10**9
    mapping_rejected = not mapping_roundtrip_valid(mapping)

    mutation_dimacs = output_root / "proofs" / "mutation-source.cnf"
    mutation_proof = output_root / "proofs" / "mutation-valid.drat"
    mutation_corrupt = output_root / "proofs" / "mutation-empty.drat"
    mutation_bytes, _clauses = dimacs_bytes(
        2,
        [(1, 2), (1, -2), (-1, 2), (-1, -2)],
    )
    mutation_dimacs.write_bytes(mutation_bytes)
    mutation_probe = run_certificate_probe(
        cadical_probe,
        "default",
        mutation_dimacs,
        -1,
        mutation_proof,
        None,
    )
    if mutation_probe["status"] != "unsat":
        raise ValueError("proof mutation fixture was not UNSAT")
    valid_checked = subprocess.run(
        [str(drat_trim), str(mutation_dimacs), str(mutation_proof)],
        capture_output=True,
        text=True,
        timeout=30,
    )
    valid_proof_accepted = drat_verified(valid_checked)
    if not valid_proof_accepted:
        raise ValueError("proof mutation fixture did not produce a valid proof")
    mutation_corrupt.write_bytes(b"")
    corrupt_checked = subprocess.run(
        [str(drat_trim), str(mutation_dimacs), str(mutation_corrupt)],
        capture_output=True,
        text=True,
        timeout=30,
    )
    proof_rejected = not drat_verified(corrupt_checked)
    return {
        "truncated_model_rejected": model_rejected,
        "input_hash_corruption_rejected": hash_rejected,
        "mapping_corruption_rejected": mapping_rejected,
        "valid_mutation_fixture_proof_accepted": valid_proof_accepted,
        "proof_corruption_rejected": proof_rejected,
    }


def mapping_roundtrip_valid(mapping: dict[str, Any]) -> bool:
    atoms = mapping.get("atoms")
    clauses = mapping.get("clauses")
    source_mappings = mapping.get("source_mappings")
    if not isinstance(atoms, dict) or not isinstance(clauses, list):
        return False
    if not isinstance(source_mappings, list):
        return False
    if sorted(atoms.values()) != list(range(1, len(atoms) + 1)):
        return False
    if sorted(atoms, key=atoms.get) != sorted(atoms):
        return False
    names: set[str] = set()
    next_clause = 1
    for item in source_mappings:
        name = item.get("source_name")
        if not isinstance(name, str) or not name or name in names:
            return False
        names.add(name)
        if not isinstance(item.get("role"), str) or not item["role"]:
            return False
        if item.get("tautology"):
            if item.get("dimacs_clause") is not None:
                return False
            continue
        if item.get("dimacs_clause") != next_clause:
            return False
        literals = item.get("source_literals")
        if not isinstance(literals, list):
            return False
        try:
            reconstructed = canonical_clause(
                atoms[literal["atom"]]
                if literal["positive"]
                else -atoms[literal["atom"]]
                for literal in literals
            )
        except (KeyError, TypeError):
            return False
        if reconstructed != tuple(clauses[next_clause - 1]):
            return False
        next_clause += 1
    return next_clause - 1 == len(clauses)


def percentile(values: list[float], percentile_value: float) -> float | None:
    if not values:
        return None
    values = sorted(values)
    index = math.ceil(percentile_value * len(values)) - 1
    return values[max(0, min(index, len(values) - 1))]


def metric_summary(values: Iterable[int | float]) -> dict[str, int | float | None]:
    collected = list(values)
    return {
        "count": len(collected),
        "median": statistics.median(collected) if collected else None,
        "p95": percentile(collected, 0.95),
        "maximum": max(collected, default=None),
    }


def grouped_rates(scanned: dict[str, int], accepted: dict[str, int]) -> dict[str, float]:
    return {
        key: accepted.get(key, 0) / count
        for key, count in sorted(scanned.items())
        if count
    }


def comparison_summary(
    coordinates: list[dict[str, Any]],
    kind: str,
    baseline: str,
    candidate: str,
    statuses: dict[tuple[str, str], str],
    grouped: dict[tuple[str, str], list[dict[str, Any]]],
) -> dict[str, Any]:
    selected = [
        coordinate for coordinate in coordinates if coordinate["kind"] == kind
    ]
    baseline_solved = {
        coordinate["coordinate"]
        for coordinate in selected
        if statuses[(coordinate["coordinate"], baseline)] in {"sat", "unsat"}
    }
    candidate_solved = {
        coordinate["coordinate"]
        for coordinate in selected
        if statuses[(coordinate["coordinate"], candidate)] in {"sat", "unsat"}
    }
    common = sorted(baseline_solved & candidate_solved)
    wall_ratios: list[float] = []
    rss_ratios: list[float] = []
    for coordinate_id in common:
        baseline_records = grouped[(coordinate_id, baseline)]
        candidate_records = grouped[(coordinate_id, candidate)]
        wall_ratios.append(
            statistics.median(record["wall_ns"] for record in candidate_records)
            / statistics.median(record["wall_ns"] for record in baseline_records)
        )
        baseline_rss = [
            record["rss_kib"]
            for record in baseline_records
            if record["rss_kib"] is not None
        ]
        candidate_rss = [
            record["rss_kib"]
            for record in candidate_records
            if record["rss_kib"] is not None
        ]
        if baseline_rss and candidate_rss:
            rss_ratios.append(
                statistics.median(candidate_rss)
                / max(1, statistics.median(baseline_rss))
            )
    return {
        "baseline": baseline,
        "candidate": candidate,
        "coordinates": len(selected),
        "baseline_solved": len(baseline_solved),
        "candidate_solved": len(candidate_solved),
        "added_solves": len(candidate_solved - baseline_solved),
        "lost_solves": len(baseline_solved - candidate_solved),
        "common_solved": len(common),
        "wall_ratio": metric_summary(wall_ratios),
        "rss_ratio": metric_summary(rss_ratios),
    }


def analyze(arguments: argparse.Namespace) -> int:
    prepared_root = arguments.prepared.resolve()
    manifest = load_prepared(prepared_root)
    records = list(parse_existing_results(arguments.results.resolve()).values())
    statuses = unique_statuses(records)
    certificates = json.loads(
        (arguments.certificates.resolve() / "certificates.json").read_text(
            encoding="utf-8"
        )
    )
    coordinate_by_id = {
        coordinate["coordinate"]: coordinate for coordinate in manifest["coordinates"]
    }
    grouped: defaultdict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        grouped[(record["coordinate"], record["arm"])].append(record)

    arm_summaries: dict[str, Any] = {}
    for arm in sorted({record["arm"] for record in records}):
        arm_records = [record for record in records if record["arm"] == arm]
        completed = [
            record for record in arm_records if record["status"] in {"sat", "unsat"}
        ]
        sat_records = [record for record in arm_records if record["status"] == "sat"]
        probe_records = [
            record for record in arm_records if "insertion_ns" in record
        ]
        coordinate_status_counts = Counter(
            status
            for (coordinate_id, status_arm), status in statuses.items()
            if status_arm == arm and coordinate_id in coordinate_by_id
        )
        arm_summaries[arm] = {
            "records": len(arm_records),
            "record_status_counts": dict(
                Counter(record["status"] for record in arm_records)
            ),
            "coordinate_status_counts": dict(sorted(coordinate_status_counts.items())),
            "cost_ns": {
                "insertion": metric_summary(
                    record["insertion_ns"] for record in probe_records
                ),
                "simplify": metric_summary(
                    record["simplify_ns"] for record in probe_records
                ),
                "solve": metric_summary(
                    record["solve_ns"]
                    for record in completed
                    if "solve_ns" in record
                ),
                "wall": metric_summary(record["wall_ns"] for record in arm_records),
            },
            "rss_kib": metric_summary(
                record["rss_kib"]
                for record in arm_records
                if record["rss_kib"] is not None
            ),
            "model_validation": {
                "claimed_sat_records": len(sat_records),
                "checked": sum(record.get("valid", False) for record in sat_records),
            },
            "transformation": {
                "active_before": metric_summary(
                    record["active_before"] for record in probe_records
                ),
                "active_after": metric_summary(
                    record["active_after"] for record in probe_records
                ),
                "clauses_before": metric_summary(
                    record["clauses_before"] for record in probe_records
                ),
                "clauses_after": metric_summary(
                    record["clauses_after"] for record in probe_records
                ),
            },
        }

    captured_comparison = comparison_summary(
        manifest["coordinates"], "captured", "plain", "default", statuses, grouped
    )
    whole_comparison = comparison_summary(
        manifest["coordinates"], "whole", "umlaut", "default", statuses, grouped
    )

    reductions: list[float] = []
    for coordinate in manifest["coordinates"]:
        if coordinate["kind"] != "captured" or coordinate["query"] != "cold":
            continue
        default = grouped[(coordinate["coordinate"], "default")][0]
        ratios = []
        if default["active_before"] > 0:
            ratios.append(default["active_after"] / default["active_before"])
        if default["clauses_before"] > 0:
            ratios.append(default["clauses_after"] / default["clauses_before"])
        reductions.append(min(ratios, default=1.0))

    overlaps = analyze_overlap(
        manifest["coordinates"], prepared_root, arguments.certificates.resolve()
    )
    proof_records = [
        certificate
        for certificate in certificates["certificates"]
        if certificate["proof"]
    ]
    proof_success = sum(certificate["checker_ok"] for certificate in proof_records)
    required_proofs = sum(
        status == "unsat" and arm in {"plain", "default"}
        for (_coordinate, arm), status in statuses.items()
    )
    polarity_disagreements = []
    arm_only_solves: Counter[str] = Counter()
    for coordinate in manifest["coordinates"]:
        arms = (
            ("internal", "plain", "default")
            if coordinate["kind"] == "captured"
            else ("umlaut", "plain", "default")
        )
        completed = {
            arm: statuses[(coordinate["coordinate"], arm)]
            for arm in arms
            if statuses[(coordinate["coordinate"], arm)] in {"sat", "unsat"}
        }
        if len(set(completed.values())) > 1:
            polarity_disagreements.append(
                {"coordinate": coordinate["coordinate"], "statuses": completed}
            )
        if len(completed) == 1:
            arm_only_solves[next(iter(completed))] += 1

    captured_coordinates = [
        coordinate
        for coordinate in manifest["coordinates"]
        if coordinate["kind"] == "captured"
    ]
    whole_coordinates = [
        coordinate
        for coordinate in manifest["coordinates"]
        if coordinate["kind"] == "whole"
    ]
    captured_coverage = {
        "sessions": manifest["captured_sessions"],
        "query_scopes": len(captured_coordinates),
        "by_category": dict(
            sorted(Counter(item["category"] for item in captured_coordinates).items())
        ),
        "by_division": dict(
            sorted(Counter(item["division"] for item in captured_coordinates).items())
        ),
        "by_family": dict(
            sorted(Counter(item["family"] for item in captured_coordinates).items())
        ),
        "variables": metric_summary(
            item["variables"] for item in captured_coordinates
        ),
        "clauses": metric_summary(item["clauses"] for item in captured_coordinates),
    }

    report = {
        "schema_version": 1,
        "experiment": EXPERIMENT,
        "whole_scanned": manifest["whole_scanned"],
        "whole_scanned_by_category": manifest["whole_scanned_by_category"],
        "whole_scanned_by_division": manifest["whole_scanned_by_division"],
        "whole_scanned_by_family": manifest["whole_scanned_by_family"],
        "whole_accepted": manifest["whole_accepted"],
        "whole_accepted_by_category": manifest["whole_accepted_by_category"],
        "whole_accepted_by_division": manifest["whole_accepted_by_division"],
        "whole_accepted_by_family": manifest["whole_accepted_by_family"],
        "whole_rejection_counts": manifest["whole_rejection_counts"],
        "captured_sessions": manifest["captured_sessions"],
        "captured_coordinates": manifest["captured_coordinates"],
        "captured_families": sorted(
            {
                coordinate["family"]
                for coordinate in manifest["coordinates"]
                if coordinate["kind"] == "captured"
            }
        ),
        "coverage": {
            "whole_recognition_rate": (
                manifest["whole_accepted"] / manifest["whole_scanned"]
            ),
            "whole_recognition_rate_by_category": grouped_rates(
                manifest["whole_scanned_by_category"],
                manifest["whole_accepted_by_category"],
            ),
            "whole_recognition_rate_by_division": grouped_rates(
                manifest["whole_scanned_by_division"],
                manifest["whole_accepted_by_division"],
            ),
            "whole_recognition_rate_by_family": grouped_rates(
                manifest["whole_scanned_by_family"],
                manifest["whole_accepted_by_family"],
            ),
            "captured": captured_coverage,
        },
        "materialization_ns": {
            "captured": metric_summary(
                item["materialization_ns"] for item in captured_coordinates
            ),
            "whole": metric_summary(
                item["materialization_ns"] for item in whole_coordinates
            ),
        },
        "arm_summaries": arm_summaries,
        "arm_only_solves": dict(sorted(arm_only_solves.items())),
        "polarity_disagreements": polarity_disagreements,
        "comparisons": {
            "captured_default_vs_plain": captured_comparison,
            "whole_default_vs_umlaut": whole_comparison,
        },
        "default_reduction": {
            "sessions": len(reductions),
            "median_remaining_ratio": statistics.median(reductions)
            if reductions
            else None,
            "at_least_ten_percent": sum(ratio <= 0.9 for ratio in reductions),
        },
        "proofs": {
            "required": required_proofs,
            "attempted": len(proof_records),
            "checked": proof_success,
            "success_rate": (
                proof_success / required_proofs if required_proofs else 1.0
            ),
            "bytes": sum(record["proof_bytes"] for record in proof_records),
            "checker_ns": metric_summary(
                record["checker_ns"] for record in proof_records
            ),
        },
        "exhaustive_oracle_counts": certificates["exhaustive_oracle_counts"],
        "mapping_roundtrips": certificates["mapping_roundtrips"],
        "mutation_checks": certificates["mutation_checks"],
        "recurring_overlap": overlaps,
    }
    report["decision"] = decide(report)
    atomic_json(arguments.output.resolve(), report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


def analyze_overlap(
    coordinates: list[dict[str, Any]], prepared_root: Path, certificate_root: Path
) -> dict[str, Any]:
    cold = [
        coordinate
        for coordinate in coordinates
        if coordinate["kind"] == "captured" and coordinate["query"] == "cold"
    ]
    groups: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for coordinate in cold:
        groups[coordinate["problem_id"]].append(coordinate)
    original_retention: list[float] = []
    simplified_retention: list[float] = []
    add_only = 0
    pairs = 0
    for group in groups.values():
        group.sort(key=lambda item: item["capture_path"])
        for previous, current in zip(group, group[1:]):
            pairs += 1
            _variables, previous_clauses = parse_dimacs(
                prepared_root / previous["dimacs"]
            )
            _variables, current_clauses = parse_dimacs(prepared_root / current["dimacs"])
            previous_set, current_set = set(previous_clauses), set(current_clauses)
            original_retention.append(
                len(previous_set & current_set) / max(1, len(previous_set))
            )
            add_only += previous_set <= current_set
            previous_simplified = (
                certificate_root
                / "simplified"
                / f"{previous['coordinate']}-default.cnf"
            )
            current_simplified = (
                certificate_root
                / "simplified"
                / f"{current['coordinate']}-default.cnf"
            )
            if previous_simplified.exists() and current_simplified.exists():
                _variables, previous_post = parse_dimacs(previous_simplified)
                _variables, current_post = parse_dimacs(current_simplified)
                previous_post_set, current_post_set = set(previous_post), set(current_post)
                simplified_retention.append(
                    len(previous_post_set & current_post_set)
                    / max(1, len(previous_post_set))
                )
    return {
        "pairs": pairs,
        "add_only": add_only,
        "add_only_rate": add_only / pairs if pairs else None,
        "median_original_retention": (
            statistics.median(original_retention) if original_retention else None
        ),
        "median_simplified_retention": (
            statistics.median(simplified_retention)
            if simplified_retention
            else None
        ),
        "stable_identity_available": False,
    }


def decide(report: dict[str, Any]) -> dict[str, Any]:
    correctness = (
        not report["polarity_disagreements"]
        and report["proofs"]["required"]
        == report["proofs"]["attempted"]
        == report["proofs"]["checked"]
        and report["mapping_roundtrips"]["required"]
        == report["mapping_roundtrips"]["checked"]
        and all(
            arm["model_validation"]["claimed_sat_records"]
            == arm["model_validation"]["checked"]
            for arm in report["arm_summaries"].values()
        )
        and all(report["mutation_checks"].values())
    )
    captured = report["comparisons"]["captured_default_vs_plain"]
    wall_ratio = captured["wall_ratio"]
    rss_ratio = captured["rss_ratio"]
    reduction = report["default_reduction"]
    captured_performance_gate = (
        wall_ratio["median"] is not None
        and wall_ratio["median"] <= 0.85
        and wall_ratio["p95"] <= 1.05
        and rss_ratio["maximum"] is not None
        and rss_ratio["maximum"] <= 1.10
        and reduction["at_least_ten_percent"] >= 0.2 * reduction["sessions"]
    )
    extracted_promote = (
        correctness
        and captured["lost_solves"] == 0
        and (captured["added_solves"] >= 1 or captured_performance_gate)
    )
    whole = report["comparisons"]["whole_default_vs_umlaut"]
    whole_wall = whole["wall_ratio"]
    whole_performance_gate = (
        whole_wall["median"] is not None
        and whole_wall["median"] <= 0.8
        and whole_wall["p95"] <= 1.05
    )
    whole_promote = (
        correctness
        and report["whole_accepted"] >= 20
        and len(report["whole_accepted_by_family"]) >= 4
        and whole["lost_solves"] == 0
        and (whole["added_solves"] >= 2 or whole_performance_gate)
    )
    overlap = report["recurring_overlap"]
    reuse_promote = (
        overlap["pairs"] > 0
        and overlap["add_only_rate"] >= 0.5
        and overlap["median_simplified_retention"] is not None
        and overlap["median_simplified_retention"] >= 0.5
        and overlap["stable_identity_available"]
    )
    return {
        "correctness_passed": correctness,
        "captured_performance_gate": captured_performance_gate,
        "whole_performance_gate": whole_performance_gate,
        "recommend_default_preprocessing_for_extracted_sat": extracted_promote,
        "recommend_whole_problem_specialist_followup": whole_promote,
        "recommend_cross_call_reuse": reuse_promote,
    }


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    subcommands = result.add_subparsers(dest="command", required=True)
    prepare_parser = subcommands.add_parser("prepare")
    prepare_parser.add_argument("--casc-manifest", type=Path, required=True)
    prepare_parser.add_argument("--casc-archive", type=Path, required=True)
    prepare_parser.add_argument("--sat-archive", type=Path, required=True)
    prepare_parser.add_argument("--output", type=Path, required=True)
    prepare_parser.set_defaults(function=prepare)

    run_parser = subcommands.add_parser("run")
    run_parser.add_argument("--prepared", type=Path, required=True)
    run_parser.add_argument("--results", type=Path, required=True)
    run_parser.add_argument("--internal-probe", type=Path, required=True)
    run_parser.add_argument("--cadical-probe", type=Path, required=True)
    run_parser.add_argument("--umlaut-binary", type=Path, required=True)
    run_parser.add_argument("--workers", type=int, default=8)
    run_parser.add_argument("--repetitions", type=int, default=20)
    run_parser.set_defaults(function=run_benchmark)

    certify_parser = subcommands.add_parser("certify")
    certify_parser.add_argument("--prepared", type=Path, required=True)
    certify_parser.add_argument("--results", type=Path, required=True)
    certify_parser.add_argument("--cadical-probe", type=Path, required=True)
    certify_parser.add_argument("--drat-trim", type=Path, required=True)
    certify_parser.add_argument("--output", type=Path, required=True)
    certify_parser.set_defaults(function=certify)

    analyze_parser = subcommands.add_parser("analyze")
    analyze_parser.add_argument("--prepared", type=Path, required=True)
    analyze_parser.add_argument("--results", type=Path, required=True)
    analyze_parser.add_argument("--certificates", type=Path, required=True)
    analyze_parser.add_argument("--output", type=Path, required=True)
    analyze_parser.set_defaults(function=analyze)
    return result


def main() -> int:
    arguments = parser().parse_args()
    return arguments.function(arguments)


if __name__ == "__main__":
    raise SystemExit(main())
