# Experiment 255: Direct substitution argument loop

## Status

Rejected in Experiment 255 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted LUSK6 profile assigns 437,245,456 exclusive instructions to
`Substitution::norm_term`, compared with 192,675,144 for C `SubstNormTerm`.
Experiment 237 attributes 9,062,386 instructions to slice iteration and
5,737,294 to iterator flattening inside the normalizer's reverse argument
push.

Replace only `arguments.iter().rev().flatten()` with an explicit reverse
slot loop and `if let Some(argument)`. Preserve the single argument borrow,
owned handle clone per stack entry, reusable stack, left-to-right variable
binding order, dereference behavior, marking, and fresh-variable allocation.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851

## Candidate

Replace `Flatten` with an explicit `if let Some(argument)` inside the reverse
slot loop. A scoped Clippy expectation records why the profiled hot path is
intentionally testing a form that `manual_flatten` would normally simplify.
No argument access, stack representation, term ownership, or normalization
semantics change.

## Validation

- All nine focused substitution tests pass, including exact binding order.
- Strict library pedantic Clippy passes.
- Formatting and `git diff --check` pass.
- The exact LUSK6 profile proves `Unsatisfiable` and exits zero.
- Direct native parent and candidate output is byte-exact; both contain the
  proof and SZS success markers and exit zero.
- All 128 measured native processes exit zero.

## Deterministic measurement

The candidate retires 9,892,541,019 instructions, 5,893,747 below the
9,898,434,766-instruction parent. This is a 0.059543% whole-prover improvement,
and the hypothetical Rust/C ratio changes from 1.883851 to 1.882729.

`Substitution::norm_term` falls from 437,245,456 to 430,992,706 exclusive
instructions:

- delta: -6,252,750;
- local improvement: -1.430%;
- the intended local boundary explains the complete whole-program reduction.

The Windows candidate binary grows 1,536 bytes, from 8,654,336 to 8,655,872
bytes.

## Native production measurement

After a byte-exact direct proof check and four alternating warmup pairs, one
independent block ran 64 alternating parent/candidate pairs with a fresh
process for each execution.

Across all 64 pairs, the candidate regresses mean paired wall time by
2.986747% and CPU time by 3.000147%. Median paired wall and CPU changes regress
3.186603% and 3.125000%; aggregate wall and CPU time regress 2.862337% and
2.867100%. The candidate wins only 10 wall pairs and 10 CPU pairs, with four
CPU ties.

The stable last 32 pairs remain decisively negative:

- mean paired wall time: +2.478903%;
- mean paired CPU time: +3.295789%;
- median paired wall time: +3.166829%;
- median paired CPU time: +3.125000%;
- aggregate wall time: +2.435574%;
- aggregate CPU time: +3.241335%;
- wins: 5 wall and 3 CPU, with 2 CPU ties.

The native regression is roughly fifty times the deterministic whole-program
gain and remains stable in the final half, so a second block is unnecessary.

## Result

Reject. Restore `rev().flatten()` and remove the candidate lint expectation.
Accepted Experiment 245 remains the baseline at 9,898,434,766 instructions,
or 1.883851 times C. Compatibility and resource matrices are skipped after
the decisive native production failure.

The measured native samples are in `native-lusk.csv`. Raw ignored artifacts
are preserved at:

```text
.artifacts/experiments/2026-07-23-017-direct-subst-argument-loop/rust-callgrind-direct-subst-argument-loop.out
.artifacts/experiments/2026-07-23-017-direct-subst-argument-loop/native-warmup.csv
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-subst-argument-loop.out \
  target-wsl-255-direct-subst-argument-loop/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
