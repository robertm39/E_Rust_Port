#!/usr/bin/env python3
"""Select training lemmas and prepare held-out treatment inputs on Linux."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import tarfile
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Sequence

from common import (
    ExperimentError,
    annotated_record,
    atomic_json,
    axiom_only_target,
    canonical_body,
    canonical_json,
    final_status,
    is_empty_clause,
    proof_step_count,
    read_jsonl,
    render_annotated,
    sha256_bytes,
    sha256_file,
    split_tptp_records,
    write_jsonl,
)


SOURCE_REVISION = "ce75ea3b68c34ab1640e0f362438a656626a5b0e"
CORPUS_SHA256 = "28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1"
SOURCE_ARCHIVE_SHA256 = (
    "8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156"
)
ARCHIVE_ROOT = "tsm-learning-018-81232361"
SOURCE_PROBLEMS = (
    "MGT067+1",
    "SWW967+1",
    "LAT265-2",
    "KLE145-10",
    "SYN563-10",
)
PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
SELECTION_SALT = "umlaut-proof-lemma-watchlist-transfer-v1"
MAX_SOURCE_LEMMAS = 8
MAX_POOL_CANDIDATES = 16
MAX_EXPLICIT_LEMMAS = 4
CONTROL_HEURISTIC = (
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),"
    "10*Refinedweight(PreferNonGoals,2,1,2,2,2),"
    "5*OrientLMaxWeight(ConstPrio,2,1,2,1,1),"
    "1*FIFOWeight(ConstPrio))"
)
VERIFY_ARGS = (
    f"--expert-heuristic={CONTROL_HEURISTIC}",
    "--term-ordering=KBO6",
    "--forward-demod-level=2",
    "--pcl-out",
    "--proof-object=1",
    "--force-deriv=2",
    "--soft-cpu-limit=1",
    "--cpu-limit=2",
    "--memory-limit=1536",
)


def utc_now() -> str:
    return datetime.now(UTC).isoformat(timespec="seconds")


def timed_run(
    command: Sequence[str],
    *,
    environment: dict[str, str],
    cwd: Path,
    timeout: int,
) -> dict[str, Any]:
    import resource

    usage_before = resource.getrusage(resource.RUSAGE_CHILDREN)
    started = time.monotonic()
    external_timeout = False
    try:
        completed = subprocess.run(
            command,
            cwd=cwd,
            env=environment,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
        return_code = completed.returncode
        stdout = completed.stdout
        stderr = completed.stderr
    except subprocess.TimeoutExpired as error:
        external_timeout = True
        return_code = None
        stdout = error.stdout or b""
        stderr = error.stderr or b""
    wall_seconds = time.monotonic() - started
    usage_after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu_seconds = (
        usage_after.ru_utime
        + usage_after.ru_stime
        - usage_before.ru_utime
        - usage_before.ru_stime
    )
    return {
        "return_code": return_code,
        "external_timeout": external_timeout,
        "stdout": stdout,
        "stderr": stderr,
        "wall_seconds": wall_seconds,
        "cpu_seconds": cpu_seconds,
    }


def load_corpus(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if sha256_file(path) != CORPUS_SHA256:
        raise ExperimentError("experiment 018 corpus hash mismatch")
    rows = read_jsonl(path)
    if not rows or rows[0].get("record_type") != "manifest":
        raise ExperimentError("invalid experiment 018 corpus")
    header, records = rows[0], rows[1:]
    if len(records) != 32:
        raise ExperimentError(f"expected 32 corpus records, found {len(records)}")
    family_sets = {
        split: {
            str(record["family"])
            for record in records
            if record["experiment_split"] == split
        }
        for split in ("train", "validation", "test")
    }
    for left, right in (
        ("train", "validation"),
        ("train", "test"),
        ("validation", "test"),
    ):
        if family_sets[left] & family_sets[right]:
            raise ExperimentError(f"{left}/{right} family leakage")
    return header, records


def verify_problem_tree(problem_root: Path, records: Sequence[dict[str, Any]]) -> None:
    for record in records:
        problem = problem_root / record["path"]
        if not problem.is_file() or sha256_file(problem) != record["sha256"]:
            raise ExperimentError(f"problem mismatch: {record['problem_id']}")
        for include in record["includes"]:
            include_path = problem_root / "problems" / "casc_2025" / include
            if not include_path.is_file():
                raise ExperimentError(f"missing included axiom file: {include_path}")


def archive_member_bytes(archive: tarfile.TarFile, name: str) -> bytes:
    member = archive.getmember(name)
    if not member.isfile():
        raise ExperimentError(f"archive member is not a file: {name}")
    stream = archive.extractfile(member)
    if stream is None:
        raise ExperimentError(f"cannot read archive member: {name}")
    return stream.read()


def extract_source_traces(
    archive_path: Path,
    output_root: Path,
    records: Sequence[dict[str, Any]],
) -> list[dict[str, Any]]:
    if sha256_file(archive_path) != SOURCE_ARCHIVE_SHA256:
        raise ExperimentError("experiment 018 source archive hash mismatch")
    by_id = {str(record["problem_id"]): record for record in records}
    selected: list[dict[str, Any]] = []
    with tarfile.open(archive_path, "r:gz") as archive:
        for problem_id in SOURCE_PROBLEMS:
            record = by_id.get(problem_id)
            if record is None or record["experiment_split"] != "train":
                raise ExperimentError(f"source problem is not frozen training: {problem_id}")
            base = (
                f"{ARCHIVE_ROOT}/runs/{record['category']}/{problem_id}"
            )
            result_data = archive_member_bytes(archive, f"{base}/result.json")
            trace_data = archive_member_bytes(archive, f"{base}/trace.pcl")
            result = json.loads(result_data)
            if (
                result.get("problem_id") != problem_id
                or result.get("problem_sha256") != record["sha256"]
                or result.get("szs_status") not in PROOF_STATUSES
                or result.get("proof") is not True
                or result.get("trace_sha256") != sha256_bytes(trace_data)
            ):
                raise ExperimentError(f"invalid archived source result: {problem_id}")
            trace_path = output_root / "source-traces" / f"{problem_id}.pcl"
            trace_path.parent.mkdir(parents=True, exist_ok=True)
            trace_path.write_bytes(trace_data)
            result_path = output_root / "source-traces" / f"{problem_id}.json"
            result_path.write_bytes(result_data.rstrip() + b"\n")
            selected.append(
                {
                    "problem_id": problem_id,
                    "category": record["category"],
                    "family": record["family"],
                    "problem_sha256": record["sha256"],
                    "trace_path": trace_path,
                    "trace_sha256": sha256_file(trace_path),
                    "archived_result_sha256": sha256_file(result_path),
                }
            )
    return selected


def select_candidates(
    sources: Sequence[dict[str, Any]],
    *,
    selector: Path,
    output_root: Path,
    environment: dict[str, str],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    candidates: list[dict[str, Any]] = []
    measurements: list[dict[str, Any]] = []
    for source in sources:
        problem_id = str(source["problem_id"])
        command = [
            str(selector),
            "--flat-lemmas",
            f"--max-lemmas={MAX_SOURCE_LEMMAS}",
            "--min-lemma-quality=0",
            "--tstp-out",
            "--output-level=1",
            str(source["trace_path"]),
        ]
        result = timed_run(
            command,
            cwd=output_root,
            environment=environment,
            timeout=120,
        )
        source_root = output_root / "selection" / problem_id
        source_root.mkdir(parents=True, exist_ok=True)
        stdout_path = source_root / "stdout.txt"
        stderr_path = source_root / "stderr.txt"
        stdout_path.write_bytes(result["stdout"])
        stderr_path.write_bytes(result["stderr"])
        if result["return_code"] != 0 or result["external_timeout"]:
            raise ExperimentError(f"lemma selector failed: {problem_id}")
        text = result["stdout"].decode("utf-8", errors="strict")
        selected_records = []
        for record in split_tptp_records(text):
            parsed = annotated_record(record)
            if parsed is None or parsed["role"] != "lemma":
                continue
            # Inline watchlists support represented clauses, not arbitrary FOF
            # formula owners. Successful saturation proofs are expected to
            # yield clause-valued selected steps.
            if parsed["kind"] not in {"cnf", "tcf"}:
                continue
            if is_empty_clause(parsed["body"]):
                continue
            selected_records.append(parsed)
        if len(selected_records) > MAX_SOURCE_LEMMAS:
            raise ExperimentError(
                f"selector exceeded frozen source cap for {problem_id}"
            )
        for selected_index, parsed in enumerate(selected_records):
            body = canonical_body(parsed["body"])
            candidate_id = hashlib.sha256(
                canonical_json(
                    {
                        "salt": SELECTION_SALT,
                        "source_problem": problem_id,
                        "selected_index": selected_index,
                        "kind": parsed["kind"],
                        "body": body,
                    }
                )
            ).hexdigest()
            candidates.append(
                {
                    "candidate_id": candidate_id,
                    "source_problem": problem_id,
                    "source_category": source["category"],
                    "source_family": source["family"],
                    "source_trace_sha256": source["trace_sha256"],
                    "selected_index": selected_index,
                    "kind": parsed["kind"],
                    "body": body,
                }
            )
        measurements.append(
            {
                "problem_id": problem_id,
                "command": command,
                "return_code": result["return_code"],
                "external_timeout": result["external_timeout"],
                "wall_seconds": result["wall_seconds"],
                "cpu_seconds": result["cpu_seconds"],
                "selected_clause_count": len(selected_records),
                "stdout_sha256": sha256_file(stdout_path),
                "stderr_sha256": sha256_file(stderr_path),
            }
        )
    if not candidates:
        raise ExperimentError("all source traces produced zero candidate clauses")
    return candidates, measurements


def pool_for_target(
    candidates: Sequence[dict[str, Any]],
    *,
    target_category: str,
    mode: str,
) -> list[dict[str, Any]]:
    if mode not in {"same", "cross"}:
        raise ExperimentError(f"invalid transfer mode: {mode}")
    eligible = [
        dict(candidate)
        for candidate in candidates
        if (
            candidate["source_category"] == target_category
            if mode == "same"
            else candidate["source_category"] != target_category
        )
    ]
    eligible.sort(
        key=lambda candidate: hashlib.sha256(
            canonical_json(
                {
                    "salt": SELECTION_SALT,
                    "mode": mode,
                    "source_problem": candidate["source_problem"],
                    "selected_index": candidate["selected_index"],
                    "body": candidate["body"],
                }
            )
        ).digest()
    )
    deduplicated: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()
    for candidate in eligible:
        key = (str(candidate["kind"]), str(candidate["body"]))
        if key in seen:
            continue
        seen.add(key)
        deduplicated.append(candidate)
        if len(deduplicated) == MAX_POOL_CANDIDATES:
            break
    return deduplicated


def candidate_conjecture(candidate: dict[str, Any], name: str) -> str:
    formula_kind = "tff" if candidate["kind"] == "tcf" else "fof"
    return f"{formula_kind}({name},conjecture,({candidate['body']}))."


def validate_candidates(
    *,
    target: dict[str, Any],
    mode: str,
    pool: Sequence[dict[str, Any]],
    problem_root: Path,
    prover: Path,
    output_root: Path,
    environment: dict[str, str],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    target_path = problem_root / target["path"]
    axiom_text = axiom_only_target(target_path.read_text(encoding="utf-8"))
    admitted: list[dict[str, Any]] = []
    attempts: list[dict[str, Any]] = []
    for attempt_index, candidate in enumerate(pool, start=1):
        if len(admitted) == MAX_EXPLICIT_LEMMAS:
            break
        attempt_root = (
            output_root
            / "admissibility"
            / target["experiment_split"]
            / target["problem_id"]
            / mode
            / f"attempt-{attempt_index:02d}"
        )
        attempt_root.mkdir(parents=True, exist_ok=True)
        check_name = f"candidate_check_{candidate['candidate_id'][:16]}"
        check_path = attempt_root / "problem.p"
        check_path.write_text(
            axiom_text + candidate_conjecture(candidate, check_name) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        command = [str(prover), *VERIFY_ARGS, str(check_path)]
        result = timed_run(
            command,
            cwd=attempt_root,
            environment=environment,
            timeout=12,
        )
        stdout_path = attempt_root / "stdout.pcl"
        stderr_path = attempt_root / "stderr.txt"
        stdout_path.write_bytes(result["stdout"])
        stderr_path.write_bytes(result["stderr"])
        stdout_text = result["stdout"].decode("utf-8", errors="replace")
        stderr_text = result["stderr"].decode("utf-8", errors="replace")
        status = final_status(stdout_text, stderr_text)
        proof_steps = proof_step_count(stdout_text)
        accepted = (
            result["return_code"] == 0
            and not result["external_timeout"]
            and status in PROOF_STATUSES
            and proof_steps > 0
        )
        attempt = {
            "attempt_index": attempt_index,
            "candidate_id": candidate["candidate_id"],
            "source_problem": candidate["source_problem"],
            "command": command,
            "problem_sha256": sha256_file(check_path),
            "return_code": result["return_code"],
            "external_timeout": result["external_timeout"],
            "szs_status": status,
            "accepted": accepted,
            "proof_steps": proof_steps,
            "cpu_seconds": result["cpu_seconds"],
            "wall_seconds": result["wall_seconds"],
            "stdout_sha256": sha256_file(stdout_path),
            "stderr_sha256": sha256_file(stderr_path),
        }
        atomic_json(attempt_root / "result.json", attempt)
        attempts.append(attempt)
        if accepted:
            admitted.append(dict(candidate))
    return admitted, {
        "attempt_count": len(attempts),
        "accepted_count": len(admitted),
        "rejected_count": len(attempts) - len(admitted),
        "cpu_seconds": sum(float(item["cpu_seconds"]) for item in attempts),
        "wall_seconds": sum(float(item["wall_seconds"]) for item in attempts),
        "attempts": attempts,
    }


def original_include_path(record: dict[str, Any]) -> str:
    prefix = "problems/casc_2025/"
    path = str(record["path"])
    if not path.startswith(prefix):
        raise ExperimentError(f"unexpected corpus problem path: {path}")
    return path[len(prefix) :]


def write_wrapper(
    path: Path,
    *,
    target: dict[str, Any],
    mode: str,
    mechanism: str,
    candidates: Sequence[dict[str, Any]],
) -> None:
    records = [
        f"% Generated {mechanism}/{mode} wrapper for {target['problem_id']}.",
        f"include('{original_include_path(target)}').",
    ]
    if mechanism == "watch":
        for index, candidate in enumerate(candidates, start=1):
            records.append(
                render_annotated(
                    kind=str(candidate["kind"]),
                    name=f"watch_{mode}_{index}_{candidate['candidate_id'][:12]}",
                    role="watchlist",
                    body=str(candidate["body"]),
                )
            )
        records.append(
            "cnf(watchlist_nontermination_sentinel,watchlist,$false)."
        )
    elif mechanism == "lemma":
        for index, candidate in enumerate(candidates, start=1):
            records.append(
                render_annotated(
                    kind=str(candidate["kind"]),
                    name=f"lemma_{mode}_{index}_{candidate['candidate_id'][:12]}",
                    role="lemma",
                    body=str(candidate["body"]),
                )
            )
    else:
        raise ExperimentError(f"invalid wrapper mechanism: {mechanism}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(records) + "\n", encoding="utf-8", newline="\n")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--source-archive", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--selector", type=Path, required=True)
    parser.add_argument("--prover", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--source-revision", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("candidate preparation may run only on Linux")
    if arguments.source_revision != SOURCE_REVISION:
        raise ExperimentError("source revision differs from preregistration")
    corpus = arguments.corpus.resolve()
    archive_path = arguments.source_archive.resolve()
    problem_root = arguments.problem_root.resolve()
    selector = arguments.selector.resolve()
    prover = arguments.prover.resolve()
    output_root = arguments.output_root.resolve()
    for executable in (selector, prover):
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ExperimentError(f"missing executable: {executable}")

    header, records = load_corpus(corpus)
    verify_problem_tree(problem_root, records)
    output_root.mkdir(parents=True, exist_ok=True)
    preregistration = Path(__file__).resolve().parent / "PREREGISTRATION.md"
    contract_body = {
        "schema_version": 1,
        "source_revision": SOURCE_REVISION,
        "corpus_sha256": sha256_file(corpus),
        "source_archive_sha256": sha256_file(archive_path),
        "selector_sha256": sha256_file(selector),
        "prover_sha256": sha256_file(prover),
        "prepare_sha256": sha256_file(Path(__file__).resolve()),
        "common_sha256": sha256_file(
            Path(__file__).resolve().parent / "common.py"
        ),
        "preregistration_sha256": sha256_file(preregistration),
        "source_problems": list(SOURCE_PROBLEMS),
        "selector_args": [
            "--flat-lemmas",
            f"--max-lemmas={MAX_SOURCE_LEMMAS}",
            "--min-lemma-quality=0",
            "--tstp-out",
            "--output-level=1",
        ],
        "max_pool_candidates": MAX_POOL_CANDIDATES,
        "max_explicit_lemmas": MAX_EXPLICIT_LEMMAS,
        "verify_args": list(VERIFY_ARGS),
    }
    contract_id = hashlib.sha256(canonical_json(contract_body)).hexdigest()
    contract_path = output_root / "preparation-contract.json"
    if contract_path.exists():
        existing = json.loads(contract_path.read_text(encoding="utf-8"))
        existing_body = {
            key: value
            for key, value in existing.items()
            if key not in {"contract_id", "created_at", "host"}
        }
        if existing_body != contract_body:
            raise ExperimentError("output contains an incompatible preparation contract")
    else:
        atomic_json(
            contract_path,
            {
                **contract_body,
                "contract_id": contract_id,
                "created_at": utc_now(),
                "host": {
                    "platform": platform.platform(),
                    "cpu_count": os.cpu_count(),
                },
            },
        )

    environment = os.environ.copy()
    environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
    sources = extract_source_traces(archive_path, output_root, records)
    candidates, selector_measurements = select_candidates(
        sources,
        selector=selector,
        output_root=output_root,
        environment=environment,
    )
    write_jsonl(output_root / "candidates.jsonl", candidates)
    atomic_json(
        output_root / "selection-summary.json",
        {
            "source_trace_count": len(sources),
            "candidate_count": len(candidates),
            "total_cpu_seconds": sum(
                float(item["cpu_seconds"]) for item in selector_measurements
            ),
            "total_wall_seconds": sum(
                float(item["wall_seconds"]) for item in selector_measurements
            ),
            "sources": selector_measurements,
        },
    )

    prepared_records: list[dict[str, Any]] = []
    for target in records:
        if target["experiment_split"] not in {"validation", "test"}:
            continue
        variants: dict[str, dict[str, Any]] = {
            "control": {
                "path": target["path"],
                "sha256": target["sha256"],
                "guidance_clause_count": 0,
                "added_clause_count": 0,
            }
        }
        for mode in ("same", "cross"):
            pool = pool_for_target(
                candidates,
                target_category=str(target["category"]),
                mode=mode,
            )
            admitted, validation = validate_candidates(
                target=target,
                mode=mode,
                pool=pool,
                problem_root=problem_root,
                prover=prover,
                output_root=output_root,
                environment=environment,
            )
            target_dir = (
                output_root
                / "prepared"
                / target["experiment_split"]
                / target["problem_id"]
            )
            watch_path = target_dir / f"watch-{mode}.p"
            lemma_path = target_dir / f"lemma-{mode}.p"
            write_wrapper(
                watch_path,
                target=target,
                mode=mode,
                mechanism="watch",
                candidates=pool,
            )
            write_wrapper(
                lemma_path,
                target=target,
                mode=mode,
                mechanism="lemma",
                candidates=admitted,
            )
            variants[f"watch_{mode}"] = {
                "path": watch_path.relative_to(output_root).as_posix(),
                "sha256": sha256_file(watch_path),
                "guidance_clause_count": len(pool),
                "added_clause_count": 0,
                "candidate_ids": [
                    candidate["candidate_id"] for candidate in pool
                ],
                "admissibility_cpu_seconds": 0.0,
            }
            variants[f"lemma_{mode}"] = {
                "path": lemma_path.relative_to(output_root).as_posix(),
                "sha256": sha256_file(lemma_path),
                "guidance_clause_count": 0,
                "added_clause_count": len(admitted),
                "candidate_ids": [
                    candidate["candidate_id"] for candidate in admitted
                ],
                "admissibility_cpu_seconds": validation["cpu_seconds"],
                "admissibility_wall_seconds": validation["wall_seconds"],
                "admissibility_attempt_count": validation["attempt_count"],
                "admissibility_rejected_count": validation["rejected_count"],
            }
        prepared = dict(target)
        prepared["target_sha256"] = target["sha256"]
        prepared["variants"] = variants
        prepared_records.append(prepared)

    prepared_records.sort(
        key=lambda record: (
            ("validation", "test").index(record["experiment_split"]),
            record["category"],
            record["problem_id"],
        )
    )
    manifest_header = {
        **header,
        "schema_version": 1,
        "kind": "umlaut-proof-lemma-watchlist-transfer",
        "problem_count": len(prepared_records),
        "source_revision": SOURCE_REVISION,
        "preparation_contract_id": contract_id,
        "preparation_contract_sha256": sha256_file(contract_path),
        "source_problem_ids": list(SOURCE_PROBLEMS),
        "source_families": sorted(
            {source["family"] for source in sources}
        ),
        "candidate_count": len(candidates),
        "split_counts": {
            split: sum(
                record["experiment_split"] == split
                for record in prepared_records
            )
            for split in ("validation", "test")
        },
    }
    manifest_path = output_root / "prepared-manifest.jsonl"
    write_jsonl(manifest_path, [manifest_header, *prepared_records])
    summary = {
        "contract_id": contract_id,
        "manifest": str(manifest_path),
        "manifest_sha256": sha256_file(manifest_path),
        "candidate_count": len(candidates),
        "source_trace_count": len(sources),
        "targets": len(prepared_records),
        "variant_clause_counts": {
            name: sum(
                int(record["variants"][name]["guidance_clause_count"])
                + int(record["variants"][name]["added_clause_count"])
                for record in prepared_records
            )
            for name in (
                "control",
                "watch_same",
                "lemma_same",
                "watch_cross",
                "lemma_cross",
            )
        },
    }
    atomic_json(output_root / "preparation-summary.json", summary)
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error

