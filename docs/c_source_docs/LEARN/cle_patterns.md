<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_patterns

## Source Files

- [LEARN/cle_patterns.h](../../../eprover/LEARN/cle_patterns.h)
- [LEARN/cle_patterns.c](../../../eprover/LEARN/cle_patterns.c)

## Purpose

Data type (previous "norm subst") for describing terms, equations and clauses as patterns of same. the GNU Lesser General Public License. New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PatternSubstCell`
- `PatternSubst_p`

### Macros And Constants

- `CLE_PATTERNS`
- `DEFAULT_LITERAL_NO`
- `NORM_ARITY_LIMIT`
- `NORM_SYMBOL_LIMIT`
- `NORM_VAR_INIT`
- `PATTERN_SEARCH_BRANCHLIMIT`
- `PatEqnLTerm(eqn, dir)`
- `PatEqnRTerm(eqn, dir)`
- `PatIdIsNormId(symbol)`
- `PatternIdGetArity(ident)`
- `PatternIdGetIdent(ident)`
- `PatternNormCode(symbol, arity)`
- `PatternSubstCellAlloc()`
- `PatternSubstCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(PatternSubstCell*)SizeMalloc(sizeof(PatternSubstCell)) SizeFree(junk, sizeof(PatternSubstCell)) PatternSubst_p PatternSubstAlloc(Sig_p sig)`
- `CompareResult PatternLitListCompare(PatternSubst_p subst1, PStack_p listrep1, PatternSubst_p subst2, PStack_p listrep2)`
- `CompareResult PatternTermCompare(PatternSubst_p subst1, Term_p t1, PatternSubst_p subst2, Term_p t2)`
- `CompareResult PatternTermPairCompare(PatternSubst_p subst1, Eqn_p eqn1, PatEqnDirection dir1, PatternSubst_p subst2,Eqn_p eqn2, PatEqnDirection dir2)`
- `FunCode PatSymbValue(PatternSubst_p subst, FunCode f_code)`
- `FunCode PatternSubstGetOriginalSymbol(PatternSubst_p subst, FunCode f)`
- `PStack_p DebugPatternClauseToStack(Clause_p clause)`
- `PatternSubst_p PatternDefaultSubstAlloc(Sig_p sig)`
- `PatternSubst_p PatternSubstCopy(PatternSubst_p subst)`
- `Term_p PatternTranslateSig(Term_p term, PatternSubst_p subst, Sig_p old_sig, Sig_p new_sig, VarBank_p new_vars)`
- `bool PatSubstExtend(PatternSubst_p subst, FunCode symbol, int arity)`
- `bool PatSymbolIsBound(PatternSubst_p subst, FunCode f_code)`
- `bool PatternLitListCompute(PatternSubst_p subst, PStack_p listrep)`
- `bool PatternSubstBacktrack(PatternSubst_p subst, PStackPointer old_state)`
- `bool PatternTermCompute(PatternSubst_p subst, Term_p term)`
- `bool PatternTermPairCompute(PatternSubst_p subst, Eqn_p eqn, PatEqnDirection direction)`
- `long PatternClauseCompute(Clause_p clause, PatternSubst_p* subst, PStack_p *listrep)`
- `void PatternClausePrint(FILE* out, PatternSubst_p subst, PStack_p listrep)`
- `void PatternEqnPrint(FILE* out, PatternSubst_p subst, Eqn_p eqn, PatEqnDirection direction)`
- `void PatternSubstFree(PatternSubst_p junk)`
- `void PatternTermPrint(FILE* out, PatternSubst_p subst, Term_p term, Sig_p sig)`

## Implementation Notes

### Internal Functions

- `collect_choices`
- `complete_state`
- `generate_print_rep`
- `get_new_fun_symbol`
- `initialize_lit_list`
- `lit_list_rep_pattern`
- `mark_minimal_literals`
- `pat_symbol_compare`
- `pat_term_size_compare`

### Source-Level Behavior

- `get_new_fun_symbol`: Return a new norm-id for a given arity.
- `pat_symb_comp_val`: Return the norm id assigned to f_code, or the alpha-rank if symbol is self-bound.
- `pat_symbol_compare`: Compare two function symbols as follows: Originally: If either symbol is unbound but should be bound, return to_uncomparable. Otherwise, compare symbols numerically (if truly bound) or by their alpha-ranks (if bound to themselves). Fresh variables are not expected to be bound. However: During pattern generation, an unbound symbol can only be bound to a larg...
- `generate_print_rep`: Given a norm-id, generate the print-representation into *id.
- `pat_term_size_compare`: Compare two terms with (a variant of) the lexicograpic extension to the ordering induced by the default weights. If two symbols of different arities are encountered, the one with the higher arity is always bigger. This ordering is used as the base for pattern comparison and is independend of actual function symbols. Note that terms with more function symbol...
- `initialize_lit_list`: For all unused literals in list mark all literals as potentially minimal in all possible directions.
- `mark_minimal_literals`: Among all literals in list that do not have EPIsUsed set, mark those literal/direction combinations that are potentially minimal in the semi-complete pattern ordering.
- `collect_choices`: For a list of literals collect all possible choices for the next literal to appear in the pattern for the list. Returns number of possibilities, choices are represented by pairs (literal,direction) on the choice stack.
- `complete_state`: Complete a state in the search. A state is described by - A list of literals, some of which may be used up already (marked with EPIsUsed) - A stack which contains exactly the used equations (and determines their order in the pattern) - A partial pattern substitution generated from the used literals - A state stack which organizes the search. It contains 3 e...
- `lit_list_rep_pattern`: Generate the representative pattern for a list of equations. Return number of possibilities tried, or 0 if routine terminates because the cost is estimated as to expensive.
- `PatternSubstAlloc`: Allocate an empty initialized pattern-substitution cell.
- `PatternDefaultSubstAlloc`: Allocate an empty initialized pattern-substitution cell where all special function symbols are bound to themselves.
- `PatternSubstFree`: Free the memory taken by a pattern-subst cell.
- `PatSubstExtend`: Extend the pattern subst to substitute symbol (if not already done). Return true if the subst has been extended.
- `PatternSubstCopy`: Copy a pattern-substitution.
- `PatSymbValue`: Return the norm id assigned to f_code.
- `PatSymbolIsBound`: Return true is f_code is either bound to a symbol or should not be bound at all.
- `PatternSubstBacktrack`: Backtrack a pattern-subst to a given state. Return true if the state differs from the current one.
- `PatternTermCompute`: Extend subst to make term into a pattern. Return true if a new renaming has been added.
- `PatternTermCompare`: Compare two term patterns.
- `PatternTermPairCompute`: Compute a pattern subst for a given equation.
- `PatternTermPairCompare`: v// Compare two equation patterns (described by pattern-subst, eqn, direction).
- `PatternLitListCompute`: Compute a pattern-subst for the list (well, stack) of oriented literals.
- `PatternLitListCompare`: Compare two patterns for clauses, each represented by a pattern substitution and a stack of (oriented) literals. This does not correspond exactly to the definition in my thesis, but agrees with it on comparisons of different patterns for the same clause (while being easier and more efficient in the general case, where it does not matter).
- `PatternClauseCompute`: Compute the representative pattern for a clause and return it (via the reference variables). Returns number of substitutions tried, or 0 if the internal resource estimator canceled the computation.
- `PatternTermPrint`: Print the pattern-term to out. Supports only standard E syntax.
- `PatternEqnPrint`: Print a pattern equation in the most reasonable form.
- `PatternClausePrint`: Print the clause pattern represented by listrep and subst and print it as a LOP list of literals. This format is primarily for machine use. Regularity and compactness are more important than beauty.
- `DebugPatternClauseToStack`: Generate the straightforward stack representation of clause (probably useful only for debugging and testing). The calling function has to return the stack!
- `PatternTranslateSig`: Create a copy of (uninstantiated) term in new_sig and new_vars, inserting all necessary idents and print-representations of norm-idents into new_sig. Norm-substituted variables are mapped to their unnormed counterparts.
- `PatternSubstGetOriginalSymbol`: Given a symbol f, return the original FunCode. Return 0 if f_code does not match any known symbol.

### Dependencies

- `"cle_patterns.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `CLE_PATTERNS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_patterns.h`, `LEARN/cle_patterns.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 1656 lines, 23 scanned public declarations, 9 scanned internal function definitions, and 31 structured function-comment blocks.
- Data type (previous "norm subst") for describing terms, equations and clauses as patterns of same. the GNU Lesser General Public License. New
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
