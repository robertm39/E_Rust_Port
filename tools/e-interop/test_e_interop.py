import math
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

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

    def test_cross_platform_path_replacements_cover_windows_separator_forms(self):
        path = Path("reference-root")
        with patch.object(e_interop, "wslpath", return_value=r"C:\work\TPTP"):
            replacements = e_interop.cross_platform_path_replacements(path, "<TPTP>")

        self.assertEqual(
            set(replacements),
            {
                (r"C:\work\TPTP", "<TPTP>"),
                ("C:/work/TPTP", "<TPTP>"),
                (str(path), "<TPTP>"),
            },
        )
        self.assertEqual(
            e_interop.normalize_output(
                "C:/work/TPTP/Axioms/SET001.ax",
                replacements,
            ),
            "<TPTP>/Axioms/SET001.ax",
        )

    def test_normalization_canonicalizes_platform_error_and_nan_spellings(self):
        linux = (
            "direct_examples: No such file or directory\n"
            " 0 terms, 0 successes,  -nan percent\n"
            "% Terms: 0  ASize: -nan MSize: 0, ADepth: -nan MDepth: 0\n"
            "direct_examples: Broken pipe\n"
        )
        windows = (
            "direct_examples: The system cannot find the file specified. (os error 2)\n"
            " 0 terms, 0 successes,   NaN percent\n"
            "% Terms: 0  ASize: nan MSize: 0, ADepth: nan MDepth: 0\n"
            "direct_examples: The pipe is being closed. (os error 232)\n"
        )

        self.assertEqual(
            e_interop.normalize_output(linux),
            e_interop.normalize_output(windows),
        )

        self.assertEqual(
            e_interop.normalize_output(
                "ex_commandline: Numerical result out of range\n"
            ),
            e_interop.normalize_output("ex_commandline: Result too large\n"),
        )

        legacy_msvcrt = (
            " 0 terms, 0 successes,  -1.#IND00 percent\n"
            "% Terms: 0  ASize: -1.#IND00 MSize: 0, ADepth: 1.#QNAN0 MDepth: 0\n"
        )
        portable = (
            " 0 terms, 0 successes,  nan percent\n"
            "% Terms: 0  ASize: nan MSize: 0, ADepth: nan MDepth: 0\n"
        )
        self.assertEqual(
            e_interop.normalize_output(legacy_msvcrt),
            e_interop.normalize_output(portable),
        )

    def test_classify_legacy_feature_suffix_normalization_is_explicit(self):
        reference = (
            "prob : (   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,"
            "  12,  13,  14,  15, -0.714286, 0.933333,  16,  17,  18,  19,  20,"
            " 22319, 32767, 0.000000, 0.000000, true, false ) :"
            " FUHS-NFFSM3-MDHHMFFBN\n"
        )
        candidate = (
            "prob : (   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,"
            "  12,  13,  14,  15, -0.714286, 0.933333,  16,  17,  18,  19,  20,"
            "   0,   0, 0.000000, 0.000000, false, false ) :"
            " FUHS-NFFSM3-MDFFFFFNN\n"
        )

        self.assertNotEqual(
            e_interop.normalize_output(reference),
            e_interop.normalize_output(candidate),
        )
        self.assertEqual(
            e_interop.normalize_output(
                reference, normalize_legacy_classify_feature_suffix=True
            ),
            e_interop.normalize_output(
                candidate, normalize_legacy_classify_feature_suffix=True
            ),
        )

    def test_normalization_canonicalizes_only_successful_server_descriptors(self):
        output = "Main loop\nAccepted 123\nAccepted -1\n"

        self.assertEqual(
            e_interop.normalize_output(output),
            "Main loop\nAccepted <DESCRIPTOR>\nAccepted -1",
        )

    def test_termprops_nan_normalization_is_summary_field_scoped(self):
        output = (
            "% unrelated ASize: -nan, ADepth: nan\n"
            "% Terms: 0  ASize: -nan MSize: 0, ADepth: nan(ind) MDepth: 0\n"
        )

        self.assertEqual(
            e_interop.normalize_output(output),
            "% unrelated ASize: -nan, ADepth: nan\n"
            "% Terms: 0  ASize: <NAN> MSize: 0, ADepth: <NAN> MDepth: 0",
        )

    def test_epclanalyse_nan_normalization_is_average_field_scoped(self):
        glibc = (
            "% Average number of literals         :   -nan\n"
            "% ...in negative clauses             :   -nan\n"
            "% unrelated metric                   :   -nan\n"
        )
        rust = (
            "% Average number of literals         :    NaN\n"
            "% ...in negative clauses             :    NaN\n"
            "% unrelated metric                   :   -nan\n"
        )

        self.assertEqual(
            e_interop.normalize_output(glibc),
            e_interop.normalize_output(rust),
        )
        legacy_msvcrt = glibc.replace("-nan", "-1.#IND00", 2)
        self.assertEqual(
            e_interop.normalize_output(legacy_msvcrt),
            e_interop.normalize_output(rust),
        )
        self.assertIn(
            "% unrelated metric                   :   -nan",
            e_interop.normalize_output(glibc),
        )

    def test_checkproof_temp_normalization_is_trace_line_scoped(self):
        linux = (
            "% Running eprover --cpu-limit=10 /tmp/epr_A1b2C3\n"
            "%> marker /tmp/epr_A1b2C3\n"
            "unrelated /tmp/epr_A1b2C3\n"
        )
        windows = (
            "% Running eprover --cpu-limit=10 C:\\Temp\\epr_d4E5f6\n"
            "%> marker C:\\Temp\\epr_d4E5f6\n"
            "unrelated /tmp/epr_A1b2C3\n"
        )

        self.assertEqual(
            e_interop.normalize_output(linux),
            e_interop.normalize_output(windows),
        )
        self.assertIn("unrelated /tmp/epr_A1b2C3", e_interop.normalize_output(linux))

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

    def test_normalization_canonicalizes_app_encoded_type_declaration_order(self):
        declarations_a = (
            "%-- $i > $o.\n"
            "tff(typedecl1, type, type_9: $tType).\n"
            "%-- $i > $i.\n"
            "tff(typedecl2, type, type_10: $tType).\n"
            "tff(symboltypedecl2, type, p: type_9)."
        )
        declarations_b = (
            "%-- $i > $i.\n"
            "tff(typedecl1, type, type_10: $tType).\n"
            "%-- $i > $o.\n"
            "tff(typedecl2, type, type_9: $tType).\n"
            "tff(symboltypedecl2, type, p: type_9)."
        )

        normalized = e_interop.normalize_output(declarations_a)
        self.assertEqual(normalized, e_interop.normalize_output(declarations_b))
        self.assertIn("tff(typedecl1, type, type_9: $tType).", normalized)
        self.assertIn("tff(typedecl2, type, type_10: $tType).", normalized)

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

    def test_default_comparison_cases_include_syntax_only_socrates(self):
        with tempfile.TemporaryDirectory() as directory:
            repo_root = Path(directory)
            smoketest = repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "SMOKETEST"
            (repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "TPTP").mkdir(
                parents=True
            )
            (repo_root / "eprover" / "EXAMPLE_PROBLEMS" / "LFHOL").mkdir(
                parents=True
            )
            smoketest.mkdir(parents=True)
            (smoketest / "socrates.p").write_text(
                "% Status   : Theorem\nfof(goal, conjecture, p(a)).\n",
                encoding="utf-8",
            )
            run_dir = repo_root / "run"
            run_dir.mkdir()

            cases = e_interop.comparison_cases(repo_root, None, run_dir)

        by_name = {case["name"]: case for case in cases}
        syntax_case = by_name["synthetic/syntax-only-socrates.p"]
        self.assertEqual(syntax_case["arguments"], ("--syntax-only",))
        self.assertEqual(syntax_case["expected_status"], "Unknown")
        self.assertEqual(syntax_case["scenario"], "syntax-only")
        self.assertIsNone(syntax_case["stdin"])
        stdin_syntax_case = by_name["synthetic/stdin-syntax-only-socrates.p"]
        self.assertEqual(stdin_syntax_case["arguments"], ("--syntax-only",))
        self.assertEqual(stdin_syntax_case["expected_status"], "Unknown")
        self.assertEqual(stdin_syntax_case["scenario"], "stdin-syntax-only")
        self.assertIn("fof(goal, conjecture, p(a)).", stdin_syntax_case["stdin"])
        print_case = by_name["synthetic/print-formulas-socrates.p"]
        self.assertEqual(print_case["arguments"], ("--print-formulas",))
        self.assertIsNone(print_case["expected_status"])
        self.assertEqual(print_case["scenario"], "print-formulas")
        self.assertIsNone(print_case["stdin"])
        stdin_print_case = by_name["synthetic/stdin-print-formulas-socrates.p"]
        self.assertEqual(stdin_print_case["arguments"], ("--print-formulas",))
        self.assertIsNone(stdin_print_case["expected_status"])
        self.assertEqual(stdin_print_case["scenario"], "stdin-print-formulas")
        self.assertIn("fof(goal, conjecture, p(a)).", stdin_print_case["stdin"])
        prune_case = by_name["synthetic/prune-socrates.p"]
        self.assertEqual(prune_case["arguments"], ("--prune",))
        self.assertEqual(prune_case["expected_status"], "Unknown")
        self.assertEqual(prune_case["scenario"], "prune")
        self.assertIsNone(prune_case["stdin"])
        stdin_prune_case = by_name["synthetic/stdin-prune-socrates.p"]
        self.assertEqual(stdin_prune_case["arguments"], ("--prune",))
        self.assertEqual(stdin_prune_case["expected_status"], "Unknown")
        self.assertEqual(stdin_prune_case["scenario"], "stdin-prune")
        self.assertIn("fof(goal, conjecture, p(a)).", stdin_prune_case["stdin"])
        cnf_case = by_name["synthetic/cnf-socrates.p"]
        self.assertEqual(cnf_case["arguments"], ("--cnf",))
        self.assertEqual(cnf_case["expected_status"], "Unknown")
        self.assertEqual(cnf_case["scenario"], "cnf")
        self.assertIsNone(cnf_case["stdin"])
        stdin_cnf_case = by_name["synthetic/stdin-cnf-socrates.p"]
        self.assertEqual(stdin_cnf_case["arguments"], ("--cnf",))
        self.assertEqual(stdin_cnf_case["expected_status"], "Unknown")
        self.assertEqual(stdin_cnf_case["scenario"], "stdin-cnf")
        self.assertIn("fof(goal, conjecture, p(a)).", stdin_cnf_case["stdin"])
        app_encode_case = by_name["synthetic/app-encode-socrates.p"]
        self.assertEqual(app_encode_case["arguments"], ("--app-encode",))
        self.assertIsNone(app_encode_case["expected_status"])
        self.assertEqual(app_encode_case["scenario"], "app-encode")
        self.assertIsNone(app_encode_case["stdin"])
        stdin_app_encode_case = by_name["synthetic/stdin-app-encode-socrates.p"]
        self.assertEqual(stdin_app_encode_case["arguments"], ("--app-encode",))
        self.assertIsNone(stdin_app_encode_case["expected_status"])
        self.assertEqual(stdin_app_encode_case["scenario"], "stdin-app-encode")
        self.assertIn("fof(goal, conjecture, p(a)).", stdin_app_encode_case["stdin"])

    def test_tool_cases_default_to_help_for_sorted_tools(self):
        cases = e_interop.tool_comparison_cases(
            [
                "checkproof",
                "classify_problem",
                "direct_examples",
                "e_axfilter",
                "e_client",
                "e_deduction_server",
                "e_ltb_runner",
                "e_server",
                "ekb_create",
                "ekb_delete",
                "ekb_ginsert",
                "ekb_insert",
                "e_stratpar",
                "eground",
                "edpll",
                "enormalizer",
                "epatternize",
                "epclanalyse",
                "epclextract",
                "epcllemma",
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
                ("CSSCPA_filter/silent-accept", ["--silent"]),
                ("CSSCPA_filter/trace-state-check", []),
                ("CSSCPA_filter/large-stateful-corpus", []),
                (
                    "CSSCPA_filter/missing-input",
                    ["missing-csscpa-input.csscpa"],
                ),
                ("checkproof/help", ["--help"]),
                ("checkproof/version", ["--version"]),
                ("checkproof/assumption-only", []),
                (
                    "checkproof/setheo-release-failure",
                    ["--prover-type=scheme-setheo"],
                ),
                (
                    "checkproof/real-e-success",
                    ['--executable="{companion:eprover}"'],
                ),
                (
                    "checkproof/real-e-failure",
                    ['--executable="{companion:eprover}"'],
                ),
                (
                    "checkproof/e-shell-success",
                    ["--output-level=3", "--executable=echo % Proof found!"],
                ),
                (
                    "checkproof/e-shell-failure",
                    ["--output-level=3", "--executable=echo NO-PROOF"],
                ),
                (
                    "checkproof/otter-shell-failure",
                    ["--prover-type=Otter", "--executable=echo NO-PROOF"],
                ),
                (
                    "checkproof/otter-shell-success",
                    [
                        "--prover-type=Otter",
                        "--executable=echo -------- PROOF --------",
                    ],
                ),
                (
                    "checkproof/spass-shell-failure",
                    ["--prover-type=SPASS", "--executable=echo NO-PROOF"],
                ),
                (
                    "checkproof/spass-shell-success",
                    ["--prover-type=SPASS", "--executable=echo Proof found."],
                ),
                (
                    "checkproof/fof-warning-setheo",
                    ["--prover-type=scheme-setheo"],
                ),
                ("checkproof/shell-step-rejection", []),
                (
                    "checkproof/missing-input",
                    ["missing-checkproof-input.pcl"],
                ),
                ("classify_problem/help", ["--help"]),
                ("classify_problem/version", ["--version"]),
                ("classify_problem/parse-features-standard", ["--parse-features"]),
                (
                    "classify_problem/parse-features-raw",
                    ["--parse-features", "--raw-class"],
                ),
                (
                    "classify_problem/parse-features-missing-colon",
                    ["--parse-features"],
                ),
                (
                    "classify_problem/parse-features-short-class",
                    ["--parse-features"],
                ),
                (
                    "classify_problem/parse-features-raw-short-class",
                    ["--parse-features", "--raw-class"],
                ),
                (
                    "classify_problem/parse-features-output-file",
                    ["--parse-features", "-o", "features.out"],
                ),
                (
                    "classify_problem/raw-lop",
                    ["--raw-class", "--lop-in"],
                ),
                ("classify_problem/old-tptp-records", ["--tptp-in"]),
                (
                    "classify_problem/raw-fof-definition-conjecture",
                    ["--raw-class", "--tstp-format"],
                ),
                (
                    "classify_problem/standard-fof-definition-conjecture",
                    ["--tstp-format"],
                ),
                (
                    "classify_problem/tstp-first-order-record-mix",
                    ["--tstp-format"],
                ),
                ("classify_problem/fool-term-let", ["--tstp-format"]),
                (
                    "classify_problem/raw-thf",
                    ["--raw-class", "--tstp-format"],
                ),
                (
                    "classify_problem/specsig-mixed-arities",
                    ["--tstp-format", "--specsig"],
                ),
                (
                    "classify_problem/tptp-header-mixed-shape",
                    ["--tstp-format", "--generate-tptp-header"],
                ),
                ("classify_problem/include-selector", ["main.p"]),
                (
                    "classify_problem/merged-positive-cnf",
                    ["--tstp-format", "--merged-classification=2"],
                ),
                (
                    "classify_problem/merged-positive-fool",
                    ["--tstp-format", "--merged-classification=2"],
                ),
                (
                    "classify_problem/merged-zero-fallback",
                    ["--tstp-format", "--merged-classification=0"],
                ),
                (
                    "classify_problem/merged-negative-unbounded",
                    ["--tstp-format", "--merged-classification=-2"],
                ),
                (
                    "classify_problem/merged-minus-one-standard",
                    ["--tstp-format", "--merged-classification=-1"],
                ),
                (
                    "classify_problem/merged-positive-thf",
                    ["--tstp-format", "--merged-classification=2"],
                ),
                (
                    "classify_problem/missing-feature-input",
                    ["--parse-features", "missing-classify-features.txt"],
                ),
                (
                    "classify_problem/missing-real-input",
                    ["--tstp-format", "missing-classify-problem.p"],
                ),
                (
                    "classify_problem/missing-output-parent",
                    ["--parse-features", "-o", "missing/features.out"],
                ),
                ("direct_examples/help", ["--help"]),
                ("direct_examples/version", ["--version"]),
                ("direct_examples/stdin-basic", []),
                (
                    "direct_examples/branching-protocol",
                    ["--negative-example-proportion=1.5", "--negative-example-number=12"],
                ),
                ("direct_examples/missing-input", ["missing-learning-input.pcl"]),
                ("e_axfilter/help", ["--help"]),
                ("e_axfilter/version", ["--version"]),
                ("e_axfilter/dump-filter-stdout", ["--dump-filter", "-o", "-"]),
                (
                    "e_axfilter/tstp-threshold-file",
                    ["--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"],
                ),
                (
                    "e_axfilter/tstp-gsine-formulas",
                    ["--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"],
                ),
                (
                    "e_axfilter/tstp-lambda-def-formulas",
                    ["--tstp-in", "-f", "filters.axf", "-o", "global.out", "problem.p"],
                ),
                (
                    "e_axfilter/tstp-seeded-all-methods",
                    [
                        "--tstp-in",
                        "-f",
                        "filters.axf",
                        "--seed-method=lda",
                        "--seeds=p",
                        "-o",
                        "global.out",
                        "problem.p",
                    ],
                ),
                (
                    "e_axfilter/output-open-missing-parent",
                    ["-o", "missing/global.out", "problem.p"],
                ),
                (
                    "e_axfilter/filter-open-missing",
                    ["-f", "missing.axf", "problem.p"],
                ),
                ("e_client/help", ["--help"]),
                ("e_client/version", ["--version"]),
                ("e_client/invalid-port", ["--port=70000"]),
                ("e_deduction_server/help", ["--help"]),
                ("e_deduction_server/version", ["--version"]),
                ("e_deduction_server/stdout-unimplemented", []),
                ("e_ltb_runner/help", ["--help"]),
                ("e_ltb_runner/version", ["--version"]),
                ("e_ltb_runner/usage-missing-spec", []),
                ("e_server/help", ["--help"]),
                ("e_server/version", ["--version"]),
                ("e_server/usage-missing-domain", []),
                ("e_stratpar/help", ["--help"]),
                ("e_stratpar/version", ["--version"]),
                ("e_stratpar/usage-missing-problem", []),
                ("edpll/help", ["--help"]),
                ("edpll/version", ["--version"]),
                ("edpll/lop-basic", ["--dimacs"]),
                ("edpll/tptp-input-clause", ["--tptp-in"]),
                ("eground/help", ["--help"]),
                ("eground/version", ["--version"]),
                ("eground/lop-basic", ["--lop-in", "--silent"]),
                ("ekb_create/help", ["--help"]),
                ("ekb_create/version", ["--version"]),
                (
                    "ekb_create/empty-kb-files",
                    [
                        "--negative-example-number=7",
                        "--negative-example-proportion=0.5",
                        "kb",
                    ],
                ),
                ("ekb_delete/help", ["--help"]),
                ("ekb_delete/version", ["--version"]),
                ("ekb_delete/drop-example", ["--knowledge-base=kb", "drop"]),
                (
                    "ekb_delete/drop-middle-example",
                    ["--knowledge-base=kb", "middle"],
                ),
                ("ekb_ginsert/help", ["--help"]),
                ("ekb_ginsert/version", ["--version"]),
                ("ekb_ginsert/stdin-protocol", ["--knowledge-base=kb"]),
                ("ekb_insert/help", ["--help"]),
                ("ekb_insert/version", ["--version"]),
                ("ekb_insert/stdin-example", ["--knowledge-base=kb"]),
                ("enormalizer/help", ["--help"]),
                ("enormalizer/version", ["--version"]),
                (
                    "enormalizer/term-basic",
                    ["-t", "{fixture:terms.lop}", "{fixture:rules.lop}"],
                ),
                ("epatternize/help", ["--help"]),
                ("epatternize/version", ["--version"]),
                ("epatternize/lop-basic", ["--lop-in"]),
                ("epclanalyse/help", ["--help"]),
                ("epclanalyse/version", ["--version"]),
                ("epclanalyse/stdin-basic", []),
                ("epclanalyse/zero-denominator-safe-boundary", []),
                (
                    "epclanalyse/missing-input",
                    ["missing-epclanalyse-input.pcl"],
                ),
                ("epclextract/help", ["--help"]),
                ("epclextract/version", ["--version"]),
                ("epclextract/stdin-basic", []),
                ("epclextract/mixed-logic-proof-closure", []),
                (
                    "epclextract/multi-file-comments",
                    [
                        "--forward-comments",
                        "{fixture:first.pcl}",
                        "{fixture:second.pcl}",
                    ],
                ),
                (
                    "epclextract/missing-input",
                    ["missing-epclextract-input.pcl"],
                ),
                ("epcllemma/help", ["--help"]),
                ("epcllemma/version", ["--version"]),
                (
                    "epcllemma/stdin-basic",
                    ["--max-lemmas=0", "--min-lemma-quality=0"],
                ),
                (
                    "epcllemma/large-relative-limit",
                    ["--min-lemma-quality=0"],
                ),
                (
                    "epcllemma/formula-lemma-pcl",
                    ["--max-lemmas=0", "--min-lemma-quality=0"],
                ),
                (
                    "epcllemma/formula-lemma-tptp",
                    [
                        "--max-lemmas=0",
                        "--min-lemma-quality=0",
                        "--tptp-out",
                    ],
                ),
                (
                    "epcllemma/formula-lemma-tstp",
                    [
                        "--max-lemmas=0",
                        "--min-lemma-quality=0",
                        "--tstp-out",
                    ],
                ),
                (
                    "epcllemma/formula-lemma-lop",
                    [
                        "--max-lemmas=0",
                        "--min-lemma-quality=0",
                        "--lop-out",
                    ],
                ),
                (
                    "epcllemma/minimum-quality-nan",
                    ["--min-lemma-quality=nan"],
                ),
                (
                    "epcllemma/minimum-quality-positive-infinity",
                    ["--min-lemma-quality=inf"],
                ),
                (
                    "epcllemma/minimum-quality-negative-infinity",
                    ["--min-lemma-quality=-inf"],
                ),
                (
                    "epcllemma/minimum-quality-negative-zero",
                    ["--min-lemma-quality=-0"],
                ),
                ("epcllemma/shell-step-rejection", []),
                (
                    "epcllemma/missing-input",
                    ["missing-epcllemma-input.pcl"],
                ),
                (
                    "epcllemma/missing-output-parent",
                    ["--output-file=missing-parent/lemmas.pcl"],
                ),
                ("ex_commandline/help", ["--help"]),
                (
                    "ex_commandline/options-basic",
                    ["--int_example=42", "--float_example", "one.p", "two.p"],
                ),
                ("ex_commandline/unknown-long-option", ["--unknown"]),
                ("ex_commandline/missing-required-argument", ["--int_example"]),
                ("ex_commandline/invalid-integer", ["--int_example=bad"]),
                (
                    "ex_commandline/integer-range",
                    ["--int_example=9223372036854775808"],
                ),
                ("ex_commandline/float-range", ["--float_example=1e9999"]),
                ("term2dag/help", ["--help"]),
                ("term2dag/stdin-basic", []),
                ("term2dag/shared-typed-boundary", []),
                ("term2dag/missing-input", ["missing-term2dag-input"]),
                ("termprops/help", ["--help"]),
                ("termprops/stdin-basic", []),
                ("termprops/empty-input", []),
                ("termprops/missing-input", ["missing-termprops-input"]),
                ("tsm_classify/help", ["--help"]),
                ("tsm_classify/version", ["--version"]),
                (
                    "tsm_classify/stdin-basic",
                    ["--index-type=IndexIdentity", "--tsm-type=Flat"],
                ),
                (
                    "tsm_classify/recursive-mixed",
                    [
                        "--index-type=IndexSymbol",
                        "--index-depth=3",
                        "--tsm-type=Recursive",
                    ],
                ),
                (
                    "tsm_classify/empty-test-set",
                    ["--index-type=IndexIdentity", "--tsm-type=Flat"],
                ),
            ],
        )
        cases_by_name = {case["name"]: case for case in cases}
        csscpa_case = cases_by_name["CSSCPA_filter/silent-accept"]
        self.assertEqual(
            csscpa_case["stdin"], "accept: cnf(csscpa_unit,axiom,p(a)).\n"
        )
        csscpa_trace_case = cases_by_name["CSSCPA_filter/trace-state-check"]
        self.assertIn("output_level 0\nstate:\noutput_level 1", csscpa_trace_case["stdin"])
        self.assertIn("check improve(0.0,0.0)", csscpa_trace_case["stdin"])
        self.assertIn("great shining CSSCPA", csscpa_trace_case["stdin"])
        csscpa_large_case = cases_by_name["CSSCPA_filter/large-stateful-corpus"]
        self.assertEqual(
            csscpa_large_case["stdin"], e_interop.CSSCPA_LARGE_STATEFUL_CORPUS
        )
        self.assertEqual(csscpa_large_case["stdin"].count("accept"), 40)
        self.assertEqual(csscpa_large_case["stdin"].count("check"), 32)
        self.assertIn("csscpa_tautology_3", csscpa_large_case["stdin"])
        self.assertIn("csscpa_contradiction_3", csscpa_large_case["stdin"])
        self.assertIn("csscpa_weighty_3", csscpa_large_case["stdin"])
        csscpa_missing_case = cases_by_name["CSSCPA_filter/missing-input"]
        self.assertTrue(csscpa_missing_case["isolated_workdir"])
        self.assertEqual(
            csscpa_missing_case["arguments"], ["missing-csscpa-input.csscpa"]
        )
        checkproof_case = cases_by_name["checkproof/assumption-only"]
        self.assertEqual(checkproof_case["stdin"], "1 : : [++p(a)] : initial\n")
        checkproof_setheo_case = cases_by_name[
            "checkproof/setheo-release-failure"
        ]
        self.assertIn("3 : : [++r(a)] : split(2)", checkproof_setheo_case["stdin"])
        checkproof_success_case = cases_by_name["checkproof/e-shell-success"]
        self.assertIn("[++p(X),--q(f(X))]", checkproof_success_case["stdin"])
        self.assertEqual(
            checkproof_success_case["arguments"],
            ["--output-level=3", "--executable=echo % Proof found!"],
        )
        checkproof_real_e_case = cases_by_name["checkproof/real-e-success"]
        self.assertEqual(
            checkproof_real_e_case["arguments"],
            ['--executable="{companion:eprover}"'],
        )
        self.assertIn("2 : : [++p(a)] : 1", checkproof_real_e_case["stdin"])
        self.assertEqual(
            cases_by_name["checkproof/otter-shell-failure"]["arguments"],
            ["--prover-type=Otter", "--executable=echo NO-PROOF"],
        )
        self.assertEqual(
            cases_by_name["checkproof/otter-shell-success"]["arguments"],
            [
                "--prover-type=Otter",
                "--executable=echo -------- PROOF --------",
            ],
        )
        self.assertEqual(
            cases_by_name["checkproof/spass-shell-failure"]["arguments"],
            ["--prover-type=SPASS", "--executable=echo NO-PROOF"],
        )
        self.assertEqual(
            cases_by_name["checkproof/spass-shell-success"]["arguments"],
            ["--prover-type=SPASS", "--executable=echo Proof found."],
        )
        checkproof_fof_case = cases_by_name["checkproof/fof-warning-setheo"]
        self.assertIn("1 : : p(a) : initial", checkproof_fof_case["stdin"])
        checkproof_missing_case = cases_by_name["checkproof/missing-input"]
        self.assertTrue(checkproof_missing_case["isolated_workdir"])
        self.assertEqual(
            checkproof_missing_case["arguments"],
            ["missing-checkproof-input.pcl"],
        )
        classify_case = cases_by_name["classify_problem/parse-features-standard"]
        self.assertIn("prob : (1,2,3,4,5,6,7,8,9,10", classify_case["stdin"])
        self.assertTrue(classify_case["normalize_legacy_classify_feature_suffix"])
        classify_raw_feature_case = cases_by_name[
            "classify_problem/parse-features-raw"
        ]
        self.assertIn("FSSMMLLCCSSNAA", classify_raw_feature_case["stdin"])
        self.assertEqual(
            cases_by_name["classify_problem/parse-features-missing-colon"]["stdin"],
            "broken\n",
        )
        self.assertTrue(
            cases_by_name["classify_problem/parse-features-output-file"][
                "normalize_legacy_classify_feature_suffix"
            ]
        )
        self.assertEqual(
            cases_by_name["classify_problem/parse-features-output-file"][
                "output_files"
            ],
            ["features.out"],
        )
        self.assertEqual(
            cases_by_name["classify_problem/raw-lop"]["stdin"],
            "p(a).\nq(a).\n",
        )
        self.assertIn(
            "input_formula(f1,axiom,p(a))",
            cases_by_name["classify_problem/old-tptp-records"]["stdin"],
        )
        self.assertIn(
            "fof(goal,conjecture,?[X]:p(f(X)))",
            cases_by_name[
                "classify_problem/raw-fof-definition-conjecture"
            ]["stdin"],
        )
        self.assertIn(
            "tcf(c1,axiom,![X:person]:p(X))",
            cases_by_name["classify_problem/tstp-first-order-record-mix"]["stdin"],
        )
        self.assertIn(
            "$let(f:$i,f:=a,f)",
            cases_by_name["classify_problem/fool-term-let"]["stdin"],
        )
        self.assertIn(
            "thf(fact,axiom,p@a)",
            cases_by_name["classify_problem/raw-thf"]["stdin"],
        )
        self.assertIn(
            "negated_conjecture",
            cases_by_name["classify_problem/specsig-mixed-arities"]["stdin"],
        )
        self.assertIn(
            "q(f(a),X)",
            cases_by_name["classify_problem/tptp-header-mixed-shape"]["stdin"],
        )
        classify_include_case = cases_by_name[
            "classify_problem/include-selector"
        ]
        self.assertIsNone(classify_include_case["stdin"])
        self.assertIn("selected.p", classify_include_case["workdir_files"])
        self.assertIn(
            "[selected]", classify_include_case["workdir_files"]["main.p"]
        )
        self.assertIn(
            "$let(f:$i,f:=a,f)",
            cases_by_name["classify_problem/merged-positive-fool"]["stdin"],
        )
        self.assertEqual(
            cases_by_name["classify_problem/merged-negative-unbounded"][
                "arguments"
            ],
            ["--tstp-format", "--merged-classification=-2"],
        )
        self.assertIn(
            "thf(fact,axiom,p@a)",
            cases_by_name["classify_problem/merged-positive-thf"]["stdin"],
        )
        self.assertTrue(
            cases_by_name["classify_problem/missing-feature-input"][
                "isolated_workdir"
            ]
        )
        self.assertTrue(
            cases_by_name["classify_problem/missing-real-input"][
                "isolated_workdir"
            ]
        )
        self.assertEqual(
            cases_by_name["classify_problem/missing-output-parent"][
                "output_absent_files"
            ],
            ["missing/features.out"],
        )
        direct_examples_case = cases_by_name["direct_examples/stdin-basic"]
        self.assertIn("2 : : [++q(a)] : 1", direct_examples_case["stdin"])
        branching_case = cases_by_name["direct_examples/branching-protocol"]
        self.assertIn("10 : : [] : 9 : 'final'", branching_case["stdin"])
        self.assertIn("12 : : [++m(a)] : 11", branching_case["stdin"])
        missing_input_case = cases_by_name["direct_examples/missing-input"]
        self.assertTrue(missing_input_case["isolated_workdir"])
        e_axfilter_case = cases_by_name["e_axfilter/dump-filter-stdout"]
        self.assertIsNone(e_axfilter_case["stdin"])
        e_axfilter_generated_case = cases_by_name["e_axfilter/tstp-threshold-file"]
        self.assertEqual(
            e_axfilter_generated_case["workdir_files"],
            {
                "filters.axf": "tiny=Threshold(10000)\n",
                "problem.p": "fof(a, axiom, p(a)).\n",
            },
        )
        self.assertEqual(
            e_axfilter_generated_case["output_files"],
            ["global.out", "problem_tiny.p"],
        )
        e_axfilter_gsine_case = cases_by_name["e_axfilter/tstp-gsine-formulas"]
        self.assertIn("fof(goal, conjecture", e_axfilter_gsine_case["workdir_files"]["problem.p"])
        self.assertEqual(
            e_axfilter_gsine_case["output_files"],
            ["global.out", "problem_formulas.p"],
        )
        e_axfilter_lambda_case = cases_by_name["e_axfilter/tstp-lambda-def-formulas"]
        self.assertIn(
            "thf(lambda_def1, definition",
            e_axfilter_lambda_case["workdir_files"]["problem.p"],
        )
        self.assertEqual(
            e_axfilter_lambda_case["output_files"],
            ["global.out", "problem_defs.p"],
        )
        e_axfilter_seeded_case = cases_by_name["e_axfilter/tstp-seeded-all-methods"]
        self.assertEqual(
            e_axfilter_seeded_case["output_files"],
            [
                "global.out",
                "problem_SA_P1_24_seed.p",
                "problem_SL_P1_24_seed.p",
                "problem_SD_P1_24_seed.p",
            ],
        )
        self.assertIn(
            "GSinE(CountTerms,hypos",
            e_axfilter_seeded_case["workdir_files"]["filters.axf"],
        )
        e_axfilter_output_error_case = cases_by_name[
            "e_axfilter/output-open-missing-parent"
        ]
        self.assertEqual(
            e_axfilter_output_error_case["workdir_files"],
            {"problem.p": "fof(a, axiom, p(a)).\n"},
        )
        e_axfilter_filter_error_case = cases_by_name["e_axfilter/filter-open-missing"]
        self.assertEqual(
            e_axfilter_filter_error_case["arguments"],
            ["-f", "missing.axf", "problem.p"],
        )
        e_client_case = cases_by_name["e_client/invalid-port"]
        self.assertIsNone(e_client_case["stdin"])
        e_deduction_case = cases_by_name["e_deduction_server/stdout-unimplemented"]
        self.assertIsNone(e_deduction_case["stdin"])
        e_ltb_runner_case = cases_by_name["e_ltb_runner/usage-missing-spec"]
        self.assertIsNone(e_ltb_runner_case["stdin"])
        e_server_case = cases_by_name["e_server/usage-missing-domain"]
        self.assertIsNone(e_server_case["stdin"])
        ekb_create_case = cases_by_name["ekb_create/empty-kb-files"]
        self.assertEqual(
            ekb_create_case["output_files"],
            [
                "kb/description",
                "kb/signature",
                "kb/problems",
                "kb/clausepatterns",
            ],
        )
        self.assertEqual(ekb_create_case["output_directories"], ["kb/FILES"])
        ekb_delete_case = cases_by_name["ekb_delete/drop-example"]
        self.assertIn("1: \"drop\"", ekb_delete_case["workdir_files"]["kb/problems"])
        self.assertIn(
            "q(a) : 1:(1,0,0,0,0,0,0).",
            ekb_delete_case["workdir_files"]["kb/clausepatterns"],
        )
        self.assertEqual(ekb_delete_case["workdir_files"]["kb/FILES/drop"], "drop problem")
        self.assertEqual(ekb_delete_case["workdir_files"]["kb/FILES/keep"], "keep problem")
        self.assertEqual(ekb_delete_case["workdir_directories"], ["kb/FILES"])
        self.assertEqual(
            ekb_delete_case["output_files"],
            ["kb/FILES/keep", "kb/problems", "kb/clausepatterns"],
        )
        self.assertEqual(ekb_delete_case["output_absent_files"], ["kb/FILES/drop"])
        middle_case = cases_by_name["ekb_delete/drop-middle-example"]
        self.assertIn('2: "middle"', middle_case["workdir_files"]["kb/problems"])
        self.assertEqual(
            middle_case["output_absent_files"], ["kb/FILES/middle"]
        )
        self.assertEqual(len(middle_case["output_files"]), 5)
        ekb_ginsert_case = cases_by_name["ekb_ginsert/stdin-protocol"]
        self.assertIn(
            "1 : : [++p(a)] : initial : 'proof'",
            ekb_ginsert_case["stdin"],
        )
        self.assertIn(
            'Version     : "0.20dev"',
            ekb_ginsert_case["workdir_files"]["kb/description"],
        )
        self.assertEqual(
            ekb_ginsert_case["workdir_files"]["kb/signature"],
            "",
        )
        self.assertEqual(ekb_ginsert_case["workdir_directories"], ["kb/FILES"])
        self.assertEqual(
            ekb_ginsert_case["output_files"],
            ["kb/FILES/__problem__1", "kb/problems", "kb/clausepatterns"],
        )
        ekb_insert_case = cases_by_name["ekb_insert/stdin-example"]
        self.assertEqual(ekb_insert_case["stdin"], "a=b.\n.\n0:(0): a=b.\n")
        self.assertEqual(
            ekb_insert_case["workdir_files"],
            {"kb/signature": "", "kb/problems": "", "kb/clausepatterns": ""},
        )
        self.assertEqual(ekb_insert_case["workdir_directories"], ["kb/FILES"])
        self.assertEqual(
            ekb_insert_case["output_files"],
            ["kb/FILES/__problem__1", "kb/problems", "kb/clausepatterns"],
        )
        e_stratpar_case = cases_by_name["e_stratpar/usage-missing-problem"]
        self.assertIsNone(e_stratpar_case["stdin"])
        edpll_lop_case = cases_by_name["edpll/lop-basic"]
        self.assertEqual(edpll_lop_case["stdin"], "p <- q. r <- r.")
        edpll_tptp_case = cases_by_name["edpll/tptp-input-clause"]
        self.assertEqual(
            edpll_tptp_case["stdin"], "input_clause(c_0_1,axiom,[++p,--q])."
        )
        eground_case = cases_by_name["eground/lop-basic"]
        self.assertEqual(eground_case["stdin"], "p(a).\n")
        enormalizer_case = cases_by_name["enormalizer/term-basic"]
        self.assertIsNone(enormalizer_case["stdin"])
        self.assertEqual(
            enormalizer_case["fixture_files"],
            {"rules.lop": "f(X)=a.\n", "terms.lop": "f(b)\n"},
        )
        epclanalyse_case = cases_by_name["epclanalyse/stdin-basic"]
        self.assertIn("[++q(a),--r(X)]", epclanalyse_case["stdin"])
        epclanalyse_zero_case = cases_by_name[
            "epclanalyse/zero-denominator-safe-boundary"
        ]
        self.assertEqual(
            epclanalyse_zero_case["stdin"],
            "1 : : p(a) : initial\n2 : : [] : 1\n",
        )
        epclanalyse_missing_case = cases_by_name["epclanalyse/missing-input"]
        self.assertTrue(epclanalyse_missing_case["isolated_workdir"])
        self.assertEqual(
            epclanalyse_missing_case["arguments"],
            ["missing-epclanalyse-input.pcl"],
        )
        epclextract_case = cases_by_name["epclextract/stdin-basic"]
        self.assertIn("3 : : [] : 2 : 'final'", epclextract_case["stdin"])
        epclextract_mixed_case = cases_by_name[
            "epclextract/mixed-logic-proof-closure"
        ]
        self.assertIn("2 : : : 1", epclextract_mixed_case["stdin"])
        self.assertIn("3 : : q(a)|r(b) : 2", epclextract_mixed_case["stdin"])
        self.assertIn("6 : : [++unused] : initial", epclextract_mixed_case["stdin"])
        epclextract_comments_case = cases_by_name[
            "epclextract/multi-file-comments"
        ]
        self.assertIsNone(epclextract_comments_case["stdin"])
        self.assertEqual(
            epclextract_comments_case["fixture_files"],
            {
                "first.pcl": "% first lead\n1 : : p(a) : initial\n% first tail\n",
                "second.pcl": "% second lead\n2 : : : 1 : 'final'\n% second tail\n",
            },
        )
        epclextract_missing_case = cases_by_name["epclextract/missing-input"]
        self.assertTrue(epclextract_missing_case["isolated_workdir"])
        self.assertEqual(
            epclextract_missing_case["arguments"],
            ["missing-epclextract-input.pcl"],
        )
        epcllemma_case = cases_by_name["epcllemma/stdin-basic"]
        self.assertIn("5 : : [++t(a)] : er(4)", epcllemma_case["stdin"])
        epcllemma_large_case = cases_by_name["epcllemma/large-relative-limit"]
        self.assertEqual(epcllemma_large_case["stdin"].count(" : : "), 1_010)
        self.assertTrue(epcllemma_large_case["stdin"].startswith("1 : : [++p(a)]"))
        self.assertIn("1010 : : [++p(a)]", epcllemma_large_case["stdin"])
        for case_name in (
            "epcllemma/formula-lemma-pcl",
            "epcllemma/formula-lemma-tptp",
            "epcllemma/formula-lemma-tstp",
            "epcllemma/formula-lemma-lop",
        ):
            self.assertEqual(
                cases_by_name[case_name]["stdin"],
                "1 : : p(a) : initial\n2 : : q(a) : 1\n",
            )
        self.assertEqual(
            cases_by_name["epcllemma/minimum-quality-nan"]["stdin"], ""
        )
        epcllemma_missing_case = cases_by_name["epcllemma/missing-input"]
        self.assertTrue(epcllemma_missing_case["isolated_workdir"])
        self.assertEqual(
            epcllemma_missing_case["arguments"],
            ["missing-epcllemma-input.pcl"],
        )
        epcllemma_output_case = cases_by_name[
            "epcllemma/missing-output-parent"
        ]
        self.assertTrue(epcllemma_output_case["isolated_workdir"])
        self.assertEqual(epcllemma_output_case["stdin"], "")
        epatternize_case = cases_by_name["epatternize/lop-basic"]
        self.assertEqual(epatternize_case["stdin"], "p(a).\n")
        ex_case = cases_by_name["ex_commandline/options-basic"]
        self.assertIsNone(ex_case["stdin"])
        self.assertEqual(
            cases_by_name["ex_commandline/integer-range"]["arguments"],
            ["--int_example=9223372036854775808"],
        )
        self.assertEqual(
            cases_by_name["ex_commandline/float-range"]["arguments"],
            ["--float_example=1e9999"],
        )
        term2dag_case = cases_by_name["term2dag/stdin-basic"]
        self.assertEqual(term2dag_case["stdin"], "f(a,a) g(f(a,a))\n")
        term2dag_typed_case = cases_by_name["term2dag/shared-typed-boundary"]
        self.assertIn("apply(F:$i > $i,a)", term2dag_typed_case["stdin"])
        self.assertIn("q(Y:$o) q(Y)", term2dag_typed_case["stdin"])
        term2dag_missing_case = cases_by_name["term2dag/missing-input"]
        self.assertTrue(term2dag_missing_case["isolated_workdir"])
        self.assertEqual(
            term2dag_missing_case["arguments"], ["missing-term2dag-input"]
        )
        termprops_case = cases_by_name["termprops/stdin-basic"]
        self.assertEqual(termprops_case["stdin"], "a f(a,a) g(f(a),a)\n")
        termprops_empty_case = cases_by_name["termprops/empty-input"]
        self.assertEqual(termprops_empty_case["stdin"], "")
        termprops_missing_case = cases_by_name["termprops/missing-input"]
        self.assertTrue(termprops_missing_case["isolated_workdir"])
        tsm_case = cases_by_name["tsm_classify/stdin-basic"]
        self.assertIn("Training:\n", tsm_case["stdin"])
        self.assertIn("Test:\n", tsm_case["stdin"])
        recursive_tsm_case = cases_by_name["tsm_classify/recursive-mixed"]
        self.assertIn("h(f(a),g(a,b))", recursive_tsm_case["stdin"])
        self.assertIn("f(h(f(b),g(b,a)))", recursive_tsm_case["stdin"])
        empty_tsm_case = cases_by_name["tsm_classify/empty-test-set"]
        self.assertTrue(empty_tsm_case["stdin"].endswith("Test:\n.\n"))

    def test_tool_fixture_materialization_substitutes_arguments(self):
        [case] = [
            case
            for case in e_interop.tool_comparison_cases(["enormalizer"])
            if case["name"] == "enormalizer/term-basic"
        ]
        with tempfile.TemporaryDirectory() as tmp:
            fixture_paths = e_interop.materialize_tool_fixture_files(case, Path(tmp))
            arguments = e_interop.substitute_tool_fixture_arguments(
                case["arguments"], fixture_paths
            )
            self.assertEqual(Path(arguments[1]).read_text(encoding="utf-8"), "f(b)\n")
            self.assertEqual(Path(arguments[2]).read_text(encoding="utf-8"), "f(X)=a.\n")

    def test_tool_companion_substitution_preserves_argument_prefix(self):
        with tempfile.TemporaryDirectory() as tmp:
            companion = Path(tmp) / "eprover"
            companion.write_bytes(b"binary")
            arguments = e_interop.substitute_tool_companion_arguments(
                ['--executable="{companion:eprover}"'],
                {"eprover": companion},
            )

        self.assertEqual(arguments, [f'--executable="{companion}"'])

    def test_tool_workdir_materialization_and_output_comparison(self):
        case = {
            "workdir_files": {"input.p": "fof(a, axiom, p(a)).\n"},
            "workdir_directories": ["kb/FILES"],
            "output_files": ["global.out", "generated/result.p"],
        }
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            reference_cwd.mkdir()
            candidate_cwd.mkdir()

            reference_directories = e_interop.materialize_tool_workdir_directories(
                case, reference_cwd
            )
            candidate_directories = e_interop.materialize_tool_workdir_directories(
                case, candidate_cwd
            )
            reference_paths = e_interop.materialize_tool_workdir_files(
                case, reference_cwd
            )
            candidate_paths = e_interop.materialize_tool_workdir_files(
                case, candidate_cwd
            )
            self.assertEqual(reference_directories, [reference_cwd / "kb" / "FILES"])
            self.assertEqual(candidate_directories, [candidate_cwd / "kb" / "FILES"])
            self.assertTrue((reference_cwd / "kb" / "FILES").is_dir())
            self.assertTrue((candidate_cwd / "kb" / "FILES").is_dir())
            self.assertEqual(
                reference_paths["input.p"].read_text(encoding="utf-8"),
                "fof(a, axiom, p(a)).\n",
            )
            self.assertEqual(
                candidate_paths["input.p"].read_text(encoding="utf-8"),
                "fof(a, axiom, p(a)).\n",
            )

            (reference_cwd / "global.out").write_text(
                f"% Parsing {reference_cwd}\\input.p\n", encoding="utf-8"
            )
            (candidate_cwd / "global.out").write_text(
                f"% Parsing {candidate_cwd}\\input.p\n", encoding="utf-8"
            )
            (reference_cwd / "generated").mkdir()
            (candidate_cwd / "generated").mkdir()
            (reference_cwd / "generated" / "result.p").write_text(
                "fof(a, axiom, p(a)).\n", encoding="utf-8"
            )
            (candidate_cwd / "generated" / "result.p").write_text(
                "fof(a, axiom, p(a)).\n", encoding="utf-8"
            )

            records, details = e_interop.compare_tool_output_files(
                case["output_files"],
                reference_cwd,
                candidate_cwd,
                [
                    (str(reference_cwd), "<WORKDIR>"),
                    (str(candidate_cwd), "<WORKDIR>"),
                ],
            )

        self.assertTrue(all(record["normalized_equal"] for record in records))
        self.assertTrue(details["global.out"]["normalized_equal"])
        self.assertTrue(details["generated/result.p"]["normalized_equal"])

    def test_tool_output_file_can_opt_into_legacy_classify_normalization(self):
        reference = (
            "prob : (   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,"
            "  12,  13,  14,  15, -0.714286, 0.933333,  16,  17,  18,  19,  20,"
            " 22319, 32767, 0.000000, 0.000000, true, false ) :"
            " FUHS-NFFSM3-MDHHMFFBN\n"
        )
        candidate = (
            "prob : (   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,"
            "  12,  13,  14,  15, -0.714286, 0.933333,  16,  17,  18,  19,  20,"
            "   0,   0, 0.000000, 0.000000, false, false ) :"
            " FUHS-NFFSM3-MDFFFFFNN\n"
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            reference_cwd.mkdir()
            candidate_cwd.mkdir()
            (reference_cwd / "features.out").write_text(reference, encoding="utf-8")
            (candidate_cwd / "features.out").write_text(candidate, encoding="utf-8")

            records, _details = e_interop.compare_tool_output_files(
                ["features.out"],
                reference_cwd,
                candidate_cwd,
                [],
                normalize_legacy_classify_feature_suffix=True,
            )

        self.assertTrue(records[0]["normalized_equal"])

    def test_tool_output_comparison_requires_declared_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            reference_cwd.mkdir()
            candidate_cwd.mkdir()
            (reference_cwd / "global.out").write_text("reference\n", encoding="utf-8")

            records, details = e_interop.compare_tool_output_files(
                ["global.out"], reference_cwd, candidate_cwd, []
            )

        self.assertEqual(
            records,
            [
                {
                    "name": "global.out",
                    "reference_exists": True,
                    "candidate_exists": False,
                    "normalized_equal": False,
                }
            ],
        )
        self.assertFalse(details["global.out"]["normalized_equal"])

    def test_tool_absent_output_file_comparison_requires_absence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            (reference_cwd / "kb" / "FILES").mkdir(parents=True)
            (candidate_cwd / "kb" / "FILES").mkdir(parents=True)
            (reference_cwd / "kb" / "FILES" / "drop").write_text(
                "still present\n", encoding="utf-8"
            )

            records = e_interop.compare_tool_absent_output_files(
                ["kb/FILES/drop"], reference_cwd, candidate_cwd
            )

        self.assertEqual(
            records,
            [
                {
                    "name": "kb/FILES/drop",
                    "reference_absent": False,
                    "candidate_absent": True,
                    "absent_equal": False,
                }
            ],
        )

    def test_tool_absent_output_file_comparison_accepts_absence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            reference_cwd.mkdir()
            candidate_cwd.mkdir()

            records = e_interop.compare_tool_absent_output_files(
                ["drop"], reference_cwd, candidate_cwd
            )

        self.assertEqual(
            records,
            [
                {
                    "name": "drop",
                    "reference_absent": True,
                    "candidate_absent": True,
                    "absent_equal": True,
                }
            ],
        )

    def test_tool_output_directory_comparison_requires_declared_directories(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            reference_cwd.mkdir()
            candidate_cwd.mkdir()
            (reference_cwd / "kb").mkdir()
            (reference_cwd / "kb" / "FILES").mkdir()

            records = e_interop.compare_tool_output_directories(
                ["kb/FILES"], reference_cwd, candidate_cwd
            )

        self.assertEqual(
            records,
            [
                {
                    "name": "kb/FILES",
                    "reference_exists": True,
                    "candidate_exists": False,
                    "equal": False,
                }
            ],
        )

    def test_tool_output_directory_comparison_accepts_matching_directories(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reference_cwd = root / "reference"
            candidate_cwd = root / "candidate"
            (reference_cwd / "kb" / "FILES").mkdir(parents=True)
            (candidate_cwd / "kb" / "FILES").mkdir(parents=True)

            records = e_interop.compare_tool_output_directories(
                ["kb/FILES"], reference_cwd, candidate_cwd
            )

        self.assertEqual(
            records,
            [
                {
                    "name": "kb/FILES",
                    "reference_exists": True,
                    "candidate_exists": True,
                    "equal": True,
                }
            ],
        )

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
            e_interop.tool_argument_cases("e_deduction_server"),
            (("--help",), ("--version",)),
        )
        self.assertEqual(
            e_interop.tool_argument_cases("e_stratpar"),
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
