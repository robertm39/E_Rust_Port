#!/usr/bin/env python3
"""Audit the retained explicit-bank/no-per-term-cache LFHO design."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


def text(repo: Path, relative: str) -> str:
    return (repo / relative).read_text(encoding="utf-8")


def contains(repo: Path, relative: str, *needles: str) -> bool:
    source = text(repo, relative)
    return all(needle in source for needle in needles)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    termtypes = text(repo, "src/terms/termtypes.rs")
    link_match = re.search(
        r"struct TermLinkData \{(?P<body>.*?)\n\}", termtypes, re.DOTALL
    )
    if link_match is None:
        raise RuntimeError("TermLinkData definition was not found")
    link_body = link_match.group("body")
    link_fields = re.findall(r"^\s*(\w+):", link_body, re.MULTILINE)

    term_cell_bytes = 136
    projected_inline_cache_bytes = term_cell_bytes + 3 * 8
    aggregate_ratio = 1.0801753448
    checks = {
        "compact_links_have_only_semantic_fields": link_fields
        == ["binding", "rw_replace", "type_"],
        "term_cell_layout_is_136_bytes": contains(
            repo,
            "src/terms/termtypes.rs",
            "assert_eq!(std::mem::size_of::<super::TermCell>(), 136);",
        ),
        "whnf_deref_takes_explicit_bank": contains(
            repo,
            "src/terms/lambda.rs",
            "pub fn whnf_deref(bank: &mut TermBank, term: &Term)",
        ),
        "fixpoint_unification_takes_explicit_bank": contains(
            repo,
            "src/terms/fixpoint_unif.rs",
            "pub fn subst_compute_fixpoint_mgu(",
            "bank: &mut TermBank",
            "whnf_deref(bank, term)?",
        ),
        "complete_match_has_banked_entry": contains(
            repo,
            "src/terms/match_mgu.rs",
            "pub fn subst_match_complete_with_bank(",
            "bank: &mut TermBank",
        ),
        "complete_mgu_has_banked_entry": contains(
            repo,
            "src/terms/match_mgu.rs",
            "pub fn subst_mgu_complete_with_bank(",
            "bank: &mut TermBank",
        ),
        "kbo6_has_banked_entry": contains(
            repo,
            "src/orderings/cto_kbolin.rs",
            "pub fn kbo6_compare_with_bank(",
            "bank: &mut TermBank",
        ),
        "lpo4_has_banked_entry": contains(
            repo,
            "src/orderings/cto_lpo.rs",
            "pub fn lpo4_compare_with_bank(",
            "bank: &mut TermBank",
        ),
        "rewrite_normalizes_zero_suffix": contains(
            repo,
            "src/terms/replace.rs",
            "if remaining_orig == 0 {",
            "return beta_normalize_db(bank, new);",
        ),
        "varhash_preserves_no_cache_deref_shape": contains(
            repo,
            "src/terms/varhash.rs",
            "let current = term_deref(&candidate, &mut current_deref);",
        ),
        "gc_has_no_nonexistent_cache_root": (
            "binding_cache" not in termtypes
            and "owner_bank" not in termtypes
            and "cache_binding" not in termtypes
        ),
        "unification_matrix_is_21_of_21": contains(
            repo,
            "experiments/2026-07-18-093-higher-order-match-csu-ownership/FINDINGS.md",
            "All 21 focused unification projections match exactly:",
        ),
        "ordering_matrix_is_18_of_18": contains(
            repo,
            "experiments/2026-07-17-087-forward-modify-ho-surface/FINDINGS.md",
            "All 18/18 configurations",
        ),
        "ordering_option_matrix_is_73_of_73": contains(
            repo,
            "experiments/2026-07-17-053-term-ordering-option-matrix/FINDINGS.md",
            "73/73",
            "Proof control owns the OCB",
        ),
        "fresh_performance_aggregate_is_below_threshold": contains(
            repo,
            "experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md",
            "`1.0801753448x` C",
            "zero unexpected differences",
        ),
    }
    report = {
        "aggregate_rust_c_ratio": aggregate_ratio,
        "check_count": len(checks),
        "checks": checks,
        "passed": sum(checks.values()),
        "projected_inline_cache_bytes": projected_inline_cache_bytes,
        "projected_inline_cache_growth_percent": round(
            100 * (projected_inline_cache_bytes - term_cell_bytes) / term_cell_bytes,
            6,
        ),
        "schema_version": 1,
        "term_cell_bytes": term_cell_bytes,
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    sys.stdout.write(encoded)

    if not all(checks.values()):
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("LFHO owner/cache audit reference changed", file=sys.stderr)
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
