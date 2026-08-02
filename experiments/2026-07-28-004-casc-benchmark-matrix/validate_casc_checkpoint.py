#!/usr/bin/env python3
"""Validate a guarded CASC checkpoint without extracting archive members."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import re
import sys
import tarfile
import tempfile
from collections import Counter
from pathlib import Path, PurePosixPath
from typing import Any, BinaryIO, Sequence

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "casc_benchmark"))

from manifest import ManifestError, load_manifest, sha256_file  # noqa: E402
from report import (  # noqa: E402
    REPORT_SCHEMA_VERSION,
    overlap_summary,
    solver_summary,
)

SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
RUN_NAME_PATTERN = re.compile(r"^[a-z0-9][a-z0-9.-]*$")
RESULT_PATTERN = re.compile(
    r"^results/(?P<solver>umlaut|vampire)/(?P<category>[a-z0-9]+)/"
    r"(?P<key>[a-z0-9-]+)\.json$"
)
JSON_LIMIT = 16 * 1024 * 1024


class ValidationError(RuntimeError):
    """Raised when checkpoint evidence violates its recorded contract."""


def sha256_stream(source: BinaryIO) -> str:
    digest = hashlib.sha256()
    while chunk := source.read(1024 * 1024):
        digest.update(chunk)
    return digest.hexdigest()


def copy_and_hash(source: BinaryIO, destination: BinaryIO) -> str:
    digest = hashlib.sha256()
    while chunk := source.read(1024 * 1024):
        digest.update(chunk)
        destination.write(chunk)
    return digest.hexdigest()


def validated_member_name(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    if (
        not name
        or path.is_absolute()
        or "\\" in name
        or ".." in path.parts
        or "." in path.parts
        or path.as_posix() != name
    ):
        raise ValidationError(f"unsafe archive member path: {name!r}")
    return path


def regular_members(archive: tarfile.TarFile) -> dict[str, tarfile.TarInfo]:
    result: dict[str, tarfile.TarInfo] = {}
    seen: set[str] = set()
    for member in archive.getmembers():
        validated_member_name(member.name)
        if not member.isfile() and not member.isdir():
            raise ValidationError(
                f"archive member is not a regular file or directory: {member.name}"
            )
        if member.name in seen:
            raise ValidationError(f"duplicate archive member: {member.name}")
        seen.add(member.name)
        if member.isfile():
            result[member.name] = member
    return result


def member_bytes(
    archive: tarfile.TarFile,
    member: tarfile.TarInfo,
    *,
    limit: int = JSON_LIMIT,
) -> bytes:
    if member.size > limit:
        raise ValidationError(
            f"archive member is too large for structured parsing: {member.name}"
        )
    source = archive.extractfile(member)
    if source is None:
        raise ValidationError(f"cannot read archive member: {member.name}")
    with source:
        value = source.read(limit + 1)
    if len(value) != member.size:
        raise ValidationError(f"short archive member read: {member.name}")
    return value


def parse_json(data: bytes, name: str) -> dict[str, Any]:
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValidationError(f"invalid JSON in {name}: {error}") from error
    if not isinstance(value, dict):
        raise ValidationError(f"expected a JSON object in {name}")
    return value


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def parse_sha256s(data: bytes) -> dict[str, str]:
    try:
        lines = data.decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise ValidationError("SHA256SUMS is not UTF-8") from error
    result: dict[str, str] = {}
    for line in lines:
        parts = line.split(maxsplit=1)
        if len(parts) != 2 or not SHA256_PATTERN.fullmatch(parts[0]):
            raise ValidationError(f"malformed SHA256SUMS line: {line!r}")
        name = parts[1]
        if name.startswith("*"):
            name = name[1:]
        if not name or "/" in name or "\\" in name or name in result:
            raise ValidationError(f"unsafe or duplicate SHA256SUMS name: {name!r}")
        result[name] = parts[0]
    if not result:
        raise ValidationError("SHA256SUMS is empty")
    return result


def checkpoint_root_name(path: Path) -> str:
    name = path.name
    if not name.endswith(".tar.gz"):
        raise ValidationError("checkpoint archive name must end in .tar.gz")
    root = name[: -len(".tar.gz")]
    validated_member_name(root)
    return root


def copy_validated_inner_archive(
    checkpoint: Path,
    expected_root: str,
    inner_path: Path,
) -> dict[str, Any]:
    with tarfile.open(checkpoint, mode="r:gz") as archive:
        members = regular_members(archive)
        prefix = f"{expected_root}/"
        if any(not name.startswith(prefix) for name in members):
            raise ValidationError("outer archive contains another top-level root")
        sums_name = f"{expected_root}/SHA256SUMS"
        if sums_name not in members:
            raise ValidationError("outer archive has no SHA256SUMS")
        sums = parse_sha256s(member_bytes(archive, members[sums_name]))
        expected_files = {
            name.removeprefix(prefix)
            for name in members
            if name != sums_name
        }
        if set(sums) != expected_files:
            missing = sorted(expected_files - set(sums))
            extra = sorted(set(sums) - expected_files)
            raise ValidationError(
                f"outer checksum inventory mismatch; missing={missing}, extra={extra}"
            )

        inner_name = f"{expected_root}/casc-runs.tar.gz"
        if inner_name not in members:
            raise ValidationError("outer archive has no casc-runs.tar.gz")
        actual_hashes: dict[str, str] = {}
        for name, member in members.items():
            if name == sums_name:
                continue
            source = archive.extractfile(member)
            if source is None:
                raise ValidationError(f"cannot read outer member: {name}")
            short_name = name.removeprefix(prefix)
            with source:
                if name == inner_name:
                    with inner_path.open("wb") as output:
                        actual_hashes[short_name] = copy_and_hash(source, output)
                else:
                    actual_hashes[short_name] = sha256_stream(source)
        for name, expected in sums.items():
            if actual_hashes[name] != expected:
                raise ValidationError(f"outer member hash mismatch: {name}")

        maintenance_name = f"{expected_root}/package-maintenance-quiescence.json"
        if maintenance_name in members:
            maintenance = parse_json(
                member_bytes(archive, members[maintenance_name]), maintenance_name
            )
            units = maintenance.get("units")
            if not isinstance(units, dict) or not units:
                raise ValidationError("maintenance evidence has no unit records")
            for unit, state in units.items():
                if (
                    not isinstance(state, dict)
                    or state.get("active_state") != "inactive"
                ):
                    raise ValidationError(f"maintenance unit was not inactive: {unit}")
                if state.get("unit_file_state") != "masked":
                    raise ValidationError(f"maintenance unit was not masked: {unit}")
        for residue in ("cgroup-residue.txt", "solver-units.txt"):
            residue_name = f"{expected_root}/{residue}"
            if residue_name in members and members[residue_name].size != 0:
                raise ValidationError(f"nonempty lifecycle residue: {residue}")
        return {
            "member_count": len(members),
            "inner_bytes": members[inner_name].size,
            "inner_sha256": actual_hashes["casc-runs.tar.gz"],
        }


def read_inner_archive(
    inner_path: Path,
    run_names: Sequence[str],
) -> tuple[dict[str, str], dict[str, bytes], int]:
    prefixes = tuple(f"casc-runs/{run_name}/" for run_name in run_names)
    combined_name = "casc-runs/combined-summary.json"
    hashes: dict[str, str] = {}
    structured: dict[str, bytes] = {}
    member_count = 0
    with tarfile.open(inner_path, mode="r:gz") as archive:
        members = regular_members(archive)
        member_count = len(members)
        for name, member in members.items():
            source = archive.extractfile(member)
            if source is None:
                raise ValidationError(f"cannot read inner member: {name}")
            capture = name.endswith(".json") and (
                name == combined_name or name.startswith(prefixes)
            )
            if capture and member.size > JSON_LIMIT:
                raise ValidationError(f"structured member is too large: {name}")
            digest = hashlib.sha256()
            chunks: list[bytes] = []
            with source:
                while chunk := source.read(1024 * 1024):
                    digest.update(chunk)
                    if capture:
                        chunks.append(chunk)
            hashes[name] = digest.hexdigest()
            if capture:
                data = b"".join(chunks)
                if len(data) != member.size:
                    raise ValidationError(f"short inner member read: {name}")
                structured[name] = data
    return hashes, structured, member_count


def safe_result_key(index: int, record: dict[str, Any]) -> str:
    suffix = hashlib.sha256(record["problem_id"].encode()).hexdigest()[:12]
    return f"{index:04d}-{record['category'].lower()}-{suffix}"


def expected_run_report(
    metadata: dict[str, Any],
    records: Sequence[dict[str, Any]],
    contract: dict[str, Any],
    results: dict[tuple[str, str], dict[str, Any]],
) -> dict[str, Any]:
    solvers = sorted(contract["solvers"])
    expected_results = len(records) * len(solvers)
    missing_results = expected_results - len(results)
    report: dict[str, Any] = {
        "schema_version": REPORT_SCHEMA_VERSION,
        "kind": "umlaut-casc-benchmark-report",
        "contract_id": contract["contract_id"],
        "manifest_sha256": contract["manifest_sha256"],
        "canonical_full_selection": contract["canonical_full_selection"],
        "targeted_problems": len(records),
        "expected_results": expected_results,
        "completed_results": len(results),
        "missing_results": missing_results,
        "complete": missing_results == 0,
        "official_context_warning": (
            f"The checked-in CSVs for {metadata['corpus']} describe official "
            "competition entries. This local pinned Vampire run is not claimed "
            "to reproduce Vampire's official CASC configuration or StarExec "
            "environment."
        ),
        "manifest_partition_counts": metadata["partition_counts"],
        "solvers": {
            solver: solver_summary(solver, records, results) for solver in solvers
        },
    }
    if len(solvers) == 2:
        report["overlap"] = overlap_summary(
            solvers[0], solvers[1], records, results
        )
    return report


def validate_run(
    *,
    hashes: dict[str, str],
    structured: dict[str, bytes],
    run_name: str,
    manifest_path: Path,
    contract_id: str,
    expected_results: int,
    allow_missing_summary: bool = False,
) -> dict[str, Any]:
    metadata, records = load_manifest(manifest_path)
    manifest_hash = sha256_file(manifest_path)
    records_by_id = {
        record["problem_id"]: (index, record)
        for index, record in enumerate(records, start=1)
    }
    prefix = f"casc-runs/{run_name}/"
    contract_name = f"{prefix}contract.json"
    summary_name = f"{prefix}summary.json"
    if contract_name not in structured:
        raise ValidationError("target run is missing contract.json")
    if summary_name not in structured and not allow_missing_summary:
        raise ValidationError("target run is missing summary.json")
    contract = parse_json(structured[contract_name], contract_name)
    if contract.get("contract_id") != contract_id:
        raise ValidationError("target contract ID mismatch")
    contract_without_id = dict(contract)
    contract_without_id.pop("contract_id", None)
    computed_contract_id = hashlib.sha256(
        (
            json.dumps(contract_without_id, sort_keys=True, separators=(",", ":"))
            + "\n"
        ).encode()
    ).hexdigest()
    if computed_contract_id != contract_id:
        raise ValidationError("target contract content does not hash to its ID")
    if contract.get("manifest_sha256") != manifest_hash:
        raise ValidationError("target contract manifest hash mismatch")
    selected_ids = [record["problem_id"] for record in records]
    if contract.get("selected_problem_ids") != selected_ids:
        raise ValidationError("target contract selection differs from the manifest")
    selected_hash = hashlib.sha256(
        ("\n".join(selected_ids) + "\n").encode()
    ).hexdigest()
    if contract.get("selected_problem_ids_sha256") != selected_hash:
        raise ValidationError("target contract selection hash mismatch")
    if contract.get("selected_problem_count") != len(records):
        raise ValidationError("target contract problem count mismatch")
    if contract.get("presentation_id") != metadata["presentation"]["id"]:
        raise ValidationError("target contract presentation mismatch")

    result_prefix = f"{prefix}results/"
    result_names = sorted(
        name
        for name in structured
        if name.startswith(result_prefix) and name.endswith(".json")
    )
    if len(result_names) != expected_results:
        raise ValidationError(
            f"expected {expected_results} result records, found {len(result_names)}"
        )
    identities: set[tuple[str, str]] = set()
    results: dict[tuple[str, str], dict[str, Any]] = {}
    expected_artifacts: set[str] = set()
    solver_counts: Counter[str] = Counter()
    for name in result_names:
        relative = name.removeprefix(prefix)
        match = RESULT_PATTERN.fullmatch(relative)
        if match is None:
            raise ValidationError(f"invalid result path: {name}")
        result = parse_json(structured[name], name)
        solver = match.group("solver")
        problem_id = result.get("problem_id")
        if not isinstance(problem_id, str) or problem_id not in records_by_id:
            raise ValidationError(f"unknown result problem ID in {name}")
        index, record = records_by_id[problem_id]
        expected_name = (
            f"{result_prefix}{solver}/{record['category'].lower()}/"
            f"{safe_result_key(index, record)}.json"
        )
        if name != expected_name:
            raise ValidationError(f"result is stored at the wrong path: {name}")
        checks = {
            "contract_id": contract_id,
            "solver": solver,
            "problem_sha256": record["sha256"],
        }
        for field, expected in checks.items():
            if result.get(field) != expected:
                raise ValidationError(f"result {name} has incompatible {field}")
        identity = (solver, problem_id)
        if identity in identities:
            raise ValidationError(f"duplicate result identity: {identity}")
        identities.add(identity)
        results[identity] = result
        solver_counts[solver] += 1
        base = name[: -len(".json")]
        for suffix, field in (
            (".stdout", "stdout_sha256"),
            (".stderr", "stderr_sha256"),
        ):
            artifact = f"{base}{suffix}"
            expected_artifacts.add(artifact)
            if artifact not in hashes:
                raise ValidationError(f"missing result artifact: {artifact}")
            if hashes[artifact] != result.get(field):
                raise ValidationError(f"result artifact hash mismatch: {artifact}")
    present_artifacts = {
        name
        for name in hashes
        if name.startswith(result_prefix)
        and (name.endswith(".stdout") or name.endswith(".stderr"))
    }
    if present_artifacts != expected_artifacts:
        raise ValidationError("orphan or unreferenced result artifacts are present")

    expected_total = len(records) * 2
    expected_summary = expected_run_report(metadata, records, contract, results)
    if summary_name in structured:
        summary = parse_json(structured[summary_name], summary_name)
        if summary != expected_summary:
            raise ValidationError(
                "summary does not reproduce from validated results"
            )
        summary_sha256 = hashes[summary_name]
        summary_embedded = True
    else:
        summary = expected_summary
        summary_sha256 = hashlib.sha256(canonical_json(summary)).hexdigest()
        summary_embedded = False

    session_names = sorted(
        name
        for name in structured
        if name.startswith(f"{prefix}sessions/") and name.endswith(".json")
    )
    if not session_names and expected_results:
        raise ValidationError("target run has no session records")
    for name in session_names:
        session = parse_json(structured[name], name)
        if session.get("contract_id") != contract_id:
            raise ValidationError(f"session belongs to another contract: {name}")
        runner = session.get("runner")
        if not isinstance(runner, dict) or any(
            runner.get(field) is None for field in ("label", "run_id", "linode_id")
        ):
            raise ValidationError(f"session has incomplete runner identity: {name}")
    return {
        "run_name": run_name,
        "corpus": metadata["corpus"],
        "official_csv_count": len(
            metadata["sources"]["official_result_file_sha256"]
        ),
        "contract_file_sha256": hashes[contract_name],
        "contract_id": contract_id,
        "manifest_sha256": manifest_hash,
        "problem_count": len(records),
        "expected_results": expected_total,
        "completed_results": expected_results,
        "missing_results": expected_total - expected_results,
        "result_counts": dict(sorted(solver_counts.items())),
        "session_count": len(session_names),
        "summary_sha256": summary_sha256,
        "summary_embedded": summary_embedded,
        "_metadata": metadata,
        "_records": records,
        "_contract": contract,
        "_results": results,
        "_summary": summary,
    }


def public_run_evidence(run: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in run.items() if not key.startswith("_")}


def expected_combined_report(
    specifications: Sequence[tuple[str, Path, str, str]],
    runs: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    release_reports: dict[str, dict[str, Any]] = {}
    combined_records: list[dict[str, Any]] = []
    combined_results: dict[tuple[str, str], dict[str, Any]] = {}
    expected_solvers: list[str] | None = None
    official_csv_count = 0
    for release, _manifest, _run_name, _contract_id in specifications:
        run = runs[release]
        contract = run["_contract"]
        solvers = sorted(contract["solvers"])
        if expected_solvers is None:
            expected_solvers = solvers
        elif solvers != expected_solvers:
            raise ValidationError("combined runs have different solver sets")
        for record in run["_records"]:
            prefixed_id = f"{release}:{record['problem_id']}"
            combined_record = dict(record)
            combined_record["problem_id"] = prefixed_id
            combined_record["release"] = release
            combined_records.append(combined_record)
            for solver in solvers:
                result = run["_results"].get((solver, record["problem_id"]))
                if result is None:
                    continue
                combined_result = dict(result)
                combined_result["problem_id"] = prefixed_id
                combined_result["release"] = release
                combined_results[(solver, prefixed_id)] = combined_result
        official_csv_count += run["official_csv_count"]
        release_reports[release] = {
            "corpus": run["corpus"],
            "manifest_sha256": run["manifest_sha256"],
            "contract_id": run["contract_id"],
            "official_csv_count": run["official_csv_count"],
            "summary": copy.deepcopy(run["_summary"]),
        }

    if expected_solvers is None:  # pragma: no cover - callers require two runs.
        raise AssertionError("combined solver set was not initialized")
    expected_results = len(combined_records) * len(expected_solvers)
    missing_results = expected_results - len(combined_results)
    value: dict[str, Any] = {
        "schema_version": 1,
        "kind": "umlaut-casc-combined-benchmark-report",
        "complete": missing_results == 0,
        "targeted_problems": len(combined_records),
        "expected_results": expected_results,
        "completed_results": len(combined_results),
        "missing_results": missing_results,
        "official_context": {
            "csv_count": official_csv_count,
            "warning": (
                "Official result CSVs are contextual. Local Umlaut and pinned "
                "Vampire runs are not claimed to reproduce official entries or "
                "the StarExec environment."
            ),
        },
        "releases": dict(sorted(release_reports.items())),
        "solvers": {
            solver: solver_summary(solver, combined_records, combined_results)
            for solver in expected_solvers
        },
    }
    if len(expected_solvers) == 2:
        value["overlap"] = overlap_summary(
            expected_solvers[0],
            expected_solvers[1],
            combined_records,
            combined_results,
        )
    return value


def validate_combined_summary(
    *,
    summary: dict[str, Any],
    structured: dict[str, bytes],
    specifications: Sequence[tuple[str, Path, str, str]],
    runs: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    expected = expected_combined_report(specifications, runs)
    if summary != expected:
        raise ValidationError(
            "combined summary does not reproduce from validated results"
        )
    return {
        "summary_sha256": hashlib.sha256(
            structured["casc-runs/combined-summary.json"]
        ).hexdigest(),
        "embedded": True,
        "releases": sorted(runs),
        "targeted_problems": expected["targeted_problems"],
        "expected_results": expected["expected_results"],
        "completed_results": expected["completed_results"],
        "missing_results": expected["missing_results"],
        "official_csv_count": expected["official_context"]["csv_count"],
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--run-name", required=True)
    parser.add_argument("--contract-id", required=True)
    parser.add_argument("--expected-results", type=int, required=True)
    parser.add_argument(
        "--combined-run",
        action="append",
        nargs=4,
        metavar=("RELEASE", "MANIFEST", "RUN_NAME", "CONTRACT_ID"),
        help=(
            "require the embedded combined report and validate one constituent "
            "release; repeat for every combined release"
        ),
    )
    parser.add_argument(
        "--combined-output",
        type=Path,
        help=(
            "write the exactly reconstructed combined report; this also permits "
            "auditing a legacy checkpoint that predates its embedded report"
        ),
    )
    parser.add_argument("--output", type=Path)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        archive = arguments.archive.resolve()
        manifest = arguments.manifest.resolve()
        expected_archive_hash = arguments.archive_sha256.lower()
        contract_id = arguments.contract_id.lower()
        if not archive.is_file():
            raise ValidationError(f"checkpoint archive does not exist: {archive}")
        if not manifest.is_file():
            raise ValidationError(f"manifest does not exist: {manifest}")
        if not SHA256_PATTERN.fullmatch(expected_archive_hash):
            raise ValidationError("--archive-sha256 must be 64 lowercase hex digits")
        if not SHA256_PATTERN.fullmatch(contract_id):
            raise ValidationError("--contract-id must be 64 lowercase hex digits")
        if arguments.expected_results < 0:
            raise ValidationError("--expected-results cannot be negative")
        if not RUN_NAME_PATTERN.fullmatch(arguments.run_name):
            raise ValidationError("--run-name is not a safe run identifier")

        combined_specifications: list[tuple[str, Path, str, str]] = []
        for raw in arguments.combined_run or []:
            release, raw_manifest, run_name, raw_contract_id = raw
            combined_manifest = Path(raw_manifest).resolve()
            combined_contract_id = raw_contract_id.lower()
            if not release or not RUN_NAME_PATTERN.fullmatch(run_name):
                raise ValidationError("--combined-run has an invalid identifier")
            if not combined_manifest.is_file():
                raise ValidationError(
                    f"combined manifest does not exist: {combined_manifest}"
                )
            if not SHA256_PATTERN.fullmatch(combined_contract_id):
                raise ValidationError("--combined-run contract ID is invalid")
            combined_specifications.append(
                (release, combined_manifest, run_name, combined_contract_id)
            )
        if combined_specifications:
            releases = [value[0] for value in combined_specifications]
            run_names = [value[2] for value in combined_specifications]
            if len(combined_specifications) < 2:
                raise ValidationError("combined validation requires at least two runs")
            if len(set(releases)) != len(releases):
                raise ValidationError("duplicate --combined-run release")
            if len(set(run_names)) != len(run_names):
                raise ValidationError("duplicate --combined-run run name")
            selected = [
                value
                for value in combined_specifications
                if value[1] == manifest
                and value[2] == arguments.run_name
                and value[3] == contract_id
            ]
            if len(selected) != 1:
                raise ValidationError(
                    "selected run is not uniquely present in --combined-run inputs"
                )
            selected_release = selected[0][0]
        else:
            run_names = [arguments.run_name]
            selected_release = None
        if arguments.combined_output is not None and not combined_specifications:
            raise ValidationError("--combined-output requires --combined-run inputs")
        actual_archive_hash = sha256_file(archive)
        if actual_archive_hash != expected_archive_hash:
            raise ValidationError("checkpoint archive SHA-256 mismatch")

        root = checkpoint_root_name(archive)
        with tempfile.TemporaryDirectory(prefix="umlaut-casc-checkpoint-") as temporary:
            inner_path = Path(temporary) / "casc-runs.tar.gz"
            outer = copy_validated_inner_archive(archive, root, inner_path)
            hashes, structured, inner_member_count = read_inner_archive(
                inner_path, run_names
            )
        combined = None
        if combined_specifications:
            combined_name = "casc-runs/combined-summary.json"
            if (
                combined_name not in structured
                and arguments.combined_output is None
            ):
                raise ValidationError("checkpoint has no combined-summary.json")
            result_counts = {
                release: sum(
                    name.startswith(f"casc-runs/{run_name}/results/")
                    and name.endswith(".json")
                    for name in structured
                )
                for release, _manifest, run_name, _contract_id in (
                    combined_specifications
                )
            }
            if result_counts[selected_release] != arguments.expected_results:
                raise ValidationError(
                    "selected result count differs from combined summary"
                )
            runs = {
                release: validate_run(
                    hashes=hashes,
                    structured=structured,
                    run_name=run_name,
                    manifest_path=run_manifest,
                    contract_id=run_contract_id,
                    expected_results=result_counts[release],
                    allow_missing_summary=arguments.combined_output is not None,
                )
                for release, run_manifest, run_name, run_contract_id in (
                    combined_specifications
                )
            }
            run = runs[selected_release]
            expected_combined = expected_combined_report(
                combined_specifications, runs
            )
            expected_combined_bytes = canonical_json(expected_combined)
            if combined_name in structured:
                combined_summary = parse_json(
                    structured[combined_name], combined_name
                )
                combined = validate_combined_summary(
                    summary=combined_summary,
                    structured=structured,
                    specifications=combined_specifications,
                    runs=runs,
                )
            else:
                combined = {
                    "summary_sha256": hashlib.sha256(
                        expected_combined_bytes
                    ).hexdigest(),
                    "releases": sorted(runs),
                    "targeted_problems": expected_combined["targeted_problems"],
                    "expected_results": expected_combined["expected_results"],
                    "completed_results": expected_combined[
                        "completed_results"
                    ],
                    "missing_results": expected_combined["missing_results"],
                    "official_csv_count": expected_combined[
                        "official_context"
                    ]["csv_count"],
                    "embedded": False,
                }
            if arguments.combined_output is not None:
                combined_destination = arguments.combined_output.resolve()
                combined_destination.parent.mkdir(parents=True, exist_ok=True)
                combined_destination.write_bytes(expected_combined_bytes)
        else:
            run = validate_run(
                hashes=hashes,
                structured=structured,
                run_name=arguments.run_name,
                manifest_path=manifest,
                contract_id=contract_id,
                expected_results=arguments.expected_results,
            )
        result = {
            "schema_version": 1,
            "kind": "umlaut-casc-checkpoint-validation",
            "archive": {
                "path": str(archive),
                "bytes": archive.stat().st_size,
                "sha256": actual_archive_hash,
                "root": root,
                "outer_regular_member_count": outer["member_count"],
            },
            "inner_archive": {
                "bytes": outer["inner_bytes"],
                "sha256": outer["inner_sha256"],
                "regular_member_count": inner_member_count,
            },
            "run": public_run_evidence(run),
        }
        if combined is not None:
            result["combined"] = combined
        output = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if arguments.output is not None:
            destination = arguments.output.resolve()
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(output, encoding="utf-8", newline="\n")
        print(output, end="")
        return 0
    except (ManifestError, OSError, tarfile.TarError, ValidationError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
