#!/usr/bin/env python3
"""Ubuntu hand-case and certificate-mutation integration matrix."""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Sequence

import connection_common as common


def run_command(command: list[str], expected: int = 0) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    if completed.returncode != expected:
        raise common.ExperimentError(
            f"unexpected exit {completed.returncode}, expected {expected}: "
            + " ".join(command)
            + "\n"
            + (completed.stdout + completed.stderr)[-4_000:]
        )
    return completed


def walk_nodes(node: dict[str, Any]) -> list[dict[str, Any]]:
    nodes = [node]
    if node["kind"] == "extension":
        nodes.extend(walk_nodes(node["branch"]))
        nodes.extend(walk_nodes(node["continuation"]))
    elif node["kind"] == "reduction":
        nodes.extend(walk_nodes(node["continuation"]))
    return nodes


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    if sys.platform != "linux":
        raise common.ExperimentError("integration prover tests may run only on Linux")
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    binary = arguments.binary.resolve()
    here = Path(__file__).resolve().parent
    hand = here / "hand"
    worker = here / "connection_worker.py"
    verifier = here / "verify_connection.py"

    with tempfile.TemporaryDirectory(prefix="umlaut-connection-") as temporary:
        root = Path(temporary)
        cases: dict[str, dict[str, Any]] = {}
        for name in (
            "extension_reduction",
            "variables_functions",
            "satisfiable_unknown",
        ):
            problem = hand / f"{name}.p"
            output = root / name
            run_command(
                [
                    sys.executable,
                    str(worker),
                    "--repo-root",
                    str(repo_root),
                    "--binary",
                    str(binary),
                    "--problem",
                    str(problem),
                    "--tptp-root",
                    str(hand),
                    "--output-root",
                    str(output),
                ]
            )
            certificate_path = output / "certificate.json"
            certificate = json.loads(certificate_path.read_text(encoding="utf-8"))
            verification = run_command(
                [
                    sys.executable,
                    str(verifier),
                    "--certificate",
                    str(certificate_path),
                    "--transcript",
                    str(output / "cnf.tstp"),
                    "--repo-root",
                    str(repo_root),
                    "--binary",
                    str(binary),
                    "--problem",
                    str(problem),
                    "--tptp-root",
                    str(hand),
                ]
            )
            cases[name] = {
                "problem": problem,
                "output": output,
                "certificate": certificate,
                "verification": json.loads(verification.stdout),
            }

        if cases["extension_reduction"]["certificate"]["status"] != "Theorem":
            raise common.ExperimentError("extension/reduction hand case was not proved")
        if cases["variables_functions"]["certificate"]["status"] != "Theorem":
            raise common.ExperimentError("variable/function hand case was not proved")
        if cases["satisfiable_unknown"]["certificate"]["status"] != "Unknown":
            raise common.ExperimentError("satisfiable hand case made a proof claim")
        if not all(case["verification"]["valid"] for case in cases.values()):
            raise common.ExperimentError("a hand-case certificate did not validate")

        base = cases["extension_reduction"]
        certificate = base["certificate"]
        extension_nodes = [
            node for node in walk_nodes(certificate["proof"])
            if node["kind"] == "extension"
        ]
        reduction_nodes = [
            node for node in walk_nodes(certificate["proof"])
            if node["kind"] == "reduction"
        ]
        if not extension_nodes or not reduction_nodes:
            raise common.ExperimentError("mutation fixture lacks required rule kinds")

        mutations: dict[str, dict[str, Any]] = {}
        changed = copy.deepcopy(certificate)
        changed["start_clause_index"] = 1
        mutations["start_clause"] = changed

        changed = copy.deepcopy(certificate)
        next(
            node for node in walk_nodes(changed["proof"])
            if node["kind"] == "extension"
        )["clause_index"] = 999_999
        mutations["extension_clause"] = changed

        changed = copy.deepcopy(certificate)
        next(
            node for node in walk_nodes(changed["proof"])
            if node["kind"] == "extension"
        )["literal_index"] = 999_999
        mutations["extension_literal"] = changed

        changed = copy.deepcopy(certificate)
        next(
            node for node in walk_nodes(changed["proof"])
            if node["kind"] == "reduction"
        )["path_index"] = 999_999
        mutations["reduction_path"] = changed

        changed = copy.deepcopy(certificate)
        changed["proof"]["goal"] = "mutated_goal"
        mutations["goal_diagnostic"] = changed

        changed = copy.deepcopy(certificate)
        changed["proof"]["goal_index"] = 999_999
        mutations["goal_index"] = changed

        changed = copy.deepcopy(certificate)
        extension_mutations = [
            node for node in walk_nodes(changed["proof"])
            if node["kind"] == "extension"
        ]
        extension_mutations[-1]["instance_id"] = extension_mutations[0]["instance_id"]
        mutations["freshness"] = changed

        rejected: list[str] = []
        for name, changed_certificate in mutations.items():
            changed_path = root / f"mutated-{name}.json"
            changed_path.write_text(
                json.dumps(changed_certificate, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            run_command(
                [
                    sys.executable,
                    str(verifier),
                    "--certificate",
                    str(changed_path),
                    "--transcript",
                    str(base["output"] / "cnf.tstp"),
                    "--repo-root",
                    str(repo_root),
                    "--binary",
                    str(binary),
                    "--problem",
                    str(base["problem"]),
                    "--tptp-root",
                    str(hand),
                ],
                expected=1,
            )
            rejected.append(name)

        transcript = base["output"] / "cnf.tstp"
        original_transcript = transcript.read_bytes()
        transcript.write_bytes(original_transcript + b"\n% mutation\n")
        try:
            run_command(
                [
                    sys.executable,
                    str(verifier),
                    "--certificate",
                    str(base["output"] / "certificate.json"),
                    "--transcript",
                    str(transcript),
                    "--repo-root",
                    str(repo_root),
                    "--binary",
                    str(binary),
                    "--problem",
                    str(base["problem"]),
                    "--tptp-root",
                    str(hand),
                ],
                expected=1,
            )
            rejected.append("transcript")
        finally:
            transcript.write_bytes(original_transcript)

        run_command(
            [
                sys.executable,
                str(verifier),
                "--certificate",
                str(base["output"] / "certificate.json"),
                "--transcript",
                str(transcript),
                "--repo-root",
                str(repo_root),
                "--binary",
                str(binary),
                "--problem",
                str(cases["satisfiable_unknown"]["problem"]),
                "--tptp-root",
                str(hand),
            ],
            expected=1,
        )
        rejected.append("problem")
        print(
            json.dumps(
                {
                    "schema_version": 1,
                    "hand_statuses": {
                        name: case["certificate"]["status"]
                        for name, case in cases.items()
                    },
                    "proofs_checked": sum(
                        bool(case["verification"]["proof_checked"])
                        for case in cases.values()
                    ),
                    "mutations_rejected": rejected,
                },
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())

