# Term free-boundary ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.2.137`. Rust's reference-counted `Term`
drop behavior is the evidence-backed replacement for C `TermFree` and
`TermTopFree`; an explicit manual-free surface is neither required nor
desirable. The vendored C source remained unchanged.

## C ownership boundaries

`TermFree` accepts an owned unshared tree. It recursively frees every
non-variable argument and its top cell, asserts that each freed non-variable
cell is not term-bank shared, and treats every free or De Bruijn variable as a
no-op because variable banks own those cells.

`TermTopFree` frees only the supplied cell and its flexible argument array. Its
callers use it for temporary top copies whose argument pointers already belong
to a term bank or remain reachable through another owner, including duplicate
term-bank insertion, unchanged lambda traversals, representation lookup, and
inside-out replacement reconstruction.

## Rust equivalence

Each Rust `Term` is an `Rc<TermCell>`, and argument slots retain `Term` handles.
Dropping the final root handle therefore releases unretained non-variable
descendants recursively. Variable-bank maps and stacks retain their variable
handles, so disposing an unshared tree releases only the tree's references to
those variables. Likewise, dropping a temporary top handle releases its own
argument references, while children retained by a bank or caller remain live.

This is the ownership outcome C obtains from its two manual functions without
exposing use-after-free, double-free, or shared-term-free operations. Rust also
handles an accidentally shared unshared DAG through reference counts rather
than making C's tree-only `TermFree` precondition part of a public unsafe
contract.

## Regression evidence

The new lifetime tests use weak handles to observe the allocation boundaries:

- dropping a top wrapper destroys that wrapper but leaves a separately retained
  child live until its owner is dropped; and
- dropping an unshared root destroys its unretained non-variable descendant,
  while its variable remains live until the VarBank is dropped.

## Validation

- focused drop-boundary tests pass;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
