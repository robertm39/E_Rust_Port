#!/usr/bin/env python3
"""Generate, measure, and profile deterministic TPTP frontend workloads."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import statistics
import subprocess
import sys
import tempfile
import time
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


DIALECTS = ("cnf", "fof", "tff", "thf")
SIZES = (1_000, 10_000, 50_000)
MODES = ("syntax", "cnf_no_preprocessing", "cnf")
TIME_FORMAT = "%e\\t%U\\t%S\\t%M\\t%x"
DHAT_TOTAL_FIELDS = {
    "total_bytes": "tb",
    "total_blocks": "tbk",
    "peak_live_bytes": "gb",
    "peak_live_blocks": "gbk",
    "end_live_bytes": "eb",
    "end_live_blocks": "ebk",
}
ANCESTRY_RE = re.compile(r"\binference\s*\(")
CNF_NAME_RE = re.compile(r"(?m)^\s*cnf\s*\(\s*([^,\s]+)")


class ExperimentError(RuntimeError):
    """A frozen experiment contract was violated."""


def canonical_json(value: Any) -> str:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    )


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def record_text(dialect: str, index: int) -> str:
    bucket = index % 257
    if dialect == "cnf":
        return (
            f"cnf(c{index},axiom,"
            f"(p{bucket}(a{bucket})|~q{bucket}(f(a{bucket}))"
            f"|r(f(a{bucket}),a{bucket}))).\n"
        )
    if dialect == "fof":
        return (
            f"fof(f{index},axiom,"
            f"(![X]:(p{bucket}(X)=>q{bucket}(f(X))))).\n"
        )
    if dialect == "tff":
        return (
            f"tff(t{index},axiom,"
            f"(![X:$i]:(p{bucket}(X)=>q{bucket}(f(X))))).\n"
        )
    if dialect == "thf":
        return (
            f"thf(h{index},axiom,"
            f"(![X:$i]:((p{bucket}@X)=>(q{bucket}@(f@X))))).\n"
        )
    raise ExperimentError(f"unknown dialect: {dialect}")


def prelude(dialect: str) -> str:
    if dialect not in {"tff", "thf"}:
        return ""
    arrow = ">" if dialect == "tff" else ">"
    rows = [f"{dialect}(f_type,type,f: $i {arrow} $i).\n"]
    for bucket in range(257):
        rows.append(
            f"{dialect}(p{bucket}_type,type,p{bucket}: $i {arrow} $o).\n"
        )
        rows.append(
            f"{dialect}(q{bucket}_type,type,q{bucket}: $i {arrow} $o).\n"
        )
    return "".join(rows)


def corpus_path(root: Path, dialect: str, size: int) -> Path:
    return root / f"{dialect}-{size:05d}.p"


def generate_corpus(root: Path) -> dict[str, Any]:
    root.mkdir(parents=True, exist_ok=True)
    files = []
    for dialect in DIALECTS:
        for size in SIZES:
            path = corpus_path(root, dialect, size)
            with path.open("w", encoding="utf-8", newline="\n") as stream:
                stream.write(prelude(dialect))
                for index in range(size):
                    stream.write(record_text(dialect, index))
            files.append(
                {
                    "dialect": dialect,
                    "records": size,
                    "path": path.name,
                    "bytes": path.stat().st_size,
                    "sha256": sha256_file(path),
                }
            )
    manifest: dict[str, Any] = {
        "schema_version": 1,
        "generator_sha256": sha256_file(Path(__file__)),
        "files": files,
    }
    manifest["manifest_id"] = sha256_bytes(
        canonical_json(manifest).encode("ascii")
    )
    (root / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return manifest


def load_manifest(root: Path) -> dict[str, Any]:
    manifest_path = root / "manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected = manifest.pop("manifest_id")
    actual = sha256_bytes(canonical_json(manifest).encode("ascii"))
    manifest["manifest_id"] = expected
    if actual != expected:
        raise ExperimentError("corpus manifest ID mismatch")
    for record in manifest["files"]:
        path = root / record["path"]
        if (
            path.stat().st_size != record["bytes"]
            or sha256_file(path) != record["sha256"]
        ):
            raise ExperimentError(f"corpus integrity failure: {path}")
    return manifest


def mode_arguments(mode: str, problem: Path | None) -> list[str]:
    if mode == "startup":
        return ["--version"]
    if problem is None:
        raise ExperimentError(f"{mode} needs a problem")
    common = ["--silent", "--output-file=/dev/null"]
    if mode == "syntax":
        return ["--syntax-only", *common, str(problem)]
    if mode == "cnf_no_preprocessing":
        return ["--cnf", "--no-preprocessing", *common, str(problem)]
    if mode == "cnf":
        return ["--cnf", *common, str(problem)]
    raise ExperimentError(f"unknown mode: {mode}")


def parse_time_record(text: str) -> dict[str, float | int]:
    rows = [row for row in text.splitlines() if row.strip()]
    if len(rows) != 1:
        raise ExperimentError(f"expected one GNU time row, got {len(rows)}")
    fields = rows[0].split("\t")
    if len(fields) != 5:
        raise ExperimentError(f"malformed GNU time row: {rows[0]!r}")
    return {
        "wall_seconds": float(fields[0]),
        "user_seconds": float(fields[1]),
        "system_seconds": float(fields[2]),
        "max_rss_kib": int(fields[3]),
        "exit_code": int(fields[4]),
    }


def run_timed(
    binary: Path,
    arguments: Sequence[str],
    *,
    output_root: Path,
    stem: str,
    timeout: float,
) -> dict[str, Any]:
    time_path = output_root / f"{stem}.time"
    stdout_path = output_root / f"{stem}.stdout"
    stderr_path = output_root / f"{stem}.stderr"
    command = [
        "/usr/bin/time",
        "-f",
        TIME_FORMAT,
        "-o",
        str(time_path),
        str(binary),
        *arguments,
    ]
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        completed = subprocess.run(
            command,
            check=False,
            stdout=stdout,
            stderr=stderr,
            timeout=timeout,
        )
    controller_wall = time.monotonic() - started
    timing = parse_time_record(time_path.read_text(encoding="utf-8"))
    if completed.returncode != 0 or timing["exit_code"] != 0:
        raise ExperimentError(
            f"{stem} failed: process={completed.returncode}, "
            f"time={timing['exit_code']}"
        )
    return {
        **timing,
        "controller_wall_seconds": controller_wall,
        "command": command,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }


def run_capture(
    binary: Path,
    arguments: Sequence[str],
    *,
    timeout: float,
) -> bytes:
    completed = subprocess.run(
        [str(binary), *arguments],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", errors="replace")[-1000:]
        raise ExperimentError(
            f"{binary.name} exited {completed.returncode}: {detail}"
        )
    return completed.stdout


def binary_for(
    implementation: str,
    dialect: str,
    rust_bin: Path,
    c_fol_bin: Path,
    c_ho_bin: Path,
) -> Path:
    if implementation == "rust":
        return rust_bin
    if implementation == "c":
        return c_ho_bin if dialect == "thf" else c_fol_bin
    raise ExperimentError(f"unknown implementation: {implementation}")


def collect_origin_gate(
    rust_bin: Path,
    corpus_root: Path,
    output_root: Path,
) -> list[dict[str, Any]]:
    records = []
    for dialect in DIALECTS:
        problem = corpus_path(corpus_root, dialect, 1_000)
        arguments = [
            "--cnf",
            "--tstp-out",
            "--output-level=4",
            str(problem),
        ]
        first = run_capture(rust_bin, arguments, timeout=300)
        second = run_capture(rust_bin, arguments, timeout=300)
        if first != second:
            raise ExperimentError(
                f"nondeterministic TSTP CNF output for {dialect}"
            )
        path = output_root / f"origin-{dialect}.out"
        path.write_bytes(first)
        text = first.decode("utf-8", errors="strict")
        names = CNF_NAME_RE.findall(text)
        records.append(
            {
                "dialect": dialect,
                "sha256": sha256_bytes(first),
                "bytes": len(first),
                "cnf_records": len(names),
                "unique_cnf_names": len(set(names)),
                "inference_records": len(ANCESTRY_RE.findall(text)),
            }
        )
    return records


def timing_command(arguments: argparse.Namespace) -> None:
    if sys.platform != "linux":
        raise ExperimentError("timing must run on Linux")
    corpus_root = arguments.corpus_root.resolve()
    output_root = arguments.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    manifest = load_manifest(corpus_root)
    binaries = {
        "rust": arguments.rust_bin.resolve(),
        "c_fol": arguments.c_fol_bin.resolve(),
        "c_ho": arguments.c_ho_bin.resolve(),
    }
    for name, binary in binaries.items():
        if not binary.is_file():
            raise ExperimentError(f"missing {name} binary: {binary}")

    metadata = {
        "schema_version": 1,
        "manifest_id": manifest["manifest_id"],
        "repetitions": arguments.repetitions,
        "binary_sha256": {
            name: sha256_file(path) for name, path in binaries.items()
        },
        "origin_gate": collect_origin_gate(
            binaries["rust"], corpus_root, output_root
        ),
    }
    (output_root / "metadata.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    output_path = output_root / "timing.jsonl"
    with output_path.open("w", encoding="utf-8", newline="\n") as stream:
        sample_index = 0
        for implementation in ("rust", "c"):
            dialect = "cnf"
            binary = binary_for(
                implementation,
                dialect,
                binaries["rust"],
                binaries["c_fol"],
                binaries["c_ho"],
            )
            run_capture(binary, ["--version"], timeout=30)
            for repetition in range(arguments.repetitions):
                stem = f"{sample_index:04d}-{implementation}-startup-{repetition}"
                record = run_timed(
                    binary,
                    mode_arguments("startup", None),
                    output_root=output_root,
                    stem=stem,
                    timeout=30,
                )
                record.update(
                    {
                        "implementation": implementation,
                        "dialect": "all",
                        "records": 0,
                        "mode": "startup",
                        "repetition": repetition,
                    }
                )
                stream.write(canonical_json(record) + "\n")
                stream.flush()
                sample_index += 1

        for size in SIZES:
            for dialect in DIALECTS:
                problem = corpus_path(corpus_root, dialect, size)
                for mode in MODES:
                    for repetition in range(arguments.repetitions):
                        for implementation in ("rust", "c"):
                            binary = binary_for(
                                implementation,
                                dialect,
                                binaries["rust"],
                                binaries["c_fol"],
                                binaries["c_ho"],
                            )
                            if repetition == 0:
                                run_capture(
                                    binary,
                                    mode_arguments(mode, problem),
                                    timeout=600,
                                )
                            stem = (
                                f"{sample_index:04d}-{implementation}-"
                                f"{dialect}-{size}-{mode}-{repetition}"
                            )
                            record = run_timed(
                                binary,
                                mode_arguments(mode, problem),
                                output_root=output_root,
                                stem=stem,
                                timeout=600,
                            )
                            record.update(
                                {
                                    "implementation": implementation,
                                    "dialect": dialect,
                                    "records": size,
                                    "mode": mode,
                                    "repetition": repetition,
                                    "problem_sha256": sha256_file(problem),
                                }
                            )
                            stream.write(canonical_json(record) + "\n")
                            stream.flush()
                            sample_index += 1
    print(output_path)


def median_metrics(records: Iterable[dict[str, Any]]) -> dict[str, float]:
    rows = list(records)
    if not rows:
        raise ExperimentError("cannot summarize an empty sample group")
    return {
        "wall_seconds": statistics.median(row["wall_seconds"] for row in rows),
        "cpu_seconds": statistics.median(
            row["user_seconds"] + row["system_seconds"] for row in rows
        ),
        "max_rss_kib": statistics.median(row["max_rss_kib"] for row in rows),
    }


def analyze_records(
    records: list[dict[str, Any]],
    *,
    expected_repetitions: int | None = None,
) -> dict[str, Any]:
    groups: dict[tuple[str, str, int, str], list[dict[str, Any]]] = defaultdict(
        list
    )
    for record in records:
        if record["exit_code"] != 0:
            raise ExperimentError("nonzero timed exit in input")
        key = (
            record["implementation"],
            record["dialect"],
            record["records"],
            record["mode"],
        )
        groups[key].append(record)
    if expected_repetitions is None:
        expected_repetitions = len(groups[("rust", "all", 0, "startup")])
    for key, rows in groups.items():
        if len(rows) != expected_repetitions:
            raise ExperimentError(
                f"incomplete group {key}: {len(rows)} != {expected_repetitions}"
            )

    startup = {
        implementation: median_metrics(
            groups[(implementation, "all", 0, "startup")]
        )
        for implementation in ("rust", "c")
    }
    strata = []
    dominant_counts: dict[str, int] = defaultdict(int)
    eligible_large: dict[str, list[tuple[str, float]]] = defaultdict(list)
    for implementation in ("rust", "c"):
        for dialect in DIALECTS:
            for size in SIZES:
                modes = {
                    mode: median_metrics(
                        groups[(implementation, dialect, size, mode)]
                    )
                    for mode in MODES
                }
                raw = {
                    "parse": modes["syntax"]["wall_seconds"]
                    - startup[implementation]["wall_seconds"],
                    "clausification": modes["cnf_no_preprocessing"][
                        "wall_seconds"
                    ]
                    - modes["syntax"]["wall_seconds"],
                    "preprocessing": modes["cnf"]["wall_seconds"]
                    - modes["cnf_no_preprocessing"]["wall_seconds"],
                }
                clamped = {name: max(0.0, value) for name, value in raw.items()}
                total = max(
                    0.0,
                    modes["cnf"]["wall_seconds"]
                    - startup[implementation]["wall_seconds"],
                )
                fractions = {
                    name: value / total if total else 0.0
                    for name, value in clamped.items()
                }
                dominant = max(fractions, key=fractions.get)
                if implementation == "rust":
                    if fractions[dominant] >= 0.5:
                        dominant_counts[dominant] += 1
                    if size == 50_000 and clamped[dominant] >= 0.025:
                        eligible_large[dominant].append(
                            (dialect, clamped[dominant])
                        )
                strata.append(
                    {
                        "implementation": implementation,
                        "dialect": dialect,
                        "records": size,
                        "modes": modes,
                        "phase_wall_seconds_raw": raw,
                        "phase_wall_seconds": clamped,
                        "phase_fractions": fractions,
                        "dominant_phase": dominant,
                    }
                )

    eligible = [
        phase
        for phase in ("parse", "clausification", "preprocessing")
        if dominant_counts[phase] >= 2 and eligible_large[phase]
    ]
    if eligible:
        selected_phase = max(
            eligible,
            key=lambda phase: max(value for _, value in eligible_large[phase]),
        )
        selected_dialect = max(
            eligible_large[selected_phase], key=lambda pair: pair[1]
        )[0]
    else:
        selected_phase = None
        selected_dialect = max(
            (
                row
                for row in strata
                if row["implementation"] == "rust"
                and row["records"] == 50_000
            ),
            key=lambda row: row["modes"]["cnf"]["wall_seconds"],
        )["dialect"]

    phase_mode = {
        "parse": "syntax",
        "clausification": "cnf_no_preprocessing",
        "preprocessing": "cnf",
        None: "cnf",
    }[selected_phase]
    result: dict[str, Any] = {
        "schema_version": 1,
        "startup": startup,
        "strata": strata,
        "selection": {
            "timing_gate_passed": selected_phase is not None,
            "dominant_counts": dict(dominant_counts),
            "selected_phase": selected_phase,
            "profile_dialect": selected_dialect,
            "profile_records": 10_000,
            "callgrind_mode": phase_mode,
        },
    }
    result["report_id"] = sha256_bytes(
        canonical_json(result).encode("ascii")
    )
    return result


def parse_dhat(path: Path) -> dict[str, int]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    allocation_points = payload.get("pps")
    if not isinstance(allocation_points, list) or not allocation_points:
        raise ExperimentError("DHAT output has no allocation points")
    result = {}
    for output_name, point_name in DHAT_TOTAL_FIELDS.items():
        if any(point_name not in point for point in allocation_points):
            raise ExperimentError(
                f"DHAT allocation point lacks field: {point_name}"
            )
        result[output_name] = sum(
            int(point[point_name]) for point in allocation_points
        )
    return result


def profile_command(arguments: argparse.Namespace) -> None:
    if sys.platform != "linux":
        raise ExperimentError("profiling must run on Linux")
    analysis = json.loads(arguments.analysis.read_text(encoding="utf-8"))
    selection = analysis["selection"]
    dialect = selection["profile_dialect"]
    size = int(selection["profile_records"])
    problem = corpus_path(arguments.corpus_root.resolve(), dialect, size)
    binary = arguments.rust_bin.resolve()
    output_root = arguments.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    records = []
    for mode in MODES:
        dhat_path = output_root / f"dhat-{mode}.json"
        stem = f"dhat-{mode}"
        profile_binary = Path("/usr/bin/valgrind")
        profile_args = [
            "--tool=dhat",
            f"--dhat-out-file={dhat_path}",
            str(binary),
            *mode_arguments(mode, problem),
        ]
        timing = run_timed(
            profile_binary,
            profile_args,
            output_root=output_root,
            stem=stem,
            timeout=1800,
        )
        records.append(
            {
                "mode": mode,
                "timing": timing,
                "dhat": parse_dhat(dhat_path),
                "dhat_sha256": sha256_file(dhat_path),
            }
        )

    callgrind_mode = selection["callgrind_mode"]
    callgrind_path = output_root / "callgrind.out"
    command = [
        "/usr/bin/valgrind",
        "--tool=callgrind",
        "--collect-jumps=yes",
        "--callgrind-out-file=" + str(callgrind_path),
        str(binary),
        *mode_arguments(callgrind_mode, problem),
    ]
    completed = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=3600,
    )
    (output_root / "callgrind.stdout").write_bytes(completed.stdout)
    (output_root / "callgrind.stderr").write_bytes(completed.stderr)
    if completed.returncode != 0:
        raise ExperimentError(
            f"Callgrind command exited {completed.returncode}"
        )
    annotated = subprocess.run(
        [
            "/usr/bin/callgrind_annotate",
            "--inclusive=yes",
            "--threshold=0.25",
            str(callgrind_path),
        ],
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=300,
    )
    if annotated.returncode != 0:
        raise ExperimentError(
            f"callgrind_annotate exited {annotated.returncode}"
        )
    annotate_path = output_root / "callgrind-annotate.txt"
    annotate_path.write_bytes(annotated.stdout)
    report = {
        "schema_version": 1,
        "dialect": dialect,
        "records": size,
        "selected_phase": selection["selected_phase"],
        "callgrind_mode": callgrind_mode,
        "binary_sha256": sha256_file(binary),
        "problem_sha256": sha256_file(problem),
        "dhat": records,
        "callgrind": {
            "command": command,
            "raw_sha256": sha256_file(callgrind_path),
            "annotate_sha256": sha256_file(annotate_path),
        },
    }
    report["report_id"] = sha256_bytes(
        canonical_json(report).encode("ascii")
    )
    (output_root / "profile.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, indent=2, sort_keys=True))


def exact_cnf_output(binary: Path, problem: Path) -> bytes:
    return run_capture(
        binary,
        ["--cnf", "--tstp-out", "--output-level=4", str(problem)],
        timeout=600,
    )


def percentage_improvement(baseline: float, candidate: float) -> float:
    if baseline <= 0.0:
        raise ExperimentError("baseline metric must be positive")
    return (baseline - candidate) / baseline * 100.0


def candidate_command(arguments: argparse.Namespace) -> None:
    if sys.platform != "linux":
        raise ExperimentError("candidate comparison must run on Linux")
    corpus_root = arguments.corpus_root.resolve()
    output_root = arguments.output_root.resolve()
    baseline_bin = arguments.baseline_bin.resolve()
    candidate_bin = arguments.candidate_bin.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    manifest = load_manifest(corpus_root)
    for binary in (baseline_bin, candidate_bin):
        if not binary.is_file():
            raise ExperimentError(f"missing comparison binary: {binary}")

    origins = []
    for size in (1_000, 10_000):
        for dialect in DIALECTS:
            problem = corpus_path(corpus_root, dialect, size)
            baseline = exact_cnf_output(baseline_bin, problem)
            candidate = exact_cnf_output(candidate_bin, problem)
            baseline_path = output_root / f"origin-baseline-{dialect}-{size}.out"
            candidate_path = output_root / f"origin-candidate-{dialect}-{size}.out"
            baseline_path.write_bytes(baseline)
            candidate_path.write_bytes(candidate)
            exact = baseline == candidate
            if not exact:
                raise ExperimentError(
                    f"candidate TSTP output differs for {dialect}/{size}"
                )
            text = candidate.decode("utf-8", errors="strict")
            origins.append(
                {
                    "dialect": dialect,
                    "records": size,
                    "exact": exact,
                    "sha256": sha256_bytes(candidate),
                    "bytes": len(candidate),
                    "cnf_records": len(CNF_NAME_RE.findall(text)),
                    "inference_records": len(ANCESTRY_RE.findall(text)),
                }
            )

    rows = []
    for size in SIZES:
        for dialect in DIALECTS:
            problem = corpus_path(corpus_root, dialect, size)
            for mode in MODES:
                for repetition in range(arguments.repetitions):
                    for implementation, binary in (
                        ("baseline", baseline_bin),
                        ("candidate", candidate_bin),
                    ):
                        stem = (
                            f"{implementation}-{dialect}-{size}-"
                            f"{mode}-{repetition}"
                        )
                        record = run_timed(
                            binary,
                            mode_arguments(mode, problem),
                            output_root=output_root,
                            stem=stem,
                            timeout=600,
                        )
                        record.update(
                            {
                                "implementation": implementation,
                                "dialect": dialect,
                                "records": size,
                                "mode": mode,
                                "repetition": repetition,
                            }
                        )
                        rows.append(record)
    timing_path = output_root / "candidate-timing.jsonl"
    timing_path.write_text(
        "".join(canonical_json(row) + "\n" for row in rows),
        encoding="utf-8",
    )

    grouped: dict[tuple[str, str, int, str], list[dict[str, Any]]] = defaultdict(
        list
    )
    for row in rows:
        grouped[
            (
                row["implementation"],
                row["dialect"],
                row["records"],
                row["mode"],
            )
        ].append(row)
    medians = []
    for key, samples in sorted(grouped.items()):
        if len(samples) != arguments.repetitions:
            raise ExperimentError(f"incomplete candidate timing group: {key}")
        implementation, dialect, size, mode = key
        medians.append(
            {
                "implementation": implementation,
                "dialect": dialect,
                "records": size,
                "mode": mode,
                **median_metrics(samples),
            }
        )

    heldout = {}
    for mode in ("syntax", "cnf"):
        values = {}
        for implementation in ("baseline", "candidate"):
            values[implementation] = next(
                row
                for row in medians
                if row["implementation"] == implementation
                and row["dialect"] == "thf"
                and row["records"] == 10_000
                and row["mode"] == mode
            )
        heldout[mode] = {
            **values,
            "wall_improvement_percent": percentage_improvement(
                values["baseline"]["wall_seconds"],
                values["candidate"]["wall_seconds"],
            ),
        }

    candidate_dhat_path = output_root / "candidate-syntax.dhat.json"
    dhat_timing = run_timed(
        Path("/usr/bin/valgrind"),
        [
            "--tool=dhat",
            f"--dhat-out-file={candidate_dhat_path}",
            str(candidate_bin),
            *mode_arguments(
                "syntax", corpus_path(corpus_root, "thf", 10_000)
            ),
        ],
        output_root=output_root,
        stem="candidate-syntax-dhat",
        timeout=1800,
    )
    candidate_dhat = parse_dhat(candidate_dhat_path)
    baseline_profile = json.loads(
        arguments.baseline_profile.read_text(encoding="utf-8")
    )
    baseline_dhat = next(
        record["dhat"]
        for record in baseline_profile["dhat"]
        if record["mode"] == "syntax"
    )
    allocation = {
        "baseline": baseline_dhat,
        "candidate": candidate_dhat,
        "total_bytes_improvement_percent": percentage_improvement(
            baseline_dhat["total_bytes"], candidate_dhat["total_bytes"]
        ),
        "peak_live_bytes_improvement_percent": percentage_improvement(
            baseline_dhat["peak_live_bytes"],
            candidate_dhat["peak_live_bytes"],
        ),
    }
    report = {
        "schema_version": 1,
        "manifest_id": manifest["manifest_id"],
        "repetitions": arguments.repetitions,
        "binary_sha256": {
            "baseline": sha256_file(baseline_bin),
            "candidate": sha256_file(candidate_bin),
        },
        "origins": origins,
        "timing_medians": medians,
        "heldout": heldout,
        "allocation": allocation,
        "candidate_dhat_timing": dhat_timing,
        "all_exit_codes_zero": all(row["exit_code"] == 0 for row in rows),
    }
    report["report_id"] = sha256_bytes(
        canonical_json(report).encode("ascii")
    )
    (output_root / "candidate.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(report, indent=2, sort_keys=True))


def analyze_command(arguments: argparse.Namespace) -> None:
    records = [
        json.loads(line)
        for line in arguments.timing.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    result = analyze_records(records, expected_repetitions=arguments.repetitions)
    arguments.output.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(result, indent=2, sort_keys=True))


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    generate = subparsers.add_parser("generate")
    generate.add_argument("--corpus-root", type=Path, required=True)
    generate.set_defaults(function=lambda args: print(
        json.dumps(generate_corpus(args.corpus_root), indent=2, sort_keys=True)
    ))

    timing = subparsers.add_parser("timing")
    timing.add_argument("--corpus-root", type=Path, required=True)
    timing.add_argument("--rust-bin", type=Path, required=True)
    timing.add_argument("--c-fol-bin", type=Path, required=True)
    timing.add_argument("--c-ho-bin", type=Path, required=True)
    timing.add_argument("--output-root", type=Path, required=True)
    timing.add_argument("--repetitions", type=int, default=5)
    timing.set_defaults(function=timing_command)

    analyze = subparsers.add_parser("analyze")
    analyze.add_argument("--timing", type=Path, required=True)
    analyze.add_argument("--output", type=Path, required=True)
    analyze.add_argument("--repetitions", type=int, default=5)
    analyze.set_defaults(function=analyze_command)

    profile = subparsers.add_parser("profile")
    profile.add_argument("--analysis", type=Path, required=True)
    profile.add_argument("--corpus-root", type=Path, required=True)
    profile.add_argument("--rust-bin", type=Path, required=True)
    profile.add_argument("--output-root", type=Path, required=True)
    profile.set_defaults(function=profile_command)

    candidate = subparsers.add_parser("candidate")
    candidate.add_argument("--corpus-root", type=Path, required=True)
    candidate.add_argument("--baseline-bin", type=Path, required=True)
    candidate.add_argument("--candidate-bin", type=Path, required=True)
    candidate.add_argument("--baseline-profile", type=Path, required=True)
    candidate.add_argument("--output-root", type=Path, required=True)
    candidate.add_argument("--repetitions", type=int, default=5)
    candidate.set_defaults(function=candidate_command)
    return parser


def main() -> None:
    arguments = make_parser().parse_args()
    if getattr(arguments, "repetitions", 1) < 1:
        raise ExperimentError("repetitions must be positive")
    arguments.function(arguments)


if __name__ == "__main__":
    try:
        main()
    except (
        ExperimentError,
        OSError,
        subprocess.SubprocessError,
        ValueError,
    ) as error:
        print(f"experiment error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
