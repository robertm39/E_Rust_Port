# Frequency-vector and packed-clause ownership audit

## Status

Completed for Bead `E_Rust_Port-j76.2.117`. Stable raw clause addresses are not
needed to port the `ccl_freqvectors` contract: Rust separates the C struct's
borrowed-vector and ownership-transfer roles, and clause-set/FV-index consumers
already maintain safe owned values and snapshots. The vendored C source remains
unchanged.

## C ownership contract

`FreqVector_p` and `FVPackedClause_p` are aliases for the same
`FreqVectorCell`. Its `clause` member is documented as an unprotected reference,
but ownership depends on the function used:

- `FreqVectorFreeReal` frees only the coordinate array and cell;
- `FVPackedClauseFreeReal` first frees `pack->clause`, then frees the vector;
- `FVUnpackClause` returns the clause and frees only the shell; and
- `FreqVectorPrint` dereferences the borrowed clause when it is non-null.

Thus the layout does not describe its own destruction rule. A caller using the
wrong typedef/free macro can leak the clause, free a borrowed clause, or retain
a dangling print pointer.

## Rust ownership decision

`FreqVector` owns only its contiguous coordinate vector and an optional numeric
clause-identity snapshot. The snapshot supports non-compatibility debug text but
is never dereferenced. Exact `FreqVectorPrint` rendering instead receives an
optional live `&Clause` plus explicit output/problem settings.

`FvPackedClause` is a separate, non-`Clone` type that always owns one `Clause`
and an optional vector. Normal drop matches `FVPackedClauseFreeReal`; consuming
`into_clause`/`fv_unpack_clause` matches `FVUnpackClause`. Focused coverage
mutates the clause through the packed owner and observes the mutation after
unpacking. A second regression drops the source clause and then safely reads the
vector's identity snapshot and no-clause rendering.

FV-index leaves keep an independent clause snapshot because the sparse
`ClauseSet` owner may relocate its values. Indexed insertion sorts the packed
owner before taking that snapshot, inserts the moved original into the set, and
indexed extraction deletes the corresponding snapshot before returning the
original. This retains C's effective immutable-while-indexed behavior without
raw-address liveness.

## Performance

The packed wrapper and `FreqVector` each retain one contiguous vector allocation
where C uses a separately allocated `long` array. Clause moves are constant-time
Rust value transfers and term handles remain shared. This audit also removes an
unnecessary clone of the full frequency vector from every
`FvIndexAnchor::insert`: a disjoint-field borrow now reads the vector while
sorting the owned clause. The clause snapshot clone remains intentional because
it replaces the unsafe FV-leaf pointer.

C's static reusable `FVCollectFreqVectorCompute` scratch array versus Rust's
call-local allocation is separate performance work already tracked by
`E_Rust_Port-j76.3.431`. FV-index storage/output fidelity and raw leaf identity
remain tracked by `E_Rust_Port-j76.2.116`, `E_Rust_Port-j76.4.202`, and
`E_Rust_Port-j76.3.436`.

## Validation

- ten focused frequency-vector tests pass, including the two ownership/lifetime
  regressions;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
