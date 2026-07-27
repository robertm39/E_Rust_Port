# LUSK6ext selection trace

## Question

Where does the C/Rust `LUSK6ext.lop` search first diverge, and is the differing
proof ancestry caused by clause selection, inference generation, or proof
extraction order?

## Setup and commands

The reference was E 3.3.5 at commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`
from the `e-interop` WSL cache. The Rust candidate was the current Windows
release build on `codex/initial-rust-port-slice`.

Representative commands, run from the repository root:

```powershell
cargo build --release
.\target\release\eprover.exe --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6ext.lop
cargo test unit_set_subsumption_uses_opposite_indexed_equality_side_like_c
cargo test indexed_forward_rewrite_preserves_c_shared_variable_match
```

```bash
/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover --auto --output-level=6 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop
gdb -q -batch -x experiments/2026-07-12-002-lusk6ext-selection/trace-c-rewrite-subst.gdb /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
```

Small normalized traces and debugger scripts are retained in this experiment
folder. Full generated output is under ignored `.artifacts/e-compare/`, notably
`lusk6ext-c-gdb-run.stdout`, `lusk6ext-c-full-new-given.txt`, and
`lusk6ext-rust-fixed-full-new-given.txt`.

## Results

- A 12-selected-clause OutputLevel-6 cutoff produced identical selected clauses,
  generation counts, rewrite counts, and unprocessed-set size in C and Rust.
  This falsifies the apparent early selection divergence inferred from proof-list
  node order.
- Before the fix, the first selected-clause divergence in a 100-processed-clause
  trace was ordinal 34. C discarded the Rust-selected equality as
  `subsumed(571)`.
- C `UnitClauseSetSubsumesClause` reaches `FindSimplifyingUnit` through the
  demodulator index. Both sides of an unorientable equality are indexed. Rust's
  former linear scan considered only the stored literal and combined badly with
  C's intentionally asymmetric swapped retry in `eqn_topsubsumes_termpair`.
- Routing both Rust unit-set subsumption entry points through the indexed helpers
  makes all 45 selections in the 100-clause trace match. The resulting Rust proof
  has 149 steps, down from 157; C has 147.
- The next full-trace selection divergence is ordinal 64. C records
  `rw(680,2574)` before selecting the contracted clause; Rust later selects the
  uncontracted clause.
- GDB confirmed clause 2574 is an unorientable equality and that C's PDTree
  search passes a live substitution directly into `instance_is_rule`. The first
  superficially relevant shared-variable match is rejected by KBO6. A later
  accepted match has query `j(g(y),j(w,y))`, substitution
  `x -> g(y), y -> w, z -> y`, and KBO6 result `to_greater`.
- The fresh 50-case differential report at
  `.artifacts/e-compare/20260712-052620-363855/` has the same three normalized
  output mismatches as the prior six-case baseline. It reports eight total
  mismatches because marginal `BOO020-1.p` and `GEO288+1.p` resource cases also
  failed in this run; the other established mismatches are unchanged.
- The five-run native WSL benchmark at
  `.artifacts/e-compare/20260712-054243-733742-benchmark/` measures a `3.608x`
  aggregate Rust/C ratio. LUSK6ext improves from the prior `6.484x` to `3.260x`;
  the required `1.10x` threshold is still not met.

## Falsification checks

- Proof-list ancestry order was not used as a proxy for selection after the
  12-clause cutoff showed identical operational counters.
- The opposite-side unit subsumption regression first proves the historical
  linear predicate fails, then proves indexed lookup finds the C candidate.
- The provisional cyclic-renaming rewrite hypothesis was rejected: C did not
  consider the target contraction right-to-left, and its first left-side match
  returned KBO6 `to_lesser`, not `to_greater`.
- Rust's compact PDTree does return the relevant indexed side. A focused test of
  C's observed accepted shared-variable query also rewrites to the expected term,
  so no broad relaxation of `SubstIsRenaming` or KBO6 was made.
- The nested `eprover/` checkout was never modified; all C probes were debugger
  scripts or generated output outside that tree.

## Conclusion and limits

The confirmed defect was using a linear unit-set scan where C uses its
demodulator index. Indexed side expansion is semantically observable because of
the preserved asymmetric top-pair retry, not merely a performance optimization.

The fix removes the first LUSK6ext search divergence and shortens the proof, but
does not make the complete proof identical. The remaining ordinal-64 divergence
involves persistent shared-term rewrite links and contraction timing. This
experiment narrowed that issue but did not localize a production-code defect;
the direct candidate and accepted shared-variable rewrite paths already pass.
The proof contraction substantially improves the LUSK6ext benchmark, but neither
the aggregate performance requirement nor the full compatibility suite is yet
satisfied.
