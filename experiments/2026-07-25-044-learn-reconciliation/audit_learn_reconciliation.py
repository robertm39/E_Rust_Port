#!/usr/bin/env python3
"""Audit the final LEARN Change Later decisions."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import sys
from pathlib import Path


ORDINALS = [890, 895, 897, 904, 905]


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


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = (repo / relative).read_text(encoding="utf-8")
    return all(needle in source for needle in needles)


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

    checks = {
        "flatten_zero_result_is_documented_and_tested": contains(
            repo,
            "eprover/LEARN/cle_annoterms.c",
            "long AnnoSetFlatten(AnnoSet_p set, PStack_p set_idents)",
            "long          count = 0, annos_found;",
            "return count;",
        )
        and contains(
            repo,
            "src/learn/annoterms.rs",
            "the implementation never increments",
            "let count = 0_i64;",
            "fn anno_set_flatten_merges_selected_sources_and_preserves_c_zero_return()",
        ),
        "duplicate_partial_insert_is_preserved_and_tested": contains(
            repo,
            "eprover/LEARN/cle_examplerep.c",
            "res1 = NumTreeStore(&(set->ident_index), rep->ident, tmp, tmp);",
            "res = StrTreeStore(&(set->name_index), rep->name, tmp, tmp);",
            "set->count = MAX(set->count, rep->ident);",
        )
        and contains(
            repo,
            "src/learn/examplerep.rs",
            "if !self.ident_index.store(ident, rep, ())",
            "if !self.name_index.store(&name, ident, ())",
            "fn example_set_insert_preserves_c_duplicate_name_side_effect()",
        )
        and contains(
            repo,
            "src/learn/kbinsert.rs",
            "let _inserted = set.insert(rep);",
            "fn kb_axioms_insert_preserves_c_duplicate_name_side_effect()",
        ),
        "user_facing_kb_tools_reject_duplicate_names_first": contains(
            repo,
            "src/prover/ekb_insert.rs",
            "if example_set_find_name(&proof_examples, &ex_name).is_some()",
            "fn duplicate_example_name_is_rejected_before_file_copy()",
        )
        and contains(
            repo,
            "src/prover/ekb_ginsert.rs",
            "if example_set_find_name(&proof_examples, &ex_name).is_some()",
            "fn duplicate_name_is_rejected_before_generation()",
        ),
        "source_count_cast_and_input_domain_are_explicit": contains(
            repo,
            "eprover/LEARN/cle_flatannoterms.c",
            "AnnotationCount(old->annotation));",
            "long sources = 0;",
            "sources+=term->sources;",
        )
        and contains(
            repo,
            "src/learn/flatannoterms.rs",
            "c_double_to_long(annotation.count())",
            "fn c_double_to_long(value: f64) -> i64",
            "value as i64",
        )
        and contains(
            repo,
            "src/learn/annotations.rs",
            "let value = parse_float(scanner)?;",
            "annotation.assign_value(count, value);",
        ),
        "parser_and_destination_term_ownership_is_explicit_and_tested": contains(
            repo,
            "eprover/LEARN/cle_kbinsert.c",
            "terms = TBAlloc(SigAlloc(sort_table));",
            "terms = TBAlloc(res_sig);",
            "ParseExampleClause(in, terms, examples->terms, ident);",
        )
        and contains(
            repo,
            "src/learn/kbinsert.rs",
            "accepts the second",
            "parser and destination term banks explicitly",
            "let mut axiom_terms = TermBank::new(Signature::new(TypeBank::new()))?;",
            "fn kb_parse_example_file_reads_axioms_separator_and_example_clauses()",
            "fn kb_parse_example_file_merges_duplicate_pattern_terms_like_anno_set_add_term()",
            "fn parse_example_clause_skips_pattern_search_over_branch_limit()",
        )
        and contains(
            repo,
            "src/prover/ekb_insert.rs",
            "let mut parse_terms = TermBank::new(internal_terms.signature().clone())?;",
            "&mut internal_terms,",
        )
        and contains(
            repo,
            "src/prover/ekb_ginsert.rs",
            "let mut parse_terms = TermBank::new(internal_terms.signature().clone())?;",
            "&mut internal_terms,",
        ),
        "latest_full_validation_covers_learn_paths": contains(
            repo,
            "experiments/2026-07-25-041-detailed-terms-reconciliation/validation-reference.json",
            '"rust_test_count": 4427',
            '"main_unexpected_difference_count": 0',
            '"tool_unexpected_difference_count": 0',
        )
        and contains(
            repo,
            "docs/rust-port-status.md",
            "duplicate-name rejection before generation",
            "generated-file reparsing through `KBParseExampleFile`",
            "duplicate-name rejection before copy",
            "`KBParseExampleFile` integration",
        ),
    }
    source_files = [
        "eprover/LEARN/cle_annoterms.c",
        "eprover/LEARN/cle_examplerep.c",
        "eprover/LEARN/cle_flatannoterms.c",
        "eprover/LEARN/cle_kbinsert.c",
        "src/learn/annotations.rs",
        "src/learn/annoterms.rs",
        "src/learn/examplerep.rs",
        "src/learn/flatannoterms.rs",
        "src/learn/kbinsert.rs",
        "src/prover/ekb_insert.rs",
        "src/prover/ekb_ginsert.rs",
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
    selected_are_learn = all(
        issues_by_id[record["id"]].get("metadata", {}).get("subsystem") == "learn"
        for record in selected
    )
    if (
        selected_ids != expected_ids
        or len(selected) != 5
        or report["content_hashes_verified"] != 5
        or not selected_are_learn
        or not all(checks.values())
    ):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("LEARN reconciliation reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
