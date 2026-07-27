# Detailed HEURISTICS reconciliation

## Status

Accepted for the 42 remaining open `heuristics` records under Beads
`E_Rust_Port-j76.4`. Direct review found no missing production heuristic
behavior. The records describe already exact surfaces, explicit Rust ownership
boundaries, safe deterministic replacements for C undefined state, or
low-level compatibility adapters outside the production banked lifecycle. No
Rust or C source changed.

## Review decisions

| Record | Decision |
|---|---|
| 761 | Keep recognizing C's complete generality-name table while rejecting every measure except terms and formulas. The same supported D-relations are exact end to end. |
| 762 | Keep initialized filter thresholds, non-fallthrough LambdaDef printing, and deterministic anonymous names. C's uninitialized value and accidental fallthrough/collision are not valid observable contracts. |
| 765 | Treat this owner deferral as completed: formula-aware GSinE and LambdaDef operate through stable live owner identities, while threshold selection remains all-or-nothing. |
| 770 | Preserve the exact PCL-plus-fixed-statistics text through explicit render functions, without reintroducing process-global output state. |
| 773 | Retain the immutable RDAG adapter for low-level/stateless use. Every production marking owner routes through the exact banked lifecycle. |
| 774 | Retain the immutable diversity adapter under the same audited banked-owner decision. |
| 775 | Keep FIFO state as a typed scalar. Heap-allocating one `double` and callback-freeing it is a C representation detail, not behavior. |
| 777 | Retain the immutable funweight adapter; production lazy-init, mark, score, offset, and reset ordering is banked and regression-pinned. |
| 778 | Keep stable generation-based HCB references instead of raw orphan-prone clause pointers. Selection order and discard behavior are exact. |
| 783 | Retain the immutable Levenshtein adapter under the completed banked-owner decision. |
| 784 | Keep LIFO state as a typed scalar while preserving decrement-before-return order. |
| 786 | Keep the complete literal-selection table and closely related wrappers: all 144 names, table order, and executions are exact. |
| 791 | Preserve the implemented smallest-orientable fallback to smallest-by-weight when no orientable literal exists; the C prose is stale. |
| 792 | Retain duplicate positive-selector wrappers because they are named entries in the exact public selection table. |
| 801 | Preserve `SelectNewComplex`'s implemented macro behavior and largest eligible fallback, not its stale type-literal comment. |
| 802 | Preserve the exact minimum-inference-position positive-selection asymmetry and standard function weight of two. |
| 805 | Preserve direct-call selected bits when a type-filter gate produces no selection; table-driven production callers begin from the expected clean state. |
| 808 | Preserve the diversification sequence and integer-derived weights with explicit reset hooks replacing hidden global-test coupling. |
| 809 | Keep generated `schedule.vars` as build-time static data. All 419 strategies, 1,618 arrays, and both class maps match the parser oracle. |
| 810 | Preserve C's auto-schedule handoff order, including the first preprocessing/search cells and default fallback. |
| 811 | Preserve the positional string-distance metric, largest-class tie behavior, and exact partial-match report even though the function name suggests edit distance. |
| 812 | Preserve sparse in-place strategy mutation when parsing placeholders; replacing the whole structure would diverge from C. |
| 813 | Keep Rust's typed no-op `norm_subst_free` convenience. C declares but never defines the symbol, so no fabricated ABI export is needed. |
| 814 | Retain the immutable orientweight adapter under the completed banked-owner decision. |
| 815 | Retain the immutable prefixweight adapter under the completed banked-owner decision. |
| 816 | Preserve `PreferHOSteps` returning normal priority. Its unused C calculation is not observable behavior. |
| 820 | Keep the explicit runtime-loaded PicoSAT wrapper with the tested internal solver fallback; eager static linkage is a deployment detail, not required E behavior. |
| 821 | Keep `ProofControl` fully initialized rather than reproducing partially initialized C cells. All fields later observed by production gates have explicit values. |
| 822 | Keep table-visible literal-selection paths explicitly banked; immutable helpers remain deliberate low-level adapters. |
| 825 | Keep `ForwardModifyClause`'s bank and higher-order capability explicit. The full higher-order rewrite/ordering surface is regression-pinned. |
| 829 | Retain the immutable refinedweight adapter under the completed banked-owner decision. |
| 831 | Retain the immutable structural-weight adapter under the completed banked-owner decision. |
| 832 | Retain the immutable relative-term-weight adapter under the completed banked-owner decision. |
| 833 | Retain the immutable TF-IDF adapter under the completed banked-owner decision. |
| 834 | Preserve matrix-backed precedence whenever a predefined precedence is present, including partial `PNoMethod` inputs. |
| 838 | Initialize the complete ordering parameter mask before varying the four search fields. The resulting 1,972-state sequence is exact and avoids C stack indeterminacy. |
| 839 | Keep mutable-bank ordering evaluation explicit for production owners and the immutable evaluator as a low-level adapter sharing the scoring body. |
| 840 | Keep `Optimize` as the intended wildcard ordering mask. The conflicting raw C branch is dormant and its CLI assignment is commented out. |
| 841 | Keep AutoCASC/AutoDev parameter cells fully initialized before applying visible C assignments; exact reference strategies do not expose stack residue. |
| 842 | Retain the immutable tree-distance adapter under the completed banked-owner decision. |
| 843 | Retain the immutable variable-weight adapter under the completed banked-owner decision. |
| 844 | Retain the generic immutable WFCB adapter for low-level/stateless use. The production proof-control lifecycle has no immutable scoring call. |

## Evidence

The retained regressions and owner studies cover every decision:

- axiom filters pin parser diagnostics, deterministic defaults and printing,
  stable clause/formula identities, and all nine executable owner cases;
- clause-feature text, FIFO/LIFO scalar order, the normalization no-op, and
  higher-order priority quirks pin their complete observable surfaces;
- a source-wide production audit plus per-family tests pin banked lazy-init,
  maximal marking, score, offset, cleanup, and reset order;
- HCB generation identities preserve orphan liveness and extraction order;
- all 144 literal selectors and their option bridge execute exactly;
- generated schedule tables, positional matching, tie-breaking, handoff, and
  sparse parser mutation are exact;
- proof-control initialization, PicoSAT lifecycle/fallback, explicit bank
  ownership, and higher-order forward modification are exercised; and
- ordering selection matches the unchanged C reference's complete 1,972-state
  search while deliberately initializing C's indeterminate fields.

The latest exact candidate passes 4,429 tests, all 50 main-prover cases, and
all 216 support-tool cases with zero unexpected differences.

## Audit

[`audit_heuristics_reconciliation.py`](audit_heuristics_reconciliation.py)
pins the exact 42 migrated identities and content hashes, checks nine grouped
source/implementation/evidence contracts, and digests the 48 unchanged C
units, 25 Rust owners/helpers, status ledger, thirteen retained findings, and
current validation reference. The audit is independent of issue status, so it
remains reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage, Change
Later wording, local links, manual-regeneration preservation, and
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
.\.venv\Scripts\python.exe experiments/2026-07-25-051-heuristics-reconciliation/audit_heuristics_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-051-heuristics-reconciliation/audit-reference.json
```
