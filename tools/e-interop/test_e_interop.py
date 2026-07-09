import math
from pathlib import Path
import tempfile
import unittest

import e_interop


class OutputParsingTests(unittest.TestCase):
    def test_extracts_last_szs_status(self):
        output = "# SZS status Started for x\n# SZS status Theorem for x\n"
        self.assertEqual(e_interop.szs_status(output), "Theorem")

    def test_extracts_expected_status(self):
        text = "% File: sample\n% Status   : CounterSatisfiable\n"
        self.assertEqual(e_interop.expected_status(text), "CounterSatisfiable")

    def test_normalization_removes_timing_and_rewrites_paths(self):
        output = "problem /tmp/a.p\n# Total time : 1.23 s\nproof\r\n"
        self.assertEqual(
            e_interop.normalize_output(output, [("/tmp/a.p", "<PROBLEM>")]),
            "problem <PROBLEM>\nproof",
        )

    def test_normalization_sorts_only_saturation_blocks(self):
        saturation_a = (
            "% SZS output start Saturation\n"
            "cnf(c_0_2, plain, p, inference(spm,[status(thm)],[c_0_1])).\n"
            "cnf(c_0_3, plain, q, inference(spm,[status(thm)],[c_0_1]))."
        )
        saturation_b = (
            "% SZS output start Saturation\n"
            "cnf(c_0_11, plain, q, inference(spm,[status(thm)],[c_0_9])).\n"
            "cnf(c_0_10, plain, p, inference(spm,[status(thm)],[c_0_8]))."
        )
        self.assertEqual(
            e_interop.normalize_output(saturation_a),
            e_interop.normalize_output(saturation_b),
        )

        proof_a = "% SZS output start CNFRefutation\ncnf(step2, plain, $true).\ncnf(step1, plain, $true)."
        proof_b = "% SZS output start CNFRefutation\ncnf(step1, plain, $true).\ncnf(step2, plain, $true)."
        self.assertNotEqual(
            e_interop.normalize_output(proof_a),
            e_interop.normalize_output(proof_b),
        )

    def test_output_shape_tracks_proof_markers(self):
        shape = e_interop.output_shape(
            "# SZS status Theorem\n# SZS output start CNFRefutation\n# SZS output end CNFRefutation\n",
            "",
        )
        self.assertEqual(shape["szs_status_count"], 1)
        self.assertEqual(shape["proof_start_count"], 1)
        self.assertEqual(shape["proof_end_count"], 1)
        self.assertFalse(shape["stderr_nonempty"])


class ComparisonTests(unittest.TestCase):
    def test_matching_results_have_no_mismatches(self):
        result = {
            "exit_code": 0,
            "timed_out": False,
            "status": "Theorem",
            "shape": {"stdout_nonempty": True},
        }
        self.assertEqual(e_interop.comparison_mismatches(result, dict(result)), [])

    def test_exit_status_and_shape_mismatches_are_reported(self):
        reference = {
            "exit_code": 0,
            "timed_out": False,
            "status": "Theorem",
            "shape": {"stdout_nonempty": True},
        }
        candidate = {
            "exit_code": 1,
            "timed_out": False,
            "status": "GaveUp",
            "shape": {"stdout_nonempty": False},
        }
        self.assertEqual(
            e_interop.comparison_mismatches(reference, candidate),
            ["exit_code", "status", "shape"],
        )

    def test_geometric_mean(self):
        self.assertTrue(math.isclose(e_interop.geometric_mean([0.5, 2.0]), 1.0))
        self.assertIsNone(e_interop.geometric_mean([]))

    def test_corpus_detects_higher_order_problem_syntax(self):
        with tempfile.TemporaryDirectory() as directory:
            corpus = Path(directory)
            (corpus / "sample.p").write_text(
                "thf(goal, conjecture, $true).\n", encoding="utf-8"
            )
            cases = e_interop.enumerate_problems(corpus, corpus)
        self.assertEqual(len(cases), 1)
        self.assertEqual(cases[0]["mode"], "ho")

    def test_tool_cases_default_to_help_for_sorted_tools(self):
        cases = e_interop.tool_comparison_cases(
            ["eground", "CSSCPA_filter", "term2dag"]
        )

        self.assertEqual(
            [(case["name"], case["arguments"]) for case in cases],
            [
                ("CSSCPA_filter/help", ["--help"]),
                ("CSSCPA_filter/version", ["--version"]),
                ("eground/help", ["--help"]),
                ("eground/version", ["--version"]),
                ("term2dag/help", ["--help"]),
            ],
        )

    def test_tool_argument_cases_skip_version_for_simple_apps(self):
        self.assertEqual(e_interop.tool_argument_cases("term2dag"), (("--help",),))
        self.assertEqual(
            e_interop.tool_argument_cases("ex_commandline"),
            (("--help",),),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("classify_problem"),
            (("--help",), ("--version",)),
        )

    def test_reference_tool_inventory_contains_ported_support_binaries(self):
        self.assertEqual(
            e_interop.REFERENCE_TOOL_BINARIES["CSSCPA_filter"],
            "EXTERNAL/CSSCPA_filter",
        )
        self.assertEqual(
            e_interop.REFERENCE_TOOL_BINARIES["classify_problem"],
            "PROVER/classify_problem",
        )
        self.assertEqual(
            e_interop.REFERENCE_TOOL_BINARIES["ex_commandline"],
            "SIMPLE_APPS/ex_commandline",
        )


if __name__ == "__main__":
    unittest.main()
