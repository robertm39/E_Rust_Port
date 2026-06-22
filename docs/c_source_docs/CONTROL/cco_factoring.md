<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_factoring

## Source Files

- [CONTROL/cco_factoring.h](../../../eprover/CONTROL/cco_factoring.h)
- [CONTROL/cco_factoring.c](../../../eprover/CONTROL/cco_factoring.c)

## Purpose

Routines for the control of factoring the GNU Lesser General Public License. <1> Mon Jun 8 17:10:03 MET DST 1998 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_FACTORING`

### Globals

- None found in the source scan.

### Exported Functions

- `long ComputeAllEqualityFactors(TB_p bank, OCB_p ocb, Clause_p clause, ClauseSet_p store, VarBank_p freshvars)`
- `long ComputeAllOrderedFactors(TB_p bank, OCB_p ocb, Clause_p clause, ClauseSet_p store, VarBank_p freshvars)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ComputeAllOrderedFactors`: Compute all ordered factors of clause and put them into store. Return number of factors.
- `ComputeAllEqualityFactors`: Compute all equality factors of clause and put them into store. Return number of factors.

### Dependencies

- `"cco_factoring.h"`
- `<ccl_factor.h>`
- `<che_proofcontrol.h>`

### Compile-Time Conditions

- `CCO_FACTORING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_factoring.h`, `CONTROL/cco_factoring.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 224 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Routines for the control of factoring the GNU Lesser General Public License. <1> Mon Jun 8 17:10:03 MET DST 1998 New
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
