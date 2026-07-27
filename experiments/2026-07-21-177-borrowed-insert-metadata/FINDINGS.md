# Borrowed top-insertion metadata

## Question

Can `TBTermTopInsert` compute child-derived metadata from one borrowed argument
slice, matching C's direct `term->args` traversal and avoiding a temporary
vector of cloned reference-counted handles?

## Setup

- Parent source: commit `8896b032` (`Borrow term arguments while collecting
  subterms`), the accepted Experiment 176 implementation.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,129,703,657 instructions with the exact proof and 4,873
  processed clauses.
- Final candidate profile:
  `.artifacts/experiments/2026-07-21-177-borrowed-insert-metadata/rust-callgrind-borrowed-insert-metadata-final.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

C's `TBTermTopInsert` loop reads each child directly from the top term's
argument array while propagating properties and summing variable count,
function count, and weight. Rust previously called `argument_clones`, creating
a `Vec` and incrementing and decrementing every child `Rc` during every
top-level insertion.

The candidate holds one scoped immutable `arguments()` borrow, unwraps each
slot by reference, and performs the same metadata operations without cloning
child handles. The scope ends before the top term's metadata is written. A
regression preserves the C assertion-shaped failure for an uninitialized
argument slot.

## Resource-boundary correction

The first complete matrix at
`.artifacts/e-compare/20260721-115451-671331/` exposed two deadline-edge
failures even though the deterministic proof and focused resource cases were
exact. BOO020 aborted in an infallible 196,608-byte allocation, and SWB008
reported that a processed clause disappeared before global-index insertion.

The SWB failure identified a real control-flow bug in the existing non-Linux
fallible-admission boundary. `ClauseSet::insert` could reject a new page after
latching the cooperative deadline, but its caller could not observe that
rejection and proceeded to resolve the missing clause for global indexing.
Insertion now reports success, indexed insertion propagates that result, and
the selected-clause path stops before global indexing when admission is
rejected. The next saturation-loop limit check emits normal `ResourceOut`.
A regression fills the first processed non-unit page, expires the deadline,
and verifies that rejected insertion returns no class, leaves the set intact,
and latches `TimeIsUp`.

The corrected isolated BOO report is
`.artifacts/e-compare/20260721-122114-559305/`. A valid SWB008 corpus including
its TPTP axiom file is exact at
`.artifacts/e-compare/20260721-123209-505820/`; both implementations return
`ResourceOut` rather than a Rust diagnostic.

## Performance result

The finalized candidate preserves the exact 4,873-clause proof at
11,993,700,044 instructions, 136,003,613 below the parent (-1.1212%). The
deterministic C/Rust ratio improves from 2.3085 to 2.2826.

`Term::argument_clones` falls from 50,686,811 to 7,110,531 exclusive
instructions (-85.97%). The inlined metadata change lowers `term_top_insert`
from 264,008,366 to 256,149,191 (-2.98%), libc `malloc` from 313,491,791 to
290,004,395 (-7.49%), and `_int_free` from 392,605,682 to 364,295,535 (-7.21%).
The final deadline-status propagation changes the Linux build's total by only
442,053 instructions relative to the initial candidate profile, so the
ownership reduction remains the source of the gain.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-124149-281227/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-123740-846790/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Focused SWB report `.artifacts/e-compare/20260721-123209-505820/` preserves
  normalized `ResourceOut` and exit 8 for both implementations.
- Full report `.artifacts/e-compare/20260721-124328-609996/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. BOO020, SWB008, SWV851, HEN011, and the synthetic one-second
  LUSK case all retain the C outcome.

## Validation

- `cargo fmt --all -- --check`
- 4,383 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept borrowed top-insertion metadata and the observable fallible-admission
result. The ownership change matches C's direct argument-array walk and removes
1.1212% of complete deterministic prover instructions. The admission result
also closes a real deadline race without changing successful insertion or
Linux signal behavior. Keep the main performance issue open: the remaining
deterministic C/Rust instruction ratio is 2.2826.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-borrowed-insert-metadata-final.out \
  target-wsl-177b-borrowed-insert-metadata/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-177-borrowed-insert-metadata
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-177-borrowed-insert-metadata\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-177-borrowed-insert-metadata\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -RustExe .\target\native-177-borrowed-insert-metadata\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
