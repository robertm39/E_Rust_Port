#!/usr/bin/env python3
"""Resume one treatment shard without concurrent JSONL writes."""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path


def load(name: str, path: Path):
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--method", choices=("clausify", "ematch", "mbqi"), required=True
    )
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--cadical-driver", type=Path, required=True)
    parser.add_argument("--drat-trim", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    arguments = parser.parse_args()
    here = Path(__file__).resolve().parent
    runner = load("ematching_mbqi_shard_runner", here / "run_experiment.py")
    repo_root = arguments.repo_root.resolve()
    problem_root = arguments.problem_root.resolve()
    corpus = arguments.corpus.resolve()
    adapter = arguments.cadical_driver.resolve()
    drat_trim = arguments.drat_trim.resolve()
    output_root = arguments.output_root.resolve()
    main_results = output_root / "results.jsonl"
    shard_results = output_root / f"results-{arguments.method}-shard.jsonl"
    completed = {
        json.loads(line)["run_id"]
        for path in (main_results, shard_results)
        if path.exists()
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    }
    records = [
        json.loads(line)
        for line in corpus.read_text(encoding="utf-8").splitlines()
        if line
    ][1:]
    pending: list[tuple[dict[str, object], int]] = []
    for record in records:
        repetitions = 1 if record["holdout_split"] == "train" else 2
        for repetition in range(1, repetitions + 1):
            pending.append((record, repetition))

    with shard_results.open("a", encoding="utf-8", newline="\n") as results:
        for index, (record, repetition) in enumerate(pending, start=1):
            coordinate = f"{record['problem_id']}-r{repetition}"
            run_id = f"{coordinate}/{arguments.method}"
            if run_id in completed:
                continue
            print(
                f"[{arguments.method} {index}/{len(pending)}] {run_id}",
                flush=True,
            )
            canonical_output = (
                output_root / "runs" / coordinate / arguments.method
            )
            recovered = runner.recover_completed_output(canonical_output)
            output = canonical_output
            if recovered is None:
                attempt = 1
                while output.exists():
                    output = canonical_output.with_name(
                        f"{arguments.method}-shard-{attempt}"
                    )
                    attempt += 1
                certificate, validation = runner.run_one(
                    python=sys.executable,
                    worker=here / "quantifier_worker.py",
                    verifier=here / "verify_certificate.py",
                    repo_root=repo_root,
                    problem=problem_root / str(record["path"]),
                    adapter=adapter,
                    drat_trim=drat_trim,
                    output=output,
                    method=arguments.method,
                )
            else:
                certificate, validation = recovered
            expected = (
                "sat"
                if record["expected_class"] == "satisfiable"
                else "unsat"
            )
            result = runner.record_result(
                run_id=run_id,
                kind="corpus",
                problem_id=str(record["problem_id"]),
                partition=str(record["holdout_split"]),
                family=str(record["family"]),
                repetition=repetition,
                method=arguments.method,
                expected_status=expected,
                source_path=str(record["path"]),
                output_path=str(output.relative_to(output_root)),
                certificate=certificate,
                validation=validation,
            )
            results.write(json.dumps(result, sort_keys=True) + "\n")
            results.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
