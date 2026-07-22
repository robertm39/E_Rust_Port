# Rejected allocation-free proof-set lookup

## Question

Can proof-parent lookup scan fixed `ProofState` clause-set fields without
allocating a temporary 11- or 12-entry `Vec<&ClauseSet>` on every call,
matching C's direct field traversal?

## Setup

- Parent source: commit `618623ed` (`Record rejected borrowed term type
  comparison`), whose executable source remains accepted Experiment 214.
- Motivation: accepted Callgrind caller attribution records 379,349 calls to
  `ProofState::proof_clause_by_derivation_ref`, each charged one Rust heap
  allocation for `proof_clause_sets`. The complete parent has 6,312,342 Rust
  allocations.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Generic-iterator profile:
  `.artifacts/experiments/2026-07-22-220-allocation-free-proof-set-lookup/rust-callgrind-allocation-free-proof-set-lookup.out`.
- Manual-scan profile:
  `.artifacts/experiments/2026-07-22-220-allocation-free-proof-set-lookup/rust-callgrind-manual-proof-set-lookup.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidates

The first candidate returns fixed arrays as chained iterators, appends the
optional watchlist without allocation, and makes the exact-reference plus
legacy-ID helper generic over a cloneable iterator. It also applies the same
private shape to the related live, quote-source, and formula set lists.

The second candidate minimizes the code-generation change after the first
failure. It restores unrelated live/formula helpers, returns fixed 11-entry
arrays only for proof and quote-source lookup, passes the optional watchlist
separately, and uses one monomorphic manual scan. Both candidates preserve set
order, make the watchlist last, scan every exact reference before the legacy
sourceless-ID fallback, and retain archive precedence.

## Local result

Both candidates remove exactly 379,349 Rust allocations, reducing the total
from 6,312,342 to 5,932,993 (-6.009640%). The generic lookup body falls from
31,129,937 to 25,819,075 exclusive instructions (-17.060304%). The simpler
manual body falls to 10,641,356 (-65.816327%). Thus the intended allocation
and lookup work is genuinely removed.

## Whole-program result

Both candidates reach the expected LUSK6 proof but fail the deterministic
gate:

- the generic iterator retires 10,723,919,126 instructions, 91,277,741 or
  0.858467% above the 10,632,641,385 parent;
- the manual scan retires 10,727,007,899 instructions, 94,366,514 or 0.887517%
  above the parent;
- their hypothetical C/Rust ratios worsen from 2.023584 to 2.040956 and
  2.041544.

The failure is a whole-binary inlining effect rather than a lookup failure.
Parent `Substitution::norm_term` includes the always-dereference path and costs
437,245,456 instructions. Both candidates make a standalone
`term_deref_always` symbol reappear at 276,328,019 instructions. The comparable
normalization aggregate rises by 142,057,666 instructions (+32.489226%) in the
generic form and by 142,867,070 (+32.674341%) in the manual form. The PD-tree
cursor also rises by 11,512,774 instructions in both. Generic TermTree
insertion falls by 14,960,621, while manual insertion is only 400,209 above
the parent; neither offsets the dereference regression.

## Validation

- All 62 proof-state tests pass for both candidates, including exact
  generation/source lookup, archive precedence, quote-source lookup, formula
  lookup, legacy fallback, and dead-parent behavior.
- Strict all-feature library pedantic Clippy and formatting pass for the first
  candidate; formatting and the same focused suite pass for the simplified
  form.
- Both release candidates reach the expected unsatisfiable result and exit
  zero under Callgrind.
- Source is restored byte-for-byte and all 62 proof-state tests plus formatting
  pass after rejection.
- Native and compatibility matrices were skipped after deterministic
  rejection.

## Decision

Reject both allocation-free proof-set lookup shapes and restore the temporary
vectors. The local hypothesis is valid but the source change destabilizes a
much larger dereference inlining decision. Test the implicated
`term_deref_always` wrapper independently before revisiting this lookup. Keep
the accepted baseline at 10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-manual-proof-set-lookup.out \
  target-wsl-220-allocation-free-proof-set-lookup/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
