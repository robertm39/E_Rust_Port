# Formula-owner memory scaling

## Question

Does the remaining Rust/C CNF memory gap scale primarily with formula wrappers or with unique terms and signature symbols, and which retained allocations account for the repeated-owner slope?

## Setup

All commands were run from the repository root on 2026-07-15 (America/New_York), starting from commit `92bdb39ad8a616ceabfb876690294808c4ddd7c0`. `generate-corpora.ps1` creates five repeated-symbol and five unique-symbol FOF corpora from 100 through 20,000 axiom owners. `benchmark-scaling.sh` alternates C and Rust order over three runs for syntax-only and CNF-only execution, recording wall/CPU time, peak RSS, exit code, and SZS status. The final Rust repeated-owner CNF slope uses five runs per point.

```powershell
.\experiments\2026-07-15-009-formula-owner-memory-scaling\generate-corpora.ps1

wsl.exe -d Ubuntu-24.04 -- bash `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-009-formula-owner-memory-scaling/benchmark-scaling.sh `
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/raw

wsl.exe -d Ubuntu-24.04 -- valgrind --tool=massif --time-unit=B `
  --massif-out-file=/mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/raw/rust.massif `
  /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover `
  --cnf --silent --output-file=/dev/null `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus/repeated-01000.p

wsl.exe -d Ubuntu-24.04 -- bash `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-009-formula-owner-memory-scaling/benchmark-final-scaling.sh `
  /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/raw/rust-retained-final-scaling.csv
```

Raw corpora, timing CSVs, statuses, and Massif profiles are under `.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/` and are intentionally ignored by Git.

## Results

The baseline scaling separated owner overhead from symbol/term growth:

| Corpus | Implementation/phase | RSS slope |
| --- | --- | ---: |
| Repeated | C syntax | 0.284 KiB/owner |
| Repeated | C CNF | 1.485 KiB/owner |
| Repeated | Rust syntax | 0.437 KiB/owner |
| Repeated | Rust CNF | 5.018 KiB/owner |
| Unique | C syntax | 0.987 KiB/owner |
| Unique | C CNF | 2.314 KiB/owner |
| Unique | Rust syntax | 2.107 KiB/owner |
| Unique | Rust CNF | 6.674 KiB/owner |

At 20,000 repeated owners, C used 10,080 KiB syntax-only and 34,664 KiB CNF, while Rust used 14,372 KiB syntax-only and 106,768 KiB CNF. The CNF increment above syntax was therefore about 1.201 KiB/owner for C and 4.581 KiB/owner for Rust. Both retained exactly 20,001 clauses and reported the same term-bank insertion/GC counts, localizing the excess to owner-side allocation rather than extra logical objects.

Massif on 1,000 repeated owners identified two large Rust/C ownership differences that could be removed without changing standard output:

- Predefined conjecture-relative weight functions retained several deep `ClauseSet` clones solely to discover conjecture symbols during lazy initialization. Rust now captures a compact `BTreeSet<FunCode>` and drops all lazy source owners after initialization.
- Proof-state initialization materialized `eval_order_cloned`, cloning every source clause before `copy_to_bank`. C traverses evaluation nodes and performs one `ClauseCopy`. Rust now traverses stable evaluation-object handles and captures only a `ClauseDerivationRef` parent.

The 1,000-owner Massif peak fell from 5,266,603 bytes to 4,108,334 bytes, a reduction of 1,158,269 bytes (22.0%). The conjecture-weight source change removed 1,011,288 bytes at the peak, and the final profile no longer contains the whole-clause `eval_order_cloned` vector.

Five-run repeated-owner CNF medians after the retained changes:

| Owners | Rust wall | Rust peak RSS |
| ---: | ---: | ---: |
| 100 | 0.01 s | 7,040 KiB |
| 1,000 | 0.03 s | 10,080 KiB |
| 5,000 | 0.15 s | 24,680 KiB |
| 10,000 | 0.34 s | 42,496 KiB |
| 20,000 | 0.64 s | 78,296 KiB |

The final Rust CNF RSS slope is 3.584 KiB/owner, down 28.6% from 5.018 KiB/owner. At 20,000 owners, peak RSS fell 28,472 KiB (26.7%) from 106,768 to 78,296 KiB. The final Rust/C RSS ratio is 2.259x rather than 3.080x. Baseline and final Rust wall medians are both 0.64 seconds, so the retained memory reduction did not trade away throughput.

## Falsification checks

All 120 baseline scaling runs exited zero and returned `SZS status Unknown`; all post-change focused runs also exited zero. Repeated corpora hold term and signature growth effectively constant, so their linear gap cannot be attributed to unique-symbol insertion. Identical retained clause counts and term-bank statistics rule out accidental clause multiplication.

Two additional profile-guided changes were tested and rejected:

- Sorting sparse clause storage in place removed a 253,952-byte temporary vector per 1,000 clauses, but did not improve 20,000-owner process RSS beyond run-to-run noise. It was reverted rather than expand ordering risk for no measured endpoint benefit.
- C creates derivation stacks with `PStackVarAlloc(3)`, while Rust uses the six-entry average-occupancy allocation. Starting Rust at three entries reduced the 20,000-owner RSS further, but changed standard `lists.p` proof output from quantified variable order `X1,X2,X3` to `X2,X1,X3`. Reverting it restored the exact C line. The current wider Rust derivation entry representation needs a compatibility-safe redesign before that allocation can change.

The unique corpora expose a separate problem: at 20,000 owners, Rust syntax-only took 116.70 seconds versus C's 0.28 seconds, and Rust CNF took 119.55 seconds versus C's 0.50 seconds. That extreme syntax cost appears before the owner allocations changed here and cannot explain the repeated-symbol result. It is tracked separately as `E_Rust_Port-j76.1.47` rather than conflated with this allocation slice.

The stable-handle regressions check lookup before extraction, invalidation after extraction, and survival of holes/insertion/sorting. Proof-state initialization tests verify copied axioms, derivation parents, watchlist behavior, evaluation GC, and output behavior. Fun-weight regressions verify that compact conjecture symbols preserve weights and that lazy source owners are released after initialization.

## Conclusion and limits

The repeated-owner gap was dominated in part by Rust ownership choices, not extra formulas, clauses, or terms. Compact lazy inputs and handle traversal remove 26.7% of the 20,000-owner RSS while preserving outcomes, exact standard proof output, and throughput.

This does not complete `E_Rust_Port-j76.1.8`. Rust still has a 2.099 KiB/owner excess CNF slope over C and 2.259x C's peak RSS on this corpus. The largest final Massif owners are the formula CNF queue, retained clause stores/archive copies, temporary canonicalization storage, derivation entries, and term-bank base tables. The unique-symbol syntax pathology is a distinct follow-up investigation.

## Validation

Focused and full repository gates passed:

```powershell
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
```

The full test run passed 4,087 library tests, all binary targets, and all three schedule integration tests. New regressions cover compact conjecture-symbol capture and cleanup, compact clause-parent derivation references, stable evaluation-object lookup/invalidation, and proof-state initialization through evaluation handles.

The standard 50-case C/Rust report is `.artifacts/e-compare/20260716-002725-172034/comparison.json`. Its six mismatches exactly match the established baseline: `BOO020-1.p`, `LUSK6ext.lop`, `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and `synthetic/cpu-limit-LUSK6.lop`. The rejected three-entry derivation-stack experiment initially added `lists.p`; reverting it restored the exact quantified-variable order and removed that mismatch.

The standard five-run benchmark is `.artifacts/e-compare/20260716-003924-405501-benchmark/benchmark.json`. Its aggregate Rust/C wall ratio is `3.107x`, improved from the previous `3.533x`; the known `BOO020-1.p` behavior difference is the single excluded case.
