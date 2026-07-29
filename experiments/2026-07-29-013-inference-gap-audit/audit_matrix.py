#!/usr/bin/env python3
"""Validate the inference-gap matrix and optionally run focused Rust witnesses."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
MATRIX_PATH = EXPERIMENT_ROOT / "capability-matrix.json"
ALLOWED_STATUSES = {
    "direct",
    "library_only",
    "partial",
    "missing",
    "owned_elsewhere",
}


class AuditError(RuntimeError):
    """The capability matrix is stale or an executable witness failed."""


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def read_text(repo_root: Path, relative: str) -> str:
    path = repo_root / relative
    if not path.is_file():
        raise AuditError(f"missing evidence file: {relative}")
    return path.read_text(encoding="utf-8")


def test_definition_count(source: str, test_filter: str) -> int:
    pattern = re.compile(
        rf"#\s*\[\s*test\s*\]\s*"
        rf"(?:#\s*\[[^\]]+\]\s*)*"
        rf"fn\s+{re.escape(test_filter)}\s*\(",
        re.MULTILINE,
    )
    return len(pattern.findall(source))


def validate_direct_row(repo_root: Path, row: dict[str, Any]) -> dict[str, Any]:
    markers: list[dict[str, Any]] = []
    for relative, marker in row.get("route_markers", []):
        source = read_text(repo_root, relative)
        count = source.count(marker)
        if count < 1:
            raise AuditError(
                f"{row['id']}: route marker {marker!r} missing from {relative}"
            )
        markers.append({"path": relative, "marker": marker, "count": count})

    proof_operation = row.get("proof_operation")
    proof_evidence = None
    if proof_operation is not None:
        relative, marker = proof_operation
        source = read_text(repo_root, relative)
        count = source.count(marker)
        if count < 1:
            raise AuditError(
                f"{row['id']}: proof marker {marker!r} missing from {relative}"
            )
        proof_evidence = {"path": relative, "marker": marker, "count": count}

    filters = [row["test_filter"]]
    secondary = row.get("secondary_test_filter")
    if secondary:
        filters.append(secondary)
    located: list[dict[str, Any]] = []
    rust_sources = list((repo_root / "src").rglob("*.rs"))
    for test_filter in filters:
        matches = []
        for path in rust_sources:
            source = path.read_text(encoding="utf-8")
            count = test_definition_count(source, test_filter)
            if count:
                matches.extend([path] * count)
        if len(matches) != 1:
            raise AuditError(
                f"{row['id']}: expected exactly one #[test] named "
                f"{test_filter}, found {len(matches)}"
            )
        located.append(
            {
                "filter": test_filter,
                "path": matches[0].relative_to(repo_root).as_posix(),
            }
        )
    return {
        "route_markers": markers,
        "proof_operation": proof_evidence,
        "tests": located,
    }


def validate_boundary_row(repo_root: Path, row: dict[str, Any]) -> dict[str, Any]:
    source = "\n".join(
        path.read_text(encoding="utf-8", errors="replace")
        for path in sorted((repo_root / "src").rglob("*.rs"))
    ).lower()
    tokens = []
    for token in row.get("absence_tokens", []):
        count = source.count(token.lower())
        if count:
            raise AuditError(
                f"{row['id']}: absence token {token!r} now occurs {count} "
                "time(s); review the semantic classification"
            )
        tokens.append({"token": token, "count": 0})

    prior = row.get("prior_executable_evidence")
    prior_evidence = None
    if prior:
        path = repo_root / prior
        if not path.is_file():
            raise AuditError(f"{row['id']}: missing prior evidence {prior}")
        payload = path.read_bytes()
        prior_evidence = {
            "path": prior,
            "bytes": len(payload),
            "sha256": sha256_bytes(payload),
        }
    return {"absence_tokens": tokens, "prior_evidence": prior_evidence}


def run_focused_test(
    repo_root: Path, test_filter: str, env: dict[str, str]
) -> dict[str, Any]:
    command = [
        "cargo",
        "test",
        "--locked",
        "--lib",
        test_filter,
        "--",
        "--exact",
    ]
    # Cargo's exact matcher needs the module path, whereas the matrix stores a
    # stable leaf name. Fall back to substring matching while still requiring
    # the harness to report exactly one test.
    command.pop()
    completed = subprocess.run(
        command,
        cwd=repo_root,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    stdout = completed.stdout.decode("utf-8", errors="replace")
    stderr = completed.stderr.decode("utf-8", errors="replace")
    running = re.findall(r"^running (\d+) test", stdout, re.MULTILINE)
    executed = sum(int(value) for value in running)
    passed = re.search(
        r"test result: ok\. 1 passed; 0 failed;", stdout
    ) is not None
    if completed.returncode != 0 or executed != 1 or not passed:
        raise AuditError(
            f"focused test {test_filter} failed contract: "
            f"return={completed.returncode}, executed={executed}"
        )
    return {
        "filter": test_filter,
        "command": command,
        "return_code": completed.returncode,
        "executed_tests": executed,
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
    }


def validate_matrix(
    repo_root: Path, matrix: dict[str, Any], run_tests: bool
) -> dict[str, Any]:
    rows = matrix.get("rows")
    if not isinstance(rows, list) or not rows:
        raise AuditError("matrix has no rows")
    ids = [row.get("id") for row in rows]
    if any(not isinstance(row_id, str) or not row_id for row_id in ids):
        raise AuditError("every row needs a non-empty string id")
    if len(set(ids)) != len(ids):
        raise AuditError("matrix row ids are not unique")
    if any(row.get("status") not in ALLOWED_STATUSES for row in rows):
        raise AuditError("matrix contains an unsupported status")

    shortlist = matrix.get("shortlist")
    if not isinstance(shortlist, list) or not 1 <= len(shortlist) <= 3:
        raise AuditError("shortlist must contain one to three entries")
    shortlist_ids = [entry.get("id") for entry in shortlist]
    if any(row_id not in ids for row_id in shortlist_ids):
        raise AuditError("shortlist references an unknown row")
    if [entry.get("rank") for entry in shortlist] != list(
        range(1, len(shortlist) + 1)
    ):
        raise AuditError("shortlist ranks are not contiguous")

    evidence: dict[str, Any] = {}
    filters: list[str] = []
    for row in rows:
        required = (
            "reference_operation",
            "soundness_preconditions",
            "umlaut_equivalent",
            "cli_reachability",
        )
        if any(not row.get(field) for field in required):
            raise AuditError(f"{row['id']}: incomplete semantic row")
        if row["status"] in {"direct", "library_only"}:
            direct = validate_direct_row(repo_root, row)
            evidence[row["id"]] = direct
            filters.extend(test["filter"] for test in direct["tests"])
        else:
            evidence[row["id"]] = validate_boundary_row(repo_root, row)

    test_results: list[dict[str, Any]] = []
    if run_tests:
        if sys.platform != "linux":
            raise AuditError("--run-tests is restricted to the Ubuntu worker")
        env = dict(os.environ)
        env.setdefault("RUST_BACKTRACE", "1")
        for test_filter in filters:
            test_results.append(run_focused_test(repo_root, test_filter, env))

    counts = {
        status: sum(row["status"] == status for row in rows)
        for status in sorted(ALLOWED_STATUSES)
    }
    report = {
        "schema_version": 1,
        "baseline_commit": matrix["baseline_commit"],
        "matrix_sha256": sha256_bytes(MATRIX_PATH.read_bytes()),
        "row_count": len(rows),
        "status_counts": counts,
        "shortlist": shortlist,
        "evidence": evidence,
        "focused_test_count": len(filters),
        "focused_tests_run": run_tests,
        "focused_test_results": test_results,
    }
    canonical = json.dumps(
        report, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    report["report_id"] = sha256_bytes(canonical)
    return report


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--run-tests", action="store_true")
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    report = validate_matrix(repo_root, matrix, arguments.run_tests)
    payload = json.dumps(
        report, indent=2, sort_keys=True, ensure_ascii=False
    ) + "\n"
    if arguments.output:
        output = arguments.output.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        temporary = output.with_name(f".{output.name}.{os.getpid()}.tmp")
        temporary.write_text(payload, encoding="utf-8")
        temporary.replace(output)
    print(
        f"OK: {report['row_count']} rows, "
        f"{report['focused_test_count']} focused tests, "
        f"report {report['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AuditError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
