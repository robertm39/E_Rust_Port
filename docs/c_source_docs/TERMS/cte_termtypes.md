<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termtypes

## Source Files

- [TERMS/cte_termtypes.h](../../../eprover/TERMS/cte_termtypes.h)
- [TERMS/cte_termtypes.c](../../../eprover/TERMS/cte_termtypes.c)

## Purpose

Declarations for the basic term type and primitive functions, mainly on single term cells. This module mostly provides only infrastructure for higher level modules. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DerefType_p`
- `RewriteLevel`
- `RewriteState`
- `TermCell`
- `TermProperties`
- `TermRef`
- `Term_p`
- `rw_desc`

### Macros And Constants

- `ARG_NUM(term)`
- `BINDING_FRESH(t)`
- `CAN_DEREF(term)`
- `CONVERT_DEREF(i, l, d)`
- `CTE_TERMTYPES`
- `DBGTermCheckUnownedSubterm(f, t, l)`
- `DBGTermCheckUnownedSubterm(out, t, location)`
- `DEFAULT_FWEIGHT`
- `DEFAULT_VWEIGHT`
- `DEREF_ALWAYS`
- `DEREF_LIMIT(t,d)`
- `DEREF_NEVER`
- `DEREF_ONCE`
- `LFHOL_UNSUPPORTED(t)`
- `MakeRewrittenTerm(orig, new, remains, bank)`
- `REWRITE_AT_SUBTERM`
- `RewriteAdr(level)`
- `TERMARG_MEM`
- `TERMCELL_DYN_MEM`
- `TERMCELL_MEM`
- `TERMP_MEM`
- `TERMS_INITIAL_ARGS`
- `TermArgTmpArrayAlloc(n)`
- `TermArgTmpArrayFree(junk, n)`
- `TermCellAlloc()`
- `TermCellArityAlloc(arity)`
- `TermCellAssignProp(term, sel, prop)`
- `TermCellDelProp(term, prop)`
- `TermCellFlipProp(term, props)`
- `TermCellFree(junk, arity)`
- `TermCellGiveProps(term, props)`
- `TermCellIsAnyPropSet(term, prop)`
- `TermCellQueryProp(term, prop)`
- `TermCellSetProp(term, prop)`
- `TermFindUnownedSubterm(t)`
- `TermGetBank(t)`
- `TermGetCache(t)`
- `TermHasAppVar(term)`
- `TermHasBoolSubterm(t)`
- `TermHasDBSubterm(term)`
- `TermHasEqNeq(t)`
- `TermHasEtaExpandableSubterm(term)`
- `TermHasLambdaSubterm(term)`
- `TermIsAnyVar(term)`
- `TermIsAppliedAnyVar(term)`
- `TermIsAppliedDBVar(term)`
- `TermIsAppliedFreeVar(term)`
- `TermIsBetaReducible(t)`
- `TermIsConst(t)`
- `TermIsDBLambda(term)`
- `TermIsDBVar(term)`
- `TermIsEtaReducible(t)`
- `TermIsFreeVar(t)`
- `TermIsLambda(term)`
- `TermIsNonFOPattern(term)`
- `TermIsPattern(term)`
- `TermIsPhonyApp(term)`
- `TermIsPhonyAppTarget(term)`
- `TermIsRRewritten(term)`
- `TermIsRewritten(term)`
- `TermIsShared(term)`
- `TermIsTopLevelAnyVar(term)`
- `TermIsTopLevelDBVar(term)`
- `TermIsTopLevelFreeVar(term)`
- `TermIsTopRewritten(term)`
- `TermNFDate(term,i)`
- `TermRWDemod(term)`
- `TermRWDemodField(term)`
- `TermRWReplace(term)`
- `TermRWReplaceField(term)`
- `TermSetBank(t,b)`
- `TermSetCache(t,c)`
- `deref_step(orig)`

### Globals

- None found in the source scan.

### Exported Functions

- `DBGTermCheckUnownedSubtermReal((out), (t), (location)) void DBGTermCheckUnownedSubtermReal(FILE* out, Term_p t, char* location)`
- `SysDateCreationTime():(term)->rw_data.nf_date[i]) static inline Term_p TermDefaultCellAlloc(void)`
- `Term_p TermAllocNewSkolem(Sig_p sig, PStack_p variables, Type_p type)`
- `Term_p TermFindUnownedSubterm(Term_p term)`
- `Term_p applied_var_deref(Term_p orig)`
- `bool TermHasInterpretedSymbol(Term_p term)`
- `bool TermIsPrefix(Term_p needle, Term_p haystack)`
- `bool TermSearchProp(Term_p term, DerefType deref, TermProperties prop)`
- `bool TermVarSearchProp(Term_p term, DerefType deref, TermProperties prop)`
- `bool TermVerifyProp(Term_p term, DerefType deref, TermProperties prop, TermProperties expected)`
- `static inline Term_p TermConstCellAlloc(FunCode symbol)`
- `static inline Term_p TermDefaultCellArityAlloc(int arity)`
- `static inline Term_p TermDeref(Term_p term, DerefType_p deref)`
- `static inline Term_p TermDerefAlways(Term_p term)`
- `static inline Term_p TermTopAlloc(FunCode f_code, int arity)`
- `static inline Term_p TermTopCopy(Term_p source)`
- `static inline Term_p TermTopCopyWithoutArgs(Term_p source)`
- `void TermDelProp(Term_p term, DerefType deref, TermProperties prop)`
- `void TermDelPropOpt(Term_p term, TermProperties prop)`
- `void TermFree(Term_p junk)`
- `void TermSetProp(Term_p term, DerefType deref, TermProperties prop)`
- `void TermStackDelProps(PStack_p stack, TermProperties prop)`
- `void TermStackSetProps(PStack_p stack, TermProperties prop)`
- `void TermTopFree(Term_p junk)`
- `void TermVarDelProp(Term_p term, DerefType deref, TermProperties prop)`
- `void TermVarSetProp(Term_p term, DerefType deref, TermProperties prop)`

## Implementation Notes

### Internal Functions

- `GetFVarHead`
- `TermConstCellAlloc`
- `TermDefaultCellAlloc`
- `TermDefaultCellArityAlloc`
- `TermDeref`
- `TermDerefAlways`
- `TermTopAlloc`
- `TermTopCopy`
- `TermTopCopyWithoutArgs`
- `deref_step`
- `register_new_cache`

### Source-Level Behavior

- `GetFVarHead`: If a term is (possibly applied) free variable, get the term which represents this free variable.
- `deref_step`: Dereference term once
- `TermDerefAlways`: Dereference a term as many times as possible.
- `TermDeref`: Dereference a term. deref* tells us how many derefences to do at most, it will be decremented for each dereferenciation. Dereferencing applied variables creates new terms, which are cached in the original applied variable. Derefing applied variable will NOT decrease deref (just like it does not decrease deref for a normal term). Because of this, additional...
- `TermTopCopyWithoutArgs`: Return a copy of the term node. Only the top node is duplicated. Arguments are not initialized.
- `TermTopCopy`: Return a copy of the term node (and potential argument pointers). Only the top node and the pointers are duplicated, the arguments are shared between source and copy. As this function operates on nodes, it does not follow bindings! Administrative stuff (refs etc. will, of course, not be copied but initialized to rational values for an unshared term).
- `TermDefaultCellAlloc`: Allocate a term cell with default values.
- `TermDefaultCellArityAlloc`: Allocate a term cell with default values. Furthermore allocates the arguments of the term using the given arity.
- `TermConstCellAlloc`: Allocate a term cell for the constant term with symbol symbol.
- `TermTopAlloc`: Allocate a term top with given f_code and (uninitialized) argument array.
- `register_new_cache`: Stores the new (binding cache, bound to) pair for applied variable.
- `insert_deref`: Makes sure that the dereferenced applied variable is shared. Due to term replacing it might be the case that some arguments are shared and some are not.
- `clear_stale_cache`: Clears the cache if it is not up to date. Assumes that cache is stale (see BINDING_FRESH).
- `applied_var_deref`: Expands applied variable to a proper term. For example, if X is bound to f a, term X b would get expanded to f a b.
- `TermFindUnownedSubterm`: Check if term has at least one subterm without term->owner_bank set. At the moment only useful for debugging...
- `DBGTermCheckUnownedSubtermReal`: Check for unowned subterms, if found, print them with a marker and a location string.
- `TermTopFree`: Return term cell and arg array (if it exists).
- `TermFree`: Return the memory taken by an (unshared) term. Does not free the variable cells, which belong to a VarBank.
- `TermNewSkolemTerm`: Create a new Skolem term (or renaming atom) with the named variables as arguments.
- `TermSetProp`: Set the properties in all term cells belonging to term. NB: The function is never called with deref once -- no changes to DEREF_ONCE
- `TermSearchProp`: If prop is set in any subterm of term, return true, otherwise false. NB: Deref not changed -- function never used.
- `TermVerifyProp`: If prop has the expected value in all subterms of term, return true. NB: Derefs not changed -- function never called with DEREF_ONCE.
- `TermDelProp`: Delete the properties in all term cells belonging to term. NB: Derefs not changed -- function never called with DEREF_ONCE
- `TermDelPropOpt`: Delete the properties in all term cells belonging to term.
- `TermVarSetProp`: Set the properties in all variable cells belonging to term. NB: Derefs not changed -- function never called with DEREF_ONCE
- `TermHasInterpretedSymbol`: Return true if the term has at least one symbol from an interpreted sort (currently the arithmetic sorts,
- `TermVarSearchProp`: If prop is set in any variable cell in term, return true, otherwise false. NB: Derefs never changed -- function not called with DEREF_ONCE
- `TermVarDelProp`: Delete the properties in all variable cells belonging to term. NB: Derefs not changed -- function not called with DEREF_ONCE
- `TermStackSetProps`: Set the given properties in all term cells on the stack.
- `TermStackDelProps`: Delete the given properties in all term cells on the stack.
- `TermIsPrefix`: Checks if candidate is a prefix of term.
- `MakeRewrittenTerm`: Rewrite the prefix of orig using new, leaving remaining_orig arguments of orig intact.

### Dependencies

- `"cte_lambda.h"`
- `"cte_termbanks.h"`
- `"cte_termtypes.h"`
- `<clb_partial_orderings.h>`
- `<clb_properties.h>`
- `<clb_ptrees.h>`
- `<clb_sysdate.h>`
- `<cte_signature.h>`
- `<cte_simpletypes.h>`

### Compile-Time Conditions

- `CONSTANT_MEM_ESTIMATE`
- `CTE_TERMTYPES`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_termtypes.h`, `TERMS/cte_termtypes.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1636 lines, 34 scanned public declarations, 11 scanned internal function definitions, and 32 structured function-comment blocks.
- Declarations for the basic term type and primitive functions, mainly on single term cells. This module mostly provides only infrastructure for higher level modules. the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TermDeref` expands a bound applied free variable without decrementing `DEREF_ONCE`; callers use `DEREF_LIMIT`/`CONVERT_DEREF` to avoid following bindings in the prefix copied from the applied-variable head. Rust mirrors the expansion shape and the unconsumed one-step deref rule in the global term helper, while term-bank insertion paths keep their explicit prefix conversion.

### Change Later

- `applied_var_deref` stores expanded applied-variable terms in the source term's `binding_cache`, records the binding that made the cache fresh, inserts the expansion through the owning term bank, and marks the cached term with `TPIsDerefedAppVar`. Rust currently performs no-cache expansion for the global helper and separate bank-local expansion where callers already have a `TermBank`; add owner-bank metadata and cache invalidation before treating repeated LFHO dereference performance or cache-aware GC behavior as C-compatible.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
