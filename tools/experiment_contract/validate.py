#!/usr/bin/env python3
"""Validate Umlaut experiment-result contracts."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path, PurePosixPath
from typing import Any, Sequence


SCHEMA_VERSION = 1
DECISIONS = {"continue", "stop", "uncertain"}
CORRECTNESS_STATUSES = {"pass", "fail", "incomplete"}
PERFORMANCE_STATUSES = {"valid", "invalid", "not_measured"}
CHECK_STATUSES = {"pass", "fail", "incomplete"}
METRIC_DIRECTIONS = {
    "lower_is_better",
    "higher_is_better",
    "descriptive",
}
SHA256_LENGTH = 64
GIT_SHA_LENGTH = 40


class DuplicateKeyError(ValueError):
    """Raised when a JSON object contains a duplicate key."""


def _object_without_duplicate_keys(
    pairs: list[tuple[str, Any]],
) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON object while rejecting duplicate keys."""

    value = json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=_object_without_duplicate_keys,
    )
    if not isinstance(value, dict):
        raise ValueError("the contract root must be an object")
    return value


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _is_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _is_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
    )


def _is_hex(value: Any, length: int) -> bool:
    return (
        isinstance(value, str)
        and len(value) == length
        and all(character in "0123456789abcdef" for character in value)
    )


def _is_nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _require_keys(
    value: Any,
    keys: set[str],
    location: str,
    errors: list[str],
) -> dict[str, Any] | None:
    if not isinstance(value, dict):
        errors.append(f"{location} must be an object")
        return None
    missing = sorted(keys - set(value))
    if missing:
        errors.append(
            f"{location} is missing required keys: {', '.join(missing)}"
        )
    return value


def _require_nonnegative_integer(
    value: Any,
    location: str,
    errors: list[str],
) -> None:
    if not _is_integer(value) or value < 0:
        errors.append(f"{location} must be a nonnegative integer")


def _validate_experiment(
    value: Any,
    errors: list[str],
) -> None:
    experiment = _require_keys(
        value,
        {"id", "title", "hypothesis", "source_revision"},
        "experiment",
        errors,
    )
    if experiment is None:
        return
    for key in ("id", "title", "hypothesis"):
        if not _is_nonempty_string(experiment.get(key)):
            errors.append(f"experiment.{key} must be a nonempty string")
    if not _is_hex(experiment.get("source_revision"), GIT_SHA_LENGTH):
        errors.append(
            "experiment.source_revision must be a lowercase full Git SHA"
        )


def _validate_treatments(
    value: Any,
    errors: list[str],
) -> None:
    treatments = _require_keys(
        value,
        {"baseline", "candidate"},
        "treatments",
        errors,
    )
    if treatments is None:
        return
    for treatment_name in ("baseline", "candidate"):
        treatment = _require_keys(
            treatments.get(treatment_name),
            {"name", "description"},
            f"treatments.{treatment_name}",
            errors,
        )
        if treatment is None:
            continue
        for key in ("name", "description"):
            if not _is_nonempty_string(treatment.get(key)):
                errors.append(
                    f"treatments.{treatment_name}.{key} "
                    "must be a nonempty string"
                )
    if (
        isinstance(treatments.get("baseline"), dict)
        and isinstance(treatments.get("candidate"), dict)
        and treatments["baseline"].get("name")
        == treatments["candidate"].get("name")
    ):
        errors.append("baseline and candidate names must differ")


def _validate_command(
    command: Any,
    index: int,
    errors: list[str],
) -> None:
    location = f"reproduction.commands[{index}]"
    command = _require_keys(
        command,
        {"purpose", "argv"},
        location,
        errors,
    )
    if command is None:
        return
    if not _is_nonempty_string(command.get("purpose")):
        errors.append(f"{location}.purpose must be a nonempty string")
    argv = command.get("argv")
    if (
        not isinstance(argv, list)
        or not argv
        or any(not _is_nonempty_string(argument) for argument in argv)
    ):
        errors.append(
            f"{location}.argv must be a nonempty string array"
        )


def _artifact_path(
    raw_path: Any,
    location: str,
    errors: list[str],
) -> PurePosixPath | None:
    if not _is_nonempty_string(raw_path):
        errors.append(f"{location}.path must be a nonempty string")
        return None
    path = PurePosixPath(raw_path)
    if path.is_absolute() or ".." in path.parts:
        errors.append(
            f"{location}.path must be repository-relative and contained"
        )
        return None
    if path.as_posix() != raw_path or raw_path.startswith("./"):
        errors.append(f"{location}.path must be normalized POSIX syntax")
        return None
    return path


def _validate_artifact(
    artifact: Any,
    index: int,
    errors: list[str],
    *,
    repo_root: Path | None,
    verify_artifacts: bool,
) -> None:
    location = f"reproduction.artifacts[{index}]"
    artifact = _require_keys(
        artifact,
        {"path", "role", "sha256", "bytes"},
        location,
        errors,
    )
    if artifact is None:
        return
    path = _artifact_path(artifact.get("path"), location, errors)
    if not _is_nonempty_string(artifact.get("role")):
        errors.append(f"{location}.role must be a nonempty string")
    if not _is_hex(artifact.get("sha256"), SHA256_LENGTH):
        errors.append(f"{location}.sha256 must be lowercase SHA-256")
    byte_count = artifact.get("bytes")
    if not _is_integer(byte_count) or byte_count < 0:
        errors.append(f"{location}.bytes must be a nonnegative integer")
    if not verify_artifacts or repo_root is None or path is None:
        return
    resolved = repo_root.joinpath(*path.parts).resolve()
    try:
        resolved.relative_to(repo_root.resolve())
    except ValueError:
        errors.append(f"{location}.path escapes the repository")
        return
    if not resolved.is_file():
        errors.append(f"{location}.path does not exist: {path}")
        return
    if _is_integer(byte_count) and resolved.stat().st_size != byte_count:
        errors.append(
            f"{location}.bytes mismatch: expected {byte_count}, "
            f"observed {resolved.stat().st_size}"
        )
    expected_sha = artifact.get("sha256")
    if _is_hex(expected_sha, SHA256_LENGTH):
        observed_sha = sha256_file(resolved)
        if observed_sha != expected_sha:
            errors.append(
                f"{location}.sha256 mismatch: expected {expected_sha}, "
                f"observed {observed_sha}"
            )


def _validate_reproduction(
    value: Any,
    errors: list[str],
    *,
    repo_root: Path | None,
    verify_artifacts: bool,
) -> None:
    reproduction = _require_keys(
        value,
        {
            "platform",
            "run_count",
            "seed_policy",
            "resource_limits",
            "commands",
            "artifacts",
        },
        "reproduction",
        errors,
    )
    if reproduction is None:
        return
    for key in ("platform", "seed_policy"):
        if not _is_nonempty_string(reproduction.get(key)):
            errors.append(
                f"reproduction.{key} must be a nonempty string"
            )
    run_count = reproduction.get("run_count")
    if not _is_integer(run_count) or run_count <= 0:
        errors.append("reproduction.run_count must be a positive integer")
    if not isinstance(reproduction.get("resource_limits"), dict):
        errors.append("reproduction.resource_limits must be an object")
    commands = reproduction.get("commands")
    if not isinstance(commands, list) or not commands:
        errors.append("reproduction.commands must be a nonempty array")
    else:
        for index, command in enumerate(commands):
            _validate_command(command, index, errors)
    artifacts = reproduction.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        errors.append("reproduction.artifacts must be a nonempty array")
    else:
        paths: set[str] = set()
        for index, artifact in enumerate(artifacts):
            _validate_artifact(
                artifact,
                index,
                errors,
                repo_root=repo_root,
                verify_artifacts=verify_artifacts,
            )
            if isinstance(artifact, dict):
                artifact_path = artifact.get("path")
                if isinstance(artifact_path, str):
                    if artifact_path in paths:
                        errors.append(
                            "reproduction.artifacts contains duplicate "
                            f"path: {artifact_path}"
                        )
                    paths.add(artifact_path)


def _validate_correctness(
    value: Any,
    errors: list[str],
) -> str | None:
    correctness = _require_keys(
        value,
        {
            "status",
            "status_pairing",
            "independent_validation",
            "checks",
        },
        "correctness",
        errors,
    )
    if correctness is None:
        return None
    status = correctness.get("status")
    if status not in CORRECTNESS_STATUSES:
        errors.append(
            "correctness.status must be pass, fail, or incomplete"
        )
    pairing = _require_keys(
        correctness.get("status_pairing"),
        {
            "paired_coordinates",
            "exact_matches",
            "polarity_disagreements",
        },
        "correctness.status_pairing",
        errors,
    )
    if pairing is not None:
        for key in (
            "paired_coordinates",
            "exact_matches",
            "polarity_disagreements",
        ):
            _require_nonnegative_integer(
                pairing.get(key),
                f"correctness.status_pairing.{key}",
                errors,
            )
        paired = pairing.get("paired_coordinates")
        exact = pairing.get("exact_matches")
        if _is_integer(paired) and _is_integer(exact) and exact > paired:
            errors.append(
                "correctness.status_pairing.exact_matches cannot exceed "
                "paired_coordinates"
            )
    validation = _require_keys(
        correctness.get("independent_validation"),
        {
            "status",
            "verified_claims",
            "coverage_gaps",
            "rejected_claims",
            "candidate_witnesses",
        },
        "correctness.independent_validation",
        errors,
    )
    if validation is not None:
        if validation.get("status") not in CHECK_STATUSES:
            errors.append(
                "correctness.independent_validation.status must be pass, "
                "fail, or incomplete"
            )
        for key in (
            "verified_claims",
            "coverage_gaps",
            "rejected_claims",
        ):
            _require_nonnegative_integer(
                validation.get(key),
                f"correctness.independent_validation.{key}",
                errors,
            )
        witnesses = validation.get("candidate_witnesses")
        if not isinstance(witnesses, list) or any(
            not _is_nonempty_string(witness) for witness in witnesses
        ):
            errors.append(
                "correctness.independent_validation.candidate_witnesses "
                "must be a string array"
            )
    checks = correctness.get("checks")
    if not isinstance(checks, list) or not checks:
        errors.append("correctness.checks must be a nonempty array")
    else:
        for index, check in enumerate(checks):
            location = f"correctness.checks[{index}]"
            check = _require_keys(
                check,
                {"name", "status", "evidence"},
                location,
                errors,
            )
            if check is None:
                continue
            if not _is_nonempty_string(check.get("name")):
                errors.append(f"{location}.name must be nonempty")
            if check.get("status") not in CHECK_STATUSES:
                errors.append(
                    f"{location}.status must be pass, fail, or incomplete"
                )
            if not _is_nonempty_string(check.get("evidence")):
                errors.append(f"{location}.evidence must be nonempty")
    return status if isinstance(status, str) else None


def _validate_metric(
    value: Any,
    location: str,
    errors: list[str],
) -> None:
    metric = _require_keys(
        value,
        {"name", "unit", "direction"},
        location,
        errors,
    )
    if metric is None:
        return
    for key in ("name", "unit"):
        if not _is_nonempty_string(metric.get(key)):
            errors.append(f"{location}.{key} must be nonempty")
    if metric.get("direction") not in METRIC_DIRECTIONS:
        errors.append(
            f"{location}.direction must be one of "
            + ", ".join(sorted(METRIC_DIRECTIONS))
        )


def _validate_noise(
    value: Any,
    location: str,
    errors: list[str],
) -> None:
    noise = _require_keys(
        value,
        {
            "method",
            "coordinate_count",
            "baseline_median_relative_range",
            "baseline_max_relative_range",
            "candidate_median_relative_range",
            "candidate_max_relative_range",
            "paired_ratio_median_relative_range",
            "paired_ratio_max_relative_range",
        },
        location,
        errors,
    )
    if noise is None:
        return
    if not _is_nonempty_string(noise.get("method")):
        errors.append(f"{location}.method must be nonempty")
    count = noise.get("coordinate_count")
    if not _is_integer(count) or count <= 0:
        errors.append(f"{location}.coordinate_count must be positive")
    for key in (
        "baseline_median_relative_range",
        "baseline_max_relative_range",
        "candidate_median_relative_range",
        "candidate_max_relative_range",
        "paired_ratio_median_relative_range",
        "paired_ratio_max_relative_range",
    ):
        metric = noise.get(key)
        if not _is_number(metric) or metric < 0:
            errors.append(f"{location}.{key} must be nonnegative")


def _validate_performance(
    value: Any,
    errors: list[str],
) -> str | None:
    performance = _require_keys(
        value,
        {
            "status",
            "primary_metric",
            "pairing",
            "observations",
            "secondary_metrics",
        },
        "performance",
        errors,
    )
    if performance is None:
        return None
    status = performance.get("status")
    if status not in PERFORMANCE_STATUSES:
        errors.append(
            "performance.status must be valid, invalid, or not_measured"
        )
    _validate_metric(
        performance.get("primary_metric"),
        "performance.primary_metric",
        errors,
    )
    pairing = _require_keys(
        performance.get("pairing"),
        {"unit", "repetitions"},
        "performance.pairing",
        errors,
    )
    if pairing is not None:
        if not _is_nonempty_string(pairing.get("unit")):
            errors.append("performance.pairing.unit must be nonempty")
        repetitions = pairing.get("repetitions")
        if not _is_integer(repetitions) or repetitions <= 0:
            errors.append(
                "performance.pairing.repetitions must be positive"
            )
    observations = performance.get("observations")
    if not isinstance(observations, list):
        errors.append("performance.observations must be an array")
    else:
        if status == "valid" and not observations:
            errors.append(
                "valid performance requires at least one observation"
            )
        for index, observation in enumerate(observations):
            location = f"performance.observations[{index}]"
            observation = _require_keys(
                observation,
                {
                    "scope",
                    "paired_coordinates",
                    "candidate_over_baseline_median",
                    "noise",
                },
                location,
                errors,
            )
            if observation is None:
                continue
            if not _is_nonempty_string(observation.get("scope")):
                errors.append(f"{location}.scope must be nonempty")
            count = observation.get("paired_coordinates")
            if not _is_integer(count) or count <= 0:
                errors.append(
                    f"{location}.paired_coordinates must be positive"
                )
            ratio = observation.get("candidate_over_baseline_median")
            if not _is_number(ratio) or ratio < 0:
                errors.append(
                    f"{location}.candidate_over_baseline_median "
                    "must be nonnegative"
                )
            _validate_noise(
                observation.get("noise"),
                f"{location}.noise",
                errors,
            )
    secondary = performance.get("secondary_metrics")
    if not isinstance(secondary, list):
        errors.append("performance.secondary_metrics must be an array")
    else:
        for index, metric in enumerate(secondary):
            location = f"performance.secondary_metrics[{index}]"
            metric = _require_keys(
                metric,
                {"name", "scope", "value"},
                location,
                errors,
            )
            if metric is None:
                continue
            for key in ("name", "scope"):
                if not _is_nonempty_string(metric.get(key)):
                    errors.append(f"{location}.{key} must be nonempty")
            if not _is_number(metric.get("value")):
                errors.append(f"{location}.value must be numeric")
    return status if isinstance(status, str) else None


def _validate_coverage(
    value: Any,
    errors: list[str],
) -> None:
    coverage = _require_keys(
        value,
        {
            "baseline_reproducible_solves",
            "candidate_reproducible_solves",
            "common_reproducible_solves",
            "candidate_only",
            "baseline_only",
        },
        "coverage",
        errors,
    )
    if coverage is None:
        return
    for key in (
        "baseline_reproducible_solves",
        "candidate_reproducible_solves",
        "common_reproducible_solves",
    ):
        _require_nonnegative_integer(
            coverage.get(key),
            f"coverage.{key}",
            errors,
        )
    for key in ("candidate_only", "baseline_only"):
        entries = coverage.get(key)
        if (
            not isinstance(entries, list)
            or any(not _is_nonempty_string(entry) for entry in entries)
            or len(entries) != len(set(entries))
        ):
            errors.append(
                f"coverage.{key} must be an array of unique strings"
            )
    baseline = coverage.get("baseline_reproducible_solves")
    candidate = coverage.get("candidate_reproducible_solves")
    common = coverage.get("common_reproducible_solves")
    candidate_only = coverage.get("candidate_only")
    baseline_only = coverage.get("baseline_only")
    if (
        _is_integer(candidate)
        and _is_integer(common)
        and isinstance(candidate_only, list)
        and candidate != common + len(candidate_only)
    ):
        errors.append(
            "candidate solve count must equal common plus candidate-only"
        )
    if (
        _is_integer(baseline)
        and _is_integer(common)
        and isinstance(baseline_only, list)
        and baseline != common + len(baseline_only)
    ):
        errors.append(
            "baseline solve count must equal common plus baseline-only"
        )


def _validate_decision(
    value: Any,
    errors: list[str],
    *,
    correctness_status: str | None,
) -> None:
    decision = _require_keys(
        value,
        {"outcome", "rule", "reasons", "production_effect"},
        "decision",
        errors,
    )
    if decision is None:
        return
    outcome = decision.get("outcome")
    if outcome not in DECISIONS:
        errors.append(
            "decision.outcome must be continue, stop, or uncertain"
        )
    if not _is_nonempty_string(decision.get("rule")):
        errors.append("decision.rule must be a nonempty string")
    reasons = decision.get("reasons")
    if (
        not isinstance(reasons, list)
        or not reasons
        or any(not _is_nonempty_string(reason) for reason in reasons)
    ):
        errors.append("decision.reasons must be a nonempty string array")
    if not _is_nonempty_string(decision.get("production_effect")):
        errors.append(
            "decision.production_effect must be a nonempty string"
        )
    if outcome == "continue" and correctness_status != "pass":
        errors.append(
            "decision.outcome cannot be continue unless correctness passes"
        )


def validate_record(
    record: dict[str, Any],
    *,
    repo_root: Path | None = None,
    verify_artifacts: bool = False,
) -> list[str]:
    """Return all structural and cross-field validation errors."""

    errors: list[str] = []
    required = {
        "schema_version",
        "experiment",
        "treatments",
        "reproduction",
        "correctness",
        "performance",
        "coverage",
        "decision",
    }
    missing = sorted(required - set(record))
    if missing:
        errors.append(
            "contract is missing required keys: " + ", ".join(missing)
        )
    if record.get("schema_version") != SCHEMA_VERSION:
        errors.append(
            f"schema_version must be exactly {SCHEMA_VERSION}"
        )
    _validate_experiment(record.get("experiment"), errors)
    _validate_treatments(record.get("treatments"), errors)
    _validate_reproduction(
        record.get("reproduction"),
        errors,
        repo_root=repo_root,
        verify_artifacts=verify_artifacts,
    )
    correctness_status = _validate_correctness(
        record.get("correctness"),
        errors,
    )
    _validate_performance(record.get("performance"), errors)
    _validate_coverage(record.get("coverage"), errors)
    _validate_decision(
        record.get("decision"),
        errors,
        correctness_status=correctness_status,
    )
    return errors


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "contracts",
        nargs="+",
        type=Path,
        help="result-contract JSON files",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    parser.add_argument(
        "--verify-artifacts",
        action="store_true",
        help="verify every declared artifact's size and SHA-256",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    failed = False
    for contract_path in arguments.contracts:
        try:
            record = load_json(contract_path.resolve())
            errors = validate_record(
                record,
                repo_root=repo_root,
                verify_artifacts=arguments.verify_artifacts,
            )
        except (OSError, ValueError, json.JSONDecodeError) as error:
            errors = [str(error)]
        if errors:
            failed = True
            for error in errors:
                print(f"{contract_path}: {error}", file=sys.stderr)
        else:
            suffix = " and artifacts" if arguments.verify_artifacts else ""
            print(f"OK: {contract_path} contract{suffix}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
