# PCL mini-protocol ownership and traversal audit

## Status

Completed for Bead `E_Rust_Port-j76.2.128`. Rust's owning indexed vector is an
evidence-backed replacement for C's raw-pointer `PDArray`; all externally
observable parsing, printing, proof-marking, and comment-forwarding behavior is
retained. The vendored C source remained unchanged.

## Storage and ownership

C allocates a one-slot `PDArray` with a fixed growth quantum of 500,000. The
first access to id 1 therefore enlarges it to 500,000 pointer slots, and even a
missing high-id lookup can enlarge storage because `PDArrayElementP` calls the
mutating element-reference helper. Steps are separately allocated raw pointers;
insert, extract, delete, and protocol destruction depend on pointer identity and
manual nulling.

Rust uses `Vec<Option<PclMiniStep>>`. It grows geometrically only for inserted
ids, returns from a missing lookup without mutation, owns every live step
exactly once, and drops it exactly once. Both representations give constant-time
indexed lookup. Rust retains C's `max_ident` high-water mark after extraction or
deletion, so iteration bounds and fast-marking seeds remain compatible.

C's duplicate false return is reachable only when the incoming pointer is the
exact stored pointer; a distinct same-id pointer asserts. Safe Rust cannot move
a still-owned step back into the protocol. Its meaningful equivalent is to
reject the id collision without replacing the stored value; protocol parsing
turns that rejection into the same duplicate-identifier diagnostic.

## Parsing, output, and traversal

Protocol parsing still begins only on `PosInt`. In both scanners that token
means a sequence of decimal digits, so zero is accepted while a negative id is
left unconsumed. Whole-protocol PCL/TSTP rendering visits ids in ascending order
and adds no record separators, while proof-clause rendering adds C's newline
after each selected step.

Comment forwarding uses an explicit output owner rather than `GlobalOut`.
Executable regression coverage verifies that
`epclextract --fast-extract --forward-comments` emits leading and trailing input
comments before the selected mini step.

C deduplicates preconditions in a pointer-keyed tree. Rust uses a `BTreeSet` of
mini ids, producing deterministic deduplicated ascending ids. Proof marking sets
the visited property before expansion and observes only the final property set
and whether any empty proof step was seen, so traversal order cannot affect the
result. Fast marking retains C's contiguous extract-marked suffix rule.

Potential syntax or policy cleanup remains in post-compatibility Beads
`E_Rust_Port-j76.4.949` through `.954` and `E_Rust_Port-j76.4.1133`.

## Validation

- focused mini-protocol tests cover the zero/negative scanner boundary, non-allocating misses,
  duplicate preservation, high-water-mark retention, ordered/deduplicated
  preconditions, parsing, output, property mutation, and fast/slow marking;
- executable coverage pins fast-mode comment forwarding at the output owner;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
