#!/usr/bin/env python3
"""Build and audit self-contained Umlaut source and Linux runtime packages."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import platform
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
FORBIDDEN_COMPONENTS = {
    ".artifacts",
    ".beads",
    ".dolt",
    ".git",
    "cadical",
    "eprover",
    "experiments",
    "gmp-6.3.0",
    "minisat",
    "problems",
    "target",
    "vampire",
    "z3",
}
FORBIDDEN_LIBRARY_NAMES = (
    "cadical",
    "gmp",
    "minisat",
    "picosat",
    "vampire",
    "viras",
    "z3",
)
REQUIRED_SOURCE_PATHS = {
    "Cargo.lock",
    "Cargo.toml",
    "LICENSE",
    "README.md",
    "THIRD_PARTY_NOTICES.md",
    "build.rs",
    "docs/dependency-packaging-matrix.md",
    "src/heuristics/schedule.vars",
    "tools/packaging/README-CASC.md",
    "tools/packaging/verify_casc_package.py",
}


class AuditError(RuntimeError):
    """Raised when a package violates a required boundary."""


def run(
    command: list[str],
    *,
    cwd: Path,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    """Run one command and preserve its output for actionable failures."""

    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if result.returncode != 0:
        rendered = " ".join(command)
        raise AuditError(
            f"command failed with exit {result.returncode}: {rendered}\n"
            f"{result.stdout}"
        )
    return result


def sha256(path: Path) -> str:
    """Return the lowercase SHA-256 of one file."""

    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def safe_member_path(name: str) -> PurePosixPath:
    """Validate one archive member path and return its normalized components."""

    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts:
        raise AuditError(f"unsafe archive member path: {name}")
    return path


def source_members(crate_path: Path) -> tuple[str, list[str]]:
    """Validate the Cargo source archive and return its root and members."""

    with tarfile.open(crate_path, "r:gz") as archive:
        members = archive.getmembers()
    if not members:
        raise AuditError("Cargo source archive is empty")

    roots = {safe_member_path(member.name).parts[0] for member in members}
    if len(roots) != 1:
        raise AuditError(f"Cargo source archive has multiple roots: {sorted(roots)}")
    root = roots.pop()
    relative_names: list[str] = []
    for member in members:
        path = safe_member_path(member.name)
        relative = PurePosixPath(*path.parts[1:])
        if not relative.parts:
            continue
        lowered = {part.lower() for part in relative.parts}
        forbidden = sorted(lowered & FORBIDDEN_COMPONENTS)
        if forbidden:
            raise AuditError(
                f"forbidden component {forbidden[0]} entered source archive "
                f"through {member.name}"
            )
        if member.issym() or member.islnk():
            raise AuditError(f"source archive must not contain links: {member.name}")
        relative_names.append(relative.as_posix())

    missing = sorted(REQUIRED_SOURCE_PATHS - set(relative_names))
    if missing:
        raise AuditError(f"source archive is missing required paths: {missing}")
    if any(name.lower().endswith(".pdf") for name in relative_names):
        raise AuditError("source archive unexpectedly contains a PDF")
    return root, sorted(relative_names)


def extract_source(crate_path: Path, destination: Path) -> Path:
    """Extract an already validated source archive."""

    root, _members = source_members(crate_path)
    with tarfile.open(crate_path, "r:gz") as archive:
        archive.extractall(destination, filter="data")
    extracted = destination / root
    if not extracted.is_dir():
        raise AuditError(f"expected extracted source root {extracted}")
    return extracted


def package_names(manifest_path: Path) -> tuple[str, str, list[str]]:
    """Read package identity and declared binary targets."""

    with manifest_path.open("rb") as source:
        manifest = tomllib.load(source)
    package = manifest.get("package")
    if not isinstance(package, dict):
        raise AuditError("Cargo.toml has no package table")
    name = package.get("name")
    version = package.get("version")
    binaries = manifest.get("bin")
    if not isinstance(name, str) or not isinstance(version, str):
        raise AuditError("Cargo.toml package name/version is invalid")
    if not isinstance(binaries, list):
        raise AuditError("Cargo.toml declares no binaries")
    bin_names = [
        entry["name"]
        for entry in binaries
        if isinstance(entry, dict) and isinstance(entry.get("name"), str)
    ]
    if "umlaut" not in bin_names:
        raise AuditError("Cargo.toml does not declare the primary umlaut binary")
    return name, version, bin_names


def assert_dependency_free(lock_path: Path) -> None:
    """Require the baseline package to contain only the Umlaut lock entry."""

    with lock_path.open("rb") as source:
        lock = tomllib.load(source)
    packages = lock.get("package")
    if (
        not isinstance(packages, list)
        or len(packages) != 1
        or packages[0].get("name") != "umlaut"
        or "dependencies" in packages[0]
    ):
        raise AuditError(
            "baseline Cargo.lock must contain only dependency-free Umlaut"
        )


def add_deterministic_file(
    archive: tarfile.TarFile,
    source: Path,
    archive_name: str,
    *,
    executable: bool,
) -> None:
    """Add one regular file with stable metadata."""

    info = tarfile.TarInfo(archive_name)
    info.size = source.stat().st_size
    info.mode = 0o755 if executable else 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = "root"
    info.gname = "root"
    with source.open("rb") as payload:
        archive.addfile(info, payload)


def build_runtime_archive(
    output_path: Path,
    *,
    root_name: str,
    binary: Path,
    source_root: Path,
) -> None:
    """Create the minimal, deterministic runtime candidate."""

    readme = source_root / "tools" / "packaging" / "README-CASC.md"
    notices = source_root / "THIRD_PARTY_NOTICES.md"
    license_path = source_root / "LICENSE"
    with output_path.open("wb") as raw_output:
        with gzip.GzipFile(
            filename="",
            fileobj=raw_output,
            mode="wb",
            mtime=0,
        ) as compressed:
            with tarfile.open(
                fileobj=compressed,
                mode="w",
                format=tarfile.PAX_FORMAT,
            ) as archive:
                add_deterministic_file(
                    archive,
                    binary,
                    f"{root_name}/bin/umlaut",
                    executable=True,
                )
                add_deterministic_file(
                    archive,
                    readme,
                    f"{root_name}/README.md",
                    executable=False,
                )
                add_deterministic_file(
                    archive,
                    license_path,
                    f"{root_name}/LICENSE",
                    executable=False,
                )
                add_deterministic_file(
                    archive,
                    notices,
                    f"{root_name}/THIRD_PARTY_NOTICES.md",
                    executable=False,
                )


def runtime_members(runtime_path: Path) -> list[str]:
    """Validate the runtime archive's minimal content."""

    with tarfile.open(runtime_path, "r:gz") as archive:
        members = archive.getmembers()
    names = sorted(member.name for member in members)
    if len(names) != 4:
        raise AuditError(f"runtime archive must contain exactly four files: {names}")
    for member in members:
        path = safe_member_path(member.name)
        if member.issym() or member.islnk() or not member.isfile():
            raise AuditError(f"invalid runtime archive member: {member.name}")
        lowered = "/".join(path.parts).lower()
        if any(name in lowered for name in FORBIDDEN_LIBRARY_NAMES):
            raise AuditError(f"optional backend entered runtime archive: {member.name}")
    return names


def command_version(command: list[str], *, cwd: Path) -> str:
    """Return a single-line tool version."""

    return run(command, cwd=cwd).stdout.strip()


def audit_dynamic_libraries(binary: Path, *, cwd: Path) -> str:
    """Reject linked optional backends and return the complete ldd report."""

    output = run(["ldd", str(binary)], cwd=cwd).stdout.strip()
    lowered = output.lower()
    found = [name for name in FORBIDDEN_LIBRARY_NAMES if name in lowered]
    if found:
        raise AuditError(f"runtime binary links forbidden optional backends: {found}")
    return output


def copy_source_archive(crate_path: Path, output_path: Path) -> None:
    """Copy Cargo's gzip tar archive to the CASC source-package filename."""

    shutil.copyfile(crate_path, output_path)


def parse_arguments(argv: Iterable[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="directory for audited source/runtime archives and manifest",
    )
    return parser.parse_args(argv)


def main(argv: Iterable[str] | None = None) -> int:
    args = parse_arguments(argv)
    if platform.system() != "Linux":
        raise AuditError("package build audit must run on the target Linux environment")

    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    name, version, _declared_bins = package_names(REPO_ROOT / "Cargo.toml")
    assert_dependency_free(REPO_ROOT / "Cargo.lock")

    with tempfile.TemporaryDirectory(prefix="umlaut-package-audit-") as temporary:
        temporary_root = Path(temporary)
        cargo_target = temporary_root / "cargo-target"
        package_env = os.environ.copy()
        package_env["CARGO_TARGET_DIR"] = str(cargo_target)
        package_result = run(
            [
                "cargo",
                "package",
                "--locked",
                "--no-verify",
                "--allow-dirty",
            ],
            cwd=REPO_ROOT,
            env=package_env,
        )
        crate_candidates = sorted((cargo_target / "package").glob("*.crate"))
        if len(crate_candidates) != 1:
            raise AuditError(
                f"expected one Cargo source archive, found {crate_candidates}"
            )
        crate_path = crate_candidates[0]
        _source_root_name, members = source_members(crate_path)

        extracted_root = extract_source(crate_path, temporary_root / "extracted")
        assert_dependency_free(extracted_root / "Cargo.lock")
        _name, _version, binary_names = package_names(
            extracted_root / "Cargo.toml"
        )
        release_target = temporary_root / "release-target"
        build_env = os.environ.copy()
        build_env["CARGO_TARGET_DIR"] = str(release_target)
        build_result = run(
            [
                "cargo",
                "build",
                "--locked",
                "--release",
                "--bins",
                "--offline",
            ],
            cwd=extracted_root,
            env=build_env,
        )
        missing_bins = [
            binary_name
            for binary_name in binary_names
            if not (release_target / "release" / binary_name).is_file()
        ]
        if missing_bins:
            raise AuditError(f"release build omitted binaries: {missing_bins}")

        primary_binary = release_target / "release" / "umlaut"
        version_output = run(
            [str(primary_binary), "--version"],
            cwd=extracted_root,
        ).stdout
        if not version_output.startswith(f"Umlaut {version}"):
            raise AuditError(f"unexpected Umlaut version output: {version_output}")
        dynamic_libraries = audit_dynamic_libraries(
            primary_binary,
            cwd=extracted_root,
        )

        source_output = output_dir / f"{name}-{version}-source.tgz"
        runtime_output = output_dir / f"{name}-{version}-casc-runtime.tgz"
        copy_source_archive(crate_path, source_output)
        build_runtime_archive(
            runtime_output,
            root_name=f"{name}-{version}",
            binary=primary_binary,
            source_root=extracted_root,
        )
        runtime_archive_members = runtime_members(runtime_output)

        manifest: dict[str, Any] = {
            "schema_version": 1,
            "package": {"name": name, "version": version},
            "platform": {
                "system": platform.platform(),
                "machine": platform.machine(),
                "rustc": command_version(["rustc", "--version"], cwd=REPO_ROOT),
                "cargo": command_version(["cargo", "--version"], cwd=REPO_ROOT),
            },
            "source_archive": {
                "file": source_output.name,
                "bytes": source_output.stat().st_size,
                "sha256": sha256(source_output),
                "members": len(members),
            },
            "runtime_archive": {
                "file": runtime_output.name,
                "bytes": runtime_output.stat().st_size,
                "sha256": sha256(runtime_output),
                "members": runtime_archive_members,
            },
            "primary_binary": {
                "bytes": primary_binary.stat().st_size,
                "sha256": sha256(primary_binary),
                "version_output": version_output.strip().splitlines(),
                "dynamic_libraries": dynamic_libraries.splitlines(),
            },
            "release_binaries": binary_names,
            "optional_backends": {
                "bundled": [],
                "linked": [],
                "internal_sat_fallback": True,
            },
            "checks": {
                "cargo_lock_dependency_free": True,
                "source_archive_forbidden_components_absent": True,
                "source_archive_pdfs_absent": True,
                "extracted_source_release_build_offline": True,
                "runtime_archive_minimal": True,
                "runtime_optional_backends_absent": True,
            },
            "commands": {
                "cargo_package": package_result.stdout.splitlines(),
                "cargo_build": build_result.stdout.splitlines(),
            },
        }
        manifest_path = output_dir / "package-audit.json"
        manifest_path.write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

    print(f"Package audit passed: {manifest_path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AuditError as error:
        print(f"package audit failed: {error}", file=os.sys.stderr)
        raise SystemExit(1) from error
