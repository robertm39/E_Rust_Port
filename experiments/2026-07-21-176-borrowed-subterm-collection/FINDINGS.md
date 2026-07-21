# Borrowed shared-subterm collection

## Question

Can `TBTermCollectSubterms` traverse one borrowed argument slice per visited
term, matching C's direct `term->args` loop and avoiding a temporary vector of
cloned reference-counted handles?

## Setup

- Parent source: commit `9a03e9f9` (`Record rejected combined term hash
  helper`), whose executable source retains accepted Experiment 174.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,407,202,652 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-176-borrowed-subterm-collection/rust-callgrind-borrowed-subterms.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

C marks each newly visited shared term with `TPOpFlag`, pushes that pointer to
the caller's stack, and recurses directly over `term->args`. Rust preserved the
marking and stack order but called `argument_clones` at every visited node. That
allocated a `Vec`, incremented every child `Rc`, and decremented those handles
again after recursion.

The candidate holds one immutable `arguments()` slice while visiting a
parent's children and passes borrowed handles into the recursive call. Recursion
only changes per-node properties and the external collector; it never mutates
the parent's argument slots. The only retained handle clone is the one required
to store each newly discovered term in the owned `PStack`. The regression now
uses the same shared child in both parent positions and verifies that the DAG
node is counted and collected once.

## Performance result

The candidate preserves the exact 4,873-clause proof at 12,129,703,657
instructions, 277,498,995 below the parent (-2.2366%). The deterministic C/Rust
ratio improves from 2.3613 to 2.3085.

`Term::argument_clones` falls from 144,209,013 to 50,686,811 exclusive
instructions (-64.85%). The two compiled `tb_term_collect_subterms` bodies fall
from a combined 125,137,842 to 107,793,589 exclusive instructions (-13.86%).
Removing the temporary vectors and their child-handle traffic also lowers
libc `malloc` from 358,177,535 to 313,491,791 and `_int_free` from 449,772,846
to 392,605,682 exclusive instructions. The dominant PD-tree cursor remains
exactly 1,556,297,359 instructions, so the gain is localized to ownership and
allocation work rather than a changed proof search.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-111049-696758/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-111245-121205/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-111659-640914/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. HEN and the synthetic one-second LUSK case both retain the C
  proof outcome.

## Validation

- `cargo fmt --all -- --check`
- 4,381 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept borrowed shared-subterm collection. It matches C's direct argument-array
walk, removes ownership work with no semantic purpose, reduces the complete
deterministic prover by 2.2366%, and passes complete proof and constrained-
resource compatibility. Keep the main performance issue open: the remaining
deterministic C/Rust instruction ratio is 2.3085.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-borrowed-subterms.out \
  target-wsl-176-borrowed-subterms/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-176-borrowed-subterms
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-176-borrowed-subterms\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-176-borrowed-subterms\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
