<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_prefixweight

## Source Files

- [HEURISTICS/che_prefixweight.h](../../../eprover/HEURISTICS/che_prefixweight.h)
- [HEURISTICS/che_prefixweight.c](../../../eprover/HEURISTICS/che_prefixweight.c)

## Purpose

Iplementation of conjecture term prefix weight (Pref) from [CICM'16/Sec.3]. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, yan

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PrefixWeightParamCell`
- `PrefixWeightParam_p`

### Macros And Constants

- `CHE_PREFIXWEIGHT`
- `PrefixWeightParamCellAlloc()`
- `PrefixWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(PrefixWeightParamCell)) SizeFree(junk, sizeof(PrefixWeightParamCell)) PrefixWeightParam_p PrefixWeightParamAlloc(void)`
- `WFCB_p ConjectureTermPrefixWeightInit( ClausePrioFun prio_fun, OCB_p ocb, ProofState_p proofstate, VarNormStyle var_norm, RelatedTermSet rel_terms, double match_weight, double miss_weight, TermWeightExtenstionStyle ext_style, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier)`
- `WFCB_p ConjectureTermPrefixWeightParse( Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double ConjectureTermPrefixWeightCompute(void* data, Clause_p clause)`
- `void ConjectureTermPrefixWeightExit(void* data)`
- `void PrefixWeightParamFree(PrefixWeightParam_p junk)`

## Implementation Notes

### Internal Functions

- `prfx_init`
- `prfx_insert_subgens`
- `prfx_insert_subterms`
- `prfx_insert_term`
- `prfx_insert_topgens`
- `prfx_term_weight`

### Source-Level Behavior

- `PrefixWeightParamAlloc`: Allocate new parameter cell.
- `PrefixWeightParamFree`: Free the parameter cell.
- `ConjectureTermPrefixWeightParse`: Parse parameters from a scanner.
- `ConjectureTermPrefixWeightInit`: Initialize parameters cell and create WFCB.
- `ConjectureTermPrefixWeightCompute`: Compute the clause weight.
- `ConjectureTermPrefixWeightExit`: Clean up the parameter cell.

### Dependencies

- `"che_prefixweight.h"`
- `<che_termweights.h>`

### Compile-Time Conditions

- `CHE_PREFIXWEIGHT`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_prefixweight.h`, `HEURISTICS/che_prefixweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 528 lines, 8 scanned public declarations, 6 scanned internal function definitions, and 6 structured function-comment blocks.
- Iplementation of conjecture term prefix weight (Pref) from [CICM'16/Sec.3]. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- `ConjectureTermPrefixWeightCompute` lazily initializes prefix terms, calls `ClauseCondMarkMaximalTerms(local->ocb, clause)`, then scores through `ClauseTermExtWeight`; the Rust initializer installs a banked callback that preserves this order with the active proof-control OCB, mutable owner bank, and clause. The shared six-family owner audit, proof-control regression, and exact executable comparison are recorded in [`experiments/2026-07-17-066-conjecture-term-owner-context/FINDINGS.md`](../../../experiments/2026-07-17-066-conjecture-term-owner-context/FINDINGS.md).
- The Rust lazy prefix-term store now uses the shared `src/clauses/pdtrees.rs` trie subset for `PDTreeInsertTerm`/`PDTreeMatchPrefix`-style traversal, so scoring follows the C single-path match/remains counts without keeping the earlier heuristic-local vector scan as the primary data structure.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- All production heuristic evaluation sites now use the banked lazy-init/mark/score path. Removing immutable already-marked-clause adapters is optional public-API simplification, not missing proof-search ownership behavior.

<!-- END MANUAL REVIEW: c_source_docs -->
