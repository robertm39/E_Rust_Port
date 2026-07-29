#!/usr/bin/env python3
"""Inventory the source and runtime package relicensing boundary."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import tarfile
from collections import Counter
from pathlib import Path, PurePosixPath
from typing import Any

E_REVISION = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"
CADICAL_REVISION = "c60730422e758ef1cebe7aeddf2dda31c996bf04"
SOURCE_SHA256 = "2d82e62955b0f2eb1a9a1c2c77007e05fefc3af0c4130aee83618416664a5b3f"
RUNTIME_SHA256 = "e79448ef845c83e1f7022a2b9b12949a16db722812862a15e104526197c687a3"
REPOSITORY_AUTHOR = "Robert Morton <robertpmorton39@gmail.com>"
E_HOLDERS = "Stephan Schulz and the named E contributors"

LICENSE_RECORDS = {
    "licenses/cadical-MIT.txt": (
        "CaDiCaL",
        CADICAL_REVISION,
        "Armin Biere and CaDiCaL contributors",
        "MIT",
    ),
    "licenses/eprover-GPL-2.0-or-later_OR_LGPL-2.1-or-later.txt": (
        "E",
        E_REVISION,
        E_HOLDERS,
        "GPL-2.0-or-later OR LGPL-2.1-or-later",
    ),
    "licenses/gmp-GPL-2.0-or-later.txt": (
        "GMP",
        "6.3.0 reference-only distribution",
        "Free Software Foundation and GMP contributors",
        "GPL-2.0-or-later",
    ),
    "licenses/gmp-GPL-3.0-or-later.txt": (
        "GMP",
        "6.3.0 reference-only distribution",
        "Free Software Foundation and GMP contributors",
        "GPL-3.0-or-later",
    ),
    "licenses/gmp-LGPL-3.0-or-later.txt": (
        "GMP",
        "6.3.0 reference-only distribution",
        "Free Software Foundation and GMP contributors",
        "LGPL-3.0-or-later",
    ),
    "licenses/minisat-MIT.txt": (
        "MiniSat",
        "37dc6c67e2af26379d88ce349eb9c4c6160e8543",
        "Niklas Een, Niklas Sorensson, and MiniSat contributors",
        "MIT",
    ),
    "licenses/picosat-MIT.txt": (
        "PicoSAT",
        f"965 retained by E {E_REVISION}",
        "Armin Biere and PicoSAT contributors",
        "MIT",
    ),
    "licenses/vampire-BSD-3-Clause.txt": (
        "Vampire",
        "3677326861181f990ce3ef461e90471ba9749225",
        "Vampire contributors",
        "BSD-3-Clause",
    ),
    "licenses/z3-MIT.txt": (
        "Z3",
        "2d48fd119ce5074b880944c2b1c59e537c99cd46",
        "Microsoft Corporation and Z3 contributors",
        "MIT",
    ),
}

OBVIOUS_UMLAUT_SOURCE = {
    "native/cadical_ffi/umlaut_cadical.cpp",
    "native/cadical_ffi/umlaut_cadical.h",
    "src/clauses/cadical.rs",
    "src/clauses/satservice.rs",
}

E_CORE_PREFIXES = (
    "BASICS/",
    "CLAUSES/",
    "CONTROL/",
    "HEURISTICS/",
    "INOUT/",
    "LEARN/",
    "ORDERINGS/",
    "PCL2/",
    "PROPOSITIONAL/",
    "PROVER/",
    "SIMPLE_APPS/",
    "TERMS/",
)


def sha256_bytes(data: bytes) -> str:
    """Return a lowercase SHA-256 digest."""
    return hashlib.sha256(data).hexdigest()


def normalized_text_bytes(data: bytes) -> bytes:
    """Normalize only Windows line endings for repository comparison."""
    return data.replace(b"\r\n", b"\n")


def archive_sha256(path: Path) -> str:
    """Hash an archive without loading it all into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def safe_archive_path(name: str) -> PurePosixPath:
    """Validate and return one relative archive member path."""
    path = PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"unsafe archive member: {name}")
    return path


def regular_members(path: Path) -> list[tuple[str, bytes]]:
    """Read regular files from an archive after path validation."""
    result = []
    with tarfile.open(path, "r:gz") as archive:
        for member in archive.getmembers():
            member_path = safe_archive_path(member.name)
            if not member.isfile():
                continue
            stream = archive.extractfile(member)
            if stream is None:
                raise ValueError(f"could not read archive member: {member.name}")
            result.append((member_path.as_posix(), stream.read()))
    return sorted(result)


def product_authority(origin: str) -> dict[str, str]:
    """Return the conservative authority decision for product-authored files."""
    if origin == "mixed_e_port_and_umlaut":
        return {
            "copyright_holders": (
                f"{E_HOLDERS} for any E-derived expression; "
                f"{REPOSITORY_AUTHOR} for Umlaut expression, subject to attestation"
            ),
            "current_license": (
                "GPL-2.0-or-later package; pinned E distribution also offers "
                "GPL-2.0-or-later OR LGPL-2.1-or-later"
            ),
            "target_route": (
                "select LGPL-3.0 through E's LGPL-2.1-or-later option and "
                "relicense Umlaut-owned expression"
            ),
            "authority_status": (
                "blocked_pending_owner_attestation_and_qualified_review_of_"
                "e_distribution_scope"
            ),
        }
    return {
        "copyright_holders": (
            f"{REPOSITORY_AUTHOR}, subject to ownership and employer attestation"
        ),
        "current_license": "GPL-2.0-or-later package",
        "target_route": "relicense copyright-controlled expression to LGPL-3.0",
        "authority_status": (
            "blocked_pending_owner_attestation_and_qualified_review"
        ),
    }


def classify_source(path: str) -> dict[str, str]:
    """Classify one source-package member; fail closed on unknown paths."""
    if path == "Cargo.toml":
        return {
            "origin": "cargo_generated_manifest",
            "upstream_project": "Cargo",
            "upstream_revision": "cargo 1.97.1 package normalization",
            "copyright_holders": "follows Cargo.toml.orig inputs",
            "current_license": "generated package metadata",
            "target_route": "regenerate after changing Cargo.toml.orig",
            "authority_status": "follows_audited_inputs",
        }
    if path in {"Cargo.toml.orig", "Cargo.lock"}:
        record = product_authority("umlaut_original_metadata")
        record.update(
            {
                "origin": (
                    "umlaut_original_metadata"
                    if path == "Cargo.toml.orig"
                    else "cargo_generated_lockfile"
                ),
                "upstream_project": "Umlaut/Cargo",
                "upstream_revision": "repository commit under audit",
            }
        )
        return record
    if path == "LICENSE":
        return {
            "origin": "verbatim_license_text",
            "upstream_project": "Free Software Foundation",
            "upstream_revision": "GPL-2.0 text",
            "copyright_holders": "Free Software Foundation, Inc.",
            "current_license": "GPL-2.0 license document",
            "target_route": "replace with unmodified official LGPL-3.0 text",
            "authority_status": "not_product_expression_retain_or_replace_verbatim",
        }
    if path in LICENSE_RECORDS:
        project, revision, holders, license_id = LICENSE_RECORDS[path]
        return {
            "origin": "verbatim_third_party_license_record",
            "upstream_project": project,
            "upstream_revision": revision,
            "copyright_holders": holders,
            "current_license": license_id,
            "target_route": "retain verbatim; do not relicense this record",
            "authority_status": "not_product_expression_retain_verbatim",
        }
    if path == "src/heuristics/schedule.vars":
        return {
            "origin": "verbatim_e_data",
            "upstream_project": "E",
            "upstream_revision": E_REVISION,
            "copyright_holders": E_HOLDERS,
            "current_license": "GPL-2.0-or-later OR LGPL-2.1-or-later",
            "target_route": (
                "select LGPL-3.0 through the LGPL-2.1-or-later option"
            ),
            "authority_status": (
                "route_documented_pending_qualified_review_of_e_distribution_scope"
            ),
        }
    if path in OBVIOUS_UMLAUT_SOURCE:
        record = product_authority("umlaut_original_source")
        record.update(
            {
                "origin": "umlaut_original_source",
                "upstream_project": "Umlaut",
                "upstream_revision": "repository commit under audit",
            }
        )
        return record
    if path == "build.rs" or path.startswith("src/"):
        record = product_authority("mixed_e_port_and_umlaut")
        record.update(
            {
                "origin": "mixed_e_port_and_umlaut",
                "upstream_project": "E and Umlaut",
                "upstream_revision": E_REVISION,
            }
        )
        return record
    if path.startswith("tools/packaging/"):
        record = product_authority("umlaut_original_packaging")
        record.update(
            {
                "origin": "umlaut_original_packaging",
                "upstream_project": "Umlaut",
                "upstream_revision": "repository commit under audit",
            }
        )
        return record
    if (
        path in {"README.md", "THIRD_PARTY_NOTICES.md"}
        or path.startswith("docs/")
    ):
        record = product_authority("umlaut_original_documentation")
        record.update(
            {
                "origin": "umlaut_original_documentation",
                "upstream_project": "Umlaut with attributed third-party facts",
                "upstream_revision": "repository commit under audit",
            }
        )
        return record
    raise ValueError(f"unclassified source-package member: {path}")


def classify_runtime(path: str) -> dict[str, str]:
    """Classify one runtime-package member; fail closed on unknown paths."""
    if path == "LICENSE":
        return classify_source(path)
    if path == "THIRD_PARTY_NOTICES.md":
        return classify_source(path)
    if path == "bin/starexec_run_default":
        record = product_authority("umlaut_original_packaging")
        record.update(
            {
                "origin": "umlaut_original_packaging",
                "upstream_project": "Umlaut",
                "upstream_revision": "repository commit under audit",
            }
        )
        return record
    if path == "starexec_description.txt":
        return {
            "origin": "generated_runtime_metadata",
            "upstream_project": "Umlaut package generator",
            "upstream_revision": "repository commit under audit",
            "copyright_holders": "follows package generator inputs",
            "current_license": "generated package metadata",
            "target_route": "regenerate after authorized license change",
            "authority_status": "follows_audited_inputs",
        }
    if path == "bin/umlaut":
        record = product_authority("mixed_e_port_and_umlaut")
        record.update(
            {
                "origin": "compiled_product_aggregate",
                "upstream_project": "Umlaut, E-derived portions, Rust toolchain",
                "upstream_revision": E_REVISION,
            }
        )
        return record
    raise ValueError(f"unclassified runtime-package member: {path}")


def e_header_license_class(header: str) -> str:
    """Classify the license phrases visible in one E source header."""
    has_gpl = bool(re.search(r"GNU General Public Licen[cs]e", header))
    has_lgpl = bool(re.search(r"GNU Lesser General Public Licen[cs]e", header))
    if has_gpl and has_lgpl:
        return "gpl_and_lgpl_phrases"
    if has_gpl:
        return "gpl_phrase_only"
    if has_lgpl:
        return "lgpl_phrase_only"
    return "no_gnu_license_phrase"


def build_e_header_records(e_source: Path) -> tuple[list[dict[str, Any]], str]:
    """Scan tracked core E C/H headers at the exact pinned revision."""
    e_source = e_source.resolve()
    revision = git_output(e_source, "rev-parse", "HEAD")
    if revision != E_REVISION:
        raise ValueError(f"unexpected E source revision: {revision}")
    copying = (e_source / "COPYING").read_bytes()
    tracked = git_output(e_source, "ls-files", "--", "*.c", "*.h").splitlines()
    paths = sorted(
        path
        for path in tracked
        if path.startswith(E_CORE_PREFIXES) and path.endswith((".c", ".h"))
    )
    records = []
    for path in paths:
        data = (e_source / path).read_bytes()
        text = data.decode("utf-8", errors="replace")
        header = "\n".join(text.splitlines()[:45])
        author_match = re.search(
            r"^\s*Authors?\s*:\s*(.+?)\s*$",
            header,
            flags=re.MULTILINE,
        )
        records.append(
            {
                "path": path,
                "bytes": len(data),
                "sha256": sha256_bytes(data),
                "header_license_class": e_header_license_class(header),
                "header_author": (
                    author_match.group(1).strip() if author_match else None
                ),
                "distribution_revision": revision,
                "distribution_license_record": "COPYING",
                "distribution_license": (
                    "GPL-2.0-or-later OR LGPL-2.1-or-later"
                ),
            }
        )
    return records, sha256_bytes(copying)


def repository_bytes(repo_root: Path, path: str, archived: bytes) -> tuple[str, bytes]:
    """Return the canonical HEAD path and bytes corresponding to a member."""
    if path == "Cargo.toml":
        return "(Cargo-generated package manifest)", archived
    repository_path = "Cargo.toml" if path == "Cargo.toml.orig" else path
    result = subprocess.run(
        ["git", "show", f"HEAD:{repository_path}"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    return repository_path, result.stdout


def strip_package_root(name: str, expected_root: str | None) -> tuple[str, str]:
    """Strip and validate the single source-package root directory."""
    member_path = safe_archive_path(name)
    if len(member_path.parts) < 2:
        raise ValueError(f"source member lacks package root: {name}")
    root = member_path.parts[0]
    if expected_root is not None and root != expected_root:
        raise ValueError(f"multiple source package roots: {expected_root}, {root}")
    return root, PurePosixPath(*member_path.parts[1:]).as_posix()


def git_output(repo_root: Path, *arguments: str) -> str:
    """Run one read-only Git query."""
    return subprocess.run(
        ["git", *arguments],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    """Write stable compact JSON Lines."""
    path.write_text(
        "".join(
            json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
            for record in records
        ),
        encoding="utf-8",
    )


def build_source_records(
    repo_root: Path, source_archive: Path
) -> list[dict[str, Any]]:
    """Build one provenance record per source-package file."""
    package_root = None
    records = []
    for archive_name, archived in regular_members(source_archive):
        package_root, path = strip_package_root(archive_name, package_root)
        repository_path, current = repository_bytes(repo_root, path, archived)
        record: dict[str, Any] = {
            "boundary": "source",
            "path": path,
            "repository_path": repository_path,
            "archived_bytes": len(archived),
            "archived_sha256": sha256_bytes(archived),
            "repository_head_bytes": len(current),
            "repository_head_sha256": sha256_bytes(current),
            "repository_head_exact_bytes_match": current == archived,
            "repository_head_normalized_content_match": (
                normalized_text_bytes(current) == normalized_text_bytes(archived)
            ),
            "observed_repository_author": (
                REPOSITORY_AUTHOR
                if not path.startswith("licenses/") and path != "LICENSE"
                else None
            ),
        }
        record.update(classify_source(path))
        records.append(record)
    return records


def build_runtime_records(runtime_archive: Path) -> list[dict[str, Any]]:
    """Build one provenance record per rootless runtime-package file."""
    records = []
    for path, data in regular_members(runtime_archive):
        record: dict[str, Any] = {
            "boundary": "runtime",
            "path": path,
            "bytes": len(data),
            "sha256": sha256_bytes(data),
        }
        record.update(classify_runtime(path))
        records.append(record)
    return records


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-archive", type=Path, required=True)
    parser.add_argument("--runtime-archive", type=Path, required=True)
    parser.add_argument("--e-source", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    """Run the complete fail-closed inventory."""
    args = parse_args()
    script_path = Path(__file__).resolve()
    repo_root = script_path.parents[2]
    source_archive = args.source_archive.resolve()
    runtime_archive = args.runtime_archive.resolve()
    source_digest = archive_sha256(source_archive)
    runtime_digest = archive_sha256(runtime_archive)
    if source_digest != SOURCE_SHA256:
        raise ValueError(f"unexpected source archive SHA-256: {source_digest}")
    if runtime_digest != RUNTIME_SHA256:
        raise ValueError(f"unexpected runtime archive SHA-256: {runtime_digest}")

    source_records = build_source_records(repo_root, source_archive)
    runtime_records = build_runtime_records(runtime_archive)
    e_header_records, e_copying_sha256 = build_e_header_records(args.e_source)
    if len(source_records) != 314:
        raise ValueError(f"expected 314 source members, found {len(source_records)}")
    if len(runtime_records) != 5:
        raise ValueError(f"expected 5 runtime members, found {len(runtime_records)}")

    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    write_jsonl(output_dir / "source-members.jsonl", source_records)
    write_jsonl(output_dir / "runtime-members.jsonl", runtime_records)
    write_jsonl(output_dir / "e-source-headers.jsonl", e_header_records)

    author_identities = sorted(
        set(git_output(repo_root, "log", "--all", "--format=%aN <%aE>").splitlines())
    )
    committer_identities = sorted(
        set(git_output(repo_root, "log", "--all", "--format=%cN <%cE>").splitlines())
    )
    commit_messages = git_output(repo_root, "log", "--all", "--format=%B")
    trailer_count = len(
        re.findall(
            r"^(?:Co-authored-by|Signed-off-by|Copyright):",
            commit_messages,
            flags=re.IGNORECASE | re.MULTILINE,
        )
    )
    summary = {
        "schema_version": 1,
        "repository_head": git_output(repo_root, "rev-parse", "HEAD"),
        "repository_commit_count": int(
            git_output(repo_root, "rev-list", "--count", "HEAD")
        ),
        "observed_git_author_identities": author_identities,
        "observed_git_committer_identities": committer_identities,
        "coauthor_or_signoff_trailer_count": trailer_count,
        "source_archive": {
            "sha256": source_digest,
            "members": len(source_records),
            "repository_head_exact_byte_matches": sum(
                bool(record["repository_head_exact_bytes_match"])
                for record in source_records
            ),
            "repository_head_normalized_content_matches": sum(
                bool(record["repository_head_normalized_content_match"])
                for record in source_records
            ),
        },
        "runtime_archive": {
            "sha256": runtime_digest,
            "members": len(runtime_records),
        },
        "e_source": {
            "revision": E_REVISION,
            "copying_sha256": e_copying_sha256,
            "core_c_h_files": len(e_header_records),
            "header_license_class_counts": dict(
                sorted(
                    Counter(
                        record["header_license_class"]
                        for record in e_header_records
                    ).items()
                )
            ),
            "named_header_authors": sorted(
                {
                    str(record["header_author"])
                    for record in e_header_records
                    if record["header_author"] is not None
                }
            ),
        },
        "source_origin_counts": dict(
            sorted(Counter(record["origin"] for record in source_records).items())
        ),
        "runtime_origin_counts": dict(
            sorted(Counter(record["origin"] for record in runtime_records).items())
        ),
        "unclassified_member_count": 0,
        "decision": "not_authorized_pending_attestation_and_qualified_legal_review",
    }
    (output_dir / "summary.json").write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
