<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_varweights

## Source Files

- [HEURISTICS/che_varweights.h](../../../eprover/HEURISTICS/che_varweights.h)
- [HEURISTICS/che_varweights.c](../../../eprover/HEURISTICS/che_varweights.c)

## Purpose

Weight functions that play around a bit ;-) the GNU Lesser General Public License. <1> Wed Jun 17 00:11:03 MET DST 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, Stephan Schulz, schulz@eprover.org

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `VarWeightParamCell`
- `VarWeightParam_p`

### Macros And Constants

- `CHE_VARWEIGHTS`
- `VarWeightParamCellAlloc()`
- `VarWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(VarWeightParamCell)) SizeFree(junk, sizeof(VarWeightParamCell)) WFCB_p TPTPTypeWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double conjecture_multiplier, double hypothesis_multiplier, double app_var_mult)`
- `WFCB_p ClauseWeightAgeInit(ClausePrioFun prio_fun, int fweight, int vweight, double pos_multiplier, double weight_multiplier, double app_var_mult)`
- `WFCB_p ClauseWeightAgeParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p DepthWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double term_weight_multiplier, double app_var_mult)`
- `WFCB_p DepthWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p NLWeightInit(ClausePrioFun prio_fun, int fweight, int vlweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double app_var_mult)`
- `WFCB_p NLWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p PNRefinedWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, int nfweight, int nvweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double app_var_mult)`
- `WFCB_p PNRefinedWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ProofWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double proof_size_multiplier, double proof_depth_multiplier, double app_var_mult)`
- `WFCB_p ProofWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p SigWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double sig_size_multiplier, double app_var_mult)`
- `WFCB_p SigWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p StaggeredWeightInit(ClausePrioFun prio_fun, double stagger_factor, ClauseSet_p axioms)`
- `WFCB_p StaggeredWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p SymTypeWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, int cweight, int pweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double app_var_mult)`
- `WFCB_p SymTypeWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p TPTPTypeWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p WeightLessDepthInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double depth_weight_multiplier, double app_var_mult)`
- `WFCB_p WeightLessDepthParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double ClauseWeightAgeCompute(void* data, Clause_p clause)`
- `double DepthWeightCompute(void* data, Clause_p clause)`
- `double NLWeightCompute(void* data, Clause_p clause)`
- `double PNRefinedWeightCompute(void* data, Clause_p clause)`
- `double ProofWeightCompute(void* data, Clause_p clause)`
- `double SigWeightCompute(void* data, Clause_p clause)`
- `double StaggeredWeightCompute(void* data, Clause_p clause)`
- `double SymTypeWeightCompute(void* data, Clause_p clause)`
- `double TPTPTypeWeightCompute(void* data, Clause_p clause)`
- `double WeightLessDepthCompute(void* data, Clause_p clause)`
- `void VarWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TPTPTypeWeightInit`: Initialize a WFCB for a TPTPTypeWeight-Evaluation function, that modifies a base refinedweight according to clause type.
- `TPTPTypeWeightParse`: Parse a TPTPTypeWeight-Evaluation function.
- `TPTPTypeWeightCompute`: Compute a weight and adjust it for clause type.
- `SigWeightInit`: Initialize a WFCB for a SigWeight-Evaluation function, that modifies a base refinedweight according to number of different function symbols occuring in the clause
- `SigWeightParse`: Parse a SigWeight-Evaluation function.
- `SigWeightCompute`: Compute a weight and adjust it for clause type.
- `ProofWeightInit`: Initialize a WFCB for a ProofWeight-Evaluation function, that modifies a base refinedweight according to clause proof lenght and depth.
- `ProofWeightParse`: Parse a ProofWeight-Evaluation function.
- `ProofWeightCompute`: Compute a weight and adjust it for proof depth and lenght.
- `DepthWeightInit`: Initialize a WFCB for a DepthWeight-Evaluation function that uses both dept and weight.
- `DepthWeightParse`: Parse a DepthWeight-Evaluation function.
- `DepthWeightCompute`: Compute a weight and adjust it for clause type.
- `WeightLessDepthInit`: Initialize a function that evaluates terms as weight-gamma*dpth.
- `WeightLessDepthParse`: Parse the above function.
- `WeightLessDepthCompute`: Compute the evaluation function.
- `NLWeightInit`: Initialize a WFCB for a Non-Linear Weight-Evaluation function, that modifies a base Refinedweight by distinguishing linear and non-linear variables.
- `NLWeightParse`: Parse a NLWeight-Evaluation function.
- `NLWeightCompute`: Compute a non-linar weight.
- `PNRefinedWeightInit`: Return an initialized WFCB for PNRefinedWeight evaluation.
- `PNRefinedWeightParse`: Parse a PNRefinedWeight definition
- `PNRefinedWeightCompute`: Compute an evaluation for a clause as in ClauseRefinedWeight, but use different weights for function symbols/variables in
- `SymTypeWeightInit`: Return an initialized WFCB for SymTypeWeight evaluation. This gives different weights to non-constant symbols, constant symbols, predicate symbols, and variables.
- `SymTypeWeightParse`: Parse a SymTypeWeight declaration, return a suitable WFCB.
- `SymTypeWeightCompute`: Compute a symbol type based clause weight.
- `ClauseWeightAgeInit`: Return an initialized WFCB for ClauseWeightAge evaluation.
- `ClauseWeightAgeParse`: Parse a clauseweight-definition.
- `ClauseWeightAgeCompute`: Compute an evaluation for a clause.
- `StaggeredWeightInit`: Initialize a staggered evaluation function (to replace FIFO). Assign weight (int)(ClauseStandardWeight(clause)/ (max(ClauseStandardWeight(initial_clause_set)*stagger_factor)). Precedence within each class is by the tie-breaking fifo.
- `StaggeredWeightParse`: Parse a staggered weight evaluation function.
- `StaggeredWeightCompute`: Compute the staggered weight of a clause.
- `VarWeightExit`: Free the data entry in a varweight WFCB.

### Dependencies

- `"che_varweights.h"`
- `<che_clausesetfeatures.h>`
- `<che_refinedweight.h>`

### Compile-Time Conditions

- `CHE_VARWEIGHTS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_varweights.h`, `HEURISTICS/che_varweights.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1428 lines, 33 scanned public declarations, 0 scanned internal function definitions, and 31 structured function-comment blocks.
- Weight functions that play around a bit ;-) the GNU Lesser General Public License. <1> Wed Jun 17 00:11:03 MET DST 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `TPTPTypeWeightCompute`, `SigWeightCompute`, `ProofWeightCompute`, `DepthWeightCompute`, `WeightLessDepthCompute`, `NLWeightCompute`, `PNRefinedWeightCompute`, and `SymTypeWeightCompute` all call `ClauseCondMarkMaximalTerms(data->ocb, clause)` before applying their specific scoring formulas. Rust preserves this with OCB-backed helpers and banked WFCB callbacks; no-bank callbacks remain compatibility fallbacks for already-marked clauses.

### Change Later

- Once all heuristic evaluation sites can pass the active `OCB`, mutable owner bank, and mutable clause, remove any remaining immutable varweight scoring fallbacks without changing the mark-then-score formulas.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
