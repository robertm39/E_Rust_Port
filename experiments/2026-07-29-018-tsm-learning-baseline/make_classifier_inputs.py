#!/usr/bin/env python3
"""Build control-only held-out label KBs and TSM classifier inputs."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence


PROOF_STATUSES = {"Theorem", "Unsatisfiable", "ContradictoryAxioms"}
ANNOTATION_RE = re.compile(r"(\d+):\(([^)]*)\)")
LABEL_EXTRACTION_REVISION = "477fa727355bace7de39d043d9b18734bd16adf4"


class ExperimentError(RuntimeError):
    """Raised when held-out labels cannot be built without leakage."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def run(
    command: Sequence[str],
    *,
    cwd: Path,
    environment: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        command,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=environment,
        timeout=120,
        check=False,
    )


def final_status(output: bytes) -> str | None:
    matches = re.findall(
        r"(?:%|#) SZS status ([A-Za-z]+)",
        output.decode("utf-8", errors="replace"),
    )
    return matches[-1] if matches else None


def pcl_label_command(
    result: dict[str, Any], telemetry_path: Path
) -> list[str]:
    original = result.get("command")
    if not isinstance(original, list) or not all(
        isinstance(argument, str) for argument in original
    ):
        raise ExperimentError("control result has no reusable command")
    if original.count("--tstp-out") != 1 or "--pcl-out" in original:
        raise ExperimentError("control result does not use the frozen TSTP output")

    command = []
    for argument in original:
        if argument == "--tstp-out":
            command.append("--pcl-out")
        elif argument.startswith("--search-telemetry="):
            command.append(f"--search-telemetry={telemetry_path}")
        else:
            command.append(argument)
    return command


def label_extraction_binary(search_root: Path) -> Path:
    result_paths = sorted(
        (search_root / "validation" / "runs" / "heldout" / "control").glob(
            "*/*/rep-1/result.json"
        )
    )
    if not result_paths:
        raise ExperimentError("no validation control result for binary identity")
    result = json.loads(result_paths[0].read_text(encoding="utf-8"))
    command = pcl_label_command(
        result, Path("binary-identity-telemetry.json")
    )
    return Path(command[0]).resolve()


def annotation_entries(path: Path) -> list[dict[str, Any]]:
    entries: list[dict[str, Any]] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("%"):
            continue
        if " : " not in line or not line.endswith("."):
            raise ExperimentError(f"unrecognized clausepattern line: {line}")
        term, annotations = line.rsplit(" : ", 1)
        count = 0.0
        proof_count = 0.0
        matches = list(ANNOTATION_RE.finditer(annotations))
        if not matches:
            raise ExperimentError(f"missing annotation: {line}")
        for match in matches:
            values = [float(value) for value in match.group(2).split(",")]
            if len(values) != 7:
                raise ExperimentError(f"expected seven KB values: {line}")
            count += values[0]
            proof_count += values[1]
        if count <= 0.0:
            raise ExperimentError(f"nonpositive source count: {line}")
        label = 1.0 if proof_count / count >= 0.5 else -1.0
        entries.append(
            {
                "term": term,
                "sources": count,
                "proof_sources": proof_count,
                "proof_rate": proof_count / count,
                "label": label,
            }
        )
    if not entries:
        raise ExperimentError(f"no clause patterns in {path}")
    return entries


def classifier_text(
    training: Sequence[dict[str, Any]], test: Sequence[dict[str, Any]]
) -> str:
    lines = ["Training:"]
    lines.extend(
        f"{entry['term']} : 1:({entry['sources']:.6f},{entry['label']:.1f})."
        for entry in training
    )
    lines.extend((".", "Test:"))
    lines.extend(
        f"{entry['term']} : 1:({entry['sources']:.6f},{entry['label']:.1f})."
        for entry in test
    )
    lines.append(".")
    return "\n".join(lines) + "\n"


def create_label_kb(
    *,
    split: str,
    search_root: Path,
    output_root: Path,
    kb_create: Path,
    kb_ginsert: Path,
    problem_root: Path,
) -> tuple[Path, list[str]]:
    kb_name = f"{split.upper()}_CONTROL_KB"
    kb_path = output_root / kb_name
    if kb_path.exists():
        raise ExperimentError(f"held-out label KB already exists: {kb_path}")
    created = run([str(kb_create), kb_name], cwd=output_root)
    (output_root / f"{split}-kb-create.stdout").write_bytes(created.stdout)
    (output_root / f"{split}-kb-create.stderr").write_bytes(created.stderr)
    if created.returncode != 0:
        raise ExperimentError(f"KB creation failed for {split}")

    inserted: list[str] = []
    result_paths = sorted(
        (search_root / split / "runs" / "heldout" / "control").glob(
            "*/*/rep-1/result.json"
        )
    )
    if not result_paths:
        raise ExperimentError(f"no control results for {split}")
    for result_path in result_paths:
        result = json.loads(result_path.read_text(encoding="utf-8"))
        if result.get("strategy") != "control":
            raise ExperimentError(f"non-control label source: {result_path}")
        if result.get("repetition") != 1:
            raise ExperimentError(f"non-frozen label repetition: {result_path}")
        if result.get("szs_status") not in PROOF_STATUSES:
            continue
        trace_output = (
            output_root
            / "traces"
            / result_path.relative_to(search_root).parent
        )
        trace_output.mkdir(parents=True, exist_ok=False)
        proof_path = trace_output / "classifier-trace.pcl"
        trace_stderr_path = trace_output / "classifier-trace.stderr"
        trace_telemetry_path = trace_output / "classifier-trace-telemetry.json"
        command = pcl_label_command(result, trace_telemetry_path)
        environment = os.environ.copy()
        environment["TPTP"] = str(problem_root / "problems" / "casc_2025")
        trace = run(command, cwd=result_path.parent, environment=environment)
        proof_path.write_bytes(trace.stdout)
        trace_stderr_path.write_bytes(trace.stderr)
        trace_status = final_status(trace.stdout)
        write_json(
            trace_output / "classifier-trace.json",
            {
                "schema_version": 1,
                "source_result": str(result_path),
                "command": command,
                "return_code": trace.returncode,
                "szs_status": trace_status,
                "stdout_sha256": sha256_file(proof_path),
                "stderr_sha256": sha256_file(trace_stderr_path),
            },
        )
        if trace.returncode != 0 or trace_status != result.get("szs_status"):
            raise ExperimentError(
                f"PCL control rerun changed result: {result['problem_id']}"
            )
        completed = run(
            [
                str(kb_ginsert),
                "-k",
                kb_name,
                "-n",
                str(result["problem_id"]),
                str(proof_path),
            ],
            cwd=output_root,
        )
        (trace_output / "classifier-insert.stdout").write_bytes(
            completed.stdout
        )
        (trace_output / "classifier-insert.stderr").write_bytes(
            completed.stderr
        )
        if completed.returncode != 0:
            raise ExperimentError(
                f"control proof insertion failed: {result['problem_id']}"
            )
        inserted.append(str(result["problem_id"]))
    if not inserted:
        raise ExperimentError(f"no successful control proofs for {split}")
    return kb_path, inserted


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--training-kb", type=Path, required=True)
    parser.add_argument("--search-root", type=Path, required=True)
    parser.add_argument("--kb-create", type=Path, required=True)
    parser.add_argument("--kb-ginsert", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--label-extraction-revision", required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    if sys.platform != "linux":
        raise ExperimentError("classifier preparation may run only on Linux")
    training_kb = arguments.training_kb.resolve()
    search_root = arguments.search_root.resolve()
    kb_create = arguments.kb_create.resolve()
    kb_ginsert = arguments.kb_ginsert.resolve()
    problem_root = arguments.problem_root.resolve()
    output_root = arguments.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    if arguments.label_extraction_revision != LABEL_EXTRACTION_REVISION:
        raise ExperimentError(
            "label-extraction revision differs from the transparent amendment"
        )
    label_binary = label_extraction_binary(search_root)
    for executable in (kb_create, kb_ginsert, label_binary):
        if not executable.is_file() or not os.access(executable, os.X_OK):
            raise ExperimentError(f"missing executable: {executable}")

    training = annotation_entries(training_kb / "clausepatterns")
    metadata: dict[str, Any] = {
        "schema_version": 1,
        "label_extraction_revision": arguments.label_extraction_revision,
        "label_extraction_binary": str(label_binary),
        "label_extraction_binary_sha256": sha256_file(label_binary),
        "training_kb": str(training_kb),
        "training_clausepatterns_sha256": sha256_file(
            training_kb / "clausepatterns"
        ),
        "training": training,
        "heldout": {},
    }
    (output_root / "train-self.tsm").write_text(
        classifier_text(training, training),
        encoding="utf-8",
        newline="\n",
    )

    for split in ("validation", "test"):
        kb_path, inserted = create_label_kb(
            split=split,
            search_root=search_root,
            output_root=output_root,
            kb_create=kb_create,
            kb_ginsert=kb_ginsert,
            problem_root=problem_root,
        )
        heldout = annotation_entries(kb_path / "clausepatterns")
        input_path = output_root / f"{split}.tsm"
        input_path.write_text(
            classifier_text(training, heldout),
            encoding="utf-8",
            newline="\n",
        )
        metadata["heldout"][split] = {
            "inserted_control_problems": inserted,
            "knowledge_base": str(kb_path),
            "clausepatterns_sha256": sha256_file(kb_path / "clausepatterns"),
            "entries": heldout,
            "input_sha256": sha256_file(input_path),
        }

    metadata["train_self_input_sha256"] = sha256_file(
        output_root / "train-self.tsm"
    )
    write_json(output_root / "metadata.json", metadata)
    print(
        json.dumps(
            {
                "training_terms": len(training),
                "validation_terms": len(
                    metadata["heldout"]["validation"]["entries"]
                ),
                "test_terms": len(metadata["heldout"]["test"]["entries"]),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ExperimentError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
