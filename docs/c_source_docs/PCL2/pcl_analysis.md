<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_analysis

## Source Files

- [PCL2/pcl_analysis.h](../../../eprover/PCL2/pcl_analysis.h)
- [PCL2/pcl_analysis.c](../../../eprover/PCL2/pcl_analysis.c)

## Purpose

Code for analysing PCL protocols, replacing (much of) what used to be in ANALYSIS for old E style proofs. the GNU Lesser General Public License. <1> Tue Feb 3 23:26:44 CET 2004

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `PCLStepUpdateGRefs(prot, step)`
- `PCL_ANALYSIS`

### Globals

- None found in the source scan.

### Exported Functions

- `long PCLExprProofDistance(PCLProt_p prot, PCLExpr_p expr)`
- `long PCLProtSelectExamples(PCLProt_p prot, long neg_examples)`
- `long PCLStepProofDistance(PCLProt_p prot, PCLStep_p step)`
- `void PCLExprUpdateGRefs(PCLProt_p prot, PCLExpr_p expr, bool proofstep)`
- `void PCLProtProofDistance(PCLProt_p prot)`
- `void PCLProtUpdateGRefs(PCLProt_p prot)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `pcl_example_cmp`: Compare two PCL steps as follows: All proof steps are equal and smaller than all non-proof steps. Non-proof steps are compared by gen_ref/(sim_ref+1).
- `PCLExprProofDistance`: Find the longest inference chain from the nearest proof clause referenced in the expression. If no proof clause is among its ancestors, return LONG_MAX. Assumes that proof clauses are marked!
- `PCLStepProofDistance`: Find the longest inference chain from the nearest proof clause referenced in the steps expression (or 0 if step is proof step). If no proof clause is among its ancestors, return LONG_MAX. Assumes that proof clauses are marked! Non-proof initial clauses get PCL_PROOF_DIST_DEFAULT.
- `PCLProtProofDistance`: Compute the proof distance for all steps in protocol. Assumes that proof steps are already identified.
- `PCLExprUpdateGRefs`: Update the reference counters in all parents of expr appropriately.
- `PCLProtUpdateGRefs`: For all steps, mark how often they are used to generate or simplify proof or non-proof clauses. Assumes that proof steps are already identified.
- `PCLProtSelectExamples`: Select examples for pattern-based learning. Selects all proof clauses and up to neg_examples negative examples. Negative examples are selected by ratio of generating to simplifying applications (generating bad, simplification good). Returns number of steps selected.

### Dependencies

- `"pcl_analysis.h"`
- `<pcl_protocol.h>`

### Compile-Time Conditions

- `PCL_ANALYSIS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_analysis.h`, `PCL2/pcl_analysis.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 438 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Code for analysing PCL protocols, replacing (much of) what used to be in ANALYSIS for old E style proofs. the GNU Lesser General Public License. <1> Tue Feb 3 23:26:44 CET 2004
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Initial Rust support is in `src/pcl2/analysis.rs`, covering expression/step/protocol proof-distance computation, cached `proof_distance` updates, generation and simplification reference counter updates by C operator class, proof/non-proof reference counter split, example selection by proof-step priority and useless-generation/useless-simplification ratio, and the C loop behavior for zero negative-example budget.
- The Rust implementation operates over `PclProtocol` full-step ids and safe step lookups while preserving the C analysis counters stored in `PclStepTreeData`.
- Focused compatibility regressions now pin dangling proof-distance diagnostics, silent dangling reference-counter updates, deterministic equal-score selection, C `float` score rounding, and the zero-negative-budget loop. On a 41-step all-ties corpus, the archived glibc C tool and Rust select the same lower-id negative example and produce byte-identical output; [`experiment 062`](../../../experiments/2026-07-16-062-pcl-analysis-edges/FINDINGS.md) records the corpus and decision.

### Change Later

- `PCLExprProofDistance` dereferences the result of `PCLProtFindStep` for quoted parents without a null check; the archived C `direct_examples` tool terminates with `SIGSEGV` on the focused malformed corpus. Rust intentionally reports a syntax diagnostic because reproducing a null dereference is neither a supported output contract nor compatible with safe Rust.
- `PCLExprUpdateGRefs` handles a direct quoted argument whose parent is missing by recursing into the quote expression, which is a no-op. Rust preserves the silent-ignore behavior for reference updates.
- `PCLProtSelectExamples` uses `qsort` over the serialized stack and sets `is_ordered=false`, so equal proof steps and equal negative-example scores have unspecified portable order. Rust retains deterministic PCL-id ordering after the same comparator categories. This matches the archived glibc target byte-for-byte on the 41-step all-ties corpus and avoids making output depend on a particular libc's undocumented sorting implementation.
- `PCLProtSelectExamples` stops immediately when `neg_examples == 0`, so it does not select proof examples despite the comment saying all proof clauses are selected. Rust preserves this loop condition.
- Example selection scores use C `float` ratios of `useless_gen_refs/(useless_simpl_refs+1)`. Rust keeps f32-shaped scoring for compatibility; later learning code may want explicit tie-breaking and overflow/precision policy.
<!-- END MANUAL REVIEW: c_source_docs -->
