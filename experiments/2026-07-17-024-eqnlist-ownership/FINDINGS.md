# Equation-list parser and stack ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.113`. The list-specific parser and
pointer-stack ownership gaps are resolved without adding intrusive literal
links. The vendored C source remains unchanged.

## Parser boundary

C `EqnListParse` performs only list-level token control: it recognizes the
format-specific literal start, returns an empty list without consuming a
non-start token, and otherwise delegates each literal to `EqnParse` while
consuming the caller's separator. Rust has the same control flow and its
`eqn_parse` delegates literal sides to `TermBank::parse_term_with_distinct_checks`,
the port's banked `TBTermParse`-equivalent path.

The new list-level regression parses four signed TPTP-prefix equations across
semicolon separators using integer, rational, float, and object terms. It
checks the four signs and canonical signature names, including E's normalized
`1.500000` float spelling. The no-literal-start/no-consumption branch remains
covered by the existing empty-list regression.

Any residual literal-level `EqnParse` or formula-position TSTP work remains
tracked by Bead `E_Rust_Port-j76.2.111`; the equation-list layer no longer adds
a separate simple-term parser or syntax restriction.

## No-copy stack ownership

C list nodes are independent heap cells connected by `next` pointers.
`EqnListToStack` and `EqnListSplitToStacks` therefore push borrowed cell
pointers, whereas `EqnListFromStack` consumes a stack and relinks the same
cells.

Rust previously returned `PStack<Eqn>` from the two non-owning helpers by
cloning every literal cell. The safe API now separates the C roles:

- `to_stack` and `split_to_stacks` return lifetime-bound `PStack<&Eqn>` views;
- `into_stack` consumes an `EqnList` and moves each owned literal into the
  stack; and
- `from_stack` consumes that owned stack and restores list order.

Pointer-equality regressions prove that borrowed stack entries reference the
literal cells inside the original list. Rust's borrow checker prevents list
mutation while those views exist, replacing C's raw-pointer lifetime
precondition without changing allocation or copying behavior.

## Validation

- all 20 focused `eqnlist` tests pass;
- all 4,228 library tests plus every integration and binary target test pass;
- formatting and strict Clippy pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
