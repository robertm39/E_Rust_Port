# Experiment 253: Cache the active PD-tree query type UID

## Status

Rejected in Experiment 253 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted LUSK6 profile assigns 1,581,288,798 exclusive instructions, or
15.98% of the whole prover, to the first-order PD-tree cursor. Each variable
alternative rereads the unchanged current query term's type UID through the
term-link `RefCell`, although several alternatives can be tested before the
lazy query-stack top changes. C reads the same shared-term field directly.

Cache only that type UID in the operation-local substitution cursor. Invalidate
it whenever symbol expansion, variable consumption, or backtracking changes
the lazy query-stack top. Preserve the 40-byte traversal frame, variable-child
order, binding logic, constraint checks, term-weight timing, and all
higher-order behavior.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851

## Candidate

Add a type UID plus validity bit to the operation-local `PdtSubstCursor`.
Repeated variable alternatives reuse the cached value while the query-stack
top is unchanged. Symbol expansion, variable consumption, backtracking that
restores a processed term, search start, and search reset invalidate it.

The candidate does not change the compact 40-byte `PdtTraversalFrame`,
variable-edge records, metadata timing for term weights, traversal order,
bindings, constraints, or higher-order dispatch. A focused lifecycle
regression checks invalidation when first-order symbol expansion exposes a
child of another type and when frame pop restores the parent.

## Validation

- All 42 focused PD-tree tests pass.
- Strict library pedantic Clippy passes.
- Formatting and `git diff --check` pass.
- The exact LUSK6 profile proves `Unsatisfiable` and exits zero.

## Measurement

The candidate retires 9,923,541,985 instructions, 25,107,219 above the
9,898,434,766-instruction parent. This is a 0.253648% whole-prover regression,
and the hypothetical Rust/C ratio worsens from 1.883851 to 1.888630.

The dominant first-order cursor rises from 1,581,288,798 to 1,597,459,439
exclusive instructions:

- delta: +16,170,641;
- cursor-local regression: +1.022624%;
- share of the whole-program regression: 64.406341%.

The remaining increase is optimized-code layout outside the isolated cursor.
The cache-validity branch, larger cursor state, and invalidation stores cost
more than repeated `term_type_uid` access. This independently confirms the
direction of Experiment 169's broader rejected variable-alternative scan
without reintroducing that experiment's inner loop or eager term-weight read.

## Result

Reject. Restore direct type-UID lookup on each variable alternative. Candidate
production source and its lifecycle test are removed, and accepted Experiment
245 remains the baseline at 9,898,434,766 instructions, or 1.883851 times C.
Native and compatibility/resource matrices are skipped after the deterministic
and intended-owner gates both regress.

The raw profile is preserved at:

```text
.artifacts/experiments/2026-07-23-015-cache-pdt-query-type-uid/rust-callgrind-cache-pdt-query-type-uid.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-cache-pdt-query-type-uid.out \
  target-wsl-253-cache-pdt-query-type-uid/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
