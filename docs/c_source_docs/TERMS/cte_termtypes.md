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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for zero-suffix rewrite normalization on 2026-07-09, shared-argument ownership on 2026-07-11, compact pointer-field ownership on 2026-07-16, term representation/free-boundary ownership on 2026-07-17, borrowed type-UID access on 2026-07-20, and duplicate top-shell reuse, borrowed normalization, KBO6 traversal, shared structural comparison, borrowed top comparison, borrowed PDTree traversal, plus split intrusive-tree link mutation on 2026-07-25.

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
- `MakeRewrittenTerm` calls `LambdaNormalizeDB` even when `remaining_orig` is zero. Simultaneous paramodulation relies on this to beta-normalize the dereferenced replacement before `TBInsertNoProps`; Rust exposes the equivalent helper within the crate rather than replacing the call with direct term-bank insertion.
- C's LFHO-only `owner_bank`, `binding_cache`, and cache-freshness pointers are intentionally not part of Rust's unified term cell. Normalization-capable production paths receive `&mut TermBank` explicitly; read-only helpers use result-equivalent no-cache expansion. This avoids stale self-pointers and a projected 17.647059% increase from the measured 136-byte term cell to at least 160 bytes for every first- and higher-order term. Exact higher-order inference/ordering matrices and the fresh 1.0801753448x Rust/C aggregate support this completed ownership/performance decision in [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).
- C keeps the nullable binding, rewrite-replacement, type, and left/right store links inline in each `TermCell`. Rust preserves that compact ownership shape with one `RefCell<TermLinks>` for the colder binding/rewrite/type metadata and two pointer-sized `Cell<Option<Term>>` wrappers for the hot intrusive-tree links. The split keeps the aggregate link storage and complete 64-bit `TermCell` at the accepted 48-byte/152-byte boundaries while avoiding dynamic borrow bookkeeping during splaying. Link reads restore the original owner before returning a clone, transfers use `Cell::take`, and the opaque debug representation never detaches a link. Exact layout, proof, instruction, and native timing evidence is retained in [`experiment 313`](../../../experiments/2026-07-25-012-split-term-tree-links/FINDINGS.md); the earlier five-pointer compaction evidence remains in [`experiment 56`](../../../experiments/2026-07-16-056-compact-term-links/FINDINGS.md). A layout-neutral follow-up that also split the rewrite-replacement owner reduced exact work only 0.0811% and regressed both full-block and stable-tail native timing, so the cold grouping is retained; falsification evidence is in [`experiment 314`](../../../experiments/2026-07-25-013-split-rewrite-link/FINDINGS.md). This safe-handle representation is a completed port design decision, not a pending raw-allocation port.
- Subsequent scalar-metadata and argument-storage compactions supersede that historical 152-byte whole-cell figure: the current 64-bit regression pins `TermLinks` at 24 bytes and `TermCell` at 136 bytes. Final layout and comprehensive performance evidence are retained in [`experiment 329`](../../../experiments/2026-07-25-028-compact-term-arguments/FINDINGS.md).
- Hot metadata consumers can read a term's optional type UID through the borrowed `TermLinks` cell without cloning the reference-counted type handle. PD-tree prefix/query construction uses this accessor while preserving the same shared-type identity value.
- `TermFree` recursively releases an unshared non-variable tree but deliberately leaves variable cells owned by the `VarBank`; `TermTopFree` releases only the top cell because its argument pointers are borrowed or transferred. Rust encodes both boundaries through reference-counted term handles: dropping the last root handle recursively releases unretained non-variable descendants, VarBank-held variables survive, and dropping a temporary top handle only releases its child references. An explicit manual-free API would weaken this ownership contract and is intentionally not exposed.
- Rust retains that ownership model while avoiding repeated destruction and allocation of uniquely owned duplicate top wrappers: a crate-private full reset guarded by `Rc::get_mut` prepares eligible arity-zero-through-two shells for the owning term bank's bounded reuse pool. This is not manual free; shared or externally retained handles fail the uniqueness check and follow normal reference-counted destruction. The safety regressions and performance evidence are retained in [`experiment 311`](../../../experiments/2026-07-25-010-reuse-duplicate-top-shells/FINDINGS.md).
- Stable `Rc<TermCell>` allocations also support a narrowly contained non-owning normalization cursor. The cursor preserves pointer provenance and never escapes the safe substitution-normalization boundary; a live root and additive-only binding mutation keep every structural, binding, and temporary expansion allocation valid until the traversal stack is empty. Argument replacement, binding removal, or general-purpose exposure would violate this contract and are intentionally unavailable. Unsafe contracts, focused liveness regressions, exact work, native timing, and complete compatibility evidence are retained in [`experiment 312`](../../../experiments/2026-07-25-011-borrowed-subst-normalization/FINDINGS.md).
- The same private cursor representation supports the first-order KBO6 balance walker under a distinct, narrower traversal contract: the live comparison root and active variable cells own every structural or binding target; first-order comparison performs no structural/removing mutation and cannot require applied-variable expansion; and reusable scratch is cleared before any stale cursor can be dereferenced after a caught panic. Higher-order walkers retain owned term handles. The complete owner reduction, native timing, liveness regression, and comprehensive validation are retained in [`experiment 316`](../../../experiments/2026-07-25-015-borrowed-kbo-balance/FINDINGS.md).
- Completed shared term allocations also support a private recursive structural-weight cursor. The safe entry dispatches to it only for two shared roots; those roots own every descendant, completed shared structure/type metadata is immutable, `arguments_mut` rejects shared terms, and the synchronous comparison invokes neither callbacks nor mutation. Unshared roots retain the safe owned comparator. Focused equivalence and mutation-guard tests plus exact-owner, repeated native, and comprehensive evidence are retained in [`experiment 317`](../../../experiments/2026-07-25-016-borrowed-struct-weight-compare/FINDINGS.md).
- Owned term-tree keys also support a private synchronous top-comparison cursor. Both owned inputs keep the top cells and initialized argument handles live; production drops every mutable argument guard before store entry; term-tree mutation touches only the separate intrusive left/right fields; and type metadata is complete and stable. The cursor preserves function-code, optional higher-order type-identity, arity, and argument-allocation ordering plus first-order debug assertions and established panic boundaries. Focused equivalence, owner reduction, repeated native timing, and comprehensive evidence are retained in [`experiment 318`](../../../experiments/2026-07-25-017-borrowed-term-top-compare/FINDINGS.md).
- Stateful first-order PDTree matching uses the cursor behind a distinct lifetime contract. The active search state owns the exact query root; an RAII guard acquires every discovered descendant owner before returning or unwinding; the pointer-deduplicated owner set grows monotonically until search exit; and reset clears both raw cursor collections before releasing descendants and then the root. Direct helpers copy function code, shared/free standard weight, type UID, and initialized argument cursors through scoped safe borrows, so an overlapping mutable `RefCell` guard still panics. Unshared compound weight and accepted substitution bindings reconstruct temporary or retained owners. Higher-order matching remains fully owned. Focused mutation-between-calls liveness and initial borrowed-cursor evidence is retained in [`experiment 319`](../../../experiments/2026-07-25-018-borrowed-pdt-query-cursor/FINDINGS.md); removal of repeated owner-set rebuild/drop work is retained in [`experiment 320`](../../../experiments/2026-07-25-019-monotonic-pdt-query-owners/FINDINGS.md).

### Change Later

- `MakeRewrittenTerm` combines prefix splicing, property/type propagation, term-bank ownership, and beta-normalization, including in the nominally no-splice zero case. Rust preserves that bundle and its production callers have parity coverage; consider splitting construction from normalization only as an API cleanup.
- C uses one mutable flexible-array `TermCell` shape both while constructing unshared terms and after term-bank sharing, even though hot metadata and rewrite paths treat shared `args[]` as immutable. A future ownership redesign should consider separate unshared-builder and immutable shared-term representations, but a direct borrowed-slice Rust prototype did not improve end-to-end `LUSK6.lop` CPU time and should not be adopted as a performance change without broader profiling.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
