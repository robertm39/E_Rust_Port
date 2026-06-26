<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_diseq_decomp

## Source Files

- [CONTROL/cco_diseq_decomp.h](../../../eprover/CONTROL/cco_diseq_decomp.h)
- [CONTROL/cco_diseq_decomp.c](../../../eprover/CONTROL/cco_diseq_decomp.c)

## Purpose

Code to control the computation of disequality decomposition. The disequality decomposition inference is f(s1,...,sn)!=f(t1,...,tn) | R s1!=t1 | ... | sn_tn | R

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_DISEQ_DECOMP`

### Globals

- None found in the source scan.

### Exported Functions

- `long ComputeDisEqDecompositions(TB_p terms, Clause_p clause, ClauseSet_p store, long diseq_decomposition, long diseq_decomp_maxarity)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ComputeDisEqDecompositions`: Compute all disequality decompositions compatiple with the parameters, and add them to store. Returns the number of clauses created.

### Dependencies

- `"cco_diseq_decomp.h"`
- `<ccl_clausesets.h>`
- `<ccl_diseq_decomp.h>`

### Compile-Time Conditions

- `CCO_DISEQ_DECOMP`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_diseq_decomp.h`, `CONTROL/cco_diseq_decomp.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 154 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Code to control the computation of disequality decomposition. The disequality decomposition inference is f(s1,...,sn)!=f(t1,...,tn) | R s1!=t1 | ... | sn_tn | R
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/clauses/diseq_decomp.rs` ports `ComputeDisEqDecompositions`: it gates on `ClauseLiteralNumber(clause) <= diseq_decomposition`, scans compact literal positions in C order, checks for negative equational literals with matching non-null top symbols and an arity no larger than `diseq_decomp_maxarity`, inserts generated clauses into the caller-owned `ClauseSet`, and returns the number generated.
- The wrapper preserves C's metadata boundary: derivation metadata is attached by the lower-level `ClauseDisEqDecomposition` builder, not copied or pushed in `ComputeDisEqDecompositions`.

### Change-Later Observations

- The size threshold is an enable-up-to limit, not a minimum: C decomposes only when the current clause has at most `diseq_decomposition` literals. Rust mirrors that exact comparison.
- The control wrapper does not copy parent proof depth, proof size, TPTP type, or SOS status onto generated decompositions; it relies on the lower-level builder for derivation metadata. Preserve this until proof/reference tests show whether generated metadata should be normalized across inference wrappers.
- The C wrapper checks only left-term arity after confirming equal top symbols; the lower-level builder asserts equal arity. Rust mirrors the wrapper gate and leaves the arity equality invariant to `clause_dis_eq_decomposition`.
<!-- END MANUAL REVIEW: c_source_docs -->
