<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_formula_wrapper

## Source Files

- [CLAUSES/ccl_formula_wrapper.h](../../../eprover/CLAUSES/ccl_formula_wrapper.h)
- [CLAUSES/ccl_formula_wrapper.c](../../../eprover/CLAUSES/ccl_formula_wrapper.c)

## Purpose

Data type wrapping formulas, with all the stuff that really only applies to input or top-level formulae, not to recursive subformulae. Also has formula sets (well, wrapped formula sets). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `WFormulaCell`
- `WFormula_p`

### Macros And Constants

- `CCL_FORMULA_WRAPPER`
- `FormulaDelProp(form, prop)`
- `FormulaGiveProps(form, prop)`
- `FormulaIsAnyPropSet(form, prop)`
- `FormulaIsConjecture(form)`
- `FormulaIsHypothesis(form)`
- `FormulaQueryProp(form, prop)`
- `FormulaQueryType(form)`
- `FormulaSetProp(form, prop)`
- `FormulaSetType(form, type)`
- `WFormHasInterpretedSymbol(form)`
- `WFormulaCellAlloc()`
- `WFormulaCellFree(junk)`
- `WFormulaIsUntyped(form)`
- `WFormulaStandardWeight(wform)`
- `WFormulaTSTPPrint(out, form, fullterms, complete)`

### Globals

- `extern bool FormulaTermEncoding`
- `extern long FormulaIdentCounter`

### Exported Functions

- `Clause_p WFormClauseToClause(WFormula_p form)`
- `FunCode WFormulaGetLambdaDefinedSym(WFormula_p form)`
- `WFormulaTSTPPrintFlex((out), (form), (fullterms), (complete), true) void WFormulaTSTPPrintFlex(FILE* out, WFormula_p form, bool fullterms, bool complete, bool as_formula)`
- `WFormula_p WFormClauseParse(Scanner_p in, TB_p terms)`
- `WFormula_p WFormulaFlatCopy(WFormula_p form)`
- `WFormula_p WFormulaOfClause(Clause_p clause, TB_p terms)`
- `WFormula_p WFormulaParse(Scanner_p in, TB_p terms)`
- `WFormula_p WFormulaTPTPParse(Scanner_p in, TB_p terms)`
- `WFormula_p WFormulaTSTPParse(Scanner_p in, TB_p terms)`
- `WFormula_p WTFormulaAlloc(TB_p terms, TFormula_p formula)`
- `char* WFormulaGetId(WFormula_p form)`
- `long WFormulaReturnFCodes(WFormula_p form, PStack_p f_codes)`
- `long WFormulaSymbolDiversity(WFormula_p form)`
- `void WFormulaAppEncode(FILE* out, WFormula_p handle)`
- `void WFormulaFree(WFormula_p form)`
- `void WFormulaGCMarkCells(WFormula_p form)`
- `void WFormulaMarkPolarity(WFormula_p form)`
- `void WFormulaPrint(FILE* out, WFormula_p form, bool fullterms)`
- `void WFormulaTPTPPrint(FILE* out, WFormula_p form, bool fullterms)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `handle_ho_def`: Parse higher order definitions of form s = t where both s and t are non-formula terms or p = f where p is a predicate symbol and f is a formula.
- `DefaultWFormulaAlloc`: Allocate and return a wrapped formula cell with all values initialized to rational default values.
- `WTFormulaAlloc`: Allocate a wrapped formula given the essential information. id will automagically be set to a new value.
- `WFormulaFree`: Free a wrapped formula.
- `WFormulaFlatCopy`: Create a flat copy of the formula.
- `WFormulaGCMarkCells`: If formula is a term formula, mark the terms. Otherwise a noop.
- `WFormulaMarkPolarity`: Mark the polarity of all subformulas in form.
- `WFormulaGetId`: Return an identifier for the formula. The pointer to the identifier is good until the next call to this function or until the formula is being destroyed, whichever comes first.
- `WFormulaTPTPParse`: Parse a formula in TPTP format.
- `FormulaTPTPPrint`: Print a formula in TPTP format.
- `WFormulaTSTPParse`: Parse a formula in TSTP format.
- `WFormulaTSTPPrintFlex`: Print a formula in TSTP format. If !complete, leave of the trailing ")." for adding optional stuff. If "as_formula" is true, print clauses as (universally quantified) formulas.
- `WFormulaAppEncode`: Encodes terms in wrapped formula's literals using app encoding. Initial WFormula is not changed.
- `WFormulaParse`: Parse a formula in any supported input format.
- `WFormClauseParse`: Parse a clause into a a WFormula disjunction.
- `WFormClauseToClause`: Convert a WFormula-encoded clause to a clause proper.
- `WFormulaPrint`: Print a (wrapped) formula in the current output format.
- `WFormulaReturnFCodes`: Push all function symbol codes from form onto f_codes. Return number of symbols found.
- `WFormulaSymbolDiversity`: Return number of different symbols in form.

### Dependencies

- `"ccl_formula_wrapper.h"`
- `<ccl_tformulae.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CCL_FORMULA_WRAPPER`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_formula_wrapper.h`, `CLAUSES/ccl_formula_wrapper.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1145 lines, 23 scanned public declarations, 0 scanned internal function definitions, and 19 structured function-comment blocks.
- Data type wrapping formulas, with all the stuff that really only applies to input or top-level formulae, not to recursive subformulae. Also has formula sets (well, wrapped formula sets). the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status

- `src/clauses/formulasets.rs` stages `DefaultWFormulaAlloc`, `WTFormulaAlloc`, generated id rendering, input-name preservation policy, formula property/type helpers, formula-term ownership, `WFormulaFlatCopy`, `WFormClauseToClause`, `WFormulaOfClause`, `WFormulaReturnFCodes`, `WFormulaSymbolDiversity`, `WFormulaGetLambdaDefinedSym`, formula-backed and clause-backed `WFormulaTPTPPrint`/`WFormulaTSTPPrintFlex`/`WFormulaPrint` rendering, staged `WFormulaCNF2`, `WFormulaAppEncode`, `WFormulaGCMarkCells`, and `WFormulaMarkPolarity`.
- Parser, full source-info ownership, exact free/delete side effects, and process-global output/input policy remain pending.

### Change Later

- C `WFormulaGetId` combines a process-global generated-id counter with the mutable `FormulasKeepInputNames` global. The Rust helper takes input-name preservation explicitly; decide later whether executable compatibility needs a global policy shim or whether explicit call-site policy is preferable.
- C `WFormulaPrint` dispatches through the process-global `OutputFormat`, and the LOP case emits a warning before intentionally falling through to TPTP output. Rust uses an explicit staged print-format argument; executable integration must decide where to reproduce the warning and global dispatch.
- C `WFormClauseToClause` allocates a fresh `Clause` from the formula payload, copies wrapper properties and source info, but does not preserve the wrapper id as the printed clause id. Rust mirrors that staged behavior; decide during proof-state integration whether clause/formula conversion should preserve a stable wrapper handle separately from generated clause ids.
- C `WFormulaOfClause` closes the encoded clause into a fresh formula wrapper but does not copy clause properties or source info. Rust mirrors that metadata drop; a later proof-document owner should decide whether compatibility output needs separate parent metadata rather than overloading the new wrapper.
- C `WFormulaTSTPPrintFlex` mixes formula printing with clause-specific paths: `as_formula` closes a clause as a formula, while the non-`as_formula` path converts back to a `Clause` and prints the raw clause core. For typed first-order clause wrappers this produces a `tcf(...)` header without using `ClauseTSTPPrint`'s universally closed typed body. Rust mirrors the split; revisit only with reference output coverage.
- C `WFormulaCNF2` mutates the wrapper formula and pushes formula-level derivation entries during each changed CNF phase. Rust now exposes those phase opcodes in the staged result but does not yet store a formula-owned derivation stack; add that stack when formula archives/proof documents become real owners.
- C `WFormulaGCMarkCells` has a comment suggesting non-term formulas are a no-op, but it calls `TFormulaGCMarkCells`/`TBGCMarkTerm` directly and therefore depends on non-null bank/formula inputs. Rust currently treats a default allocated wrapper without a formula term as a no-op during GC marking so staged construction can be traversed safely; revisit this if exact assertion behavior becomes part of compatibility testing.
- C `WFormulaMarkPolarity` also assumes a non-null term formula. Rust currently treats a default allocated wrapper without a formula term as a no-op during wrapper/set-level polarity marking; revisit this with the wrapper insertion policy if exact assertion behavior becomes observable.
- C `WFormulaGetLambdaDefinedSym` accepts an `equiv(eqn(lhs, rhs), ...)` lambda-definition shape without checking that `rhs` is `$true`, but `FormulaSetDefinitionStatistics` only counts that predicate-definition shape when `rhs` is literally the shared `$true` term. Rust mirrors both shapes in their respective helper paths; keep this mismatch visible when connecting formula statistics to proof-state feature extraction.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
