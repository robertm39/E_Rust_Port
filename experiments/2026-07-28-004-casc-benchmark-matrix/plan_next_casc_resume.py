#!/usr/bin/env python3
"""Validate a CASC checkpoint and plan its next guarded resume."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Sequence

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[1]
VALIDATOR = SCRIPT_DIR / "validate_casc_checkpoint.py"
RESUME_CONTROLLER = SCRIPT_DIR / "resume_j13_checkpoint.ps1"
RUNNER = REPO_ROOT / "linode-runner.ps1"
RELEASES = {
    "j13": {
        "manifest": REPO_ROOT / "benchmarks" / "casc_2026_manifest.jsonl",
        "run_name": "casc-j13-2026-089e06c8-v2",
        "contract_id": (
            "9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676"
        ),
        "expected_results": 2700,
    },
    "casc2025": {
        "manifest": REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl",
        "run_name": "casc30-2025-089e06c8-v2",
        "contract_id": (
            "e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa"
        ),
        "expected_results": 5802,
    },
}


class PlanningError(RuntimeError):
    """Raised when trusted evidence cannot support a safe resume plan."""


def parse_utc(value: object, field: str) -> datetime:
    if not isinstance(value, str):
        raise PlanningError(f"allowance has no {field}")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise PlanningError(f"allowance has invalid {field}") from error
    if parsed.tzinfo is None:
        raise PlanningError(f"allowance {field} lacks a timezone")
    return parsed.astimezone(timezone.utc)


def build_resume_plan(
    *,
    release: str,
    checkpoint: Path,
    checkpoint_sha256: str,
    validation: dict[str, Any],
    allowance: dict[str, Any],
    max_session_wall_seconds: int,
    boundary_guard_seconds: int,
) -> dict[str, Any]:
    config = RELEASES[release]
    service_runtime_seconds = max_session_wall_seconds + 300
    checks = {
        "validation schema": (validation.get("schema_version"), 1),
        "validation kind": (
            validation.get("kind"),
            "umlaut-casc-checkpoint-validation",
        ),
        "archive SHA-256": (
            validation.get("archive", {}).get("sha256"),
            checkpoint_sha256,
        ),
        "run name": (validation.get("run", {}).get("run_name"), config["run_name"]),
        "contract ID": (
            validation.get("run", {}).get("contract_id"),
            config["contract_id"],
        ),
        "release result boundary": (
            validation.get("run", {}).get("expected_results"),
            config["expected_results"],
        ),
        "allowance kind": (
            allowance.get("kind"),
            "umlaut-linode-high-memory-allowance",
        ),
        "allowance schema": (allowance.get("schema_version"), 1),
        "required seconds": (
            allowance.get("required_seconds"),
            service_runtime_seconds,
        ),
        "active high-memory hosts": (
            allowance.get("active_managed_high_memory"),
            0,
        ),
    }
    for name, (actual, expected) in checks.items():
        if actual != expected:
            raise PlanningError(f"{name} mismatch: {actual!r} != {expected!r}")
    completed = validation["run"].get("completed_results")
    if isinstance(completed, bool) or not isinstance(completed, int):
        raise PlanningError("validation has an invalid completed result count")
    if not 0 <= completed <= config["expected_results"]:
        raise PlanningError("validated result count is outside the release boundary")

    observed_at = parse_utc(allowance.get("observed_at_utc"), "observed_at_utc")
    required_now = allowance.get("required_start_available_now")
    if not isinstance(required_now, bool):
        raise PlanningError("allowance has no Boolean required-start decision")
    remaining = allowance.get("remaining_seconds")
    if isinstance(remaining, bool) or not isinstance(remaining, int):
        raise PlanningError("allowance has an invalid remaining duration")
    if required_now != (remaining >= service_runtime_seconds):
        raise PlanningError("allowance required-start decision is inconsistent")
    not_before: datetime | None = None
    if not required_now:
        projected = parse_utc(
            allowance.get("projected_earliest_required_start_utc"),
            "projected_earliest_required_start_utc",
        )
        if projected < observed_at:
            raise PlanningError("allowance projects a required start in the past")
        not_before = projected + timedelta(seconds=boundary_guard_seconds)

    result: dict[str, Any] = {
        "schema_version": 1,
        "kind": "umlaut-casc-next-resume-plan",
        "release": release,
        "checkpoint": {
            "path": str(checkpoint.resolve()),
            "sha256": checkpoint_sha256,
            "completed_results": completed,
            "expected_results": config["expected_results"],
        },
        "allowance": {
            "observed_at_utc": allowance["observed_at_utc"],
            "required_seconds": service_runtime_seconds,
            "required_start_available_now": required_now,
            "projected_earliest_required_start_utc": allowance.get(
                "projected_earliest_required_start_utc"
            ),
            "remaining_seconds": allowance.get("remaining_seconds"),
        },
    }
    if completed == config["expected_results"]:
        result["status"] = "release_complete"
        result["controller"] = None
        return result
    if not_before is not None and not_before > observed_at + timedelta(hours=24):
        result["status"] = "replan_within_24_hours"
        result["controller"] = None
        return result

    arguments = [
        "-Release",
        release,
        "-CheckpointArchive",
        str(checkpoint.resolve()),
        "-CheckpointSha256",
        checkpoint_sha256,
        "-ExpectedInitialResults",
        str(completed),
        "-MaxSessionWallSeconds",
        str(max_session_wall_seconds),
    ]
    if not_before is not None:
        arguments.extend(
            ["-NotBeforeUtc", not_before.isoformat(timespec="seconds")]
        )
    arguments.append("-Execute")
    result["status"] = "ready_to_arm"
    result["controller"] = {
        "script": str(RESUME_CONTROLLER),
        "arguments": arguments,
    }
    return result


def run_json(command: Sequence[str], description: str) -> dict[str, Any]:
    completed = subprocess.run(
        list(command),
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise PlanningError(f"{description} failed: {detail}")
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise PlanningError(f"{description} did not emit JSON") from error
    if not isinstance(value, dict):
        raise PlanningError(f"{description} did not emit a JSON object")
    return value


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", choices=tuple(RELEASES), required=True)
    parser.add_argument("--checkpoint", type=Path, required=True)
    parser.add_argument("--checkpoint-sha256", required=True)
    parser.add_argument("--max-session-wall-seconds", type=int, default=14400)
    parser.add_argument("--boundary-guard-seconds", type=int, default=10)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        checkpoint = arguments.checkpoint.resolve()
        checkpoint_sha256 = arguments.checkpoint_sha256.lower()
        if arguments.max_session_wall_seconds <= 0:
            raise PlanningError("--max-session-wall-seconds must be positive")
        if not 0 <= arguments.boundary_guard_seconds <= 60:
            raise PlanningError("--boundary-guard-seconds must be in 0..60")
        if len(checkpoint_sha256) != 64 or any(
            value not in "0123456789abcdef" for value in checkpoint_sha256
        ):
            raise PlanningError("--checkpoint-sha256 must be lowercase hex")
        config = RELEASES[arguments.release]
        validation = run_json(
            [
                sys.executable,
                str(VALIDATOR),
                "--archive",
                str(checkpoint),
                "--archive-sha256",
                checkpoint_sha256,
                "--manifest",
                str(config["manifest"]),
                "--run-name",
                str(config["run_name"]),
                "--contract-id",
                str(config["contract_id"]),
            ],
            "checkpoint validation",
        )
        powershell = shutil.which("powershell.exe") or shutil.which("powershell")
        if powershell is None:
            raise PlanningError("PowerShell is required to query runner allowance")
        service_runtime_seconds = arguments.max_session_wall_seconds + 300
        allowance = run_json(
            [
                powershell,
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(RUNNER),
                "allowance",
                "--required-seconds",
                str(service_runtime_seconds),
            ],
            "high-memory allowance query",
        )
        plan = build_resume_plan(
            release=arguments.release,
            checkpoint=checkpoint,
            checkpoint_sha256=checkpoint_sha256,
            validation=validation,
            allowance=allowance,
            max_session_wall_seconds=arguments.max_session_wall_seconds,
            boundary_guard_seconds=arguments.boundary_guard_seconds,
        )
        output = json.dumps(plan, indent=2, sort_keys=True) + "\n"
        if arguments.output is not None:
            destination = arguments.output.resolve()
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(output, encoding="utf-8", newline="\n")
        print(output, end="")
        return 0
    except (OSError, PlanningError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
