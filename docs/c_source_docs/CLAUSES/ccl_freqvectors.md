<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_freqvectors

## Source Files

- [CLAUSES/ccl_freqvectors.h](../../../eprover/CLAUSES/ccl_freqvectors.h)
- [CLAUSES/ccl_freqvectors.c](../../../eprover/CLAUSES/ccl_freqvectors.c)

## Purpose

Functions for handling frequency count vectors and permutation vectors. 2003-2018 by the author. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FVCollectCell`
- `FVCollect_p`
- `FVIndexType`
- `FVPackedClause_p`
- `FreqVectorCell`
- `FreqVector_p`
- `PermVector_p`
- `Tuple3Cell`

### Macros And Constants

- `CCL_FREQVECTORS`
- `FVACCompatSize(size)`
- `FVCollectCellAlloc()`
- `FVCollectCellFree(junk)`
- `FVFullSize(size)`
- `FVINDEX_MAX_FEATURES_DEFAULT`
- `FVINDEX_SYMBOL_SLACK_DEFAULT`
- `FVPackedClauseFree(junk)`
- `FVSSCompatSize(size)`
- `FVSize(size, features)`
- `FV_CLAUSE_FEATURES`
- `FreqVectorCellAlloc()`
- `FreqVectorCellFree(junk)`
- `FreqVectorFree(junk)`
- `FreqVectorSub(dest, s1, s2)`
- `PermVectorAlloc(size)`
- `PermVectorCopy(vec)`
- `PermVectorFree(junk)`
- `PermVectorPrint(out,vec)`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p FVUnpackClause(FVPackedClause_p pack)`
- `FVCollect_p BillFeaturesCollectAlloc(Sig_p sig, long len)`
- `FVCollect_p BillPlusFeaturesCollectAlloc(Sig_p sig, long len)`
- `FVCollect_p FVCollectAlloc(FVIndexType features, bool use_litcount, long ass_vec_len, long res_vec_len, long pos_count_base, long pos_count_offset, long pos_count_mod, long neg_count_base, long neg_count_offset, long neg_count_mod, long pos_depth_base, long pos_depth_offset, long pos_depth_mod, long neg_depth_base, long neg_depth_offset, long neg_depth_mod)`
- `FVPackedClause_p FVPackClause(Clause_p clause, PermVector_p perm, FVCollect_p cspec)`
- `FreqVector_p FVCollectFreqVectorCompute(Clause_p clause, FVCollect_p cspec)`
- `FreqVector_p OptimizedVarFreqVectorCompute(Clause_p clause, PermVector_p perm, FVCollect_p cspec)`
- `FreqVector_p VarFreqVectorCompute(Clause_p clause, FVCollect_p cspec)`
- `PERF_CTR_DECL(FreqVecTimer)`
- `PermVector_p PermVectorComputeInternal(FreqVector_p fmax, FreqVector_p fmin, FreqVector_p sums, long max_len, bool eliminate_uninformative)`
- `void FVCollectFree(FVCollect_p junk)`
- `void FVCollectInit(FVCollect_p handle, FVIndexType features, bool use_litcount, long ass_vec_len, long res_vec_len, long pos_count_base, long pos_count_offset, long pos_count_mod, long neg_count_base, long neg_count_offset, long neg_count_mod, long pos_depth_base, long pos_depth_offset, long pos_depth_mod, long neg_depth_base, long neg_depth_offset, long neg_depth_mod)`
- `void FVPackedClauseFreeReal(FVPackedClause_p pack)`
- `void FreqVectorAdd(FreqVector_p dest, FreqVector_p s1, FreqVector_p s2)`
- `void FreqVectorFreeReal(FreqVector_p junk)`
- `void FreqVectorInitialize(FreqVector_p vec, long value)`
- `void FreqVectorMax(FreqVector_p dest, FreqVector_p s1, FreqVector_p s2)`
- `void FreqVectorMin(FreqVector_p dest, FreqVector_p s1, FreqVector_p s2)`
- `void FreqVectorMulAdd(FreqVector_p dest, FreqVector_p s1, long f1, FreqVector_p s2, long f2)`
- `void FreqVectorPrint(FILE* out, FreqVector_p vec)`
- `void VarFreqVectorAddVals(FreqVector_p vec, long symbols, FVIndexType features, Clause_p clause)`

## Implementation Notes

### Internal Functions

- `gather_feature_vec`

### Source-Level Behavior

- `tuple_3_compare_23lex`: Compare 2 tuple-2 cells lexicographically, with diff more significant than value, which is more significant than pos.
- `gather_feature_vec`: Gather a feature from a full feature vector according to cspec.
- `PermVectorComputeInternal`: Find a "good" permutation (and selection) vector for FVIndexing by: - Ordering features from lesser to higher informativity - Selecting the best max_len features - Optionally drop features that have no projected informational value.
- `FreqVectorAlloc`: Allocate a frequency vector that can hold up to sig_start non function symbol count features and sig_count function symbol counts (in both positive and negative variety).
- `FreqVectorFree`: Free a frequency vector.
- `FreqVectorInitialize`: Store value in all fields of vec.
- `FreqVectorPrint`: Print a frequency vector.
- `VarFreqVectorAddVals`: Add values for up to symbol type features to the freq vector.
- `VarFreqVectorCompute`: Allocate and return a frequency vector for clause based on the other supplied parameters.
- `OptimizedVarFreqVectorCompute`: Compute an "optimized" frequency count vector, based on a given permutation vector. If no permutation vector is given, return a VarFreqVector.
- `FVCollectInit`: Initialize an FVCollectCell.
- `FVCollectAlloc`: Allocate an initialized FVCollectCell.
- `FVCollectFree`: Free a FVCollectCell.
- `FVCollectFreqVectorCompute`: Compute a Feature Vector for the clause based on cspec.
- `BillFeaturesCollectAlloc`: Generate a CollectSpec as follows - positive literals - negative literals foreach relation symbol positive occurrences negative occurrences foreach function symbol positive occurrences negative occurrences positive maxdepth negative maxdepth
- `BillPlusFeaturesCollectAlloc`: Generate a CollectSpec as follows - positive literals - negative literals foreach relation symbol positive occurrences negative occurrences foreach function symbol positive occurrences negative occurrences positive maxdepth negative maxdepth All overflow counts All overflow depths
- `FVPackClause`: If index is an index, compute and return a StandardFreqVector for clause, otherwise pack clause into a dummy frequency vector cell and return than.
- `FVUnpackClause`: Unpack a packed clause, i.e. return the clause and throw away the container.
- `FVPackedClauseFreeReal`: Fully free a packed clause.
- `FreqVectorAdd`: Component-wise addition of both sources. Guaranteed to work if dest is a source (but not maximally efficient - who cares). Yes, it's worth mentioning it ;-)
- `FreqVectorMulAdd`: Component-wise addition of both weighted sources. Guaranteed to work if dest is a source (but not maximally efficient - who cares). Yes, it's worth mentioning it ;-)
- `FreqVectorMax`: Compute componentwise max of vectors. See above.
- `FreqVectorMin`: Compute componentwise min of vectors. See above.

### Dependencies

- `"ccl_freqvectors.h"`
- `<ccl_clauses.h>`
- `<clb_fixdarrays.h>`
- `<clb_pdarrays.h>`
- `<clb_regmem.h>`

### Compile-Time Conditions

- `CCL_FREQVECTORS`
- `NDEBUG`

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

Source files reviewed: `CLAUSES/ccl_freqvectors.h`, `CLAUSES/ccl_freqvectors.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1302 lines, 30 scanned public declarations, 1 scanned internal function definitions, and 23 structured function-comment blocks.
- Functions for handling frequency count vectors and permutation vectors. 2003-2018 by the author. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
