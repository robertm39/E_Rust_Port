# Unique-symbol parser scaling

## Question

Why did Rust syntax-only parsing grow from 0.18 seconds at 1,000 unique-symbol formula owners to 116.70 seconds at 20,000 owners when C remained effectively linear, and can the superlinear path be removed without changing formula-owner routing compatibility?

## Setup

All commands were run from the repository root on 2026-07-16 (America/New_York), starting from commit `9b453861c3cd4f6cd102e8339dc67d950e8c302f`. `generate-corpora.ps1` creates atom, implication, negation, and quantified FOF corpora with 100 through 20,000 owners and unique predicate, function, or constant names. `benchmark.sh` records five samples per implementation and size with `/usr/bin/time`; `analyze.py` rejects nonzero exits or incomplete sample groups and reports medians.

```powershell
.\experiments\2026-07-16-010-unique-symbol-parser-scaling\generate-corpora.ps1

wsl.exe -d Ubuntu-24.04 -- bash -lc `
  'cd /mnt/c/Users/rober/Code/E_Rust_Port && IMPLEMENTATIONS=c,current SHAPE=atom PHASE=syntax experiments/2026-07-16-010-unique-symbol-parser-scaling/benchmark.sh /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/current.csv'

wsl.exe -d Ubuntu-24.04 -- python3 `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-16-010-unique-symbol-parser-scaling/analyze.py `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/current.csv
```

The implication, negation, and quantified syntax runs set `SHAPE` to `implication`, `negated`, and `quantified`. The CNF run sets `SHAPE=atom PHASE=cnf`. Raw corpora and timing CSVs are under `.artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/` and are intentionally ignored by Git. Baseline Rust values come from the preceding experiment's `.artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/raw/scaling-metrics.csv`.

## Results

The superlinear work was not signature name lookup or insertion. Formula-owner routing created `TermBank::detached_empty()` parser probes, which deep-cloned the entire live signature before each formula. With unique symbols, the signature grew on every iteration and made syntax routing quadratic. The same clone occurred in probes for top-level negated or parenthesized `$distinct` formulas.

Rust now recognizes conservative, unambiguous first-order FOF/TFF shapes by scanning tokens only. Ordinary atoms, grouped and negated atoms, uniform associative chains, single nonassociative operators per parenthesis depth, and leading quantifier prefixes route directly to the represented parser. Ambiguous uppercase heads, mixed or chained operators, nested quantifier operands, FOOL `$ite`/`$let`, application/lambda syntax, and square-list forms retain the existing full parser probe. Top-level `$distinct` gates now use lexical lookahead; recognized forms still go through the existing validating parser.

Five-run syntax-only medians for unique atoms:

| Owners | Baseline Rust | Current Rust | C | Baseline/current |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 0.01 s | 0.00 s | 0.00 s | n/a |
| 1,000 | 0.18 s | 0.01 s | 0.02 s | 18.0x |
| 5,000 | 4.42 s | 0.03 s | 0.10 s | 147.3x |
| 10,000 | 20.83 s | 0.06 s | 0.17 s | 347.2x |
| 20,000 | 116.70 s | 0.12 s | 0.44 s | 972.5x |

At 20,000 owners, current Rust syntax time is 0.273x C. Rust peak RSS also falls from the 48,244 KiB baseline to 36,908 KiB, a 23.5% reduction, because the transient signature snapshot is gone.

The broader shape checks remain linear at 20,000 owners:

| Shape | Current Rust | C | Rust/C | Rust peak RSS | C peak RSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| Implication | 0.25 s | 0.75 s | 0.333x | 64,456 KiB | 40,480 KiB |
| Negated atom | 0.18 s | 0.62 s | 0.290x | 41,912 KiB | 26,560 KiB |
| Quantified atom | 0.14 s | 0.56 s | 0.250x | 33,060 KiB | 20,960 KiB |

The 20,000-owner atom CNF median falls from 119.55 to 0.60 seconds, a 199.3x speedup, and is 1.034x C's 0.58 seconds. Its 111,092 KiB Rust peak remains 2.17x C's 51,240 KiB; that retained-owner memory gap is separate work tracked by `E_Rust_Port-j76.1.8`.

## Falsification checks

Every focused C and current Rust sample exited zero. The four corpus shapes exercise the direct atom path, binary formula operands, leading negation, and leading quantification. A regression test also builds a live 4,096-symbol signature, classifies fresh supported formulas without mutating it, and verifies that ambiguous syntax keeps the compatibility-preserving parser-probing route.

The standard 50-case C/Rust differential report is `.artifacts/e-compare/20260716-013502-512262/comparison.json`. Its seven mismatches have all appeared in earlier reports: the established `BOO020-1.p`, `LUSK6ext.lop`, `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and `synthetic/cpu-limit-LUSK6.lop` gaps, plus the intermittent proof-text-only `lists.p` mismatch. Both implementations report `Theorem` for `lists.p`; earlier standard runs alternate between exact output and that already-observed normalized proof difference. No new parser-routing mismatch appeared.

The standard five-run benchmark is `.artifacts/e-compare/20260716-014758-969048-benchmark/benchmark.json`. Its aggregate Rust/C wall ratio is 2.995x, improved from the preceding 3.107x report. The harness excluded the one known timeout/outcome mismatch from the aggregate.

## Conclusion and limits

Repeated deep signature snapshots caused the unique-symbol quadratic behavior. Conservative token classification and lexical `$distinct` gates remove those snapshots from common first-order formula shapes while preserving full parser probes for ambiguous syntax. Rust is now linear and faster than C in syntax-only measurements through 20,000 unique owners, while atom CNF throughput is comparable to C.

This does not remove every `detached_empty()` formula probe. Ambiguous FOOL, higher-order, and syntactically mixed inputs intentionally keep the full compatibility check until their ownership boundary can be proven without parsing. It also does not complete the remaining formula-owner memory work in `E_Rust_Port-j76.1.8`.

## Validation

Focused and full repository gates passed:

```powershell
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --lib distinct --all-features
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
```

The full test run passed 4,088 library tests, every binary target, and all three schedule integration tests. The final Linux release also rebuilt successfully through the standard benchmark harness. The upstream C checkout remained clean and unchanged.
