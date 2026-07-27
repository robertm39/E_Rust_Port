# Experiment 294: Force-inline term argument borrowing

## Question

Can forcing `Term::arguments` into the structural-weight comparator remove its
hot borrow-helper call boundary while preserving the accepted borrow lifetime
and comparison code shape?

## Baseline

- Accepted source: Experiment 293, commit `b6f59ac1`.
- Exact default-feature LUSK6 Callgrind: 8,718,487,029 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.659286.
- `Term::arguments` owns 65,174,512 instructions over 3,127,396 calls. All
  calls are the four left/right edges of `term_struct_weight_compare`.

## Candidate

Add only a measured `#[inline(always)]` boundary to `Term::arguments`.
Argument representation, `RefCell` borrowing, `Ref::map`, comparator order,
borrow lifetime, and every caller remain unchanged.

This is distinct from rejected Experiment 252, which moved the argument
borrows before the arity check and held them across a larger comparator region.
The present candidate preserves the accepted separate `arity()` and
`arguments()` access shape and tests only call-boundary code generation.

## Validation

- All 232 focused term tests pass, including structural comparison and term
  argument representation coverage.
- Formatting and `git diff --check` pass.
- Candidate WSL and native fingerprints record exactly
  `features=["default"]`.
- The candidate reaches the exact LUSK6 proof under Callgrind.
- Three parent and eight candidate native runs produce identical 378-byte
  stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- All 128 measured native processes prove and exit zero.

## Measurement

The candidate retires 8,678,969,887 instructions, 39,517,142 below the
8,718,487,029-instruction parent. This is a 0.453257% reduction, and the
hypothetical Rust/C ratio improves from 1.659286 to 1.651765. The WSL binary
shrinks 26,160 bytes and the native binary shrinks 8,192 bytes.

Native production timing decisively reverses the instrumented result. Across
64 alternating pairs, wall and CPU means regress 0.880166% and 1.945016%;
paired means regress 0.927526% and 2.021494%. The candidate wins only 22 wall
and 15 CPU pairs, with 7 CPU ties.

The final 32 pairs remain worse: wall and CPU means regress 0.635370% and
2.300151%, paired means regress 0.677631% and 2.361027%, and the candidate
wins only 11 wall and 6 CPU pairs, with 3 CPU ties.

## Result

Reject. Inlining the borrow helper removes deterministic call overhead and
shrinks both binaries, but it substantially regresses native CPU throughput.
Restore the accepted Experiment 293 source byte-for-byte. Compatibility and
full repository gates are skipped after the decisive production rejection.
