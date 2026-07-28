"""Tests for the shell-free TPTP solution validation gate."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

from validate_tptp_solution import (
    EXIT_COVERAGE_GAP,
    EXIT_REJECTED,
    EXIT_VERIFIED,
    ValidationError,
    extract_output_blocks,
    parse_command_json,
    validate_solution,
)


PROOF_BODY = """\
fof(a, axiom, p(a), file('problem.p', a)).
fof(goal, conjecture, p(a), file('problem.p', goal)).
fof(nc, negated_conjecture, ~p(a),
    inference(assume_negation,[status(cth)],[goal])).
cnf(last, negated_conjecture, $false,
    inference(cn,[status(thm)],[a,nc])).
"""


class ValidationGateTests(unittest.TestCase):
    """Exercise accepted, rejected, and explicitly uncovered paths."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.problem = self.root / "problem.p"
        self.solution = self.root / "solution.s"
        self.checker = self.root / "checker.py"
        self.problem.write_text(
            "% Status : Theorem\n"
            "fof(a, axiom, p(a)).\n"
            "fof(goal, conjecture, p(a)).\n",
            encoding="utf-8",
        )
        self.checker.write_text(
            "from pathlib import Path\n"
            "import sys\n"
            "text = Path(sys.argv[-1]).read_text(encoding='utf-8')\n"
            "if 'bad_mapping' in text or '$false' not in text:\n"
            "    print('% SZS status VerifiedBad')\n"
            "    raise SystemExit(1)\n"
            "print('% SZS status VerifiedGood')\n",
            encoding="utf-8",
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def command(self) -> list[str]:
        return [sys.executable, str(self.checker), "{artifact}"]

    def write_solution(self, status: str, output_type: str, body: str) -> None:
        self.solution.write_text(
            f"% SZS status {status}\n"
            f"% SZS output start {output_type}\n"
            f"{body}"
            f"% SZS output end {output_type}\n",
            encoding="utf-8",
        )

    def validate(
        self,
        *,
        proof_command: list[str] | None = None,
        model_command: list[str] | None = None,
    ) -> tuple[dict[str, object], int]:
        return validate_solution(
            problem=self.problem,
            solution=self.solution,
            proof_command=proof_command,
            model_command=model_command,
            syntax_command=None,
            timeout_seconds=10,
        )

    def test_accepts_only_positive_external_proof_verdict(self) -> None:
        self.write_solution("Theorem", "CNFRefutation", PROOF_BODY)

        report, exit_code = self.validate(proof_command=self.command())

        self.assertEqual(exit_code, EXIT_VERIFIED)
        self.assertEqual(report["verdict"], "verified")
        self.assertEqual(report["claim_kind"], "proof")

    def test_missing_external_checker_is_coverage_gap(self) -> None:
        self.write_solution("Theorem", "CNFRefutation", PROOF_BODY)

        report, exit_code = self.validate()

        self.assertEqual(exit_code, EXIT_COVERAGE_GAP)
        self.assertEqual(report["verdict"], "coverage_gap")

    def test_refutation_without_false_is_rejected_before_checker(self) -> None:
        self.write_solution(
            "Theorem",
            "CNFRefutation",
            "fof(a, axiom, p(a)).\n",
        )

        report, exit_code = self.validate(proof_command=self.command())

        self.assertEqual(exit_code, EXIT_REJECTED)
        self.assertEqual(report["verdict"], "rejected")
        self.assertIn("no $false", report["reasons"][0])

    def test_known_non_theorem_rejects_theorem_claim(self) -> None:
        self.problem.write_text(
            "% Status : CounterSatisfiable\n"
            "fof(a, axiom, p(a)).\n"
            "fof(goal, conjecture, q(a)).\n",
            encoding="utf-8",
        )
        self.write_solution("Theorem", "CNFRefutation", PROOF_BODY)

        report, exit_code = self.validate(proof_command=self.command())

        self.assertEqual(exit_code, EXIT_REJECTED)
        self.assertEqual(report["checks"][0]["outcome"], "fail")

    def test_model_claim_without_interpretation_is_coverage_gap(self) -> None:
        self.problem.write_text(
            "% Status : CounterSatisfiable\n"
            "fof(a, axiom, p(a)).\n"
            "fof(goal, conjecture, q(a)).\n",
            encoding="utf-8",
        )
        self.solution.write_text(
            "% SZS status CounterSatisfiable\n",
            encoding="utf-8",
        )

        report, exit_code = self.validate(model_command=self.command())

        self.assertEqual(exit_code, EXIT_COVERAGE_GAP)
        self.assertIn("interpretation", report["reasons"][0])

    def test_corrupted_model_is_rejected_by_external_checker(self) -> None:
        self.problem.write_text(
            "% Status : Satisfiable\nfof(a, axiom, p(a)).\n",
            encoding="utf-8",
        )
        self.write_solution(
            "Satisfiable",
            "FiniteModel",
            "fof(bad_mapping, interpretation, $false).\n",
        )

        report, exit_code = self.validate(model_command=self.command())

        self.assertEqual(exit_code, EXIT_REJECTED)
        self.assertIn("VerifiedBad", report["reasons"][0])

    def test_non_success_status_is_not_applicable(self) -> None:
        self.solution.write_text("% SZS status ResourceOut\n", encoding="utf-8")

        report, exit_code = self.validate()

        self.assertEqual(exit_code, EXIT_VERIFIED)
        self.assertEqual(report["verdict"], "not_applicable")

    def test_unterminated_output_block_is_malformed(self) -> None:
        with self.assertRaisesRegex(ValidationError, "unterminated"):
            extract_output_blocks(
                "% SZS output start CNFRefutation\n"
                "cnf(last, plain, $false).\n"
            )

    def test_command_json_requires_a_nonempty_string_array(self) -> None:
        self.assertEqual(
            parse_command_json(
                json.dumps(["proofcheck", "-p", "{problem}", "{artifact}"]),
                "--proof-command-json",
            ),
            ["proofcheck", "-p", "{problem}", "{artifact}"],
        )
        with self.assertRaises(ValidationError):
            parse_command_json("[]", "--proof-command-json")


if __name__ == "__main__":
    unittest.main()
