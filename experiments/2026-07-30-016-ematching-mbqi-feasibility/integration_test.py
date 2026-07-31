#!/usr/bin/env python3
"""End-to-end hand-corpus and mutation tests using the real SAT adapter."""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
import sys
import tempfile
from pathlib import Path


def load(name: str, path: Path):
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def expect_rejected(callback, label: str) -> None:
    try:
        callback()
    except Exception:
        return
    raise AssertionError(f"mutation was accepted: {label}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    arguments = parser.parse_args()
    repo_root = arguments.repo_root.resolve()
    here = Path(__file__).resolve().parent
    worker = load("quantifier_integration_worker", here / "quantifier_worker.py")
    verifier = load(
        "quantifier_integration_verifier", here / "verify_certificate.py"
    )

    expectations = {
        "ground-unsat.p": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "unary-chain-unsat.p": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "unary-chain-sat.p": {
            "clausify": "sat",
            "ematch": "sat",
            "mbqi": "sat",
        },
        "multipattern-unsat.p": {
            "clausify": "unsat",
            "ematch": "unsat",
            "mbqi": "unsat",
        },
        "incomplete-trigger-sat.p": {
            "clausify": "sat",
            "ematch": "unknown",
            "mbqi": "sat",
        },
    }

    with tempfile.TemporaryDirectory(prefix="ematching-mbqi-") as raw_root:
        root = Path(raw_root)
        outputs: dict[tuple[str, str], Path] = {}
        validations = 0
        for problem_name, methods in expectations.items():
            problem = here / "hand" / problem_name
            for method, expected in methods.items():
                output = root / problem.stem / method
                certificate = worker.run(
                    method=method,
                    repo_root=repo_root,
                    problem_path=problem,
                    adapter=arguments.cadical_driver.resolve(),
                    drat_trim=arguments.drat_trim.resolve(),
                    output_root=output,
                    budget_seconds=2.0,
                    max_instances=10_000,
                    max_steps=25_000,
                )
                if certificate["status"] != expected:
                    raise AssertionError(
                        f"{problem_name}/{method}: "
                        f"{certificate['status']} != {expected}"
                    )
                verifier.verify_certificate(
                    certificate_path=output / "certificate.json",
                    problem_path=problem,
                    repo_root=repo_root,
                    drat_trim=arguments.drat_trim.resolve(),
                )
                validations += 1
                outputs[(problem_name, method)] = output

        ematch_certificate = json.loads(
            (
                outputs[("unary-chain-unsat.p", "ematch")]
                / "certificate.json"
            ).read_text(encoding="utf-8")
        )
        if (
            ematch_certificate["method_data"]["round_count"] < 2
            or ematch_certificate["method_data"]["candidate_matches"] == 0
        ):
            raise AssertionError("unary E-matching did not exercise rounds")
        multipattern = json.loads(
            (
                outputs[("multipattern-unsat.p", "ematch")]
                / "certificate.json"
            ).read_text(encoding="utf-8")
        )
        if multipattern["method_data"]["multipatterns"] != 1:
            raise AssertionError("multipattern case did not infer a join")
        incomplete = json.loads(
            (
                outputs[("incomplete-trigger-sat.p", "ematch")]
                / "certificate.json"
            ).read_text(encoding="utf-8")
        )
        if incomplete["method_data"]["first_ungenerated_counterexample"] is None:
            raise AssertionError("incomplete trigger did not expose a counterexample")

        def mutated_copy(
            source: Path, label: str
        ) -> tuple[Path, dict[str, object]]:
            destination = root / "mutations" / label
            shutil.copytree(source, destination)
            certificate_path = destination / "certificate.json"
            certificate = json.loads(
                certificate_path.read_text(encoding="utf-8")
            )
            return certificate_path, certificate

        problem = here / "hand" / "unary-chain-unsat.p"

        path, value = mutated_copy(
            outputs[("unary-chain-unsat.p", "ematch")], "substitution"
        )
        variable_instance = next(
            instance
            for instance in value["instances"]
            if instance["substitution"]
        )
        variable = next(iter(variable_instance["substitution"]))
        variable_instance["substitution"][variable] = (
            "b"
            if variable_instance["substitution"][variable] != "b"
            else "a"
        )
        value["semantic_sha256"] = verifier.stable_json_sha256(
            verifier.semantic_payload(value)
        )
        write_json(path, value)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=problem,
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "substitution",
        )

        path, value = mutated_copy(
            outputs[("unary-chain-unsat.p", "ematch")], "ground-clause"
        )
        value["instances"][-1]["ground_clause"][0]["positive"] = not value[
            "instances"
        ][-1]["ground_clause"][0]["positive"]
        value["semantic_sha256"] = verifier.stable_json_sha256(
            verifier.semantic_payload(value)
        )
        write_json(path, value)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=problem,
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "ground-clause",
        )

        path, value = mutated_copy(
            outputs[("unary-chain-unsat.p", "ematch")], "trigger"
        )
        trigger = next(
            record
            for record in value["method_data"]["triggers"]
            if record["pattern"]
        )
        trigger["pattern"][0] = "invented(X)"
        value["semantic_sha256"] = verifier.stable_json_sha256(
            verifier.semantic_payload(value)
        )
        write_json(path, value)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=problem,
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "trigger-binding",
        )

        mbqi_source = outputs[("incomplete-trigger-sat.p", "mbqi")]
        path, value = mutated_copy(mbqi_source, "refinement-model")
        value["method_data"]["refinement_log"][0]["true_atoms"].append(
            "invented()"
        )
        value["semantic_sha256"] = verifier.stable_json_sha256(
            verifier.semantic_payload(value)
        )
        write_json(path, value)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=here / "hand" / "incomplete-trigger-sat.p",
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "refinement-model",
        )

        path, value = mutated_copy(
            outputs[("ground-unsat.p", "clausify")], "dimacs"
        )
        dimacs = path.parent / str(value["dimacs_path"])
        dimacs.write_text(
            dimacs.read_text(encoding="ascii") + "c mutation\n",
            encoding="ascii",
            newline="\n",
        )
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=here / "hand" / "ground-unsat.p",
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "dimacs",
        )

        path, value = mutated_copy(
            outputs[("ground-unsat.p", "clausify")], "drat"
        )
        proof = path.parent / str(value["proof"]["proof_path"])
        proof.write_bytes(proof.read_bytes() + b"0\n")
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=path,
                problem_path=here / "hand" / "ground-unsat.p",
                repo_root=repo_root,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "drat",
        )

        summary = {
            "hand_certificates_checked": validations,
            "mutations_rejected": [
                "substitution",
                "ground_clause",
                "trigger_binding",
                "refinement_model",
                "dimacs",
                "drat",
            ],
        }
        print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
