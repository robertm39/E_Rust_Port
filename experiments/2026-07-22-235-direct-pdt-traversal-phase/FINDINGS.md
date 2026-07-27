# Experiment 235: Reject direct PD-tree traversal phases

## Question

Can each PD-tree cursor frame encode its current symbols, variables, or done
phase directly, avoiding the hot-loop traversal-order lookup without the extra
monomorphizations that regressed Experiment 234?

## Baseline

- Source: commit `7afc7626`, whose executable source remains accepted
  Experiment 231.
- Exact LUSK6 Callgrind: 9,923,564,772 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.888634.

## Candidate

- Replace the frame's one-byte numeric step index with a one-byte private phase
  enum that directly represents symbols-first, variables-first, each second
  phase, and completion.
- Compute the initial phase from the recorded traversal order once per cursor
  call and copy it into new frames.
- Advance phases directly, retaining the shared first-order/higher-order cursor
  monomorphizations from Experiment 231.
- Keep `PdtTraversalFrame` at 40 bytes on 64-bit targets.

## Validation

- All 42 focused PD-tree tests pass, including a new test of both phase
  transition sequences and the existing traversal-order, live-substitution,
  higher-order, constraint, and backtracking coverage.
- Strict all-feature library pedantic Clippy and formatting pass.
- The deterministic LUSK6 run proves Unsatisfiable with the expected 4,873
  processed clauses and exits zero.

## Measurement

Exact Callgrind instructions regress from 9,923,564,772 to 9,932,459,603: an
increase of 8,894,831 or 0.089633%. The implied Rust/C ratio worsens from
1.888634 to 1.890327.

The first-order cursor plus visible callees rises from 1,709,361,574 to
1,721,133,871 instructions, an increase of 11,772,297 or 0.688696%. The
exclusive cursor body rises by 9,155,484 instructions and cursor initialization
rises by 2,616,714; unchanged call counts show the phase representation itself
costs more than the numeric step plus recorded-order lookup. Work outside this
aggregate improves by 2,877,466 instructions but does not offset the local
regression.

The raw candidate profile is
`.artifacts/experiments/2026-07-22-235-direct-pdt-traversal-phase/rust-callgrind-direct-pdt-traversal-phase.out`.
The retained parent profile is
`.artifacts/experiments/2026-07-22-231-specialize-pdt-cursor/rust-callgrind-specialize-pdt-cursor.out`.

## Decision

Reject. Directly encoded phases preserve semantics and frame size but regress
both the cursor aggregate and the exact whole-program profile. Native timing
and compatibility matrices are skipped after deterministic rejection. Source
is restored byte-for-byte to the Experiment 231 accepted baseline.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-pdt-traversal-phase.out \
  target-wsl-235-direct-pdt-traversal-phase/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
