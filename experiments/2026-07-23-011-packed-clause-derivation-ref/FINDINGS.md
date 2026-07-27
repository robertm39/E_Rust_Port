# Packed clause derivation reference

## Status

Rejected in Experiment 249 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

`ClauseDerivationRef` stores its identifier, four-bit CSSCPA source, and
process-local generation in three `u64` words. Packing the source into the low
four bits of the generation word could reduce:

- `ClauseDerivationRef` from 24 to 16 bytes;
- `DerivationEntry` from 32 to 24 bytes;
- clause derivation stack traffic and storage;
- PD-tree occurrence and ordered-key storage that embeds clause references.

The candidate retained the complete C-visible source range and used a checked
60-bit generation range. Exact equality, ordering, hashing, and debug rendering
were preserved.

## Baseline

The accepted Experiment 245 LUSK6 profile is:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851
- Rust allocation calls: 4,290,002

## Candidate validation

The candidate:

- passed all 36 focused derivation tests;
- passed `cargo check --locked --lib`;
- proved LUSK6 and exited zero;
- preserved the exact accepted proof-search statistics, including:
  - 4,873 processed clauses;
  - 120,780 generated clauses;
  - 505,214 total rewrite steps;
  - 2,479,628 term-bank top insertions.

Layout regressions confirmed that the candidate reduced
`ClauseDerivationRef` from 24 to 16 bytes and `DerivationEntry` from 32 to
24 bytes. Boundary tests covered source 15, the maximum packed generation,
source overflow, and generation overflow.

## Exact performance result

The exact LUSK6 Callgrind result was:

- Candidate instructions: 11,098,014,155
- Delta from accepted Rust: +1,199,579,389
- Delta percentage: +12.118880%
- Candidate Rust/C ratio: 2.112153

The instruction regression is decisive, so native timing, compatibility
matrices, and resource matrices were skipped under the deterministic rejection
rule.

The candidate and accepted profiles have the same proof-search counts, but the
candidate's added instructions are distributed broadly through term-bank
insertion, rewriting, indexed paramodulation, structural comparison, and
allocation. This is consistent with the smaller derivation allocation shape
perturbing heap addresses and downstream pointer-keyed tree topology; it is an
inference from the broad profile shift, not a directly isolated cause. The
packing masks and shifts alone do not account for the 1.20-billion-instruction
increase.

## Decision

Reject the packed reference and restore the accepted three-word
`ClauseDerivationRef` byte-for-byte. The smaller derivation entry is not a
performance improvement on the representative proof search and substantially
worsens the exact instruction ratio.

Post-revert validation:

- `git diff --exit-code -- src/clauses/clause.rs src/clauses/derivation.rs`
- `cargo test --locked --lib clauses::derivation::tests --quiet`: 33 passed
- `cargo fmt --all -- --check`
- vendored `eprover/` status: clean

Do not retry this packed derivation representation without a design that also
controls the downstream allocation-address/topology effect or new profile
evidence that the representative workload has changed.

## Raw artifact

The ignored Callgrind profile is:

`.artifacts/experiments/2026-07-23-011-packed-clause-derivation-ref/rust-callgrind-packed-clause-derivation-ref.out`

## Reproduction

```bash
CARGO_TARGET_DIR=target-wsl-249-packed-clause-derivation-ref \
  cargo build --locked --release --bin eprover

valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-packed-clause-derivation-ref.out \
  target-wsl-249-packed-clause-derivation-ref/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
