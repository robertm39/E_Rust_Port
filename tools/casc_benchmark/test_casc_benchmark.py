#!/usr/bin/env python3
"""Regression tests for the CASC manifest, batch, and report contracts."""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path, PurePosixPath

THIS_DIR = Path(__file__).resolve().parent
REPO_ROOT = THIS_DIR.parents[1]
sys.path.insert(0, str(THIS_DIR))

import batch  # noqa: E402
import combined_report  # noqa: E402
import corpus_archive  # noqa: E402
import manifest  # noqa: E402
import report  # noqa: E402

MANIFEST_PATH = REPO_ROOT / "benchmarks" / "casc_2025_manifest.jsonl"
J13_MANIFEST_PATH = REPO_ROOT / "benchmarks" / "casc_2026_manifest.jsonl"


class ManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata, cls.records = manifest.load_manifest(MANIFEST_PATH)

    def test_checked_in_manifest_covers_casc_30_exactly(self):
        self.assertEqual(self.metadata["problem_count"], 2901)
        self.assertEqual(self.metadata["sources"]["axiom_count"], 2425)
        self.assertEqual(sum(self.metadata["category_counts"].values()), 2901)
        self.assertEqual(
            self.metadata["category_counts"],
            {
                "EPS": 100,
                "EPU": 100,
                "FEQ": 400,
                "FNE": 100,
                "ICU": 101,
                "SLH": 1000,
                "TEQ": 400,
                "TFE": 50,
                "TFI": 100,
                "TFN": 150,
                "TNE": 100,
                "UEQ": 300,
            },
        )
        self.assertEqual(len({record["problem_id"] for record in self.records}), 2901)

    def test_holdout_never_splits_a_family(self):
        family_splits: dict[str, set[str]] = {}
        for record in self.records:
            family_splits.setdefault(record["family"], set()).add(
                record["holdout_split"]
            )
        self.assertTrue(family_splits)
        self.assertTrue(all(len(splits) == 1 for splits in family_splits.values()))
        self.assertEqual(set(self.metadata["partition_counts"]), {
            "train",
            "validation",
            "test",
        })
        category_splits: dict[str, set[str]] = {}
        for record in self.records:
            category_splits.setdefault(record["category"], set()).add(
                record["holdout_split"]
            )
        self.assertTrue(
            all(
                splits == {"train", "validation", "test"}
                for splits in category_splits.values()
            )
        )

    def test_html_problem_typography_is_normalized(self):
        self.assertEqual(manifest.normalize_problem_id("HWV092‑1"), "HWV092-1")
        self.assertEqual(manifest.normalize_problem_id("SWC537_1*"), "SWC537_1")
        self.assertEqual(manifest.normalize_problem_id("FOO001+1.p"), "FOO001+1")

    def test_difficulty_bands_are_ordinal_quintiles(self):
        self.assertEqual(manifest.difficulty_band(1, 100), "q1")
        self.assertEqual(manifest.difficulty_band(20, 100), "q1")
        self.assertEqual(manifest.difficulty_band(21, 100), "q2")
        self.assertEqual(manifest.difficulty_band(100, 100), "q5")

    def test_slh_theory_name_is_the_family(self):
        text = "% Names : Combinable_Wands/example [Ref]\n"
        self.assertEqual(
            manifest.source_family("SLH", "SLH0001^1", text),
            "SLH:Combinable_Wands",
        )


class J13ManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.metadata, cls.records = manifest.load_manifest(J13_MANIFEST_PATH)

    def test_checked_in_manifest_covers_j13_atp_corpus_exactly(self):
        self.assertEqual(self.metadata["problem_count"], 1350)
        self.assertEqual(self.metadata["sources"]["axiom_count"], 2438)
        self.assertEqual(
            self.metadata["category_counts"],
            {
                "FEQ": 300,
                "FNE": 100,
                "FNN": 50,
                "FNQ": 100,
                "TEQ": 300,
                "TNE": 100,
                "UEQ": 400,
            },
        )
        self.assertNotIn("PRV", self.metadata["category_counts"])
        self.assertEqual(
            len(self.metadata["sources"]["official_result_file_sha256"]), 26
        )
        self.assertTrue(
            all(record["limit_kind"] == "wall" for record in self.records)
        )
        self.assertTrue(
            all(record["limit_seconds"] == 180 for record in self.records)
        )
        self.assertTrue(
            all(
                record["path"].startswith("problems/casc_2026/")
                for record in self.records
            )
        )

    def test_j13_holdout_never_splits_a_family(self):
        family_splits: dict[str, set[str]] = {}
        category_splits: dict[str, set[str]] = {}
        for record in self.records:
            family_splits.setdefault(record["family"], set()).add(
                record["holdout_split"]
            )
            category_splits.setdefault(record["category"], set()).add(
                record["holdout_split"]
            )
        self.assertTrue(all(len(splits) == 1 for splits in family_splits.values()))
        self.assertTrue(
            all(
                splits == {"train", "validation", "test"}
                for splits in category_splits.values()
            )
        )


class BatchContractTests(unittest.TestCase):
    def sample_record(self, **changes):
        record = {
            "problem_id": "SYN001+1",
            "sha256": "a" * 64,
            "category": "FNE",
            "division": "FOF",
            "expected_class": "theorem",
            "limit_kind": "wall",
            "limit_seconds": 240,
            "family": "SYN",
            "holdout_split": "test",
            "difficulty_band": "q4",
        }
        record.update(changes)
        return record

    def test_umlaut_and_vampire_commands_pin_schedules_and_limits(self):
        record = self.sample_record()
        umlaut = batch.solver_command(
            "umlaut",
            Path("/bin/umlaut"),
            record,
            Path("/problems/SYN001+1.p"),
            cores=8,
            memory_mib=131072,
            seed=1,
        )
        vampire = batch.solver_command(
            "vampire",
            Path("/bin/vampire"),
            record,
            Path("/problems/SYN001+1.p"),
            cores=8,
            memory_mib=131072,
            seed=7,
        )
        self.assertIn("--auto-schedule=8", umlaut)
        self.assertIn("--memory-limit=131072", umlaut)
        self.assertIn("casc_2025", vampire)
        self.assertIn("131072", vampire)
        self.assertIn("7", vampire)

    def test_non_theorem_uses_complete_or_sat_schedule(self):
        record = self.sample_record(
            category="TFN",
            division="TFN",
            expected_class="non_theorem",
            limit_seconds=120,
        )
        umlaut = batch.solver_command(
            "umlaut",
            Path("/bin/umlaut"),
            record,
            Path("/problems/SWW001_1.p"),
            cores=8,
            memory_mib=131072,
            seed=1,
        )
        vampire = batch.solver_command(
            "vampire",
            Path("/bin/vampire"),
            record,
            Path("/problems/SWW001_1.p"),
            cores=8,
            memory_mib=131072,
            seed=1,
        )
        self.assertIn("--satauto-schedule=8", umlaut)
        self.assertIn("casc_sat_2025", vampire)
        self.assertIn("--intent", vampire)

    def test_slh_is_one_core_and_externally_cpu_accounted(self):
        record = self.sample_record(
            category="SLH",
            division="SLH",
            limit_kind="cpu",
            limit_seconds=15,
        )
        vampire = batch.solver_command(
            "vampire",
            Path("/bin/vampire"),
            record,
            Path("/problems/SLH0001^1.p"),
            cores=8,
            memory_mib=131072,
            seed=1,
        )
        cores_index = vampire.index("--cores")
        time_index = vampire.index("--time_limit")
        self.assertEqual(vampire[cores_index + 1], "1")
        self.assertEqual(vampire[time_index + 1], "0")

    def test_status_matching_does_not_use_peer_as_oracle(self):
        self.assertTrue(batch.expected_status_match("theorem", "Theorem"))
        self.assertTrue(batch.expected_status_match("non_theorem", "Satisfiable"))
        self.assertFalse(batch.expected_status_match("theorem", "CounterSatisfiable"))
        self.assertEqual(
            batch.classify_result(
                status="Theorem",
                return_code=0,
                termination_reason=None,
                oom_kills=0,
            ),
            "solved",
        )
        self.assertEqual(
            batch.classify_result(
                status=None,
                return_code=-9,
                termination_reason="wall",
                oom_kills=0,
            ),
            "timeout",
        )

    def test_vampire_portfolio_prefix_preserves_timeout_status(self):
        stdout = (
            "% (7937)Proof not found in time 179.942 s\n"
            "% (7937)SZS status Timeout for SEV254^5\n"
        )
        self.assertEqual(batch.szs_statuses(stdout), ["Timeout"])
        self.assertEqual(
            batch.classify_result(
                status=batch.szs_statuses(stdout)[-1],
                return_code=1,
                termination_reason=None,
                oom_kills=0,
            ),
            "timeout",
        )

    def test_existing_contract_must_match_exactly(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            contract = {"contract_id": "one", "schema_version": 1}
            batch.ensure_contract(root, contract)
            batch.ensure_contract(root, contract)
            with self.assertRaisesRegex(batch.BatchError, "incompatible"):
                batch.ensure_contract(root, {"contract_id": "two"})

    def test_session_records_runner_identity(self):
        runner = {
            "label": "e-rust-codex-260728-example",
            "run_id": "e-rust-codex-260728-example",
            "linode_id": 101605637,
        }
        session = batch.session_value(
            session_id="session-test",
            contract_id="contract-test",
            host={"hostname": "benchmark-host"},
            cgroup_root=Path("/sys/fs/cgroup"),
            runner=runner,
        )
        self.assertEqual(session["runner"], runner)
        self.assertEqual(session["cgroup_root"], str(Path("/sys/fs/cgroup")))

    def test_resumable_session_limits_are_not_part_of_the_contract(self):
        arguments = batch.parse_args(
            [
                "--manifest",
                "manifest.jsonl",
                "--problem-root",
                ".",
                "--output-root",
                "results",
                "--umlaut-binary",
                "umlaut",
                "--solvers",
                "umlaut",
                "--max-new-results",
                "7",
                "--max-session-wall-seconds",
                "3600",
            ]
        )
        self.assertEqual(arguments.max_new_results, 7)
        self.assertEqual(arguments.max_session_wall_seconds, 3600)


class CorpusArchiveTests(unittest.TestCase):
    def test_member_paths_are_confined_to_the_ignored_corpus_tree(self):
        valid = "problems/casc_2025/FNE/SYN001+1.p"
        self.assertEqual(
            corpus_archive.validated_member_path(valid).as_posix(), valid
        )
        for invalid in [
            "../outside.p",
            "/absolute/problem.p",
            "problems/casc_2025/../../outside.p",
            "problems/casc_2025/FNE/link",
            "other/corpus/SYN001+1.p",
        ]:
            with self.subTest(invalid=invalid):
                with self.assertRaises(corpus_archive.CorpusArchiveError):
                    corpus_archive.validated_member_path(invalid)

    def test_j13_member_paths_use_the_manifest_prefix(self):
        prefix = PurePosixPath("problems/casc_2026")
        valid = "problems/casc_2026/FEQ/MGT090+1.p"
        self.assertEqual(
            corpus_archive.validated_member_path(valid, prefix).as_posix(), valid
        )
        with self.assertRaises(corpus_archive.CorpusArchiveError):
            corpus_archive.validated_member_path(
                "problems/casc_2025/FEQ/MGT090+1.p", prefix
            )

    def test_tar_metadata_is_reproducible_and_non_executable(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "problem.p"
            path.write_text("fof(a,axiom,p).\n", encoding="utf-8")
            info = corpus_archive.normalized_tar_info(
                path, "problems/casc_2025/FNE/problem.p"
            )
            self.assertEqual(info.mtime, 0)
            self.assertEqual(info.mode, 0o644)
            self.assertEqual(info.uid, 0)
            self.assertEqual(info.gid, 0)


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.run_root = Path(self.temporary.name)
        self.metadata, all_records = manifest.load_manifest(MANIFEST_PATH)
        self.records = all_records[:2]
        self.contract = {
            "contract_id": "contract-test",
            "manifest_sha256": manifest.sha256_file(MANIFEST_PATH),
            "selected_problem_ids": [record["problem_id"] for record in self.records],
            "selected_problem_count": len(self.records),
            "canonical_full_selection": False,
            "solvers": {"umlaut": {}, "vampire": {}},
        }
        (self.run_root / "contract.json").write_bytes(
            batch.canonical_json(self.contract)
        )

    def tearDown(self):
        self.temporary.cleanup()

    def write_result(
        self,
        solver: str,
        record: dict,
        *,
        status: str,
        solved: bool,
        wall: float,
    ):
        base = self.run_root / "results" / solver / record["category"].lower()
        base.mkdir(parents=True, exist_ok=True)
        stem = f"{record['problem_id'].replace('/', '_')}-{solver}"
        stdout = base / f"{stem}.stdout"
        stderr = base / f"{stem}.stderr"
        stdout.write_text(f"% SZS status {status}\n", encoding="utf-8")
        stderr.write_text("", encoding="utf-8")
        value = {
            "contract_id": self.contract["contract_id"],
            "solver": solver,
            "problem_id": record["problem_id"],
            "classification": "solved" if solved else "gave_up",
            "expected_status_match": solved,
            "final_szs_status": status,
            "wall_seconds": wall,
            "cpu_seconds": wall * 0.9,
            "peak_memory_mib": 12.5,
            "stdout_path": stdout.relative_to(self.run_root).as_posix(),
            "stderr_path": stderr.relative_to(self.run_root).as_posix(),
            "stdout_sha256": manifest.sha256_file(stdout),
            "stderr_sha256": manifest.sha256_file(stderr),
        }
        (base / f"{stem}.json").write_bytes(batch.canonical_json(value))

    def test_partial_report_exposes_overlap_unique_solves_and_missing(self):
        first, second = self.records
        self.write_result("umlaut", first, status="Theorem", solved=True, wall=1.0)
        self.write_result("vampire", first, status="Theorem", solved=True, wall=0.5)
        self.write_result("umlaut", second, status="GaveUp", solved=False, wall=2.0)
        value = report.build_report(
            MANIFEST_PATH, self.run_root, require_complete=False
        )
        self.assertEqual(value["completed_results"], 3)
        self.assertEqual(value["missing_results"], 1)
        overall = value["overlap"]["overall"]["all"]
        self.assertEqual(overall["both_solved"], 1)
        self.assertEqual(overall["incomplete"], 1)
        self.assertIn("official competition entries", value["official_context_warning"])

    def test_complete_report_rejects_missing_results(self):
        with self.assertRaisesRegex(batch.BatchError, "incomplete"):
            report.build_report(MANIFEST_PATH, self.run_root, require_complete=True)

    def test_combined_report_keeps_release_identities_distinct(self):
        first, second = self.records
        self.write_result("umlaut", first, status="Theorem", solved=True, wall=1.0)
        self.write_result("vampire", first, status="Theorem", solved=True, wall=0.5)
        self.write_result("umlaut", second, status="GaveUp", solved=False, wall=2.0)
        self.write_result(
            "vampire", second, status="GaveUp", solved=False, wall=2.5
        )
        value = combined_report.build_combined_report(
            [
                ("first", MANIFEST_PATH, self.run_root),
                ("second", MANIFEST_PATH, self.run_root),
            ]
        )
        self.assertTrue(value["complete"])
        self.assertEqual(value["targeted_problems"], 4)
        self.assertEqual(value["completed_results"], 8)
        self.assertEqual(
            value["solvers"]["umlaut"]["groups"]["release"]["first"][
                "targeted"
            ],
            2,
        )


if __name__ == "__main__":
    unittest.main()
