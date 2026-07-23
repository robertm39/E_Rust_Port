# Experiment 234: Reject specialized PD-tree traversal order

## Question

Does dispatching the recorded symbols-first versus variables-first order once per
PD-tree cursor call improve production proof-search performance compared with
decoding that invariant inside every cursor state-machine iteration?

## Baseline

- Source: commit `c66c48dd` (Experiment 231 accepted source; Experiments 232–233
  are evidence-only).
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.

## Candidate

- Capture the recorded traversal order at the two public cursor entry points.
- Dispatch the shared cursor through four const-generic variants covering
  first-order versus higher-order mode and symbols-first versus variables-first
  order.
- Replace the hot-loop load of `state.traversal_order` with a comparison against
  the compile-time order parameter.
- Retain a debug assertion that the selected variant matches the active search
  state.

## Validation

- All 41 focused PD-tree tests pass, including both recorded traversal orders,
  live substitutions, repeated variables, higher-order queries, constraints,
  and backtracking.
- Strict all-feature library pedantic Clippy passes.
- Formatting passes after applying rustfmt.
- The deterministic LUSK6 run proves Unsatisfiable with the expected 4,873
  processed clauses and exits zero.

## Measurement

Exact Callgrind instructions regress from 9,923,564,772 to 10,076,432,779:
an increase of 152,868,007 or 1.540455%. The implied Rust/C ratio worsens from
1.888634 to 1.917727.

The active LUSK path uses the `first_order = true, symbols_first = false`
variant. Its exclusive cursor body is 1,362,463,856 instructions, but the
cursor plus visible callees totals 1,732,384,611 versus 1,709,361,574 in the
parent, a regression of 23,023,037 or 1.346879%. In particular, symbol-query
advance becomes a 226,260,463-instruction out-of-line edge; variable advance,
cursor start, substitution binding, and backtracking retain the same call
counts. Another 129,844,970 regressed instructions occur outside that cursor
aggregate, consistent with the broader optimizer/code-layout shift from
doubling the existing problem-mode monomorphizations.

The raw candidate profile is
`.artifacts/experiments/2026-07-22-234-specialize-pdt-traversal-order/rust-callgrind-specialize-pdt-traversal-order.out`.
The retained parent profile is
`.artifacts/experiments/2026-07-22-231-specialize-pdt-cursor/rust-callgrind-specialize-pdt-cursor.out`.

## Decision

Reject. The candidate preserves semantics but loses decisively in the exact
instruction profile. Native timing and compatibility matrices are skipped
after deterministic rejection. Source is restored byte-for-byte to the
Experiment 231 accepted baseline.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-specialize-pdt-traversal-order.out \
  target-wsl-234-specialize-pdt-traversal-order/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
