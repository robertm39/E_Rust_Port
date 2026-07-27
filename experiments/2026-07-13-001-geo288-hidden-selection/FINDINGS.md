# GEO288 Hidden Selection Divergence

## Question

What causes C to perform one additional hidden HCB selection before visible
selected-clause ordinal 559 on `GEO288+1.p`, after long-list triviality parity
has been restored?

## Setup

- C reference: `/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover`
- Rust executable: `target/release/eprover`
- Problem: `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`
- Shared arguments: `--auto --output-level=0 --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new`

The Rust executable was rebuilt in release mode after each production probe.
Short traces used `--processed-clauses-limit` to stop after the relevant
selected-clause window. C traces ran the cached reference binary under GDB in
`Ubuntu-24.04`; Rust traces used temporary `eprintln!` probes that were removed
from production source before validation.

## Results

1. The first raw HCB-identifier mismatch is call 995. C selects
   `LONG_MIN+3080` and Rust selects `LONG_MIN+3079`, but both clauses are
   structurally `vplus(X1,v0)=X1 <- rreal(X1)`. The schedule's
   `current_eval`/`select_count` state agrees. This mismatch is an identity/order
   tie, not the first structural inference difference.
2. The later C-only selected clause is a 20-literal paramodulant printed as
   `i_0_4972`. Rust generates the same clause with the same variable identities
   and parent derivation. Before this slice, generated-clause admission rejected
   it through C-compatible `EqnLongListIsTrivial` ordering even though the
   quadratic comparison exposed the opposite-polarity duplicate directly.
3. Rust was running `ClauseIsTautology` against the permanent proof-search term
   bank. C uses persistent `state->tmp_terms`. The wrong Rust owner permanently
   interned scratch terms, including `ron(-12,-8)`, early enough to change the
   long-list comparison order. `ProofState` now owns/adopts a persistent scratch
   bank and forward tautology checks use it. The scratch bank is swept at C's
   post-generation position when it exceeds 256 non-variable nodes.
4. After fixing the scratch owner, the first permanent-bank growth mismatch in
   selected-clause processing was the second processing occurrence of clause
   417. C unconditionally calls `ClauseCopyDisjoint` after selected-clause
   variable normalization, even when generation/paramodulation will not use the
   copy. Rust copied only on the paramodulation path. The port now normalizes
   once, creates one unconditional disjoint copy, reuses it for ExtSup, choice
   scanning, and paramodulation, and drops it before scratch-bank GC.
5. With both corrections, normalized permanent-bank growth matches C for all
   468 captured forward-contraction calls through clause 276. Rust remains at a
   uniform absolute offset of 39 terms established during formula CNF:
   C enters proof control at `in_count=47187`; Rust enters at `47226`.
6. The structurally matched 20-literal generated clauses now report
   `long_is_trivial=false` and survive admission while the quadratic check still
   reports the duplicate. This is the required C compatibility behavior.
7. GEO288 still reaches the 60-second resource limit after these fixes. The
   remaining investigation starts at the independent 39-term CNF sharing
   offset and later HCB tie permutations, not at the repaired scratch/copy
   boundaries.

## Validation

- `cargo test` passed 4,041 library tests, all binary targets, the three
  `eprover_schedule` integration tests, and doc tests.
- `cargo clippy --all-targets --all-features -- -D warnings`, the locked Windows
  release build, and all four C-source documentation integrity checks passed.
- The 50-case differential report at
  `.artifacts/e-compare/20260713-075545-040361/` retains the established six
  mismatches and introduces no new one. The remaining cases are `BOO020-1.p`,
  `LUSK6ext.lop`, `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and the
  synthetic CPU-limit `LUSK6.lop` fixture.
- The five-run native report at
  `.artifacts/e-compare/20260713-080946-892541-benchmark/` measures a `3.440x`
  aggregate Rust/C median wall-time ratio. `BOO020-1.p` is excluded for
  differing behavior; `LUSK6.lop` measures `3.204x` and `LUSK6ext.lop`
  measures `3.103x`. Performance remains outside the required `1.10x`.

## Raw Artifacts

Generated traces are stored under the ignored directory
`.artifacts/experiments/2026-07-13-001-geo288-hidden-selection/`.

Key files are:

- `c-hcb-calls.txt` and `rust-hcb-calls-after-copy.txt`
- `c-ident276-banks.txt` and `rust-forward-contract-after-copy.txt`
- `c-ident417-insertions.txt`
- `c-hcb-call995-clause.txt` and `rust-selected-call995-after-fixes.txt`
- `c-second-selected-after-123.txt`
- `rust-clause-4972-entries.txt` and
  `rust-target-admission-after-tmpbank.txt`

`compare-hcb.py` compares HCB traces and can enumerate all identifier
mismatches with `--all`. `compare-bank-chronology.py` compares term-bank growth
after normalizing each trace to its initial count.

## Falsification Checks

- The C and Rust clause-276 parents and three equality-resolution intermediate
  clauses were compared structurally and by variable code; they agree.
- The target 20-literal raw clause is generated on both sides, ruling out a
  missing paramodulation or equality-resolution inference.
- The first HCB identifier mismatch was captured structurally in both binaries;
  the clauses agree, ruling out that ID difference as the causal structural
  divergence.
- A one-call scratch-bank probe showed that the permanent-bank insertion came
  from `clause_is_tautology`, and moving only that call to `tmp_terms` removed
  it. This rules out ordinary rewriting and disjoint inference copying as the
  source of that pollution.
- Entry-by-entry processing chronology matched until clause 417 and the C
  insertion backtrace reached `ClauseCopyDisjoint` from `ProcessClause`.
  Unconditional Rust copying closed that mismatch and every later normalized
  growth delta in the 468-call capture.
- `TermTopCompare` was checked directly: despite stale header commentary, C does
  not compare term properties in the sharing key. Property-key mismatch is not
  the source of the remaining 39 terms.

## Conclusion And Limits

Persistent scratch-bank ownership and unconditional selected-clause disjoint
copying are compatibility-visible allocation semantics, not optional memory
optimizations. The production port and regression tests now preserve both.

This experiment does not explain Rust's 39 additional unique permanent terms at
the end of formula CNF, nor does it establish full GEO288 proof-search parity.
Those are the next boundary. The ignored raw traces are diagnostic evidence;
the checked-in scripts and this record are the reproducible experiment assets.
