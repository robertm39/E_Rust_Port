#!/usr/bin/env python3
"""Build upstream E in WSL and compare it with the Rust port.

This program is intentionally standard-library-only.  It runs inside WSL; the
PowerShell wrapper is the supported Windows entry point.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import random
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterable, Sequence


SZS_RE = re.compile(r"\bSZS\s+status\s+([^\s]+)", re.IGNORECASE)
SZS_OUTPUT_START_RE = re.compile(r"\bSZS\s+output\s+start\s+([^\s]+)", re.IGNORECASE)
SZS_OUTPUT_END_RE = re.compile(r"\bSZS\s+output\s+end\s+([^\s]+)", re.IGNORECASE)
EXPECTED_RE = re.compile(r"^%\s*Status\s*:\s*([^\s]+)", re.MULTILINE | re.IGNORECASE)
SATURATION_GENERATED_ID_RE = re.compile(r"\bc_\d+_\d+\b")
VOLATILE_LINE_RE = re.compile(
    r"(?:User time|System time|Total time|Maximum resident|date|timestamp)\s*:",
    re.IGNORECASE,
)
PROBLEM_SUFFIXES = {".p", ".lop"}
DEFAULT_DISTRO = "Ubuntu-24.04"
REFERENCE_TOOL_BINARIES = {
    "CSSCPA_filter": "EXTERNAL/CSSCPA_filter",
    "checkproof": "PROVER/checkproof",
    "classify_problem": "PROVER/classify_problem",
    "direct_examples": "PROVER/direct_examples",
    "e_axfilter": "PROVER/e_axfilter",
    "e_client": "PROVER/e_client",
    "e_deduction_server": "PROVER/e_deduction_server",
    "e_ltb_runner": "PROVER/e_ltb_runner",
    "e_server": "PROVER/e_server",
    "e_stratpar": "PROVER/e_stratpar",
    "edpll": "PROVER/edpll",
    "eground": "PROVER/eground",
    "ekb_create": "PROVER/ekb_create",
    "ekb_delete": "PROVER/ekb_delete",
    "ekb_ginsert": "PROVER/ekb_ginsert",
    "ekb_insert": "PROVER/ekb_insert",
    "enormalizer": "PROVER/enormalizer",
    "epatternize": "PROVER/epatternize",
    "epclanalyse": "PROVER/epclanalyse",
    "epclextract": "PROVER/epclextract",
    "epcllemma": "PROVER/epcllemma",
    "ex_commandline": "SIMPLE_APPS/ex_commandline",
    "term2dag": "SIMPLE_APPS/term2dag",
}
DEFAULT_TOOL_ARGUMENT_CASES = (("--help",),)
VERSIONED_REFERENCE_TOOLS = frozenset(REFERENCE_TOOL_BINARIES) - {
    "ex_commandline",
    "term2dag",
}
TOOL_FUNCTIONAL_CASES = {
    "ex_commandline": (
        ("options-basic", ("--int_example=42", "--float_example", "one.p", "two.p"), None),
    ),
    "term2dag": (
        ("stdin-basic", (), "f(a,a) g(f(a,a))\n"),
    ),
}


class InteropError(RuntimeError):
    pass


def run_checked(
    command: Sequence[str],
    *,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
    capture: bool = True,
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        list(map(str, command)),
        cwd=cwd,
        env=env,
        text=True,
        errors="replace",
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout or "").strip()
        raise InteropError(
            f"Command failed ({result.returncode}): {' '.join(map(str, command))}"
            + (f"\n{detail}" if detail else "")
        )
    return result


def git(source: Path, *arguments: str) -> str:
    command = [
        "git",
        "-c",
        f"safe.directory={source}",
        # The checked-in upstream source lives on the Windows filesystem.  Windows
        # Git may materialize it with CRLF line endings, while the WSL build
        # driver runs Linux Git with LF defaults.  Use the Windows normalization
        # policy only when inspecting that original checkout; cloned build trees
        # on WSL/ext4 keep normal Linux line endings.
        "-c",
        "core.autocrlf=true",
        "-C",
        str(source),
        *arguments,
    ]
    return run_checked(command).stdout.strip()


def assert_original_clean(source: Path) -> None:
    status = git(source, "status", "--porcelain=v1", "--untracked-files=all")
    if status:
        raise InteropError(
            "The original eprover repository is not clean. Refusing to build because "
            "the reference source must remain untouched:\n" + status
        )


def cache_root() -> Path:
    return Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "e-rust-port"


def reference_manifest_path() -> Path:
    return cache_root() / "reference.json"


def safe_remove_cache_path(path: Path) -> None:
    cache = cache_root().resolve()
    resolved = path.resolve()
    if resolved == cache or cache not in resolved.parents:
        raise InteropError(f"Refusing to remove path outside the managed cache: {resolved}")
    shutil.rmtree(resolved)


def os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    release = Path("/etc/os-release")
    if release.exists():
        for line in release.read_text(encoding="utf-8").splitlines():
            if "=" in line:
                key, value = line.split("=", 1)
                values[key] = value.strip().strip('"')
    return values


def first_line(command: Sequence[str]) -> str:
    return run_checked(command).stdout.splitlines()[0].strip()


def build_one(source: Path, commit: str, mode: str) -> dict[str, Any]:
    build_dir = cache_root() / "sources" / commit / mode
    binary_name = "eprover-ho" if mode == "ho" else "eprover"
    installed_binary = cache_root() / "bin" / commit / mode / binary_name

    if build_dir.exists():
        safe_remove_cache_path(build_dir)
    build_dir.parent.mkdir(parents=True, exist_ok=True)
    run_checked(
        [
            "git",
            "-c",
            f"safe.directory={source}",
            "clone",
            "--no-hardlinks",
            "--no-checkout",
            str(source),
            str(build_dir),
        ],
        capture=False,
    )
    run_checked(["git", "-C", str(build_dir), "checkout", "--detach", commit])

    configure = ["./configure"] + (["--enable-ho"] if mode == "ho" else [])
    run_checked(configure, cwd=build_dir, capture=False)
    run_checked(["make", "commit_id"], cwd=build_dir, capture=False)
    jobs = str(max(1, os.cpu_count() or 1))
    run_checked(["make", "-j", jobs], cwd=build_dir, capture=False)

    built_binary = build_dir / "PROVER" / binary_name
    if not built_binary.is_file():
        raise InteropError(f"Expected reference binary was not built: {built_binary}")
    installed_binary.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(built_binary, installed_binary)
    installed_binary.chmod(installed_binary.stat().st_mode | 0o111)
    tools = copy_reference_tools(build_dir, commit) if mode == "fol" else {}

    version = run_checked([str(installed_binary), "--version"]).stdout.strip()
    help_result = subprocess.run(
        [str(installed_binary), "--help"],
        text=True,
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if help_result.returncode != 0 or not help_result.stdout.strip():
        raise InteropError(f"{binary_name} --help did not complete successfully")

    smoke_problem = (
        build_dir / "EXAMPLE_PROBLEMS" / "LFHOL" / "permute_func_no_axioms.p"
        if mode == "ho"
        else build_dir / "EXAMPLE_PROBLEMS" / "SMOKETEST" / "socrates.p"
    )
    smoke_env = os.environ.copy()
    smoke_env["TPTP"] = str(smoke_problem.parent)
    smoke = run_checked(
        [
            str(installed_binary),
            str(smoke_problem),
            "--auto",
            "--silent",
            "--cpu-limit=10",
        ],
        cwd=smoke_problem.parent,
        env=smoke_env,
    )
    smoke_status = szs_status(smoke.stdout)
    if smoke_status is None:
        raise InteropError(f"{binary_name} smoke test did not emit an SZS status")

    return {
        "mode": mode,
        "configure": configure[1:],
        "binary": str(installed_binary),
        "build_source": str(build_dir),
        "version": version,
        "smoke_status": smoke_status,
        "sha256": sha256_file(installed_binary),
        "tools": tools,
    }


def copy_reference_tools(build_dir: Path, commit: str) -> dict[str, str]:
    tools: dict[str, str] = {}
    for name, relative in sorted(REFERENCE_TOOL_BINARIES.items()):
        built_binary = build_dir / relative
        if not built_binary.is_file():
            raise InteropError(f"Expected reference tool was not built: {built_binary}")
        installed_binary = cache_root() / "bin" / commit / "tools" / name
        installed_binary.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(built_binary, installed_binary)
        installed_binary.chmod(installed_binary.stat().st_mode | 0o111)
        tools[name] = str(installed_binary)
    return tools


def build_reference(args: argparse.Namespace) -> None:
    repo_root = args.repo_root.resolve()
    source = repo_root / "eprover"
    if not (source / ".git").exists():
        raise InteropError(f"Expected the upstream repository at {source}")

    assert_original_clean(source)
    before_commit = git(source, "rev-parse", "HEAD")
    print(f"Building E reference commit {before_commit} in {cache_root()}", flush=True)
    builds = [build_one(source, before_commit, mode) for mode in ("fol", "ho")]
    assert_original_clean(source)
    after_commit = git(source, "rev-parse", "HEAD")
    if after_commit != before_commit:
        raise InteropError("The original eprover HEAD changed during the build")

    release = os_release()
    manifest = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "upstream_commit": before_commit,
        "upstream_source": str(source),
        "compiler": first_line(["gcc", "--version"]),
        "make": first_line(["make", "--version"]),
        "platform": platform.platform(),
        "distribution": {
            "id": release.get("ID", "unknown"),
            "version": release.get("VERSION_ID", "unknown"),
            "pretty_name": release.get("PRETTY_NAME", "unknown"),
        },
        "builds": {build["mode"]: build for build in builds},
    }
    manifest_path = reference_manifest_path()
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    write_json(manifest_path, manifest)
    print(f"Reference manifest: {manifest_path}")
    for build in builds:
        print(f"  {build['mode']}: {build['binary']}")


def load_manifest() -> dict[str, Any]:
    path = reference_manifest_path()
    if not path.is_file():
        raise InteropError(
            f"Reference manifest not found at {path}. Run build-reference first."
        )
    manifest = json.loads(path.read_text(encoding="utf-8"))
    for mode in ("fol", "ho"):
        binary = Path(manifest["builds"][mode]["binary"])
        if not binary.is_file():
            raise InteropError(f"Reference binary is missing: {binary}")
    for tool, binary_name in manifest["builds"]["fol"].get("tools", {}).items():
        binary = Path(binary_name)
        if not binary.is_file():
            raise InteropError(f"Reference tool {tool} is missing: {binary}")
    return manifest


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def wslpath(path: Path, windows: bool = False) -> str:
    option = "-w" if windows else "-a"
    return run_checked(["wslpath", option, str(path)]).stdout.strip()


def expected_status(text: str) -> str | None:
    match = EXPECTED_RE.search(text)
    return match.group(1) if match else None


def szs_status(text: str) -> str | None:
    matches = SZS_RE.findall(text)
    return matches[-1] if matches else None


def normalize_output(text: str, replacements: Iterable[tuple[str, str]] = ()) -> str:
    normalized = text.replace("\r\n", "\n")
    for old, new in replacements:
        if old:
            normalized = normalized.replace(old, new)
    lines = [line.rstrip() for line in normalized.splitlines() if not VOLATILE_LINE_RE.search(line)]
    lines = normalize_saturation_blocks(lines)
    return "\n".join(lines).strip()


def normalize_saturation_blocks(lines: Iterable[str]) -> list[str]:
    """Sort saturation listings while preserving proof order.

    E can emit the same saturated clause/formula set in a different order across
    two runs of the same binary.  That is not a semantic proof-output mismatch.
    Actual refutation/proof blocks remain order-sensitive because proof order is
    part of the derivation structure.
    """

    result: list[str] = []
    saturation_body: list[str] | None = None

    def flush_saturation_body() -> None:
        nonlocal saturation_body
        if saturation_body is not None:
            result.extend(sorted(saturation_body, key=lambda line: line.strip()))
            saturation_body = None

    for line in lines:
        start = SZS_OUTPUT_START_RE.search(line)
        if start and start.group(1).lower() == "saturation":
            flush_saturation_body()
            result.append(line)
            saturation_body = []
            continue

        if saturation_body is not None:
            end = SZS_OUTPUT_END_RE.search(line)
            if end:
                flush_saturation_body()
                result.append(line)
            else:
                saturation_body.append(normalize_saturation_line(line))
            continue

        result.append(line)

    flush_saturation_body()
    return result


def normalize_saturation_line(line: str) -> str:
    return SATURATION_GENERATED_ID_RE.sub("<CLAUSE_ID>", line)


def output_shape(stdout: str, stderr: str) -> dict[str, Any]:
    return {
        "szs_status_count": len(SZS_RE.findall(stdout)),
        "proof_start_count": len(re.findall(r"SZS output start", stdout, re.IGNORECASE)),
        "proof_end_count": len(re.findall(r"SZS output end", stdout, re.IGNORECASE)),
        "stdout_nonempty": bool(stdout.strip()),
        "stderr_nonempty": bool(stderr.strip()),
    }


def execute(
    executable: Path,
    arguments: Sequence[str],
    *,
    timeout: int,
    env: dict[str, str],
    stdin_text: str | None = None,
    cwd: Path | None = None,
) -> dict[str, Any]:
    command = [str(executable), *map(str, arguments)]
    started = time.perf_counter()
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            input=stdin_text,
            text=True,
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        timed_out = False
        exit_code: int | None = result.returncode
        stdout = result.stdout
        stderr = result.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        exit_code = None
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode("utf-8", "replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", "replace")
    elapsed = time.perf_counter() - started
    return {
        "command": command,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "wall_seconds": elapsed,
        "stdout": stdout,
        "stderr": stderr,
        "status": szs_status(stdout),
        "shape": output_shape(stdout, stderr),
    }


def enumerate_problems(repo_root: Path, corpus: Path | None) -> list[dict[str, Any]]:
    if corpus:
        roots = [(corpus, "ho" if "lfhol" in str(corpus).lower() else "fol")]
    else:
        examples = repo_root / "eprover" / "EXAMPLE_PROBLEMS"
        roots = [
            (examples / "SMOKETEST", "fol"),
            (examples / "TPTP", "fol"),
            (examples / "LFHOL", "ho"),
        ]

    cases: list[dict[str, Any]] = []
    seen: set[Path] = set()
    for root, mode in roots:
        if not root.is_dir():
            raise InteropError(f"Corpus directory does not exist: {root}")
        for problem in sorted(root.rglob("*"), key=lambda item: str(item).lower()):
            if not problem.is_file() or problem.suffix.lower() not in PROBLEM_SUFFIXES:
                continue
            if "axioms" in {part.lower() for part in problem.parts}:
                continue
            resolved = problem.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            text = problem.read_text(encoding="utf-8", errors="replace")
            problem_mode = (
                "ho"
                if mode == "ho"
                or "^" in problem.name
                or re.search(r"^\s*thf\s*\(", text, re.MULTILINE | re.IGNORECASE)
                else "fol"
            )
            cases.append(
                {
                    "name": str(problem.relative_to(root)),
                    "path": problem,
                    "mode": problem_mode,
                    "expected_status": expected_status(text),
                    "stdin": None,
                    "scenario": "file",
                }
            )
    if not cases:
        raise InteropError("The selected corpus does not contain any .p or .lop problems")
    return cases


def tptp_root(repo_root: Path, corpus: Path | None, problem: Path) -> Path:
    if corpus:
        return corpus
    bundled = repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "TPTP"
    return bundled if "TPTP" in problem.parts else problem.parent


def common_arguments(timeout: int, memory_limit_mb: int, proof: bool) -> list[str]:
    arguments = [
        "--auto",
        "--silent",
        f"--cpu-limit={timeout}",
        f"--memory-limit={memory_limit_mb}",
        "--detsort-rw",
        "--detsort-new",
    ]
    if proof:
        arguments.append("--proof-object=1")
    return arguments


def comparison_cases(
    repo_root: Path,
    corpus: Path | None,
    run_dir: Path,
) -> list[dict[str, Any]]:
    cases = enumerate_problems(repo_root, corpus)
    if corpus:
        return cases

    socrates = repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "SMOKETEST" / "socrates.p"
    if socrates.is_file():
        text = socrates.read_text(encoding="utf-8", errors="replace")
        cases.append(
            {
                "name": "synthetic/stdin-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": expected_status(text),
                "stdin": text,
                "scenario": "stdin",
            }
        )

    malformed = run_dir / "fixtures" / "malformed.p"
    malformed.parent.mkdir(parents=True, exist_ok=True)
    malformed.write_text("fof(broken, conjecture, (\n", encoding="utf-8")
    cases.append(
        {
            "name": "synthetic/malformed.p",
            "path": malformed,
            "mode": "fol",
            "expected_status": None,
            "stdin": None,
            "scenario": "malformed",
        }
    )

    harder = repo_root / "eprover" / "PROVER" / "LUSK6.lop"
    if harder.is_file():
        cases.append(
            {
                "name": "synthetic/cpu-limit-LUSK6.lop",
                "path": harder,
                "mode": "fol",
                "expected_status": None,
                "stdin": None,
                "scenario": "cpu-limit",
                "cpu_limit": 1,
            }
        )

    if socrates.is_file():
        cases.append(
            {
                "name": "synthetic/memory-limit-socrates.p",
                "path": socrates,
                "mode": "fol",
                "expected_status": expected_status(
                    socrates.read_text(encoding="utf-8", errors="replace")
                ),
                "stdin": None,
                "scenario": "memory-limit",
                "memory_limit_mb": 16,
            }
        )
    return cases


def comparison_mismatches(reference: dict[str, Any], candidate: dict[str, Any]) -> list[str]:
    mismatches: list[str] = []
    for field in ("exit_code", "timed_out", "status", "shape"):
        if reference[field] != candidate[field]:
            mismatches.append(field)
    return mismatches


def compare(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    output_root = repo_root / ".artifacts" / "e-compare"
    run_id = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f")
    run_dir = output_root / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    corpus = args.corpus.resolve() if args.corpus else None
    cases = comparison_cases(repo_root, corpus, run_dir)

    if args.self_test:
        candidate_kind = "linux-reference"
        rust_windows = None
    else:
        rust_windows = args.rust_windows.resolve()
        if not rust_windows.is_file() or rust_windows.suffix.lower() != ".exe":
            raise InteropError(f"Windows Rust executable not found: {rust_windows}")
        candidate_kind = "windows-rust"

    records: list[dict[str, Any]] = []
    mismatch_count = 0
    for index, case in enumerate(cases, 1):
        print(f"[{index}/{len(cases)}] {case['mode']} {case['name']}", flush=True)
        reference_binary = Path(manifest["builds"][case["mode"]]["binary"])
        problem: Path = case["path"]
        tptp = tptp_root(repo_root, corpus, problem)
        proof = case["scenario"] == "file"
        case_cpu_limit = case.get("cpu_limit", args.timeout)
        case_memory_limit = case.get("memory_limit_mb", args.memory_limit_mb)
        options = common_arguments(case_cpu_limit, case_memory_limit, proof)
        reference_args = options if case["stdin"] is not None else [str(problem), *options]
        reference_env = os.environ.copy()
        reference_env["TPTP"] = str(tptp)
        reference = execute(
            reference_binary,
            reference_args,
            timeout=args.timeout + 10,
            env=reference_env,
            stdin_text=case["stdin"],
            cwd=problem.parent,
        )

        if args.self_test:
            candidate_binary = reference_binary
            candidate_args = reference_args
            candidate_env = reference_env
            candidate_cwd = problem.parent
        else:
            assert rust_windows is not None
            candidate_binary = rust_windows
            windows_problem = wslpath(problem, windows=True)
            candidate_args = options if case["stdin"] is not None else [windows_problem, *options]
            candidate_env = os.environ.copy()
            candidate_env["TPTP"] = wslpath(tptp, windows=True)
            candidate_cwd = problem.parent

        candidate = execute(
            candidate_binary,
            candidate_args,
            timeout=args.timeout + 10,
            env=candidate_env,
            stdin_text=case["stdin"],
            cwd=candidate_cwd,
        )
        mismatches = comparison_mismatches(reference, candidate)

        replacements = [
            (str(problem), "<PROBLEM>"),
            (wslpath(problem, windows=True), "<PROBLEM>"),
            (str(tptp), "<TPTP>"),
            (wslpath(tptp, windows=True), "<TPTP>"),
        ]
        reference_normalized = normalize_output(reference["stdout"], replacements)
        candidate_normalized = normalize_output(candidate["stdout"], replacements)
        normalized_output_equal = reference_normalized == candidate_normalized
        if not normalized_output_equal and case["scenario"] != "cpu-limit":
            mismatches.append("normalized_stdout")

        if mismatches:
            mismatch_count += 1
            mismatch_dir = run_dir / "mismatches" / f"{index:04d}"
            mismatch_dir.mkdir(parents=True, exist_ok=True)
            for label, result in (("reference", reference), ("candidate", candidate)):
                (mismatch_dir / f"{label}.stdout").write_text(result["stdout"], encoding="utf-8")
                (mismatch_dir / f"{label}.stderr").write_text(result["stderr"], encoding="utf-8")
            (mismatch_dir / "reference.normalized").write_text(reference_normalized, encoding="utf-8")
            (mismatch_dir / "candidate.normalized").write_text(candidate_normalized, encoding="utf-8")

        records.append(
            {
                "name": case["name"],
                "scenario": case["scenario"],
                "mode": case["mode"],
                "expected_status": case["expected_status"],
                "reference_status": reference["status"],
                "candidate_status": candidate["status"],
                "reference_matches_expected": (
                    case["expected_status"] is None
                    or (reference["status"] or "").lower()
                    == case["expected_status"].lower()
                ),
                "candidate_matches_expected": (
                    case["expected_status"] is None
                    or (candidate["status"] or "").lower()
                    == case["expected_status"].lower()
                ),
                "reference_exit_code": reference["exit_code"],
                "candidate_exit_code": candidate["exit_code"],
                "reference_seconds": reference["wall_seconds"],
                "candidate_seconds": candidate["wall_seconds"],
                "normalized_output_equal": normalized_output_equal,
                "mismatches": mismatches,
            }
        )

    summary = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "candidate_kind": candidate_kind,
        "reference_manifest": manifest,
        "timeout_seconds": args.timeout,
        "memory_limit_mb": args.memory_limit_mb,
        "case_count": len(records),
        "mismatch_count": mismatch_count,
        "cases": records,
    }
    write_json(run_dir / "comparison.json", summary)
    write_csv(run_dir / "comparison.csv", records)
    print(f"Comparison report: {run_dir}")
    print(f"Cases: {len(records)}; mismatches: {mismatch_count}")
    if mismatch_count:
        raise InteropError("Compatibility mismatches were found")


def tool_comparison_cases(tool_names: Sequence[str]) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for tool in sorted(tool_names):
        for arguments in tool_argument_cases(tool):
            label = "-".join(part.strip("-") or "dash" for part in arguments)
            cases.append(
                {
                    "tool": tool,
                    "name": f"{tool}/{label}",
                    "arguments": list(arguments),
                    "scenario": label,
                    "stdin": None,
                }
            )
        for label, arguments, stdin_text in TOOL_FUNCTIONAL_CASES.get(tool, ()):
            cases.append(
                {
                    "tool": tool,
                    "name": f"{tool}/{label}",
                    "arguments": list(arguments),
                    "scenario": label,
                    "stdin": stdin_text,
                }
            )
    return cases


def tool_argument_cases(tool: str) -> tuple[tuple[str, ...], ...]:
    if tool in VERSIONED_REFERENCE_TOOLS:
        return (*DEFAULT_TOOL_ARGUMENT_CASES, ("--version",))
    return DEFAULT_TOOL_ARGUMENT_CASES


def compare_tools(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    reference_tools = manifest["builds"]["fol"].get("tools", {})
    if not reference_tools:
        raise InteropError(
            "Reference manifest has no support-tool binaries. Run build-reference again."
        )

    selected = args.tool or sorted(reference_tools)
    unknown = sorted(set(selected) - set(reference_tools))
    if unknown:
        raise InteropError("Unknown reference tool(s): " + ", ".join(unknown))

    output_root = repo_root / ".artifacts" / "e-compare"
    run_id = dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f") + "-tools"
    run_dir = output_root / run_id
    run_dir.mkdir(parents=True, exist_ok=False)

    if args.self_test:
        candidate_kind = "linux-reference-tools"
        rust_bin_dir = None
    else:
        rust_bin_dir = args.rust_windows_bin_dir.resolve()
        if not rust_bin_dir.is_dir():
            raise InteropError(f"Windows Rust bin directory not found: {rust_bin_dir}")
        candidate_kind = "windows-rust-tools"

    records: list[dict[str, Any]] = []
    mismatch_count = 0
    cases = tool_comparison_cases(selected)
    for index, case in enumerate(cases, 1):
        tool = case["tool"]
        print(f"[{index}/{len(cases)}] {case['name']}", flush=True)
        reference_binary = Path(reference_tools[tool])
        environment = os.environ.copy()
        reference = execute(
            reference_binary,
            case["arguments"],
            timeout=args.timeout,
            env=environment,
            stdin_text=case["stdin"],
            cwd=repo_root,
        )

        if args.self_test:
            candidate_binary = reference_binary
        else:
            assert rust_bin_dir is not None
            candidate_binary = rust_bin_dir / f"{tool}.exe"
            if not candidate_binary.is_file():
                raise InteropError(f"Windows Rust tool executable not found: {candidate_binary}")

        candidate = execute(
            candidate_binary,
            case["arguments"],
            timeout=args.timeout,
            env=environment,
            stdin_text=case["stdin"],
            cwd=repo_root,
        )
        mismatches = comparison_mismatches(reference, candidate)
        reference_normalized_stdout = normalize_output(reference["stdout"])
        candidate_normalized_stdout = normalize_output(candidate["stdout"])
        reference_normalized_stderr = normalize_output(reference["stderr"])
        candidate_normalized_stderr = normalize_output(candidate["stderr"])
        normalized_stdout_equal = reference_normalized_stdout == candidate_normalized_stdout
        normalized_stderr_equal = reference_normalized_stderr == candidate_normalized_stderr
        if not normalized_stdout_equal:
            mismatches.append("normalized_stdout")
        if not normalized_stderr_equal:
            mismatches.append("normalized_stderr")

        if mismatches:
            mismatch_count += 1
            mismatch_dir = run_dir / "mismatches" / f"{index:04d}"
            mismatch_dir.mkdir(parents=True, exist_ok=True)
            for label, result in (("reference", reference), ("candidate", candidate)):
                (mismatch_dir / f"{label}.stdout").write_text(result["stdout"], encoding="utf-8")
                (mismatch_dir / f"{label}.stderr").write_text(result["stderr"], encoding="utf-8")
            (mismatch_dir / "reference.normalized.stdout").write_text(
                reference_normalized_stdout, encoding="utf-8"
            )
            (mismatch_dir / "candidate.normalized.stdout").write_text(
                candidate_normalized_stdout, encoding="utf-8"
            )
            (mismatch_dir / "reference.normalized.stderr").write_text(
                reference_normalized_stderr, encoding="utf-8"
            )
            (mismatch_dir / "candidate.normalized.stderr").write_text(
                candidate_normalized_stderr, encoding="utf-8"
            )

        records.append(
            {
                "name": case["name"],
                "tool": tool,
                "scenario": case["scenario"],
                "arguments": case["arguments"],
                "stdin": case["stdin"] is not None,
                "reference_exit_code": reference["exit_code"],
                "candidate_exit_code": candidate["exit_code"],
                "reference_seconds": reference["wall_seconds"],
                "candidate_seconds": candidate["wall_seconds"],
                "normalized_stdout_equal": normalized_stdout_equal,
                "normalized_stderr_equal": normalized_stderr_equal,
                "mismatches": mismatches,
            }
        )

    summary = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "candidate_kind": candidate_kind,
        "reference_manifest": manifest,
        "timeout_seconds": args.timeout,
        "case_count": len(records),
        "mismatch_count": mismatch_count,
        "cases": records,
    }
    write_json(run_dir / "tool-comparison.json", summary)
    write_csv(run_dir / "tool-comparison.csv", records)
    print(f"Tool comparison report: {run_dir}")
    print(f"Cases: {len(records)}; mismatches: {mismatch_count}")
    if mismatch_count:
        raise InteropError("Support-tool compatibility mismatches were found")


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        return
    fields = list(rows[0].keys())
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            encoded = dict(row)
            for key, value in encoded.items():
                if isinstance(value, (list, dict)):
                    encoded[key] = json.dumps(value, sort_keys=True)
            writer.writerow(encoded)


def stage_benchmark_corpus(
    repo_root: Path, corpus: Path | None, commit: str
) -> tuple[Path, list[dict[str, Any]]]:
    destination = cache_root() / "benchmark-corpus" / commit
    if destination.exists():
        safe_remove_cache_path(destination)
    destination.mkdir(parents=True)
    if corpus:
        staged = destination / "custom"
        shutil.copytree(corpus, staged)
        cases = enumerate_problems(repo_root, staged)
    else:
        source = repo_root / "eprover" / "EXAMPLE_PROBLEMS"
        staged = destination / "EXAMPLE_PROBLEMS"
        shutil.copytree(source, staged)
        cases = enumerate_problems(repo_root, staged / "SMOKETEST")
    if str(staged.resolve()).startswith("/mnt/"):
        raise InteropError("Benchmark corpus must be staged on WSL's Linux filesystem")
    return staged, cases


def build_rust_linux(repo_root: Path, commit: str) -> tuple[Path, dict[str, str]]:
    if not (repo_root / "Cargo.toml").is_file():
        raise InteropError(
            "Cargo.toml does not exist yet. The benchmark command is ready, but it "
            "requires the Rust port with a binary target named 'eprover'."
        )
    if shutil.which("cargo") is None:
        raise InteropError(
            "cargo is not installed in WSL. Install Rust with rustup, then rerun benchmark."
        )
    target_dir = cache_root() / "rust-target" / commit
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(target_dir)
    run_checked(
        ["cargo", "build", "--locked", "--release", "--bin", "eprover"],
        cwd=repo_root,
        env=environment,
        capture=False,
    )
    binary = target_dir / "release" / "eprover"
    if not binary.is_file():
        raise InteropError(f"Cargo did not produce the required binary: {binary}")
    if str(binary.resolve()).startswith("/mnt/"):
        raise InteropError("Rust benchmark binary must reside on WSL's Linux filesystem")
    metadata = {
        "cargo": first_line(["cargo", "--version"]),
        "rustc": first_line(["rustc", "--version"]),
        "sha256": sha256_file(binary),
    }
    return binary, metadata


def timed_execution(
    executable: Path,
    arguments: Sequence[str],
    *,
    timeout: int,
    env: dict[str, str],
    cwd: Path,
) -> dict[str, Any]:
    with tempfile.NamedTemporaryFile(prefix="e-time-", delete=False) as handle:
        metrics_path = Path(handle.name)
    command = [
        "/usr/bin/time",
        "-f",
        "%U,%S,%M",
        "-o",
        str(metrics_path),
        str(executable),
        *map(str, arguments),
    ]
    started = time.perf_counter()
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        timed_out = False
        exit_code: int | None = result.returncode
    except subprocess.TimeoutExpired:
        timed_out = True
        exit_code = None
        result = None
    wall = time.perf_counter() - started
    user = system = max_rss = None
    if metrics_path.exists():
        metrics = metrics_path.read_text(encoding="utf-8", errors="replace").strip().splitlines()
        if metrics:
            values = metrics[-1].split(",")
            if len(values) == 3:
                try:
                    user, system, max_rss = float(values[0]), float(values[1]), int(values[2])
                except ValueError:
                    pass
        metrics_path.unlink(missing_ok=True)
    stdout = result.stdout if result else ""
    return {
        "exit_code": exit_code,
        "timed_out": timed_out,
        "wall_seconds": wall,
        "cpu_seconds": (user + system) if user is not None and system is not None else None,
        "max_rss_kb": max_rss,
        "status": szs_status(stdout),
    }


def geometric_mean(values: Sequence[float]) -> float | None:
    positive = [value for value in values if value > 0 and math.isfinite(value)]
    if not positive:
        return None
    return math.exp(sum(math.log(value) for value in positive) / len(positive))


def benchmark(args: argparse.Namespace) -> None:
    manifest = load_manifest()
    repo_root = args.repo_root.resolve()
    commit = manifest["upstream_commit"]
    rust_binary, rust_metadata = build_rust_linux(repo_root, commit)
    corpus = args.corpus.resolve() if args.corpus else None
    staged_root, cases = stage_benchmark_corpus(repo_root, corpus, commit)
    output_root = repo_root / ".artifacts" / "e-compare"
    run_dir = output_root / (
        dt.datetime.now().strftime("%Y%m%d-%H%M%S-%f") + "-benchmark"
    )
    run_dir.mkdir(parents=True, exist_ok=False)

    # Warm both binaries on the first problem. Warmup data is intentionally discarded.
    warm = cases[0]
    warm_tptp = staged_root if corpus else staged_root / "TPTP"
    warm_env = os.environ.copy()
    warm_env["TPTP"] = str(warm_tptp)
    options = common_arguments(args.timeout, args.memory_limit_mb, proof=False)
    for binary in (
        Path(manifest["builds"][warm["mode"]]["binary"]),
        rust_binary,
    ):
        timed_execution(
            binary,
            [str(warm["path"]), *options],
            timeout=args.timeout + 10,
            env=warm_env,
            cwd=warm["path"].parent,
        )

    rng = random.Random(0)
    samples: list[dict[str, Any]] = []
    for case_index, case in enumerate(cases, 1):
        print(f"[{case_index}/{len(cases)}] {case['name']}", flush=True)
        tptp = staged_root if corpus else staged_root / "TPTP"
        environment = os.environ.copy()
        environment["TPTP"] = str(tptp)
        binaries = {
            "c": Path(manifest["builds"][case["mode"]]["binary"]),
            "rust": rust_binary,
        }
        for iteration in range(args.runs):
            order = ["c", "rust"]
            rng.shuffle(order)
            for implementation in order:
                result = timed_execution(
                    binaries[implementation],
                    [str(case["path"]), *options],
                    timeout=args.timeout + 10,
                    env=environment,
                    cwd=case["path"].parent,
                )
                samples.append(
                    {
                        "name": case["name"],
                        "mode": case["mode"],
                        "iteration": iteration + 1,
                        "implementation": implementation,
                        **result,
                    }
                )

    summaries: list[dict[str, Any]] = []
    ratios: list[float] = []
    for case in cases:
        row: dict[str, Any] = {"name": case["name"], "mode": case["mode"]}
        grouped = {
            implementation: [
                sample
                for sample in samples
                if sample["name"] == case["name"]
                and sample["implementation"] == implementation
            ]
            for implementation in ("c", "rust")
        }
        for implementation, values in grouped.items():
            row[f"{implementation}_median_wall_seconds"] = statistics.median(
                value["wall_seconds"] for value in values
            )
            cpu_values = [value["cpu_seconds"] for value in values if value["cpu_seconds"] is not None]
            rss_values = [value["max_rss_kb"] for value in values if value["max_rss_kb"] is not None]
            row[f"{implementation}_median_cpu_seconds"] = (
                statistics.median(cpu_values) if cpu_values else None
            )
            row[f"{implementation}_max_rss_kb"] = max(rss_values) if rss_values else None
        c_wall = row["c_median_wall_seconds"]
        rust_wall = row["rust_median_wall_seconds"]
        c_outcomes = {(value["exit_code"], value["timed_out"], value["status"]) for value in grouped["c"]}
        rust_outcomes = {(value["exit_code"], value["timed_out"], value["status"]) for value in grouped["rust"]}
        behavior_matches = len(c_outcomes) == 1 and c_outcomes == rust_outcomes
        row["behavior_matches"] = behavior_matches
        ratio = rust_wall / c_wall if behavior_matches and c_wall > 0 else None
        row["rust_to_c_wall_ratio"] = ratio
        row["regression_over_threshold"] = bool(
            ratio is not None and ratio > args.regression_threshold
        )
        if ratio is not None:
            ratios.append(ratio)
        summaries.append(row)

    aggregate_ratio = geometric_mean(ratios)
    behavior_mismatch_count = sum(not row["behavior_matches"] for row in summaries)
    report = {
        "schema_version": 1,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "reference_manifest": manifest,
        "rust": rust_metadata,
        "runs": args.runs,
        "seed": 0,
        "timeout_seconds": args.timeout,
        "memory_limit_mb": args.memory_limit_mb,
        "regression_threshold": args.regression_threshold,
        "aggregate_rust_to_c_wall_ratio": aggregate_ratio,
        "behavior_mismatch_count": behavior_mismatch_count,
        "regression_over_threshold": bool(
            aggregate_ratio is not None and aggregate_ratio > args.regression_threshold
        ),
        "cases": summaries,
        "samples": samples,
    }
    write_json(run_dir / "benchmark.json", report)
    write_csv(run_dir / "benchmark.csv", summaries)
    write_csv(run_dir / "benchmark-samples.csv", samples)
    print(f"Benchmark report: {run_dir}")
    if aggregate_ratio is not None:
        print(f"Aggregate Rust/C wall-time ratio: {aggregate_ratio:.3f}x")
        if aggregate_ratio > args.regression_threshold:
            print(
                f"WARNING: ratio exceeds the {args.regression_threshold:.3f}x regression threshold",
                file=sys.stderr,
            )
    if behavior_mismatch_count:
        print(
            f"WARNING: {behavior_mismatch_count} benchmark cases had differing outcomes; "
            "their timing ratios were excluded",
            file=sys.stderr,
        )


def doctor(_: argparse.Namespace) -> None:
    missing = [
        tool
        for tool in ("gcc", "gawk", "git", "make", "python3", "/usr/bin/time")
        if shutil.which(tool) is None
    ]
    release = os_release()
    if missing:
        raise InteropError("Missing WSL dependencies: " + ", ".join(missing))
    print(f"Distribution: {release.get('PRETTY_NAME', 'unknown')}")
    if release.get("ID") != "ubuntu" or release.get("VERSION_ID") != "24.04":
        print(
            f"WARNING: tested baseline is {DEFAULT_DISTRO}; found "
            f"{release.get('ID', 'unknown')} {release.get('VERSION_ID', 'unknown')}",
            file=sys.stderr,
        )
    print(first_line(["gcc", "--version"]))
    print(first_line(["python3", "--version"]))


def path_argument(value: str) -> Path:
    return Path(value)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    subparsers = result.add_subparsers(dest="command", required=True)

    doctor_parser = subparsers.add_parser("doctor", help="validate WSL build dependencies")
    doctor_parser.set_defaults(function=doctor)

    build_parser = subparsers.add_parser("build-reference", help="build FOL and HO E references")
    build_parser.add_argument("--repo-root", type=path_argument, required=True)
    build_parser.set_defaults(function=build_reference)

    compare_parser = subparsers.add_parser("compare", help="compare WSL E with the Windows Rust EXE")
    compare_parser.add_argument("--repo-root", type=path_argument, required=True)
    candidate = compare_parser.add_mutually_exclusive_group(required=True)
    candidate.add_argument("--rust-windows", type=path_argument)
    candidate.add_argument("--self-test", action="store_true")
    compare_parser.add_argument("--corpus", type=path_argument)
    compare_parser.add_argument("--timeout", type=int, default=60)
    compare_parser.add_argument("--memory-limit-mb", type=int, default=2048)
    compare_parser.set_defaults(function=compare)

    compare_tools_parser = subparsers.add_parser(
        "compare-tools", help="compare WSL support tools with Windows Rust tool EXEs"
    )
    compare_tools_parser.add_argument("--repo-root", type=path_argument, required=True)
    tool_candidate = compare_tools_parser.add_mutually_exclusive_group(required=True)
    tool_candidate.add_argument("--rust-windows-bin-dir", type=path_argument)
    tool_candidate.add_argument("--self-test", action="store_true")
    compare_tools_parser.add_argument(
        "--tool",
        action="append",
        help="support tool to compare; may be repeated; defaults to all archived tools",
    )
    compare_tools_parser.add_argument("--timeout", type=int, default=30)
    compare_tools_parser.set_defaults(function=compare_tools)

    benchmark_parser = subparsers.add_parser("benchmark", help="benchmark native Linux C and Rust binaries")
    benchmark_parser.add_argument("--repo-root", type=path_argument, required=True)
    benchmark_parser.add_argument("--corpus", type=path_argument)
    benchmark_parser.add_argument("--runs", type=int, default=5)
    benchmark_parser.add_argument("--timeout", type=int, default=60)
    benchmark_parser.add_argument("--memory-limit-mb", type=int, default=2048)
    benchmark_parser.add_argument("--regression-threshold", type=float, default=1.10)
    benchmark_parser.set_defaults(function=benchmark)
    return result


def main(argv: Sequence[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        args.function(args)
        return 0
    except (InteropError, OSError, KeyError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
