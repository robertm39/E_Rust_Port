# Rejected forced inline always-dereference wrapper

## Question

Can the single-caller `term_deref_always` wrapper be forced inline so
unrelated source changes cannot make its 276-million-instruction body reappear
outside substitution normalization?

## Setup

- Parent source: commit `ab08fac9` (`Record rejected allocation-free proof
  lookup`), whose executable source remains accepted Experiment 214.
- Candidate: add only `#[inline(always)]` and its narrow pedantic-Clippy
  expectation to the crate-private `term_deref_always` wrapper. Its sole
  production caller is `Substitution::norm_term`; algorithm, ownership, and
  result are unchanged.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-221-force-inline-deref-always-wrapper/rust-callgrind-force-inline-deref-always-wrapper.out`.
- Native timing: default-feature release binaries, two alternating warmup
  pairs, then 64 alternating measured pairs in `native-lusk.csv`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate reaches the expected LUSK6 proof and falls from 10,632,641,385
to 10,630,700,102 instructions, a reduction of 1,941,283 or 0.018258%. The
hypothetical C/Rust ratio improves from 2.023584 to 2.023215. The standalone
wrapper is absent in both profiles, confirming that the parent already happens
to inline it for this binary.

The small gain is compiler-layout redistribution rather than a direct wrapper
saving. The PD-tree cursor and Rust allocation count reproduce exactly.
`TermTree::insert` falls by 15,839,576 instructions (-2.404848%), while
`Substitution::norm_term` rises by 6,073,187 (+1.388965%); smaller changes
account for the remainder.

## Native result

The production gate rejects the annotation. Across 64 warmed alternating
pairs:

- wall mean and median regress 0.399752% and 1.372666%;
- paired wall mean and median regress 0.551652% and 0.707373%;
- CPU mean and median regress 0.352447% and 0.877193%;
- paired CPU mean and median regress 0.488209% and 0.877193%;
- the candidate wins only 24 wall pairs and 26 CPU pairs, with one CPU tie.

The final 32 pairs are stronger rejection evidence: wall mean and median
regress 1.466484% and 1.965779%, while CPU mean and median regress 1.508502%
and 1.315789%. All 128 measured processes and all four warmup processes prove
and exit zero. The candidate executable is 8,644,608 bytes, 512 bytes smaller
than the 8,645,120-byte parent.

## Validation

- All 18 term-cell tests and all nine substitution tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The release candidate reaches the expected unsatisfiable result and exits
  zero under Callgrind.
- Source is restored byte-for-byte and the same focused tests plus formatting
  pass after rejection.
- Compatibility matrices were skipped after native rejection.

## Decision

Reject and restore the unannotated wrapper. Forced inlining prevents the
specific standalone symbol, but on the accepted source its small instrumented
layout gain becomes a consistent production regression. It is therefore not a
free prerequisite for reviving Experiment 220. Keep the accepted baseline at
10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```powershell
.\run-native.ps1 `
  -ParentExe ..\..\target\native-214-move-termtree-insert-links\release\eprover.exe `
  -CandidateExe ..\..\target\native-221-force-inline-deref-always-wrapper\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-deref-always-wrapper.out \
  target-wsl-221-force-inline-deref-always-wrapper/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
