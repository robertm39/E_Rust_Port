# VarBank TypeBank ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.2.138`. Retaining the shared default `Type`
handle is an evidence-backed compatibility decision, not a remaining parser
construction-order gap. The vendored C source remained unchanged.

## C ownership surface

`VarBankAlloc(TypeBank_p)` stores the whole pointer in `sort_table`. Within
`cte_termvars.c`, ordinary behavior dereferences that pointer in two places:

- `VarBankExtNameAssertAlloc` passes `sort_table->default_type` to fresh-variable
  allocation; and
- the verbose typed-name path passes the bank to `TypePrintTSTP` for rendering.

Every typed allocation path receives its `Type_p` explicitly. The C TypeBank
sets `default_type` to its shared `$i` type during allocation and does not mutate
that field later.

## Rust equivalence

Rust `VarBank` retains the shared default `Type` handle captured at construction.
That handle preserves the same type identity without introducing a self-
referential mutable ownership edge between `TermBank`, `Signature`, `TypeBank`,
and `VarBank`. Typed variables still receive a shared `Type` explicitly, and
the variable stacks/counters are dynamic maps keyed by type UID rather than
arrays whose capacity depends on construction time.

The new regression constructs a VarBank, then defines and shares a user sort in
the original TypeBank. The bank allocates a typed variable in that late sort,
creates its UID-keyed stack, and still allocates an untyped external name with
the identical retained `$i` handle. Parser callers therefore do not need to
delay VarBank construction until all user sorts are known.

## Validation

- focused `terms::termvars` tests cover the late-sort/default-sort decision;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
