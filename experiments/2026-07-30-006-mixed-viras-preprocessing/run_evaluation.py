#!/usr/bin/env python3
"""Run the preregistered mixed-problem VIRAS preprocessing evaluation."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import re
import statistics
import subprocess
import time
import tomllib
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


EXPECTED_SCHEDULE_SHA256 = (
    "491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068"
)
EXPECTED_FAMILIES = {"ARI", "CSR", "DAT", "HWV", "ITP", "NUM", "SEV", "SWC", "SWW", "SYO"}
SZS_RE = re.compile(r"(?m)^%\s*SZS status\s+([A-Za-z]+)")
TPTP_STATUS_RE = re.compile(r"(?m)^%\s*Status\s*:\s*([A-Za-z]+)")
STATS_RE = re.compile(r"(?m)^%\s*VIRAS QE preprocessing:\s*(.+)$")
STAT_FIELD_RE = re.compile(r"([a-z_]+)=(\d+)")
FAMILY_RE = re.compile(r"^([A-Za-z]+)")
SOLVED_STATUSES = {"Theorem", "Unsatisfiable", "Satisfiable", "CounterSatisfiable"}
TIMING_LINE_RE = re.compile(r"(?i)\b(?:time|seconds?|cpu)\b")


class EvaluationError(RuntimeError):
    """Raised when a frozen evaluation or soundness gate is violated."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def percentile(values: Iterable[float], quantile: float) -> float | None:
    ordered = sorted(values)
    if not ordered:
        return None
    index = max(0, math.ceil(quantile * len(ordered)) - 1)
    return ordered[index]


def parse_szs(stdout: str) -> str | None:
    statuses = SZS_RE.findall(stdout)
    return statuses[-1] if statuses else None


def parse_preprocess_stats(stdout: str) -> dict[str, int] | None:
    matches = STATS_RE.findall(stdout)
    if not matches:
        return None
    if len(matches) != 1:
        raise EvaluationError("expected exactly one VIRAS preprocessing statistics line")
    fields = {name: int(value) for name, value in STAT_FIELD_RE.findall(matches[0])}
    required = {
        "formulas",
        "quantified",
        "imported",
        "checked",
        "applied",
        "unsupported",
        "resource_unknown",
        "fragment_unknown",
        "source_nodes",
        "result_nodes",
        "branch_proofs",
    }
    if fields.keys() != required:
        raise EvaluationError(
            f"VIRAS preprocessing field mismatch: {sorted(fields)} != {sorted(required)}"
        )
    if fields["checked"] != fields["applied"]:
        raise EvaluationError("an applied formula was not natively checked")
    if (
        fields["imported"]
        != fields["applied"]
        + fields["resource_unknown"]
        + fields["fragment_unknown"]
    ):
        raise EvaluationError("imported outcome accounting does not balance")
    if fields["quantified"] != fields["imported"] + fields["unsupported"]:
        raise EvaluationError("quantified outcome accounting does not balance")
    return fields


def normalized_deterministic_output(text: str) -> str:
    return "\n".join(
        line.rstrip()
        for line in text.splitlines()
        if not TIMING_LINE_RE.search(line)
    )


def run_process(command: list[str], timeout: float) -> dict[str, Any]:
    started = time.perf_counter_ns()
    try:
        completed = subprocess.run(
            command,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
        )
        timed_out = False
        returncode = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        returncode = None
        stdout = error.stdout or ""
        stderr = error.stderr or ""
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        if isinstance(stderr, bytes):
            stderr = stderr.decode(errors="replace")
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    return {
        "returncode": returncode,
        "timed_out": timed_out,
        "elapsed_ms": elapsed_ms,
        "status": parse_szs(stdout),
        "stdout_sha256": sha256_bytes(stdout.encode()),
        "stderr_sha256": sha256_bytes(stderr.encode()),
        "stderr": stderr[-2_000:],
        "_stdout": stdout,
    }


def prover_command(binary: Path, problem: Path, *, enabled: bool) -> list[str]:
    command = [
        str(binary),
        "--auto",
        "--cpu-limit=1",
        "--memory-limit=2048",
        "--tstp-format",
        "--output-level=4",
        "--proof-object=1",
    ]
    if enabled:
        command.append("--viras-qe-preprocess")
    command.append(str(problem))
    return command


def evaluate_problem(binary: Path, problem: Path, timeout: float) -> dict[str, Any]:
    source = problem.read_text(encoding="utf-8", errors="replace")
    family_match = FAMILY_RE.match(problem.name)
    if family_match is None:
        raise EvaluationError(f"problem has no family prefix: {problem.name}")
    status_match = TPTP_STATUS_RE.search(source)
    baseline = run_process(prover_command(binary, problem, enabled=False), timeout)
    enabled = run_process(prover_command(binary, problem, enabled=True), timeout)
    stats = parse_preprocess_stats(enabled["_stdout"])
    viras_inferences = enabled["_stdout"].count("inference(viras_qe")
    proof_publication_ok = stats is not None and (
        stats["applied"] == 0 or viras_inferences >= stats["applied"]
    )
    enabled["viras"] = stats
    enabled["viras_inferences"] = viras_inferences
    enabled["proof_publication_ok"] = proof_publication_ok
    return {
        "file": problem.name,
        "family": family_match.group(1),
        "tptp_status": status_match.group(1) if status_match else None,
        "baseline": baseline,
        "enabled": enabled,
    }


def solved(status: str | None) -> bool:
    return status in SOLVED_STATUSES


def latency_summary(values: list[float]) -> dict[str, float | None]:
    return {
        "median_ms": statistics.median(values) if values else None,
        "p95_ms": percentile(values, 0.95),
        "maximum_ms": max(values) if values else None,
    }


def aggregate_documents(documents: list[dict[str, Any]]) -> dict[str, Any]:
    baseline_statuses = Counter(document["baseline"]["status"] or "NoStatus" for document in documents)
    enabled_statuses = Counter(document["enabled"]["status"] or "NoStatus" for document in documents)
    common = baseline_only = enabled_only = changed_status = 0
    for document in documents:
        baseline_status = document["baseline"]["status"]
        enabled_status = document["enabled"]["status"]
        baseline_solved = solved(baseline_status)
        enabled_solved = solved(enabled_status)
        if baseline_solved and enabled_solved:
            common += 1
            if baseline_status != enabled_status:
                changed_status += 1
        elif baseline_solved:
            baseline_only += 1
        elif enabled_solved:
            enabled_only += 1

    stats_records = [
        document["enabled"]["viras"]
        for document in documents
        if document["enabled"]["viras"] is not None
    ]
    stat_totals = {
        name: sum(record[name] for record in stats_records)
        for name in (
            "formulas",
            "quantified",
            "imported",
            "checked",
            "applied",
            "unsupported",
            "resource_unknown",
            "fragment_unknown",
            "source_nodes",
            "result_nodes",
            "branch_proofs",
        )
    }
    growth_ratios = [
        record["result_nodes"] / record["source_nodes"]
        for record in stats_records
        if record["applied"] and record["source_nodes"]
    ]
    paired_latency_ratios = [
        document["enabled"]["elapsed_ms"] / document["baseline"]["elapsed_ms"]
        for document in documents
        if document["baseline"]["elapsed_ms"] > 0
    ]
    applied_documents = sum(
        record["applied"] > 0 for record in stats_records
    )
    proof_failures = [
        document["file"]
        for document in documents
        if document["enabled"]["viras"] is not None
        and document["enabled"]["viras"]["applied"] > 0
        and not document["enabled"]["proof_publication_ok"]
    ]
    return {
        "documents": len(documents),
        "families": sorted({document["family"] for document in documents}),
        "baseline_statuses": dict(sorted(baseline_statuses.items())),
        "enabled_statuses": dict(sorted(enabled_statuses.items())),
        "solve_delta": {
            "common_solved": common,
            "baseline_only": baseline_only,
            "enabled_only": enabled_only,
            "changed_solved_status": changed_status,
            "baseline_solved": common + baseline_only,
            "enabled_solved": common + enabled_only,
        },
        "coverage": {
            "documents_with_stats": len(stats_records),
            "documents_applied": applied_documents,
            **stat_totals,
        },
        "proof": {
            "attempted_publications": stat_totals["applied"],
            "native_checks": stat_totals["checked"],
            "failed_documents": proof_failures,
            "success_rate": (
                stat_totals["checked"] / stat_totals["applied"]
                if stat_totals["applied"]
                else None
            ),
        },
        "formula_growth": {
            "document_ratios": len(growth_ratios),
            "median_ratio": statistics.median(growth_ratios) if growth_ratios else None,
            "p95_ratio": percentile(growth_ratios, 0.95),
            "maximum_ratio": max(growth_ratios) if growth_ratios else None,
            "aggregate_ratio": (
                stat_totals["result_nodes"] / stat_totals["source_nodes"]
                if stat_totals["source_nodes"]
                else None
            ),
        },
        "latency": {
            "baseline": latency_summary(
                [document["baseline"]["elapsed_ms"] for document in documents]
            ),
            "enabled": latency_summary(
                [document["enabled"]["elapsed_ms"] for document in documents]
            ),
            "paired_ratio": {
                "median": statistics.median(paired_latency_ratios)
                if paired_latency_ratios
                else None,
                "p95": percentile(paired_latency_ratios, 0.95),
                "maximum": max(paired_latency_ratios) if paired_latency_ratios else None,
            },
        },
    }


def determinism_check(
    binary: Path,
    problem: Path,
    timeout: float,
) -> dict[str, Any]:
    first = run_process(prover_command(binary, problem, enabled=True), timeout)
    second = run_process(prover_command(binary, problem, enabled=True), timeout)
    first_normalized = normalized_deterministic_output(first["_stdout"])
    second_normalized = normalized_deterministic_output(second["_stdout"])
    return {
        "file": problem.name,
        "equal": (
            first["returncode"] == second["returncode"]
            and first["status"] == second["status"]
            and first["stderr"] == second["stderr"]
            and first_normalized == second_normalized
        ),
        "first_sha256": sha256_bytes(first_normalized.encode()),
        "second_sha256": sha256_bytes(second_normalized.encode()),
    }


def validate_report(report: dict[str, Any]) -> None:
    summary = report["summary"]
    if summary["documents"] != 100:
        raise EvaluationError(f"expected 100 held-out documents, got {summary['documents']}")
    if set(summary["families"]) != EXPECTED_FAMILIES:
        raise EvaluationError(f"held-out family mismatch: {summary['families']}")
    if report["schedule_sha256"] != EXPECTED_SCHEDULE_SHA256:
        raise EvaluationError("automatic schedule hash changed")
    if report["default_features"]:
        raise EvaluationError("default Cargo feature list is no longer empty")
    if summary["proof"]["failed_documents"]:
        raise EvaluationError(
            f"unchecked VIRAS publications: {summary['proof']['failed_documents']}"
        )
    if summary["proof"]["attempted_publications"] != summary["proof"]["native_checks"]:
        raise EvaluationError("native proof count differs from inserted result count")
    for document in report["documents"]:
        for arm in ("baseline", "enabled"):
            invocation = document[arm]
            if invocation["timed_out"]:
                raise EvaluationError(
                    f"external controller timeout for {document['file']} {arm}"
                )
            if invocation["returncode"] not in {0, 1, 8, 10}:
                raise EvaluationError(
                    f"unexpected prover exit {invocation['returncode']} for "
                    f"{document['file']} {arm}"
                )
        if (
            document["enabled"]["viras"] is None
            and document["enabled"]["returncode"] != 8
        ):
            raise EvaluationError(
                f"missing preprocessing record without CPU cutoff: {document['file']}"
            )
    if summary["proof"]["attempted_publications"] and len(report["determinism"]) != 2:
        raise EvaluationError("transformed and pass-through determinism cases are required")
    for check in report["determinism"]:
        if not check["equal"]:
            raise EvaluationError(f"nondeterministic opt-in output for {check['file']}")


def public_record(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: public_record(item)
            for key, item in value.items()
            if not key.startswith("_")
        }
    if isinstance(value, list):
        return [public_record(item) for item in value]
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--umlaut-binary", type=Path, required=True)
    parser.add_argument("--tfi-corpus", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--timeout", type=float, default=20.0)
    args = parser.parse_args()

    problems = sorted(args.tfi_corpus.glob("*.p"))
    if len(problems) != 100:
        raise EvaluationError(f"expected 100 TFI files, found {len(problems)}")
    if args.workers < 1:
        raise EvaluationError("worker count must be positive")
    os.environ["TPTP"] = str(args.tfi_corpus.resolve().parent)

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        documents = list(
            executor.map(
                lambda problem: evaluate_problem(
                    args.umlaut_binary.resolve(), problem.resolve(), args.timeout
                ),
                problems,
            )
        )

    by_family: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for document in documents:
        by_family[document["family"]].append(document)

    transformed = next(
        (
            args.tfi_corpus / document["file"]
            for document in documents
            if document["enabled"]["viras"] is not None
            and document["enabled"]["viras"]["applied"] > 0
        ),
        None,
    )
    passthrough = next(
        (
            args.tfi_corpus / document["file"]
            for document in documents
            if document["enabled"]["viras"] is not None
            and document["enabled"]["viras"]["applied"] == 0
        ),
        None,
    )
    determinism = []
    if transformed is not None:
        determinism.append(determinism_check(args.umlaut_binary, transformed, args.timeout))
    if passthrough is not None:
        determinism.append(determinism_check(args.umlaut_binary, passthrough, args.timeout))

    cargo = tomllib.loads((args.repo_root / "Cargo.toml").read_text(encoding="utf-8"))
    report = {
        "schema": "umlaut-mixed-viras-evaluation-v1",
        "command_policy": {
            "workers": args.workers,
            "timeout_seconds": args.timeout,
            "cpu_limit_seconds": 1,
            "memory_limit_mib": 2048,
            "output_level": 4,
            "proof_object": 1,
            "tptp_root": str(args.tfi_corpus.resolve().parent),
        },
        "schedule_sha256": sha256_file(
            args.repo_root / "src" / "heuristics" / "schedule.vars"
        ),
        "default_features": cargo["features"]["default"],
        "summary": aggregate_documents(documents),
        "families": {
            family: aggregate_documents(family_documents)
            for family, family_documents in sorted(by_family.items())
        },
        "determinism": determinism,
        "documents": documents,
    }
    validate_report(report)
    public = public_record(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(public, indent=2, sort_keys=True) + "\n").encode()
    args.output.write_bytes(encoded)
    print(f"wrote {args.output} sha256={sha256_bytes(encoded)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
