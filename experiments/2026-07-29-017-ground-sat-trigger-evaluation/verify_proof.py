#!/usr/bin/env python3
"""Verify the proof-only SATCheck core-reconstruction witness on Linux."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from types import ModuleType
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
REPO_ROOT = EXPERIMENT_ROOT.parents[1]
PROOFCHECK_HELPER = (
    EXPERIMENT_ROOT.parent
    / "2026-07-27-004-soundness-validation-gates"
    / "run_validation.py"
)
EXPECTED_SAT = {
    "checks": 1,
    "satisfiable": 0,
    "unsatisfiable": 1,
    "input_clauses": 4,
    "post_purity_clauses": 4,
    "unsat_core_clauses": 4,
}
CORE_DIMACS = """p cnf 2 4
1 2 0
-1 2 0
1 -2 0
-1 -2 0
"""
CDCL_INFERENCE = re.compile(
    r"inference\(cdclpropres,\[status\(thm\)\],"
    r"\[(?P<parents>[^\]]+)\]\)"
)


class VerificationError(RuntimeError):
    """Raised when the proof-only validation contract fails."""


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise VerificationError(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


PROOFCHECK = load_module("ground_sat_proofcheck_helper", PROOFCHECK_HELPER)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run(
    command: Sequence[str],
    *,
    cwd: Path,
    environment: dict[str, str],
    stdout: Path,
    stderr: Path,
    timeout: int,
) -> subprocess.CompletedProcess[bytes]:
    completed = subprocess.run(
        list(command),
        cwd=cwd,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=timeout,
    )
    stdout.write_bytes(completed.stdout)
    stderr.write_bytes(completed.stderr)
    return completed


def validate_telemetry(path: Path) -> dict[str, Any]:
    telemetry = json.loads(path.read_text(encoding="utf-8"))
    observed = {
        key: telemetry["sat"][key] for key in EXPECTED_SAT
    }
    if observed != EXPECTED_SAT:
        raise VerificationError(
            f"unexpected SATCheck telemetry: {observed}"
        )
    if telemetry["outcome"]["reason"] != "sat_check":
        raise VerificationError("witness did not terminate through SATCheck")
    return telemetry


def validate_proofs(first: Path, second: Path) -> list[str]:
    if first.read_bytes() != second.read_bytes():
        raise VerificationError("proof repetitions are not byte-identical")
    text = first.read_text(encoding="utf-8")
    if "% SZS status Unsatisfiable" not in text:
        raise VerificationError("proof has no Unsatisfiable status")
    match = CDCL_INFERENCE.search(text)
    if match is None:
        raise VerificationError("proof has no cdclpropres inference")
    parents = [
        parent.strip()
        for parent in match.group("parents").split(",")
        if parent.strip()
    ]
    if len(parents) != 4 or len(set(parents)) != 4:
        raise VerificationError(
            f"expected four distinct core parents, got {parents}"
        )
    return parents


def verify(arguments: argparse.Namespace) -> dict[str, Any]:
    repo = arguments.repo.resolve()
    output = arguments.output_root.resolve()
    output.mkdir(parents=True, exist_ok=True)
    problem = arguments.problem.resolve()
    proof = arguments.proof.resolve()
    proof_repeat = arguments.proof_repeat.resolve()
    telemetry = arguments.telemetry.resolve()
    cadical = arguments.cadical.resolve()
    for path in (problem, proof, proof_repeat, telemetry, cadical):
        if not path.is_file():
            raise VerificationError(f"missing validation input: {path}")

    telemetry_record = validate_telemetry(telemetry)
    core_parents = validate_proofs(proof, proof_repeat)
    environment = os.environ.copy()
    environment["TPTP"] = str(problem.parent)

    core_path = output / "reported-core.cnf"
    core_path.write_text(
        CORE_DIMACS,
        encoding="utf-8",
        newline="\n",
    )
    cadical_stdout = output / "cadical.stdout"
    cadical_stderr = output / "cadical.stderr"
    cadical_result = run(
        [str(cadical), str(core_path)],
        cwd=cadical.parent,
        environment=environment,
        stdout=cadical_stdout,
        stderr=cadical_stderr,
        timeout=60,
    )
    cadical_text = cadical_stdout.read_text(
        encoding="utf-8", errors="replace"
    )
    if (
        cadical_result.returncode != 20
        or "s UNSATISFIABLE" not in cadical_text
    ):
        raise VerificationError("CaDiCaL did not re-solve the core as UNSAT")

    external = output / "external"
    external.mkdir(exist_ok=True)
    proofcheck = PROOFCHECK.download_proofcheck(external)
    self_stdout = output / "proofcheck-self-certify.stdout"
    self_stderr = output / "proofcheck-self-certify.stderr"
    self_result = run(
        [str(proofcheck), "-self-certify"],
        cwd=proofcheck.parent,
        environment=environment,
        stdout=self_stdout,
        stderr=self_stderr,
        timeout=300,
    )
    self_text = self_stdout.read_text(
        encoding="utf-8", errors="replace"
    ) + self_stderr.read_text(encoding="utf-8", errors="replace")
    if self_result.returncode != 0 or "117 passed" not in self_text:
        raise VerificationError("ProofCheck self-certification failed")

    gate = repo / "tools/validation/validate_tptp_solution.py"
    gate_report = output / "proofcheck-gate.json"
    gate_stdout = output / "proofcheck-gate.stdout"
    gate_stderr = output / "proofcheck-gate.stderr"
    proof_command = [
        str(proofcheck),
        "-j",
        "2",
        "-t",
        "5",
        "-T",
        "120",
        "-p",
        str(problem),
        str(proof),
    ]
    gate_command = [
        sys.executable,
        str(gate),
        str(problem),
        str(proof),
        "--report",
        str(gate_report),
        "--timeout-seconds",
        "120",
        "--proof-command-json",
        json.dumps(proof_command, separators=(",", ":")),
    ]
    gate_result = run(
        gate_command,
        cwd=repo,
        environment=environment,
        stdout=gate_stdout,
        stderr=gate_stderr,
        timeout=180,
    )
    gate_record = json.loads(gate_report.read_text(encoding="utf-8"))
    if gate_record["verdict"] not in {"verified", "coverage_gap"}:
        raise VerificationError(
            f"external proof gate rejected the witness: {gate_record}"
        )

    report = {
        "schema_version": 1,
        "kind": "umlaut-ground-sat-proof-validation",
        "problem_sha256": sha256_file(problem),
        "proof_sha256": sha256_file(proof),
        "proof_repeat_sha256": sha256_file(proof_repeat),
        "telemetry_sha256": sha256_file(telemetry),
        "telemetry_sat": {
            key: telemetry_record["sat"][key] for key in EXPECTED_SAT
        },
        "core_parents": core_parents,
        "core_dimacs_sha256": sha256_file(core_path),
        "cadical": {
            "binary_sha256": sha256_file(cadical),
            "returncode": cadical_result.returncode,
            "result": "unsatisfiable",
            "stdout_sha256": sha256_file(cadical_stdout),
            "stderr_sha256": sha256_file(cadical_stderr),
        },
        "proofcheck": {
            "tag": PROOFCHECK.PROOFCHECK_TAG,
            "release_archive_sha256": PROOFCHECK.PROOFCHECK_SHA256,
            "binary_sha256": sha256_file(proofcheck),
            "self_certify_returncode": self_result.returncode,
            "gate_returncode": gate_result.returncode,
            "gate_verdict": gate_record["verdict"],
            "gate_reasons": gate_record["reasons"],
            "gate_report_sha256": sha256_file(gate_report),
        },
    }
    report["report_id"] = hashlib.sha256(
        json.dumps(
            report,
            sort_keys=True,
            separators=(",", ":"),
        ).encode()
    ).hexdigest()
    report_path = output / "proof-validation.json"
    report_path.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return report


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--problem", type=Path, required=True)
    parser.add_argument("--proof", type=Path, required=True)
    parser.add_argument("--proof-repeat", type=Path, required=True)
    parser.add_argument("--telemetry", type=Path, required=True)
    parser.add_argument("--cadical", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    if sys.platform != "linux":
        raise VerificationError("proof validation requires Linux")
    report = verify(parse_args(argv))
    print(
        f"OK: core UNSAT; ProofCheck "
        f"{report['proofcheck']['gate_verdict']}; "
        f"report {report['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        VerificationError,
        OSError,
        ValueError,
        json.JSONDecodeError,
        subprocess.SubprocessError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2) from error
