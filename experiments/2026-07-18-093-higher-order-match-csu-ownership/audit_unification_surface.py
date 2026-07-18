#!/usr/bin/env python3
"""Audit the complete-match/MGU and CSU production call-site surface."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any


EXPECTED_C_CSU = {
    "eprover/CLAUSES/ccl_eqnresolution.c": 1,
    "eprover/CLAUSES/ccl_factor.c": 1,
    "eprover/CONTROL/cco_paramodulation.c": 2,
}
EXPECTED_RUST_CSU = {
    "src/clauses/eqnresolution.rs": 1,
    "src/clauses/factor.rs": 1,
    "src/clauses/paramodulation.rs": 2,
}
EXPECTED_UNBANKED_MATCH = {
    "src/clauses/eqn.rs": 2,
    "src/clauses/subsumption.rs": 6,
    "src/clauses/unfold_defs.rs": 1,
    "src/clauses/unit_simplify.rs": 2,
}
EXPECTED_UNBANKED_MGU = {"src/clauses/eqn.rs": 2}
POST_COMPATIBILITY_CACHE_BEADS = {
    "E_Rust_Port-j76.3.643",
    "E_Rust_Port-j76.4.1313",
}


def relative(repo: Path, path: Path) -> str:
    return path.relative_to(repo).as_posix()


def count_calls(
    repo: Path,
    roots: list[Path],
    suffix: str,
    call: str,
    excluded: set[str] | None = None,
) -> dict[str, int]:
    result: Counter[str] = Counter()
    excluded = excluded or set()
    pattern = re.compile(rf"\b{re.escape(call)}\s*\(")
    for root in roots:
        for path in root.rglob(f"*{suffix}"):
            rel = relative(repo, path)
            if rel in excluded:
                continue
            count = len(pattern.findall(path.read_text(encoding="utf-8")))
            if count:
                result[rel] = count
    return dict(sorted(result.items()))


def read_beads(repo: Path) -> dict[str, dict[str, Any]]:
    issues: dict[str, dict[str, Any]] = {}
    for line in (repo / ".beads/issues.jsonl").read_text(encoding="utf-8").splitlines():
        record = json.loads(line)
        if record.get("_type") == "issue":
            issues[record["id"]] = record
    return issues


def digest(value: Any) -> str:
    rendered = json.dumps(value, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(rendered.encode("utf-8")).hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    args = parser.parse_args()
    repo = args.repo.resolve()

    c_csu = count_calls(
        repo,
        [repo / "eprover/CLAUSES", repo / "eprover/CONTROL"],
        ".c",
        "CSUIterInit",
    )
    rust_csu = count_calls(
        repo,
        [repo / "src/clauses"],
        ".rs",
        "CsuIterator::new",
    )
    unbanked_match = count_calls(
        repo,
        [repo / "src/clauses"],
        ".rs",
        "subst_match_complete",
    )
    unbanked_mgu = count_calls(
        repo,
        [repo / "src/clauses"],
        ".rs",
        "subst_mgu_complete",
    )

    checks = {
        "c_csu_call_sites_exact": c_csu == EXPECTED_C_CSU,
        "rust_csu_call_sites_exact": rust_csu == EXPECTED_RUST_CSU,
        "unbanked_match_confined": unbanked_match == EXPECTED_UNBANKED_MATCH,
        "unbanked_mgu_confined": unbanked_mgu == EXPECTED_UNBANKED_MGU,
    }

    for path, expected_count in EXPECTED_RUST_CSU.items():
        source = (repo / path).read_text(encoding="utf-8")
        checks[f"{path}:mutable_bank_iteration"] = (
            source.count(".next_csu_element(bank, &mut subst)") == expected_count
        )

    banked_twins = {
        "src/clauses/eqn.rs": (
            "subsume_term_pair_directed_with_bank",
            "unify_term_pair_directed_with_bank",
        ),
        "src/clauses/subsumption.rs": (
            "eqn_topsubsumes_termpair_with_bank",
            "literal_matches_directed_with_subst_with_bank",
        ),
        "src/clauses/unit_simplify.rs": (
            "unit_literal_side_matches_top_pair_with_bank",
        ),
    }
    for path, names in banked_twins.items():
        source = (repo / path).read_text(encoding="utf-8")
        for name in names:
            checks[f"{path}:{name}"] = f"fn {name}(" in source

    unfold_source = (repo / "src/clauses/unfold_defs.rs").read_text(encoding="utf-8")
    checks["unfold_defs_unbanked_is_fo_only"] = (
        "fn term_top_unfold_def_fo(" in unfold_source
        and "let matched = subst_match_complete(lside, term, &mut subst);" in unfold_source
        and "fn term_top_unfold_def_ho(" in unfold_source
    )

    issues = read_beads(repo)
    cache_tracking = {
        issue_id: issues.get(issue_id, {}).get("status")
        for issue_id in sorted(POST_COMPATIBILITY_CACHE_BEADS)
    }
    checks["owner_cache_work_remains_separately_tracked"] = all(
        status == "open" for status in cache_tracking.values()
    )

    report = {
        "checks": checks,
        "c_csu_call_sites": c_csu,
        "rust_csu_call_sites": rust_csu,
        "unbanked_match_call_sites": unbanked_match,
        "unbanked_mgu_call_sites": unbanked_mgu,
        "owner_cache_beads": cache_tracking,
    }
    report["sha256"] = digest(report)

    if args.output:
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if args.expected:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("static audit differs from the retained reference")
            return 1
    failed = [name for name, passed in checks.items() if not passed]
    if failed:
        print("failed checks:")
        for name in failed:
            print(f"- {name}")
        return 1
    print(f"validated {len(checks)} higher-order ownership/dispatch checks")
    print(f"report sha256: {report['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
