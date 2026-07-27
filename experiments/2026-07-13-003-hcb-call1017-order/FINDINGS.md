# GEO288 HCB Order And Contextual Subsumption

## Question

Why does the first post-CNF `GEO288+1.p` HCB identifier mismatch occur at
selection call 1017, and does that mismatch explain why C proves the problem in
about four seconds while Rust reaches the 60-second limit?

## Setup

- C reference:
  `/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover`
- Rust executable: `target/release/eprover.exe`
- Problem: `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`
- Shared arguments:
  `--auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new`
- C debugger: GDB in `Ubuntu-24.04` under WSL.
- Temporary Rust trace probes were removed before final validation.

Representative commands, run from the repository root:

```powershell
wsl -d Ubuntu-24.04 -- gdb -q -batch -x experiments/2026-07-13-003-hcb-call1017-order/capture-c-hcb-call1017.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
wsl -d Ubuntu-24.04 -- gdb -q -batch -x experiments/2026-07-13-003-hcb-call1017-order/trace-c-hcb-structures.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
python experiments/2026-07-13-003-hcb-call1017-order/compare-hcb-structures.py .artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-hcb-structures.txt .artifacts/experiments/2026-07-13-003-hcb-call1017-order/rust-hcb-structures.txt
python experiments/2026-07-13-003-hcb-call1017-order/compare-fvi-queries.py .artifacts/experiments/2026-07-13-003-hcb-call1017-order/c-fvi-queries.txt .artifacts/experiments/2026-07-13-003-hcb-call1017-order/rust-fvi-queries.txt
cargo build --release --locked --features instrument-perf-ctr
target/release/eprover.exe --auto --output-level=0 --print-statistics --cpu-limit=60 --memory-limit=2048 --processed-clauses-limit=5000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p
```

## Results

1. The first identifier mismatch is an allocation-order permutation, not a
   clause-structure or heuristic-evaluation mismatch. At calls 1017 and 1025,
   C and Rust select the same normalized clauses with the same evaluations,
   dates, weights, proof depths, proof sizes, and parent provenance, but the
   identifiers assigned to those two clauses are swapped.
2. The generated simultaneous-paramodulation children are allocated in this
   order:

   - C: `+3079` at C position 2, `+3080` at position 0,
     `+3081` at position 6.
   - Rust: `+3079` at C position 6, `+3080` at position 0,
     `+3081` at position 2.

   The compared subterms have the same term-bank entry numbers. C orders
   `SubtermOcc` records by raw term pointers, while Rust orders its
   shared handles by allocation identity. Reproducing the exact C order would
   require allocator-layout emulation rather than a semantic port.
3. `compare-hcb-structures.py` finds no structural mismatch in the
   first 3,000 common HCB selections. The C trace was extended to 10,000 calls
   and stopped at the script's explicit ceiling, so it is not a theorem trace.
4. C and Rust build the same FV-index permutation:
   `[19,24,29,21,34,4,16,11,15,33,28,0,3,32,35,20,1]`.
   The first 45 aligned indexed queries also have identical packed vectors and
   candidate matcher counts; the old traces then diverge at Rust query 46.
   The GDB result-register capture was not reliable, so the current trace
   script omits it and the comparator accepts but ignores that field in the
   preserved raw traces.
5. A 3,000-processed-clause bound exposed the actual performance defect.
   Principal proof counters matched, but C made `1,459,564` non-unit
   subsumption calls while Rust made `11,195,271`. Rust's forward
   contextual simplify-reflect helper scanned `ClauseSet::iter()`;
   the C `ClauseContextualSimplifyReflect` call reaches
   `ClauseSetSubsumesClause`, which automatically uses
   `set->fvindex`. Backward contextual candidate discovery had the
   same accidental linear bypass.
6. Routing both contextual directions through the set-owned FV anchor reduces
   the 3,000-bound Rust count to `1,458,982`, versus C's
   `1,459,564`. Recursive calls are `39,370` versus C's
   `39,395`, and the principal generated, processed, retained,
   contextual-SR, paramodulation, and rewrite counters match.
7. Contextual deletion can leave a unit query for the next loop iteration.
   C's indexed branch accepts it, but the plain fallback asserts a non-unit
   candidate. The Rust indexed wrapper now preserves that branch-dependent
   contract; focused tests cover direct indexed unit lookup and an indexed
   contextual reduction followed by the unit query.
8. At a 5,000-clause bound, the final preserved C run records
   `1,992,304` non-unit subsumption calls and Rust records
   `2,019,719`; an earlier C run recorded `1,992,683`. Main
   generation counters match. The final C run retains 23,279 nonredundant
   clauses, an earlier run retained 23,280, and Rust retains 23,278. This small
   C run-to-run shift is consistent with ASLR changing its pointer-keyed
   first-hit order.
9. Reversing Rust FV-index leaf traversal was rejected. It only approximated
   C's splay-tree root order, changed retained-clause counts, and did not
   materially improve the search.
10. The instrumented 5,000-bound Rust run takes about 13.6 seconds.
    `ParamodTimer` accounts for 9.29 seconds,
    `FVIndexTimer` for 1.45 seconds, and
    `SetSubsumeTimer` for 1.68 seconds. Full Rust GEO288 still reaches
    the 60-second limit; C proves it after 10,215 processed clauses. Indexed
    paramodulation is therefore the next measured performance target.

11. The final 50-case native differential report has five mismatches, down from
    six because `BOO020-1.p` now matches. GEO288, `HEN011-2.p`, and the
    synthetic CPU-limit `LUSK6.lop` case remain outcome mismatches;
    `LUSK6ext.lop` and `sledgehammer.p` differ only in normalized stdout. The
    five-run benchmark measures a `3.086` aggregate Rust/C median wall-time
    ratio over the nine behavior-matching cases. `LUSK6.lop` measures `3.037`
    and `LUSK6ext.lop` measures `2.803`. Repeated `BOO020-1.p` runs are
    excluded because their timeout-bound outcomes differ, despite matching in
    the one-shot differential report.

## Raw Artifacts

Generated output is under the ignored directory
`.artifacts/experiments/2026-07-13-003-hcb-call1017-order/`.

Key files:

- `c-hcb-call1017.txt`, `c-hcb-call1017-clause.txt`, and
  `c-hcb-call1025-clause.txt`
- `c-clause-allocation-backtraces.txt` and
  `c-clause-allocation-bodies.txt`
- `c-paramod-positions.txt`, `rust-paramod-positions.txt`,
  and `rust-pm-allocations.txt`
- `c-hcb-structures.txt` and `rust-hcb-structures.txt`
- `c-fvi-queries.txt` and `rust-fvi-queries.txt`
- `rust-perf-5000.txt`
- `c-release-5000.txt` and `rust-release-5000.txt`
- `c-release-full.txt` and `rust-release-full.txt`
- Differential report `.artifacts/e-compare/20260713-191159-630133/`
- Benchmark report
  `.artifacts/e-compare/20260713-192710-976425-benchmark/`

The checked-in GDB scripts are tied to the cached optimized C binary's
instruction offsets. Rebuilds may require relocating those breakpoints.

## Falsification Checks

- Clause text, evaluation cells, dates, weights, proof metadata, and derivation
  parents were compared at both sides of the first identifier swap.
- Allocation tracing isolated all three sibling clauses and their C-position
  provenance, ruling out a missing inference.
- Structural HCB comparison ignores identifiers and therefore tests whether
  the permutation changes the selected clause sequence itself.
- The identical FV layout and 45 aligned query vectors/matcher counts rule out
  an immediate feature-permutation defect. The query sequence then diverges,
  so this comparison makes no claim beyond that prefix.
- The reverse-leaf trial tested whether newest-first traversal could stand in
  for C pointer-splay ordering; its changed retained population falsified that
  surrogate.
- The bounded before/after counter comparison isolates the large call reduction
  to contextual FV-index routing. Full-problem timeout behavior confirms that
  this fix alone does not establish GEO288 completion parity.
- The final 50-case comparison checks that the fix removes one established
  outcome mismatch without introducing a new one. The repeated benchmark keeps
  the timeout-sensitive differing case out of the aggregate ratio.

## Conclusion And Limits

The call-1017 mismatch comes from allocator-address ordering of simultaneous
paramodulation positions. The first 3,000 normalized HCB clause structures
remain aligned, so emulating the C allocator is not justified by this trace.

The material defect was an accidental Rust linear scan in forward and backward
contextual simplify-reflect. That defect is fixed and covered by indexed
unit-lifecycle tests. C's nullable index ownership, branch-dependent unit
precondition, and pointer-keyed leaf order are documented as change-later
issues.

This experiment does not establish full GEO288 parity, exact semantic
first-hit ordering after the 3,000-call prefix, or overall performance parity.
The final differential and benchmark reports leave those limits explicit. The
next investigation should profile and port the remaining indexed paramodulation
optimizations.
