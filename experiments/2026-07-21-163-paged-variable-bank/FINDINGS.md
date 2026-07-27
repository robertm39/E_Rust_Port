# Paged direct variable-bank lookup

## Question

Can Rust replace the hot ordered-map lookup in `VarBank` with C-like direct
negated-f-code indexing without reopening the maintained BOO020 memory
boundary?

## Setup

- Parent source: commit `96ab2940` (`Stabilize subterm index ordering`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,889,454,347 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained accepted profile is
`.artifacts/experiments/2026-07-21-163-paged-variable-bank/rust-callgrind-paged-varbank.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## Dense-vector falsification

C's `VarBank` indexes a `PDArray` by `-f_code`; the Rust parent used a
`BTreeMap<FunCode, Term>`. A direct `Vec<Option<Term>>` prototype preserved the
exact proof at 12,756,221,787 instructions, 133,232,560 below the parent
(-1.0337%). `VarBank::var_assert_alloc` fell from 237,830,146 to 108,406,315
exclusive instructions. The profile is
`.artifacts/experiments/2026-07-21-163-dense-varbank/rust-callgrind-dense-varbank.out`,
and proof report `.artifacts/e-compare/20260721-031945-482748/` is exact.

That layout is not viable. BOO020 aborts in the Windows allocator while asking
for 139,264 bytes near the 2-GiB boundary in report
`.artifacts/e-compare/20260721-032126-644751/`. Correcting the vector's initial
length to C's 30-entry `PDArray` size does not change the failure; report
`.artifacts/e-compare/20260721-032858-149558/` aborts on the same request.
Temporary banks can contain a few high-numbered variable codes, so a single
dense vector materializes too many empty slots.

## Accepted representation

`VariableTable` uses a direct two-level index. Its outer vector addresses
64-entry pages, and each boxed page is allocated only when one of its variable
codes is present. Lookup remains constant-time through the negated f-code, but
sparse high codes do not allocate intervening term slots. The surrounding
sort stacks, counters, shadow allocation, and C-compatible collection loop are
unchanged. A regression allocates code `-1_000_002` and verifies that only one
term page is live.

The focused BOO run `.artifacts/e-compare/20260721-033650-039765/` closes the
failure reproduced by both dense variants. Combined resource report
`.artifacts/e-compare/20260721-034133-930994/` is exact for BOO020 and SWV851;
both implementations return normalized `ResourceOut` rather than an allocator
abort. Focused proof report `.artifacts/e-compare/20260721-035048-707736/` is
exact for all four proof cases.

## Performance result

The paged candidate preserves the exact LUSK6 proof at 12,778,448,460
instructions. This saves 111,005,887 instructions globally (-0.8612%) and
improves the C/Rust instruction ratio from 2.453 to 2.432.
`VarBank::var_assert_alloc` falls to 124,117,451 exclusive instructions, a
113,712,695-instruction reduction (-47.81%). The extra page indirection costs
22,226,673 instructions relative to the rejected monolithic vector but retains
most of its end-to-end benefit without its sparse-memory failure.

## One-second boundary

Full report `.artifacts/e-compare/20260721-035242-153842/` has one unexpected
difference plus the declared `sledgehammer.p` proof-order difference. The
unexpected row is the already narrow synthetic one-second LUSK6 case: C proves
in 0.409 seconds, while Rust reports `ResourceOut` after 1.007 seconds. All
ordinary proof and constrained resource rows in the same 50-case inventory
match.

This result occurred during a host-wide slowdown rather than a demonstrated
candidate regression. Immediately afterward the candidate missed the exact
one-second invocation in 20 of 20 isolated runs, but the accepted parent binary
also missed it in five of five runs. Without the cutoff, ten alternating
parent/candidate pairs all produced the same proof; their medians were about
1.960 and 1.968 wall seconds respectively, with the parent's first cold run at
3.364 seconds. The deterministic instruction result therefore governs this
slice, while the one-second compatibility margin remains open under the
existing performance Bead.

## Validation

- `cargo fmt --all -- --check`
- 4,376 library tests plus all integration targets and features
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept the lazy 64-entry paged variable table and reject both monolithic dense
vectors. The accepted layout restores C-like constant-time variable lookup,
improves deterministic end-to-end instructions by 0.8612%, preserves exact
proof and constrained-memory behavior in focused suites, and bounds sparse
term-slot allocation. Whole-prover performance parity remains incomplete at a
2.432 Callgrind ratio, and the synthetic one-second boundary remains too narrow
to close the active performance issue.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-paged-varbank.out \
  target-wsl-163-paged-varbank/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-163-paged-varbank
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-163-paged-varbank\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-163-paged-varbank\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
