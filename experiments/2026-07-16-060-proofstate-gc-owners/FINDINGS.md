# Proof-state GC owner contexts

## Question

Can the remaining formula-CNF garbage-collection helper slices be replaced by
a typed proof-state owner context while preserving C `TBGCCollect` coverage,
standalone formula-tool behavior, and proof output?

C stores untyped clause/formula-set pointers in the term bank and marks every
registered set. Rust had stable numeric proof-state handles, but
`ProofState::collect_term_garbage` rebuilt handle and owner vectors on every
collection, while formula CNF constructed explicit local root slices. The
local slices had already been widened to the active formula set, its archive,
and the generated clause set after the GEO288 sharing investigation, but they
were not an owner-typed representation of the full proof-state registry.

## Candidate

The retained implementation makes clause and formula proof-state roots typed
enums with stable handle conversions and direct owner resolution.
`ProofState::collect_term_garbage` walks those variants, checks current
registration, marks the resolved owners, and sweeps without allocating handle
vectors or scanning temporary `(handle, owner)` arrays.

Formula transformations now receive a `FormulaSetGcContext`. Standalone tools
use a local context that marks the same active/archive/clause trio as before.
Proof-state CNF callers receive a borrow-checked owner context containing every
other registered proof-state set; the active formula axioms, formula axiom
archive, and clause axioms are supplied by the live CNF operation. The context
therefore marks all 12 clause owners and all four formula owners while still
honoring watchlist deregistration. The generic slice-based `tb_gc_collect`
surface remains available and tested as the low-level compatibility helper,
but it has no production call sites.

## Setup and commands

Focused owner and CNF tests:

```powershell
cargo test --lib proof_state_collect_term_garbage -- --nocapture
cargo test --lib proof_state_formula_cnf_gc -- --nocapture
cargo clippy --lib --all-features -- -D warnings -W clippy::pedantic
```

Full Rust validation:

```powershell
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings -W clippy::pedantic
cargo test --locked --all-targets --all-features
cargo build --locked --release --bin eprover
wsl -d Ubuntu-24.04 -- bash -lc "cd /mnt/c/Users/rober/Code/E_Rust_Port && CARGO_TARGET_DIR=/tmp/e-rust-port-codex-target cargo build --locked --release --bin eprover"
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Results

- The direct collection regression populates every one of the 12 clause-root
  variants and four formula-root variants, runs collection, verifies all 16
  owners retain their terms, and verifies an unrooted term is recovered.
- The formula-CNF regression stores a retained term only in the unrelated
  general formula archive, triggers CNF garbage collection through the typed
  proof-state context, and verifies that term survives while an unrooted term
  is recovered.
- All 4,180 library tests and every binary/integration target pass. Locked
  all-target/all-feature check and pedantic Clippy pass, as do Windows and WSL
  release builds.
- The 50-case report is
  `.artifacts/e-compare/20260716-202430-166087/comparison.json`. Forty-six
  cases match. The established `HEN011-2.p`, `sledgehammer.p`, and synthetic
  one-second CPU-limit cases remain, and the known near-limit `GEO288+1.p`
  case reports `ResourceOut` at 55.98 wall seconds. The immediately preceding
  report proved GEO288 in 55.11 seconds, so this membership remains
  load/limit-sensitive rather than evidence of a new formula-CNF output
  divergence.
- The standard five-run native report is
  `.artifacts/e-compare/20260716-204050-608640-benchmark/benchmark.json`.
  It measures a 2.649x aggregate Rust/C wall ratio across the nine
  behavior-matching cases. `LUSK6.lop` is 2.724x with 241,432 KiB Rust maximum
  RSS, and `LUSK6ext.lop` is 2.518x with 467,912 KiB. The timeout-sensitive
  `BOO020-1.p` outcome differs and is excluded. These values show no sustained
  proof-search memory regression from the owner-context representation; the
  project-wide 1.10x performance requirement remains unmet.

## Falsification checks

- Standalone `FormulaSetCNF2` tests still use the local context and pass,
  including recovery of unrooted CNF scratch terms and retention from the
  active set, archive, and generated clauses.
- The proof-state context checks the live term-bank registry before each typed
  root, so discarding the optional watchlist continues to remove it from the
  effective root domain.
- `rg 'tb_gc_collect\(' src` finds only the four low-level unit tests; no
  production formula transformation rebuilds helper root slices.
- The nested upstream `eprover/` checkout remains unmodified.

## Conclusion and limits

Retain the typed owner contexts. They encode the C global-root contract without
raw pointers, eliminate collection-time helper allocations and nested handle
resolution scans, and make full proof-state versus standalone root domains
explicit. The change closes the GC-marker representation part of
`E_Rust_Port-j76.1.8`.

The bead remains open because exact formula free/delete allocation behavior and
the formula derivation-owner overhead are still pending. This experiment does
not claim whole-port performance parity or resolve the known near-limit proof
search cases.
