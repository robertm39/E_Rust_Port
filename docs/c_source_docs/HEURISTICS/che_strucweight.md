<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_strucweight

## Source Files

- [HEURISTICS/che_strucweight.h](../../../eprover/HEURISTICS/che_strucweight.h)
- [HEURISTICS/che_strucweight.c](../../../eprover/HEURISTICS/che_strucweight.c)

## Purpose

Iplementation of conjecture structural distance weight (Struc) from [CICM'16/Sec.3]. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, yan

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `StrucWeightParamCell`
- `StrucWeightParam_p`

### Macros And Constants

- `CHE_STRUCWEIGHT`
- `StrucWeightParamCellAlloc()`
- `StrucWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(StrucWeightParamCell)) SizeFree(junk, sizeof(StrucWeightParamCell)) StrucWeightParam_p StrucWeightParamAlloc(void)`
- `WFCB_p ConjectureStrucDistanceWeightInit( ClausePrioFun prio_fun, OCB_p ocb, ProofState_p proofstate, VarNormStyle var_norm, RelatedTermSet rel_terms, double var_mismatch, double sym_mismatch, double inst_factor, double gen_factor, TermWeightExtenstionStyle ext_style, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier)`
- `WFCB_p ConjectureStrucDistanceWeightParse( Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double ConjectureStrucDistanceWeightCompute(void* data, Clause_p clause)`
- `void ConjectureStrucDistanceWeightExit(void* data)`
- `void StrucWeightParamFree(StrucWeightParam_p junk)`

## Implementation Notes

### Internal Functions

- `strc_init`
- `strc_insert_subgens`
- `strc_insert_subterms`
- `strc_insert_term`
- `strc_insert_topgens`
- `strc_term_weight`

### Source-Level Behavior

- `StrucWeightParamAlloc`: Allocate new parameter cell.
- `StrucWeightParamFree`: Free the parameter cell.
- `ConjectureStrucDistanceWeightParse`: Parse parameters from a scanner.
- `ConjectureStrucDistanceWeightInit`: Initialize parameters cell and create WFCB.
- `ConjectureStrucDistanceWeightCompute`: Compute the clause weight.
- `ConjectureStrucDistanceWeightExit`: Clean up the parameter cell.

### Dependencies

- `"che_strucweight.h"`
- `<che_termweights.h>`
- `<float.h>`

### Compile-Time Conditions

- `CHE_STRUCWEIGHT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
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

Source files reviewed: `HEURISTICS/che_strucweight.h`, `HEURISTICS/che_strucweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 623 lines, 8 scanned public declarations, 6 scanned internal function definitions, and 6 structured function-comment blocks.
- Iplementation of conjecture structural distance weight (Struc) from [CICM'16/Sec.3]. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- `ConjectureStrucDistanceWeightCompute` lazily initializes conjecture terms, then calls `ClauseCondMarkMaximalTerms` before `ClauseTermExtWeight`. The Rust initializer installs a banked callback that preserves this order with the active proof-control OCB, mutable owner bank, and clause. The shared six-family owner audit, proof-control regression, and exact executable comparison are recorded in [`experiments/2026-07-17-066-conjecture-term-owner-context/FINDINGS.md`](../../../experiments/2026-07-17-066-conjecture-term-owner-context/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- All production heuristic evaluation sites now use the banked lazy-init/mark/score path. Removing immutable already-marked-clause adapters is optional public-API simplification, not missing proof-search ownership behavior.

<!-- END MANUAL REVIEW: c_source_docs -->
