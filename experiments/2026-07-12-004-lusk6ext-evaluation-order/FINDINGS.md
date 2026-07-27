# LUSK6ext evaluation-order investigation

## Question

Why does the Rust proof first diverge from C at selected-clause ordinal 64 even
though both implementations assign the competing clauses identical HCB
evaluation tuples?

## Reproduction

The reference command was:

```sh
gdb -q -batch \
  -x trace-c-target-eval.gdb \
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover
```

`trace-c-demod-addresses.gdb` uses the same reference and problem to record the
`ClausePosCell` addresses passed to `PDTreeInsert` for the two competing rules.
Both scripts assume the optimized x86-64 System V layout recorded in their
comments.

## Findings

- C source clause 618 and Rust source clause 593 receive the same four HCB
  evaluations. The other selected-clause candidate also receives the same tuple
  in both implementations, so evaluation and orphan handling are not the first
  divergence.
- C selects raw clause 680 and rewrites it with demodulator 2574. Rust selected
  the corresponding raw clause 655 but formerly tried equivalent demodulator
  546 first, rewrote farther, and then discarded the result by positive-unit
  forward subsumption.
- Both demodulators have the same indexed left-side prefix. C stores leaf
  `ClausePos*` values in a `PTree` and traverses them by ascending address. The
  older rule 571 was inserted at `0x...93ca60`; the later rule 2574 reused
  `0x...93a8e0`, so C returns 2574 first. Rust previously kept the leaf payloads
  in insertion order and returned the older rule first.
- `ClausePosCell` uses the process-global `SizeMalloc` size-class free list, so
  C's ordering depends on allocator reuse shared with unrelated objects. It is
  observable search behavior but is not a semantic ranking rule.

## Resolution

Rust traverses same-leaf terminal occurrences in reverse insertion order. This
is a stable, local surrogate for the C allocator order observed on the canonical
reference. A regression contains both `LUSK6ext` rules and proves that rule 2574
wins. Exact raw-address emulation was rejected because it would couple safe Rust
search behavior to unrelated allocation traffic without providing a portable
contract.

The 50-case comparison at
`.artifacts/e-compare/20260712-173907-711516/` removes the `LUSK6ext.lop`
normalized-output mismatch and introduces no new mismatch. The suite falls from
seven mismatches to six.

The five-run native benchmark at
`.artifacts/e-compare/20260712-181629-341073-benchmark/` measures a `3.385x`
aggregate Rust/C wall-time ratio. `LUSK6ext.lop` measures `3.099x`, improved
from `3.211x` in the preceding baseline. Performance remains well outside the
required `1.10x`; `BOO020-1.p` is still excluded because outcomes differ.
