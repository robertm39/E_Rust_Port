<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_funweights

## Source Files

- [HEURISTICS/che_funweights.h](../../../eprover/HEURISTICS/che_funweights.h)
- [HEURISTICS/che_funweights.c](../../../eprover/HEURISTICS/che_funweights.c)

## Purpose

Heuristic weight functions dealing with individual weights for different symbols. the GNU Lesser General Public License. <1> Sat May 7 20:57:21 CEST 2005

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FunWeightParamCell`
- `FunWeightParam_p`

### Macros And Constants

- `CHE_FUNWEIGHTS`
- `FunWeightParamCellAlloc()`
- `FunWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(FunWeightParamCell)) SizeFree(junk, sizeof(FunWeightParamCell)) FunWeightParam_p FunWeightParamAlloc(void)`
- `WFCB_p ConjectureRelativeSymbolTypeWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ConjectureRelativeSymbolWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ConjectureSimplifiedSymbolWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ConjectureSymbolWeightInit(ClausePrioFun prio_fun, OCB_p ocb, ClauseSet_p axioms, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, long cweight, long pweight, long conj_fweight, long conj_cweight, long conj_pweight, double app_var_mult, void (*init_fun)(struct funweightparamcell*))`
- `WFCB_p ConjectureSymbolWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ConjectureTypeBasedWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p FunWeightInit(ClausePrioFun prio_fun, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, PStack_p fweights, double app_var_mult)`
- `WFCB_p FunWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p RelevanceLevelWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p RelevanceLevelWeightParse2(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p SymOffsetWeightInit(ClausePrioFun prio_fun, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, PStack_p fweights, double app_var_mult)`
- `WFCB_p SymOffsetWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double GenericFunWeightCompute(void* data, Clause_p clause)`
- `double SymOffsetWeightCompute(void* data, Clause_p clause)`
- `void FunWeightParamFree(FunWeightParam_p junk)`
- `void GenericFunWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- `init_conj_t_vector`
- `init_conj_typeweight_vector`
- `init_conj_vector`
- `init_fun_weights`
- `init_relevance_vector`
- `init_relevance_vector2`
- `parse_op_weight`

### Source-Level Behavior

- `init_conj_vector`: Initialize the function weight vector based on the data in data ;-). Factored out so it can be called from the weight function(s).
- `init_conj_t_vector`: Initialize the function weight vector based on the data in data ;-). Factored out so it can be called from the weight function(s). NB: Does not consider occurences of symbols themselves but the occurence of symbol's type. data->type_freqs stays NULL!
- `init_conj_typeweight_vector`: Initialize the function weight vector based on the data in data ;-). Factored out so it can be called from the weight function(s). Initializes function symbol weights to be equal to the inverse of occurence of symbol's type + 2*occurence of symbol in the conjecture(s). Leaves type data in the data->type_freqs.
- `init_relevance_vector2`: Initialize the function weight vector based on the data in data ;-). Uses relevance levels.
- `init_relevance_vector`: Initialize the function weight vector based on the data in data ;-). Uses relevance levels.
- `parse_op_weight`: Parse a tuple fun:weight and push it onto the result stack.
- `parse_op_signweight`: Parse a tuple fun:weight and push it onto the result stack.
- `FunWeightParamAlloc`: Return an FunWeightParamCell where the pointer-related members are properly initialized.
- `FunWeightParamFree`: Free a initialized FunWeightParamCell, including the data stored on the weight_stack (if any).
- `ConjectureSymbolWeightInit`: Return an initialized WFCB for FunWeight evaluation. This gives different weights to conjecture predicates/function symbols, and non-conjecture predicate/function symbols.
- `RelevanceLevelWeightInit`: Return an initialized WFCB for FunWeight evaluation. This gives different weights based on the relevancy level.
- `RelevanceLevelWeightInit2`: Return an initialized WFCB for FunWeight evaluation. This gives different weights based on the relevancy level.
- `ConjectureSymbolWeightParse`: Parse a funweight-weight function giving different weights to conjecture symbols and other symbols.
- `ConjectureSimplifiedSymbolWeightParse`: Parse a funweight-weight function giving different weights to conjecture symbols and other symbols. Does not special-case constants.
- `ConjectureRelativeSymbolWeightParse`: As above, but give the weight of conjecture symbols as a multiple of non-conjecture symbols weight. Note that all weights are rounded down to the next integer!
- `ConjectureTypeBasedWeightParse`: Assign each function symbol the weight equal to occurence of the symbol's type in conjecture + 2*symbols occurence in conjecture
- `ConjectureRelativeSymbolTypeWeightParse`: As above, but give the weight of conjecture symbols as a multiple of non-conjecture symbols weight. Note that all weights are rounded down to the next integer! NOTE: Symbol is considered a conjecture symbol if a symbol of the same type appears in the conjecture -- difference from above functions.
- `RelevanceLevelWeightParse`: Parse the specification of a RelevanceLevelWeight function. The parameters are:
- `RelevanceLevelWeightParse2`: Parse the specification of a RelevanceLevelWeight function. The parameters are:
- `FunWeightInit`: Initialize a weight function with explicit weights for (some) function symbols.
- `FunWeightParse`: Parse a FunWeight evaluation function.
- `SymOffsetWeightInit`: Initialize a weight function with explicit offsets for (some) function symbols.
- `SymOffsetWeightParse`: Parse a FunWeight evaluation function.
- `GenericFunWeightCompute`: Compute a clause weight as Refinedweight(), but use the function symbol weights in data->fweights for individual values.
- `SymOffsetWeightCompute`: Compute a clause weight as Refinedweight(), but use the function symbol weights in data->fweights to compute an extra per-symbol (not per symbol occurrence!) offset.
- `GenericFunWeightExit`: Free an FunWeightParamCell, including the optional weight array.

### Dependencies

- `"che_funweights.h"`
- `<ccl_relevance.h>`
- `<che_refinedweight.h>`

### Compile-Time Conditions

- `CHE_FUNWEIGHTS`

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

Source files reviewed: `HEURISTICS/che_funweights.h`, `HEURISTICS/che_funweights.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1725 lines, 19 scanned public declarations, 7 scanned internal function definitions, and 26 structured function-comment blocks.
- Heuristic weight functions dealing with individual weights for different symbols. the GNU Lesser General Public License. <1> Sat May 7 20:57:21 CEST 2005
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `GenericFunWeightCompute` calls the configured `init_fun` lazily, then `ClauseCondMarkMaximalTerms(data->ocb, clause)`, then `ClauseFunWeight` with `data->fweights` and optional `type_freqs`; Rust preserves that init/mark/score order with an OCB-backed helper and a banked WFCB callback for callers that can pass the owner bank.
- `SymOffsetWeightCompute` follows the same lazy-init and `ClauseCondMarkMaximalTerms` order before ordinary clause weighting, then calls `ClauseAddFunOccs`, adds one configured offset per distinct symbol, and resets each touched occurrence-array slot to zero; Rust preserves the sequence with an OCB-backed helper and banked WFCB callback.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Once all heuristic evaluation sites can pass the active `OCB`, mutable owner bank, and mutable clause, remove any remaining immutable funweight scoring fallbacks without changing the C lazy-init, mark, score, offset, and occurrence-reset ordering.

<!-- END MANUAL REVIEW: c_source_docs -->
