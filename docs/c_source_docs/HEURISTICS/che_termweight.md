<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_termweight

## Source Files

- [HEURISTICS/che_termweight.h](../../../eprover/HEURISTICS/che_termweight.h)
- [HEURISTICS/che_termweight.c](../../../eprover/HEURISTICS/che_termweight.c)

## Purpose

Iplementation of conjecture subterm weight (Term) from [CICM'16/Sec.3]. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, yan

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermWeightParamCell`
- `TermWeightParam_p`

### Macros And Constants

- `CHE_TERMWEIGHT`
- `TermWeightParamCellAlloc()`
- `TermWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(TermWeightParamCell)) SizeFree(junk, sizeof(TermWeightParamCell)) TermWeightParam_p TermWeightParamAlloc(void)`
- `WFCB_p ConjectureRelativeTermWeightInit( ClausePrioFun prio_fun, OCB_p ocb, ProofState_p proofstate, VarNormStyle var_norm, RelatedTermSet rel_terms, long vweight, long fweight, long cweight, long pweight, long conj_fweight, long conj_cweight, long conj_pweight, TermWeightExtenstionStyle ext_style, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier)`
- `WFCB_p ConjectureRelativeTermWeightParse( Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double ConjectureRelativeTermWeightCompute(void* data, Clause_p clause)`
- `void ConjectureRelativeTermWeightExit(void* data)`
- `void TBInsertClauseTermsNormalized( TB_p bank, Clause_p clause, VarNormStyle var_norm, RelatedTermSet rel_terms)`
- `void TermWeightParamFree(TermWeightParam_p junk)`

## Implementation Notes

### Internal Functions

- `termweight_init`
- `termweight_insert`
- `termweight_insert_subgens`
- `termweight_insert_subterms`
- `termweight_insert_topgens`
- `termweight_term_weight`
- `termweight_update_conjecture_freqs`

### Source-Level Behavior

- `TBInsertClauseTermsNormalized`: Insert clause related terms into the term bank with a given normalizations.
- `TermWeightParamAlloc`: Allocate new parameter cell.
- `TermWeightParamFree`: Free the parameter cell.
- `ConjectureRelativeTermWeightParse`: Parse parameters from a scanner.
- `ConjectureRelativeTermWeightInit`: Initialize parameters cell and create WFCB.
- `ConjectureRelativeTermWeightCompute`: Compute the clause weight.
- `ConjectureRelativeTermWeightExit`: Clean up the parameter cell.

### Dependencies

- `"che_termweight.h"`
- `<che_termweights.h>`

### Compile-Time Conditions

- `CHE_TERMWEIGHT`

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

Source files reviewed: `HEURISTICS/che_termweight.h`, `HEURISTICS/che_termweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 626 lines, 9 scanned public declarations, 7 scanned internal function definitions, and 7 structured function-comment blocks.
- Iplementation of conjecture subterm weight (Term) from [CICM'16/Sec.3]. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- `ConjectureRelativeTermWeightCompute` lazily initializes related-term frequencies, calls `ClauseCondMarkMaximalTerms(local->ocb, clause)`, then scores through `ClauseTermExtWeight`; the Rust port preserves that ordering with an OCB-backed helper and a banked WFCB callback for callers that can pass the owner bank.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Once all heuristic evaluation sites can pass the active `OCB`, mutable owner bank, and mutable clause, remove any remaining immutable relative-term scoring fallbacks without changing the lazy-init, mark, then score sequence.

<!-- END MANUAL REVIEW: c_source_docs -->
