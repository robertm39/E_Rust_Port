<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_dbvars

## Source Files

- [TERMS/cte_dbvars.h](../../../eprover/TERMS/cte_dbvars.h)
- [TERMS/cte_dbvars.c](../../../eprover/TERMS/cte_dbvars.c)

## Purpose

Functions for the management of shared De Bruijn variables. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DBVarBank_p`

### Macros And Constants

- `CTE_DBVARS`
- `DBVarBankAlloc()`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p _RequestDBVar(DBVarBank_p db_bank, Type_p type, long db_index)`
- `void DBVarBankFree(DBVarBank_p db_bank)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `var_bank_var_alloc`: Return a pointer to the newly created variable with the given f_code and sort in the variable bank.
- `_RequestDBVar`: Create (or find) a unique, shared term that represents a DB variable with the given type and index. Function always returns the same results given the same arguments.
- `DBVarBankFree`: Release all memory used by de Bruijn variable bank.

### Dependencies

- `"cte_dbvars.h"`
- `<clb_intmap.h>`
- `<clb_objtrees.h>`
- `<cte_termtypes.h>`

### Compile-Time Conditions

- `CTE_DBVARS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_dbvars.h`, `TERMS/cte_dbvars.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 208 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Functions for the management of shared De Bruijn variables. the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
