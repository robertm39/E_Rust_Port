#!/usr/bin/env python3
"""Unit tests for the incremental SAT experiment scripts."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / f"{name}.py")
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {name}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


generate = load("generate_workloads")
validate = load("validate_results")
benchmark = load("benchmark")
isat_to_dimacs = load("isat_to_dimacs")
strip_picosat_rupd32 = load("strip_picosat_rupd32")
select_capture_corpus = load("select_capture_corpus")
capture = load("capture")
prepare_captures = load("prepare_captures")
analyze = load("analyze")
select_dispatch = load("select_dispatch")
combine_workloads = load("combine_workloads")
generate_measurements = load("generate_measurement_workloads")
validate_large_cores = load("validate_large_cores")
measure_proof_overhead = load("measure_proof_overhead")
analyze_cancellation = load("analyze_cancellation")


class GeneratorTests(unittest.TestCase):
    def test_pigeonhole_two_into_one_is_unsatisfiable(self) -> None:
        maximum, clauses = generate.pigeonhole(2, 1)
        state = validate.QueryState(tuple(clauses), (), maximum)
        self.assertFalse(validate.brute_force(state))

    def test_parity_chain_accepts_both_parities(self) -> None:
        maximum, clauses = generate.parity_chain(4)
        even = validate.QueryState(tuple(clauses), (maximum,), maximum)
        odd = validate.QueryState(tuple(clauses), (-maximum,), maximum)
        self.assertTrue(validate.brute_force(even))
        self.assertTrue(validate.brute_force(odd))

    def test_generation_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as first_raw, tempfile.TemporaryDirectory() as second_raw:
            first = Path(first_raw)
            second = Path(second_raw)
            generate.emit_fixtures(first)
            generate.emit_structured(first)
            generate.emit_fixtures(second)
            generate.emit_structured(second)
            first_files = sorted(path.relative_to(first) for path in first.rglob("*.isat"))
            second_files = sorted(path.relative_to(second) for path in second.rglob("*.isat"))
            self.assertEqual(first_files, second_files)
            self.assertTrue(first_files)
            for relative in first_files:
                self.assertEqual(
                    (first / relative).read_bytes(), (second / relative).read_bytes()
                )

    def test_measurement_generation_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory() as first_raw, tempfile.TemporaryDirectory() as second_raw:
            first = Path(first_raw)
            second = Path(second_raw)
            first_manifest = generate_measurements.generate(first)
            second_manifest = generate_measurements.generate(second)
            self.assertEqual(
                [item["sha256"] for item in first_manifest],
                [item["sha256"] for item in second_manifest],
            )
            self.assertEqual(len(first_manifest), 4)
            self.assertEqual(
                validate.parse_session(first / "proof-pigeonhole-8-7.isat")[
                    "solve"
                ].max_variable,
                56,
            )


class ValidatorTests(unittest.TestCase):
    def test_parser_tracks_incremental_clause_prefixes(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "sample.isat"
            path.write_text(
                "p isat 2\na 1 0\nq before -1 0 0\na -1 0\nq after -1 0 0\n",
                encoding="utf-8",
            )
            queries = validate.parse_session(path)
            self.assertEqual(queries["before"].clauses, ((1,),))
            self.assertEqual(queries["after"].clauses, ((1,), (-1,)))

    def test_validator_rejects_wrong_model(self) -> None:
        state = validate.QueryState(((1,),), (), 1)
        failures = validate.validate_record(
            {"backend": "cadical", "status": "sat", "model": [-1], "core": []},
            state,
        )
        self.assertIn("model does not satisfy active formula", failures)

    def test_validator_rejects_satisfiable_core(self) -> None:
        state = validate.QueryState(((1, 2),), (-1, -2), 2)
        failures = validate.validate_record(
            {
                "backend": "cadical",
                "status": "unsat",
                "model": [],
                "core": [-1],
            },
            state,
        )
        self.assertIn("returned assumption core is satisfiable", failures)

    def test_internal_missing_model_is_declared_gap(self) -> None:
        state = validate.QueryState(((1,),), (), 1)
        self.assertEqual(
            validate.validate_record(
                {
                    "backend": "internal-dpll",
                    "status": "sat",
                    "model": [],
                    "core": [],
                },
                state,
            ),
            [],
        )

    def test_zero_variable_sat_has_complete_empty_model(self) -> None:
        state = validate.QueryState((), (), 0)
        self.assertEqual(
            validate.validate_record(
                {
                    "backend": "cadical",
                    "status": "sat",
                    "model": [],
                    "core": [],
                },
                state,
            ),
            [],
        )

    def test_large_core_collector_deduplicates_repetitions(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            session = root / "large.isat"
            session.write_text(
                "p isat 17\na 1 0\nq check -1 0 -1 0\n",
                encoding="utf-8",
            )
            record = {
                "session": str(session),
                "query": "check",
                "backend": "cadical",
                "status": "unsat",
                "assumptions": 1,
                "core": [-1],
            }
            results = root / "results.jsonl"
            results.write_text(
                json.dumps(record) + "\n" + json.dumps(record) + "\n",
                encoding="utf-8",
            )
            cases = validate_large_cores.collect_cases(root, [results])
            self.assertEqual(len(cases), 1)
            case = next(iter(cases.values()))
            self.assertEqual(case.origins, 2)
            self.assertIn("q core_check -1 0 -1 0", validate_large_cores.render_session(case))


class BenchmarkTests(unittest.TestCase):
    def test_discovers_sorted_unique_sessions(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            nested = root / "nested"
            nested.mkdir()
            first = root / "b.isat"
            second = nested / "a.isat"
            first.write_text("p isat 0\nq q -1 0 0\n", encoding="utf-8")
            second.write_text("p isat 0\nq q -1 0 0\n", encoding="utf-8")
            self.assertEqual(
                benchmark.discover_sessions([root, first]),
                sorted([first.resolve(), second.resolve()]),
            )

    def test_backend_parser_requires_existing_executable(self) -> None:
        with self.assertRaises(Exception):
            benchmark.parse_backend("missing=/definitely/not/a/program")

    def test_capture_selection_is_balanced_and_deterministic(self) -> None:
        records = []
        for split in select_capture_corpus.SPLITS:
            for category in select_capture_corpus.CATEGORIES:
                for index in range(4):
                    records.append(
                        {
                            "record_type": "problem",
                            "holdout_split": split,
                            "category": category,
                            "problem_id": f"{category}-{split}-{index}",
                            "sha256": f"{index:064x}",
                        }
                    )
        first = select_capture_corpus.select(records, 2)
        second = select_capture_corpus.select(list(reversed(records)), 2)
        self.assertEqual(first, second)
        self.assertEqual(len(first), 30)

    def test_capture_label_is_path_safe(self) -> None:
        self.assertEqual(
            capture.safe_label(
                {
                    "holdout_split": "test",
                    "category": "FEQ",
                    "problem_id": "ABC001+2",
                }
            ),
            "test-FEQ-ABC001_2",
        )

    def test_dimacs_shape_reads_header(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "sample.cnf"
            path.write_text("p cnf 3 2\n1 0\n-1 2 0\n", encoding="ascii")
            self.assertEqual(capture.dimacs_shape(path), (3, 2))

    def test_capture_preparation_parses_and_renders_session(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "sample.cnf"
            path.write_text("p cnf 3 2\n1 0\n-1 2 0\n", encoding="ascii")
            maximum, clauses = prepare_captures.parse_dimacs(path)
            self.assertEqual((maximum, clauses), (3, [(1,), (-1, 2)]))
            rendered = prepare_captures.render_session(maximum, clauses, "ab" * 32)
            self.assertIn("q cold -1 0 0\n", rendered)
            self.assertIn("q warm2 -1 0 0\n", rendered)
            self.assertIn("q assume_pos -1 0 ", rendered)

    def test_capture_assumptions_are_unique_and_bounded(self) -> None:
        assumptions = prepare_captures.deterministic_assumptions(7, "12" * 32, False)
        self.assertEqual(len(assumptions), 4)
        self.assertEqual(len(set(assumptions)), 4)
        self.assertTrue(all(1 <= literal <= 7 for literal in assumptions))

    def test_percentile_uses_nearest_rank(self) -> None:
        self.assertEqual(analyze.percentile(range(1, 101), 0.95), 95.0)
        self.assertIsNone(analyze.percentile([], 0.95))

    def test_analysis_reports_status_disagreement(self) -> None:
        base = {
            "record_type": "query",
            "session": "s",
            "query": "q",
            "repetition": 0,
            "elapsed_ns": 10,
            "assumptions": 0,
            "insertion_ns": 1,
            "core_ns": 0,
            "core": [],
        }
        summary = analyze.summarize(
            [
                {**base, "backend": "internal-dpll", "status": "sat"},
                {**base, "backend": "cadical", "status": "unsat"},
            ]
        )
        self.assertEqual(len(summary["status_disagreements"]), 1)

    def test_proof_summary_reports_overhead(self) -> None:
        records = [
            {
                "backend": "cadical",
                "proof": proof,
                "returncode": 0,
                "status": "unsat",
                "elapsed_ns": elapsed,
                "process_wall_ns": elapsed + 10,
                "peak_rss_kib": 1,
                "proof_bytes": 5 if proof else 0,
            }
            for proof, elapsed in ((False, 100), (True, 125))
        ]
        summary = measure_proof_overhead.summarize(records)
        self.assertTrue(summary["valid"])
        self.assertEqual(
            summary["backends"]["cadical"]["median_solve_overhead_ratio"],
            1.25,
        )

    def test_cancellation_filename_extracts_deadline(self) -> None:
        match = analyze_cancellation.DEADLINE_PATTERN.search(
            "cancel-pigeonhole-14-13-1000us.isat"
        )
        self.assertIsNotNone(match)
        self.assertEqual(match.group(1), "1000")

    def test_dispatch_charges_candidate_insertion_once(self) -> None:
        base = {
            "record_type": "query",
            "session": "s",
            "repetition": 0,
            "clauses": 32,
            "status": "sat",
            "core_ns": 0,
        }
        records = [
            {
                **base,
                "backend": "internal-dpll",
                "query": "cold",
                "elapsed_ns": 100,
                "insertion_ns": 0,
            },
            {
                **base,
                "backend": "internal-dpll",
                "query": "warm",
                "elapsed_ns": 100,
                "insertion_ns": 0,
            },
            {
                **base,
                "backend": "cadical",
                "query": "cold",
                "elapsed_ns": 20,
                "insertion_ns": 50,
            },
            {
                **base,
                "backend": "cadical",
                "query": "warm",
                "elapsed_ns": 20,
                "insertion_ns": 50,
            },
        ]
        policies = select_dispatch.evaluate(records, "cadical")
        selected = next(
            policy for policy in policies if policy["threshold_clauses"] == 32
        )
        self.assertEqual(selected["total_cost_ns"], 90)
        self.assertEqual(selected["baseline_total_cost_ns"], 200)

    def test_workload_combination_deduplicates_capture_hash(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            inputs = []
            for index in range(2):
                source = root / f"source{index}"
                source.mkdir()
                session = source / f"s{index}.isat"
                session.write_text("p isat 0\nq cold -1 0 0\n", encoding="ascii")
                session_hash = combine_workloads.sha256(session)
                (source / "manifest.json").write_text(
                    json.dumps(
                        {
                            "sessions": [
                                {
                                    "session": session.name,
                                    "session_sha256": session_hash,
                                    "capture_sha256": "a" * 64,
                                    "clauses": 0,
                                    "problem_id": f"P{index}",
                                }
                            ]
                        }
                    ),
                    encoding="utf-8",
                )
                inputs.append(source)
            result = combine_workloads.combine(inputs, root / "combined")
            self.assertEqual(result["source_sessions"], 2)
            self.assertEqual(result["unique"], 1)


class DimacsConversionTests(unittest.TestCase):
    def test_converts_exact_query_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "sample.isat"
            path.write_text(
                "p isat 2\na 1 0\nq first -1 0 0\na -1 0\nq second -1 0 0\n",
                encoding="utf-8",
            )
            self.assertEqual(
                isat_to_dimacs.convert(path, "first"),
                "p cnf 2 1\n1 0\n",
            )

    def test_rejects_assumption_query(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            path = Path(raw) / "sample.isat"
            path.write_text(
                "p isat 1\na 1 0\nq first -1 0 -1 0\n",
                encoding="utf-8",
            )
            with self.assertRaises(ValueError):
                isat_to_dimacs.convert(path, "first")

    def test_strips_valid_rupd32_header(self) -> None:
        header = b"%RUPD32 2 3" + b" " * 20 + b"\n"
        self.assertEqual(
            strip_picosat_rupd32.strip_trace(header + b"1 0\n0\n", (2, 3)),
            b"1 0\n0\n",
        )

    def test_rejects_rupd32_shape_mismatch(self) -> None:
        with self.assertRaises(ValueError):
            strip_picosat_rupd32.strip_trace(b"%RUPD32 2 3\n0\n", (3, 2))


if __name__ == "__main__":
    unittest.main()
