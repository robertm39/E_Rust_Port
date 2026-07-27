<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROPOSITIONAL / cpr_dpll

## Source Files

- [PROPOSITIONAL/cpr_dpll.h](../../../eprover/PROPOSITIONAL/cpr_dpll.h)
- [PROPOSITIONAL/cpr_dpll.c](../../../eprover/PROPOSITIONAL/cpr_dpll.c)

## Purpose

Definitions for the main DPLL algorithm. the GNU Lesser General Public License. <1> Tue May 6 02:04:46 CEST 2003 New

Within the source tree, this unit belongs to `PROPOSITIONAL`. Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DPLLStateCell`
- `DPLLState_p`

### Macros And Constants

- `CPR_DPLL`
- `DPLLStateCellAlloc()`
- `DPLLStateCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `DPLLState_p DPLLStateAlloc(DPLLFormula_p form)`
- `bool DPLLAssignVar(DPLLState_p state, PLiteralCode assignment)`
- `void DPLLRetractLastAss(DPLLState_p state)`
- `void DPLLStateFree(DPLLState_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `deactivate_clauses`: Deactivete all clauses in *tree and record them on state->decativated. Return number of clauses.
- `shorten_clauses`: Shorten all clauses in *tree by one.
- `DPLLStateAlloc`: Allocate an initialized DPLL search state.
- `DPLLStateFree`: Free a DPLL search state
- `DPLLAssignVar`: Extend the assignment with the given new propositional variable assignment. Return true if no empty clause has been generated.

### Dependencies

- `"cpr_dpll.h"`
- `<cpr_dpllformula.h>`
- `<cpr_varset.h>`

### Compile-Time Conditions

- `CPR_DPLL`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; reconciled with the complete reference executable call path on 2026-07-17.

Source files reviewed: `PROPOSITIONAL/cpr_dpll.h`, `PROPOSITIONAL/cpr_dpll.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PROPOSITIONAL` covering 2 source file(s), about 268 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- DPLL solver state machine. Assignment, propagation, and backtracking behavior should be treated as algorithmic reference.
- Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/propositional/dpll.rs` ports the complete implemented `cpr_dpll.c` behavior over the Rust `DpllFormula`, `DpllClause`, and `AtomSet` ports: `DPLLStateAlloc`, state release through Rust ownership, and the current `DPLLAssignVar` stub result. The reference `edpll` executable allocates and immediately frees this state without calling assignment or a solve loop, so a standalone SAT/UNSAT result is not missing drop-in behavior.
- Rust stores assignments as signed literal codes, deactivation subset markers as `None` entries in a typed vector, unprocessed unit clauses as formula clause indices, and open atoms in the ported `AtomSet`. Open-atom insertion follows C's increasing atom scan plus front insertion, so iteration observes the same reversed order.
- `DPLLAssignVar` pushes the assignment and a deactivation marker, then calls Rust equivalents of the currently stubbed C helpers. Since C `shorten_clauses` returns zero, the ported assignment currently returns `false` for allocated atoms, matching the observable implemented C body rather than a complete solver.

### Change Later

- `deactivate_clauses` and `shorten_clauses` are empty C stubs, so `DPLLAssignVar` cannot perform real propagation and reports failure after every allocated assignment. Rust preserves this for drop-in compatibility. A real DPLL algorithm would be a post-compatibility extension and should use a new mode or executable with its own behavioral tests.
- `DPLLRetractLastAss` is declared in `cpr_dpll.h` but has no definition in `cpr_dpll.c`. Rust deliberately does not invent a callable retraction body. Any future solver extension must define retraction together with propagation, conflict handling, branching, and result rendering rather than presenting it as a missing port of existing C behavior.
- The negative-assignment branch in `DPLLAssignVar` still deactivates positive clauses and shortens negative clauses after negating the assignment, the same branches used for positive assignments. With the helper stubs this has no effect today, but real propagation should verify whether negative assignments should swap those sets.
<!-- END MANUAL REVIEW: c_source_docs -->
