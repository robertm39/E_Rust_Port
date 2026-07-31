#!/usr/bin/env python3
"""Replay every result artifact and run the remaining corruption checks."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any


RUN_DIRECTORIES = {
    "saturation_long": "saturation-long",
    "instgen_long": "instgen-long",
    "saturation_short": "saturation-short",
    "instgen_short": "instgen-short",
    "cooperative_saturation": "cooperative-saturation",
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load(name: str, path: Path):
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def run_proof_gate(
    *,
    validation_gate: Path,
    proofcheck: Path,
    problem: Path,
    solution: Path,
    report: Path,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            "python3",
            str(validation_gate),
            str(problem),
            str(solution),
            "--proof-command-json",
            json.dumps(
                [str(proofcheck), "-p", "{problem}", "{artifact}"]
            ),
            "--report",
            str(report),
        ],
        check=False,
        capture_output=True,
        text=True,
        timeout=180,
    )


def verify_hash(path: Path, expected: Any, label: str) -> None:
    if not path.is_file() or sha256_file(path) != expected:
        raise ValueError(f"{label} hash mismatch")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--results-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--proofcheck", type=Path, required=True)
    parser.add_argument("--validation-gate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    repo = arguments.repo_root.resolve()
    results = arguments.results_root.resolve()
    experiment = Path(__file__).resolve().parent
    verifier = load(
        "instgen_full_result_verifier", experiment / "verify_certificate.py"
    )
    runner = load(
        "instgen_full_result_runner", experiment / "run_experiment.py"
    )
    coordinates = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(results.glob("*-r*/coordinate.json"))
    ]
    if len(coordinates) != 47:
        raise ValueError(f"expected 47 coordinates, found {len(coordinates)}")

    candidate_certificates = 0
    candidate_models = 0
    candidate_proofs = 0
    umlaut_proofs = 0
    augmented_inputs = 0
    proof_mutation_fixture: tuple[Path, Path] | None = None
    augmented_mutation_fixture: tuple[Path, str] | None = None
    with tempfile.TemporaryDirectory() as raw_temporary:
        temporary = Path(raw_temporary)
        for coordinate in coordinates:
            root = results / coordinate["coordinate_id"]
            problem = arguments.problem_root.resolve() / next(
                record["source_path"]
                for record in (
                    coordinate["runs"]["instgen_long"],
                    coordinate["runs"]["instgen_short"],
                )
            )
            if sha256_file(problem) != coordinate["source_sha256"]:
                raise ValueError("coordinate source hash mismatch")
            expected = coordinate["expected_status"]
            for method in runner.METHODS if hasattr(runner, "METHODS") else (
                "saturation",
                "standalone",
                "portfolio",
                "cooperative",
            ):
                outcome = coordinate["methods"][method]
                if outcome["verified"] and outcome["status"] != expected:
                    raise ValueError("verified method polarity mismatch")

            for name in ("instgen_long", "instgen_short"):
                run = coordinate["runs"][name]
                run_root = root / RUN_DIRECTORIES[name]
                certificate_path = run_root / "worker" / "certificate.json"
                certificate = json.loads(
                    certificate_path.read_text(encoding="utf-8")
                )
                for key, value in certificate.items():
                    if run.get(key) != value:
                        raise ValueError(f"embedded certificate mismatch: {key}")
                checked = verifier.verify_certificate(
                    certificate_path=certificate_path,
                    problem_path=problem,
                    repo_root=repo,
                    drat_trim=arguments.drat_trim.resolve(),
                )
                candidate_certificates += 1
                candidate_models += int(checked["model_checked"])
                candidate_proofs += int(checked["proof_checked"])
                verify_hash(
                    run_root / "runner.stdout.txt",
                    run["runner_stdout_sha256"],
                    "candidate stdout",
                )
                verify_hash(
                    run_root / "runner.stderr.txt",
                    run["runner_stderr_sha256"],
                    "candidate stderr",
                )
                verify_hash(
                    run_root / "runner.time.txt",
                    run["runner_time_sha256"],
                    "candidate time",
                )

            augmented = coordinate.get("augmented")
            if augmented is not None:
                augmented_path = root / augmented["path"]
                verify_hash(
                    augmented_path, augmented["sha256"], "augmented input"
                )
                augmented_inputs += 1
                if augmented_mutation_fixture is None:
                    augmented_mutation_fixture = (
                        augmented_path,
                        augmented["sha256"],
                    )

            for name in (
                "saturation_long",
                "saturation_short",
                "cooperative_saturation",
            ):
                run = coordinate["runs"][name]
                if run is None:
                    continue
                run_root = root / RUN_DIRECTORIES[name]
                solution = run_root / run["solution_path"]
                verify_hash(solution, run["solution_sha256"], "Umlaut solution")
                verify_hash(
                    run_root / "stderr.txt",
                    run["stderr_sha256"],
                    "Umlaut stderr",
                )
                if run["time_sha256"] is not None:
                    verify_hash(
                        run_root / "time.txt",
                        run["time_sha256"],
                        "Umlaut time",
                    )
                if run["status"] != "unsat":
                    continue
                proof_problem = (
                    root / coordinate["augmented"]["path"]
                    if name == "cooperative_saturation"
                    else problem
                )
                replay = run_proof_gate(
                    validation_gate=arguments.validation_gate.resolve(),
                    proofcheck=arguments.proofcheck.resolve(),
                    problem=proof_problem,
                    solution=solution,
                    report=temporary
                    / f"{coordinate['coordinate_id']}-{name}.json",
                )
                if replay.returncode != 0:
                    raise ValueError(
                        "Umlaut proof replay failed: "
                        + (replay.stdout + replay.stderr)[-2000:]
                    )
                umlaut_proofs += 1
                if proof_mutation_fixture is None:
                    proof_mutation_fixture = (proof_problem, solution)

        if augmented_mutation_fixture is None:
            raise ValueError("no augmented-input mutation fixture")
        source, expected_hash = augmented_mutation_fixture
        corrupted_augmented = temporary / "mutated-augmented.p"
        data = bytearray(source.read_bytes())
        data[-2] ^= 1
        corrupted_augmented.write_bytes(data)
        if sha256_file(corrupted_augmented) == expected_hash:
            raise ValueError("augmented-input mutation was not detected")

        if proof_mutation_fixture is None:
            raise ValueError("no Umlaut-proof mutation fixture")
        proof_problem, _ = proof_mutation_fixture
        corrupted_solution = temporary / "mutated-solution.txt"
        corrupted_solution.write_text(
            "% SZS status Unsatisfiable\n", encoding="utf-8"
        )
        proof_mutation = run_proof_gate(
            validation_gate=arguments.validation_gate.resolve(),
            proofcheck=arguments.proofcheck.resolve(),
            problem=proof_problem,
            solution=corrupted_solution,
            report=temporary / "mutated-proof-report.json",
        )
        if proof_mutation.returncode == 0:
            raise ValueError("Umlaut proof mutation was accepted")

    report = {
        "schema_version": 1,
        "coordinates": len(coordinates),
        "candidate_certificates_replayed": candidate_certificates,
        "candidate_models_checked": candidate_models,
        "candidate_drat_proofs_checked": candidate_proofs,
        "umlaut_proofs_replayed": umlaut_proofs,
        "augmented_inputs_checked": augmented_inputs,
        "mutations_rejected": {
            "augmented_clause": True,
            "umlaut_proof": True,
        },
        "passed": True,
    }
    arguments.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
