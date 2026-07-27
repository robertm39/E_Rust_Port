# Full PCL-protocol ownership and traversal audit

## Status

Completed for Bead `E_Rust_Port-j76.2.131`. Rust's owning sorted protocol
storage preserves the observable C lookup, ordering, parsing, and proof-marking
contracts while removing stale raw-pointer caches and null-parent traversal.
The vendored C source remained unchanged.

## C behavior

`PCLProtCell` owns a `PTree` of step pointers and a second `PStack` cache of the
same pointers in identifier order. Insertions mark the cache stale and increment
`number` before tree insertion reports a duplicate. Protocol parsing treats a
duplicate as fatal immediately, so the pre-error count cannot be observed by a
continuing successful C caller.

Comment-preserving parsing writes token comments through `GlobalOut`.
Precondition collection resolves quoted identifiers to step pointers, stores
them in a generic pointer-ordered tree without a null check, and proof marking
extracts that tree in pointer order. FOF stripping replaces dependent
justifications with `initial` but does not set `PCLIsInitial`.

## Rust equivalence

`PclProtocol` owns one `Vec<PclStep>` in C-comparator identifier order. Binary
search implements find and duplicate detection, common monotonically increasing
protocol identifiers append without shifting, and ordered rendering needs no
second pointer cache. Extraction transfers the owned step. Rust's borrow rules
prevent step references from surviving vector mutation, while the boxed clause
arm independently preserves C's `Clause_p` address stability.

Duplicate rejection keeps `step_count()` equal to stored membership. A focused
parse regression confirms that the first of two equal identifiers remains the
only member after the syntax diagnostic.

`parse_with_output` forwards and clears comments through an explicit writer;
`epclextract` selects it when comment forwarding is enabled. Precondition
collection reports dangling references as syntax diagnostics, deduplicates
parents, and returns them in deterministic C-comparator PCL-id order. Current
consumers use those parents only for property queries/marking, so replacing C's
allocator-dependent pointer order has no textual or logical effect.

FOF stripping retains C's unusual split exactly: a dependent clause receives
an `initial` justification, but its `PCLIsInitial` property stays clear.

## Performance

The common increasing-id parse path performs logarithmic duplicate lookup plus
amortized constant-time vector append, comparable to C tree insertion while
avoiding one tree node per step and a later pointer-stack serialization pass.
Arbitrary out-of-order insertion can shift vector elements, but moves only the
owned step headers; clause payloads remain boxed and terms remain shared. The
existing 1,010-step executable protocol coverage and full repository suite do
not expose a throughput regression.

## Validation

- 10 focused `pcl2::protocol` tests cover ordering, comment forwarding,
  duplicates/counts, ownership transfer, proof marking, dangling parents,
  unique parent order, quoted arguments, FOF stripping, and bulk output;
- source inspection confirms `epclextract`'s explicit comment-output routing;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
