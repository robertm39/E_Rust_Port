# Detailed ORDERINGS reconciliation

## Status

Accepted for the six remaining open `orderings` records under Beads
`E_Rust_Port-j76.4`. Direct C/Rust review found no missing production ordering
behavior. Four records are intentional compatibility contracts, one is a safe
replacement for an unsafe debug-only C edge, and one is an upstream
declaration/definition inconsistency for which Rust already has the useful
internal operation. No Rust or C source changed.

## Review decisions

| Record | Decision |
|---|---|
| 908 | Follow `KBOVarGreater`'s implementation, not its reversed subset comment. Both C and Rust add `+1` for the proposed greater term and `-1` for the lesser term, then accept only when every balance is nonnegative. The classic-KBO variable-distribution regression pins greater, lesser, and incomparable cases. |
| 910 | Retain ordinary `i64` addition in the recursive classic-KBO hot path. C uses unchecked signed `long` accumulation and defines no overflow result; changing Rust to checked, saturating, or explicit wrapping arithmetic would invent a new ordering at an unreachable resource-scale boundary and add hot-path policy without compatibility evidence. Current exact ordering matrices and whole-port performance cover the production domain. |
| 911 | Preserve first-order production KBO6's initialized `Equal` result for distinct equal-weight heads under an unordered/partial precedence. The disabled C slow checker disagrees, but the shipped production path and Rust regression are unambiguous. Higher-order LFHO retains its separate `Uncomparable` result. |
| 913 | Preserve dispatch by the process problem type and `HoOrderKind`, regardless of whether the particular terms visibly contain higher-order syntax. Rust mirrors C's first-order/LFHO/Lambda branches; no-bank guards remain limited to calls that cannot perform required normalization, while production proof control supplies the live bank. |
| 919 | Retain the intended debug-LPO result without C's out-of-bounds tail indexing. Rust checks both arities before indexing and traverses each remaining tail over that side's actual range. Unequal-arity same-head regressions cover both directions, and core debug LPO results agree with maintained LPO. |
| 922 | Treat `OCBSetMinConst` as an upstream source inconsistency. C declares but never defines the symbol and has no link-level behavior to reproduce. Rust's explicit internal setter operates on the same per-type slot and is covered alongside conditional set and minimum-constant search. |

## Evidence

The low-level ordering regressions directly cover all six decisions:

- classic KBO variable balances, term weights, precedence, and
  lexicographic/arity paths;
- first-order KBO6's partial-precedence result, distinct LFHO result, global
  higher-order dispatch, Lambda bank normalization, and scratch-state policy;
- safe debug-LPO core, variable, and unequal-arity comparisons; and
- OCB per-type conditional/forced minimum-constant updates and typed
  minimum-constant discovery.

The retained production evidence is broader:

- the 73-case ordering option matrix is byte-exact across the six executable
  orderings and matching FOL/`ENABLE_LFHO` C builds;
- all 18 higher-order forward-modification ordering configurations are exact;
- the accepted borrowed KBO6 balance walker reduced its complete hot boundary
  by 38.44% and preserved exact proofs; and
- the latest exact candidate passes 4,429 tests, all 50 main-prover cases, and
  all 216 support-tool cases with zero unexpected differences.

The maintained aggregate is 1.0801753448x C, so no overflow or dispatch policy
change is justified by performance. The ordering evidence is retained in
[`experiment 316`](../2026-07-25-015-borrowed-kbo-balance/FINDINGS.md),
[`experiment 336`](../2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md),
and the latest comprehensive
[`validation reference`](../2026-07-25-046-external-reconciliation/validation-reference.json).

## Audit

[`audit_orderings_reconciliation.py`](audit_orderings_reconciliation.py) pins
the exact six migrated identities and content hashes, checks the six
source/implementation decisions plus current full-port evidence, and digests
the eight unchanged C units, four Rust ordering owners, the status ledger, and
the retained performance/validation records. The audit is independent of
issue status, so it remains reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage,
Change Later wording, local links, manual-regeneration preservation, and
`git diff --check` pass. The unchanged implementation is covered by the exact
Experiment 046 lifecycle:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,418 library plus 11 integration tests pass, 4,429 total;
- native release and compile-only Windows GNU x64 all-target/all-feature builds
  pass; and
- 50 main plus 216 support-tool comparisons have zero unexpected differences.

No Rust or C toolchain ran on the local Windows host. The vendored C checkout
is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-047-orderings-reconciliation/audit_orderings_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-047-orderings-reconciliation/audit-reference.json
```
