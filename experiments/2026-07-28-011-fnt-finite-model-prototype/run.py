#!/usr/bin/env python3
"""Resumable controller for the bounded FNT finite-model experiment."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


STATUS_RE = re.compile(r"(?im)^%\s*SZS\s+status\s+([A-Za-z][A-Za-z0-9_-]*)")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]


def write_jsonl(path: Path, records: Sequence[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
        newline="\n",
    )


def final_status(text: str) -> str | None:
    matches = STATUS_RE.findall(text)
    return matches[-1] if matches else None


def run_command(command: Sequence[str], timeout: float) -> tuple[int, float, str, str]:
    def decoded(value: str | bytes | None) -> str:
        if value is None:
            return ""
        if isinstance(value, bytes):
            return value.decode("utf-8", errors="replace")
        return value

    started = time.monotonic()
    try:
        completed = subprocess.run(
            command,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        return (
            completed.returncode,
            time.monotonic() - started,
            completed.stdout,
            completed.stderr,
        )
    except subprocess.TimeoutExpired as error:
        stdout = decoded(error.stdout)
        stderr = decoded(error.stderr)
        return 124, time.monotonic() - started, stdout, stderr


def selected_records(args: argparse.Namespace) -> list[dict[str, Any]]:
    records = read_jsonl(args.manifest)
    return [
        record
        for record in records
        if args.split == "all" or record["split"] == args.split
    ]


def inventory(args: argparse.Namespace) -> int:
    results: list[dict[str, Any]] = []
    raw = args.output / "inventory"
    raw.mkdir(parents=True, exist_ok=True)
    for index, record in enumerate(selected_records(args), 1):
        problem = args.corpus_root / record["path"]
        report = raw / f"{record['problem_id']}.json"
        solution = raw / f"{record['problem_id']}.s"
        if args.force or not report.exists():
            command = [
                sys.executable,
                str(args.prototype),
                str(problem),
                "--umlaut",
                str(args.umlaut),
                "--sat",
                str(args.sat),
                "--mode",
                "sorted",
                "--analyze-only",
                "--report",
                str(report),
            ]
            code, _, stdout, stderr = run_command(command, args.wall_seconds)
            solution.write_text(stdout + stderr, encoding="utf-8", newline="\n")
            if code == 124:
                report.write_text(
                    json.dumps(
                        {
                            "schema_version": 1,
                            "problem": str(problem),
                            "mode": "sorted",
                            "outcome": "timeout",
                            "reason": "inventory controller timeout",
                        },
                        indent=2,
                    )
                    + "\n",
                    encoding="utf-8",
                    newline="\n",
                )
            elif not report.exists():
                report.write_text(
                    json.dumps(
                        {
                            "schema_version": 1,
                            "problem": str(problem),
                            "mode": "sorted",
                            "outcome": "input_error",
                            "reason": (
                                "inventory worker exited without a report "
                                f"(code {code}): {stderr[-2000:]}"
                            ),
                        },
                        indent=2,
                    )
                    + "\n",
                    encoding="utf-8",
                    newline="\n",
                )
        result = json.loads(report.read_text(encoding="utf-8"))
        result.update(
            {
                "problem_id": record["problem_id"],
                "category": record["category"],
                "family": record["family"],
                "split": record["split"],
            }
        )
        results.append(result)
        if index % 25 == 0:
            print(f"inventory {index}/{len(selected_records(args))}", flush=True)
    write_jsonl(args.output / "inventory.jsonl", results)
    return 0


def prototype_runs(args: argparse.Namespace) -> int:
    inventory_by_id = {
        record["problem_id"]: record
        for record in read_jsonl(args.inventory_results)
    }
    modes = args.modes.split(",")
    results: list[dict[str, Any]] = []
    records = [
        record
        for record in selected_records(args)
        if inventory_by_id.get(record["problem_id"], {}).get("outcome") == "supported"
    ]
    for mode in modes:
        raw = args.output / "prototype" / mode
        raw.mkdir(parents=True, exist_ok=True)
        for index, record in enumerate(records, 1):
            problem = args.corpus_root / record["path"]
            report = raw / f"{record['problem_id']}.json"
            solution = raw / f"{record['problem_id']}.s"
            validation = raw / f"{record['problem_id']}.validation.json"
            if args.force or not report.exists():
                command = [
                    sys.executable,
                    str(args.prototype),
                    str(problem),
                    "--umlaut",
                    str(args.umlaut),
                    "--sat",
                    str(args.sat),
                    "--mode",
                    mode,
                    "--max-size",
                    str(args.max_size),
                    "--max-size-vectors",
                    str(args.max_size_vectors),
                    "--sat-timeout-seconds",
                    str(args.sat_seconds),
                    "--report",
                    str(report),
                ]
                code, wall, stdout, stderr = run_command(command, args.wall_seconds)
                solution.write_text(stdout, encoding="utf-8", newline="\n")
                if code == 124:
                    partial = (
                        json.loads(report.read_text(encoding="utf-8"))
                        if report.exists()
                        else {
                            "schema_version": 1,
                            "problem": str(problem),
                            "mode": mode,
                            "max_size": args.max_size,
                        }
                    )
                    partial.update(
                        {
                            "outcome": "timeout",
                            "reason": "controller wall timeout",
                            "controller_wall_seconds": wall,
                        }
                    )
                    report.write_text(
                        json.dumps(partial, indent=2, sort_keys=True) + "\n",
                        encoding="utf-8",
                        newline="\n",
                    )
                if stderr:
                    (raw / f"{record['problem_id']}.stderr").write_text(
                        stderr, encoding="utf-8", newline="\n"
                    )
            result = json.loads(report.read_text(encoding="utf-8"))
            result.update(
                {
                    "problem_id": record["problem_id"],
                    "category": record["category"],
                    "family": record["family"],
                    "split": record["split"],
                    "solution_status": final_status(
                        solution.read_text(encoding="utf-8")
                    ),
                    "validation_verdict": None,
                }
            )
            if result["outcome"] == "model" and (args.force or not validation.exists()):
                validation_command = [
                    sys.executable,
                    str(args.validate_model),
                    str(problem),
                    str(solution),
                    "--validator",
                    str(args.validator),
                    "--vampire",
                    str(args.vampire),
                    "--report",
                    str(validation),
                ]
                run_command(validation_command, args.validation_seconds)
            if validation.exists():
                validation_report = json.loads(validation.read_text(encoding="utf-8"))
                result["validation_verdict"] = validation_report.get("verdict")
            results.append(result)
            if index % 25 == 0:
                print(f"{mode} {index}/{len(records)}", flush=True)
    write_jsonl(args.output / f"prototype-{args.split}.jsonl", results)
    return 0


def baseline_runs(args: argparse.Namespace) -> int:
    results: list[dict[str, Any]] = []
    records = selected_records(args)
    if args.inventory_results is not None:
        supported = {
            record["problem_id"]
            for record in read_jsonl(args.inventory_results)
            if record.get("outcome") == "supported"
        }
        records = [
            record for record in records if record["problem_id"] in supported
        ]
    systems = {
        "umlaut-auto": lambda problem: [
            str(args.umlaut),
            "--auto",
            f"--soft-cpu-limit={int(args.baseline_seconds)}",
            f"--cpu-limit={int(args.wall_seconds)}",
            "--tstp-format",
            str(problem),
        ],
        "vampire-casc-sat": lambda problem: [
            str(args.vampire),
            "--mode",
            "casc",
            "--intent",
            "sat",
            "--proof",
            "tptp",
            "--time_limit",
            str(int(args.baseline_seconds)),
            str(problem),
        ],
    }
    requested_systems = set(args.systems.split(","))
    unknown_systems = requested_systems - systems.keys()
    if unknown_systems:
        raise SystemExit(
            "unknown baseline system(s): " + ", ".join(sorted(unknown_systems))
        )
    for system, command_builder in systems.items():
        raw = args.output / "baselines" / system
        raw.mkdir(parents=True, exist_ok=True)
        for index, record in enumerate(records, 1):
            problem = args.corpus_root / record["path"]
            output = raw / f"{record['problem_id']}.out"
            metadata = raw / f"{record['problem_id']}.json"
            if system in requested_systems and (args.force or not metadata.exists()):
                code, wall, stdout, stderr = run_command(
                    command_builder(problem), args.wall_seconds
                )
                output.write_text(stdout + stderr, encoding="utf-8", newline="\n")
                metadata.write_text(
                    json.dumps(
                        {
                            "returncode": code,
                            "wall_seconds": wall,
                            "status": final_status(stdout + stderr),
                        },
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n",
                    encoding="utf-8",
                    newline="\n",
                )
            if not metadata.exists():
                continue
            result = json.loads(metadata.read_text(encoding="utf-8"))
            result.update(
                {
                    "system": system,
                    "problem_id": record["problem_id"],
                    "category": record["category"],
                    "family": record["family"],
                    "split": record["split"],
                }
            )
            results.append(result)
            if index % 25 == 0:
                print(f"{system} {index}/{len(records)}", flush=True)
    write_jsonl(args.output / f"baselines-{args.split}.jsonl", results)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("inventory", "prototype", "baselines"))
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--corpus-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--prototype", type=Path)
    parser.add_argument("--validate-model", type=Path)
    parser.add_argument("--validator", type=Path)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--sat", type=Path, required=True)
    parser.add_argument("--vampire", type=Path)
    parser.add_argument("--inventory-results", type=Path)
    parser.add_argument("--split", choices=("all", "train", "validation", "test"), default="all")
    parser.add_argument("--modes", default="naive,sorted,sorted-symmetry")
    parser.add_argument("--max-size", type=int, default=3)
    parser.add_argument("--max-size-vectors", type=int, default=1024)
    parser.add_argument("--sat-seconds", type=float, default=5.0)
    parser.add_argument("--baseline-seconds", type=float, default=5.0)
    parser.add_argument("--systems", default="umlaut-auto,vampire-casc-sat")
    parser.add_argument("--wall-seconds", type=float, default=8.0)
    parser.add_argument("--validation-seconds", type=float, default=130.0)
    parser.add_argument("--force", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.phase == "inventory":
        return inventory(args)
    if args.phase == "prototype":
        required = (
            args.prototype,
            args.validate_model,
            args.validator,
            args.vampire,
            args.inventory_results,
        )
        if any(value is None for value in required):
            raise SystemExit("prototype phase requires prototype, validation, and inventory paths")
        return prototype_runs(args)
    if args.vampire is None:
        raise SystemExit("baselines phase requires --vampire")
    return baseline_runs(args)


if __name__ == "__main__":
    raise SystemExit(main())
