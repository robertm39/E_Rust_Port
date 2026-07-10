# COL042 Rewrite-Cache Divergence

Date: 2026-07-09

## Hypothesis

The first post-checkpoint divergence on `COL042-8.p` came from a simplification or clause-selection mismatch after processed clause 119.

## Setup

- C reference: archived level-6 output in `.artifacts/experiments/2026-07-09-002-col042-rewrite-cache/c-level6-cap120.txt`.
- Rust candidate: release `eprover.exe` with deterministic rewrite/new-clause sorting and a 120 processed-clause limit.
- Comparison scripts: `compare_eval_sequence.py` and `compare_selected.py` in this directory.

## Commands

```powershell
target\release\eprover.exe --auto --output-level=6 --processed-clauses-limit=120 --cpu-limit=15 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\TPTP\COL042-8.p --print-statistics
python experiments\2026-07-09-002-col042-rewrite-cache\compare_eval_sequence.py <rust-output> <c-output>
target\release\eprover.exe --auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1 eprover\EXAMPLE_PROBLEMS\TPTP\COL042-8.p --print-statistics
```

## Results

- Binary checkpoints at 80, 100, 110, 115, 116, 117, 118, and 119 processed clauses had matching aggregate search statistics. The first visible mismatch was at 120.
- The initial 120-clause mismatch was 993 generated clauses and 524 rewrite steps in C versus 911 and 496 in Rust. C selected the expected `apply(apply(w1,X1),...)` clause while Rust selected an older clause that immediately rewrote to a tautology.
- HCB schedule position, evaluation-vector ordering, FIFO tie-breaking, backward-rewrite counts, indexed demodulator candidates, and `instance_is_rule` ordering checks all matched. These hypotheses were falsified.
- Comparing every `['eval']` clause found the first latent mismatch at evaluation 196. C performed a second rewrite through the old unorientable `w1` equation; Rust performed only the preceding oriented rewrite.
- C had already removed that `w1` equation from the active demodulator set. Its rewrite still fired through a persistent term rewrite link. C paramodulation resets a paired `freshvars` bank before substitution normalization, so generated clauses reuse canonical variable codes such as `X1`; alpha-equivalent terms therefore share term-bank cells and cached links.
- Rust's per-call normalization bank advanced past every existing variable before generation. Generated clauses used globally increasing variables such as `X307`, preventing the equivalent term from sharing C's cached rewrite link.
- Resetting the temporary paramodulation bank before normalization restored C's canonical variable reuse. The corrected capped run has a 951-clause common evaluation prefix out of 951 on both sides and matches the key aggregate statistics exactly: 124 processed, 48 trivial, 19 subsumed, 57 remaining, 993 generated, 524 rewrites, and 271 cached rewrites.
- The full corrected Rust run proves the problem with `SZS status Unsatisfiable` after 942 processed clauses, 38,734 generated clauses, and 24,245 rewrite steps. Before the fix it exhausted the 60-second limit.

## Conclusion

The divergence was caused by variable-normalization state, not demodulator lookup, KBO6, or HCB selection. In E, resetting paired fresh-variable counters is part of term-sharing and rewrite-cache semantics, not merely a naming convention.

## Limits

- The C executable could not be rerun in the current environment; the comparison used the previously captured C trace.
- C and Rust still differ in allocator/GC counters at the 120-clause checkpoint (`Termbank termtop insertions` and collected term cells). Semantic evaluation order and proof-search counters match.
- The Rust port still allocates temporary normalization banks per inference instead of owning one reusable shadow bank in the proof session, so performance remains behind C even though this proof now completes.
