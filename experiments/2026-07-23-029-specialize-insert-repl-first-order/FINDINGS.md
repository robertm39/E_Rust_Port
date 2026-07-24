# Experiment 267: Specialize first-order replacement insertion

## Status

Accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can `TermBank::insert_repl` avoid higher-order applied-variable dereference
work when the active problem is first-order, without changing the general
higher-order or not-yet-classified insertion path?

## Hypothesis and implementation

Upstream `TBInsertRepl` computes an LFHO dereference prefix limit at each
recursive node and converts the dereference mode for each child. In a
first-order problem there are no applied free variables: the prefix limit is
always zero, and child conversion preserves the current dereference mode.

The accepted Rust path nevertheless called
`deref_root_no_whnf_if_changed` and `convert_lfho_deref` throughout roughly
2.26 million recursive replacement calls in the exact workload.

The candidate:

- reads the thread-local problem type once at the public entry;
- dispatches exactly `ProblemType::FirstOrder` to a const-specialized
  recursive helper;
- uses `term_deref_if_changed` directly and passes the current dereference
  mode to first-order children;
- keeps `HigherOrder` and `NotInitialized` on the byte-for-byte-equivalent
  general LFHO logic; and
- recurses inside the selected const instantiation, so it does not repeat the
  problem-type dispatch.

A focused first-order regression follows a bound variable once, replaces a
nested bank term, and verifies the resulting shared term. Existing replacement
tests continue to cover LFHO applied-variable prefix expansion and ordinary
property behavior.

## Deterministic measurement

Accepted Experiment 261:

- Rust instructions: 9,106,424,013
- C instructions: 5,254,361,329
- Rust/C ratio: 1.733117

The candidate preserves the exact `Unsatisfiable` LUSK6 proof and retires
9,024,090,576 instructions:

- global delta: -82,333,437;
- global improvement: 0.904125%;
- new Rust/C ratio: 1.717448.

The intended replacement/dereference boundary changes as follows:

| Exclusive owner | Accepted | Candidate | Change |
| --- | ---: | ---: | ---: |
| replacement recursion | 252,232,352 | 333,579,421 | +81,347,069 |
| general changed-only root dereference | 177,772,483 | 45,705,727 | -132,066,756 |
| aggregate | 430,004,835 | 379,285,148 | -50,719,687 (-11.795143%) |

The replacement helper grows because LLVM emits two first-order recursive
clones, but removing the LFHO root checks more than repays that cost. The
remaining global improvement comes from downstream inlining and layout around
the specialized call graph.

The raw profile is retained at:

```text
.artifacts/experiments/2026-07-23-029-specialize-insert-repl-first-order/rust-callgrind-specialize-insert-repl-first-order.out
```

## Native production measurement

The default-feature Windows candidate is 8,937,472 bytes, 8,704 bytes larger
than the 8,928,768-byte accepted Experiment 261 binary. Both binaries exit zero
and emit byte-identical proof output.

Four alternating warmup pairs were excluded. Across 64 alternating measured
pairs:

- wall means improve 0.637579%, from 1.521399 to 1.511699 seconds;
- process-CPU means improve 0.511636%, from 1.479248 to 1.471680 seconds;
- wall and CPU medians improve 1.668633% and 2.127660%;
- mean paired wall and CPU improvements are 0.628809% and 0.481155%;
- median paired wall and CPU improvements are 1.600244% and 1.985063%;
- the candidate wins 52 wall and 40 CPU pairs, with four CPU ties.

The stable last 32 pairs remain positive:

- wall and CPU means improve 0.496473% and 0.333667%;
- wall and CPU medians improve 1.738049% and 1.604278%;
- mean paired wall and CPU improvements are 0.458959% and 0.311176%;
- median paired wall and CPU improvements are 1.603176% and 0.537634%;
- the candidate wins 26 wall and 16 CPU pairs, with three CPU ties.

Raw warmup and measured rows are in `native-warmup.csv` and
`native-lusk.csv`.

## Compatibility and validation

- The maintained report
  `.artifacts/e-compare/20260723-202716-326849` completes all 50 cases with
  zero unexpected mismatches and only the declared `sledgehammer`
  normalized-output difference.
- The report covers the maintained one-second LUSK6 proof, HEN, GEO,
  higher-order cases, and the BOO/SWV resource boundaries.
- All three focused replacement tests pass with default and all features.
- The full serial all-target/all-feature suite passes 4,393 library tests plus
  every integration and binary target.
- Strict default-library, all-feature-library, and
  all-target/all-feature pedantic Clippy pass.
- The locked all-target/all-feature release build, formatting, and
  `git diff --check` pass.
- C-source coverage, Change Later wording, Markdown links, and
  regeneration-preservation checks pass.
- The original `eprover/` checkout remains clean.

The first full-target Clippy attempt exhausted the Windows paging file while
the compiler requested another 1.5 MiB. Repeating serially with incremental
compilation disabled completed successfully, so the failed attempt is
environmental rather than a code diagnostic.

## Falsification checks

- The first-order regression forces the new dispatch and checks both
  dereferencing and recursive replacement.
- The existing applied-free-variable test remains on the general path and
  verifies LFHO prefix-limit semantics.
- Direct WSL and Callgrind runs both prove the exact theorem and exit zero.
- Windows candidate and accepted binaries produce byte-identical proof text.
- Deterministic instructions, all-pair native means, and stable last-half
  native means all improve.
- The full maintained matrix includes first-order, higher-order, proof,
  protocol, CPU-limit, and memory-limit behavior rather than relying on the
  profiled theorem alone.

## Decision and limits

Accept. The specialization removes provably inert LFHO work from first-order
replacement recursion, improves exact instructions by 0.904%, improves warmed
native wall and CPU time, and preserves every maintained compatibility and
resource result. The accepted baseline becomes 9,024,090,576 instructions, or
1.717448 times C.

The port is still not at performance parity: the exact workload retires
71.7448% more instructions than C, and the maintained HEN workload remains
materially slower. Keep Bead `E_Rust_Port-j76.5.3` open.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-specialize-insert-repl-first-order.out \
  target-wsl-267-insert-repl-fo/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-261-global-size-freelist\release\eprover.exe `
  -CandidateExe .\target\native-267-insert-repl-fo\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-029-specialize-insert-repl-first-order\native-lusk.csv
```

```bash
python3 tools/e-interop/e_interop.py compare \
  --repo-root . \
  --rust-windows target/native-267-insert-repl-fo/release/eprover.exe \
  --timeout 60 \
  --memory-limit-mb 2048
```
