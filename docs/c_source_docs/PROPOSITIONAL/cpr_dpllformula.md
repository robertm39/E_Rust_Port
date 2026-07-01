<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROPOSITIONAL / cpr_dpllformula

## Source Files

- [PROPOSITIONAL/cpr_dpllformula.h](../../../eprover/PROPOSITIONAL/cpr_dpllformula.h)
- [PROPOSITIONAL/cpr_dpllformula.c](../../../eprover/PROPOSITIONAL/cpr_dpllformula.c)

## Purpose

Base data structure for representing the state of a propositional formula (in CNF) for a DPLL procedure. I'm doing this for the first time, so it probably is sub-perfect.... the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROPOSITIONAL`. Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AtomCell`
- `Atom_p`
- `DPLLFormulaCell`
- `DPLLFormula_p`

### Macros And Constants

- `ATOM_GROWTH_FACTOR`
- `CPR_DPLLFORMULA`
- `DEFAULT_ATOM_NUMBER`
- `DPLLFormulaCellAlloc()`
- `DPLLFormulaCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `DPLLFormula_p DPLLFormulaAlloc(void)`
- `void DPLLFormulaFree(DPLLFormula_p junk)`
- `void DPLLFormulaInsertClause(DPLLFormula_p form, DPLLClause_p clause)`
- `void DPLLFormulaParseLOP(Scanner_p in, Sig_p sig, DPLLFormula_p form)`
- `void DPLLFormulaPrint(FILE* out,DPLLFormula_p form, DPLLOutputFormat format, bool print_atoms)`
- `void DPLLRegisterClauseLiteral(DPLLFormula_p form, DPLLClause_p clause, PLiteralCode lit)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `dpll_form_add_atom_space`: Create more space for atoms in an DPLLFormula.
- `DPLLFormulaAlloc`: Allocate an empty and initialized DPLLFormula().
- `DPLLFormulaFree`: Free all of the DPLLFormula().
- `DPLLFormulaPrint`: Print a DPLLFormula.
- `DPLLRegisterClauseLiteral`: Register a clause at an atom.
- `DPLLFormulaInsertClause`: Insert a new clause into a formula. The clause is expected to be non-tautological and contain no redundant literals. Moreover, for the sake of printing LOP, the atoms should be registered in form->sig.
- `DPLLFormulaParseLOP`: Parse a set of LOP clauses into a DPLLFormula.

### Dependencies

- `"cpr_dpllformula.h"`
- `<cpr_propclauses.h>`

### Compile-Time Conditions

- `CPR_DPLLFORMULA`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROPOSITIONAL/cpr_dpllformula.h`, `PROPOSITIONAL/cpr_dpllformula.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PROPOSITIONAL` covering 2 source file(s), about 382 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Base data structure for representing the state of a propositional formula (in CNF) for a DPLL procedure. I'm doing this for the first time, so it probably is sub-perfect.... the GNU Lesser General Public License.
- Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/propositional/dpllformula.rs` ports `DPLLFormulaAlloc`, `dpll_form_add_atom_space`, `DPLLRegisterClauseLiteral`, `DPLLFormulaInsertClause`, `DPLLFormulaPrint`, and `DPLLFormulaParseLOP` adapted to Rust's explicit `TermBank` parser ownership.
- Rust stores owned `DpllClause` values and indexes active-clause sets by clause index instead of raw clause pointers. Duplicate clause registration still asserts, preserving the `PTreeStore` duplicate-entry invariant.
- Atom-table growth preserves C's lazy first allocation of 500 cells and subsequent 1.5x growth shape. Occurrence counters and positive/negative active sets are split per atom as in C.
- `parse_lop` returns the C progress text that `DPLLFormulaParseLOP` writes to `GlobalOut`, including the already-printed clause period followed by `...accepted` or `...discarded (tautology)`.

### Change Later

- `DPLLRegisterClauseLiteral` computes `atom = ABS(lit)` but grows the atom table with `while(form->atom_no <= lit)`, so negative literals can skip allocation before indexing `atoms[ABS(lit)]`. Rust grows by the absolute atom code to avoid reproducing a memory hazard; add reference coverage before deciding whether any legacy path depends on the buggy signed condition.
- `DPLLFormulaPrint` prints `pos_occur` twice in the atom-debug table instead of printing `neg_occur` in the second column. Rust preserves that observable rendering for compatibility; a later diagnostic API should either fix the second column or label the output as legacy.
- `DPLLFormulaParseLOP` allocates a temporary term bank around a borrowed `Sig_p`, then clears `terms->sig` before freeing the bank. Rust keeps the term bank explicit at the call boundary, which is safer and should remain the preferred shape unless full DPLL parser integration needs C's borrowed-signature lifetime exactly.
<!-- END MANUAL REVIEW: c_source_docs -->
