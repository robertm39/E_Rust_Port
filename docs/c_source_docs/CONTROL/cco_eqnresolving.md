<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_eqnresolving

## Source Files

- [CONTROL/cco_eqnresolving.h](../../../eprover/CONTROL/cco_eqnresolving.h)
- [CONTROL/cco_eqnresolving.c](../../../eprover/CONTROL/cco_eqnresolving.c)

## Purpose

Routines for the control of equality resolution. the GNU Lesser General Public License. <1> Mon Jun 8 17:10:03 MET DST 1998 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_EQNRESOLVING`

### Globals

- None found in the source scan.

### Exported Functions

- `long ClauseERNormalizeVar(TB_p bank, Clause_p clause, ClauseSet_p store, VarBank_p freshvars, bool strong)`
- `long ComputeAllEqnResolvents(TB_p bank, Clause_p clause, ClauseSet_p store, VarBank_p freshvars)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ComputeAllEqnResolvents`: Compute all equality resolvents of clause and put them into store. Returns number of clauses generated.
- `ClauseERNormalizeVar`: Perform destructive equality resolution inferences on negative pure variable literals (or, if strong, on negative literals with at least one variable side), insert the final result into store. Return the number of inferences.

### Dependencies

- `"cco_eqnresolving.h"`
- `<ccl_eqnresolution.h>`
- `<che_proofcontrol.h>`

### Compile-Time Conditions

- `CCO_EQNRESOLVING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_eqnresolving.h`, `CONTROL/cco_eqnresolving.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 233 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Routines for the control of equality resolution. the GNU Lesser General Public License. <1> Mon Jun 8 17:10:03 MET DST 1998 New
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- Rust now ports `ClauseERNormalizeVar` for the first-order destructive equality-resolution path used by generated-clause insertion: it scans negative pure-variable literals, optionally accepts one-variable-side literals for the strong mode, repeatedly computes one equality resolvent, replaces the original clause's literals, increments proof depth/size, and returns the inference count so the caller can requeue the mutated clause.
- Rust now ports the first-order `ComputeAllEqnResolvents` insertion wrapper: it skips clauses with no negative literals or `CPNoGeneration`, iterates candidate negative literals through the explicit maximal-literal filter, inserts generated resolvents into a caller-owned clause set, copies proof depth/size, TPTP type, and SOS state, and returns the generated count.
- First-order derivation pushes are ported for `DCEqRes` on generated resolvents and `DCDesEqRes` on destructive equality-resolution replacements.
- Higher-order CSU enumeration, `subst_is_ho` propagation, lambda normalization, and proof-documentation output remain pending.

### Change-Later Observations

- C `ClauseERNormalizeVar` mutates the original clause and inserts that same pointer into the supplied store only if at least one inference fired. Rust returns the owned mutated clause plus count so the proof-control caller can reinsert it into `tmp_store`; revisit the API once proof-state clause ownership has stable in-set handles.
- The C routine increments proof depth and proof size on the mutated clause for every destructive equality-resolution step but accounts generated/resolution statistics in callers such as `insert_new_clauses`. Rust keeps that split between the clause helper and proof-control statistics.
- `DCDesEqRes` carries no parent pointer in C because the modified clause is its own implicit premise. Rust records only the operation entry for each destructive step; revisit if proof-output reconstruction later needs an explicit self-reference.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
