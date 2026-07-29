#!/usr/bin/env python3
"""Run the minimized and held-out TSTP ancestry repair matrix on Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Sequence


PROOFCHECK_SHA256 = (
    "92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e"
)
SZS_STATUS_RE = re.compile(
    r"^[%#]\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)\b",
    re.MULTILINE | re.IGNORECASE,
)
SUCCESS_STATUSES = {"unsatisfiable", "contradictoryaxioms", "theorem"}
START_MARKER = "% SZS output start CNFRefutation"
END_MARKER = "% SZS output end CNFRefutation"


class ExperimentError(RuntimeError):
    """The experiment failed a frozen integrity or correctness gate."""


@dataclass(frozen=True)
class Case:
    name: str
    problem: Path
    cpu_seconds: int
    wall_seconds: int
    options: tuple[str, ...] = ()
    expected_proofcheck: str = "verifiedgood"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def parse_status(output: str) -> str:
    matches = SZS_STATUS_RE.findall(output)
    return matches[-1].lower() if matches else "missing"


def annotated_body_span(line: str) -> tuple[int, int]:
    open_index = line.find("(")
    if open_index < 0:
        raise ExperimentError("annotated formula has no opening parenthesis")
    depth = 0
    quoted = False
    commas: list[int] = []
    index = open_index + 1
    while index < len(line):
        char = line[index]
        if quoted:
            if char == "'":
                if index + 1 < len(line) and line[index + 1] == "'":
                    index += 2
                    continue
                quoted = False
        elif char == "'":
            quoted = True
        elif char == "(" or char == "[":
            depth += 1
        elif char == ")" or char == "]":
            if depth == 0:
                break
            depth -= 1
        elif char == "," and depth == 0:
            commas.append(index)
            if len(commas) == 3:
                break
        index += 1
    if len(commas) < 3:
        raise ExperimentError("annotated formula has fewer than four arguments")
    return commas[1] + 1, commas[2]


def mutate_annotated_body(line: str) -> str:
    start, end = annotated_body_span(line)
    leading = len(line[start:end]) - len(line[start:end].lstrip())
    trailing = len(line[start:end]) - len(line[start:end].rstrip())
    return (
        line[: start + leading]
        + "umlaut_mutation_symbol"
        + (" " * trailing)
        + line[end:]
    )


def leaf_line_indexes(proof: str) -> list[int]:
    return [
        index
        for index, line in enumerate(proof.splitlines())
        if re.match(r"^\s*(?:cnf|fof|tff|tcf|thf)\s*\(", line, re.I)
        and "file(" in line
    ]


def definition_line_indexes(proof: str) -> list[int]:
    return [
        index
        for index, line in enumerate(proof.splitlines())
        if "introduced(definition,[new_symbols(definition,[" in line
    ]


def mutate_proof_line(proof: str, line_index: int) -> str:
    lines = proof.splitlines()
    lines[line_index] = mutate_annotated_body(lines[line_index])
    return "\n".join(lines) + "\n"


def truncate_refutation(proof: str) -> str:
    if END_MARKER not in proof:
        raise ExperimentError("proof has no CNFRefutation end marker")
    return proof.replace(END_MARKER, "", 1)


def run_capture(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout: float,
) -> tuple[subprocess.CompletedProcess[bytes], float]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    return completed, time.monotonic() - started


def write_process_artifacts(
    root: Path,
    stem: str,
    completed: subprocess.CompletedProcess[bytes],
) -> None:
    (root / f"{stem}.stdout.txt").write_bytes(completed.stdout)
    (root / f"{stem}.stderr.txt").write_bytes(completed.stderr)


def run_prover(binary: Path, case: Case, case_root: Path) -> dict[str, Any]:
    command = [
        str(binary),
        "--auto",
        "--silent",
        "--tstp-out",
        "--proof-object=1",
        f"--cpu-limit={case.cpu_seconds}",
        "--memory-limit=2048",
        *case.options,
        str(case.problem),
    ]
    completed, wall_seconds = run_capture(
        command,
        cwd=binary.parent,
        timeout=case.wall_seconds,
    )
    solution = completed.stdout.decode("utf-8", errors="replace")
    solution_path = case_root / "solution.txt"
    solution_path.write_text(solution, encoding="utf-8")
    (case_root / "prover.stderr.txt").write_bytes(completed.stderr)
    status = parse_status(solution)
    if completed.returncode != 0:
        raise ExperimentError(
            f"{case.name}: prover exited {completed.returncode} with {status}"
        )
    if status not in SUCCESS_STATUSES:
        raise ExperimentError(f"{case.name}: prover status is {status}")
    if solution.count(START_MARKER) != 1 or solution.count(END_MARKER) != 1:
        raise ExperimentError(f"{case.name}: incomplete proof framing")
    return {
        "command": command,
        "return_code": completed.returncode,
        "status": status,
        "wall_seconds": wall_seconds,
        "solution_bytes": len(completed.stdout),
        "solution_sha256": sha256_file(solution_path),
    }


def proofcheck(
    checker: Path,
    problem: Path,
    proof: Path,
    root: Path,
    stem: str,
) -> dict[str, Any]:
    completed, wall_seconds = run_capture(
        [
            str(checker),
            "-j",
            "4",
            "-t",
            "3",
            "-T",
            "180",
            "-p",
            str(problem),
            str(proof),
        ],
        cwd=checker.parent,
        timeout=190,
    )
    write_process_artifacts(root, stem, completed)
    combined = (
        completed.stdout.decode("utf-8", errors="replace")
        + "\n"
        + completed.stderr.decode("utf-8", errors="replace")
    )
    return {
        "return_code": completed.returncode,
        "status": parse_status(combined),
        "wall_seconds": wall_seconds,
        "stdout_sha256": sha256_file(root / f"{stem}.stdout.txt"),
        "stderr_sha256": sha256_file(root / f"{stem}.stderr.txt"),
    }


def validation_gate(
    repo_root: Path,
    checker: Path,
    problem: Path,
    solution: Path,
    root: Path,
    stem: str,
) -> dict[str, Any]:
    report_path = root / f"{stem}.json"
    completed, wall_seconds = run_capture(
        [
            sys.executable,
            str(repo_root / "tools/validation/validate_tptp_solution.py"),
            str(problem),
            str(solution),
            "--proof-command-json",
            json.dumps(
                [
                    str(checker),
                    "-j",
                    "4",
                    "-T",
                    "180",
                    "-p",
                    "{problem}",
                    "{artifact}",
                ]
            ),
            "--report",
            str(report_path),
        ],
        cwd=repo_root,
        timeout=190,
    )
    write_process_artifacts(root, stem, completed)
    return {
        "return_code": completed.returncode,
        "wall_seconds": wall_seconds,
        "report": (
            json.loads(report_path.read_text(encoding="utf-8"))
            if report_path.is_file()
            else None
        ),
    }


def run_positive_case(
    repo_root: Path,
    binary: Path,
    checker: Path,
    case: Case,
    artifact_root: Path,
) -> dict[str, Any]:
    case_root = artifact_root / case.name
    case_root.mkdir(parents=True, exist_ok=True)
    prover = run_prover(binary, case, case_root)
    solution_path = case_root / "solution.txt"
    external = proofcheck(
        checker,
        case.problem,
        solution_path,
        case_root,
        "proofcheck",
    )
    if external["status"] != case.expected_proofcheck:
        raise ExperimentError(
            f"{case.name}: ProofCheck returned {external['status']}, "
            f"expected {case.expected_proofcheck}"
        )
    gate = validation_gate(
        repo_root,
        checker,
        case.problem,
        solution_path,
        case_root,
        "validation",
    )
    expected_gate_code = (
        0 if case.expected_proofcheck == "verifiedgood" else 2
    )
    if gate["return_code"] != expected_gate_code:
        raise ExperimentError(
            f"{case.name}: validation gate exited {gate['return_code']}, "
            f"expected {expected_gate_code}"
        )
    return {
        "case": asdict(case),
        "problem_sha256": sha256_file(case.problem),
        "prover": prover,
        "proofcheck": external,
        "validation": gate,
    }


def run_mutations(
    checker: Path,
    problem: Path,
    solution: Path,
    root: Path,
    include_source: bool,
    include_definition: bool,
) -> list[dict[str, Any]]:
    proof = solution.read_text(encoding="utf-8")
    indexes = leaf_line_indexes(proof) if include_source else []
    if include_definition:
        indexes.extend(definition_line_indexes(proof))
    if not indexes:
        raise ExperimentError(f"{solution}: no mutation candidates")
    results = []
    for ordinal, index in enumerate(indexes, start=1):
        mutated_path = root / f"mutation-{ordinal:02d}.txt"
        mutated_path.write_text(
            mutate_proof_line(proof, index),
            encoding="utf-8",
        )
        result = proofcheck(
            checker,
            problem,
            mutated_path,
            root,
            f"mutation-{ordinal:02d}.proofcheck",
        )
        if result["status"] != "verifiedbad":
            raise ExperimentError(
                f"{solution}: mutation {ordinal} returned {result['status']}"
            )
        results.append(
            {
                "line_index": index,
                "proof_sha256": sha256_file(mutated_path),
                "proofcheck": result,
            }
        )
    return results


def run_truncation_gate(
    repo_root: Path,
    checker: Path,
    problem: Path,
    solution: Path,
    root: Path,
) -> dict[str, Any]:
    truncated_path = root / "truncated.txt"
    truncated_path.write_text(
        truncate_refutation(solution.read_text(encoding="utf-8")),
        encoding="utf-8",
    )
    result = validation_gate(
        repo_root,
        checker,
        problem,
        truncated_path,
        root,
        "truncated.validation",
    )
    if result["return_code"] not in {1, 3}:
        raise ExperimentError(
            "truncated proof was not rejected by the validation gate"
        )
    return {
        "proof_sha256": sha256_file(truncated_path),
        "validation": result,
    }


def run_interrupted_output_case(
    binary: Path,
    problem: Path,
    artifact_root: Path,
) -> dict[str, Any]:
    root = artifact_root / "interrupted-proof-render"
    root.mkdir(parents=True, exist_ok=True)
    command = [
        str(binary),
        "--auto",
        "--silent",
        "--tstp-out",
        "--proof-object=1",
        "--cpu-limit=10",
        "--memory-limit=2048",
        str(problem),
    ]
    environment = os.environ.copy()
    environment["UMLAUT_TEST_PROOF_RENDER_DELAY_MS"] = "5000"
    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            cwd=binary.parent,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=1,
            env=environment,
        )
    except subprocess.TimeoutExpired as error:
        completed = subprocess.CompletedProcess(
            command,
            -9,
            error.stdout or b"",
            error.stderr or b"",
        )
    else:
        raise ExperimentError("proof-render fault injection did not interrupt")
    wall_seconds = time.monotonic() - started
    write_process_artifacts(root, "interrupted", completed)
    output = completed.stdout.decode("utf-8", errors="replace")
    errors = completed.stderr.decode("utf-8", errors="replace")
    if "% UMLAUT test proof-render delay active" not in errors:
        raise ExperimentError("proof-render fault injection was not reached")
    status = parse_status(output)
    if status in SUCCESS_STATUSES:
        raise ExperimentError("interrupted run published a success status")
    if START_MARKER in output or END_MARKER in output:
        raise ExperimentError("interrupted run published a proof block marker")
    return {
        "command": command,
        "return_code": completed.returncode,
        "status": status,
        "wall_seconds": wall_seconds,
        "stdout_sha256": sha256_file(root / "interrupted.stdout.txt"),
        "stderr_sha256": sha256_file(root / "interrupted.stderr.txt"),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--artifact-root", type=Path, required=True)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--debug-umlaut", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--held-out-root", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    arguments = parse_args()
    if sys.platform != "linux":
        raise ExperimentError("the experiment must run on Linux")
    repo_root = arguments.repo_root.resolve()
    artifact_root = arguments.artifact_root.resolve()
    binary = arguments.umlaut.resolve()
    debug_binary = arguments.debug_umlaut.resolve()
    checker = arguments.proofcheck.resolve()
    held_out = arguments.held_out_root.resolve()
    fixture_root = (
        repo_root
        / "experiments/2026-07-29-009-tstp-input-leaf-provenance/fixtures"
    )
    if (
        not (repo_root / "Cargo.toml").is_file()
        or not binary.is_file()
        or not debug_binary.is_file()
    ):
        raise ExperimentError("repository or Umlaut binary is missing")
    if not checker.is_file() or sha256_file(checker) != PROOFCHECK_SHA256:
        raise ExperimentError("pinned ProofCheck binary hash mismatch")
    artifact_root.mkdir(parents=True, exist_ok=True)

    self_certification, self_cert_seconds = run_capture(
        [str(checker), "-self-certify"],
        cwd=checker.parent,
        timeout=300,
    )
    write_process_artifacts(
        artifact_root,
        "proofcheck-self-certify",
        self_certification,
    )
    self_cert_text = (
        self_certification.stdout + self_certification.stderr
    ).decode(
        "utf-8",
        errors="replace",
    )
    if (
        self_certification.returncode != 0
        or "Tests: 117 run, 117 passed, 0 failed" not in self_cert_text
    ):
        raise ExperimentError("ProofCheck self-certification failed")

    cases = [
        Case("minimized-plain", fixture_root / "plain-source.p", 10, 30),
        Case("minimized-negated", fixture_root / "negated-source.p", 10, 30),
        Case(
            "minimized-definition",
            fixture_root / "definition.p",
            10,
            30,
            ("--definitional-cnf=1",),
            "unknown",
        ),
        Case("held-out-col003-19", held_out / "UEQ/COL003-19.p", 20, 60),
        Case("held-out-syn846-1", held_out / "EPU/SYN846-1.p", 20, 60),
        Case(
            "held-out-puz008-2-static",
            held_out / "EPU/PUZ008-2.p",
            20,
            60,
            (
                "--split-clauses=7",
                "--split-method=2",
                "--split-aggressive",
                "--split-reuse-defs",
            ),
            "unknown",
        ),
        Case("held-out-grp667-4", held_out / "UEQ/GRP667-4.p", 20, 90),
    ]
    for case in cases:
        if not case.problem.is_file():
            raise ExperimentError(f"missing problem: {case.problem}")

    results = [
        run_positive_case(repo_root, binary, checker, case, artifact_root)
        for case in cases
    ]
    definition_gap = proofcheck(
        checker,
        fixture_root / "used-definition-problem.p",
        fixture_root / "used-definition-proof.s",
        artifact_root,
        "used-definition-gap",
    )
    gap_output = "".join(
        (artifact_root / f"used-definition-gap.{stream}.txt").read_text(
            encoding="utf-8", errors="replace"
        )
        for stream in ("stdout", "stderr")
    )
    if (
        definition_gap["status"] != "unknown"
        or "unspecified non-conservative rule" not in gap_output
    ):
        raise ExperimentError("used-definition checker gap changed")
    result_by_name = {result["case"]["name"]: result for result in results}
    mutation_results = {}
    for name in (
        "minimized-plain",
        "minimized-negated",
    ):
        case = next(candidate for candidate in cases if candidate.name == name)
        case_root = artifact_root / name
        mutation_results[name] = run_mutations(
            checker,
            case.problem,
            case_root / "solution.txt",
            case_root,
            include_source=True,
            include_definition=False,
        )
    plain_root = artifact_root / "minimized-plain"
    truncation = run_truncation_gate(
        repo_root,
        checker,
        cases[0].problem,
        plain_root / "solution.txt",
        plain_root,
    )
    interrupted = run_interrupted_output_case(
        debug_binary,
        cases[0].problem,
        artifact_root,
    )

    report = {
        "schema_version": 1,
        "controller_sha256": sha256_file(Path(__file__).resolve()),
        "umlaut_sha256": sha256_file(binary),
        "debug_umlaut_sha256": sha256_file(debug_binary),
        "proofcheck_sha256": sha256_file(checker),
        "proofcheck_self_certification": {
            "return_code": self_certification.returncode,
            "wall_seconds": self_cert_seconds,
            "stdout_sha256": sha256_file(
                artifact_root / "proofcheck-self-certify.stdout.txt"
            ),
            "stderr_sha256": sha256_file(
                artifact_root / "proofcheck-self-certify.stderr.txt"
            ),
        },
        "positive_cases": results,
        "used_definition_checker_gap": definition_gap,
        "mutation_cases": mutation_results,
        "truncation_case": truncation,
        "interrupted_case": interrupted,
    }
    report_path = artifact_root / "report.json"
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True, default=str) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "report": str(report_path),
                "report_sha256": sha256_file(report_path),
                "verified_good_cases": sum(
                    result["proofcheck"]["status"] == "verifiedgood"
                    for result in results
                ),
                "coverage_gap_cases": sum(
                    result["proofcheck"]["status"] == "unknown"
                    for result in results
                ),
                "mutations": sum(map(len, mutation_results.values())),
                "verdict": "pass_with_tracked_definition_gap",
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
