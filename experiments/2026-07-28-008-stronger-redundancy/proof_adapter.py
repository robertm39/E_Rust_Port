#!/usr/bin/env python3
"""Add audited ProofCheck-required Skolem records to Umlaut TSTP proofs."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import re
import sys
from pathlib import Path
from types import ModuleType
from typing import Any


EXPERIMENT_ROOT = Path(__file__).resolve().parent
BASE_ADAPTER_PATH = (
    EXPERIMENT_ROOT.parent
    / "2026-07-28-007-unit-equality-completion"
    / "proof_adapter.py"
)
MISSING_SKOLEM_METADATA_RE = re.compile(
    r"inference\s*\(\s*skolemize\s*,\s*"
    r"\[\s*status\s*\(\s*esa\s*\)\s*\]",
    re.IGNORECASE,
)
SKOLEM_SYMBOL_RE = re.compile(r"\besk\d+_\d+\b")
QUANTIFIER_RE = re.compile(r"([!?])\s*\[([^\]]+)\]")


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("stronger_redundancy_adapter_base", BASE_ADAPTER_PATH)


class AdapterError(RuntimeError):
    """A proof cannot be annotated without changing its logical fields."""


def sha256_text(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def skolem_symbols(value: str) -> set[str]:
    return set(SKOLEM_SYMBOL_RE.findall(value))


def cited_parent_symbols(
    source: str, formulas: dict[str, str]
) -> set[str]:
    symbols: set[str] = set()
    for name, body in formulas.items():
        pattern = (
            r"(?<![A-Za-z0-9_$])"
            + re.escape(name)
            + r"(?![A-Za-z0-9_$])"
        )
        if re.search(pattern, source):
            symbols.update(skolem_symbols(body))
    return symbols


def symbol_order(symbol: str) -> tuple[int, int, str]:
    match = re.fullmatch(r"esk(\d+)_(\d+)", symbol)
    if match is None:
        raise AdapterError(f"unsupported Skolem symbol: {symbol}")
    return int(match.group(1)), int(match.group(2)), symbol


def quantifier_environment(
    formula: str,
) -> tuple[list[str], dict[str, tuple[str, ...]]]:
    universals: list[str] = []
    existential_arguments: dict[str, tuple[str, ...]] = {}
    for kind, raw_variables in QUANTIFIER_RE.findall(formula):
        variables = [
            variable.strip().split(":", 1)[0].strip()
            for variable in raw_variables.split(",")
        ]
        if any(not variable for variable in variables):
            raise AdapterError("empty quantified variable")
        if kind == "!":
            universals.extend(variables)
        else:
            for variable in variables:
                existential_arguments[variable] = tuple(universals)
    return universals, existential_arguments


def skolem_records(
    source: str,
    formulas: dict[str, str],
    introduced: list[str],
) -> tuple[str, list[str], dict[str, str]]:
    candidates = []
    for name, body in formulas.items():
        pattern = (
            r"(?<![A-Za-z0-9_$])"
            + re.escape(name)
            + r"(?![A-Za-z0-9_$])"
        )
        if re.search(pattern, source):
            _, environment = quantifier_environment(body)
            if len(environment) == len(introduced):
                candidates.append((name, environment))
    if len(candidates) != 1:
        raise AdapterError(
            "expected one cited parent with "
            f"{len(introduced)} existential variables, found "
            f"{[name for name, _ in candidates]}"
        )
    parent_name, environment = candidates[0]
    records = []
    replacements = {}
    for variable, symbol in zip(
        environment, sorted(introduced, key=symbol_order), strict=True
    ):
        arguments = environment[variable]
        term = (
            symbol
            if not arguments
            else f"{symbol}({','.join(arguments)})"
        )
        records.append(f"skolemize({variable},{term})")
        replacements[variable] = term
    return parent_name, records, replacements


def replace_variable_tokens(value: str, replacements: dict[str, str]) -> str:
    result = value
    for variable in sorted(replacements, key=len, reverse=True):
        result = re.sub(
            r"(?<![A-Za-z0-9_$])"
            + re.escape(variable)
            + r"(?![A-Za-z0-9_$])",
            replacements[variable],
            result,
        )
    return result


def skolemized_intermediate_body(
    parent_body: str,
    result_body: str,
    existential_replacements: dict[str, str],
) -> str:
    parent_universals, _ = quantifier_environment(parent_body)
    result_universals, _ = quantifier_environment(result_body)
    if len(parent_universals) != len(result_universals):
        raise AdapterError(
            "parent/result universal-variable counts differ: "
            f"{len(parent_universals)} != {len(result_universals)}"
        )
    universal_rename = dict(
        zip(parent_universals, result_universals, strict=True)
    )
    replacements = {
        variable: replace_variable_tokens(term, universal_rename)
        for variable, term in existential_replacements.items()
    }
    replacements.update(universal_rename)
    without_existentials = re.sub(
        r"\?\s*\[[^\]]+\]\s*:\s*",
        "",
        parent_body,
    )
    return replace_variable_tokens(without_existentials, replacements)


def add_skolem_metadata(proof_text: str) -> tuple[str, dict[str, Any]]:
    statements = BASE.split_tptp_statements(proof_text)
    parsed = [BASE.parse_annotated(statement) for statement in statements]
    formulas = {
        item.name: item.body for item in parsed if item is not None
    }
    output: list[str] = []
    changes = []
    for original, item in zip(statements, parsed, strict=True):
        if item is None or not MISSING_SKOLEM_METADATA_RE.search(
            item.fields[3] if len(item.fields) > 3 else ""
        ):
            output.append(original.strip())
            continue
        if len(item.fields) < 4:
            raise AdapterError(
                f"skolemized statement lacks a source field: {item.name}"
            )
        source = item.fields[3]
        current_symbols = skolem_symbols(item.body)
        parent_symbols = cited_parent_symbols(source, formulas)
        introduced = sorted(current_symbols - parent_symbols)
        if not introduced:
            raise AdapterError(
                f"cannot identify a new Skolem symbol for {item.name}"
            )
        parent_name, records, existential_replacements = skolem_records(
            source, formulas, introduced
        )
        useful_info = ",".join(
            [
                "status(esa)",
                f"new_symbols(skolem,[{','.join(introduced)}])",
                *records,
            ]
        )
        top_level_skolemize = re.match(
            r"^\s*inference\s*\(\s*skolemize\b",
            source,
            re.IGNORECASE,
        )
        if top_level_skolemize is None:
            inference_rules = {
                rule.lower()
                for rule in re.findall(
                    r"inference\s*\(\s*([A-Za-z0-9_]+)",
                    source,
                    re.IGNORECASE,
                )
            }
            allowed = {
                "distribute",
                "fof_nnf",
                "skolemize",
                "variable_rename",
            }
            if not inference_rules or not inference_rules <= allowed:
                raise AdapterError(
                    f"unsupported compound skolem source in {item.name}: "
                    f"{sorted(inference_rules)}"
                )
            intermediate_name = f"{item.name}_skolem"
            if intermediate_name in formulas:
                raise AdapterError(
                    f"intermediate name collision: {intermediate_name}"
                )
            intermediate_body = skolemized_intermediate_body(
                formulas[parent_name],
                item.body,
                existential_replacements,
            )
            intermediate = item.with_fields(
                [
                    intermediate_name,
                    "plain",
                    intermediate_body,
                    f"inference(skolemize,[{useful_info}],[{parent_name}])",
                ]
            )
            output.append(intermediate.render())
            rewritten_source = (
                "inference(distribute,[status(thm)],"
                f"[{intermediate_name}])"
            )
            count = 1
            compound_source_split = True
        else:
            replacement = f"inference(skolemize,[{useful_info}]"
            rewritten_source, count = MISSING_SKOLEM_METADATA_RE.subn(
                replacement, source
            )
            if count < 1:
                raise AdapterError(
                    f"failed to annotate skolemization in {item.name}"
                )
            compound_source_split = False
        fields = list(item.fields)
        fields[3] = rewritten_source
        rewritten = item.with_fields(fields)
        if (
            rewritten.kind != item.kind
            or rewritten.name != item.name
            or rewritten.role != item.role
            or rewritten.body != item.body
        ):
            raise AdapterError(
                f"logical fields changed while annotating {item.name}"
            )
        output.append(rewritten.render())
        changes.append(
            {
                "step": item.name,
                "occurrences": count,
                "introduced_symbols": introduced,
                "skolemize_records": records,
                "compound_source_split": compound_source_split,
                "intermediate_step": (
                    intermediate_name
                    if compound_source_split
                    else None
                ),
                "formula_sha256": sha256_text(item.body),
                "source_before_sha256": sha256_text(source),
                "source_after_sha256": sha256_text(rewritten_source),
            }
        )
    prepared = "\n".join(output) + "\n"
    report = {
        "schema_version": 1,
        "adapter": "proofcheck-skolem-records-v1",
        "original_proof_sha256": sha256_text(proof_text),
        "prepared_proof_sha256": sha256_text(prepared),
        "statement_count": len(statements),
        "changed_statement_count": len(changes),
        "logical_formula_fields_unchanged": True,
        "changes": changes,
    }
    return prepared, report


def write_proofcheck_view(
    *,
    solution_path: Path,
    prepared_path: Path,
    report_path: Path,
) -> dict[str, Any]:
    solution_text = solution_path.read_text(
        encoding="utf-8", errors="strict"
    )
    proof_text = BASE.extract_proof_block(solution_text)
    prepared, report = add_skolem_metadata(proof_text)
    prepared_path.parent.mkdir(parents=True, exist_ok=True)
    prepared_path.write_text(prepared, encoding="utf-8", newline="\n")
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_bytes(
        json.dumps(
            report,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=True,
        ).encode("utf-8")
        + b"\n"
    )
    return report
