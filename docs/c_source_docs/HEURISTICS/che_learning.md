<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_learning

## Source Files

- [HEURISTICS/che_learning.h](../../../eprover/HEURISTICS/che_learning.h)
- [HEURISTICS/che_learning.c](../../../eprover/HEURISTICS/che_learning.c)

## Purpose

Evaluation of a clause by tsm-based learning algorithms the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TSMParamCell`
- `TSMParam_p`

### Macros And Constants

- `CHE_LEARNING`
- `DEFAULT_POS_MULT`
- `TSMParamCellAlloc()`
- `TSMParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(TSMParamCell)) SizeFree(junk, sizeof(TSMParamCell)) WFCB_p TSMWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, bool flat_clauses, double learnweight, char* kb, ProofState_p state, long sel_no, double set_part, double dist_part, IndexType indextype, TSMType tsmtype, long depth, double proofs_w, double dist_w, double p_simp_w, double f_simp_w, double p_gen_w, double f_gen_w)`
- `WFCB_p TSMRWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, bool flat_clauses, double learnweight, char* kb, ProofState_p state, long sel_no, double set_part, double dist_part, IndexType indextype, TSMType tsmtype, long depth, double proofs_w, double dist_w, double p_simp_w, double f_simp_w, double p_gen_w, double f_gen_w)`
- `WFCB_p TSMRWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p TSMWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double TSMRWeightCompute(void* data, Clause_p clause)`
- `double TSMWeightCompute(void* data, Clause_p clause)`
- `void TSMWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- `tsm_param_init`

### Source-Level Behavior

- `tsm_param_init`: Return an initialized TSMParaCell.
- `TSMWeightInit`: Initialize a TSM-based evaluation function.
- `TSMWeightParse`: Parse a TSMWeight-definition. The format is: TSMWeight(prio_fun, fweight, vweight, learnweight, flat|rec, <kb-name>, max_proof_examples, max_proof_parts, max_dist_part, tsmtype, indextype, indexdepth, proofs_w, dist_w, p_simp_w, f_simp_w, p_gen_w, f_gen_w)
- `TSMWeightCompute`: Compute a TSM-based weight for a clause.
- `TSMRWeightInit`: Initialize a TSM-based refined evaluation function.
- `TSMRWeightParse`: Parse a refine TSMWeight-definition. The format is: TSMWeight(prio_fun, fweight, vweight, max_term, max_lit, pos_lit, learnweight, flat|rec, <kb-name>, max_proof_examples, max_proof_parts, max_dist_part, tsmtype, indextype, indexdepth, proofs_w, dist_w, p_simp_w, f_simp_w, p_gen_w, f_gen_w, subsum_w)
- `TSMRWeightCompute`: Compute a TSM-based weight for a clause.
- `TSMWeightExit`: Free a TSMParamCell.

### Dependencies

- `"che_learning.h"`
- `<che_wfcb.h>`
- `<cle_tsmio.h>`

### Compile-Time Conditions

- `CHE_LEARNING`

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

Source files reviewed: `HEURISTICS/che_learning.h`, `HEURISTICS/che_learning.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 696 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 8 structured function-comment blocks.
- Evaluation of a clause by tsm-based learning algorithms the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TSMWeightCompute` and `TSMRWeightCompute` lazily construct the expensive TSM on first clause evaluation, not during parsing. Rust preserves that lazy boundary; callers should not rely on KB files being opened before the WFCB is actually scored.
- The C evaluator stores `ProofState_p` and reuses `state->terms` both for `TSMFromKB` signature mutation and for per-clause flat/recursive representations. Rust currently owns a private cloned evaluation bank and copies each scored clause into it before pattern encoding. Revisit this once proof-state term banks are shared session owners.
- `TSMWeightExit` frees `local->tsmadmin->subst` and `local->pat_subst` only after the lazy TSM was created. Rust models this with owned evaluator state; cleanup remains tied to WFCB drop rather than a public manual free.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
