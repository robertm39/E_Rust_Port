# Simple-type identity and ordering audit

## Status

Completed for Bead `E_Rust_Port-j76.2.134`. Rust's safe shared `Type` handle is
the evidence-backed replacement for raw `Type_p`; it preserves the identity and
ordering semantics C actually exposes. The vendored C source remained
unchanged.

## C behavior

C allocates `TypeCell` and its argument-pointer array separately. Shared types
are retained by a `TypeBank`, while temporary unshared copies are explicitly
freed. Identity-sensitive operations compare `Type_p` values directly.

`TypesCmp` compares constructor code, arity, and then corresponding argument
addresses through `PCmp`. Its source contains an explicit note that this is a
source of clause-sorting differences. The numeric ordering and reuse of heap
addresses therefore cannot be a stable cross-run, cross-allocator, or
cross-build output contract; only comparison against the live allocations in
the current process is meaningful.

## Rust equivalence

Rust `Type` is an `Rc<TypeCell>`. `PartialEq` uses `Rc::ptr_eq`, and
`type_identity_cmp`/`types_cmp` compare `Rc::as_ptr` addresses. Reference
counting does not insert a semantic ID or structural comparator into these
paths. Shared TypeBank values keep the same allocation live, making identity
and ordering stable over the same ownership interval as the C bank.

The strengthened regressions establish that:

- `Type` and `Option<Type>` are one pointer wide on 64-bit targets;
- `type_identity_cmp` has exactly the sign obtained by comparing the two live
  `Rc` allocation addresses; and
- the arrow-argument tie-break in `types_cmp` returns that identity comparison,
  after constructor and arity agree.

Allocator reuse may choose different numeric addresses after temporary values
are freed, but C has the same allocator-dependent behavior and explicitly does
not promise a reproducible order. Rust's safe lifetime prevents use-after-free
without weakening any live-pointer comparison.

## Validation

- focused `terms::simpletypes` tests cover layout, identity, and comparator
  behavior;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
