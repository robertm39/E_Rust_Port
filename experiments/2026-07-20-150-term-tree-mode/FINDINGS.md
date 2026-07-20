# Term-tree mode capture and first-order type assertions

## Question

Can the hot term-tree comparator avoid repeated thread-local syntax-mode reads
and release-time ownership checks that C performs only as assertions, while
preserving higher-order type ordering and exact splay-tree behavior?

## Setup

- Parent source: commit `5852c932` (`Skip empty PDTree query recycling`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Production parent profile: 14,396,452,335 instructions with exact proof.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained at
`.artifacts/experiments/2026-07-20-150-term-tree-mode/`. Compatibility reports
are retained under `.artifacts/e-compare/`.

## Operation-scoped syntax mode

Rust's public `term_top_compare` reads the request-local problem type from a
thread-local cell. The splay loop called it for every comparison and rotation,
although a request cannot change from first-order to higher-order syntax during
one `find`, `insert`, or `extract`. C likewise observes one stable global
`problemType` during the operation.

The first accepted change reads the mode once after confirming the tree is
nonempty, passes it through every splay comparison, and reuses it for the final
root comparison. Empty-tree operations still perform no mode read, and each
server thread continues to capture its own request-local mode.

This candidate produces the exact proof at 14,392,006,361 instructions,
4,445,974 below the parent (-0.031%). Comparator-exclusive work is unchanged,
while `splay_term_tree` falls from 225,162,521 to 218,202,242 instructions.
The intermediate profile is retained as `rust-callgrind-term-tree-mode.out`.

## First-order type preconditions

C's `TermTopCompare` asserts that both types exist and that first-order terms
with equal function codes share one type pointer. Those checks disappear in a
production C build. Rust instead cloned both reference-counted `Type` handles
and performed an unconditional equality assertion on every equal-code
first-order comparison.

The final candidate keeps the presence and equality checks as Rust debug
assertions, so all debug/tests continue to validate the invariant. Higher-order
mode still retrieves both types, panics if either is absent, and compares type
identity as a semantic part of the term-tree key in every build.

The combined candidate produces the exact proof at 14,191,666,721
instructions, 204,785,614 below the parent (-1.42%) and 200,339,640 below mode
capture alone. `term_top_compare_for_problem` exclusive work falls from
659,862,666 to 458,520,131 instructions (-30.51%). Final splay work is
218,130,752 instructions. The final profile is retained as
`rust-callgrind-fol-types.out`.

## Compatibility result

The four-case proof report at
`.artifacts/e-compare/20260720-165238-827498/` has zero mismatches. The
two-case resource report at `.artifacts/e-compare/20260720-165913-900295/`
also has zero mismatches; BOO020 and SWV851 retain C-compatible `ResourceOut`
behavior at the stabilized Windows boundary.

The complete Rust suite passes 4,369 unit tests plus every integration target.
Strict all-target/all-feature pedantic Clippy, formatting, and the all-feature
release build pass.

## Falsification checks

- Focused term-tree tests cover function-code, type, arity, argument-identity,
  higher-order distinct-type, insertion, lookup, extraction, deletion, and
  property traversal behavior.
- Debug builds still retrieve and compare first-order type handles, so an
  invariant violation remains a test/debug failure exactly where C asserts.
- Higher-order type identity remains ahead of arity and argument identity in
  the production comparator key.
- Exact proof output and unchanged processed-clause behavior rule out a
  search-order explanation for the instruction reduction.
- Native proof and resource corpora cover normalized output, exits, and the
  maintained Windows allocation boundary.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-fol-types.out \
  target-wsl-150-fol-types/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-150-fol-types
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-150-fol-types\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept operation-scoped problem-type capture and debug-only first-order type
preconditions in the term-tree comparator. Keep higher-order type identity as
an unconditional production key. Keep the main parity issue open: the
deterministic workload is materially cheaper, but the synthetic one-second
LUSK cutoff and overall C/Rust performance ratio still fail the project-wide
acceptance criteria.
