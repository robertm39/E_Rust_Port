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

    def test_environment_with_path_prefix_prepends_existing_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            prefix = Path(directory)
            environment = e_interop.environment_with_path_prefix(prefix)

        self.assertTrue(environment["PATH"].startswith(str(prefix)))

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
            [
                "eground",
                "edpll",
                "epclanalyse",
                "epclextract",
                "CSSCPA_filter",
                "term2dag",
                "ex_commandline",
                "termprops",
                "tsm_classify",
            ]
        )

        self.assertEqual(
            [(case["name"], case["arguments"]) for case in cases],
            [
                ("CSSCPA_filter/help", ["--help"]),
                ("CSSCPA_filter/version", ["--version"]),
                ("edpll/help", ["--help"]),
                ("edpll/version", ["--version"]),
                ("edpll/lop-basic", ["--dimacs"]),
                ("edpll/tptp-input-clause", ["--tptp-in"]),
                ("eground/help", ["--help"]),
                ("eground/version", ["--version"]),
                ("epclanalyse/help", ["--help"]),
                ("epclanalyse/version", ["--version"]),
                ("epclanalyse/stdin-basic", []),
                ("epclextract/help", ["--help"]),
                ("epclextract/version", ["--version"]),
                ("epclextract/stdin-basic", []),
                ("ex_commandline/help", ["--help"]),
                (
                    "ex_commandline/options-basic",
                    ["--int_example=42", "--float_example", "one.p", "two.p"],
                ),
                ("term2dag/help", ["--help"]),
                ("term2dag/stdin-basic", []),
                ("termprops/help", ["--help"]),
                ("termprops/stdin-basic", []),
                ("tsm_classify/help", ["--help"]),
                ("tsm_classify/version", ["--version"]),
                (
                    "tsm_classify/stdin-basic",
                    ["--index-type=IndexIdentity", "--tsm-type=Flat"],
                ),
            ],
        )
        cases_by_name = {case["name"]: case for case in cases}
        edpll_lop_case = cases_by_name["edpll/lop-basic"]
        self.assertEqual(edpll_lop_case["stdin"], "p <- q. r <- r.")
        edpll_tptp_case = cases_by_name["edpll/tptp-input-clause"]
        self.assertEqual(
            edpll_tptp_case["stdin"], "input_clause(c_0_1,axiom,[++p,--q])."
        )
        epclanalyse_case = cases_by_name["epclanalyse/stdin-basic"]
        self.assertIn("[++q(a),--r(X)]", epclanalyse_case["stdin"])
        epclextract_case = cases_by_name["epclextract/stdin-basic"]
        self.assertIn("3 : : [] : 2 : 'final'", epclextract_case["stdin"])
        ex_case = cases_by_name["ex_commandline/options-basic"]
        self.assertIsNone(ex_case["stdin"])
        term2dag_case = cases_by_name["term2dag/stdin-basic"]
        self.assertEqual(term2dag_case["stdin"], "f(a,a) g(f(a,a))\n")
        termprops_case = cases_by_name["termprops/stdin-basic"]
        self.assertEqual(termprops_case["stdin"], "a f(a,a) g(f(a),a)\n")
        tsm_case = cases_by_name["tsm_classify/stdin-basic"]
        self.assertIn("Training:\n", tsm_case["stdin"])
        self.assertIn("Test:\n", tsm_case["stdin"])

    def test_tool_argument_cases_skip_version_for_simple_apps(self):
        self.assertEqual(e_interop.tool_argument_cases("term2dag"), (("--help",),))
        self.assertEqual(e_interop.tool_argument_cases("termprops"), (("--help",),))
        self.assertEqual(
            e_interop.tool_argument_cases("ex_commandline"),
            (("--help",),),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("classify_problem"),
            (("--help",), ("--version",)),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("edpll"),
            (("--help",), ("--version",)),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("epclanalyse"),
            (("--help",), ("--version",)),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("tsm_classify"),
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
        self.assertEqual(
            e_interop.REFERENCE_TOOL_BINARIES["termprops"],
            "PROVER/termprops",
        )
        self.assertIn("termprops", e_interop.ARCHIVED_REFERENCE_TOOL_LINKS)
        self.assertEqual(
            e_interop.REFERENCE_TOOL_BINARIES["tsm_classify"],
            "PROVER/tsm_classify",
        )
        self.assertIn("tsm_classify", e_interop.ARCHIVED_REFERENCE_TOOL_LINKS)

    def test_archived_reference_tool_source_patches_are_idempotent(self):
        with tempfile.TemporaryDirectory() as directory:
            build_dir = Path(directory)
            prover = build_dir / "PROVER"
            prover.mkdir()
            source = prover / "termprops.c"
            source.write_text(
                "ProblemType problemType  = PROBLEM_NOT_INIT;\n"
                "in = CreateScanner(StreamTypeFile, state->argv[i], true, NULL);\n",
                encoding="utf-8",
            )

            e_interop.apply_archived_reference_tool_source_patches(build_dir, "termprops")
            first = source.read_text(encoding="utf-8")
            e_interop.apply_archived_reference_tool_source_patches(build_dir, "termprops")
            second = source.read_text(encoding="utf-8")

        self.assertEqual(
            first,
            "/* problemType is provided by BASICS.a in current upstream. */\n"
            "in = CreateScanner(StreamTypeFile, state->argv[i], true, NULL, true);\n",
        )
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
