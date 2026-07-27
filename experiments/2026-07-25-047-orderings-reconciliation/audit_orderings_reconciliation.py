#!/usr/bin/env python3
"""Audit the final ORDERINGS Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [908, 910, 911, 913, 919, 922]


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

    c_ocb = source(repo, "eprover/ORDERINGS/cto_ocb.c")
    checks = {
        "classic_kbo_variable_condition_follows_the_implementation": contains(
            repo,
            "eprover/ORDERINGS/cto_kbo.c",
            "Return true if vars(s) multisetsubseteq vars(t), false otherwise.",
            "VarHashAddVarDistrib(hash, s, deref_s, 1);",
            "VarHashAddVarDistrib(hash, t, deref_t, -1);",
            "if(handle->val < 0)",
            "return false;",
        )
        and contains(
            repo,
            "src/orderings/cto_kbo.rs",
            "Return whether the variable condition permits `s > t`.",
            "hash.add_var_distrib(s, deref_s, 1);",
            "hash.add_var_distrib(t, deref_t, -1);",
            "all(|entry| entry.value() >= 0)",
            "fn variable_distribution_comparison_matches_c_cases()",
        ),
        "classic_kbo_retains_the_unchecked_hot_weight_path": contains(
            repo,
            "eprover/ORDERINGS/cto_kbo.c",
            "long weight;",
            "weight = getweight(ocb, t->f_code);",
            "weight += gettermweight(ocb, t->args[i], deref);",
        )
        and contains(
            repo,
            "src/orderings/cto_kbo.rs",
            "let mut weight = get_weight(ocb, term.f_code());",
            "weight += get_term_weight(ocb, &arg, deref);",
            "fn term_weight_uses_function_and_variable_weights()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/"
            "FINDINGS.md",
            "1.0801753448x Rust/C aggregate",
            "zero unexpected",
            "compatibility differences.",
        ),
        "first_order_kbo6_unordered_heads_keep_the_production_equal_result": contains(
            repo,
            "eprover/ORDERINGS/cto_kbolin.c",
            "CompareResult res = to_equal;",
            "CompareResult tmp = OCBFunCompare(ocb, s->f_code, t->f_code);",
            "if(tmp == to_greater)",
            "else if(tmp == to_lesser)",
            "return res;",
        )
        and contains(
            repo,
            "src/orderings/cto_kbolin.rs",
            "let mut res = CompareResult::Equal;",
            "fn kbo6_preserves_linear_unordered_head_equal_result()",
            "CompareResult::Equal",
            "fn kbo6_higher_order_lfho_uses_higher_order_unordered_head_result()",
            "CompareResult::Uncomparable",
        ),
        "higher_order_kbo6_dispatches_from_problem_type_and_order_kind": contains(
            repo,
            "eprover/ORDERINGS/cto_kbolin.c",
            "if(problemType == PROBLEM_HO)",
            "if(ocb->ho_order_kind == LFHO_ORDER)",
            "res = kbolincmp_ho(ocb, s, t, deref_s, deref_t);",
            "res = kbolincmp_lambda(ocb, s, t, deref_s, deref_t);",
            "res = kbolincmp(ocb, s, t, deref_s, deref_t);",
        )
        and contains(
            repo,
            "src/orderings/cto_kbolin.rs",
            "if problem_type() == ProblemType::HigherOrder {",
            "match ocb.ho_order_kind",
            "HoOrderKind::LfhoOrder =>",
            "HoOrderKind::LambdaOrder =>",
            "fn kbo6_higher_order_lambda_order_bank_api_normalizes_lambda_applications()",
        )
        and contains(
            repo,
            "experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/"
            "FINDINGS.md",
            "explicit-bank WHNF, fixpoint, complete match/MGU, KBO6, and LPO4",
            "18/18 higher-order forward-modification ordering configurations",
            "73/73 ordering option matrix",
        ),
        "debug_lpo_preserves_intended_results_without_out_of_bounds_indexing": contains(
            repo,
            "eprover/ORDERINGS/cto_lpo_debug.c",
            "for (i=0; i<MAX(s->arity,t->arity); i++)",
            "while ((j<MAX(s->arity, t->arity)) && (res_help == res))",
            "D_LPOCompare(ocb, s, t->args[j],",
            "D_LPOCompare(ocb, s->args[j], t,",
        )
        and contains(
            repo,
            "src/orderings/cto_lpo_debug.rs",
            "for index in 0..s.arity().max(t.arity())",
            "if t.arity() <= index {",
            "if s.arity() <= index {",
            "for index in start..t.arity()",
            "for index in start..s.arity()",
            "fn debug_lpo_equal_head_length_cases_follow_c_surface()",
        ),
        "missing_c_minimum_constant_setter_is_an_explicit_source_inconsistency": contains(
            repo,
            "eprover/ORDERINGS/cto_ocb.h",
            "void OCBSetMinConst(OCB_p ocb, Type_p type, FunCode cand);",
        )
        and "OCBSetMinConst(" not in c_ocb
        and contains(
            repo,
            "src/orderings/ocb.rs",
            "pub fn set_min_const(&mut self, type_: &Type, candidate: FunCode)",
            "self.min_constants.insert(type_.type_uid(), candidate);",
            "fn min_constant_helpers_use_type_uid_slots()",
            "ocb.set_min_const(&individual, first);",
        ),
        "full_ordering_and_port_compatibility_evidence_is_current": contains(
            repo,
            "docs/rust-port-status.md",
            "Classic first-order KBO from `cto_kbo`",
            "First-order linear KBO6 from `cto_kbolin`",
            "73/73 cases across matching FOL and `ENABLE_LFHO` C references",
            "all six executable orderings",
        )
        and contains(
            repo,
            "experiments/2026-07-25-015-borrowed-kbo-balance/FINDINGS.md",
            "all 21 KBO tests",
            "all 50 main cases have zero unexpected differences",
            "all 216 support-tool cases have zero unexpected differences",
        )
        and contains(
            repo,
            "experiments/2026-07-25-046-external-reconciliation/"
            "validation-reference.json",
            '"rust_test_count": 4429',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        ),
    }
    source_files = [
        "eprover/ORDERINGS/cto_kbo.c",
        "eprover/ORDERINGS/cto_kbo.h",
        "eprover/ORDERINGS/cto_kbolin.c",
        "eprover/ORDERINGS/cto_kbolin.h",
        "eprover/ORDERINGS/cto_lpo_debug.c",
        "eprover/ORDERINGS/cto_lpo_debug.h",
        "eprover/ORDERINGS/cto_ocb.c",
        "eprover/ORDERINGS/cto_ocb.h",
        "src/orderings/cto_kbo.rs",
        "src/orderings/cto_kbolin.rs",
        "src/orderings/cto_lpo_debug.rs",
        "src/orderings/ocb.rs",
        "docs/rust-port-status.md",
        "experiments/2026-07-25-015-borrowed-kbo-balance/FINDINGS.md",
        "experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/"
        "FINDINGS.md",
        "experiments/2026-07-25-046-external-reconciliation/"
        "validation-reference.json",
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
    selected_are_orderings = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem")
        == "orderings"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 6
        or report["content_hashes_verified"] != 6
        or not selected_are_orderings
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("ORDERINGS reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
