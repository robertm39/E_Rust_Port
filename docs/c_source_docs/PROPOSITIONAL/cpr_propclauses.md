<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROPOSITIONAL / cpr_propclauses

## Source Files

- [PROPOSITIONAL/cpr_propclauses.h](../../../eprover/PROPOSITIONAL/cpr_propclauses.h)
- [PROPOSITIONAL/cpr_propclauses.c](../../../eprover/PROPOSITIONAL/cpr_propclauses.c)

## Purpose

Datatypes for the efficient representation of propositional clauses for a DPLL procedure. the GNU Lesser General Public License. <1> Wed Apr 23 12:10:35 CEST 2003

Within the source tree, this unit belongs to `PROPOSITIONAL`. Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DPLLClauseCell`
- `DPLLClause_p`
- `DPLLOutputFormat`

### Macros And Constants

- `CPR_PROPCLAUSES`
- `DPLLClauseCellAlloc()`
- `DPLLClauseCellFree(junk)`
- `DPLLClauseIsUnit(clause)`

### Globals

- None found in the source scan.

### Exported Functions

- `DPLLClause_p DPLLClauseFromClause(PropSig_p psig, Clause_p clause)`
- `bool DPLLClauseNormalize(DPLLClause_p clause)`
- `void DPLLClauseFree(DPLLClause_p junk)`
- `void DPLLClausePrintDimacs(FILE* out, DPLLClause_p clause)`
- `void DPLLClausePrintLOP(FILE* out, PropSig_p psig, DPLLClause_p clause)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `p_atom_compare`: Compare the two propositional atoms pointed to. Atoms are compared by alsolute value, then by sign (smaller is smaller, positive is smaller).
- `DPLLClauseFree`: Free a DPLLClause.
- `DPLLClauseFromClause`: Convert a propositional (not ground!) E clause into a DPLL clause. No simplification or checking is done (except for the fact that the clause is indeed propositional)!
- `DPLLClauseNormalize`: Destructively normalize a clause: Literals are sorted by atom encoding (positive comes before negative if both exist). Doubly occuring literals are removed. Return value is true if clause is tautological, false otherwise. Does not reduce size of literal array, as I don't expect much reduction in the number of atoms here.
- `DPLLClausePrintLOP`: Print a propositional clause in LOP format.
- `DPLLClausePrintDimacs`: Print a DPLL clause in DIMACS format (note that DIMACS input files require a header, individual clauses are not a complete syntactic element! Also not that most provers reading DIMACS require the sequence "0\n" as an end of clause marker and that most cannot deal with the empty clause. I'm printing the empty clause nonetheless, since this output is unlikely...

### Dependencies

- `"cpr_propclauses.h"`
- `<ccl_clauses.h>`
- `<cpr_propsig.h>`

### Compile-Time Conditions

- `CPR_PROPCLAUSES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROPOSITIONAL/cpr_propclauses.h`, `PROPOSITIONAL/cpr_propclauses.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PROPOSITIONAL` covering 2 source file(s), about 364 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Bridge between first-order clauses and propositional clauses; ownership and mapping choices affect SAT integration.
- Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
