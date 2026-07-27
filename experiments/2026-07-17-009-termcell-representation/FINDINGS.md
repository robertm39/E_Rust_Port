# TermCell representation reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.135`. Rust's safe shared handle and
interior-mutability representation is the retained, measured port design; it
is not waiting to be replaced with C's raw flexible-array allocation. The
vendored C source remained unchanged.

## Representation contract

C allocates each `TermCell` and its flexible `args[]` tail as one raw object.
It uses raw pointer identity throughout term sharing, mutable pointer fields for
bindings/rewrite/index links, and external conventions for owned, borrowed,
shared, and variable-bank cells.

Rust uses `Rc<TermCell>` for stable identity and ownership, a separate checked
argument vector, ordinary `Cell` fields for scalar metadata, and one compact
`RefCell<TermLinks>` for C's five nullable binding/rewrite/type/tree pointers.
The permanent 64-bit layout regression now pins both `Term` and `Option<Term>`
to one pointer, so every populated or empty argument slot has the same width as
a C `Term_p`. It also retains the existing 152-byte `TermCell` and 48-byte
compact link-boundary assertions.

## Existing exact measurements

The representation's most recent causal experiment is
[`../2026-07-16-056-compact-term-links/FINDINGS.md`](../2026-07-16-056-compact-term-links/FINDINGS.md).
It compared an exact pristine baseline and the retained layout with identical
Windows and native-Linux workloads, plus upstream C:

- compact links removed exactly 32 bytes per Rust term node;
- the 20,000-owner unique corpus reduced useful live heap by 1,920,136 bytes
  and total live heap by 2.00%; and
- sustained `LUSK6`/`LUSK6ext` proof-search peak RSS fell 6.35%/7.14%, with
  unchanged successful outcomes and no wall-time regression.

A safe store-owned arena prototype documented in the term-tree review regressed
paired `LUSK6` CPU time by 1.31%, while a borrowed-slice argument prototype did
not improve end-to-end CPU time. The current safe representation therefore has
better evidence than the available replacement designs. Overall Rust/C memory
and runtime parity still require broader hot-path work, but that is not evidence
that raw C allocation semantics should become a Rust API.

## Scoped remaining LFHO work

C's `binding_cache` and `owner_bank` fields support cached applied-variable
dereferencing. Rust currently expands through explicit global or bank-local
paths without that cache. This is a distinct optimization/lifecycle task,
already tracked by `E_Rust_Port-j76.3.643` and
`E_Rust_Port-j76.4.1313`; closing the base representation decision does not
claim those behaviors are implemented.

## Validation

- the focused 64-bit layout regression covers handle, optional argument,
  compact-link, and `TermCell` sizes plus all five link accessors;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
