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
        "combined_release": "CASC-J13",
        "manifest": REPO_ROOT / "benchmarks" / "casc_2026_manifest.jsonl",
        "run_name": "casc-j13-2026-089e06c8-v2",
        "contract_id": (
            "9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676"
        ),
        "targeted_problems": 1350,
        "expected_results": 2700,
        "official_csv_count": 26,
    },
    "casc2025": {
        "combined_release": "CASC-2025",
        "manifest": REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl",
        "run_name": "casc30-2025-089e06c8-v2",
        "contract_id": (
            "e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa"
        ),
        "targeted_problems": 2901,
        "expected_results": 5802,
        "official_csv_count": 40,
    },
}
COMBINED_REPORT_ORDER = ("casc2025", "j13")


class PlanningError(RuntimeError):
    """Raised when trusted evidence cannot support a safe resume plan."""


def validated_result_count(
    *,
    release: str,
    checkpoint_sha256: str,
    validation: dict[str, Any],
) -> int:
    """Return a result count only after validating release identity evidence."""
    config = RELEASES[release]
    archive = validation.get("archive")
    run = validation.get("run")
    if not isinstance(archive, dict):
        raise PlanningError("validation has no archive object")
    if not isinstance(run, dict):
        raise PlanningError("validation has no run object")
    checks = {
        "validation schema": (validation.get("schema_version"), 1),
        "validation kind": (
            validation.get("kind"),
            "umlaut-casc-checkpoint-validation",
        ),
        "archive SHA-256": (archive.get("sha256"), checkpoint_sha256),
        "run name": (run.get("run_name"), config["run_name"]),
        "contract ID": (run.get("contract_id"), config["contract_id"]),
        "release result boundary": (
            run.get("expected_results"),
            config["expected_results"],
        ),
    }
    for name, (actual, expected) in checks.items():
        if actual != expected:
            raise PlanningError(f"{name} mismatch: {actual!r} != {expected!r}")
    completed = run.get("completed_results")
    if isinstance(completed, bool) or not isinstance(completed, int):
        raise PlanningError("validation has an invalid completed result count")
    if not 0 <= completed <= config["expected_results"]:
        raise PlanningError("validated result count is outside the release boundary")
    return completed


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
    completed = validated_result_count(
        release=release,
        checkpoint_sha256=checkpoint_sha256,
        validation=validation,
    )
    checks = {
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


def build_campaign_complete_plan(
    *,
    checkpoint: Path,
    checkpoint_sha256: str,
    completed_results: dict[str, int],
    combined_validation: dict[str, Any],
) -> dict[str, Any]:
    expected_results = {
        release: int(config["expected_results"])
        for release, config in RELEASES.items()
    }
    if completed_results != expected_results:
        raise PlanningError("campaign completion requires every release boundary")
    final_release = tuple(RELEASES)[-1]
    validated_result_count(
        release=final_release,
        checkpoint_sha256=checkpoint_sha256,
        validation=combined_validation,
    )
    combined = combined_validation.get("combined")
    if not isinstance(combined, dict):
        raise PlanningError("campaign completion has no combined evidence")
    expected_targeted_problems = sum(
        int(config["targeted_problems"]) for config in RELEASES.values()
    )
    expected_combined_results = sum(
        int(config["expected_results"]) for config in RELEASES.values()
    )
    expected_official_csv_count = sum(
        int(config["official_csv_count"]) for config in RELEASES.values()
    )
    combined_checks = {
        "combined report embedding": (combined.get("embedded"), True),
        "combined releases": (
            combined.get("releases"),
            sorted(
                str(config["combined_release"])
                for config in RELEASES.values()
            ),
        ),
        "combined targeted problems": (
            combined.get("targeted_problems"),
            expected_targeted_problems,
        ),
        "combined expected results": (
            combined.get("expected_results"),
            expected_combined_results,
        ),
        "combined completed results": (
            combined.get("completed_results"),
            expected_combined_results,
        ),
        "combined missing results": (combined.get("missing_results"), 0),
        "combined official CSV count": (
            combined.get("official_csv_count"),
            expected_official_csv_count,
        ),
    }
    for name, (actual, expected) in combined_checks.items():
        if actual != expected:
            raise PlanningError(f"{name} mismatch: {actual!r} != {expected!r}")
    summary_sha256 = combined.get("summary_sha256")
    if not isinstance(summary_sha256, str) or len(summary_sha256) != 64:
        raise PlanningError("combined report has an invalid SHA-256")
    if any(value not in "0123456789abcdef" for value in summary_sha256):
        raise PlanningError("combined report has an invalid SHA-256")
    return {
        "schema_version": 1,
        "kind": "umlaut-casc-next-resume-plan",
        "release": None,
        "status": "campaign_complete",
        "checkpoint": {
            "path": str(checkpoint.resolve()),
            "sha256": checkpoint_sha256,
            "completed_results": completed_results,
            "expected_results": expected_results,
        },
        "allowance": None,
        "combined_report": {
            "sha256": summary_sha256,
            "targeted_problems": combined["targeted_problems"],
            "completed_results": combined["completed_results"],
            "official_csv_count": combined["official_csv_count"],
        },
        "controller": None,
    }


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


def emit_plan(plan: dict[str, Any], destination: Path | None) -> None:
    output = json.dumps(plan, indent=2, sort_keys=True) + "\n"
    if destination is not None:
        resolved = destination.resolve()
        resolved.parent.mkdir(parents=True, exist_ok=True)
        resolved.write_text(output, encoding="utf-8", newline="\n")
    print(output, end="")


def validation_command(
    *,
    checkpoint: Path,
    checkpoint_sha256: str,
    release: str,
    combined: bool = False,
) -> list[str]:
    config = RELEASES[release]
    command = [
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
    ]
    if combined:
        for combined_key in COMBINED_REPORT_ORDER:
            combined_config = RELEASES[combined_key]
            command.extend(
                [
                    "--combined-run",
                    str(combined_config["combined_release"]),
                    str(combined_config["manifest"]),
                    str(combined_config["run_name"]),
                    str(combined_config["contract_id"]),
                ]
            )
    return command


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--release",
        choices=("auto", *RELEASES),
        default="auto",
        help="release to resume, or the first incomplete release (default: auto)",
    )
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
        releases = (
            tuple(RELEASES)
            if arguments.release == "auto"
            else (arguments.release,)
        )
        completed_results: dict[str, int] = {}
        selected_release: str | None = None
        selected_validation: dict[str, Any] | None = None
        for release in releases:
            config = RELEASES[release]
            validation = run_json(
                validation_command(
                    checkpoint=checkpoint,
                    checkpoint_sha256=checkpoint_sha256,
                    release=release,
                ),
                f"{release} checkpoint validation",
            )
            completed_results[release] = validated_result_count(
                release=release,
                checkpoint_sha256=checkpoint_sha256,
                validation=validation,
            )
            if completed_results[release] < config["expected_results"]:
                selected_release = release
                selected_validation = validation
                break
        if selected_release is None:
            if arguments.release != "auto":
                selected_release = arguments.release
                selected_validation = validation
            else:
                combined_validation = run_json(
                    validation_command(
                        checkpoint=checkpoint,
                        checkpoint_sha256=checkpoint_sha256,
                        release=tuple(RELEASES)[-1],
                        combined=True,
                    ),
                    "complete campaign validation",
                )
                plan = build_campaign_complete_plan(
                    checkpoint=checkpoint,
                    checkpoint_sha256=checkpoint_sha256,
                    completed_results=completed_results,
                    combined_validation=combined_validation,
                )
                emit_plan(plan, arguments.output)
                return 0
        if selected_validation is None:  # pragma: no cover - defensive invariant
            raise PlanningError("release selection produced no validation evidence")
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
            release=selected_release,
            checkpoint=checkpoint,
            checkpoint_sha256=checkpoint_sha256,
            validation=selected_validation,
            allowance=allowance,
            max_session_wall_seconds=arguments.max_session_wall_seconds,
            boundary_guard_seconds=arguments.boundary_guard_seconds,
        )
        emit_plan(plan, arguments.output)
        return 0
    except (OSError, PlanningError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
