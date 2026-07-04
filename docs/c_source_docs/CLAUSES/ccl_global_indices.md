<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_global_indices

## Source Files

- [CLAUSES/ccl_global_indices.h](../../../eprover/CLAUSES/ccl_global_indices.h)
- [CLAUSES/ccl_global_indices.c](../../../eprover/CLAUSES/ccl_global_indices.c)

## Purpose

Code abstracting several (optional) indices into one structure. the GNU Lesser General Public License. <1> Fri May 7 21:13:39 CEST 2010 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GlobalIndices`
- `GlobalIndices_p`

### Macros And Constants

- `CCL_GLOBAL_INDICES`
- `GetExtFromIdx(g)`
- `GetExtIntoIdx(g)`
- `GetExtMaxDepth(g)`
- `SetExtFromIdx(g, v)`
- `SetExtIntoIdx(g, v)`
- `SetExtMaxDepth(g, v)`

### Globals

- None found in the source scan.

### Exported Functions

- `PERF_CTR_DECL(BWRWIndexTimer)`
- `PERF_CTR_DECL(PMIndexTimer)`
- `void GlobalIndicesDeleteClause(GlobalIndices_p indices, Clause_p clause, bool lambda_demod)`
- `void GlobalIndicesFreeIndices(GlobalIndices_p indices)`
- `void GlobalIndicesInit(GlobalIndices_p indices, Sig_p sig, char* rw_bw_index_type, char* pm_from_index_type, char* pm_into_index_type, int ext_rules_max_depth)`
- `void GlobalIndicesInsertClause(GlobalIndices_p indices, Clause_p clause, bool lambda_demod)`
- `void GlobalIndicesInsertClauseSet(GlobalIndices_p indices, ClauseSet_p set, bool lambda_demod)`
- `void GlobalIndicesNull(GlobalIndices_p indices)`
- `void GlobalIndicesReset(GlobalIndices_p indices)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `GlobalIndicesNull`: Set the global indices to NULL.
- `GlobalIndicesInit`: Initialize the global indices as required by the parameters.
- `GlobalIndicesFreeIndices`: Free the existing indices.
- `GlobalIndicesReset`: Reset all exisiting indices.
- `GlobalIndicesInsertClause`: Add a clause to all exisiting global indices.
- `GlobalIndicesDeleteClause`: Remove a clause from all exisiting global indices.
- `GlobalIndicesInsertClauseSet`: Insert all clause in set into the indices.

### Dependencies

- `"ccl_global_indices.h"`
- `<ccl_clausesets.h>`
- `<ccl_ext_index.h>`
- `<ccl_overlap_index.h>`
- `<ccl_subterm_index.h>`

### Compile-Time Conditions

- `CCL_GLOBAL_INDICES`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_global_indices.h`, `CLAUSES/ccl_global_indices.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 453 lines, 13 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Code abstracting several (optional) indices into one structure. the GNU Lesser General Public License. <1> Fri May 7 21:13:39 CEST 2010 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- Rust currently ports the `GlobalIndicesNull`/`GlobalIndicesInit`/`GlobalIndicesFreeIndices`/`GlobalIndicesReset` shape, the backward-rewrite subterm-index path, the paramodulation overlap from/into/negative-atom index paths, and the higher-order-gated LFHO extension into/from index paths.
- `GlobalIndicesInit` stores `pm_negp_index_type` from `pm_into_index_type`, not from a separate argument. Rust mirrors that derived field.
- `GlobalIndicesInit` asserts that the process-global `problemType` is initialized, then allocates extension indexes only when it is `PROBLEM_HO`. Rust exposes the same gate through an explicit `ProblemType` initializer instead of reading global state at this boundary, and the supported executable bridge now passes the parsed problem type when constructing caller-owned proof-search indexes.
- `GlobalIndicesInsertClause` marks the clause `CPIsGlobalIndexed` before inserting into optional indexes. `GlobalIndicesDeleteClause` clears the bit before deleting from optional indexes. Rust preserves that mutation order.
- `GlobalIndicesInsertClause` calls `OverlapIndexInsertIntoClause2` when `pm_into_index` exists, so the matching negative-atom index is expected to exist too. Rust preserves that invariant with a paired `pm_negp_index` allocation and assertion.
- Extension index insertion runs after backward-rewrite and PM indexes, applies the configured max-depth gate inside `ExtIndexInsert*Clause`, and deletes without a depth gate. Rust preserves that call order and gating.
- `GlobalIndicesInsertClauseSet` returns immediately if `bw_rw_index` is null, so a PM-only configuration would not mark or insert the set through this helper. Rust preserves that no-op gate.
- Rust's optional `print-index-stats` Cargo feature exposes C-shaped distribution lines for the four proof-search global indexes and the `pm_from_index` DOT graph over the caller-owned executable global-index bridge.

### Change Later

- `GlobalIndicesReset` frees and reinitializes indexes but does not clear `CPIsGlobalIndexed` on any clauses; C callers reset after freeing clause sets. Rust mirrors the index reset and should keep clause-flag cleanup explicit if reset is ever exposed with live clauses.
- Global indices in C store raw pointers to optional subterm, overlap, and extension indexes against the proof-state signature. Rust uses a borrowed-signature shell for now; later proof-state integration should avoid self-referential ownership, likely by moving the signature behind an explicit shared proof-session handle.
- Indexed clause occurrences are keyed by live clause identity in C, so delete must happen before a clause is extracted, archived, or moved out of its owning set. Rust's caller-owned wrappers preserve that ordering; a future stable-handle index can make the lifecycle less pointer-shaped after compatibility is secured.
- Rust global-index clause insert/delete take an explicit `&TermBank` so the overlap split helper can distinguish equational literals until equations have a typed owner-bank back-pointer.
- C uses process-global `problemType` during initialization, so the same argument list can allocate different index sets depending on earlier parser/control state. Rust's explicit `ProblemType` initializer is easier to audit; keep state-owned proof-session construction responsible for passing the C-equivalent value when caller-owned executable indexes are replaced.
<!-- END MANUAL REVIEW: c_source_docs -->
