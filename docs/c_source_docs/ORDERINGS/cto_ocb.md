<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_ocb

## Source Files

- [ORDERINGS/cto_ocb.h](../../../eprover/ORDERINGS/cto_ocb.h)
- [ORDERINGS/cto_ocb.c](../../../eprover/ORDERINGS/cto_ocb.c)

## Purpose

Global definitions for orderings: Comparison results, precedences, order control blocks. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LiteralCmp`
- `OCBCell`
- `OCB_p`
- `TermOrdering`

### Macros And Constants

- `CTO_OCB`
- `MK_HO_VB_KEY(key, x)`
- `OCBCellAlloc()`
- `OCBCellFree(junk)`
- `OCBDBWeight(ocb)`
- `OCBDesignatedMinTerm(ocb, terms, type)`
- `OCBFunComparePos(ocb, f1, f2)`
- `OCBFunWeightPos(ocb, f)`
- `OCBLamWeight(ocb)`
- `OCBPrecedenceGetState(ocb)`
- `OCB_FUN_DEFAULT_WEIGHT`
- `W_DEFAULT_WEIGHT`

### Globals

- `extern char* TONames[]`

### Exported Functions

- `CompareResult OCBFunCompareMatrix(OCB_p ocb, FunCode f1, FunCode f2)`
- `FunCode OCBFindMinConst(OCB_p ocb, Type_p type)`
- `FunCode OCBMinConst(OCB_p ocb, Type_p type)`
- `FunCode OCBTermMaxFunCode(OCB_p ocb, Term_p term)`
- `OCB_p OCBAlloc(TermOrdering type, bool prec_by_weight, Sig_p sig, HoOrderKind ho_order_kind)`
- `PStackGetSP((ocb)->statestack) void OCBCondSetMinConst(OCB_p ocb, Type_p type, FunCode cand)`
- `PStackPointer OCBPrecedenceAddTuple(OCB_p ocb, FunCode f1, FunCode f2, CompareResult relation)`
- `TBCreateMinTerm((terms),OCBFindMinConst((ocb),(type))) static inline long OCBFunWeight(OCB_p ocb, FunCode f)`
- `bool OCBPrecedenceBacktrack(OCB_p ocb, PStackPointer state)`
- `static inline CompareResult OCBFunCompare(OCB_p ocb, FunCode f1, FunCode f2)`
- `static inline long OCBFunPrecWeight(OCB_p ocb, FunCode f)`
- `void OCBDebugPrint(FILE* out, OCB_p ocb)`
- `void OCBFree(OCB_p junk)`
- `void OCBResetHOVarMap(OCB_p ocb)`
- `void OCBSetMinConst(OCB_p ocb, Type_p type, FunCode cand)`

## Implementation Notes

### Internal Functions

- `OCBFunCompare`
- `OCBFunPrecWeight`
- `OCBFunWeight`
- `alloc_precedence`

### Source-Level Behavior

- `OCBFunWeight`: Return the weight of f in ocb. For symbols entered in the OCB after creation return OCB_FUN_DEFAULT_WEIGHT.
- `OCBFunPrecWeight`: If f has a weight in ocb->prec_weights, return it. Otherwise return a unique negative ficticious weight smaller than all normal weights.
- `OCBFunCompare`: Return comparison result of two symbols in precedence. Symbols not covered by the ocb are smaller than all others (except for $true), and older symbols are smaller than new ones.
- `free_val`: Frees the value stored in the
- `ocb_trans_compute`: Given the relations between f1 and f2, and f2 and f3, compute the relation between f1 and f3. Return true, if it can be set, false otherwise.
- `alloc_precedence`: Initialize handle->precedence or handle->prec_weights according to the value of prec_by_weight.
- `OCBAlloc`: Allocate an initialized order control block.
- `OCBFree`: Free the memory taken by an order control block. Note: The signature is not considered part of the ocb and is not free'd.
- `OCBDebugPrint`: Print an OCB in debug-friendly form (not suitable for re-parsing, revealing a lot of internal information).
- `OCBPrecedenceAddTuple`: Add a new binary relation to the precedence stored in the ocb and compute the new transitive closure of the to_greater, to_smaller and to_equal. Store updated cell in ocb->statestackcell. Return the new stackpointer if everything went fine, undo all changes and return 0 otherwise.
- `OCBPrecedenceBacktrack`: Backtrack the precedence matrix to a given state. Return true if the stack is non-empty afterwards, false otherwise.
- `OCBMinConst`: Return mininmal constant for type (if already fixed). Return 0 otherwise.
- `OCBCondSetMinConst`: Set mininmal constant for type (if not already fixed).
- `OCBFindMinConst`: Find a minimal (by precedence) function symbol constant in ocb->sig. Store it in ocb->min_constant. If no constant exists, create one.
- `OCBTermMaxFunCode`: Return the (or rather a) maximal function symbol (according to ocb->precedence) from term. Follows bindings exactly once (i.e. assumes that substitutions are matches).
- `OCBFunCompareMatrix`: Return comparison result of two symbols in precedence via the full precedence matrix. Symbols not covered by the ocb are smaller than all others. Equal symbols are not allowed (captured at OCBFunCompare).
- `OCBResetHOVarMap`: Resets mapping of (applied) variables to number of occurrences.

### Dependencies

- `"cto_ocb.h"`
- `<che_to_params.h>`
- `<clb_objmaps.h>`
- `<cte_termbanks.h>`

### Compile-Time Conditions

- `CTO_OCB`

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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_ocb.h`, `ORDERINGS/cto_ocb.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 959 lines, 20 scanned public declarations, 4 scanned internal function definitions, and 17 structured function-comment blocks.
- Ordering control block. Centralizes precedence/weights and ordering configuration shared by KBO/LPO.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Compatibility Notes

- `OCBAlloc` stores a borrowed `Sig_p` and snapshots `sig->f_count` into `sig_size`; symbols inserted into the signature after allocation are still compared through fallback rules. Rust keeps `sig_size` in the OCB and passes the live signature explicitly for property/type lookups.
- `alloc_precedence` allocates `prec_weights` with `SizeMalloc` when precedence is represented by weights, but does not initialize the array in this unit. Normal C construction fills it through precedence generation before use; Rust initializes the vector deterministically and should preserve generated values once OCB mutation is wired in.
- `OCBFunWeight` returns `OCB_FUN_DEFAULT_WEIGHT` for symbols whose f-code is greater than the OCB's saved `sig_size`. `OCBFunCompareMatrix` treats old symbols as greater than later symbols, and two later symbols are ordered by `Q_TO_PART(f2-f1)`, making higher/newer f-codes lesser.
- `OCBFunCompare` gives `$true` a special lowest-precedence result before distinct-symbol handling. Distinct object/integer properties override both precedence weights and matrix ordering by comparing the right distinct flag minus the left distinct flag.
- `OCBPrecedenceAddTuple` is documented as returning the new stack pointer, but the implementation returns the old stack pointer for an already-present relation, `1` for a newly inserted successful relation, and `0` for conflict/failure. Rust preserves this return surface for now.
- On transitive-closure failure, `OCBPrecedenceAddTuple` pops and clears only the most recent stored pair instead of rolling the matrix all the way back to the saved old state. Keep this compatibility hazard visible before changing rollback semantics.
- `OCBFindMinConst` is named/commented as finding a minimal constant, but the scan replaces the candidate when `OCBFunCompare(i, cand) == to_greater`. Rust therefore records the precedence-greater matching constant as the designated one.
- `OCBSetMinConst` is declared in the header but has no implementation in `cto_ocb.c`; Rust provides an explicit setter for internal use, but C-linkage compatibility should treat the missing C definition as a source inconsistency.
- `OCBTermMaxFunCode` skips argument zero in its recursive scan (`for(i=1; i<term->arity; i++)`). Rust preserves this exactly; decide later whether a corrected traversal belongs behind a compatibility switch once ordering reference tests cover the affected callers.
- `OCBDebugPrint` handles a null `ocb->sig` for the signature and weight sections, but the precedence-matrix section still calls `OCBFunCompare`, which uses `ocb->sig` for distinct-symbol checks. Rust keeps the signature outside the OCB and uses raw matrix cells when debug-printing without one; revisit only if null-signature OCB diagnostics become observable.

### Change Later

- `OCBPrecedenceAddTuple` has a surprising return surface: already-present relations return the old state, newly inserted successful relations return `1`, and conflicts return `0`. It also rolls back only the most recent stored pair on transitive-closure failure rather than restoring the whole saved matrix state. Rust preserves this for compatibility; a later precedence API should return a typed result and make rollback atomic.
- `OCBSetMinConst` is declared in the header but not implemented in `cto_ocb.c`. Rust has an explicit setter for internal use; treat the missing C definition as a source inconsistency until link-level compatibility or dead-declaration cleanup decides whether the symbol should exist.
- `OCBTermMaxFunCode` skips the first argument during recursive scanning. Keep the C-compatible traversal until reference ordering/search tests prove whether this affects callers, then consider a corrected traversal behind an explicit compatibility switch.
- `OCBDebugPrint` partially handles `ocb->sig == NULL` but can still reach `OCBFunCompare` paths that dereference the signature. Rust avoids that null-dereference shape in diagnostics; if null-signature OCBs become observable, keep the safer rendering and document the compatibility deviation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
