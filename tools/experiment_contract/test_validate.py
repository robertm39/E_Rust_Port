"""Tests for the experiment-result contract validator."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from types import ModuleType


REPO_ROOT = Path(__file__).resolve().parents[2]
TOOL_ROOT = Path(__file__).resolve().parent


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


VALIDATE = load_module("experiment_contract_validate", TOOL_ROOT / "validate.py")


class ContractValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.template = VALIDATE.load_json(TOOL_ROOT / "template.json")

    def record(self) -> dict[str, object]:
        return copy.deepcopy(self.template)

    def test_template_is_valid_and_schema_is_parseable(self) -> None:
        schema = json.loads(
            (TOOL_ROOT / "experiment-result.schema.json").read_text(
                encoding="utf-8"
            )
        )

        self.assertEqual(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema",
        )
        self.assertEqual(
            VALIDATE.validate_record(
                self.record(),
                repo_root=REPO_ROOT,
                verify_artifacts=True,
            ),
            [],
        )

    def test_missing_top_level_section_is_rejected(self) -> None:
        record = self.record()
        del record["correctness"]

        errors = VALIDATE.validate_record(record)

        self.assertTrue(
            any("missing required keys: correctness" in error for error in errors)
        )

    def test_continue_requires_passing_correctness(self) -> None:
        record = self.record()
        record["decision"]["outcome"] = "continue"

        errors = VALIDATE.validate_record(record)

        self.assertIn(
            "decision.outcome cannot be continue unless correctness passes",
            errors,
        )

    def test_solve_count_arithmetic_is_checked(self) -> None:
        record = self.record()
        record["coverage"]["candidate_reproducible_solves"] = 2
        record["coverage"]["common_reproducible_solves"] = 1

        errors = VALIDATE.validate_record(record)

        self.assertIn(
            "candidate solve count must equal common plus candidate-only",
            errors,
        )

    def test_valid_performance_requires_observation_and_noise(self) -> None:
        record = self.record()
        record["performance"]["status"] = "valid"
        record["performance"]["observations"] = [
            {
                "scope": "heldout/common-solved",
                "paired_coordinates": 2,
                "candidate_over_baseline_median": 0.9,
            }
        ]

        errors = VALIDATE.validate_record(record)

        self.assertTrue(
            any(
                "performance.observations[0] is missing required keys: noise"
                in error
                for error in errors
            )
        )

    def test_duplicate_json_keys_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"schema_version": 1, "schema_version": 1}\n')

            with self.assertRaises(VALIDATE.DuplicateKeyError):
                VALIDATE.load_json(path)

    def test_artifact_digest_mismatch_is_rejected(self) -> None:
        record = self.record()
        record["reproduction"]["artifacts"][0]["sha256"] = "0" * 64

        errors = VALIDATE.validate_record(
            record,
            repo_root=REPO_ROOT,
            verify_artifacts=True,
        )

        self.assertTrue(
            any(".sha256 mismatch:" in error for error in errors)
        )

    def test_artifact_parent_traversal_is_rejected(self) -> None:
        record = self.record()
        record["reproduction"]["artifacts"][0]["path"] = "../outside"

        errors = VALIDATE.validate_record(record)

        self.assertTrue(
            any("must be repository-relative and contained" in error for error in errors)
        )


if __name__ == "__main__":
    unittest.main()
