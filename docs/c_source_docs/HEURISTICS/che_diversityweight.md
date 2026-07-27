<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_diversityweight

## Source Files

- [HEURISTICS/che_diversityweight.h](../../../eprover/HEURISTICS/che_diversityweight.h)
- [HEURISTICS/che_diversityweight.c](../../../eprover/HEURISTICS/che_diversityweight.c)

## Purpose

Evaluation of a clause by refined diversity clause weight, using weight penalty factors for maximal terms and literals, and penalties for clauses with many different function symbols and variables. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DiversityWeightParamCell`
- `DiversityWeightParam_p`

### Macros And Constants

- `CHE_DIVERSITYWEIGHT`
- `DEFAULT_MAX_MULT`
- `DiversityWeightParamCellAlloc()`
- `DiversityWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(DiversityWeightParamCell)) SizeFree(junk, sizeof(DiversityWeightParamCell)) WFCB_p DiversityWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double fdiff1weight, double fdiff2weight, double vdiff1weight, double vdiff2weight, double app_var_mult)`
- `WFCB_p DiversityWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double DiversityWeightCompute(void* data, Clause_p clause)`
- `void DiversityWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DiversityWeightInit`: Return an initialized WFCB for DiversityWeight evaluation.
- `DiversityWeightParse`: Parse a DiversityWeight-definition.
- `DiversityWeightCompute`: Compute an evaluation for a clause.
- `DiversityWeightExit`: Free the data entry in a clauseweight WFCB.

### Dependencies

- `"che_diversityweight.h"`
- `<che_clauseweight.h>`

### Compile-Time Conditions

- `CHE_DIVERSITYWEIGHT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_diversityweight.h`, `HEURISTICS/che_diversityweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 301 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Evaluation of a clause by refined diversity clause weight, using weight penalty factors for maximal terms and literals, and penalties for clauses with many different function symbols and variables. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- `DiversityWeightCompute` calls `ClauseCondMarkMaximalTerms(local->ocb, clause)` before `ClauseWeight`, then computes function-symbol and variable diversity penalties from the marked clause; the Rust initializer installs a banked callback that preserves this order with the active proof-control OCB, mutable owner bank, and clause. The shared diversity/orient owner audit, proof-control regression, and exact executable comparison are recorded in [`experiments/2026-07-17-067-diversity-orient-owner-context/FINDINGS.md`](../../../experiments/2026-07-17-067-diversity-orient-owner-context/FINDINGS.md).
- The private production WFCB counts function and variable diversity in one operation-local subterm traversal. Non-variable terms retain `ClauseReturnFCodes`' operation-flag visit/reset behavior, while free variables are recorded independently so a stale variable flag cannot suppress their count. Public C-shaped collection helpers remain unchanged, only the bounded variable-ID vector is retained, and exact/replicated native measurements improve whole-prover instructions by 1.82% and native wall/CPU means by 1.61%/1.49%. Proof determinism and the maintained 50-case matrix remain exact; evidence is recorded in [`experiments/2026-07-24-013-fused-diversity-traversal/FINDINGS.md`](../../../experiments/2026-07-24-013-fused-diversity-traversal/FINDINGS.md).
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- The immutable diversity scoring callback remains a low-level/test adapter for clauses whose orientation flags are already current. Removing that public compatibility surface after an API review is optional cleanup; production HCB evaluation already uses the banked owner path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
