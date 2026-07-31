#!/usr/bin/env python3
"""Linux integration and mutation checks for candidate certificates."""

from __future__ import annotations

import argparse
import copy
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


def expect_rejected(callback, label: str) -> None:
    try:
        callback()
    except Exception:
        return
    raise AssertionError(f"mutation was accepted: {label}")


def write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    arguments = parser.parse_args()
    repo = arguments.repo_root.resolve()
    experiment = Path(__file__).resolve().parent
    candidate = load("instgen_integration_candidate", experiment / "instgen.py")
    verifier = load(
        "instgen_integration_verifier",
        experiment / "verify_certificate.py",
    )

    with tempfile.TemporaryDirectory() as raw_directory:
        root = Path(raw_directory)
        sat_problem = root / "sat.p"
        sat_problem.write_text(
            "cnf(s1,axiom,(p(X)|~q(X))).\n"
            "cnf(s2,axiom,q(a)).\n",
            encoding="utf-8",
        )
        unsat_problem = root / "unsat.p"
        unsat_problem.write_text(
            "cnf(u0,axiom,s(a)).\n"
            "cnf(u1,axiom,~s(b)).\n"
            "cnf(u2,axiom,(s(X)|p(X)|q(X))).\n"
            "cnf(u3,axiom,(s(X)|p(X)|~q(X))).\n"
            "cnf(u4,axiom,(s(X)|~p(X)|q(X))).\n"
            "cnf(u5,axiom,(s(X)|~p(X)|~q(X))).\n",
            encoding="utf-8",
        )
        sat_root = root / "sat"
        unsat_root = root / "unsat"
        sat = candidate.run(
            problem_path=sat_problem,
            adapter=arguments.cadical_driver.resolve(),
            drat_trim=arguments.drat_trim.resolve(),
            output_root=sat_root,
            budget_seconds=5.0,
        )
        unsat = candidate.run(
            problem_path=unsat_problem,
            adapter=arguments.cadical_driver.resolve(),
            drat_trim=arguments.drat_trim.resolve(),
            output_root=unsat_root,
            budget_seconds=5.0,
        )
        if sat["status"] != "sat":
            raise AssertionError(f"SAT fixture returned {sat['status']}")
        if unsat["status"] != "unsat" or unsat["refinement_iterations"] < 1:
            raise AssertionError(
                "UNSAT fixture did not require and verify a refinement"
            )
        sat_verified = verifier.verify_certificate(
            certificate_path=sat_root / "certificate.json",
            problem_path=sat_problem,
            repo_root=repo,
            drat_trim=arguments.drat_trim.resolve(),
        )
        unsat_verified = verifier.verify_certificate(
            certificate_path=unsat_root / "certificate.json",
            problem_path=unsat_problem,
            repo_root=repo,
            drat_trim=arguments.drat_trim.resolve(),
        )
        if not sat_verified["model_checked"] or not unsat_verified["proof_checked"]:
            raise AssertionError("terminal certificates were not checked")

        original = json.loads(
            (unsat_root / "certificate.json").read_text(encoding="utf-8")
        )

        def mutate_certificate(label: str, mutation) -> None:
            case = root / f"mutation-{label}"
            shutil.copytree(unsat_root, case)
            value = copy.deepcopy(original)
            mutation(value)
            write_json(case / "certificate.json", value)
            expect_rejected(
                lambda: verifier.verify_certificate(
                    certificate_path=case / "certificate.json",
                    problem_path=unsat_problem,
                    repo_root=repo,
                    drat_trim=arguments.drat_trim.resolve(),
                ),
                label,
            )

        mutate_certificate(
            "source-hash",
            lambda value: value.__setitem__("source_sha256", "0" * 64),
        )
        mutate_certificate(
            "substitution",
            lambda value: value["instances"][-1]["substitution"].__setitem__(
                "X", "not_in_domain"
            ),
        )
        mutate_certificate(
            "ground-clause",
            lambda value: value["instances"][-1]["ground_clause"][0].__setitem__(
                "positive",
                not value["instances"][-1]["ground_clause"][0]["positive"],
            ),
        )

        proof_case = root / "mutation-proof"
        shutil.copytree(unsat_root, proof_case)
        proof_path = (
            proof_case
            / original["proof"]["proof_path"]
        )
        proof_path.write_bytes(b"")
        corrupted = copy.deepcopy(original)
        corrupted["proof"]["proof_sha256"] = verifier.sha256_file(proof_path)
        write_json(proof_case / "certificate.json", corrupted)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=proof_case / "certificate.json",
                problem_path=unsat_problem,
                repo_root=repo,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "proof",
        )

        sat_original = json.loads(
            (sat_root / "certificate.json").read_text(encoding="utf-8")
        )
        sat_case = root / "mutation-model"
        shutil.copytree(sat_root, sat_case)
        sat_mutated = copy.deepcopy(sat_original)
        sat_mutated["true_atoms"] = []
        write_json(sat_case / "certificate.json", sat_mutated)
        expect_rejected(
            lambda: verifier.verify_certificate(
                certificate_path=sat_case / "certificate.json",
                problem_path=sat_problem,
                repo_root=repo,
                drat_trim=arguments.drat_trim.resolve(),
            ),
            "model",
        )

        print(
            json.dumps(
                {
                    "mutations_rejected": 5,
                    "sat": sat_verified,
                    "unsat": unsat_verified,
                    "unsat_refinements": unsat["refinement_iterations"],
                },
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
