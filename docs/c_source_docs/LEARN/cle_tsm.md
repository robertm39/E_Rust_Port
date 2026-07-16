<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_tsm

## Source Files

- [LEARN/cle_tsm.h](../../../eprover/LEARN/cle_tsm.h)
- [LEARN/cle_tsm.c](../../../eprover/LEARN/cle_tsm.c)

## Purpose

Finally, the term space map! the GNU Lesser General Public License.

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TSACell`
- `TSA_p`
- `TSMAdminCell`
- `TSMAdmin_p`
- `TSMCell`
- `TSMType`
- `TSM_p`

### Macros And Constants

- `CLE_TSM`
- `GetTSMType(name)`
- `TSACellAlloc()`
- `TSACellFree(junk)`
- `TSMAdminCellAlloc()`
- `TSMAdminCellFree(junk)`
- `TSMCellAlloc()`
- `TSMCellFree(junk)`
- `TSMEvalNormalize(eval, limit)`
- `TSM_MAX_TERMTOP`

### Globals

- `extern char* TSMTypeNames[]`

### Exported Functions

- `IndexType TSMFindOptimalIndex(TSMAdmin_p admin, FlatAnnoSet_p set, long *depth, IndexType indextype, double limit)`
- `TSA_p TSACreate(TSMAdmin_p admin, FlatAnnoTerm_p list)`
- `TSMAdmin_p TSMAdminAlloc(Sig_p sig, TSMType type)`
- `TSM_p TSMCreate(TSMAdmin_p admin, FlatAnnoSet_p set)`
- `double TSMComputeAverageEval(TSMAdmin_p admin, FlatAnnoSet_p set)`
- `double TSMComputeClassificationLimit(TSMAdmin_p admin, FlatAnnoSet_p set)`
- `double TSMEvalTerm(TSMAdmin_p admin, Term_p term, PatternSubst_p subst)`
- `double TSMFindPartLimit(FlatAnnoSet_p set, double part)`
- `double TSMFlatAnnoSetEntropy(FlatAnnoSet_p set, double limit)`
- `double TSMRemainderEntropy(PDArray_p partition, long *parts, double limit, long max_index)`
- `long TSMCreateSubtermSet(FlatAnnoSet_p set, FlatAnnoTerm_p list, int sel)`
- `long TSMPartitionSet(PDArray_p partition, TSMIndex_p index, FlatAnnoSet_p set, PDArray_p cache)`
- `void TSAFree(TSA_p tsa)`
- `void TSMAdminBuildTSM(TSMAdmin_p admin, FlatAnnoSet_p set, IndexType type, int depth, PatternSubst_p subst)`
- `void TSMAdminFree(TSMAdmin_p junk)`
- `void TSMFree(TSM_p tsm)`
- `void TSMPrintFlat(FILE* out, TSM_p tsm)`
- `void TSMPrintRek(FILE* out, TSMAdmin_p admin, TSM_p tsm, int depth)`

## Implementation Notes

### Internal Functions

- `compute_list_entropy`
- `dist_combi_entropy`
- `distribution_entropy`
- `evaluate_index`
- `evaluate_index_desc`
- `evaluate_top_index`
- `tsm_rec_eval`
- `tsm_rec_eval_no_weight`
- `tsmbasealloc`
- `tsmcomplete`

### Source-Level Behavior

- `dist_combi_entropy`: Compute the entropy of a class/partition distribution.
- `distribution_entropy`: Compute the entropy of a distribution.
- `evaluate_index`: Given an index and a set, return the relative information gain from this index.
- `evaluate_index_desc`: Given an index description, return the relative information gain from this index.
- `evaluate_top_index`: Evlauate all termtop index functions described by indextype. If one of them beats to_beat, set *best_type to the value of the best index function and return its relative information gain.
- `compute_list_entropy`: Return the entropy of list (and the length in *count).
- `tsm_rec_eval`: Recursivly evaluate a term with a tsm. Return the weighted sum of all found evaluations in *res and the weight directly.
- `tsm_rec_eval_no_weight`: Recursivly evaluate a term with a tsm. Return the sum of all found evaluations in *res and the number of matched nodes directly.
- `tsmbasealloc`: Return a tsm with admin and index set.
- `tsmcomplete`: Complete a base tsm cell.
- `TSMRemainderEntropy`: Compute the remainder entropy of the pos/neg distinction (defined by the terms evaluation) under the assumption of the partition. *parts is set to the number of non-empty partitions.
- `TSMFlatAnnoSetEntropy`: Compute the entropy of a flat annotation set under the assumption that terms with eval >limit are class one, all other terms are class2.
- `TSMPartitionSet`: Generates a partition by assigning each FlatAnnoterm from set to an element of index(set). Returns largest index. If cache is != 0, use it as a cache (Du!) ;-)
- `TSMFindOptimalIndex`: Find the optimal index (i.e. the one with the largest relative information gain) among those specified. If *depth != 0, try only at that depth.
- `TSMInsertSubtermSet`: Given a list of FlatAnnoTerms(), insert new, non-reference-carrying FlatAnnoterms corresponding to the subterms at position select into set. Return number of elements in new set.
- `TSMAdminAlloc`: Return an initialized TSMAdminCell suitable for building an TSM with.
- `TSMAdminFree`: Free a TSMAdmin data structure
- `TSMAdminBuildTSM`: Given a set of flatly annotated terms, build a TSM.
- `TSMCreate`: Create a TSM according to the specification in admin.
- `TSMFree`: Free a TSM.
- `TSACreate`: Create a TSA from a list of flatly annotated terms.
- `TSAFree`: Free a TSA.
- `TSMEvalTerm`: Return an evaluation of term (as the weighted average evaluation of all TSM nodes selected by term)
- `TSMComputeClassificationLimit`: Evaluate all terms and return the (avgevalpos+avgevalneg)/2.
- `TSMPrintFlat`: Print the tsm's tsa-distribution.
- `TSMPrintRek`: Print a complete TSM

### Dependencies

- `"cle_tsm.h"`
- `<clb_ddarrays.h>`
- `<cle_flatannoterms.h>`
- `<cle_indexfunctions.h>`

### Compile-Time Conditions

- `CLE_TSM`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_tsm.h`, `LEARN/cle_tsm.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 1526 lines, 26 scanned public declarations, 10 scanned internal function definitions, and 26 structured function-comment blocks.
- Term-space map core; preserve indexing and feature-map behavior for learned guidance compatibility.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TSMEvalNormalize(eval, limit)` uses a strict `< limit` comparison; values exactly equal to the limit classify as positive.
- `TSMRemainderEntropy` computes a weighted average over non-empty partition buckets and divides by `global_count` without an empty-partition guard, so an empty partition yields NaN while `parts` remains zero.
- `TSMPartitionSet` assigns `FlatAnnoTerm.next` links as scratch bucket chains. Any Rust implementation that keeps flat annotation terms shared must preserve or isolate this mutation carefully.
- `TSMPartitionSet` prepends each traversed flat annotation term to its bucket, so per-bucket list order is the reverse of `NumTree` traversal for terms with the same key.
- Non-null partition caches store `key + 1` by `term->entry_no`; zero means "not cached". The caller must keep caches aligned with the index shape because a cached entry bypasses `TSMIndexInsert`.
- `evaluate_index` returns zero for a single non-empty partition. For a perfect split, its denominator can become zero and the C double result is positive infinity.
- `TSMFindOptimalIndex` updates the incumbent only on strict `>` gains, so earlier candidates win ties. With `IndexDynamic`, this means arity, symbol, identity, then top variants/depths are considered in C source order.
- If only one concrete arity/symbol/identity index is requested, `TSMFindOptimalIndex` returns it without forcing the output depth to zero; the caller's incoming depth is preserved even though that depth is irrelevant to the selected index.
- `TSMCreateSubtermSet` asserts that every listed term has the selected direct subterm, then inserts borrowed subterms as new flat annotations using the source term's eval, eval weight, and source count.
- `cle_tsm.h` declares `TSMFindPartLimit`, but no implementation appears in this checkout. Treat it as header-only surface until a caller or reference implementation requires it.
- `TSMAdminAlloc` creates `emptytsm` before `admin->subst` is set, so `TSMIndexAlloc(IndexEmpty, ...)` receives a null substitution pointer even though the non-empty index kinds assume stable total substitutions.
- `TSMAdminCell.subst` is shared by every non-empty recursive TSM and by the index terms below it; it is not copied by `tsmbasealloc`. Rust now encodes that lifetime with one `Rc<PatternSubst>` across the admin and recursive indexes. On the 10,000+10,000-term corpus this ownership correction reduced maximum RSS from 175,316 to 44,912 KiB without changing output.
- `TSMAdminAlloc` only initializes `tsmstack`/`cachestack` for `TSMTypeRecurrentLocal`; the non-recurrent-local `cachestack` field is left unused. `TSMAdminBuildTSM` then appends the fixed recurrent-local arity/symbol/top stack and its caches in source order.
- `TSACreate` shares child TSM pointers for flat, recurrent, and recurrent-local modes and allocates owned child TSMs only for recursive mode. `TSAFree` mirrors this by recursively freeing child maps only when `admin->tsmtype == TSMTypeRecursive`.
- `TSMAdminAlloc` creates a private `TBAlloc(sig)` index bank that shares the live signature pointer, and `TSMAdminFree` nulls `index_bank->sig` before freeing that bank. Any cleanup should make this signature/session ownership explicit.
- `tsm_rec_eval_no_weight` calls the weighted `tsm_rec_eval` for matched child nodes and for recurrent unmapped descent, so its unweighted behavior applies only to the current matched node.
- `TSMPrintRek` calls `TSMIndexPrint(stdout, ...)` instead of using its `out` parameter for the index section, and it has no cycle guard for recurrent maps.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
