# PCL expression ownership and rendering audit

## Status

Completed for Bead `E_Rust_Port-j76.2.125`. Rust's typed recursive expression
tree is an evidence-backed replacement for C's untagged pointer/integer
`PDArray` representation. Parsing, ownership, arities, PCL/TSTP output, and
legacy edge behavior are covered without changing the vendored C source.

## Ownership

C allocates every expression cell separately and immediately allocates a
two-word `PDArray`. Each logical argument consumes adjacent child and optional
position slots. Quote slot zero is either an owned full `PCLId` pointer or a
mini identifier stored inline through the integer union member; initial slot
zero owns `ClauseInfo`; compound slots own recursive expressions. Correct
destruction therefore depends on both the opcode and the caller choosing
`PCLExprFree` versus `PCLMiniExprFree`.

Rust makes those alternatives explicit in `PclExpressionData` and `PclQuote`.
Compound children remain boxed and separately allocated like C children;
optional positions and full identifiers are owned values, while mini ids are
plain `i64`. Enum-guided drop cannot apply the full-id destructor to an inline
mini id, leak a position, or recursively free the wrong active slot. Root
expressions can also live directly inside their owning step instead of
requiring a separate allocation.

## Parsing and output

All opcode discriminants from `PCLOpNoOp == 0` through `PCLOpMaxOp == 30` are
pinned. A table-driven regression covers all 26 operator names accepted by the
C parser, their exact fixed or one-or-more arity, PCL round trip, and exact TSTP
rendering. Separate tests cover source-backed and bare `initial`, full and mini
quotes, wrong arity, and the prefix behavior of `PCLStepExtract` (including the
C-shaped acceptance of `proofless`).

The four existing compatibility quirks remain deliberate:

- both quote and compound position guards look for `(` before calling a
  position parser that expects a decimal integer, so the opening token is
  rejected in place;
- manually stored positions are concatenated in PCL output but omitted from
  TSTP output;
- `PCLOpURewrite` retains its discriminant and lemma weight but has no parser or
  printer spelling in this unit; and
- `cdclpropres` and `ar` require at least one argument.

Those possible post-compatibility changes remain tracked by Beads
`E_Rust_Port-j76.4.931` through `.934` and `E_Rust_Port-j76.3.43`.

## Allocation and performance

C creates its argument `PDArray` with initial size two and fixed growth two.
Because each argument occupies two slots, every argument after the first
allocates a replacement array and copies all earlier slots. This is linear
capacity growth and quadratic cumulative copying for a wide variable-arity
expression. It also allocates the two slots for leaf and zero-argument nodes.

Rust's empty vector allocates nothing, then grows geometrically while retaining
contiguous argument/position pairs as typed structs. A regression parses and
round-trips a 2,048-parent `ar(...)` expression. The child allocation count and
serialized order remain equivalent to C, while argument-array growth is
strictly better asymptotically and does not need null slot sentinels.

## Validation

- focused expression tests cover 15 cases, including exhaustive opcode and
  operator/rendering tables, large variable arity, both stored-position output
  sites, and both position-parser mismatch sites;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
