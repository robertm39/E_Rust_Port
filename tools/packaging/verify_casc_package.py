#!/usr/bin/env python3
"""Build and audit self-contained Umlaut source and Linux runtime packages."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import platform
import signal
import shutil
import subprocess
import tarfile
import tempfile
import time
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
    "docs/search-telemetry.md",
    "native/cadical_ffi/umlaut_cadical.cpp",
    "native/cadical_ffi/umlaut_cadical.h",
    "src/heuristics/schedule.vars",
    "tools/packaging/README-CASC.md",
    "tools/packaging/starexec_run_default",
    "tools/packaging/verify_casc_package.py",
}
RUNTIME_FILES = {
    "LICENSE": False,
    "THIRD_PARTY_NOTICES.md": False,
    "bin/starexec_run_default": True,
    "bin/umlaut": True,
    "starexec_description.txt": False,
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
    binary: Path,
    source_root: Path,
) -> None:
    """Create the minimal, deterministic StarExec installation package."""

    readme = source_root / "tools" / "packaging" / "README-CASC.md"
    wrapper = source_root / "tools" / "packaging" / "starexec_run_default"
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
                    "bin/umlaut",
                    executable=True,
                )
                add_deterministic_file(
                    archive,
                    wrapper,
                    "bin/starexec_run_default",
                    executable=True,
                )
                add_deterministic_file(
                    archive,
                    license_path,
                    "LICENSE",
                    executable=False,
                )
                add_deterministic_file(
                    archive,
                    notices,
                    "THIRD_PARTY_NOTICES.md",
                    executable=False,
                )
                add_deterministic_file(
                    archive,
                    readme,
                    "starexec_description.txt",
                    executable=False,
                )


def runtime_members(runtime_path: Path) -> list[str]:
    """Validate the StarExec archive's rootless, minimal content and modes."""

    with tarfile.open(runtime_path, "r:gz") as archive:
        members = archive.getmembers()
    names = sorted(member.name for member in members)
    forbidden_members = [
        name
        for name in names
        if any(library in name.lower() for library in FORBIDDEN_LIBRARY_NAMES)
    ]
    if forbidden_members:
        raise AuditError(
            f"optional backend entered runtime archive: {forbidden_members}"
        )
    expected = sorted(RUNTIME_FILES)
    if names != expected:
        raise AuditError(
            f"runtime archive members differ from StarExec allowlist: {names}"
        )
    for member in members:
        path = safe_member_path(member.name)
        if member.issym() or member.islnk() or not member.isfile():
            raise AuditError(f"invalid runtime archive member: {member.name}")
        expected_mode = 0o755 if RUNTIME_FILES[member.name] else 0o644
        if member.mode != expected_mode:
            raise AuditError(
                f"runtime member {member.name} has mode {member.mode:o}, "
                f"expected {expected_mode:o}"
            )
    return names


def extract_runtime(runtime_path: Path, destination: Path) -> Path:
    """Extract an already validated rootless StarExec installation package."""

    runtime_members(runtime_path)
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(runtime_path, "r:gz") as archive:
        archive.extractall(destination, filter="data")
    return destination


def file_tree(directory: Path) -> dict[str, str]:
    """Return stable relative-path hashes for every regular file below a root."""

    return {
        path.relative_to(directory).as_posix(): sha256(path)
        for path in sorted(directory.rglob("*"))
        if path.is_file()
    }


def require_success(result: subprocess.CompletedProcess[str], label: str) -> None:
    """Raise an actionable audit failure for one StarExec emulation command."""

    if result.returncode != 0:
        raise AuditError(
            f"{label} failed with exit {result.returncode}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )


def starexec_environment(
    *,
    tptp_root: Path,
    temporary_root: Path,
    cpu_limit: str = "17",
    memory_limit: str = "4096",
) -> dict[str, str]:
    """Build the public StarExec environment used by local emulation."""

    environment = os.environ.copy()
    environment.update(
        {
            "TPTP": str(tptp_root),
            "TMPDIR": str(temporary_root),
            "STAREXEC_WALLCLOCK_LIMIT": "60",
            "STAREXEC_CPU_LIMIT": cpu_limit,
            "STAREXEC_MAX_MEM": memory_limit,
            "STAREXEC_MAX_WRITE": "64",
        }
    )
    return environment


def audit_wrapper_arguments(
    install_root: Path,
    *,
    work_root: Path,
) -> dict[str, Any]:
    """Check wrapper quoting, resource arguments, and inherited environment."""

    bin_dir = install_root / "bin"
    binary = bin_dir / "umlaut"
    saved_binary = bin_dir / "umlaut.audit-real"
    wrapper = bin_dir / "starexec_run_default"
    audit_output = work_root / "wrapper-arguments.txt"
    problem = work_root / "problem path with spaces.p"
    output_dir = work_root / "output path with spaces"
    tptp_root = work_root / "TPTP path with spaces"
    temporary_root = work_root / "tmp-wrapper"
    for directory in (output_dir, tptp_root, temporary_root):
        directory.mkdir(parents=True)
    problem.write_text("fof(goal, conjecture, $true).\n", encoding="utf-8")
    fake_binary = """#!/bin/sh
{
    printf 'TPTP=%s\\n' "${TPTP:-}"
    printf 'WALL=%s\\n' "${STAREXEC_WALLCLOCK_LIMIT:-}"
    printf 'CPU=%s\\n' "${STAREXEC_CPU_LIMIT:-}"
    printf 'MEM=%s\\n' "${STAREXEC_MAX_MEM:-}"
    for argument in "$@"; do
        printf 'ARG=%s\\n' "$argument"
    done
} > "${UMLAUT_STAREXEC_AUDIT_FILE:?}"
printf '%% SZS status Theorem\\n'
"""
    binary.rename(saved_binary)
    try:
        binary.write_text(fake_binary, encoding="utf-8", newline="\n")
        binary.chmod(0o755)
        environment = starexec_environment(
            tptp_root=tptp_root,
            temporary_root=temporary_root,
        )
        environment["UMLAUT_STAREXEC_AUDIT_FILE"] = str(audit_output)
        result = subprocess.run(
            [str(wrapper), str(problem), str(output_dir)],
            cwd=bin_dir,
            env=environment,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=10,
        )
        require_success(result, "StarExec wrapper argument audit")
    finally:
        binary.unlink(missing_ok=True)
        saved_binary.rename(binary)

    lines = audit_output.read_text(encoding="utf-8").splitlines()
    expected_arguments = [
        "ARG=--auto",
        "ARG=--tstp-out",
        "ARG=--proof-object=1",
        "ARG=--output-level=0",
        "ARG=--cpu-limit=17",
        "ARG=--memory-limit=4096",
        f"ARG={problem}",
    ]
    expected_environment = [
        f"TPTP={tptp_root}",
        "WALL=60",
        "CPU=17",
        "MEM=4096",
    ]
    if lines != expected_environment + expected_arguments:
        raise AuditError(f"unexpected StarExec wrapper contract: {lines}")
    if result.stdout != "% SZS status Theorem\n" or result.stderr:
        raise AuditError(
            "StarExec wrapper did not preserve the fake solver's output streams"
        )
    return {
        "arguments": [line.removeprefix("ARG=") for line in expected_arguments],
        "environment": {
            "TPTP": str(tptp_root),
            "STAREXEC_WALLCLOCK_LIMIT": "60",
            "STAREXEC_CPU_LIMIT": "17",
            "STAREXEC_MAX_MEM": "4096",
        },
        "problem_path_with_spaces_preserved": True,
    }


def audit_include_job(
    install_root: Path,
    *,
    work_root: Path,
) -> dict[str, Any]:
    """Run an include-using TPTP theorem through the extracted package."""

    tptp_root = work_root / "TPTP"
    axioms = tptp_root / "Axioms"
    problems = work_root / "problems"
    output_dir = work_root / "output"
    temporary_root = work_root / "tmp-normal"
    for directory in (axioms, problems, output_dir, temporary_root):
        directory.mkdir(parents=True)
    (axioms / "UML001.ax").write_text(
        "fof(umlaut_include_axiom, axiom, p(a)).\n",
        encoding="utf-8",
    )
    problem = problems / "UML001+1.p"
    problem.write_text(
        "include('Axioms/UML001.ax').\n"
        "fof(umlaut_include_goal, conjecture, p(a)).\n",
        encoding="utf-8",
    )

    install_before = file_tree(install_root)
    environment = starexec_environment(
        tptp_root=tptp_root,
        temporary_root=temporary_root,
    )
    result = subprocess.run(
        [
            str(install_root / "bin" / "starexec_run_default"),
            str(problem),
            str(output_dir),
        ],
        cwd=install_root / "bin",
        env=environment,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=30,
    )
    require_success(result, "include-using StarExec job")
    required_output = (
        "% SZS status Theorem",
        "% SZS output start CNFRefutation",
        "% SZS output end CNFRefutation",
    )
    missing = [marker for marker in required_output if marker not in result.stdout]
    if missing:
        raise AuditError(
            f"include-using StarExec job omitted SZS markers {missing}\n"
            f"{result.stdout}"
        )
    if result.stderr:
        raise AuditError(f"include-using StarExec job wrote stderr:\n{result.stderr}")
    if file_tree(install_root) != install_before:
        raise AuditError("normal StarExec job modified the installation tree")
    if any(temporary_root.iterdir()) or any(output_dir.iterdir()):
        raise AuditError("normal StarExec job left temporary or output files")
    return {
        "exit_status": result.returncode,
        "problem": problem.relative_to(work_root).as_posix(),
        "include": "Axioms/UML001.ax",
        "status": "Theorem",
        "proof_delimiters": "CNFRefutation",
        "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
        "stderr_bytes": len(result.stderr.encode()),
        "normal_exit_files_created": [],
    }


def audit_external_signal(
    install_root: Path,
    *,
    work_root: Path,
    signal_number: int,
    signal_name: str,
) -> dict[str, Any]:
    """Require one CASC termination signal to produce ResourceOut promptly."""

    signal_root = work_root / signal_name
    output_dir = signal_root / "output"
    temporary_root = signal_root / "tmp"
    tptp_root = signal_root / "TPTP"
    for directory in (output_dir, temporary_root, tptp_root):
        directory.mkdir(parents=True)
    problem = work_root / "large-signal-problem.p"
    if not problem.exists():
        with problem.open("wb") as output:
            output.write(b"%")
            output.write(b"x" * (64 * 1024 * 1024))
            output.write(b"\nfof(signal_goal, conjecture, $true).\n")
    environment = starexec_environment(
        tptp_root=tptp_root,
        temporary_root=temporary_root,
        cpu_limit="0",
        memory_limit="4096",
    )
    process = subprocess.Popen(
        [
            str(install_root / "bin" / "starexec_run_default"),
            str(problem),
            str(output_dir),
        ],
        cwd=install_root / "bin",
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    expected_executable = (install_root / "bin" / "umlaut").resolve()
    executable_deadline = time.monotonic() + 2
    executable_seen = False
    while time.monotonic() < executable_deadline:
        if process.poll() is not None:
            break
        try:
            executable_seen = (
                Path(os.readlink(f"/proc/{process.pid}/exe")).resolve()
                == expected_executable
            )
        except FileNotFoundError:
            break
        if executable_seen:
            break
        time.sleep(0.005)
    if not executable_seen:
        stdout, stderr = process.communicate()
        raise AuditError(
            f"{signal_name} fixture did not exec Umlaut before exit: "
            f"{process.returncode}\n{stdout}\n{stderr}"
        )
    time.sleep(0.05)
    if process.poll() is not None:
        stdout, stderr = process.communicate()
        raise AuditError(
            f"{signal_name} fixture exited before signal: "
            f"{process.returncode}\n{stdout}\n{stderr}"
        )
    os.kill(process.pid, signal_number)
    try:
        stdout, stderr = process.communicate(timeout=5)
    except subprocess.TimeoutExpired as error:
        process.kill()
        process.communicate()
        raise AuditError(f"{signal_name} did not terminate Umlaut promptly") from error
    if process.returncode == 0 or "% SZS status ResourceOut" not in stdout:
        raise AuditError(
            f"{signal_name} produced an invalid termination result: "
            f"exit {process.returncode}\nstdout:\n{stdout}\nstderr:\n{stderr}"
        )
    if any(output_dir.iterdir()):
        raise AuditError(f"{signal_name} wrote unexpected StarExec output files")
    return {
        "signal": signal_name,
        "exit_status": process.returncode,
        "status": "ResourceOut",
        "termination_seconds_upper_bound": 5,
        "temporary_files_after_signal": sorted(
            path.relative_to(temporary_root).as_posix()
            for path in temporary_root.rglob("*")
        ),
    }


def audit_starexec_installation(
    runtime_path: Path,
    *,
    work_root: Path,
) -> dict[str, Any]:
    """Emulate public StarExec install/run behavior against the real package."""

    install_root = extract_runtime(runtime_path, work_root / "install")
    install_before = file_tree(install_root)
    wrapper = audit_wrapper_arguments(install_root, work_root=work_root / "wrapper")
    if file_tree(install_root) != install_before:
        raise AuditError("wrapper argument audit changed the restored installation")
    include_job = audit_include_job(install_root, work_root=work_root / "job")
    signals = [
        audit_external_signal(
            install_root,
            work_root=work_root / "signals",
            signal_number=signal.SIGALRM,
            signal_name="SIGALRM",
        ),
        audit_external_signal(
            install_root,
            work_root=work_root / "signals",
            signal_number=signal.SIGXCPU,
            signal_name="SIGXCPU",
        ),
    ]
    if file_tree(install_root) != install_before:
        raise AuditError("StarExec emulation modified the installation package")
    return {
        "install_root_entries": sorted(path.name for path in install_root.iterdir()),
        "wrapper": wrapper,
        "include_job": include_job,
        "signals": signals,
    }


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
        runtime_output = output_dir / f"{name}-{version}-starexec.tgz"
        copy_source_archive(crate_path, source_output)
        build_runtime_archive(
            runtime_output,
            binary=primary_binary,
            source_root=extracted_root,
        )
        runtime_archive_members = runtime_members(runtime_output)
        starexec_audit = audit_starexec_installation(
            runtime_output,
            work_root=temporary_root / "starexec-emulation",
        )

        manifest: dict[str, Any] = {
            "schema_version": 2,
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
            "delivery_contract": {
                "checked_utc_date": "2026-07-28",
                "latest_published_casc": "CASC-J13",
                "casc_2027_rules_published": False,
                "organizer_exemplar_obtained": False,
                "real_starexec_job_completed": False,
                "public_contract_emulated": True,
                "sources": [
                    "https://tptp.org/CASC/J13/Design.html",
                    "https://tptp.org/CASC/J13/Schedule.html",
                    "https://starexec.acorn.miami.edu/starexec/secure/add/solver.help",
                    "https://starexec.acorn.miami.edu/starexec/public/StarExecUserGuide.pdf",
                ],
            },
            "starexec_emulation": starexec_audit,
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
                "source_interfaces": ["native/cadical_ffi"],
                "internal_sat_fallback": True,
            },
            "checks": {
                "cargo_lock_dependency_free": True,
                "source_archive_forbidden_components_absent": True,
                "source_archive_pdfs_absent": True,
                "source_archive_cadical_shim_present": True,
                "extracted_source_release_build_offline": True,
                "runtime_archive_minimal": True,
                "runtime_archive_rootless": True,
                "runtime_optional_backends_absent": True,
                "starexec_wrapper_contract_emulated": True,
                "starexec_include_job_passed": True,
                "sigalrm_resource_out_passed": True,
                "sigxcpu_resource_out_passed": True,
                "normal_exit_files_absent": True,
            },
            "commands": {
                "cargo_package": package_result.stdout.splitlines(),
                "cargo_build": build_result.stdout.splitlines(),
                "rebuild": [
                    "python3 tools/packaging/verify_casc_package.py "
                    "--output-dir \"$OUTPUT_DIR\"",
                ],
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
