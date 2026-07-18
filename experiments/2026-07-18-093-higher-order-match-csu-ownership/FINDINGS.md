# Higher-order matching, CSU, and term-bank ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.44`. The migrated residual claims are
stale: every production CSU call site in the pinned C source has a Rust
consumer, and production higher-order complete matching/unification receives
the mutable term bank needed for eta/weak-head normalization. C's
`binding_cache` remains a separately tracked post-compatibility optimization,
not a missing unification semantic. The vendored C checkout remained
unchanged.

## Ownership decision

C stores `owner_bank` on each LFHO `TermCell`. `SubstMatchComplete` recovers the
bank independently from both terms for eta reduction, while the higher-order
MGU and CSU paths recover it for weak-head normalization and applied-variable
expansion. C also stores the expansion in `binding_cache` and invalidates that
cache when the head binding changes.

Rust deliberately passes the live `&mut TermBank` through proof control,
clause inference, complete match/MGU, and CSU iteration. That preserves the
observable normalization, term sharing, binding construction, and diagnostic
rollback behavior without embedding a raw back-pointer in every shared term.
All production inference terms in these paths belong to the proof-state bank.
The no-cache expansion can repeat work, but it does not change the term seen by
the inference. Cache lifetime/GC and repeated-dereference performance remain
open under `E_Rust_Port-j76.3.643` and `E_Rust_Port-j76.4.1313`.

The retained unbanked Rust APIs are confined to low-level compatibility
helpers that have bank-aware production twins: 11 complete-match invocations
across equation, subsumption, and unit-simplification helpers, plus the
explicit first-order definition-unfolding branch; and two complete-MGU
invocations in equation helpers. Higher-order production callers use the
bank-aware variants.

## Complete CSU call-site audit

[`audit_unification_surface.py`](audit_unification_surface.py) scans source
rather than relying on migrated prose. Outside the CSU implementation itself,
the pinned C tree contains exactly four `CSUIterInit` constructions:

| Consumer | C | Rust |
| --- | ---: | ---: |
| equality resolution | 1 | 1 |
| equality factoring | 1 | 1 |
| indexed paramodulation, both directions | 2 | 2 |

Each Rust construction advances the iterator with the mutable proof-state bank
and restores the substitution through explicit destruction/error paths. There
is therefore no remaining C proof-control CSU consumer to integrate. Direct,
unindexed higher-order paramodulation remains a single complete-MGU path
because the C source also calls `SubstMguComplete` there rather than the CSU
iterator.

The retained static report passes 14/14 checks and has SHA-256
`d5208301e10f70dc40d210b817a3f106e863f407baab6f61b3f43a38f525b66e`.

## Executable comparison

[`compare_unification_surface.py`](compare_unification_surface.py) compares
the Windows Rust release executable with the isolated `--enable-ho` C build
from upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
The C executable SHA-256 is
`317e261b4915d16834de9f5a133ecd07fe6e21dfdc8c5f06072ed75b3e56b7e1`;
the compared Rust executable SHA-256 is
`7d0e88db682874e8ac8d07423bd88ecac8fa301ab7538d55ae819f8e8f4609db`.

All 21 focused unification projections match exactly:

- the new applied-variable rewrite binds `F` in `F @ a` to `h`, produces
  `h @ b = c`, records one rewrite step, and has empty stderr in both builds;
- branching equality resolution produces the same two resolvents in the same
  projection/imitation order and reports two equation resolutions;
- branching equality factoring produces the same two factors in the same
  order and reports two factorizations; and
- the three direct single-MGU fixtures (rigid-prefix, flex-flex, and raw
  eta/lambda-DB) match inference traces, exit status, and focused counters
  under all six optimized C ordering choices, for 18 configurations.

The retained comparison report has SHA-256
`d2e2a2f8964d6879fffde07fa25e601b06558adeb5d2da51b59b098377cfdcd1`.

## Orthogonal terminal-status finding

After producing the identical rewritten clause and counters in the new
axioms-only match fixture, C exhausts the unprocessed set as `GaveUp`/exit 10,
whereas Rust reports `Satisfiable`/exit 1. The comparison therefore projects
that case onto its matching outputs and does not claim terminal-status parity.
The newly exposed incompatibility was tracked as bug
`E_Rust_Port-j76.2.140` and is now resolved by the exact terminal-status matrix
in [`experiments/2026-07-18-097-exhausted-higher-order-status`](../2026-07-18-097-exhausted-higher-order-status/FINDINGS.md); the branching CSU and direct-MGU cases continue to compare their exit codes exactly.

## Validation

- static ownership/dispatch audit: 14/14 checks;
- focused executable comparison: 21/21 unification projections;
- retained-reference reruns for both experiment scripts;
- full all-target/all-feature test suite and strict pedantic Clippy;
- release `eprover` build and all C-source documentation integrity gates; and
- clean nested `eprover/` worktree.
