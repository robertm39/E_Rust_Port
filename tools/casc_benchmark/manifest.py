#!/usr/bin/env python3
"""Build and validate immutable CASC benchmark manifests."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence

SCHEMA_VERSION = 1
MANIFEST_KIND = "umlaut-casc-benchmark-manifest"
PARTITION_SALT = "umlaut-casc30-family-holdout-v1"
PROBLEM_ARCHIVE_SHA256 = (
    "7a3e701d0cb374a3dae9ec6f40419985d648a1d5e65e97f85f832a3871d65720"
)
AXIOM_ARCHIVE_SHA256 = (
    "83901fa2041dfE98d8a49a25cc878ef9d0b479f47dbe47dd3f4f5450e9e04e8a"
).lower()
OFFICIAL_DESIGN_URL = "https://tptp.org/CASC/30/Design.html"
OFFICIAL_ARCHIVE_URL = "https://tptp.org/CASC/30/"

_DASH_TRANSLATION = str.maketrans(
    {
        "\u2010": "-",
        "\u2011": "-",
        "\u2012": "-",
        "\u2013": "-",
        "\u2014": "-",
        "\u2212": "-",
    }
)
_INCLUDE_RE = re.compile(
    r"(?im)^\s*include\s*\(\s*(['\"])(?P<path>.+?)\1(?:\s*,|\s*\))"
)
_HEADER_FIELD_RE = re.compile(
    r"(?im)^%\s*(?P<key>Status|Rating|Names)\s*:\s*(?P<value>.*?)\s*$"
)


@dataclass(frozen=True)
class Category:
    division: str
    expected_class: str
    limit_kind: str
    limit_seconds: int
    source: str


CATEGORIES: dict[str, Category] = {
    "TNE": Category("THF", "theorem", "wall", 240, "tptp"),
    "TEQ": Category("THF", "theorem", "wall", 240, "tptp"),
    "TFI": Category("TFA", "theorem", "wall", 120, "tptp"),
    "TFE": Category("TFA", "theorem", "wall", 120, "tptp"),
    "TFN": Category("TFN", "non_theorem", "wall", 120, "tptp"),
    "FNE": Category("FOF", "theorem", "wall", 240, "tptp"),
    "FEQ": Category("FOF", "theorem", "wall", 240, "tptp"),
    "EPU": Category("EPR", "unsatisfiable", "wall", 120, "tptp"),
    "EPS": Category("EPR", "satisfiable", "wall", 120, "tptp"),
    "UEQ": Category("UEQ", "unsatisfiable", "wall", 240, "tptp"),
    "SLH": Category("SLH", "theorem", "cpu", 15, "sledgehammer"),
    "ICU": Category("ICU", "theorem", "wall", 480, "entrant"),
}

CATEGORIES_J13: dict[str, Category] = {
    "TNE": Category("THF", "theorem", "wall", 180, "tptp"),
    "TEQ": Category("THF", "theorem", "wall", 180, "tptp"),
    "FNE": Category("FOF", "theorem", "wall", 180, "tptp"),
    "FEQ": Category("FOF", "theorem", "wall", 180, "tptp"),
    "FNN": Category("FNT", "non_theorem", "wall", 180, "tptp"),
    "FNQ": Category("FNT", "non_theorem", "wall", 180, "tptp"),
    "UEQ": Category("UEQ", "unsatisfiable", "wall", 180, "tptp"),
}


@dataclass(frozen=True)
class ReleaseSpec:
    release: str
    corpus: str
    problems_relative: Path
    results_relative: Path
    categories: dict[str, Category]
    problem_count: int
    partition_salt: str
    presentation_id: str
    presentation_description: str
    design_url: str
    archive_url: str
    problem_archive_sha256: str | None = None
    axiom_archive_sha256: str | None = None


RELEASES: dict[str, ReleaseSpec] = {
    "2025": ReleaseSpec(
        release="2025",
        corpus="CASC-30 (2025) official competition problems",
        problems_relative=Path("problems/casc_2025"),
        results_relative=Path("casc_2025_results"),
        categories=CATEGORIES,
        problem_count=2901,
        partition_salt=PARTITION_SALT,
        presentation_id="casc30-official-obfuscated",
        presentation_description=(
            "Official CASC-30 presentation, including organizer reordering "
            "and equality/connective reversals where applicable."
        ),
        design_url=OFFICIAL_DESIGN_URL,
        archive_url=OFFICIAL_ARCHIVE_URL,
        problem_archive_sha256=PROBLEM_ARCHIVE_SHA256,
        axiom_archive_sha256=AXIOM_ARCHIVE_SHA256,
    ),
    "2026": ReleaseSpec(
        release="2026",
        corpus="CASC-J13 (2026) official competition ATP problems",
        problems_relative=Path("problems/casc_2026"),
        # The imported official result directory retains its published typo.
        results_relative=Path("cast_2026_results"),
        categories=CATEGORIES_J13,
        problem_count=1350,
        partition_salt="umlaut-casc-j13-family-holdout-v1",
        presentation_id="casc-j13-official-obfuscated",
        presentation_description=(
            "Official CASC-J13 presentation, including organizer comment "
            "stripping, formula reordering, connective swaps, and equality "
            "reversals where applicable."
        ),
        design_url="https://tptp.org/CASC/J13/Design.html",
        archive_url="https://tptp.org/CASC/J13/",
    ),
}


def release_spec(release: str) -> ReleaseSpec:
    try:
        return RELEASES[release]
    except KeyError as error:  # pragma: no cover - argparse constrains the CLI.
        raise ManifestError(f"unsupported CASC release {release!r}") from error


class ManifestError(RuntimeError):
    """Raised when the corpus or manifest violates the frozen contract."""


def sha256_bytes(data: bytes) -> str:
    """Return a lowercase SHA-256 digest."""

    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    """Hash one file without loading it all into memory."""

    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_sha256(paths: Iterable[Path], root: Path) -> str:
    """Hash sorted relative paths and file digests as one immutable tree."""

    digest = hashlib.sha256()
    for path in sorted(paths):
        relative = path.relative_to(root).as_posix().encode()
        digest.update(relative)
        digest.update(b"\0")
        digest.update(sha256_file(path).encode())
        digest.update(b"\0")
    return digest.hexdigest()


def normalize_problem_id(value: str) -> str:
    """Normalize typography added by the published HTML result tables."""

    normalized = unicodedata.normalize("NFKC", value).translate(_DASH_TRANSLATION)
    normalized = normalized.strip()
    normalized = re.sub(r"\*+$", "", normalized)
    if normalized.lower().endswith(".p"):
        normalized = normalized[:-2]
    return normalized


def header_fields(text: str) -> dict[str, str]:
    """Read the small amount of retained TPTP header metadata, if present."""

    result: dict[str, str] = {}
    for match in _HEADER_FIELD_RE.finditer(text):
        result.setdefault(match.group("key").lower(), match.group("value").strip())
    return result


def source_family(category: str, problem_id: str, text: str) -> str:
    """Return the indivisible family key used by held-out partitions."""

    if category == "SLH":
        names = header_fields(text).get("names", "")
        theory = names.split("/", maxsplit=1)[0].strip()
        if theory:
            return f"SLH:{theory}"
    match = re.match(r"[A-Za-z]{3}", problem_id)
    if not match:
        raise ManifestError(f"cannot derive source family from {problem_id!r}")
    return match.group(0).upper()


def partition_for_family(family: str, salt: str = PARTITION_SALT) -> str:
    """Return the initial deterministic 70/15/15 assignment for one family."""

    material = f"{salt}\0{family}".encode()
    bucket = int.from_bytes(hashlib.sha256(material).digest()[:8], "big") % 10_000
    if bucket < 7_000:
        return "train"
    if bucket < 8_500:
        return "validation"
    return "test"


def family_partition_map(
    records: Sequence[dict[str, Any]], *, salt: str = PARTITION_SALT
) -> dict[str, str]:
    """Assign whole families, repairing hashes so every category has each split."""

    split_names = ("train", "validation", "test")
    target_fraction = {"train": 0.70, "validation": 0.15, "test": 0.15}
    family_category_counts: dict[str, dict[str, int]] = {}
    category_totals: dict[str, int] = {}
    for record in records:
        family = record["family"]
        category = record["category"]
        counts = family_category_counts.setdefault(family, {})
        counts[category] = counts.get(category, 0) + 1
        category_totals[category] = category_totals.get(category, 0) + 1
    assignment = {
        family: partition_for_family(family, salt) for family in family_category_counts
    }

    def family_presence() -> dict[str, dict[str, int]]:
        result = {
            category: {split: 0 for split in split_names}
            for category in category_totals
        }
        for family, category_counts in family_category_counts.items():
            split = assignment[family]
            for category in category_counts:
                result[category][split] += 1
        return result

    def objective() -> float:
        counts = {
            category: {split: 0 for split in split_names}
            for category in category_totals
        }
        for family, category_counts in family_category_counts.items():
            split = assignment[family]
            for category, count in category_counts.items():
                counts[category][split] += count
        score = 0.0
        for category, total in category_totals.items():
            for split in split_names:
                target = total * target_fraction[split]
                score += ((counts[category][split] - target) ** 2) / max(target, 1)
        return score

    category_count = len({record["category"] for record in records})
    for _iteration in range(len(family_category_counts) * category_count):
        presence = family_presence()
        missing = [
            (category, split)
            for category in sorted(presence)
            for split in split_names
            if presence[category][split] == 0
        ]
        if not missing:
            return assignment
        category, required_split = missing[0]
        candidates: list[tuple[float, str, str]] = []
        for family, category_counts in family_category_counts.items():
            old_split = assignment[family]
            if category not in category_counts or old_split == required_split:
                continue
            if any(
                presence[affected_category][old_split] <= 1
                for affected_category in category_counts
            ):
                continue
            assignment[family] = required_split
            candidates.append((objective(), family, old_split))
            assignment[family] = old_split
        if not candidates:
            raise ManifestError(
                f"cannot give category {category} a {required_split} family "
                "without splitting another category's only family"
            )
        _score, chosen, old_split = min(candidates)
        if assignment[chosen] != old_split:
            raise AssertionError("partition candidate changed during scoring")
        assignment[chosen] = required_split
    raise ManifestError("family partition repair did not converge")


def difficulty_band(index: int, count: int) -> str:
    """Map official within-category order to one of five ordinal bands."""

    if index < 1 or index > count:
        raise ManifestError(f"invalid difficulty index {index}/{count}")
    band = min(5, ((index - 1) * 5 // count) + 1)
    return f"q{band}"


def parse_includes(text: str) -> list[str]:
    """Extract include paths in source order."""

    return [match.group("path") for match in _INCLUDE_RE.finditer(text)]


def _result_rows(results_root: Path, category: str) -> list[str]:
    path = results_root / f"category_{category.lower()}_results.csv"
    try:
        with path.open(encoding="utf-8-sig", newline="") as source:
            rows = list(csv.DictReader(source))
    except OSError as error:
        raise ManifestError(f"cannot read official results {path}: {error}") from error
    if not rows or "problem" not in rows[0]:
        raise ManifestError(f"official results have no problem column: {path}")
    problem_ids = [normalize_problem_id(row["problem"]) for row in rows]
    if len(problem_ids) != len(set(problem_ids)):
        raise ManifestError(f"official results contain duplicate problems: {path}")
    return problem_ids


def _corpus_files(
    problems_root: Path, categories: dict[str, Category] = CATEGORIES
) -> dict[str, dict[str, Path]]:
    files: dict[str, dict[str, Path]] = {}
    for category in categories:
        category_root = problems_root / category
        if not category_root.is_dir():
            raise ManifestError(f"missing category directory: {category_root}")
        entries: dict[str, Path] = {}
        for path in sorted(category_root.glob("*.p")):
            problem_id = normalize_problem_id(path.stem)
            if problem_id in entries:
                raise ManifestError(f"duplicate normalized problem path: {path}")
            entries[problem_id] = path
        files[category] = entries
    return files


def _relative_posix(path: Path, root: Path) -> str:
    return path.relative_to(root).as_posix()


def _metadata(
    repo_root: Path, records: list[dict[str, Any]], spec: ReleaseSpec
) -> dict[str, Any]:
    results_root = repo_root / spec.results_relative
    problems_root = repo_root / spec.problems_relative
    axiom_files = list((problems_root / "Axioms").rglob("*.ax"))
    problem_files = list(problems_root.glob("*/*.p"))
    result_files = sorted(results_root.glob("*_results.csv"))
    summary_files = sorted(results_root.glob("*_summary.csv"))
    partitions: dict[str, int] = {}
    categories: dict[str, int] = {}
    divisions: dict[str, int] = {}
    families: set[str] = set()
    for record in records:
        partitions[record["holdout_split"]] = (
            partitions.get(record["holdout_split"], 0) + 1
        )
        categories[record["category"]] = categories.get(record["category"], 0) + 1
        divisions[record["division"]] = divisions.get(record["division"], 0) + 1
        families.add(record["family"])
    if spec.release == "2025":
        resource_policy = {
            "memory_limit_mib": 131072,
            "wall_seconds": {
                "THF": 240,
                "TFA": 120,
                "TFN": 120,
                "FOF": 240,
                "EPR": 120,
                "UEQ": 240,
                "ICU": 480,
            },
            "cpu_seconds": {"SLH": 15},
        }
        sources: dict[str, Any] = {
            "design_url": spec.design_url,
            "archive_url": spec.archive_url,
            "problem_archive_sha256": spec.problem_archive_sha256,
            "axiom_archive_sha256": spec.axiom_archive_sha256,
            "axiom_count": len(axiom_files),
            "axiom_tree_sha256": tree_sha256(axiom_files, repo_root),
            "official_result_file_sha256": {
                _relative_posix(path, repo_root): sha256_file(path)
                for path in [*result_files, *summary_files]
            },
        }
    else:
        resource_policy = {
            "memory_limit_mib": 131072,
            "wall_seconds": {
                division: 180
                for division in sorted({value.division for value in spec.categories.values()})
            },
            "cpu_seconds": {},
            "limit_evidence": (
                "CASC-J13 imposed wall limits, published a 120-second minimum, "
                "and the official result tables contain accepted runs through "
                "180.00 seconds; this release contract uses the announced "
                "180-second competition boundary."
            ),
        }
        sources = {
            "design_url": spec.design_url,
            "archive_url": spec.archive_url,
            "corpus_root": spec.problems_relative.as_posix(),
            "problem_tree_sha256": tree_sha256(problem_files, repo_root),
            "axiom_count": len(axiom_files),
            "axiom_tree_sha256": tree_sha256(axiom_files, repo_root),
            "official_result_file_sha256": {
                _relative_posix(path, repo_root): sha256_file(path)
                for path in [*result_files, *summary_files]
            },
            "official_result_context": (
                "All 26 published CSVs are hashed; PRV is contextual and is "
                "not part of the Problems.tgz ATP corpus."
            ),
        }
    return {
        "record_type": "manifest",
        "schema_version": SCHEMA_VERSION,
        "kind": MANIFEST_KIND,
        "corpus": spec.corpus,
        "problem_count": len(records),
        "family_count": len(families),
        "category_counts": dict(sorted(categories.items())),
        "division_counts": dict(sorted(divisions.items())),
        "partition_counts": dict(sorted(partitions.items())),
        "partition_policy": {
            "unit": "complete source family",
            "salt": spec.partition_salt,
            "thresholds_per_10000": {
                "train": [0, 6999],
                "validation": [7000, 8499],
                "test": [8500, 9999],
            },
            "slh_family_source": "first path component of retained TPTP Names field",
            "other_family_source": "three-letter TPTP or entrant prefix",
            "category_coverage": (
                "Initial hashes are deterministically repaired, without splitting "
                "families, until every category has train, validation, and test "
                "families."
            ),
        },
        "difficulty_policy": {
            "kind": "official_within_category_order_proxy",
            "bands": 5,
            "warning": (
                "Ordinal proxy only; CASC stripped TPTP headers. It is not a "
                "numeric TPTP rating and must not be used as one."
            ),
        },
        "resource_policy": resource_policy,
        "presentation": {
            "id": spec.presentation_id,
            "description": spec.presentation_description,
        },
        "sources": sources,
    }


def build_manifest(
    repo_root: Path, release: str = "2025"
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Build the complete manifest from the checked-in corpus and CSVs."""

    repo_root = repo_root.resolve()
    spec = release_spec(release)
    problems_root = repo_root / spec.problems_relative
    results_root = repo_root / spec.results_relative
    files = _corpus_files(problems_root, spec.categories)
    records: list[dict[str, Any]] = []
    seen: set[str] = set()

    for category, details in spec.categories.items():
        official_ids = _result_rows(results_root, category)
        category_files = files[category]
        missing = sorted(set(official_ids) - set(category_files))
        extra = sorted(set(category_files) - set(official_ids))
        if missing or extra:
            raise ManifestError(
                f"{category} corpus/result mismatch: missing={missing[:5]}, "
                f"extra={extra[:5]}"
            )
        for order, problem_id in enumerate(official_ids, start=1):
            if problem_id in seen:
                raise ManifestError(f"problem occurs in multiple categories: {problem_id}")
            seen.add(problem_id)
            path = category_files[problem_id]
            data = path.read_bytes()
            try:
                text = data.decode("utf-8")
            except UnicodeDecodeError as error:
                raise ManifestError(f"problem is not UTF-8: {path}") from error
            fields = header_fields(text)
            family = source_family(category, problem_id, text)
            records.append(
                {
                    "record_type": "problem",
                    "problem_id": problem_id,
                    "path": _relative_posix(path, repo_root),
                    "sha256": sha256_bytes(data),
                    "size_bytes": len(data),
                    "category": category,
                    "division": details.division,
                    "expected_class": details.expected_class,
                    "limit_kind": details.limit_kind,
                    "limit_seconds": details.limit_seconds,
                    "source_kind": details.source,
                    "family": family,
                    "official_category_order": order,
                    "official_category_count": len(official_ids),
                    "difficulty_band": difficulty_band(order, len(official_ids)),
                    "header_status": fields.get("status"),
                    "header_rating": fields.get("rating"),
                    "includes": parse_includes(text),
                    "presentation_id": spec.presentation_id,
                }
            )

    if len(records) != spec.problem_count:
        raise ManifestError(
            f"expected {spec.problem_count} problems, found {len(records)}"
        )
    partitions = family_partition_map(records, salt=spec.partition_salt)
    for record in records:
        record["holdout_split"] = partitions[record["family"]]
    all_problem_files = list(problems_root.glob("*/*.p"))
    if len(all_problem_files) != len(records):
        raise ManifestError(
            f"corpus contains {len(all_problem_files)} .p files, manifest has "
            f"{len(records)}"
        )
    return _metadata(repo_root, records, spec), records


def manifest_bytes(metadata: dict[str, Any], records: Iterable[dict[str, Any]]) -> bytes:
    """Serialize the deterministic JSON Lines representation."""

    lines = [json.dumps(metadata, sort_keys=True, separators=(",", ":"))]
    lines.extend(
        json.dumps(record, sort_keys=True, separators=(",", ":")) for record in records
    )
    return ("\n".join(lines) + "\n").encode()


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Load and structurally validate a manifest."""

    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ManifestError(f"cannot read manifest {path}: {error}") from error
    if not lines:
        raise ManifestError(f"empty manifest: {path}")
    try:
        values = [json.loads(line) for line in lines]
    except json.JSONDecodeError as error:
        raise ManifestError(f"invalid JSON Lines manifest {path}: {error}") from error
    metadata, *records = values
    if metadata.get("record_type") != "manifest":
        raise ManifestError("first manifest record is not metadata")
    if metadata.get("schema_version") != SCHEMA_VERSION:
        raise ManifestError(
            f"unsupported manifest schema {metadata.get('schema_version')!r}"
        )
    if metadata.get("kind") != MANIFEST_KIND:
        raise ManifestError(f"unexpected manifest kind {metadata.get('kind')!r}")
    if metadata.get("problem_count") != len(records):
        raise ManifestError("manifest problem_count does not match record count")
    problem_ids: set[str] = set()
    for record in records:
        if record.get("record_type") != "problem":
            raise ManifestError("non-problem record after manifest metadata")
        problem_id = record.get("problem_id")
        if not isinstance(problem_id, str) or problem_id in problem_ids:
            raise ManifestError(f"duplicate or invalid problem ID {problem_id!r}")
        problem_ids.add(problem_id)
    return metadata, records


def verify_corpus(
    repo_root: Path, metadata: dict[str, Any], records: Sequence[dict[str, Any]]
) -> None:
    """Verify that every manifest problem and include still matches the corpus."""

    repo_root = repo_root.resolve()
    problems_root = repo_root / metadata["sources"].get(
        "corpus_root", "problems/casc_2025"
    )
    axiom_files = list((problems_root / "Axioms").rglob("*.ax"))
    problem_files = list(problems_root.glob("*/*.p"))
    if len(axiom_files) != metadata["sources"]["axiom_count"]:
        raise ManifestError(
            f"axiom count mismatch: {len(axiom_files)} != "
            f"{metadata['sources']['axiom_count']}"
        )
    if tree_sha256(axiom_files, repo_root) != metadata["sources"]["axiom_tree_sha256"]:
        raise ManifestError("axiom tree hash does not match the manifest")
    expected_problem_tree = metadata["sources"].get("problem_tree_sha256")
    if (
        expected_problem_tree is not None
        and tree_sha256(problem_files, repo_root) != expected_problem_tree
    ):
        raise ManifestError("problem tree hash does not match the manifest")
    for record in records:
        path = repo_root / record["path"]
        if not path.is_file():
            raise ManifestError(f"manifest problem is missing: {path}")
        if sha256_file(path) != record["sha256"]:
            raise ManifestError(f"manifest problem hash mismatch: {path}")
        for include in record["includes"]:
            include_path = problems_root / include
            if not include_path.is_file():
                raise ManifestError(
                    f"{record['problem_id']} has missing include {include!r}"
                )
    if metadata["problem_count"] != len(records):
        raise ManifestError("metadata count changed during corpus verification")


def default_output(repo_root: Path, release: str = "2025") -> Path:
    return repo_root / "benchmarks" / f"casc_{release}_manifest.jsonl"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--release", choices=sorted(RELEASES), default="2025")
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail unless the existing output exactly matches regenerated bytes",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    repo_root = arguments.repo_root.resolve()
    output = (
        arguments.output.resolve()
        if arguments.output
        else default_output(repo_root, arguments.release).resolve()
    )
    try:
        metadata, records = build_manifest(repo_root, arguments.release)
        expected = manifest_bytes(metadata, records)
        if arguments.check:
            if not output.is_file():
                raise ManifestError(f"manifest does not exist: {output}")
            actual = output.read_bytes()
            if actual != expected:
                raise ManifestError(
                    f"manifest is stale: {output}; regenerate without --check"
                )
        else:
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_bytes(expected)
        print(
            f"OK: {len(records)} CASC {arguments.release} problems, "
            f"manifest SHA-256 {sha256_bytes(expected)}"
        )
        return 0
    except ManifestError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
