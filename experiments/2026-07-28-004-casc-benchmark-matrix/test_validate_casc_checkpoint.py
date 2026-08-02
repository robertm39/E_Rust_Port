#!/usr/bin/env python3
"""Focused tests for the guarded CASC checkpoint validator."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("validate_casc_checkpoint.py")
SPEC = importlib.util.spec_from_file_location("validate_casc_checkpoint", SCRIPT)
if SPEC is None or SPEC.loader is None:  # pragma: no cover
    raise RuntimeError(f"cannot load {SCRIPT}")
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


class PathAndInventoryTests(unittest.TestCase):
    def test_rejects_unsafe_member_paths(self) -> None:
        for name in ("/absolute", "../escape", "root/../escape", "root\\file"):
            with self.subTest(name=name):
                with self.assertRaises(VALIDATOR.ValidationError):
                    VALIDATOR.validated_member_name(name)

    def test_rejects_unsafe_and_duplicate_checksum_names(self) -> None:
        digest = "a" * 64
        for value in (
            f"{digest}  path/file\n".encode(),
            f"{digest}  file\n{digest}  file\n".encode(),
            b"not-a-hash  file\n",
        ):
            with self.subTest(value=value):
                with self.assertRaises(VALIDATOR.ValidationError):
                    VALIDATOR.parse_sha256s(value)

    def test_rejects_duplicate_and_link_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            duplicate = Path(temporary) / "duplicate.tar"
            with tarfile.open(duplicate, "w") as archive:
                archive.addfile(tarfile.TarInfo("root"))
                archive.addfile(tarfile.TarInfo("root"))
            with tarfile.open(duplicate, "r") as archive:
                with self.assertRaises(VALIDATOR.ValidationError):
                    VALIDATOR.regular_members(archive)

    def test_reconciles_outer_result_inventory(self) -> None:
        nested = "casc-runs/run/results/umlaut/foo/result.json"
        absolute = f"/opt/e-rust-port/{nested}"
        hashes = {nested: "a" * 64}
        captured = {
            "result-count.txt": b"1 /root/checkpoint/result-files.txt\n",
            "result-files.txt": f"{absolute}\n".encode(),
        }
        evidence = VALIDATOR.validate_outer_result_inventory(
            captured=captured,
            hashes=hashes,
            run_name="run",
            expected_results=1,
        )
        self.assertEqual(evidence["result_count"], 1)
        derived = VALIDATOR.validate_outer_result_inventory(
            captured=captured,
            hashes=hashes,
            run_name="run",
            expected_results=None,
        )
        self.assertEqual(derived["count_source"], "outer-inventory")

        captured["result-count.txt"] = b"2 result-files.txt\n"
        with self.assertRaisesRegex(
            VALIDATOR.ValidationError, "count differs"
        ):
            VALIDATOR.validate_outer_result_inventory(
                captured=captured,
                hashes=hashes,
                run_name="run",
                expected_results=1,
            )

    def test_rejects_nonterminal_outer_lifecycle(self) -> None:
        captured = {
            "processes.txt": b"PID PPID COMMAND ARGS\n1 0 systemd /sbin/init\n",
            "service-properties.txt": (
                b"Restart=no\nMainPID=0\nResult=success\nNRestarts=0\n"
                b"ExecMainStatus=0\nActiveState=inactive\nSubState=dead\n"
            ),
        }
        evidence = VALIDATOR.validate_outer_lifecycle_evidence(captured)
        self.assertEqual(evidence["exec_main_status"], 0)

        captured["processes.txt"] += (
            b"42 1 umlaut /root/umlaut-4e87dac3 --auto problem.p\n"
        )
        with self.assertRaisesRegex(
            VALIDATOR.ValidationError, "benchmark process"
        ):
            VALIDATOR.validate_outer_lifecycle_evidence(captured)

        captured["processes.txt"] = b"PID PPID COMMAND ARGS\n"
        captured["service-properties.txt"] = (
            b"Restart=no\nMainPID=42\nNRestarts=0\nExecMainStatus=0\n"
            b"ActiveState=active\nSubState=running\n"
        )
        with self.assertRaisesRegex(VALIDATOR.ValidationError, "MainPID"):
            VALIDATOR.validate_outer_lifecycle_evidence(captured)

            linked = Path(temporary) / "linked.tar"
            with tarfile.open(linked, "w") as archive:
                link = tarfile.TarInfo("root/link")
                link.type = tarfile.SYMTYPE
                link.linkname = "target"
                archive.addfile(link)
            with tarfile.open(linked, "r") as archive:
                with self.assertRaises(VALIDATOR.ValidationError):
                    VALIDATOR.regular_members(archive)


class RunValidationTests(unittest.TestCase):
    def make_fixture(
        self,
        root: Path,
        *,
        run_name: str = "fixture",
        problem_id: str = "FIX001+1",
    ) -> tuple[Path, dict[str, str], dict[str, bytes], str]:
        root.mkdir(parents=True, exist_ok=True)
        metadata = {
            "record_type": "manifest",
            "schema_version": 1,
            "kind": "umlaut-casc-benchmark-manifest",
            "corpus": f"{run_name}-corpus",
            "problem_count": 1,
            "presentation": {"id": "fixture-presentation"},
            "partition_counts": {"test": 1},
            "sources": {
                "official_result_file_sha256": {"official.csv": "2" * 64}
            },
        }
        record = {
            "record_type": "problem",
            "problem_id": problem_id,
            "category": "FOO",
            "division": "FOF",
            "holdout_split": "test",
            "difficulty_band": "easy",
            "sha256": "1" * 64,
        }
        manifest = root / "manifest.jsonl"
        manifest.write_bytes(canonical_json(metadata) + canonical_json(record))
        manifest_hash = hashlib.sha256(manifest.read_bytes()).hexdigest()
        selected_hash = hashlib.sha256(f"{problem_id}\n".encode()).hexdigest()
        contract = {
            "schema_version": 1,
            "kind": "umlaut-casc-benchmark-run",
            "manifest_sha256": manifest_hash,
            "selected_problem_count": 1,
            "selected_problem_ids": [problem_id],
            "selected_problem_ids_sha256": selected_hash,
            "presentation_id": "fixture-presentation",
            "canonical_full_selection": True,
            "solvers": ["umlaut", "vampire"],
        }
        contract_id = hashlib.sha256(canonical_json(contract)).hexdigest()
        contract["contract_id"] = contract_id
        stdout = b"proof\n"
        stderr = b""
        result = {
            "contract_id": contract_id,
            "problem_id": problem_id,
            "problem_sha256": "1" * 64,
            "solver": "umlaut",
            "classification": "solved",
            "expected_status_match": True,
            "final_szs_status": "Theorem",
            "wall_seconds": 0.1,
            "cpu_seconds": 0.1,
            "peak_memory_mib": 1.0,
            "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
        }
        summary = VALIDATOR.expected_run_report(
            metadata,
            [record],
            contract,
            {("umlaut", problem_id): result},
        )
        session = {
            "contract_id": contract_id,
            "runner": {"label": "runner", "run_id": "run", "linode_id": 1},
        }
        prefix = f"casc-runs/{run_name}/"
        key = VALIDATOR.safe_result_key(1, record)
        base = f"{prefix}results/umlaut/foo/{key}"
        values = {
            f"{prefix}contract.json": canonical_json(contract),
            f"{prefix}summary.json": canonical_json(summary),
            f"{prefix}sessions/session.json": canonical_json(session),
            f"{base}.json": canonical_json(result),
            f"{base}.stdout": stdout,
            f"{base}.stderr": stderr,
        }
        hashes = {
            name: hashlib.sha256(value).hexdigest() for name, value in values.items()
        }
        structured = {
            name: value for name, value in values.items() if name.endswith(".json")
        }
        return manifest, hashes, structured, contract_id

    def test_accepts_consistent_run(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            manifest, hashes, structured, contract_id = self.make_fixture(
                Path(temporary)
            )
            result = VALIDATOR.validate_run(
                hashes=hashes,
                structured=structured,
                run_name="fixture",
                manifest_path=manifest,
                contract_id=contract_id,
                expected_results=1,
            )
        self.assertEqual(result["completed_results"], 1)
        self.assertEqual(result["result_counts"], {"umlaut": 1})

    def test_rejects_tampered_contract_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            manifest, hashes, structured, contract_id = self.make_fixture(
                Path(temporary)
            )
            name = "casc-runs/fixture/contract.json"
            contract = json.loads(structured[name])
            contract["presentation_id"] = "tampered"
            structured[name] = canonical_json(contract)
            hashes[name] = hashlib.sha256(structured[name]).hexdigest()
            with self.assertRaisesRegex(
                VALIDATOR.ValidationError, "content does not hash"
            ):
                VALIDATOR.validate_run(
                    hashes=hashes,
                    structured=structured,
                    run_name="fixture",
                    manifest_path=manifest,
                    contract_id=contract_id,
                    expected_results=1,
                )

    def test_reconstructs_missing_summary_only_when_requested(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            manifest, hashes, structured, contract_id = self.make_fixture(
                Path(temporary)
            )
            name = "casc-runs/fixture/summary.json"
            del hashes[name]
            del structured[name]
            with self.assertRaisesRegex(
                VALIDATOR.ValidationError, "missing summary"
            ):
                VALIDATOR.validate_run(
                    hashes=hashes,
                    structured=structured,
                    run_name="fixture",
                    manifest_path=manifest,
                    contract_id=contract_id,
                    expected_results=1,
                )
            result = VALIDATOR.validate_run(
                hashes=hashes,
                structured=structured,
                run_name="fixture",
                manifest_path=manifest,
                contract_id=contract_id,
                expected_results=1,
                allow_missing_summary=True,
            )
            self.assertFalse(result["summary_embedded"])

    def test_rejects_orphan_result_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            manifest, hashes, structured, contract_id = self.make_fixture(
                Path(temporary)
            )
            hashes["casc-runs/fixture/results/umlaut/foo/orphan.stdout"] = (
                hashlib.sha256(b"").hexdigest()
            )
            with self.assertRaisesRegex(
                VALIDATOR.ValidationError, "orphan or unreferenced"
            ):
                VALIDATOR.validate_run(
                    hashes=hashes,
                    structured=structured,
                    run_name="fixture",
                    manifest_path=manifest,
                    contract_id=contract_id,
                    expected_results=1,
                )

    def test_validates_two_release_combined_summary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            left = self.make_fixture(
                root / "left", run_name="left-run", problem_id="FIX001+1"
            )
            right = self.make_fixture(
                root / "right", run_name="right-run", problem_id="FIX002+1"
            )
            left_manifest, left_hashes, left_structured, left_contract = left
            right_manifest, right_hashes, right_structured, right_contract = right
            hashes = left_hashes | right_hashes
            structured = left_structured | right_structured
            specifications = [
                ("LEFT", left_manifest, "left-run", left_contract),
                ("RIGHT", right_manifest, "right-run", right_contract),
            ]
            runs = {
                release: VALIDATOR.validate_run(
                    hashes=hashes,
                    structured=structured,
                    run_name=run_name,
                    manifest_path=manifest,
                    contract_id=contract_id,
                    expected_results=1,
                )
                for release, manifest, run_name, contract_id in specifications
            }
            combined = VALIDATOR.expected_combined_report(specifications, runs)
            combined_name = "casc-runs/combined-summary.json"
            structured[combined_name] = canonical_json(combined)
            evidence = VALIDATOR.validate_combined_summary(
                summary=combined,
                structured=structured,
                specifications=specifications,
                runs=runs,
            )
            self.assertEqual(evidence["completed_results"], 2)
            self.assertEqual(evidence["official_csv_count"], 2)
            self.assertEqual(
                evidence["release_completed_results"],
                {"LEFT": 1, "RIGHT": 1},
            )

            combined["releases"]["RIGHT"]["summary"]["completed_results"] = 0
            with self.assertRaisesRegex(
                VALIDATOR.ValidationError, "does not reproduce"
            ):
                VALIDATOR.validate_combined_summary(
                    summary=combined,
                    structured=structured,
                    specifications=specifications,
                    runs=runs,
                )


if __name__ == "__main__":
    unittest.main()
