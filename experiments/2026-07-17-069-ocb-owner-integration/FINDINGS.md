# OCB higher-order and proof-control ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.69`. The higher-order variable map and
the complete production ordering-owner bridge are implemented and exercised.
The vendored C checkout remained unchanged.

## Question

Does Rust preserve C's Lambda-order variable-map identity and balance behavior,
and are there proof-state-owned ordering-generation callers outside the current
proof-control bridge that still need porting?

## Method

[`compare_ocb.py`](compare_ocb.py) audits C and Rust higher-order map updates,
the map-only reset boundary, KBO6 reset integration, all production
proof-control ordering-selection call sites, and two permanent Rust
regressions. It then runs a FOL KBO6 proof through the ordinary C reference and
a THF Lambda-order KBO6 proof through the matching `ENABLE_LFHO` C reference.

## Findings

All eight source and regression contracts pass:

- C keys `ho_vb` by fluid-term pointer identity; Rust keys its safe map by the
  stable identity of the shared term owner. Both increment/decrement paths make
  the same zero-to-positive, negative-to-zero, zero-to-negative, and
  positive-to-zero counter transitions and update `wb` by `var_weight`.
- `OCBResetHOVarMap` frees only the C map. Rust likewise clears only `ho_vb`;
  KBO6's enclosing Lambda-order reset separately zeroes `wb`, `pos_bal`,
  `neg_bal`, and `max_var`.
- A focused OCB regression proves aliases share one map entry, distinct term
  owners with the same variable code retain distinct entries, signed balances
  are exact, and the direct reset leaves aggregate counters untouched.
- C has exactly one production `TOSelectOrdering` call site:
  `ProofControlInit` stores the result before parsing weight functions and
  heuristics. Rust's clause-only and clause/formula initialization variants do
  the same with explicit mutable term-bank/axiom owners.
- A focused proof-control regression selects a higher-order KBO6 Lambda-order
  OCB, confirms the live signature-size snapshot and one-slot Lambda variable
  vector, and retains configured lambda/DB weights of 30/12.

Both executable cases are byte-exact against the appropriate isolated C
reference:

| Case | C reference | Exact |
| --- | --- | :---: |
| FOL KBO6 proof | ordinary | yes |
| THF KBO6 Lambda-order proof | `ENABLE_LFHO` | yes |

Exact exit codes, byte counts, hashes, and future mismatch payloads are retained
in [`results-summary.json`](results-summary.json). The broader executable
ordering surface remains 73/73 exact in
[`experiments/2026-07-17-053-term-ordering-option-matrix/FINDINGS.md`](../2026-07-17-053-term-ordering-option-matrix/FINDINGS.md).

## Ownership decision

C stores a borrowed `Sig_p` and raw `ProofState_p`-reachable owners in the
selection stack. Rust stores the resulting OCB in `ProofControl`, snapshots the
signature size exactly as C does, and passes the live signature or mutable term
bank explicitly to operations that need it. This avoids a raw pointer from OCB
to movable Rust owners without losing the C construction or evaluation order.

There are no additional C production ordering-selection callers to port. The
separate lower-level ordering generation, automatic selection policy, and
ordering-algorithm compatibility Beads remain independent scope, not missing
proof-state ownership.

## Validation

- reproducible source audit: all eight contracts passed;
- focused OCB identity/reset and proof-control higher-order owner regressions:
  passed;
- executable C/Rust matrix: 2/2 byte-exact against matching FOL/HO references;
- strict all-target/all-feature tests, Clippy, release build, and formatting:
  passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, rerun/diff check, and vendored-tree check:
  passed.
