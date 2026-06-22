<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_eqnresolution

## Source Files

- [CLAUSES/ccl_eqnresolution.h](../../../eprover/CLAUSES/ccl_eqnresolution.h)
- [CLAUSES/ccl_eqnresolution.c](../../../eprover/CLAUSES/ccl_eqnresolution.c)

## Purpose

Routines for performing (ordered) equality resolution. the GNU Lesser General Public License. <1> Fri Jun 5 18:36:46 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_EQNRESOLUTION`

### Globals

- `extern bool EqResOnMaximalLiteralsOnly`

### Exported Functions

- `Clause_p ComputeEqRes(TB_p bank, ClausePos_p pos, VarBank_p freshvars, bool* subst_is_ho, PStack_p res_cls)`
- `Eqn_p ClausePosFirstEqResLiteral(Clause_p clause, ClausePos_p pos)`
- `Eqn_p ClausePosNextEqResLiteral(ClausePos_p pos)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `build_resolvent`: Actually builds eq resolvent
- `ComputeEqRes`: Given a clause and a position, try to perform equality resolution and return the resulting clause. If res_cls is NULL, then it assumes that you want to enumerate only single clause which is returned! Else, it returns NULL but fills res_cls with all clauses
- `ClausePosFirstEqResLiteral`: Find the first negative maximal literal in clause and return it.
- `ClausePosNextEqResLiteral`: Find the next negative maximal literal in clause and return it.

### Dependencies

- `"ccl_eqnresolution.h"`
- `<ccl_clausesets.h>`
- `<cte_ho_csu.h>`

### Compile-Time Conditions

- `CCL_EQNRESOLUTION`

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

Source files reviewed: `CLAUSES/ccl_eqnresolution.h`, `CLAUSES/ccl_eqnresolution.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 258 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Routines for performing (ordered) equality resolution. the GNU Lesser General Public License. <1> Fri Jun 5 18:36:46 MET DST 1998 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
