# Direct first-order PD-tree symbol dispatch

## Question

Can the first-order PD-tree cursor test a query term's function code directly,
matching C's symbol branch, instead of constructing full variable metadata for
a token that the symbol branch immediately rejects?

## Setup

- Parent source: commit `487e4963` (`Record rejected owned substitution
  normalization`), whose executable source is Experiment 166 commit
  `62afa4a7`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,625,510,206 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained profile is
`.artifacts/experiments/2026-07-21-168-pdt-first-order-symbol/rust-callgrind-fo-symbol.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Structural attribution

Experiment 155 specialized first-order prefix-token classification, but the
production symbol step still constructed `PrefixToken::FreeVar` for every
negative function code. That construction reads term identity, type UID, and
standard weight before `matches!` discards the token. C `pdtree_forward`
instead tests whether the query is a top-level free variable and, for an
ordinary first-order symbol, looks up `term->f_code` directly in
`f_alternatives`.

Experiment 166 made Rust's `fun_alternatives` the actual child map, so the
cursor can now follow the same path. The accepted branch returns immediately
for a negative first-order code and otherwise probes the integer map directly.
Higher-order and uninitialized modes still use the complete classifier and
object-token child dispatch, including DB variables, applied variables, and
lambdas. The test-only first-order token helper is retained to compare this
classification boundary against the general implementation.

## Performance result

The candidate preserves the exact 4,873-clause proof at 12,557,467,650
instructions. This is 68,042,556 below the parent (-0.5389%), improving the
deterministic C/Rust ratio from 2.403 to 2.390. The reduction is localized:
`search_next_matching_occurrence_impl` falls from 1,650,291,596 to
1,582,253,858 exclusive instructions, saving 68,037,738 (-4.122%). The other
major profile entries are unchanged.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-080831-822628/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-081033-716098/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-081445-634277/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. HEN and the synthetic one-second LUSK case both retain the C
  proof outcome, and all higher-order cases preserve their prior behavior.

## Validation

- `cargo fmt --all -- --check`
- 4,379 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept direct first-order function-code dispatch in the PD-tree symbol branch.
It removes work that C never performs, leaves higher-order classification
unchanged, reduces the dominant cursor by 4.122%, and passes the complete proof
and constrained-resource matrix. Keep the main performance issue open: the
remaining deterministic C/Rust instruction ratio is 2.390.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-fo-symbol.out \
  target-wsl-168-pdt-fo-symbol/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-168-pdt-fo-symbol
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-168-pdt-fo-symbol\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-168-pdt-fo-symbol\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
