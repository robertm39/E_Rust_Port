# Sledgehammer Pointer-Order Trace

## Question

Which stable C fields explain the remaining selected-sort and quantified-variable
ordering differences in `LFHOL/sledgehammer.p`?

## Setup

- Rust baseline commit: `3e4fb0a6` (`Clean up cross-platform Rust checks`).
- C reference executable:
  `/home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/ho/eprover-ho`.
- Input: `eprover/EXAMPLE_PROBLEMS/LFHOL/sledgehammer.p`.
- GDB probes: `inspect_sort_tree.gdb` and `inspect_proof_variables.gdb` in
  this directory.
- Targeted five-case LFHOL comparison after the retained sort change:
  `.artifacts/e-compare/20260711-215944-036826/`.

The probes were run under Ubuntu WSL from the repository root:

```bash
gdb -q -batch \
  -x experiments/2026-07-11-005-sledgehammer-pointer-order/inspect_sort_tree.gdb \
  --args /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/ho/eprover-ho \
  --auto --silent --proof-object=1 \
  eprover/EXAMPLE_PROBLEMS/LFHOL/sledgehammer.p

gdb -q -batch \
  -x experiments/2026-07-11-005-sledgehammer-pointer-order/inspect_proof_variables.gdb \
  --args /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/ho/eprover-ho \
  --auto --silent --proof-object=1 \
  eprover/EXAMPLE_PROBLEMS/LFHOL/sledgehammer.p
```

## Selected Sorts

`TypeBankPrintSelectedSortDefs` traverses a pointer-keyed `PTree`. The observed
in-order nodes had this `(f_code, type_uid)` sequence:

```text
(11, 10), (9, 11), (10, 13), (13, 14), (7, 17), (12, 24)
```

Although the immediate C mechanism is pointer ordering, its output for this run
exactly follows ascending type UID and does not follow function code or source
declaration order. Rust now orders selected user sorts by `type_uid`. The
targeted LFHOL rerun makes every selected-sort declaration byte-equivalent to C;
the other four cases remain exact.

## Quantified Variables

The `PTreeToPStack` probe captured the six variables used to close C proof step
`c_0_27`. Root/right/left traversal produced:

```text
X6, X7, X2, X5, X4, X3
```

The output routine reverses that stack to `X3,X4,X5,X2,X7,X6`. Raw pointer
order places `X3 < X2 < X4 < X5 < X6 < X7`; the `X2`/`X3` inversion is not
explained by f-code, entry number, type, or first occurrence. Rebuilding Rust
without a semantic change also changed its same-sort binder permutations,
confirming that the current Rust identity-address ordering is allocator-sensitive
too.

## Falsification Checks

- Sorting selected sorts by function code fails the traced C sequence.
- Sorting variables by f-code, entry number, first occurrence, or type does not
  reproduce all ten differing binder lines.
- The post-change LFHOL report has one mismatch, `sledgehammer.p`, and that
  mismatch contains only ten same-sort quantified-variable permutations.
- The selected-sort unit test defines sorts in one order and interns them in the
  opposite order, so it distinguishes type UID from constructor-code ordering.

## Conclusion

Type UID is a stable compatibility key for the observed selected-sort output and
is retained in Rust. The residual binder order is a genuine allocator/freelist
artifact of C's pointer tree. No problem-specific variable-order heuristic was
added; exact emulation would require allocator-compatible identities or output
canonicalization.

## Limits

- The type-UID conclusion is traced for the reference build and workload above;
  C still obtains the order indirectly from addresses and can vary with allocator
  history.
- The variable trace explains the residual output but does not make it stable.
- The comparison harness intentionally reports binder permutations rather than
  canonicalizing them, so the remaining textual mismatch stays visible.
