# PCL analysis compatibility edges

## Question

Do the remaining `pcl_analysis` differences represent missing drop-in behavior,
or are they intentional safe-Rust decisions that preserve every defined and
externally useful C behavior?

## Source audit

C `PCLExprProofDistance` passes the result of `PCLProtFindStep` directly to
`PCLStepProofDistance`. A missing quoted parent therefore dereferences null.
Rust checks the lookup and returns a syntax diagnostic. By contrast,
`PCLExprUpdateGRefs` deliberately treats a missing directly quoted generation
or simplification parent as a recursive quote visit, which is a no-op; Rust
preserves that silent counter-update behavior.

`PCLProtSelectExamples` serializes steps by PCL id, uses `qsort`, and compares
all proof steps equal. Non-proof steps compare the result of a C `float`
division, `useless_gen_refs / (useless_simpl_refs + 1)`. Equal scores have no
portable C tie order. Rust performs the same `f32` division and uses stable
PCL-id order for ties. Both implementations stop the entire selection loop
when the negative-example budget is zero, including before selecting proof
steps.

## Differential corpora

`equal-score-selection.pcl` contains 40 equal-score non-proof initial clauses
and one proof initial clause. A negative-example budget of one exposes which
tied negative is selected. `dangling-proof-distance.pcl` contains one step
whose quoted parent does not exist.

The archived upstream tool was built from commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` with GCC 13.3 and glibc 2.39.

```powershell
wsl.exe -d Ubuntu-24.04 -- `
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/tools/direct_examples `
  --negative-example-proportion=1 `
  experiments/2026-07-16-062-pcl-analysis-edges/equal-score-selection.pcl

.\target\release\direct_examples.exe `
  --negative-example-proportion=1 `
  experiments\2026-07-16-062-pcl-analysis-edges\equal-score-selection.pcl
```

The actual C command uses the `/mnt/c/...` form of the corpus path under WSL.
Raw outputs are intentionally ignored under
`.artifacts/experiments/2026-07-16-062-pcl-analysis-edges/`.

## Results

The equal-score outputs are byte-identical, with SHA-256
`54c04f8cb0edcc06a1480b27d3d71d8dd33256a68a1b1610dc0703e521e32fe1`.
Both select PCL id 1 as the sole negative example and id 100 as the proof
example. This confirms that deterministic PCL-id ties match the current target
C/glibc behavior on a corpus large enough to exercise a nontrivial `qsort`.

On the malformed corpus, the archived C process terminates with `SIGSEGV` while
Rust exits with its syntax-error status and prints
`direct_examples: Dangling reference in PCL protocol!`. The C failure is a null
dereference, not a diagnostic surface to reproduce.

Four new unit regressions complement the executable evidence:

- proof-distance lookup reports the exact syntax diagnostic for a dangling id;
- reference-counter updates silently ignore a missing direct parent while
  still updating a live sibling parent;
- tied examples select the lower PCL id even when parsed in reverse order;
- values separated above the 24-bit `float` precision boundary compare as the
  same C-shaped score.

The existing zero-negative-budget regression remains in place.

## Compatibility decision

No production change is retained. Rust keeps the safe diagnostic rather than
emulating undefined null-dereference behavior, preserves C's silent no-op where
the C code actually checks the missing parent, and retains deterministic PCL-id
ties. Matching an undocumented libc-specific unstable order would reduce
portability without improving the defined drop-in contract; the current target
already matches exactly. C `float` rounding and the surprising zero-budget loop
remain compatibility-visible.

## Validation

The focused eight-test `pcl2::analysis::tests` module passes. The current
five-case `direct_examples` comparison report is
`.artifacts/e-compare/20260716-220759-170178-tools/`; help, version, basic
stdin, branching protocol, and missing-input cases all match with zero
mismatches.

Final repository gates pass formatting, all-target/all-feature checking,
pedantic Clippy with warnings denied, all 4,186 library tests plus binary and
integration targets, and a locked release build of every binary. The 32 Python
interoperability tests and all four C-source documentation checks also pass.
