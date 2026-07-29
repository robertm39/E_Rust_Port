"""Tests for the integrity-pinned external ProofGuard adapter."""

from __future__ import annotations

import hashlib
import subprocess
import tempfile
import unittest
from pathlib import Path

from run_pinned_proofguard import (
    AdapterError,
    checker_status,
    run_checker,
    sha256_file,
    verify_checkout,
)


class PinnedProofGuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.checker = self.root / "proover-check"
        self.engine = self.root / "proover.py"
        self.checker.write_text(
            "print('% SZS status VerifiedGood')\n",
            encoding="utf-8",
        )
        self.engine.write_text("# test engine\n", encoding="utf-8")
        self.git("init")
        self.git("config", "user.name", "Validation Test")
        self.git("config", "user.email", "validation@example.invalid")
        self.git(
            "remote",
            "add",
            "origin",
            "https://example.invalid/proofguard.git",
        )
        self.git("add", "proover-check", "proover.py")
        self.git("commit", "-m", "fixture")
        self.commit = self.git("rev-parse", "HEAD").stdout.strip()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def git(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *arguments],
            cwd=self.root,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def verify(self) -> None:
        verify_checkout(
            self.root,
            expected_commit=self.commit,
            expected_remote="https://example.invalid/proofguard.git",
            expected_checker_sha256=sha256_file(self.checker),
            expected_engine_sha256=sha256_file(self.engine),
        )

    def test_verifies_exact_clean_checkout(self) -> None:
        self.verify()

    def test_rejects_dirty_checkout(self) -> None:
        self.engine.write_text("# changed\n", encoding="utf-8")

        with self.assertRaisesRegex(AdapterError, "dirty"):
            self.verify()

    def test_rejects_checker_hash_mismatch(self) -> None:
        with self.assertRaisesRegex(AdapterError, "hash mismatch"):
            verify_checkout(
                self.root,
                expected_commit=self.commit,
                expected_remote="https://example.invalid/proofguard.git",
                expected_checker_sha256="0" * 64,
                expected_engine_sha256=sha256_file(self.engine),
            )

    def test_checker_status_is_single_line_and_strict(self) -> None:
        self.assertEqual(
            checker_status("% SZS status VerifiedBad : mutation\n"),
            "verifiedbad",
        )
        with self.assertRaises(AdapterError):
            checker_status(
                "% SZS status VerifiedGood\n% SZS status VerifiedBad\n"
            )

    def test_runs_checker_without_a_shell(self) -> None:
        eprover = self.root / "fake-eprover"
        problem = self.root / "problem.p"
        proof = self.root / "proof.s"
        eprover.write_bytes(b"external backend")
        problem.write_text("cnf(a,axiom,p).\n", encoding="utf-8")
        proof.write_text("cnf(a,axiom,p).\n", encoding="utf-8")

        completed = run_checker(
            self.checker,
            eprover,
            problem,
            proof,
            time_limit=5,
        )

        self.assertEqual(checker_status(completed.stdout), "verifiedgood")
        self.assertEqual(
            hashlib.sha256(eprover.read_bytes()).hexdigest(),
            sha256_file(eprover),
        )


if __name__ == "__main__":
    unittest.main()
