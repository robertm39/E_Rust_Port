#!/usr/bin/env python3
"""Audit the source-level reconciliation of the remaining TERMS reviews."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from types import ModuleType
from typing import Any


DECISION_GROUPS = {
    "c_compatibility_contracts": [
        1220,
        1224,
        1237,
        1259,
        1303,
        1305,
        1324,
    ],
    "deterministic_or_safe_ownership": [
        1218,
        1231,
        1235,
        1246,
        1279,
        1293,
        1311,
        1315,
    ],
    "parser_and_output_boundaries": [
        1280,
        1281,
        1282,
        1289,
        1290,
        1294,
        1316,
        1321,
        1322,
    ],
    "retained_measured_api_or_performance_shape": [
        1256,
        1263,
        1306,
    ],
    "source_parity_or_landed_owner": [
        1254,
        1317,
    ],
}


def load_backlog_audit(repo: Path) -> ModuleType:
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


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = (repo / relative).read_text(encoding="utf-8")
    return all(needle in source for needle in needles)


def function_body(source: str, start: str, end: str) -> str:
    return source.rsplit(start, maxsplit=1)[1].split(end, maxsplit=1)[0]


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

    ordinals = sorted(
        ordinal for group in DECISION_GROUPS.values() for ordinal in group
    )
    if len(ordinals) != len(set(ordinals)):
        raise RuntimeError("decision groups contain duplicate ordinals")
    expected_ids = {f"E_Rust_Port-j76.4.{ordinal}" for ordinal in ordinals}
    selected = sorted(
        (record for record in records if record["id"] in expected_ids),
        key=lambda record: record["ordinal"],
    )
    selected_ids = {record["id"] for record in selected}
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

    c_lambda = (repo / "eprover/TERMS/cte_lambda.c").read_text(encoding="utf-8")
    rust_lambda = (repo / "src/terms/lambda.rs").read_text(encoding="utf-8")
    c_beta = function_body(
        c_lambda,
        "Term_p do_beta_normalize_db(TB_p bank, Term_p t)",
        "// Function: LambdaNormalizeDB",
    )
    rust_beta = function_body(
        rust_lambda,
        "fn do_beta_normalize_db(bank: &mut TermBank, term: &Term)",
        "#[cfg(test)]",
    )

    evidence_checks = {
        "ac_and_fixpoint_compatibility_quirks_are_explicit": contains(
            repo,
            "src/terms/acterms.rs",
            "handle.args.sort_by(",
            "term_standard_weight(left) != term_standard_weight(right)",
            "|| left.is_phony_app()",
            "|| right.is_phony_app()",
        )
        and contains(
            repo,
            "src/terms/fixpoint_unif.rs",
            "if !left.is_free_var() && !right.is_free_var()",
            "return Ok(OracleUnifResult::NotInFragment)",
            "fn fixpoint_mgu_reports_non_variable_pair_outside_fragment()",
        ),
        "fp_and_gc_owners_are_explicit_and_live": contains(
            repo,
            "src/terms/fp_index.rs",
            "pub fn find_unifiable<'a>(",
            "sig: &Signature",
            "self.find_unifiable_rek(key.samples(), sig, 0, collect)",
        )
        and contains(
            repo,
            "src/terms/garbage_coll.rs",
            "pub struct GcSetHandle(usize)",
            "clause_sets: BTreeSet<GcSetHandle>",
            "formula_sets: BTreeSet<GcSetHandle>",
            "fn registration_uses_pointer_identity_sets()",
        ),
        "higher_order_boundaries_and_safe_whnf_bindings_are_preserved": contains(
            repo,
            "src/terms/ho_bindings.rs",
            "let Some(rigid_type) = bank.signature().get_type(rhs.f_code()).cloned() else",
            "return Ok(None)",
        )
        and contains(
            repo,
            "src/terms/lambda.rs",
            "let mut bindings = vec![None; total_bound]",
            "replace_bound_vars(bank, &matrix, &bindings, 0)?",
        )
        and contains(
            repo,
            "src/terms/match_mgu.rs",
            "if !result && t.is_non_fo_pattern() && s.is_non_fo_pattern()",
            "subst_compute_mgu_pattern(bank, t, s, subst)?",
        ),
        "beta_only_rebuilding_matches_c_without_eta_flatten_repair": (
            "flatten_and_make_shared" not in c_beta
            and "flatten_and_make_shared" not in rust_beta
            and "TermTopFree(res)" in c_beta
            and "bank.term_top_insert(copy)" in rust_beta
        ),
        "parser_and_output_boundaries_are_owned_and_explicit": contains(
            repo,
            "src/terms/termbanks.rs",
            "fn parse_ite_tformula_tstp_subset(",
            "recover_untyped_ite_branch_to_expected_sort",
            "terms.sort_by_key(Term::entry_no)",
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "The low-level term/formula parser surface is reconciled through production owners",
            "Residual term-valued FOOL atom recovery and the checked/simple parser split are explicit compatibility grammar boundaries",
            "first-order `$let` rendering through term-bank printing",
        ),
        "term_core_api_and_performance_decisions_remain_measured": contains(
            repo,
            "src/terms/termfunc.rs",
            "pub fn term_collect_variables(",
            "let mut stack = vec![term.clone()]",
            "pub fn term_app_encode(",
            "pub fn term_compute_order(_sig: &Signature, term: &Term)",
        )
        and contains(
            repo,
            "experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md",
            "`1.0801753448x` C",
        ),
        "intrusive_term_tree_owner_is_private_and_noncloneable": contains(
            repo,
            "src/terms/termtrees.rs",
            "externally assembled trees could alias and relink the same cells",
            "#[derive(Debug, Default)]",
            "pub(crate) struct TermTree",
        )
        and not contains(
            repo,
            "src/terms/termtrees.rs",
            "#[derive(Clone, Debug, Default)]",
        ),
        "proof_state_freshvars_owner_and_family_reset_policies_landed": contains(
            repo,
            "src/clauses/proofstate.rs",
            "fresh_vars: VarBank",
            "pub const fn fresh_vars(&self) -> &VarBank",
        )
        and contains(
            repo,
            "src/clauses/eqnresolution.rs",
            "pub fn compute_eq_res_with_fresh_vars(",
            "freshvars.reset_v_counts()",
        )
        and contains(
            repo,
            "src/clauses/factor.rs",
            "pub fn compute_ordered_factor_with_fresh_vars(",
            "pub fn compute_equality_factor_with_fresh_vars(",
            "callers control the count state",
        )
        and contains(
            repo,
            "src/clauses/paramodulation.rs",
            "pub fn compute_all_paramodulants_indexed_with_fresh_vars(",
            "freshvars.reset_v_counts()",
        ),
        "type_output_is_uid_ordered_and_varhash_retains_c_width": contains(
            repo,
            "src/terms/typebanks.rs",
            "selected_sorts.sort_by_key(|type_| type_.type_uid())",
            "types.sort_by_key(|type_| type_.type_uid())",
        )
        and contains(
            repo,
            "src/terms/varhash.rs",
            "pub fn add_value(&mut self, var: &Term, value: i64) -> i64",
            "entry.value += value",
        ),
        "latest_full_compatibility_validation_is_clean": contains(
            repo,
            "experiments/2026-07-25-040-whnf-deref-policy/FINDINGS.md",
            "4,427 total",
            "all 50 main-prover cases have zero unexpected differences",
            "all 216 support-tool cases have zero unexpected differences",
            "The C checkout is clean",
        ),
    }

    source_files = [
        "src/terms/acterms.rs",
        "src/terms/fixpoint_unif.rs",
        "src/terms/fp_index.rs",
        "src/terms/garbage_coll.rs",
        "src/terms/ho_bindings.rs",
        "src/terms/lambda.rs",
        "src/terms/match_mgu.rs",
        "src/terms/termbanks.rs",
        "src/terms/termfunc.rs",
        "src/terms/termtrees.rs",
        "src/terms/typebanks.rs",
        "src/terms/varhash.rs",
        "src/clauses/proofstate.rs",
        "src/clauses/eqnresolution.rs",
        "src/clauses/factor.rs",
        "src/clauses/paramodulation.rs",
        "docs/rust-port-status.md",
    ]
    source_digest = hashlib.sha256(
        b"".join((repo / relative).read_bytes() for relative in source_files)
    ).hexdigest()
    report: dict[str, Any] = {
        "content_hashes_verified": sum(
            record["content_sha_matches"] is True for record in selected
        ),
        "decision_count": len(selected),
        "decision_digest": decision_digest,
        "decision_group_counts": {
            name: len(group) for name, group in sorted(DECISION_GROUPS.items())
        },
        "evidence_checks": evidence_checks,
        "exact_text_still_in_current_docs": sum(
            record["legacy_text_in_current_source"] for record in selected
        ),
        "schema_version": 1,
        "source_digest": source_digest,
        "source_file_count": len(source_files),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    selected_are_terms = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem") == "terms"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 29
        or report["content_hashes_verified"] != 29
        or not selected_are_terms
        or not all(evidence_checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("detailed TERMS reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
