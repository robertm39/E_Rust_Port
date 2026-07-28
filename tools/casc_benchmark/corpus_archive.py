#!/usr/bin/env python3
"""Pack or safely extract the ignored CASC-30 corpus for a Linode runner."""

from __future__ import annotations

import argparse
import gzip
import os
import shutil
import sys
import tarfile
from pathlib import Path, PurePosixPath
from typing import BinaryIO, Sequence

from manifest import ManifestError, load_manifest, sha256_file, verify_corpus

ARCHIVE_PREFIX = PurePosixPath("problems/casc_2025")
ALLOWED_SUFFIXES = {".p", ".ax"}


class CorpusArchiveError(RuntimeError):
    """Raised when the corpus transfer boundary is unsafe or incomplete."""


def corpus_files(repo_root: Path) -> list[Path]:
    root = repo_root / Path(*ARCHIVE_PREFIX.parts)
    files = sorted(
        path
        for path in root.rglob("*")
        if path.is_file() and path.suffix in ALLOWED_SUFFIXES
    )
    problem_count = sum(path.suffix == ".p" for path in files)
    axiom_count = sum(path.suffix == ".ax" for path in files)
    if problem_count != 2901 or axiom_count != 2425:
        raise CorpusArchiveError(
            f"expected 2901 problems and 2425 axioms, found "
            f"{problem_count} and {axiom_count}"
        )
    return files


def normalized_tar_info(path: Path, arcname: str) -> tarfile.TarInfo:
    info = tarfile.TarInfo(arcname)
    info.size = path.stat().st_size
    info.mode = 0o644
    info.mtime = 0
    info.uid = 0
    info.gid = 0
    info.uname = "root"
    info.gname = "root"
    return info


def write_archive(repo_root: Path, output: BinaryIO) -> None:
    """Write deterministic gzip and tar streams."""

    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as tar:
            for path in corpus_files(repo_root):
                arcname = path.relative_to(repo_root).as_posix()
                with path.open("rb") as source:
                    tar.addfile(normalized_tar_info(path, arcname), source)


def pack(repo_root: Path, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    temporary = output_path.with_name(f".{output_path.name}.{os.getpid()}.tmp")
    try:
        with temporary.open("wb") as output:
            write_archive(repo_root, output)
        os.replace(temporary, output_path)
    finally:
        if temporary.exists():
            temporary.unlink()


def validated_member_path(name: str) -> PurePosixPath:
    path = PurePosixPath(name)
    if (
        path.is_absolute()
        or ".." in path.parts
        or not path.parts
        or path.parts[: len(ARCHIVE_PREFIX.parts)] != ARCHIVE_PREFIX.parts
        or path.suffix not in ALLOWED_SUFFIXES
    ):
        raise CorpusArchiveError(f"unsafe or unexpected archive member: {name!r}")
    return path


def extract(archive_path: Path, destination: Path) -> None:
    """Extract regular corpus files without trusting tar paths or metadata."""

    target_root = destination / Path(*ARCHIVE_PREFIX.parts)
    if target_root.exists():
        raise CorpusArchiveError(f"refusing to overwrite corpus tree: {target_root}")
    extracted = 0
    try:
        with tarfile.open(archive_path, mode="r:gz") as archive:
            for member in archive:
                member_path = validated_member_path(member.name)
                if not member.isfile():
                    raise CorpusArchiveError(
                        f"archive member is not a regular file: {member.name!r}"
                    )
                source = archive.extractfile(member)
                if source is None:
                    raise CorpusArchiveError(
                        f"cannot read archive member: {member.name!r}"
                    )
                destination_path = destination / Path(*member_path.parts)
                destination_path.parent.mkdir(parents=True, exist_ok=True)
                with destination_path.open("xb") as output:
                    shutil.copyfileobj(source, output, length=1024 * 1024)
                extracted += 1
    except BaseException:
        if target_root.exists():
            shutil.rmtree(target_root)
        raise
    if extracted != 5326:
        if target_root.exists():
            shutil.rmtree(target_root)
        raise CorpusArchiveError(f"expected 5326 archive files, extracted {extracted}")


def verify(repo_root: Path, manifest_path: Path) -> None:
    metadata, records = load_manifest(manifest_path)
    verify_corpus(repo_root, metadata, records)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    pack_parser = commands.add_parser("pack")
    pack_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    pack_parser.add_argument("--output", type=Path, required=True)
    extract_parser = commands.add_parser("extract")
    extract_parser.add_argument("--archive", type=Path, required=True)
    extract_parser.add_argument("--destination", type=Path, required=True)
    extract_parser.add_argument("--manifest", type=Path, required=True)
    verify_parser = commands.add_parser("verify")
    verify_parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    verify_parser.add_argument("--manifest", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    try:
        if arguments.command == "pack":
            output = arguments.output.resolve()
            pack(arguments.repo_root.resolve(), output)
            print(
                f"OK: corpus archive {output}, SHA-256 {sha256_file(output)}"
            )
        elif arguments.command == "extract":
            destination = arguments.destination.resolve()
            try:
                extract(arguments.archive.resolve(), destination)
                verify(destination, arguments.manifest.resolve())
            except BaseException:
                target = destination / Path(*ARCHIVE_PREFIX.parts)
                if target.exists():
                    shutil.rmtree(target)
                raise
            print("OK: safely extracted and verified 2901 problems and 2425 axioms")
        elif arguments.command == "verify":
            verify(arguments.repo_root.resolve(), arguments.manifest.resolve())
            print("OK: verified 2901 problems and 2425 axioms")
        else:  # pragma: no cover
            raise AssertionError(arguments.command)
        return 0
    except (CorpusArchiveError, ManifestError, OSError, tarfile.TarError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
