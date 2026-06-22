<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_gdweight

## Source Files

- [HEURISTICS/che_gdweight.h](../../../eprover/HEURISTICS/che_gdweight.h)
- [HEURISTICS/che_gdweight.c](../../../eprover/HEURISTICS/che_gdweight.c)

## Purpose

Evaluation of a clause by E's version of TWEE-inspired goal-direced weight. Conjecture ground terms get a lower (better) weight here. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GDWeightParamCell`
- `GDWeightParam_p`

### Macros And Constants

- `CHE_GDWEIGHT`
- `DEFAULT_POS_MULT`
- `GDWeightParamCellAlloc()`
- `GDWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(GDWeightParamCell)) SizeFree(junk, sizeof(GDWeightParamCell)) WFCB_p GDClauseWeightInit(ClausePrioFun prio_fun, ClauseSet_p axioms, int fweight, int vweight, double pos_multiplier, double goal_multiplier, long goal_const, double app_var_mult)`
- `WFCB_p GDClauseWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double GDClauseWeightCompute(void* data, Clause_p clause)`
- `void GDClauseWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `gd_term_weight`: Compute the weight of a term, counting variables as vweight and function symbols as fweight. Conjecture ground terms are treated special.
- `gd_literal_weight`: Return weight of a literal. Atoms are weight without equational encoding. pos_multiplier is applied to positive literals. Applied variable's weights are multiplied by app_var_mult.
- `gd_clause_weight`: Compute the weight of a clause by counting function symbols and variables and applying various modifiers.
- `initialize_goal_terms`: Set TPIsConjectureTerm in all terms occuring in elements from axioms with type negated_conjecture.
- `GDClauseWeightInit`: Return an initialized WFCB for GDClauseWeight evaluation. The new parameters are goal_multiplier and goal_const. Goal terms are evaluated as (tw*goal_multiplier)+goal_const. To mimmic TWEE, goal_multiplier should be 0.0 and goal_const should be fweight (i.e. goal terms have the same weight as a normal constant). To model normal clauseweight, set goal_multip...
- `GDClauseWeightParse`: Parse a clauseweight-definition.
- `GDClauseWeightCompute`: Compute an evaluation for a clause.
- `GDClauseWeightExit`: Free the data entry in a clauseweight WFCB.

### Dependencies

- `"che_gdweight.h"`
- `<che_wfcb.h>`

### Compile-Time Conditions

- `CHE_GDWEIGHT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_gdweight.h`, `HEURISTICS/che_gdweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 423 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Evaluation of a clause by E's version of TWEE-inspired goal-direced weight. Conjecture ground terms get a lower (better) weight here. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
