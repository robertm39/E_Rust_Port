#!/usr/bin/env python3
"""Prepare an exact-source-leaf THF proof view for Nörgler 1.1.

Nörgler 1.1 parses THF and semantically re-proves inference steps, but its
source-leaf comparison does not recognize alpha-equivalent variable names or
associative parenthesis changes. This adapter replaces a file-cited leaf body
with its exact cited source only when both have the same variable-incidence and
non-parenthesis token streams. Nörgler then checks that exact source leaf and
semantically re-proves every descendant from it.
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
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


def load_module(name: str, path: Path) -> ModuleType:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load module: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


BASE = load_module("higher_order_gap_norgler_adapter_base", BASE_ADAPTER_PATH)
AdapterError = BASE.AdapterError


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def variable_order(tokens: list[str]) -> list[str]:
    variables: list[str] = []
    for token in tokens:
        if BASE.VARIABLE_RE.fullmatch(token) and token not in variables:
            variables.append(token)
    return variables


def structural_signature(tokens: list[str]) -> tuple[str, ...]:
    variables: dict[str, str] = {}
    signature = []
    for token in tokens:
        if token in {"(", ")"}:
            continue
        if BASE.VARIABLE_RE.fullmatch(token):
            token = variables.setdefault(
                token, f"VAR{len(variables) + 1}"
            )
        signature.append(token)
    return tuple(signature)


def source_rewrite(
    proof_body: str, source_body: str
) -> tuple[str, dict[str, str]]:
    proof_tokens = BASE.tokenize_formula(proof_body)
    source_tokens = BASE.tokenize_formula(source_body)
    if structural_signature(proof_tokens) != structural_signature(source_tokens):
        raise AdapterError(
            "proof/source token streams differ beyond variable spelling "
            "and redundant parentheses"
        )
    proof_variables = variable_order(proof_tokens)
    source_variables = variable_order(source_tokens)
    if len(proof_variables) != len(source_variables):
        raise AdapterError(
            "proof/source variable counts differ: "
            f"{len(proof_variables)} != {len(source_variables)}"
        )
    mapping = dict(zip(proof_variables, source_variables, strict=True))
    rewritten = source_body
    if structural_signature(BASE.tokenize_formula(rewritten)) != structural_signature(
        source_tokens
    ):
        raise AdapterError("rewritten source leaf failed its structural audit")
    return rewritten, mapping


def adapt_norgler_sources(
    *, proof_text: str, proof_base: Path
) -> tuple[str, dict[str, Any]]:
    statements = BASE.split_tptp_statements(proof_text)
    parsed = [BASE.parse_annotated(statement) for statement in statements]
    if any(formula is None for formula in parsed):
        raise AdapterError("proof contains a non-annotated TPTP statement")
    formulas = [formula for formula in parsed if formula is not None]
    if not formulas:
        raise AdapterError("proof contains no annotated formulas")
    names = [formula.name for formula in formulas]
    if len(set(names)) != len(names):
        raise AdapterError("proof formula names are not unique")

    cache: dict[Path, dict[str, Any]] = {}
    adapted = []
    leaf_audits = []
    for formula in formulas:
        fields = list(formula.fields)
        file_source = (
            BASE.parse_file_source(fields[3], proof_base)
            if len(fields) >= 4
            else None
        )
        if file_source is None:
            adapted.append(formula)
            continue
        source_path, source_label = file_source
        source = BASE.source_formula(source_path, source_label, cache)
        if formula.kind != source.kind:
            raise AdapterError(
                f"source kind mismatch for {formula.name}: "
                f"{formula.kind} != {source.kind}"
            )
        if formula.role != source.role:
            raise AdapterError(
                f"source role mismatch for {formula.name}: "
                f"{formula.role} != {source.role}"
            )
        rewritten, mapping = source_rewrite(formula.body, source.body)
        fields[2] = rewritten
        adapted.append(formula.with_fields(fields))
        leaf_audits.append(
            {
                "proof_name": formula.name,
                "source_label": source_label,
                "source_path": str(source_path.resolve()),
                "source_role": source.role,
                "variable_mapping": mapping,
                "proof_body_sha256": BASE.sha256_text(formula.body),
                "rewritten_body_sha256": BASE.sha256_text(rewritten),
                "source_body_sha256": BASE.sha256_text(source.body),
                "structural_signature_sha256": BASE.sha256_text(
                    "\x1f".join(
                        structural_signature(BASE.tokenize_formula(source.body))
                    )
                ),
            }
        )
    if not leaf_audits:
        raise AdapterError("proof has no file-cited input leaves")
    prepared = "\n".join(formula.render() for formula in adapted) + "\n"
    body = {
        "schema_version": 1,
        "adapter": "norgler-1.1-thf-exact-source-v1",
        "proof_formula_count": len(formulas),
        "input_leaf_count": len(leaf_audits),
        "changed_fields": ["input_leaf.body.exact_cited_source"],
        "non_parenthesis_token_stream_unchanged": True,
        "inference_sources_unchanged": True,
        "original_proof_sha256": BASE.sha256_text(proof_text),
        "prepared_proof_sha256": BASE.sha256_text(prepared),
        "leaf_audits": leaf_audits,
    }
    return prepared, {
        **body,
        "report_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }


def write_norgler_view(
    *,
    solution_path: Path,
    prepared_path: Path,
    report_path: Path,
) -> dict[str, Any]:
    proof_text = BASE.extract_proof_block(
        solution_path.read_text(encoding="utf-8", errors="strict")
    )
    prepared, report = adapt_norgler_sources(
        proof_text=proof_text,
        proof_base=solution_path.parent,
    )
    prepared_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    prepared_path.write_text(prepared, encoding="utf-8", newline="\n")
    report_path.write_bytes(canonical_json(report) + b"\n")
    return report
