#!/usr/bin/env python3
"""Focused contracts for the bounded AVATAR restart prototype."""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from unittest import mock
from pathlib import Path


EXPERIMENT_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(EXPERIMENT_ROOT))

import tptp_split as SPLIT  # noqa: E402
import select_corpus as CORPUS  # noqa: E402
import verify_certificate as VERIFY  # noqa: E402


class TptpSplitTests(unittest.TestCase):
    def test_comments_quotes_and_nested_terms_are_lexically_safe(self) -> None:
        text = """
        % a period. and a fake cnf(x).
        cnf(c1, axiom, (p('a|b',f(X,1.5)) | q(Y))). /* ignored. */
        """
        statements = SPLIT.split_statements(text)
        self.assertEqual(len(statements), 1)
        record = SPLIT.parse_cnf_statement(statements[0], 0)
        self.assertEqual(record["literals"], ["p('a|b',f(X,1.5))", "q(Y)"])

    def test_shared_variables_form_connected_components(self) -> None:
        analysis = SPLIT.analyze_problem(
            "cnf(c,axiom,(p(X)|q(Y)|r(X)|s(a))).", 4
        )
        components = analysis["split_records"][0]["components"]
        self.assertEqual(
            [component["literals"] for component in components],
            [["p(X)", "r(X)"], ["q(Y)"], ["s(a)"]],
        )

    def test_ground_literals_are_separate_components(self) -> None:
        analysis = SPLIT.analyze_problem(
            "cnf(c,axiom,(p(a)|q(b)|r(c))).", 1
        )
        self.assertEqual(analysis["selector_count"], 3)
        self.assertEqual(analysis["split_clauses"], [[1, 2, 3]])

    def test_alpha_equivalent_components_reuse_selector(self) -> None:
        analysis = SPLIT.analyze_problem(
            "\n".join(
                [
                    "cnf(c1,axiom,(p(X)|q(a))).",
                    "cnf(c2,axiom,(p(Y)|r(b))).",
                ]
            ),
            2,
        )
        first = analysis["split_records"][0]["components"][0]["selector"]
        second = analysis["split_records"][1]["components"][0]["selector"]
        self.assertEqual(first, second)
        self.assertEqual(analysis["selector_count"], 3)

    def test_selection_prefers_more_components_then_more_literals(self) -> None:
        analysis = SPLIT.analyze_problem(
            "\n".join(
                [
                    "cnf(two,axiom,(p(X)|q(Y))).",
                    "cnf(three,axiom,(a(X)|b(Y)|c(Z))).",
                    "cnf(large_two,axiom,(d(X)|e(X)|f(Y))).",
                ]
            ),
            2,
        )
        self.assertEqual(
            [record["name"] for record in analysis["split_records"]],
            ["three", "large_two"],
        )
        self.assertIn("cnf(two,axiom,(p(X)|q(Y))).", analysis["base_statements"])

    def test_branch_removes_split_clause_and_adds_active_components(self) -> None:
        analysis = SPLIT.analyze_problem(
            "\n".join(
                [
                    "cnf(base,axiom,(~p(a))).",
                    "cnf(split,axiom,(p(X)|q(Y))).",
                ]
            ),
            1,
        )
        branch = SPLIT.render_branch(
            analysis, [1], source_sha256="0" * 64, model_index=1
        )
        self.assertIn("cnf(base,axiom,(~p(a))).", branch)
        self.assertNotIn("cnf(split", branch)
        self.assertIn("cnf(avatar_component_1, plain, (p(X))).", branch)
        self.assertNotIn("q(Y)", branch)

    def test_non_cnf_and_includes_are_rejected(self) -> None:
        with self.assertRaises(SPLIT.SplitError):
            SPLIT.analyze_problem("fof(f,axiom,p(a)).", 1)
        with self.assertRaises(SPLIT.SplitError):
            SPLIT.analyze_problem("include('Axioms/SET001.ax').", 1)

    def test_unterminated_comment_is_rejected(self) -> None:
        with self.assertRaises(SPLIT.SplitError):
            SPLIT.split_statements("/* missing")


class CorpusSelectionTests(unittest.TestCase):
    def test_stable_score_is_partition_and_cohort_specific(self) -> None:
        first = CORPUS.stable_score("train", "neutral", "P1")
        self.assertEqual(
            first,
            CORPUS.stable_score("train", "neutral", "P1"),
        )
        self.assertNotEqual(
            first,
            CORPUS.stable_score("test", "neutral", "P1"),
        )
        self.assertNotEqual(
            first,
            CORPUS.stable_score("train", "split_sensitive", "P1"),
        )

    def test_classification_uses_only_syntax_and_manifest_fields(self) -> None:
        record = {
            "division": "EPR",
            "expected_class": "unsatisfiable",
            "size_bytes": 100,
            "includes": [],
            "path": "unused.p",
        }
        with mock.patch.object(
            CORPUS,
            "analyze_file",
            return_value={
                "cnf_count": 5,
                "selected_split_count": 1,
                "selector_count": 2,
                "splittable_clause_count": 1,
                "statement_count": 5,
            },
        ):
            result = CORPUS.classify_problem(Path("."), record)
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result[0], "split_sensitive")


class CertificateTests(unittest.TestCase):
    def make_certificate(
        self, directory: Path
    ) -> tuple[Path, Path, dict[str, object]]:
        source = "cnf(split,axiom,(p(X)|q(Y))).\n"
        problem = directory / "problem.p"
        problem.write_text(source, encoding="utf-8")
        source_hash = VERIFY.sha256_file(problem)
        records = VERIFY.parse_source(source)
        abstraction = VERIFY.expected_abstraction(records, 6)
        branches = []
        for index, (model, active) in enumerate(
            [([1, -2], [1]), ([-1, 2], [2])], 1
        ):
            branch_path = directory / f"branch-{index}.p"
            branch_path.write_text(
                VERIFY.render_expected_branch(
                    records,
                    abstraction,
                    active,
                    source_hash,
                    index,
                ),
                encoding="utf-8",
            )
            proof_path = directory / f"proof-{index}.txt"
            proof_path.write_text("synthetic proof\n", encoding="utf-8")
            branches.append(
                {
                    "model_index": index,
                    "sat_model": model,
                    "active_selectors": active,
                    "branch_path": branch_path.name,
                    "branch_sha256": VERIFY.sha256_file(branch_path),
                    "proof_verified": True,
                    "proof_path": proof_path.name,
                    "proof_sha256": VERIFY.sha256_file(proof_path),
                    "learned_conflict": [-selector for selector in active],
                }
            )
        certificate: dict[str, object] = {
            "schema_version": 1,
            "source_sha256": source_hash,
            "max_split_clauses": 6,
            "abstraction": abstraction,
            "branches": branches,
            "final_status": "unsatisfiable",
        }
        certificate_path = directory / "certificate.json"
        certificate_path.write_text(
            json.dumps(certificate), encoding="utf-8"
        )
        return certificate_path, problem, certificate

    def verify(
        self, certificate_path: Path, problem_path: Path
    ) -> dict[str, object]:
        return VERIFY.verify_certificate(
            certificate_path,
            problem_path,
            Path("proofcheck"),
            Path("gate"),
            proof_callback=lambda *_: None,
        )

    def test_independent_certificate_replay_accepts_sound_trace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            certificate, problem, _ = self.make_certificate(directory)
            report = self.verify(certificate, problem)
            self.assertEqual(report["final_status"], "unsatisfiable")
            self.assertEqual(report["verified_conflicts"], 2)

    def test_certificate_mutations_fail_closed(self) -> None:
        mutations = {
            "component": lambda value: value["abstraction"][
                "split_records"
            ][0]["components"][0].__setitem__("literals", ["bad(a)"]),
            "active": lambda value: value["branches"][0].__setitem__(
                "active_selectors", [2]
            ),
            "model": lambda value: value["branches"][0].__setitem__(
                "sat_model", [1, 2]
            ),
            "conflict": lambda value: value["branches"][0].__setitem__(
                "learned_conflict", [-2]
            ),
            "branch_hash": lambda value: value["branches"][0].__setitem__(
                "branch_sha256", "0" * 64
            ),
            "proof_hash": lambda value: value["branches"][0].__setitem__(
                "proof_sha256", "0" * 64
            ),
            "final_status": lambda value: value.__setitem__(
                "final_status", "unknown"
            ),
        }
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            certificate_path, problem, certificate = self.make_certificate(
                directory
            )
            for name, mutate in mutations.items():
                with self.subTest(name=name):
                    changed = copy.deepcopy(certificate)
                    mutate(changed)
                    certificate_path.write_text(
                        json.dumps(changed), encoding="utf-8"
                    )
                    with self.assertRaises(VERIFY.VerificationError):
                        self.verify(certificate_path, problem)


if __name__ == "__main__":
    unittest.main()
