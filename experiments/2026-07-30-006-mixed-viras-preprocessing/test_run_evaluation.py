from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("run_evaluation.py")
SPEC = importlib.util.spec_from_file_location("mixed_viras_evaluation", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
evaluation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(evaluation)


class MixedVirasEvaluationTests(unittest.TestCase):
    def test_stats_parser_accepts_balanced_checked_record(self) -> None:
        output = (
            "% VIRAS QE preprocessing: formulas=7 quantified=3 imported=2 "
            "checked=1 applied=1 unsupported=1 resource_unknown=1 "
            "fragment_unknown=0 source_nodes=19 result_nodes=4 branch_proofs=2\n"
        )
        self.assertEqual(
            evaluation.parse_preprocess_stats(output),
            {
                "formulas": 7,
                "quantified": 3,
                "imported": 2,
                "checked": 1,
                "applied": 1,
                "unsupported": 1,
                "resource_unknown": 1,
                "fragment_unknown": 0,
                "source_nodes": 19,
                "result_nodes": 4,
                "branch_proofs": 2,
            },
        )

    def test_stats_parser_rejects_unchecked_publication(self) -> None:
        output = (
            "% VIRAS QE preprocessing: formulas=1 quantified=1 imported=1 "
            "checked=0 applied=1 unsupported=0 resource_unknown=0 "
            "fragment_unknown=0 source_nodes=2 result_nodes=1 branch_proofs=1\n"
        )
        with self.assertRaises(evaluation.EvaluationError):
            evaluation.parse_preprocess_stats(output)

    def test_determinism_normalization_removes_only_timing_lines(self) -> None:
        source = (
            "% SZS status Theorem\n"
            "% Preprocessing time : 0.010 s\n"
            "fof(c_1,plain,p,inference(viras_qe,[status(thm)],[a])).\n"
        )
        normalized = evaluation.normalized_deterministic_output(source)
        self.assertEqual(
            normalized,
            "% SZS status Theorem\n"
            "fof(c_1,plain,p,inference(viras_qe,[status(thm)],[a])).",
        )

    def test_percentile_uses_nearest_rank(self) -> None:
        self.assertEqual(evaluation.percentile([1.0, 2.0, 3.0, 4.0], 0.95), 4.0)
        self.assertIsNone(evaluation.percentile([], 0.95))


if __name__ == "__main__":
    unittest.main()
