#!/usr/bin/env python3
"""Compare every advertised literal selector through the C and Rust CLIs."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


PCL_LINE = re.compile(r"^\s*(-?\d+) : :(.*?) : (.*)$")
GENERATED_OPS = {
    "er",
    "ef",
    "pm",
    "spm",
    "cs",
    "csr",
    "split",
}
COUNTER_NAMES = (
    "% Processed clauses",
    "% Generated clauses",
    "% Paramodulations",
    "% Factorizations",
    "% Equation resolutions",
    "% Current number of unprocessed clauses",
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def extract_c_names(source: str) -> list[str]:
    start = source.index("static LitSelNameFunAssocCell name_fun_assoc[]")
    end = source.index("{NULL", start)
    return re.findall(r'\{"([^"]+)"', source[start:end])


def extract_rust_names(source: str) -> list[str]:
    start = source.index("pub const LITERAL_SELECTION_NAMES")
    end = source.index("];", start)
    return re.findall(r'"([^"]+)"', source[start:end])


def extract_counters(stdout: str) -> dict[str, int]:
    counters: dict[str, int] = {}
    for line in stdout.splitlines():
        for name in COUNTER_NAMES:
            if line.startswith(name):
                counters[name.removeprefix("% ")] = int(line.rsplit(":", 1)[1])
    if len(counters) != len(COUNTER_NAMES):
        raise RuntimeError(f"missing focused counters: {counters}")
    return counters


def extract_status(stdout: str) -> str:
    statuses = [line for line in stdout.splitlines() if line.startswith("% SZS status ")]
    if len(statuses) != 1:
        raise RuntimeError(f"expected one SZS status, got {statuses}")
    return statuses[0].removeprefix("% SZS status ")


def normalize_clause(clause: str) -> str:
    return clause.replace(" ", "")


def extract_proof_surface(stdout: str) -> tuple[str, list[dict[str, str]]]:
    given: str | None = None
    generated: list[dict[str, str]] = []
    for line in stdout.splitlines():
        match = PCL_LINE.match(line)
        if match is None:
            continue
        _ident, clause, source = match.groups()
        if "'new_given'" in source:
            given = normalize_clause(clause)
            continue
        operation = source.split("(", 1)[0]
        if operation in GENERATED_OPS:
            generated.append(
                {
                    "kind": operation,
                    "clause": normalize_clause(clause),
                }
            )
    if given is None:
        raise RuntimeError("missing new_given proof step")
    return given, generated


def summarize(completed: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    given, generated = extract_proof_surface(stdout)
    return {
        "exit_code": completed.returncode,
        "stderr": stderr,
        "status": extract_status(stdout),
        "given": given,
        "generated": generated,
        "counters": extract_counters(stdout),
    }


def run_batch(exe: str, fixture: str, selectors: list[str]) -> dict[str, dict[str, Any]]:
    common = [
        "--output-level=6",
        "--pcl-out",
        "--print-statistics",
        "--no-preprocessing",
        "--expert-heuristic=(1*FIFOWeight(ConstPrio))",
        "--processed-clauses-limit=1",
        "--detsort-rw",
        "--detsort-new",
    ]
    reports: dict[str, dict[str, Any]] = {}
    for selector in selectors:
        completed = subprocess.run(
            [exe, *common, f"--literal-selection-strategy={selector}", fixture],
            check=False,
            capture_output=True,
            timeout=30,
        )
        reports[selector] = summarize(completed)
    return reports


def canonical_digest(value: Any) -> str:
    rendered = json.dumps(value, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(rendered.encode("utf-8")).hexdigest()


def grouped_behaviors(
    selectors: list[str], reports: dict[str, dict[str, Any]]
) -> list[dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for selector in selectors:
        summary = reports[selector]
        digest = canonical_digest(summary)
        group = groups.setdefault(
            digest,
            {
                "sha256": digest,
                "selectors": [],
                "summary": summary,
            },
        )
        group["selectors"].append(selector)
    return sorted(groups.values(), key=lambda group: group["selectors"][0])


def run_worker(args: argparse.Namespace) -> int:
    selectors = json.loads(sys.stdin.read())
    report = run_batch(args.exe, args.fixture, selectors)
    sys.stdout.write(json.dumps(report, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture")
    parser.add_argument("--repo", type=Path)
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    if args.worker:
        if args.exe is None or args.fixture is None:
            parser.error("--worker requires --exe and --fixture")
        return run_worker(args)

    if args.repo is None or args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --repo, --c-exe, --rust-exe, and --output")

    repo = args.repo.resolve()
    experiment = Path(__file__).resolve().parent
    fixture = experiment / "selection.p"
    c_names = extract_c_names(
        (repo / "eprover/HEURISTICS/che_litselection.c").read_text(encoding="utf-8")
    )
    rust_names = extract_rust_names(
        (repo / "src/heuristics/hcb.rs").read_text(encoding="utf-8")
    )

    rust_reports = run_batch(str(args.rust_exe.resolve()), str(fixture.resolve()), rust_names)
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture",
            windows_to_wsl(fixture),
        ],
        input=json.dumps(c_names).encode("utf-8"),
        check=False,
        capture_output=True,
        timeout=300,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode
    c_reports = json.loads(worker.stdout.decode("utf-8"))

    selector_reports = []
    mismatches: dict[str, Any] = {}
    for selector in rust_names:
        c = c_reports[selector]
        rust = rust_reports[selector]
        exact = c == rust
        selector_reports.append(
            {
                "name": selector,
                "exact": exact,
                "summary_sha256": canonical_digest(c),
            }
        )
        if not exact:
            mismatches[selector] = {"c": c, "rust": rust}

    table_exact = c_names == rust_names
    report = {
        "schema_version": 1,
        "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "table_order_exact": table_exact,
        "selector_count": len(rust_names),
        "exact_count": sum(item["exact"] for item in selector_reports),
        "all_exact": table_exact and not mismatches,
        "all_stderr_empty": all(not result["stderr"] for result in c_reports.values())
        and all(not result["stderr"] for result in rust_reports.values()),
        "distinct_behavior_count": len(grouped_behaviors(c_names, c_reports)),
        "behaviors": grouped_behaviors(c_names, c_reports),
        "selectors": selector_reports,
        "mismatches": mismatches,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("report does not match retained reference", file=sys.stderr)
            return 1
    if not report["all_exact"] or not report["all_stderr_empty"]:
        print("literal-selection CLI surface differs", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
