# Direct LFHO paramodulation and legacy ordering release surface

## Question

Does Rust cover the remainder of C's direct, unindexed higher-order
paramodulation surface, including flex-flex and eta-lambda/DB overlaps, and does
it accept every term ordering that the optimized C executable accepts for an
explicit higher-order run?

## Reference audit

The reference was upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` from the native WSL `e-interop`
cache.

- `ComputeOverlap`, `ClauseOrderedSimParamod`, and
  `ClauseOrderedSuperSimParamod` call `SubstMguComplete`, so direct unindexed
  paramodulation intentionally uses one complete MGU rather than the indexed
  CSU iterator.
- The current `ENABLE_LFHO` body of `CheckHOUnificationConstraints` ignores all
  four arguments and returns `true`. Rust therefore must not invent a stricter
  trailing-argument or applied-variable-head constraint.
- Classic KBO and the standard/copy LPO entry points assert that the problem is
  not higher-order in the C source. The optimized reference binary compiles
  those assertions out. Explicit higher-order runs with `KBO`, `LPO`,
  `LPOCopy`, and `LPO4Copy` process the direct fixture normally and generate
  the same paramodulant as `KBO6`/`LPO4`.

Before the fix, Rust rejected that optimized-release surface: classic KBO and
legacy LPO panicked on visible higher-order cells, while paramodulation and
forward contraction admitted only KBO6/LPO4. The pre-fix probe is retained as
`ordering-boundary.sh`.

## Change

Rust now mirrors the optimized executable rather than imposing an
assertion-build-only restriction:

- classic KBO and legacy LPO/copy traverse explicit higher-order cells through
  their existing symbol/argument algorithms;
- direct paramodulation and `ForwardModifyClause` admit all six concrete C
  ordering choices: `KBO`, `KBO6`, `LPO`, `LPOCopy`, `LPO4`, and `LPO4Copy`;
- bank-backed KBO6/LPO4 normalization behavior remains unchanged;
- direct MGU regressions cover applied-variable rigid-prefix binding,
  different-arity flex-flex reorientation, and a raw eta-expanded lambda/DB
  argument. The eta regression also checks the unifier directly before clause
  construction, because normal term-bank insertion eta-reduces that argument.

## Correctness evidence

Run:

```bash
bash experiments/2026-07-15-003-lfho-paramod-direct-mgu/trace.sh
```

The script compares three fixtures under all six ordering modes, for 18 native
C/Rust configurations. Every configuration exits at the same two-clause
resource bound, has an exact normalized inference trace, and has exact
processed/generated/paramodulation counters. The eta fixture generates one
paramodulant under every ordering. The rigid-prefix and flex-flex fixtures also
generate one except for C's additional LPO4 self-overlaps: two for
`rigid-prefix` and three for `flex-flex`; Rust matches those exactly.

The Rust regression suite now covers all 18 ordering/overlap combinations (16
new tests plus the two existing rigid-prefix cases) and adds an end-to-end
executable test for the four formerly rejected legacy choices. The
targeted executable test finishes with one generated clause and one
paramodulation in each case, with no diagnostic or panic.

The final 50-case differential is retained at
`.artifacts/e-compare/20260715-195614-153354/`. It has the same six established
mismatches as the preceding slice and no new regression: resource/status
differences for `BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, and the synthetic
one-second `LUSK6.lop` case, plus normalized proof-output differences for
`LUSK6ext.lop` and `sledgehammer.p`.

## Performance evidence

The three-run native focused benchmark is retained at
`.artifacts/e-compare/20260715-194449-186395-benchmark/`. All three automatic
problem outcomes match. The aggregate Rust/C wall-time ratio is `3.461x`:
`3.356x` for `eta-wrapper`, `3.457x` for `flex-flex`, and `3.575x` for
`rigid-prefix`. These inputs take C only about 2.5-3.4 ms and Rust about
8.7-11.4 ms, so the measurement is dominated by process startup and does not
show a paramodulation-loop regression. It does still record the real
short-running executable cost; the port-wide performance target remains open.

The final five-run native suite is retained at
`.artifacts/e-compare/20260715-200938-546890-benchmark/`. It measures a `3.137x`
aggregate Rust/C median wall-time ratio over nine behavior-matching cases,
improved from `3.296x` in the preceding slice. `BOO020-1.p` is excluded for its
known outcome mismatch. LUSK6 measures `2.576x` and LUSK6ext `2.627x`; every
included case remains above the required `1.10x`, so project-wide performance
parity remains incomplete.

## Conclusion

The Bead's suspected unification-constraint gap is not present in the current C
source, and Rust's direct complete-MGU implementation covers the audited
rigid-prefix, flex-flex, and eta/DB cases. The actual compatibility defect was
the optimized C ordering surface. That surface is now accepted end to end with
exact focused C/Rust inference behavior. Broader higher-order work should be
driven by a concrete C-supported counterexample rather than by the now-removed
ordering preflight.
