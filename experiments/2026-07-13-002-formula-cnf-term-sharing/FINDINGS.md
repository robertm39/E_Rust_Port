# Formula CNF Term Sharing Offset

## Question

Why does Rust retain 39 more permanent non-variable terms than C after
clausifying `GEO288+1.p`, before clausal preprocessing and proof control?

## Setup

- C reference: `/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover`
- Rust executable: `target/release/eprover`
- Problem: `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`
- Shared arguments: `--auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new`

Phase traces compare term-bank insertion count, live non-variable nodes, and
cumulative GC recovery at formula-CNF entry, after simplification, after
definition introduction, around each wrapped formula conversion, and after the
final collection. C is observed under GDB in `Ubuntu-24.04`; temporary Rust
phase probes are removed before production validation.

## Results

1. C and Rust enter `FormulaSetCNF2` with the same `in_count=4564` and
   `live=4564`. Rust has 402 additional attempted top insertions, but this
   difference is already established before CNF and remains constant.
2. Both reach simplification with `in_count=5610` and `live=5610`. The first
   structural mismatch occurs in simplification GC: C recovers 1,048 nodes and
   retains 5,412, while Rust recovered 1,945 and retained 4,515.
3. C `TBGCCollect` marks every clause and formula set registered in the term
   bank. Rust's staged `FormulaSetSimplify` collector marked only the active
   formula set, so it prematurely removed 897 terms still reachable from the
   pre-CNF formula archive and clause set.
4. The premature collection changes the outer `FormulaSetCNF2` GC schedule.
   Rust first diverges in unique terms while converting formula 1 and finishes
   CNF at `in_count=47226`, 39 above C's `47187`.
5. `FormulaSetCNF2` now passes its clause set and formula archive as explicit
   roots to every simplification collection in both silent and documenting
   paths. Standalone simplification retains its existing self-only root scope.
6. After the fix, all 156 formula-entry records and the four named boundary
   records match C exactly for unique insertions, live nodes, and cumulative GC
   recovery. Both finish at `in_count=47187`, `live=17799`, and
   `recovered=29388`; the attempted-insertion delta remains a harmless constant
   402.
7. Exact CNF chronology moves the first HCB identifier permutation from call
   995 to call 1017 and reduces identifier mismatches in the first 1,900 calls
   from 135 to 47. The HCB evaluation schedule remains aligned.
8. GEO288 still reaches the 60-second Rust resource limit. The remaining
   boundary is after exact formula-CNF term chronology and is not explained by
   the former 39-term offset.
9. The final 50-case native comparison at
   `.artifacts/e-compare/20260713-094120-648101/` retains the established six
   mismatches: normalized output differs for `LUSK6ext.lop` and
   `sledgehammer.p`; behavior differs for `BOO020-1.p`, `GEO288+1.p`,
   `HEN011-2.p`, and the synthetic CPU-limit `LUSK6.lop` fixture.
10. The final five-run benchmark at
    `.artifacts/e-compare/20260713-095433-604515-benchmark/` measures a `3.356`
    aggregate Rust/C median wall-time ratio, compared with `3.440` in the
    preceding slice. `BOO020-1.p` is excluded because behavior differs; all
    nine matching cases remain above the required `1.10` threshold.

## Raw Artifacts

Generated traces are stored under the ignored directory
`.artifacts/experiments/2026-07-13-002-formula-cnf-term-sharing/`.

Key files are `c-cnf-phases.txt`, `rust-cnf-phases.txt`,
`rust-cnf-phases-with-roots.txt`, and `rust-hcb-after-gc-roots.txt`.
`compare-cnf-phases.py` verifies structural phase parity while separately
checking that the attempted-insertion delta stays constant.

## Falsification Checks

- Formula-CNF entry and pre-simplification unique/live counts agree, ruling out
  parser ownership and pre-CNF higher-order/FOOL phases as the source of the
  39 terms.
- Adding only the archive and clause roots changes simplification recovery from
  1,945 to C's 1,048 and makes every later structural counter agree. This
  isolates the cause to root coverage rather than sweep mechanics.
- The +402 attempted-insertion difference remains constant across all compared
  phases and formula entries, so it does not create unique shared terms or
  alter GC retention in this run.
- A focused regression keeps preexisting archive-formula and clause terms
  shared while still recovering unreachable CNF scratch terms.

## Conclusion And Limits

The 39-term formula-CNF sharing offset was caused by incomplete explicit GC
roots in Rust and is fixed. C's global GC registration is compatibility-visible
because it controls term entry chronology and later proof-search tie ordering.

This experiment does not explain the remaining HCB permutations after call
1017, the remaining six native comparison mismatches, the roughly 3.36x
benchmark gap, or the GEO288 timeout. The next proof-search investigation
starts from exact formula-CNF structural counters rather than from bank
ownership.
