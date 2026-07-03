<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_specsigfeatures

## Source Files

- [HEURISTICS/che_specsigfeatures.h](../../../eprover/HEURISTICS/che_specsigfeatures.h)
- [HEURISTICS/che_specsigfeatures.c](../../../eprover/HEURISTICS/che_specsigfeatures.c)

## Purpose

Definitions for determining various features of specifications, i.e. clause and (later) formula sets. This is analoguous to che_clausesetfeatures.[ch], but uses different features. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `SpecSigFeatureCell`
- `SpecSigFeature_p`

### Macros And Constants

- `CHE_SPECSIGFEATURES`
- `EqnCollectSigFeatures(eqn, features)`
- `SPECSIG_AX_FTRS`
- `SPECSIG_AX_NEGEQ`
- `SPECSIG_AX_POSEQ`
- `SPECSIG_AX_SYMD`
- `SPECSIG_AX_SYMD_NEG`
- `SPECSIG_AX_SYMD_POS`
- `SPECSIG_CJ_FTRS`
- `SPECSIG_CJ_NEGEQ`
- `SPECSIG_CJ_POSEQ`
- `SPECSIG_CJ_SYMD`
- `SPECSIG_CJ_SYMD_NEG`
- `SPECSIG_CJ_SYMD_POS`
- `SPECSIG_CS_FTRS`
- `SPECSIG_GLOBAL_FTRS`
- `SPECSIG_GLOBAL_GNRL`
- `SPECSIG_GLOBAL_HORN`
- `SPECSIG_GLOBAL_SIG`
- `SPECSIG_GLOBAL_UNIT`
- `SPECSIG_NEG_EL_OFFSET`
- `SPECSIG_POS_EL_OFFSET`
- `SPECSIG_SIGFTRS`
- `SPECSIG_SYMD_OFFSET`
- `SPECSIG_TOTAL_FTR_NO`
- `SpecSigFeatureCellAlloc()`
- `SpecSigFeatureCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(SpecSigFeatureCell*)SizeMalloc(sizeof(SpecSigFeatureCell)) void SpecSigFeatureInit(SpecSigFeature_p specftrs)`
- `TermCollectSigFeatures((eqn)->bank->sig, (eqn->lterm), (features));\ TermCollectSigFeatures((eqn)->bank->sig, (eqn->rterm), (features)) void ClauseCollectSigFeatures(Clause_p clause, long* features)`
- `void ClauseComputeSigFeatures(Clause_p clause, long* features)`
- `void ClauseSetCollectSigFeatures(Sig_p sig, ClauseSet_p set, SpecSigFeature_p specftrs)`
- `void SpecSigFeaturePrint(FILE*out, SpecSigFeature_p specftrs)`
- `void TermCollectSigFeatures(Sig_p sig, Term_p term, long* features)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `term_collect_sig_features_rek`: Collect information of number and depth of occurrence of function symbols of different arity in term.
- `SpecSigFeatureInit`: Initialize (set to 0) all features. Could use memset(), but this is more transparent and non critical...
- `TermCollectSigFeatures`: Collect information of number and depth of occurrence of function symbols of different arity in term.
- `ClauseCollectSigFeatures`: Collect positive and negative signature features (distribution of arities) for the clause. Structure of features (L = SIG_FEATURE_ARITY_LIMIT) features[0]: Number of positive equational literals features[1]: Number of negative equational literals For positive literals: features[...2+L]: Frequency of of pred-symbols of arity n features[...2+2L]: Frequency of...
- `ClauseComputeSigFeatures`: Compute the signature-based features of the clause. As above, but zeros out the result vector first.

### Dependencies

- `"che_specsigfeatures.h"`
- `<ccl_proofstate.h>`

### Compile-Time Conditions

- `CHE_SPECSIGFEATURES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_specsigfeatures.h`, `HEURISTICS/che_specsigfeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 403 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Definitions for determining various features of specifications, i.e. clause and (later) formula sets. This is analoguous to che_clausesetfeatures.[ch], but uses different features. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- The file-level comment says this unit handles clause and "later" formula sets, but this checkout exports only term, clause, and `ClauseSetCollectSigFeatures` collectors..

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Add formula-set signature-vector collection only as a cleaned extension or if a C reference version with formula support is introduced; do not infer it from the comment alone.

<!-- END MANUAL REVIEW: c_source_docs -->
