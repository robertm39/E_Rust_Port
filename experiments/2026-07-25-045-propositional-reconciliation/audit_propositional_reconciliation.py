#!/usr/bin/env python3
"""Audit the final PROPOSITIONAL Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [989, 993, 994, 996, 999]


def load_backlog_audit(repo: Path):
    path = (
        repo
        / "experiments/2026-07-25-029-post-compat-backlog-audit/audit_backlog.py"
    )
    spec = importlib.util.spec_from_file_location("post_compat_backlog_audit", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load the post-compatibility audit module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def source(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    text = source(repo, relative)
    return all(needle in text for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    audit = load_backlog_audit(repo)
    issues = audit.load_children("E_Rust_Port-j76.4")
    records = [
        audit.issue_record("E_Rust_Port-j76.4", issue) for issue in issues
    ]
    audit.validate_parent("E_Rust_Port-j76.4", records)
    expected_ids = {f"E_Rust_Port-j76.4.{ordinal}" for ordinal in ORDINALS}
    selected = sorted(
        (record for record in records if record["id"] in expected_ids),
        key=lambda record: record["ordinal"],
    )
    issues_by_id = {issue["id"]: issue for issue in issues}
    stable_records = [
        {
            "content_sha256": record["content_sha256"],
            "id": record["id"],
            "legacy_text": record["legacy_text"],
            "ordinal": record["ordinal"],
            "source_file": record["source_file"],
        }
        for record in selected
    ]
    decision_digest = hashlib.sha256(
        json.dumps(
            stable_records, sort_keys=True, separators=(",", ":")
        ).encode("utf-8")
    ).hexdigest()

    dpll_c = source(repo, "eprover/PROPOSITIONAL/cpr_dpll.c")
    checks = {
        "undefined_retraction_is_outside_the_reference_contract": contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_dpll.h",
            "void      DPLLRetractLastAss(DPLLState_p state);",
        )
        and "DPLLRetractLastAss(" not in dpll_c
        and contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_dpll.c",
            "long deactivate_clauses(DPLLState_p state, PTree_p *clauses)",
            "long shorten_clauses(DPLLState_p state, PTree_p *clauses)",
            "return res;",
        )
        and contains(
            repo,
            "eprover/PROVER/edpll.c",
            "dpllstate = DPLLStateAlloc(form);",
            "UNUSED(dpllstate); /* Stiffle warning for now */",
        )
        and contains(
            repo,
            "src/propositional/dpll.rs",
            "fn deactivate_clauses(",
            "fn shorten_clauses(",
            "fn assign_var_pushes_assignment_and_marker_and_returns_stub_false()",
        ),
        "explicit_parser_bank_preserves_borrowed_signature_behavior": contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_dpllformula.c",
            "TB_p terms = TBAlloc(sig);",
            "terms->sig = NULL;",
            "TBFree(terms);",
        )
        and contains(
            repo,
            "src/propositional/dpllformula.rs",
            "C `DPLLFormulaParseLOP`, adapted to Rust's explicit `TermBank`.",
            "bank: &mut TermBank,",
            "fn parse_lop_accepts_normalized_clauses_and_discards_tautologies()",
        ),
        "deterministic_normalization_replaces_nonportable_c_comparator": contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_propclauses.c",
            "abs_a1 = ABS(*a1);",
            "abs_a2 = ABS(*a1);",
            "qsort(clause->literals, clause->lit_no, sizeof(PLiteralCode),",
        )
        and contains(
            repo,
            "src/propositional/propclauses.rs",
            "sort_by(|left, right| compare_literals(*left, *right))",
            "fn normalize_sorts_deduplicates_and_keeps_storage()",
            "assert_eq!(clause.literals(), &[1, -1, 2, 3, -3]);",
        )
        and contains(
            repo,
            "experiments/2026-07-17-002-edpll-diagnostic-parity/FINDINGS.md",
            "C's `p_atom_compare` typo",
            "non-strict comparator",
            "ordering to reproduce.",
            "direct normalization tests and integrated output",
        ),
        "normalization_retains_allocated_literal_storage": contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_propclauses.c",
            "Does not reduce size of literal",
            "handle->mem_size  = lit_no*sizeof(PLiteralCode);",
            "clause->lit_no = clause->active_no = to+1;",
        )
        and contains(
            repo,
            "src/propositional/propclauses.rs",
            "pub fn storage_len(&self) -> usize",
            "assert_eq!(clause.storage_len(), 7);",
            "assert_eq!(clause.lit_no(), 5);",
        ),
        "safe_owned_propositional_names_preserve_bimap_behavior": contains(
            repo,
            "eprover/PROPOSITIONAL/cpr_propsig.c",
            "handle->key = SecureStrdup(name);",
            "PStackPushP(psig->enc_to_name,handle->key);",
            "StrTreeInsert(&(psig->name_to_enc), handle);",
        )
        and contains(
            repo,
            "src/propositional/propsig.rs",
            "enc_to_name: Vec<Option<String>>",
            "name_to_enc: BTreeMap<String, PLiteralCode>",
            "self.enc_to_name.push(Some(owned_name.clone()));",
            "let replaced = self.name_to_enc.insert(owned_name, enc);",
            "fn insertion_assigns_codes_from_reserved_stack_top()",
        ),
        "exact_edpll_and_full_port_validation_are_current": contains(
            repo,
            "experiments/2026-07-17-002-edpll-diagnostic-parity/FINDINGS.md",
            "all 15 permanent `edpll` cases ran",
            "zero mismatches and zero expected differences",
        )
        and contains(
            repo,
            "experiments/2026-07-25-041-detailed-terms-reconciliation/validation-reference.json",
            '"rust_test_count": 4427',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "The complete implemented propositional `cpr_dpll` state-shell behavior",
            "implementing propagation, retraction, or SAT/UNSAT output would be a post-compatibility extension",
        ),
    }
    source_files = [
        "eprover/PROPOSITIONAL/cpr_dpll.c",
        "eprover/PROPOSITIONAL/cpr_dpll.h",
        "eprover/PROPOSITIONAL/cpr_dpllformula.c",
        "eprover/PROPOSITIONAL/cpr_propclauses.c",
        "eprover/PROPOSITIONAL/cpr_propsig.c",
        "eprover/PROVER/edpll.c",
        "src/propositional/dpll.rs",
        "src/propositional/dpllformula.rs",
        "src/propositional/propclauses.rs",
        "src/propositional/propsig.rs",
        "experiments/2026-07-17-002-edpll-diagnostic-parity/FINDINGS.md",
        "docs/rust-port-status.md",
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "evidence_checks": checks,
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_ids = {record["id"] for record in selected}
    selected_are_propositional = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "propositional"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 5
        or report["content_hashes_verified"] != 5
        or not selected_are_propositional
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("PROPOSITIONAL reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
