<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termvars

## Source Files

- [TERMS/cte_termvars.h](../../../eprover/TERMS/cte_termvars.h)
- [TERMS/cte_termvars.c](../../../eprover/TERMS/cte_termvars.c)

## Purpose

Functions for the management of shared variables. the GNU Lesser General Public License. now obsolete cte_vartrans.h)

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `VarBankCell`
- `VarBankNamedCell`
- `VarBankNamed_p`
- `VarBankStack_p`
- `VarBank_p`

### Macros And Constants

- `CTE_TERMVARS`
- `DEFAULT_VARBANK_SIZE`
- `INITIAL_SORT_STACK_SIZE`
- `VarBankCellAlloc()`
- `VarBankCellFree(junk)`
- `VarBankGetAltFreshVar(bank, sort)`
- `VarBankNamedCellAlloc()`
- `VarBankNamedCellFree(junk)`
- `VarFCodeIsAltCode(f_code)`
- `VarIsAltVar(var)`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p VarBankExtNameAssertAlloc(VarBank_p bank, char* name)`
- `Term_p VarBankExtNameAssertAllocSort(VarBank_p bank, char* name, Type_p sort)`
- `Term_p VarBankExtNameFind(VarBank_p bank, char* name)`
- `Term_p VarBankFCodeFind(VarBank_p bank, FunCode f_code)`
- `Term_p VarBankGetFreshVar(VarBank_p bank, Type_p sort)`
- `Term_p VarBankVarAlloc(VarBank_p bank, FunCode f_code, Type_p sort)`
- `VarBankStack_p VarBankCreateStack(VarBank_p bank, TypeUniqueID sort)`
- `VarBank_p VarBankAlloc(TypeBank_p sort_table)`
- `long VarBankCardinality(VarBank_p bank)`
- `long VarBankCollectVars(VarBank_p bank, PStack_p stack)`
- `static inline Term_p VarBankVarAssertAlloc(VarBank_p bank, FunCode f_code, Type_p sort)`
- `static inline VarBankStack_p VarBankGetStack(VarBank_p bank, TypeUniqueID sort)`
- `void VarBankClearExtNames(VarBank_p bank)`
- `void VarBankClearExtNamesNoReset(VarBank_p bank)`
- `void VarBankFree(VarBank_p junk)`
- `void VarBankPairShadow(VarBank_p primary, VarBank_p secondary)`
- `void VarBankPopEnv(VarBank_p bank)`
- `void VarBankPushEnv(VarBank_p bank)`
- `void VarBankResetVCounts(VarBank_p bank)`
- `void VarBankSetVCountsToUsed(VarBank_p bank)`
- `void VarBankVarsDelProp(VarBank_p bank, TermProperties prop)`
- `void VarBankVarsSetProp(VarBank_p bank, TermProperties prop)`

## Implementation Notes

### Internal Functions

- `VarBankGetAltVar`
- `VarBankGetStack`
- `VarBankVarAssertAlloc`

### Source-Level Behavior

- `VarBankGetStack`: Obtain a pointer to the stack that stores variables of a given sort.
- `VarBankVarAssertAlloc`: Return a pointer to the variable with the given f_code and sort in the variable bank. Create the variable if it does not exist.
- `VarBankGetAltVar`: Given variable X_n, return Y_n (i.e. the one with f_code increased by one - -1 goes to -2).
- `var_named_new`: Create a new VarBankNamedCell associating name and variable.
- `var_named_free`: free a VarBankNamed structure.
- `clear_env_stack`: clear the env stack, removing all named cells
- `VarBankAlloc`: Allocate an empty, initialized VarBank-Structure, return pointer to it.
- `VarBankFree`: Deallocate a VarBankCell.
- `VarBankPairShadow`: Pair two variable banks to ensure that they ave consistent id/variable mappings. Primary may contain variables, secondary should be empty.
- `VarBankCreateStack`: Create a stack for variables of the given sort.
- `VarBankResetVCounts`: Reset all the fresh variable counters for the different sorts.
- `VarBankSetVCountsToUsed`: Set all the fresh variable counters for the different sorts to the maximum number of variables allocated for that sort.
- `VarBankClearExtNames`: Reset the External name -> FunCode association state
- `VarBankClearExtNamesNoReset`: Reset the External name -> FunCode association state, but do not reset the variable counter
- `VarBankVarsSetProp`: Set the given properties in all variables.
- `VarBankVarsDelProp`: Delete the given properties in all variables.
- `VarBankFCodeFind`: Return the pointer to the variable associated with given f_code if it exists in the VarBank, NULL otherwise.
- `VarBankExtNameFind`: Return the pointer to the variable associated with given external name if it exists in the VarBank, NULL otherwise.
- `var_bank_var_alloc`: Return a pointer to the newly created variable with the given f_code and sort in the variable bank.
- `VarBankVarAlloc`: Return a pointer to the newly created variable with the given f_code and sort in the variable bank.
- `VarBankGetFreshVar`: Return a pointer to the next "fresh" variable. Freshness is controlled by the v_count entry in the variable bank, which is increased by this function. The variable is only guaranteed to be fresh if VarBankVarAssertAlloc() calls are not mixed with VarBankGetFreshVar() calls. As of 2010-02-10 this will only return even numbered variables - odd ones are reserv...
- `VarBankExtNameAssertAlloc`: Return a pointer to the variable with the given external name in the variable bank. Create a new variable if none with the given name exists and assign it the next unused FunCode.
- `VarBankExtNameAssertAllocSort`: Return a pointer to the variable with the given external name and sort in the variable bank. Create a new variable if none with the given name exists and assign it the next unused FunCode.
- `VarBankCardinality`: Returns the number of variables in the whole var bank
- `VarBankCollectVars`: Collect all the variables of the bank onto the given stack. Returns the total number of variables pushed onto the stack.

### Dependencies

- `"cte_termvars.h"`
- `<clb_pdarrays.h>`
- `<clb_pstacks.h>`
- `<cte_termtypes.h>`
- `<cte_typebanks.h>`

### Compile-Time Conditions

- `CTE_TERMVARS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for inference canonicalization and rewrite-cache coupling on 2026-07-09, and TypeBank/default-sort ownership on 2026-07-17.

Source files reviewed: `TERMS/cte_termvars.h`, `TERMS/cte_termvars.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1029 lines, 27 scanned public declarations, 3 scanned internal function definitions, and 25 structured function-comment blocks.
- Functions for the management of shared variables. the GNU Lesser General Public License. now obsolete cte_vartrans.h)
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `VarBankCollectVars` prints `VarBankCollectVars()...` and `...VarBankCollectVars()` directly to stdout around its collection loop, and that loop scans `i < max_var`, so a variable stored exactly at `max_var` is skipped. Rust preserves the loop-bound quirk in `VarBank::collect_vars` and exposes `collect_vars_with_output` for the C progress text while keeping the ordinary helper output-free.
- Paired proof-state variable banks contain distinct variable cells with synchronized f-codes and types. Inference constructors reset the shadow bank's per-sort counters, bind source variables to its low canonical codes, and only then insert instantiated terms into the live term bank. This makes variable-counter state observable through shared term identity and rewrite-link caching.
- C retains the whole `TypeBank_p`, but ordinary allocation reads it only for the bank's immutable shared `default_type`; typed allocation receives its `Type_p` explicitly. Rust retains that same shared default `Type` handle and keys variable stacks/counters dynamically by type UID. A regression constructs the Rust bank first, inserts a user sort later, then proves typed allocation uses the late shared sort while default-name allocation retains the identical `$i` handle. Parser construction therefore does not need to wait for all user sorts.
- C stores variables in a `PDArray` indexed directly by `-f_code`. Rust now uses the same logical direct index through lazily allocated 64-entry pages: hot `VarBankVarAssertAlloc`-equivalent work falls 47.81% in the retained LUSK6 Callgrind profile, while a million-scale sparse-code regression allocates only one term page. A monolithic Rust vector was rejected because BOO020 reproduced an allocator abort near the maintained 2-GiB boundary; the paged table preserves exact focused BOO020/SWV851 `ResourceOut` behavior.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- C's `VarBankPushEnv`/`VarBankPopEnv` stack restores old external-name bindings only when `VarBankExtNameAssertAllocSort` shadows a name with a different type; same-type quantifier shadowing is handled later by the full formula variable-renaming pipeline rather than by the raw variable bank. Rust keeps the C-shaped assert-allocation helpers, but the temporary executable FOF/TFF bridge uses declaration-specific scoped allocation so same-name quantified variables cannot create self-referential Skolem bindings before the real `TFormula` owner exists.
- C inference wrappers receive a reusable `freshvars` bank paired with the live term-bank variables, and individual inference families choose whether to reset or consume its counters. Rust uses short-lived normalization banks and mirrors the paramodulation reset, but proof-state-owned reuse remains the compatibility target once inference wrappers can borrow the state owner directly. A later API should encode reset policy in the operation type instead of relying on ambient mutable counter state.
- `VarBankCollectVars` couples variable collection to debug progress output. Keep the output-aware wrapper for compatibility callers, but prefer the quiet collection helper for ordinary Rust code after drop-in behavior is secured.
- Formula closure stores variable pointers in `PTree`, so same-sort quantifier order can expose raw term-cell allocation addresses even though the order is logically irrelevant. Rust mirrors the pointer-tree traversal with safe term identities, but exact C text can still differ because its arity-sized term allocator and freelist determine those addresses. A later C renderer should choose a stable alpha-order; exact compatibility work should address allocator-backed identity globally rather than special-casing variable names.

<!-- END MANUAL REVIEW: c_source_docs -->
