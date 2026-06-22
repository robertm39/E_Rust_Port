<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_lemmas

## Source Files

- [PCL2/pcl_lemmas.h](../../../eprover/PCL2/pcl_lemmas.h)
- [PCL2/pcl_lemmas.c](../../../eprover/PCL2/pcl_lemmas.c)

## Purpose

Definition for dealing with lemmas in PCL protocols. the GNU Lesser General Public License. <1> Sun Jun 15 22:47:43 CEST 2003 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `InferenceWeight_p`
- `LemmaParamCell`
- `LemmaParam_p`

### Macros And Constants

- `InferenceWeightCellAlloc()`
- `InferenceWeightCellFree(junk)`
- `InferenceWeightsFree(junk)`
- `LEMMA_ACT_PM_W`
- `LEMMA_ACT_SIMPL_W`
- `LEMMA_HORN_BONUS_W`
- `LEMMA_O_GEN_W`
- `LEMMA_PAS_SIMPL_W`
- `LEMMA_PROOF_DAG_W`
- `LEMMA_PROOF_TREE_W`
- `LEMMA_SIZE_BASE_W`
- `LEMMA_TREE_BASE_W`
- `LemmaParamCellAlloc()`
- `LemmaParamCellFree(junk)`
- `LemmaParamFree(cell)`
- `PCL_LEMMAS`

### Globals

- None found in the source scan.

### Exported Functions

- `(InferenceWeight_p)SizeMalloc(sizeof(InferenceWeightType)) SizeFree(junk, sizeof(InferenceWeightType)) InferenceWeight_p InferenceWeightsAlloc(void)`
- `LemmaParam_p LemmaParamAlloc(void)`
- `PCLStep_p PCLProtComputeLemmaWeights(PCLProt_p prot, LemmaParam_p params)`
- `float PCLStepComputeLemmaWeight(PCLProt_p prot, PCLStep_p step, LemmaParam_p params)`
- `int PCLStepLemmaCmp(PCLStep_p step1, PCLStep_p step2)`
- `int PCLStepLemmaCmpWrapper(const void* s1, const void* s2)`
- `long PCLExprProofSize(PCLProt_p prot, PCLExpr_p expr, InferenceWeight_p iw, bool use_lemmas)`
- `long PCLProtFlatFindLemmas(PCLProt_p prot, LemmaParam_p params, InferenceWeight_p iw, long max_number, float quality_limit)`
- `long PCLProtRecFindLemmas(PCLProt_p prot, LemmaParam_p params, InferenceWeight_p iw, long max_number, float quality_limit)`
- `long PCLProtSeqFindLemmas(PCLProt_p prot, LemmaParam_p params, InferenceWeight_p iw, long max_number, float quality_limit)`
- `long PCLStepProofSize(PCLProt_p prot, PCLStep_p step, InferenceWeight_p iw, bool use_lemmas)`
- `void PCLExprUpdateRefs(PCLProt_p prot, PCLExpr_p expr)`
- `void PCLProtComputeProofSize(PCLProt_p prot, InferenceWeight_p iw, bool use_lemmas)`
- `void PCLProtUpdateRefs(PCLProt_p prot)`
- `void PCLStepUpdateRefs(PCLProt_p prot, PCLStep_p step)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `LemmaParamAlloc`: Allocate an initialized parameter block for the lemma detection algorithm.
- `InferenceWeightsAlloc`: Allocate an inference weight parameter data structure, initialized with default values.
- `PCLExprUpdateRefs`: Given a PCL expression, update the counter in all leaves according to the inferences they directly paricipate in.
- `PCLStepUpdateRefs`: Update reference counters from this step to its parents.
- `PCLProtUpdateRefs`: For all steps in prot update the reference counters
- `PCLStepLemmaCmpWrapper`: Wrapper for PCLStepLemmaCmp in IntOrP's
- `PCLStepLemmaCmp`: Compare the lemma rating of two PCL steps, returning -1, 0, 1 depending on outcome.
- `PCLExprProofSize`: Compute the proof size of the expression (including proofs for children). Assumes that all previous steps already have correct weight.
- `PCLStepProofSize`: Compute and return the proof size of step. Caches result in the step. If use_lemmas is true, always return 0
- `PCLProtComputeProofSize`: Compute proof weight for all steps. If use_lemmas is true, assume proof weight of lemmas is 0 (but still record it).
- `PCLStepComputeLemmaWeight`: Compute the lemma quality of a PCL step based on the information stored in it.
- `PCLProtComputeLemmaWeights`: Compute the lemma rating for all steps. Return the step with the best lemma rating.
- `PCLProtSeqFindLemmas`: Mark all lemmas in procol which have a lemma rating of at least quality_limit, but not more than max_number. Goes from first to last step, taking already marked lemmas into account. Assumes topologically ordered protocol (otherwise lemma ratings might be off). Returns number of lemmas found.
- `PCLProtRecFindLemmas`: Recursively mark lemmas in prot as follows: Find the globally best one, mark it. Recalculate all weight. Repeat. Return number of lemmas found. Terminate if max_number lemmas have been found or quality drops below quality limit.
- `PCLProtFlatFindLemmas`: Find lemmas by computing scores once, sorting (by score) and picking the best lemmas (down to quality_limit). Returns number of lemmas selected.

### Dependencies

- `"pcl_lemmas.h"`
- `<pcl_protocol.h>`

### Compile-Time Conditions

- `PCL_LEMMAS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_lemmas.h`, `PCL2/pcl_lemmas.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 809 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 15 structured function-comment blocks.
- Definition for dealing with lemmas in PCL protocols. the GNU Lesser General Public License. <1> Sun Jun 15 22:47:43 CEST 2003 New
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
