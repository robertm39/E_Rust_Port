#!/usr/bin/env python3
"""Resumable family-held-out controller for the typed FNT experiment."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Callable, Sequence


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


def run_command(
    command: Sequence[str],
    timeout: float,
    environment: dict[str, str],
) -> tuple[int, float, str, str]:
    started = time.monotonic()
    try:
        process = subprocess.Popen(
            command,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            start_new_session=os.name == "posix",
        )
    except OSError as error:
        return 127, time.monotonic() - started, "", str(error)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
        return process.returncode, time.monotonic() - started, stdout, stderr
    except subprocess.TimeoutExpired:
        if os.name == "posix":
            os.killpg(process.pid, signal.SIGKILL)
        else:
            process.kill()
        stdout, stderr = process.communicate()
        return 124, time.monotonic() - started, stdout, stderr


def selected_records(args: argparse.Namespace) -> list[dict[str, Any]]:
    records = read_jsonl(args.manifest)
    return [record for record in records if record["split"] == args.split]


def record_path(args: argparse.Namespace, record: dict[str, Any]) -> Path:
    return args.corpus_root / record["path"]


def environment(args: argparse.Namespace) -> dict[str, str]:
    result = dict(os.environ)
    result["TPTP"] = str(args.corpus_root.resolve())
    return result


def parallel_records(
    args: argparse.Namespace,
    records: list[dict[str, Any]],
    worker: Callable[[dict[str, Any]], dict[str, Any]],
    aggregate: Path,
) -> None:
    completed: dict[str, dict[str, Any]] = {}
    if aggregate.exists() and not args.force:
        completed = {
            record["problem_id"]: record for record in read_jsonl(aggregate)
        }
    pending = [record for record in records if record["problem_id"] not in completed]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as executor:
        futures = {executor.submit(worker, record): record for record in pending}
        for index, future in enumerate(concurrent.futures.as_completed(futures), 1):
            result = future.result()
            completed[result["problem_id"]] = result
            ordered = [completed[record["problem_id"]] for record in records if record["problem_id"] in completed]
            write_jsonl(aggregate, ordered)
            print(f"{args.phase} {index}/{len(pending)} {result['problem_id']}", flush=True)


def inventory(args: argparse.Namespace) -> None:
    records = selected_records(args)
    raw = args.output / "inventory" / args.split
    raw.mkdir(parents=True, exist_ok=True)
    env = environment(args)

    def worker(record: dict[str, Any]) -> dict[str, Any]:
        report = raw / f"{record['problem_id']}.json"
        command = [
            sys.executable,
            str(args.worker),
            str(record_path(args, record)),
            "--umlaut",
            str(args.umlaut),
            "--sat-probe",
            str(args.sat_probe),
            "--analyze-only",
            "--report",
            str(report),
        ]
        code, wall, stdout, stderr = run_command(command, args.wall_seconds, env)
        if report.exists():
            result = json.loads(report.read_text(encoding="utf-8"))
        else:
            result = {
                "schema_version": 2,
            }
        if code == 124:
            result.update(
                {
                    "outcome": "timeout",
                    "reason": "inventory controller timeout",
                }
            )
        elif code not in {0, 2} and "outcome" not in result:
            result.update(
                {
                    "outcome": "error",
                    "reason": f"worker exited {code} without final report: {stderr[-1000:]}",
                }
            )
        result.update(
            {
                "problem_id": record["problem_id"],
                "category": record["category"],
                "family": record["family"],
                "split": record["split"],
                "controller_wall_seconds": wall,
                "solution_status": final_status(stdout + stderr),
            }
        )
        return result

    parallel_records(args, records, worker, args.output / f"inventory-{args.split}.jsonl")


def prototype(args: argparse.Namespace) -> None:
    inventory_records = read_jsonl(args.inventory)
    supported = {
        record["problem_id"]
        for record in inventory_records
        if record.get("outcome") == "supported"
    }
    records = [
        record for record in selected_records(args) if record["problem_id"] in supported
    ]
    raw = args.output / "prototype" / args.split
    raw.mkdir(parents=True, exist_ok=True)
    env = environment(args)

    def worker(record: dict[str, Any]) -> dict[str, Any]:
        report = raw / f"{record['problem_id']}.json"
        solution = raw / f"{record['problem_id']}.s"
        validation = raw / f"{record['problem_id']}.validation.json"
        command = [
            sys.executable,
            str(args.worker),
            str(record_path(args, record)),
            "--umlaut",
            str(args.umlaut),
            "--sat-probe",
            str(args.sat_probe),
            "--max-size",
            str(args.max_size),
            "--max-size-vectors",
            str(args.max_size_vectors),
            "--max-ground-instances",
            str(args.max_ground_instances),
            "--sat-timeout-seconds",
            str(args.sat_seconds),
            "--report",
            str(report),
        ]
        code, wall, stdout, stderr = run_command(command, args.wall_seconds, env)
        solution.write_text(stdout, encoding="utf-8", newline="\n")
        if report.exists():
            result = json.loads(report.read_text(encoding="utf-8"))
        else:
            result = {
                "schema_version": 2,
            }
        if code == 124:
            result.update(
                {
                    "outcome": "timeout",
                    "reason": "prototype controller timeout",
                }
            )
        elif code not in {0, 2} and "outcome" not in result:
            result.update(
                {
                    "outcome": "error",
                    "reason": f"worker exited {code} without final report: {stderr[-1000:]}",
                }
            )
        verdict = None
        if result.get("outcome") == "model":
            validation_command = [
                sys.executable,
                str(args.validate_model),
                str(record_path(args, record)),
                str(solution),
                "--validator",
                str(args.validator),
                "--vampire",
                str(args.vampire),
                "--report",
                str(validation),
            ]
            run_command(validation_command, args.validation_seconds, env)
            if validation.exists():
                verdict = json.loads(
                    validation.read_text(encoding="utf-8")
                ).get("verdict")
        result.update(
            {
                "problem_id": record["problem_id"],
                "category": record["category"],
                "family": record["family"],
                "split": record["split"],
                "controller_wall_seconds": wall,
                "solution_status": final_status(stdout),
                "validation_verdict": verdict,
            }
        )
        return result

    parallel_records(args, records, worker, args.output / f"prototype-{args.split}.jsonl")


def baseline(args: argparse.Namespace) -> None:
    records = selected_records(args)
    raw = args.output / "baseline" / args.split
    raw.mkdir(parents=True, exist_ok=True)
    env = environment(args)

    def worker(record: dict[str, Any]) -> dict[str, Any]:
        output = raw / f"{record['problem_id']}.out"
        command = [
            str(args.umlaut),
            "--auto",
            f"--soft-cpu-limit={int(args.baseline_seconds)}",
            f"--cpu-limit={int(args.wall_seconds)}",
            "--tstp-format",
            str(record_path(args, record)),
        ]
        code, wall, stdout, stderr = run_command(command, args.wall_seconds + 2, env)
        output.write_text(stdout + stderr, encoding="utf-8", newline="\n")
        return {
            "schema_version": 2,
            "problem_id": record["problem_id"],
            "category": record["category"],
            "family": record["family"],
            "split": record["split"],
            "returncode": code,
            "wall_seconds": wall,
            "status": final_status(stdout + stderr),
        }

    parallel_records(args, records, worker, args.output / f"baseline-{args.split}.jsonl")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("inventory", "prototype", "baseline"))
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--corpus-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--split", choices=("train", "validation", "test"), required=True)
    parser.add_argument("--worker", type=Path)
    parser.add_argument("--umlaut", type=Path, required=True)
    parser.add_argument("--sat-probe", type=Path)
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--validate-model", type=Path)
    parser.add_argument("--validator", type=Path)
    parser.add_argument("--vampire", type=Path)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--wall-seconds", type=float, default=15.0)
    parser.add_argument("--baseline-seconds", type=float, default=10.0)
    parser.add_argument("--sat-seconds", type=float, default=5.0)
    parser.add_argument("--validation-seconds", type=float, default=120.0)
    parser.add_argument("--max-size", type=int, default=3)
    parser.add_argument("--max-size-vectors", type=int, default=2048)
    parser.add_argument("--max-ground-instances", type=int, default=5_000_000)
    parser.add_argument("--force", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.phase in {"inventory", "prototype"} and (
        args.worker is None or args.sat_probe is None
    ):
        raise SystemExit("inventory/prototype requires --worker and --sat-probe")
    if args.phase == "prototype" and any(
        value is None
        for value in (
            args.inventory,
            args.validate_model,
            args.validator,
            args.vampire,
        )
    ):
        raise SystemExit("prototype requires inventory and validation paths")
    if args.phase == "inventory":
        inventory(args)
    elif args.phase == "prototype":
        prototype(args)
    else:
        baseline(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
