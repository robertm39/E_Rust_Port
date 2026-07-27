# PD-tree cursor and transient proof-state ownership

## Question

Can the remaining proof-search instruction gap be reduced without reopening
the maintained 60-second/2-GiB resource failures, and can the intermittent
`BOO020-1.p` 139,264-byte allocation abort be removed without slowing the
proof-capable `HEN011-2.p` boundary?

## Setup

- Parent source: commit `e850e34a` (`Avoid transient clause-page growth`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Deterministic profile: upstream smoke-test `LUSK6.lop` under WSL Callgrind
  with `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Native proof corpus: `GEO288+1.p`, `HEN011-2.p`, `LUSK6.lop`, and
  `LUSK6ext.lop`, with proof objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at a 60-second CPU
  limit and 2-GiB data allowance.

Raw profiles and rejected-candidate boundary samples are retained under
`.artifacts/experiments/2026-07-19-145-pdt-cursor-path/`. The accepted direct
BOO repetitions are copied into this experiment directory.

## Throughput candidates

The parent profile retired 15,985,039,196 instructions. The accepted changes
were measured cumulatively:

| Candidate | Instructions | Change from preceding row |
| --- | ---: | ---: |
| Parent | 15,985,039,196 | - |
| Remove Rust-only PD-tree path prewalk | 15,252,115,665 | -4.59% |
| Feature-gate per-tree visited counter | 15,224,046,191 | -0.18% |
| Borrow term type UID | 15,146,313,486 | -0.51% |
| Single-pass prefix-token classification | 15,123,733,139 | -0.15% |
| Store cursor binding indices | 14,913,726,802 | -1.39% |
| Match release `NDEBUG` substitution checks | 14,884,183,123 | -0.20% |
| 1,024-header clause pages | 14,879,751,273 | -0.03% |
| Evaluate eval-store clauses in place | 14,568,419,539 | -2.09% |
| Final page-reclaim/headroom source | 14,571,495,734 | +0.02% |

The removed prewalk had been more conservative than C: C checks root
constraints before traversal and lets the cursor reject absent paths. The
live cursor now does the same. Speculative variable bindings retain
variable-child and query-cell indices instead of cloning two reference-counted
term handles per binding. Type-UID access borrows the term's type link, prefix
classification reads the function code once, and optimized substitution
binding/backtracking uses debug assertions like C under `NDEBUG`.

The per-tree `visited_count` mutation is now limited to the existing
`measure-expensive` feature. The independent `pdt-count-nodes` global counter
remains available. A candidate that cached the complete root constraint/path
snapshot retired 15,235,899,671 instructions and was rejected because it was
slower than feature-gating alone.

## Resource diagnosis

An optimized PDB-backed failure at a reduced diagnostic Job Object limit
resolved the 139,264-byte allocation through `SparseClauseStore::push_back`,
`ClauseSet::insert`, and `proof_state_queue_generated_clause_for_eval`. The
evaluation store had 3,072 live clauses and was requesting its fourth
1,024-header page while the drained prefix of `tmp_store` still owned pages.

Two independent ownership defects contributed:

1. `proof_state_eval_clause_set` extracted and reinserted every evaluated
   clause so safe evaluation roots stayed synchronized. That changed an
   in-place C batch into a growing queue. Rust now mutates the clauses in place
   and rebuilds its evaluation roots once after the batch.
2. A FIFO drain kept fully consumed leading pages until the entire sparse store
   became empty. The generated-clause tmp-store path now releases a completely
   drained page immediately by shifting only `Vec` page handles and adjusting
   identifier, derivation, and evaluation-object slots. It does not move live
   `Clause` values.

A broader candidate compacted every FIFO extraction. It made the resource
fixtures pass, but changed focused HEN from a proof to `ResourceOut`; it was
rejected. Restricting movement-based compaction to `tmp_store` restored HEN but
did not make BOO deterministic. Page-handle reclamation keeps the targeted
memory benefit without the evaluation-store transfer cost.

The Windows Job Object charges total private process commit, unlike C's
`RLIMIT_DATA`. The retained PSAPI wrapper now reports private-commit headroom
relative to the configured process ceiling. Before allocating a new sparse
page, ordinary non-Linux searches retain a one-second cooperative CPU
lookahead; only headroom within two page quanta expands it to two seconds.
Every requested lookahead is capped at 5% of the configured CPU window, so the
synthetic one-second workload receives almost its complete budget instead of
being expired at its first page allocation. Linux retains asynchronous
`SIGXCPU` and does not execute this allocation guard.

## Results

The final deterministic profile proves the exact workload after 4,873
processed clauses and retires 14,571,495,734 instructions. This is
1,413,543,462 instructions (8.84%) below the parent and only 0.02% above the
in-place-evaluation checkpoint. Against the archived C profile of
5,254,361,329 instructions, the remaining ratio is approximately 2.77.

The final focused resource report is
`.artifacts/e-compare/20260720-110321-183584/`: two cases, zero mismatches. The
final focused proof report is `.artifacts/e-compare/20260720-110750-346993/`:
four cases, zero mismatches, including HEN. The exact-source three-run BOO CSV
records normal `ResourceOut`/8 outcomes at the real CPU boundary.

The exact final production build's loaded 50-case report is
`.artifacts/e-compare/20260720-114727-932451/`. BOO and SWV both match C's
normal `ResourceOut` result, but three undeclared rows remain: HEN reaches
`ResourceOut` at 59.30 seconds, the synthetic one-second LUSK case reaches its
cutoff, and one LCL run selects a different valid refutation. The isolated LCL
rerun at `.artifacts/e-compare/20260720-120354-603854/` has zero mismatches.
The corrected isolated HEN rerun, including its TPTP axiom tree, is
`.artifacts/e-compare/20260720-120544-506645/`: both provers produce the same
normalized refutation, but Rust takes 52.43 seconds versus C's 18.78 seconds.
HEN is therefore closed functionally but remains load-sensitive at the
maintained cutoff.

The proportional cap corrects the one-second case's premature 0.09-second
expiry, but Rust still needs about 1.19 CPU seconds for the proof-object variant
and does not yet beat the one-second production cutoff. The main performance
parity Bead therefore remains open.

## Falsification checks

- Cursor tests pin branch order, repeated-variable consistency, accepted live
  substitution bindings, and both optional counter configurations.
- Sparse-owner tests pin in-place compaction, fixed page capacity, prefix-page
  reclamation, identifier/derivation positions, and evaluation-object handles.
- Evaluation tests pin unchanged eval-store membership, order, and slot span.
- Signal tests pin unlimited, short-window proportional, and expired-deadline
  behavior; OS-wrapper tests pin saturating memory headroom.
- Three direct BOO runs plus exact BOO/SWV comparison cover the intermittent
  allocation boundary.
- Exact GEO/HEN/LUSK proof comparison covers proof order and normalized output.
- The final loaded matrix plus isolated LCL/HEN reruns distinguish transient
  proof ordering and host-load sensitivity from stable output regressions.
- Callgrind retains the exact proof and processed-clause count.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-145-proportional
& .\experiments\2026-07-19-134-compact-clause-owners\measure_windows.ps1 `
  -Binary .\target\native-145-proportional\release\eprover.exe `
  -Problem .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\BOO020-1.p `
  -OutputCsv .\experiments\2026-07-19-145-pdt-cursor-path\proportional-boo.csv `
  -Label proportional -Runs 3 -CpuLimit 60
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-145-proportional\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-headroom.out \
  target-wsl-145-headroom/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

## Decision

Accept the cursor ownership reductions, release/debug parity changes, in-place
eval-store evaluation, 1,024-header pages, tmp-store page-handle reclamation,
non-Linux batch polling, and proportional memory-pressure deadline guard.
Reject global extraction compaction and root constraint snapshots. Keep the
main parity issue open for the approximately 2.77-times instruction gap and
the remaining one-second cutoff.
