#!/usr/bin/env python3
"""Freeze the syntax-only corpus for the bounded Inst-Gen-style study."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from collections import Counter
from pathlib import Path
from types import ModuleType
from typing import Any


SELECTION_SALT = "umlaut-instgen-epr-v1"
MAX_SOURCE_BYTES = 200_000
MAX_CLAUSES = 1_500
MAX_CONSTANTS = 512
MAX_PREDICATES = 4_096
MAX_VARIABLES_PER_CLAUSE = 64
MAX_LITERALS_PER_CLAUSE = 128
QUOTAS = {
    ("train", "GRP", "satisfiable"): 3,
    ("train", "SYN", "satisfiable"): 3,
    ("train", "NLP", "satisfiable"): 2,
    ("train", "MSC", "unsatisfiable"): 3,
    ("validation", "PUZ", "satisfiable"): 6,
    ("validation", "PUZ", "unsatisfiable"): 4,
    ("validation", "SWV", "unsatisfiable"): 6,
    ("test", "PLA", "unsatisfiable"): 2,
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_module(name: str, path: Path) -> ModuleType:
    specification = importlib.util.spec_from_file_location(name, path)
    if specification is None or specification.loader is None:
        raise RuntimeError(f"cannot load parser module {path}")
    module = importlib.util.module_from_spec(specification)
    sys.modules[name] = module
    specification.loader.exec_module(module)
    return module


def is_variable(symbol: str) -> bool:
    return (
        not symbol.startswith(("'", '"'))
        and bool(symbol)
        and (symbol[0].isupper() or symbol[0] == "_")
    )


def stable_score(record: dict[str, Any]) -> str:
    material = "\0".join(
        (
            SELECTION_SALT,
            record["holdout_split"],
            record["family"],
            record["expected_class"],
            record["problem_id"],
            record["sha256"],
        )
    )
    return hashlib.sha256(material.encode("utf-8")).hexdigest()


def syntax_metrics(
    path: Path, split_parser: ModuleType, term_parser: ModuleType
) -> dict[str, Any]:
    statements = split_parser.split_statements(path.read_text(encoding="utf-8"))
    constants: set[str] = set()
    predicates: set[tuple[str, int]] = set()
    variable_counts: list[int] = []
    literal_counts: list[int] = []
    for statement_index, statement in enumerate(statements):
        prefix = statement.partition("(")[0].strip().lower()
        if prefix != "cnf":
            raise ValueError(f"top_level_{prefix or 'empty'}")
        clause = split_parser.parse_cnf_statement(statement, statement_index)
        variables: set[str] = set()
        if len(clause["literals"]) > MAX_LITERALS_PER_CLAUSE:
            raise ValueError("literals_per_clause")
        for literal_text in clause["literals"]:
            literal = term_parser.parse_literal(literal_text)
            if literal.atom.relation == "eq":
                raise ValueError("equality")
            if literal.atom.relation.startswith("$") and literal.atom.relation not in {
                "$true",
                "$false",
            }:
                raise ValueError("interpreted_predicate")
            predicates.add(
                (literal.atom.relation, len(literal.atom.arguments))
            )
            for term in literal.atom.arguments:
                if term.arguments:
                    raise ValueError("positive_arity_function")
                if is_variable(term.symbol):
                    variables.add(term.symbol)
                else:
                    constants.add(term.symbol)
        if len(variables) > MAX_VARIABLES_PER_CLAUSE:
            raise ValueError("variables_per_clause")
        variable_counts.append(len(variables))
        literal_counts.append(len(clause["literals"]))

    if len(statements) > MAX_CLAUSES:
        raise ValueError("clause_count")
    if len(constants) > MAX_CONSTANTS:
        raise ValueError("constant_count")
    if len(predicates) > MAX_PREDICATES:
        raise ValueError("predicate_count")
    domain_size = max(1, len(constants))
    ground_instances = sum(
        domain_size**variable_count for variable_count in variable_counts
    )
    return {
        "clauses": len(statements),
        "constants": len(constants),
        "domain_size": domain_size,
        "predicates": len(predicates),
        "max_variables_per_clause": max(variable_counts, default=0),
        "max_literals_per_clause": max(literal_counts, default=0),
        "ground_instances": str(ground_instances),
    }


def select(
    repo_root: Path, manifest_path: Path
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    split_parser = load_module(
        "instgen_selection_split",
        repo_root
        / "experiments/2026-07-29-008-avatar-restart-prototype/tptp_split.py",
    )
    term_parser = load_module(
        "instgen_selection_terms",
        repo_root
        / "experiments/2026-07-30-002-real-ground-theory-traces/trace_model.py",
    )
    records = [
        json.loads(line)
        for line in manifest_path.read_text(encoding="utf-8").splitlines()
        if line
    ]
    source_header = records[0]
    if source_header.get("record_type") != "manifest":
        raise ValueError("source manifest must start with a manifest record")

    cells: dict[tuple[str, str, str], list[dict[str, Any]]] = {
        cell: [] for cell in QUOTAS
    }
    exclusions: Counter[str] = Counter()
    for source in records[1:]:
        if source.get("record_type") != "problem":
            continue
        cell = (
            str(source.get("holdout_split")),
            str(source.get("family")),
            str(source.get("expected_class")),
        )
        if cell not in cells:
            exclusions["outside_quota_cells"] += 1
            continue
        if source.get("division") != "EPR":
            exclusions["not_epr"] += 1
            continue
        if source.get("includes"):
            exclusions["includes"] += 1
            continue
        if int(source["size_bytes"]) > MAX_SOURCE_BYTES:
            exclusions["source_bytes"] += 1
            continue
        path = repo_root / source["path"]
        if sha256_file(path) != source["sha256"]:
            raise ValueError(f"source hash mismatch: {source['problem_id']}")
        try:
            metrics = syntax_metrics(path, split_parser, term_parser)
        except (OSError, UnicodeError, ValueError, split_parser.SplitError,
                term_parser.TraceError) as error:
            exclusions[f"syntax:{error}"] += 1
            continue
        selected = {
            key: source[key]
            for key in (
                "category",
                "difficulty_band",
                "division",
                "expected_class",
                "family",
                "holdout_split",
                "path",
                "problem_id",
                "sha256",
                "size_bytes",
            )
        }
        selected["selection_score"] = stable_score(source)
        selected["syntax_metrics"] = metrics
        cells[cell].append(selected)

    output: list[dict[str, Any]] = []
    for cell, quota in QUOTAS.items():
        pool = sorted(
            cells[cell],
            key=lambda record: (
                record["selection_score"],
                record["problem_id"],
            ),
        )
        if len(pool) < quota:
            raise ValueError(f"{cell}: need {quota}, found {len(pool)}")
        output.extend(pool[:quota])
    output.sort(
        key=lambda record: (
            ("train", "validation", "test").index(record["holdout_split"]),
            record["family"],
            record["expected_class"],
            record["selection_score"],
            record["problem_id"],
        )
    )

    family_partitions: dict[str, set[str]] = {}
    for record in output:
        family_partitions.setdefault(record["family"], set()).add(
            record["holdout_split"]
        )
    leaking = {
        family: sorted(partitions)
        for family, partitions in family_partitions.items()
        if len(partitions) != 1
    }
    if leaking:
        raise ValueError(f"family leakage: {leaking}")

    header = {
        "record_type": "manifest",
        "schema_version": 1,
        "kind": "umlaut-instgen-epr-corpus",
        "problem_count": len(output),
        "family_count": len(family_partitions),
        "source_manifest": manifest_path.relative_to(repo_root).as_posix(),
        "source_manifest_sha256": sha256_file(manifest_path),
        "selection_policy": {
            "candidate_blind": True,
            "salt": SELECTION_SALT,
            "quotas": {
                "/".join(cell): quota for cell, quota in QUOTAS.items()
            },
            "max_source_bytes": MAX_SOURCE_BYTES,
            "max_clauses": MAX_CLAUSES,
            "max_constants": MAX_CONSTANTS,
            "max_predicates": MAX_PREDICATES,
            "max_variables_per_clause": MAX_VARIABLES_PER_CLAUSE,
            "max_literals_per_clause": MAX_LITERALS_PER_CLAUSE,
            "ranking": (
                "SHA-256(salt, partition, family, expected class, "
                "problem ID, source SHA-256)"
            ),
        },
        "partition_counts": dict(
            sorted(Counter(row["holdout_split"] for row in output).items())
        ),
        "expected_class_counts": dict(
            sorted(Counter(row["expected_class"] for row in output).items())
        ),
        "excluded_source_records": dict(sorted(exclusions.items())),
    }
    return header, output


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
    )
    parser.add_argument(
        "--manifest",
        type=Path,
        default=Path("benchmarks/casc_2025_manifest.jsonl"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(__file__).with_name("corpus.jsonl"),
    )
    arguments = parser.parse_args()
    repo_root = arguments.repo_root.resolve()
    manifest = arguments.manifest
    if not manifest.is_absolute():
        manifest = repo_root / manifest
    header, records = select(repo_root, manifest.resolve())
    arguments.output.resolve().write_text(
        "".join(
            json.dumps(record, sort_keys=True) + "\n"
            for record in (header, *records)
        ),
        encoding="utf-8",
        newline="\n",
    )
    print(
        json.dumps(
            {
                "families": header["family_count"],
                "output": str(arguments.output.resolve()),
                "problems": len(records),
                "sha256": sha256_file(arguments.output.resolve()),
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
