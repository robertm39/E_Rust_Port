"""Local controller tests for the CASC package audit."""

from __future__ import annotations

import gzip
import hashlib
import importlib.util
import io
import tarfile
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("verify_casc_package.py")
SPEC = importlib.util.spec_from_file_location("verify_casc_package", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"could not load {MODULE_PATH}")
audit = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audit)


def write_tar(path: Path, names: list[str]) -> None:
    """Write a small gzip tar containing regular files at `names`."""

    with tarfile.open(path, "w:gz") as archive:
        for name in names:
            payload = b"fixture"
            info = tarfile.TarInfo(name)
            info.size = len(payload)
            archive.addfile(info, io.BytesIO(payload))


class SourceArchiveTests(unittest.TestCase):
    def valid_names(self) -> list[str]:
        root = "umlaut-0.1.0"
        return [
            f"{root}/{relative}"
            for relative in sorted(audit.REQUIRED_SOURCE_PATHS)
        ]

    def test_required_source_archive_is_accepted(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "source.crate"
            write_tar(archive, self.valid_names())
            root, members = audit.source_members(archive)
        self.assertEqual(root, "umlaut-0.1.0")
        self.assertEqual(members, sorted(audit.REQUIRED_SOURCE_PATHS))

    def test_forbidden_package_roots_are_rejected(self):
        for component in sorted(audit.FORBIDDEN_COMPONENTS):
            with self.subTest(component=component):
                with tempfile.TemporaryDirectory() as temporary:
                    archive = Path(temporary) / "source.crate"
                    names = self.valid_names()
                    names.append(
                        f"umlaut-0.1.0/{component}/unexpected-package-input"
                    )
                    write_tar(archive, names)
                    with self.assertRaisesRegex(
                        audit.AuditError,
                        f"forbidden component {component}",
                    ):
                        audit.source_members(archive)

    def test_pdf_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "source.crate"
            names = self.valid_names()
            names.append("umlaut-0.1.0/viras_docs/paper.pdf")
            write_tar(archive, names)
            with self.assertRaisesRegex(audit.AuditError, "PDF"):
                audit.source_members(archive)


class RuntimeArchiveTests(unittest.TestCase):
    def test_runtime_archive_is_deterministic_and_minimal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            (source / "tools" / "packaging").mkdir(parents=True)
            binary = root / "umlaut"
            binary.write_bytes(b"binary")
            (source / "tools" / "packaging" / "README-CASC.md").write_text(
                "readme\n",
                encoding="utf-8",
            )
            (source / "LICENSE").write_text("license\n", encoding="utf-8")
            (source / "THIRD_PARTY_NOTICES.md").write_text(
                "notices\n",
                encoding="utf-8",
            )
            first = root / "first.tgz"
            second = root / "second.tgz"
            for target in (first, second):
                audit.build_runtime_archive(
                    target,
                    root_name="umlaut-0.1.0",
                    binary=binary,
                    source_root=source,
                )
            first_hash = hashlib.sha256(first.read_bytes()).hexdigest()
            second_hash = hashlib.sha256(second.read_bytes()).hexdigest()
            members = audit.runtime_members(first)
        self.assertEqual(first_hash, second_hash)
        self.assertEqual(
            members,
            [
                "umlaut-0.1.0/LICENSE",
                "umlaut-0.1.0/README.md",
                "umlaut-0.1.0/THIRD_PARTY_NOTICES.md",
                "umlaut-0.1.0/bin/umlaut",
            ],
        )

    def test_runtime_rejects_optional_backend_name(self):
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "runtime.tgz"
            write_tar(
                archive,
                [
                    "umlaut-0.1.0/bin/umlaut",
                    "umlaut-0.1.0/lib/libpicosat.so",
                    "umlaut-0.1.0/LICENSE",
                    "umlaut-0.1.0/README.md",
                ],
            )
            with self.assertRaisesRegex(
                audit.AuditError,
                "optional backend",
            ):
                audit.runtime_members(archive)


class RepositoryBoundaryTests(unittest.TestCase):
    def test_copied_e_inputs_match_pinned_hashes(self):
        expected = {
            audit.REPO_ROOT / "src" / "heuristics" / "schedule.vars":
                "491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068",
            audit.REPO_ROOT
            / "tests"
            / "fixtures"
            / "eprover-17026b1"
            / "e_options.h":
                "9b432caf9253a8e3b5b47901154ff419a17ba1ef7d788a17fcaf019186c87f3d",
        }
        for path, digest in expected.items():
            with self.subTest(path=path):
                self.assertEqual(audit.sha256(path), digest)

    def test_build_and_test_inputs_no_longer_read_ignored_e_checkout(self):
        build_script = (audit.REPO_ROOT / "build.rs").read_text(encoding="utf-8")
        schedule_module = (
            audit.REPO_ROOT / "src" / "heuristics" / "new_autoschedule.rs"
        ).read_text(encoding="utf-8")
        options_module = (
            audit.REPO_ROOT / "src" / "prover" / "options.rs"
        ).read_text(encoding="utf-8")
        combined = build_script + schedule_module + options_module
        self.assertNotIn("eprover/HEURISTICS/schedule.vars", combined)
        self.assertNotIn("eprover/PROVER/e_options.h", combined)

    def test_lock_file_has_no_external_dependencies(self):
        audit.assert_dependency_free(audit.REPO_ROOT / "Cargo.lock")

    def test_cargo_package_allowlist_is_root_anchored(self):
        with (audit.REPO_ROOT / "Cargo.toml").open("rb") as source:
            manifest = audit.tomllib.load(source)
        include = manifest["package"]["include"]
        self.assertTrue(include)
        self.assertTrue(
            all(pattern.startswith("/") for pattern in include),
            "unanchored Cargo include patterns can match ignored reference trees",
        )

    def test_runtime_gzip_header_has_stable_timestamp(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary) / "empty.gz"
            with target.open("wb") as raw:
                with gzip.GzipFile(fileobj=raw, mode="wb", mtime=0) as stream:
                    stream.write(b"test")
            header = target.read_bytes()[:10]
        self.assertEqual(header[4:8], b"\x00\x00\x00\x00")


if __name__ == "__main__":
    unittest.main()
