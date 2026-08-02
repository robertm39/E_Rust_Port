#!/usr/bin/env python3
"""Validate a guarded CASC checkpoint without extracting archive members."""

from __future__ import annotations

import argparse
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

SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
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
    run_name: str,
) -> tuple[dict[str, str], dict[str, bytes], int]:
    prefix = f"casc-runs/{run_name}/"
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
            capture = name.startswith(prefix) and name.endswith(".json")
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


def validate_run(
    *,
    hashes: dict[str, str],
    structured: dict[str, bytes],
    run_name: str,
    manifest_path: Path,
    contract_id: str,
    expected_results: int,
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
    if contract_name not in structured or summary_name not in structured:
        raise ValidationError("target run is missing contract.json or summary.json")
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

    summary = parse_json(structured[summary_name], summary_name)
    expected_total = len(records) * 2
    summary_checks = {
        "contract_id": contract_id,
        "manifest_sha256": manifest_hash,
        "completed_results": expected_results,
        "expected_results": expected_total,
        "missing_results": expected_total - expected_results,
        "complete": expected_results == expected_total,
    }
    for field, expected in summary_checks.items():
        if summary.get(field) != expected:
            raise ValidationError(f"summary has incompatible {field}")
    for solver in ("umlaut", "vampire"):
        solver_summary = summary.get("solvers", {}).get(solver)
        if not isinstance(solver_summary, dict):
            raise ValidationError(f"summary has no {solver} section")
        completed = solver_summary.get("groups", {}).get("overall", {}).get(
            "all", {}
        ).get("completed")
        if completed != solver_counts[solver]:
            raise ValidationError(f"summary {solver} count mismatch")

    session_names = sorted(
        name
        for name in structured
        if name.startswith(f"{prefix}sessions/") and name.endswith(".json")
    )
    if not session_names:
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
        "contract_file_sha256": hashes[contract_name],
        "contract_id": contract_id,
        "manifest_sha256": manifest_hash,
        "problem_count": len(records),
        "expected_results": expected_total,
        "completed_results": expected_results,
        "missing_results": expected_total - expected_results,
        "result_counts": dict(sorted(solver_counts.items())),
        "session_count": len(session_names),
        "summary_sha256": hashes[summary_name],
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--archive-sha256", required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--run-name", required=True)
    parser.add_argument("--contract-id", required=True)
    parser.add_argument("--expected-results", type=int, required=True)
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
        actual_archive_hash = sha256_file(archive)
        if actual_archive_hash != expected_archive_hash:
            raise ValidationError("checkpoint archive SHA-256 mismatch")

        root = checkpoint_root_name(archive)
        with tempfile.TemporaryDirectory(prefix="umlaut-casc-checkpoint-") as temporary:
            inner_path = Path(temporary) / "casc-runs.tar.gz"
            outer = copy_validated_inner_archive(archive, root, inner_path)
            hashes, structured, inner_member_count = read_inner_archive(
                inner_path, arguments.run_name
            )
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
            "run": run,
        }
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
