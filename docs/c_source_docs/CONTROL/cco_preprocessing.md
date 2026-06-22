<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_preprocessing

## Source Files

- [CONTROL/cco_preprocessing.h](../../../eprover/CONTROL/cco_preprocessing.h)
- [CONTROL/cco_preprocessing.c](../../../eprover/CONTROL/cco_preprocessing.c)

## Purpose

This module encapsulates some of the main proofstate preprocessing, mostly to keep the complexity of eprover.c under control.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_PREPROCESSING`

### Globals

- None found in the source scan.

### Exported Functions

- `long ProofStateClausalPreproc(ProofState_p proofstate, HeuristicParms_p h_parms)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ProofStateClausalPreproc`: Perform various (optional) preprocessing steps on the proof state unprocessed clauses.

### Dependencies

- `"cco_preprocessing.h"`
- `<ccl_bce.h>`
- `<ccl_gd_transformation.h>`
- `<ccl_pred_elim.h>`
- `<ccl_proofstate.h>`
- `<cco_ho_inferences.h>`
- `<che_hcb.h>`

### Compile-Time Conditions

- `CCO_PREPROCESSING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_preprocessing.h`, `CONTROL/cco_preprocessing.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 184 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Preprocessing pipeline. Step ordering changes can alter completeness, clause IDs, and proof output.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
