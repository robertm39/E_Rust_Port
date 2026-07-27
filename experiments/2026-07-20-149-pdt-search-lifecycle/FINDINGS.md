# PD-tree search initialization lifecycle

## Question

Can the remaining PD-tree search-initialization overhead be reduced without
changing prefix-query cells, subtree spans, traversal order, or the live
substitution cursor?

## Setup

- Parent source: commit `75c535ca` (`Avoid cloning first-order MGU queue
  terms`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Production parent profile: 14,421,005,745 instructions with exact proof.
- Source-attribution profile: the retained line-enabled profile from
  Experiment 148, which records 877,339 search initializations. Its debug-info
  code generation is not used for production percentages.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained at
`.artifacts/experiments/2026-07-20-149-active-query/`. Compatibility reports
are retained under `.artifacts/e-compare/`.

## Rejected query-builder candidates

The parent query builder uses a recyclable vector of `Enter(Term)` and
`Exit(index)` frames. The first candidate let the query own each term
immediately and retained only one `(cell, next argument)` frame for active
non-leaf paths. It passed all 37 PD-tree tests, including the independent
recursive prefix-cell and subtree-span oracle, and produced the exact proof.
Repeated query indexing and active-frame updates dominated the eliminated
pushes, however: the candidate retired 14,960,276,504 instructions, a
539,270,759-instruction regression (+3.74%). Its profile is retained as
`rust-callgrind-active-query.out`.

The second candidate combined the three query-shape predicates into one
function-code/property/head classifier. Its test oracle explicitly matched
top-level-free-variable traversal and the first traversed argument for free
variables, DB variables, applied free/DB variables, lambdas, constants, and a
malformed empty phony application. It also preserved the exact proof, but
retired 14,426,595,483 instructions, 5,589,738 above the parent (+0.039%).
LLVM's inlining and common-subexpression choices for the existing predicates
were better than the manual branch tree. The profile is retained as
`rust-callgrind-query-shape.out`.

Both structural candidates were reverted completely.

## Accepted lifecycle result

Every valid search begins with `search_active == false`; C asserts the same
single-active-search precondition. Rust's `record_search_exit` moves the owned
query vector back to its recyclable scratch slot and clears `search_state`.
`record_search_init` nevertheless called the recycler again, so all 877,339
measured initializations borrowed the state only to observe `None`.

The accepted change removes that release-time no-op and replaces it with a
debug assertion that the previous exit already recycled the query. Query
construction, owned-term release, cursor reset, traversal state, and exit-time
recycling are unchanged.

The candidate produces the exact proof at 14,396,452,335 instructions,
24,553,410 below the parent (-0.170%). Cursor-exclusive work is exactly
unchanged at 1,302,541,615 instructions. `record_search_init` exclusive work
falls from 1,064,053,711 to 1,062,299,033 instructions, while the removed
recycler call and its inlined `RefCell`/option work account for the remaining
global reduction. The accepted profile is retained as
`rust-callgrind-no-init-recycle.out`.

## Compatibility result

The four-case proof report at
`.artifacts/e-compare/20260720-162303-636570/` has zero mismatches. The
two-case resource report at `.artifacts/e-compare/20260720-162513-383776/`
also has zero mismatches; BOO020 and SWV851 retain C-compatible `ResourceOut`
behavior at the stabilized Windows boundary.

The complete Rust suite passes 4,369 unit tests plus every integration target.
Strict all-target/all-feature pedantic Clippy, formatting, and the all-feature
release build pass.

## Falsification checks

- The independent recursive query oracle pins every prefix cell and subtree
  span; all 37 PD-tree tests passed for both rejected structural candidates.
- Exact proof output and the unchanged cursor instruction count rule out a
  search-order explanation for the accepted improvement.
- The debug invariant checks both halves of the lifecycle: a search cannot be
  active, and its previous owned query must already have been recycled.
- Existing storage-reuse coverage proves exit transfers the large query
  allocation to scratch and the next search reuses its capacity.
- Native proof and resource corpora cover normalized output, exit behavior,
  and the maintained Windows allocation boundary.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-no-init-recycle.out \
  target-wsl-149-no-init-recycle/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-149-no-init-recycle
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-149-no-init-recycle\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept removal of the redundant initialization-time query recycle and retain
the debug lifecycle invariant. Reject the active-path frame and combined-shape
classifiers. Keep the main parity issue open: the deterministic workload is
cheaper, but the synthetic one-second LUSK cutoff and overall C/Rust
performance ratio still fail the project-wide acceptance criteria.
