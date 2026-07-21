# Borrowed term-cell hashing

## Question

Can Rust compute C's term-cell-store hash from one borrowed argument slice,
avoiding repeated `RefCell` borrows and temporary `Rc` clones while preserving
the exact pointer-derived bucket key?

## Setup

- Parent source: commit `fc90095c` (`Record rejected cold evaluation arena`),
  whose executable source is accepted Experiment 170 commit `b4a8eed6`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,525,374,625 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained candidate profile is
`.artifacts/experiments/2026-07-21-174-borrowed-term-hash/rust-callgrind-borrowed-term-hash.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Structural attribution

C `TermCellHash` reads arity and the first two argument pointers directly from
the term cell. Rust previously called `Term::arity` once per threshold and
`Term::argument` once per selected child. Those helpers repeatedly borrowed
the `TermArgs` `RefCell`, and each argument read cloned and later dropped an
`Rc<TermCell>` solely to recover its address.

The accepted implementation holds one immutable argument-slice borrow,
checks its length, and passes borrowed child handles directly to
`term_identity_id`. Function-code normalization, pointer shifts, XOR order,
masking, zero/unary/n-ary dispatch, and uninitialized-slot panic behavior are
unchanged. A regression pins the panic contract in addition to the existing
exact hash tests.

## Performance result

The candidate preserves the exact 4,873-clause proof at 12,407,202,652
instructions, 118,171,973 below the parent (-0.9435%). The deterministic
C/Rust ratio improves from 2.3838 to 2.3613.

The old `TermCellStore::insert` boundary plus standalone `term_cell_hash`
retired 161,994,624 exclusive instructions. LLVM folds the borrowed hash into
the insertion boundary, which now retires 107,487,162 instructions, a direct
54,507,462 reduction (-33.65%). Eliminated temporary-handle drops and changed
inlining account for the remaining whole-program reduction; dominant PD-tree
and evaluation-index functions are unchanged.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-101255-049269/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-101439-101758/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-101901-006302/` has 50 cases,
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

Accept borrowed term-cell hashing. It directly matches C's pointer-read shape,
removes ownership work that contributes nothing to the hash, reduces the whole
deterministic prover by 0.9435%, and passes complete proof and constrained-
resource compatibility. Keep the main performance issue open: the remaining
deterministic C/Rust instruction ratio is 2.3613.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-borrowed-term-hash.out \
  target-wsl-174-borrowed-term-hash/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-174-borrowed-term-hash
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-174-borrowed-term-hash\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-174-borrowed-term-hash\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
