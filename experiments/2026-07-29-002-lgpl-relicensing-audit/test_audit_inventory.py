"""Focused tests for the LGPL relicensing inventory."""

from __future__ import annotations

import importlib.util
import io
import tarfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("audit_inventory.py")
SPEC = importlib.util.spec_from_file_location("audit_inventory", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {SCRIPT}")
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class AuditInventoryTests(unittest.TestCase):
    """Pin fail-closed classification and archive handling."""

    def test_every_declared_special_source_class_has_expected_route(self) -> None:
        schedule = AUDIT.classify_source("src/heuristics/schedule.vars")
        self.assertEqual(schedule["origin"], "verbatim_e_data")
        self.assertIn("LGPL-2.1-or-later", schedule["current_license"])

        shim = AUDIT.classify_source("native/cadical_ffi/umlaut_cadical.cpp")
        self.assertEqual(shim["origin"], "umlaut_original_source")
        self.assertIn("attestation", shim["authority_status"])

        rust = AUDIT.classify_source("src/terms/terms.rs")
        self.assertEqual(rust["origin"], "mixed_e_port_and_umlaut")
        self.assertIn("qualified_review", rust["authority_status"])

    def test_license_records_are_retained_instead_of_relicensed(self) -> None:
        record = AUDIT.classify_source("licenses/cadical-MIT.txt")
        self.assertEqual(record["current_license"], "MIT")
        self.assertEqual(
            record["authority_status"],
            "not_product_expression_retain_verbatim",
        )

    def test_unknown_source_and_runtime_members_fail_closed(self) -> None:
        with self.assertRaisesRegex(ValueError, "unclassified source"):
            AUDIT.classify_source("mystery/source.c")
        with self.assertRaisesRegex(ValueError, "unclassified runtime"):
            AUDIT.classify_runtime("bin/mystery")

    def test_unsafe_archive_path_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "unsafe archive"):
            AUDIT.safe_archive_path("../escape")
        with self.assertRaisesRegex(ValueError, "unsafe archive"):
            AUDIT.safe_archive_path("/absolute")

    def test_repository_comparison_normalizes_only_crlf(self) -> None:
        self.assertEqual(
            AUDIT.normalized_text_bytes(b"one\r\ntwo\r\n"),
            b"one\ntwo\n",
        )
        self.assertNotEqual(
            AUDIT.normalized_text_bytes(b"one\r\ntwo\r\n"),
            AUDIT.normalized_text_bytes(b"one\nchanged\n"),
        )

    def test_e_header_classification_keeps_gpl_only_ambiguity_visible(self) -> None:
        self.assertEqual(
            AUDIT.e_header_license_class(
                "released under the GNU General Public Licence."
            ),
            "gpl_phrase_only",
        )
        self.assertEqual(
            AUDIT.e_header_license_class(
                "GNU General Public License and GNU Lesser General Public License"
            ),
            "gpl_and_lgpl_phrases",
        )

    def test_regular_members_returns_only_files(self) -> None:
        archive_bytes = io.BytesIO()
        with tarfile.open(fileobj=archive_bytes, mode="w:gz") as archive:
            directory = tarfile.TarInfo("root")
            directory.type = tarfile.DIRTYPE
            archive.addfile(directory)
            payload = b"evidence"
            member = tarfile.TarInfo("root/file.txt")
            member.size = len(payload)
            archive.addfile(member, io.BytesIO(payload))
        archive_bytes.seek(0)

        with self.subTest("temporary archive"):
            import tempfile

            with tempfile.TemporaryDirectory() as temporary:
                path = Path(temporary) / "input.tgz"
                path.write_bytes(archive_bytes.getvalue())
                self.assertEqual(
                    AUDIT.regular_members(path),
                    [("root/file.txt", b"evidence")],
                )


if __name__ == "__main__":
    unittest.main()
