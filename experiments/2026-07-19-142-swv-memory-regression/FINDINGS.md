# SWV proof-state memory regression

## Question

Which live Rust owner causes the remaining `SWV851-1.p` allocator abort at
the maintained 60-second CPU and 2 GiB memory limits, and can it be removed
without changing proof search, resource semantics, or the C-visible result?

## Setup

- Parent source: commit `4c7fe547` (`Document Windows resource-boundary
  diagnosis`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Resource arguments: `--auto --silent --cpu-limit=60 --memory-limit=2048
  --detsort-rw --detsort-new --proof-object=1`.
- Boundary fixtures: upstream `BOO020-1.p` and `SWV851-1.p`.
- Deterministic fixture: unchanged `LUSK6.lop` under WSL Callgrind with a
  600-second CPU limit.

Experiment 141 established that the final SWV failure was a live-set gap, not
a broken timeout: Rust approached the 2.25 GiB Windows Job Object allowance
immediately before its process-CPU deadline, while C returned normal
`ResourceOut`/8. Raising the allowance, reserving emergency memory, or moving
the deadline would only conceal that gap.

The CSV files in this directory retain every accepted and rejected Windows
boundary run. They record exit status, process CPU, sampled peak working set,
and the failed allocation size where applicable.

## Layout reductions

Several exact-size reductions survived repeated search checks:

- `Clause` now packs its 31 property bits and 33-bit positive-literal count
  into one `u64`. Negative count and total count are derived from the literal
  list. This removes two redundant `usize` fields and reduces `Clause` from
  160 to at most 136 bytes.
- `EvalCell` stores its fixed evaluation array as `Box<[SimpleEvalCell]>`
  rather than `Vec<SimpleEvalCell>`, reducing the owner to 32 bytes.
- `TermCell` stores its fixed-arity argument slots as a boxed slice, reducing
  the cell from 152 to 144 bytes without weakening the existing `RefCell`
  mutation boundary.
- A dead-clause sweep rebuilds occupied sparse pages after deletion. This was
  retained only after the allocation-free streaming variant repeatedly failed
  SWV; the rebuild releases fragmented page storage at this workload's actual
  deletion point.

The packed clause header keeps a debug assertion for the property-bit range
and masks in optimized builds. An unconditional release assertion changed
boundary code generation enough to make both resource cases unstable, while
all defined `FormulaProperties` already fit the checked 31-bit range.

## SAT ownership parity

C's `SatClauseCell` stores a borrowed `Clause_p source`; it does not clone a
complete source clause for every propositional clause. Rust previously cloned
the 136- to 160-byte `Clause` owner into every `SatClause` and kept all SAT
clauses in one reallocating vector.

`SatClauseSet<'a>` now borrows production source clauses for exactly the SAT
check's lifetime. Its tagged source owner retains an owned branch for
self-contained synthetic sets, but the prover import path no longer clones a
complete `Clause`. Unsat-core extraction clones only the selected source
clauses that escape the set. SAT clauses are kept in 4,096-element chunks, so
growth has a bounded allocation and existing elements never move between
chunks. The final representation stores clauses inline in each chunk; the temporary
`Vec<Vec<Box<SatClause>>>` version passed resource checks but failed the
strict `clippy::vec_box` gate and needlessly allocated every clause separately.

No unsafe code is used. The lifetime relationship makes the C pointer
ownership rule explicit and prevents a SAT source from outliving its proof
clause.

## Symbolized root cause

Borrowed SAT sources moved the remaining failure to a repeatable 511,716-byte
request. `borrowed-sat-sources-swv.csv` records one normal `ResourceOut` and
two failures of exactly that size. A full Windows backtrace contained hundreds
of repeated frames at module-relative address `0x39dd72`. The release PDB and
Visual Studio's `llvm-symbolizer` mapped the frame to
`sat_check_proof_state_until_time_limit`; disassembly showed the recursive
call returning immediately before an alignment-one allocation and copy.

The allocation was `dpll()`'s `assignment.to_vec()`. The SAT instance had
511,715 numbered atoms, so every recursive decision cloned a 511,716-byte
`Vec<Option<bool>>`. Hundreds of simultaneously live branch copies accounted
for the late-search gap and matched both the allocation size and stack shape.

The solver now mutates one assignment array and records newly assigned atom
indices in a rollback trail. Each recursive call restores both its propagated
unit assignments and its branch assignment to a checkpoint. Decision order,
unit-propagation order, and the shared decision budget are unchanged.

## Resource results

Before the rollback change, the borrowed-source candidate failed two of three
SWV runs at sampled peaks of 2,312,388 and 2,312,876 KiB. With the rollback
trail and the temporary boxed chunk store, all three SWV runs returned normal
`ResourceOut`; sampled peaks fell to 1,952,344--1,960,060 KiB. BOO also
returned `ResourceOut` in all three runs. These measurements are retained in
`dpll-trail-swv.csv` and `dpll-trail-boo.csv`.

The final inline chunk store also passed three of three runs for both fixtures:

| Fixture | Outcomes | Sampled peak range |
| --- | --- | ---: |
| SWV851 | 3/3 `ResourceOut`/8 | 2,088,428--2,146,960 KiB |
| BOO020 | 3/3 `ResourceOut`/8 | 2,182,492--2,258,640 KiB |

The inline candidate advances farther before the same CPU deadline, so its
time-bound sampled peak is not a direct retained-byte comparison with the
slower boxed candidate. The relevant stability result is that all six final
runs reached the normal resource path and no allocator abort recurred.

The exact final Rust/C resource comparison is
`.artifacts/e-compare/20260719-225334-748594/`: both cases have zero
mismatches. The final proof-producing LUSK6 comparison is likewise exact in
`.artifacts/e-compare/20260719-225752-200705/`.

The 50-case final-store report at
`.artifacts/e-compare/20260719-230721-145586/` has two unexpected differences:
the already-open one-second LUSK6 cutoff and an intermittent HEN011 cutoff.
That run was globally slow--the C HEN011 proof took 33.93 seconds rather than
the earlier approximately 19 seconds--and Rust reached `ResourceOut` instead
of completing its proof. On an otherwise isolated four-case proof corpus, the
same stable binary matches C exactly on GEO288, HEN011, LUSK6, and LUSK6ext in
`.artifacts/e-compare/20260719-233127-796691/`. HEN011 therefore remains a
performance-margin concern, not a deterministic search or output difference.

## Rejected variants

The retained CSV names describe the candidate under test. Important
falsifications include:

- evaluation boxing or clause-header packing alone still left 168-byte
  boundary failures;
- packing evaluation metadata into smaller integer fields caused repeatable
  524,288-byte BOO allocation failures;
- reducing sparse clause pages from 4,096 to 256 entries passed BOO but made
  SWV fail earlier on tiny requests;
- streaming dead-clause rebuilding passed BOO three times but failed all three
  SWV runs;
- splitting term rewrite metadata out of line and sharing argument/link
  mutation reduced `TermCell` further, but first caused recursive `RefCell`
  borrow panics and then exposed deterministic 176,160,768-byte SAT growth;
- boxing SAT sources moved the failure to a 41,943,040-byte vector growth;
  boxing each SAT clause moved it to an 8,388,608-byte pointer-vector growth;
- merely borrowing SAT sources removed those owners but exposed the recursive
  511,716-byte assignment clone described above.
- compiling the synthetic owned-source branch out of production reduced each
  SAT clause by one word, but the faster/smaller run advanced farther before
  the deadline and made BOO fail two of three times on 278,528- and
  557,056-byte requests. The rejected measurements are retained in
  `dpll-trail-inline-borrowed-boo.csv` and
  `dpll-trail-inline-borrowed-swv.csv`.

These failures show why no individual layout-size result was accepted without
repeated whole-prover resource runs.

## Deterministic performance result

The final profile contains 17,441,814,419 instructions and is retained at
`.artifacts/experiments/2026-07-19-142-swv-memory-regression/callgrind-current.out`.
It preserves the exact proof and all 4,873 processed clauses. This is
177,779,204 instructions, or 1.03%, above experiment 140's
17,264,035,215-instruction profile. It remains 12.35% below the retained
19,899,749,157-instruction older baseline.

The increase is accepted for this slice because it closes a repeatable
resource failure, restores C's borrowed SAT-source ownership shape, and
removes depth-multiplied assignment copies. The separate one-second LUSK6
throughput gap remains open and is not reclassified as a resource issue.

## Falsification checks

- A DPLL regression verifies that a satisfying recursive search restores both
  branch and unit-propagation assignments and leaves the rollback trail empty.
- A SAT-store regression inserts through the 4,096-element chunk boundary and
  verifies indexing and iteration on both sides.
- Clause, evaluation, and term layout tests pin the accepted size reductions.
- Focused SAT-interface and predicate-elimination tests pass with borrowed
  source lifetimes.
- The strict all-target/all-feature pedantic clippy gate passes.
- Repeated Windows boundary runs and exact C comparisons cover both BOO and
  SWV; the isolated four-case proof corpus remains exact.
- Callgrind pins the unchanged proof search and quantifies the 1.03%
  instruction tradeoff.
- The vendored C checkout remains unchanged.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --target-dir target\dpll-trail-inline
& .\experiments\2026-07-19-134-compact-clause-owners\measure_windows.ps1 `
  -Binary .\target\dpll-trail-inline\release\eprover.exe `
  -Problem .\eprover\EXAMPLE_PROBLEMS\TPTP\SWV851-1.p `
  -OutputCsv .\experiments\2026-07-19-142-swv-memory-regression\dpll-trail-inline-swv.csv `
  -Label dpll-trail-inline-swv -Runs 3 -CpuLimit 60
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\dpll-trail-inline\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept the compact fixed-size owners, borrowed and chunked SAT representation,
and rollback-trail DPLL search. Together they replace the SWV allocator abort
with exact C-compatible `ResourceOut` behavior under the unchanged resource
limits. Keep the main parity issue open for the synthetic one-second LUSK6
throughput mismatch and HEN011's intermittent full-matrix performance margin.
