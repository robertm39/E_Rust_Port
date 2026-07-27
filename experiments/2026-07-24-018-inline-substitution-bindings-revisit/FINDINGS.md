# Experiment 291: Revisit inline substitution bindings

## Question

Does retaining four substitution bindings inline now improve production
performance after Experiment 290 independently established and retained the
`term_deref_always` inlining boundary that dominated the earlier rejection?

## Baseline

- Accepted source: Experiment 290, commit `66fee19f`.
- Exact default-feature LUSK6 Callgrind: 8,800,386,737 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.674873.
- `Substitution::add_binding` owns exactly 269,213 `RawVec` growth calls and
  33,382,544 inclusive instructions in the accepted profile.
- `Substitution::norm_term` owns another 102,237 `RawVec` growth calls.

## Candidate

Retain four `Option<Term>` slots directly in `Substitution`, matching the
four-term capacity obtained by the original vector's first growth. On a fifth
live binding, move the initialized prefix into a retained overflow vector and
keep using that vector until the substitution is fully backtracked. The next
independent binding sequence returns to inline storage while retaining overflow
capacity for a later wide sequence.

Iteration joins the initialized inline prefix with overflow storage, preserving
binding order. Stack positions remain the total live length, and every binding
producer and backtracking path uses the shared push/pop state machine.

## Reason for revisiting Experiment 246

Experiment 246 removed 130,703 allocation calls but regressed exact
instructions by 1.358281%. Its dominant cause was compiler layout:
`term_deref_always` became a standalone 276,328,019-instruction function and
the comparable normalization aggregate rose by 156,125,312 instructions.

Experiment 290 now retains a separately justified `#[inline(always)]` boundary
for that sole-caller wrapper as part of the accepted allocation-free clause
partition. The accepted profile has no standalone `term_deref_always` symbol.
This changes the decisive condition behind the earlier rejection and makes one
bounded remeasurement appropriate. The exact-size allocator introduced since
Experiment 246 also makes the net result uncertain rather than assumed.

## Validation

- All 10 substitution tests pass. The new regression covers six live bindings,
  ordered spill, partial backtracking, rebinding while overflow remains active,
  full backtracking, retained capacity, and subsequent inline reuse.
- The candidate fingerprint records exactly `features=["default"]`.
- The candidate reaches the exact 4,873-processed-clause LUSK6 proof and exits
  zero under Callgrind.
- The accepted forced `term_deref_always` wrapper remains inlined: neither
  candidate nor parent profile contains a standalone wrapper symbol.
- Native timing and compatibility gates were skipped after the decisive
  deterministic instruction regression.

## Measurement

The candidate retires 8,809,027,245 instructions, 8,640,508 above the
8,800,386,737-instruction parent. This is a 0.098183% whole-prover regression,
and the hypothetical Rust/C ratio rises from 1.674873 to 1.676517.

The candidate reduces the binding stack's `RawVec` growth calls from 269,213 to
42,574, eliminating 226,639 growths. The savings do not repay the extra
per-operation branching, larger substitution representation, and drop work.

The directly attributable normalization/binding/backtrack/drop aggregate rises
from 512,744,182 to 539,890,818 instructions, an increase of 27,146,636:

- `Substitution::norm_term`: 443,318,643 to 454,840,178.
- `Substitution::add_binding` versus `push_binding`: 27,895,630 to 40,530,143.
- `Substitution::backtrack_single`: 31,315,479 to 31,045,315.
- `Substitution` drop glue: 10,214,430 to 13,475,182.

Unlike Experiment 246, this regression is not caused by outlining
`term_deref_always`; the Experiment 290 inlining boundary remains effective.
The remaining local representation cost is independently decisive.

## Result

Reject. The changed compiler boundary justified one remeasurement, but inline
substitution bindings still regress the exact whole-prover workload. Restore
the Experiment 290 production source byte-for-byte and retain this profile as
the final boundary for the four-slot representation.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-inline-bindings-candidate.out \
  target-wsl-291-inline-substitution-bindings-revisit/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
