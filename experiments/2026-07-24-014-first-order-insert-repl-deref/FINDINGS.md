# Experiment 287: First-order replacement dereference

## Status

In progress for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the already-monomorphized first-order `TermBank::insert_repl` recursion
follow only ordinary variable bindings, matching non-LFHO C `TermDeref`,
without paying applied-free-variable checks at every recursive node?

## Baseline

- Parent commit: `6cd747e4` (accepted Experiment 286).
- Exact default-feature LUSK6 Callgrind: 8,828,399,104 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.680204.
- Native parent:
  `target/native-286-fused-diversity-traversal/release/eprover.exe`.

## Candidate

The public replacement entry already reads `problem_type()` once and selects
a const-generic first-order recursive body. The candidate routes only that
body through a free-variable-only changed dereferencer. `DerefType::Once`
still consumes exactly one ordinary binding, `DerefType::Always` still
follows the full binding chain, and the general higher-order body continues
to use the existing applied-variable-capable helper unchanged.

## Result

### Variant A: forced inline

The exact theorem is preserved, but the candidate retires 8,950,908,440
instructions: 122,509,336 more than the accepted baseline, a 1.387673%
regression. Raw call arcs show less recursive replacement and top-insertion
work, but the newly inlined first-order body grows and perturbs work outside
the intended owner.

Variant B keeps the first-order dereference helper out of line so its loop is
not duplicated into both optimized recursive replacement bodies.

### Variant B: out of line

The exact theorem is again preserved, but the candidate retires
8,999,604,652 instructions: 171,205,548 more than the accepted baseline, a
1.939260% regression. The saved LFHO checks do not repay the helper call and
the resulting optimized-layout changes.

## Validation

- Both focused first-order dereference tests passed.
- All three replacement-insertion tests passed, including the existing LFHO
  applied-variable prefix regression.
- Strict all-feature library pedantic Clippy and formatting passed.
- Both default-feature fingerprints record exactly `features=["default"]`.
- Both Callgrind candidates prove the exact LUSK6 theorem, report
  `Unsatisfiable`, and exit zero.
- The vendored `eprover/` checkout was not modified.

## Decision

Reject both variants and restore accepted Experiment 286 source byte-for-byte.
The first-order replacement body is already specialized enough that a second
dereference implementation loses to compiler layout and call overhead.
Native timing and the full compatibility matrix are skipped because both
deterministic candidates regress by more than one percent.

Raw evidence:

- `.artifacts/experiments/2026-07-24-014-first-order-insert-repl-deref/callgrind-candidate.out`
- `.artifacts/experiments/2026-07-24-014-first-order-insert-repl-deref/callgrind-candidate-out-of-line.out`
