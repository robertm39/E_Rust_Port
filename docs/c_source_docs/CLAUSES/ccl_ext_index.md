<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_ext_index

## Source Files

- [CLAUSES/ccl_ext_index.h](../../../eprover/CLAUSES/ccl_ext_index.h)
- [CLAUSES/ccl_ext_index.c](../../../eprover/CLAUSES/ccl_ext_index.c)

## Purpose

A simple index mapping symbols to ClauseTPos trees. See .c file for details on functionality. <1> Thu Jun 3 11:30:36 CEST 2010 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ExtIndex_p`

### Macros And Constants

- `CCL_EXT_DEC_IDX`
- `ExtIdxAlloc()`
- `TYPE_EXT_ELIGIBLE(t)`

### Globals

- None found in the source scan.

### Exported Functions

- `bool TermHasExtEligSubterm(Term_p t)`
- `void CollectExtSupFromPos(Clause_p cl, PStack_p pos_stack)`
- `void CollectExtSupIntoPos(Clause_p cl, PStack_p pos_stack)`
- `void ExtIndexDeleteFromClause(ExtIndex_p into_index, Clause_p cl)`
- `void ExtIndexDeleteIntoClause(ExtIndex_p into_index, Clause_p cl)`
- `void ExtIndexFree(ExtIndex_p into_index)`
- `void ExtIndexInsertFromClause(ExtIndex_p into_index, Clause_p cl, int max_depth)`
- `void ExtIndexInsertIntoClause(ExtIndex_p into_index, Clause_p cl, int max_depth)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `delete_idx`: Delete clause from the index.
- `insert_idx`: Given a clause and a stack containg pairs symbol, compact position insert them into idx.
- `collect_into_pos_term`: Fill the stack with pairs (function symbol, position) eligible for ExtSup inferences. Returns true if t has a functional subterm.
- `build_into_pos_stack`: Insert all positions that are into-targets of ExtSup inference to index.
- `handle_into_idx`: Perform a generic operation on into idx
- `handle_from_idx`: Perform a generic operation on from idx
- `TermHasExtEligSubterm`: Check if a term actually has an eligible subterm for ExtSup
- `ExtIndexInsertIntoClause`: Insert all positions that are into-targets of ExtSup inference to index.
- `ExtIndexDeleteIntoClause`: Delete the clause from into index
- `ExtIndexInsertFromClause`: Insert all positions that are into-targets of ExtSup inference to index.
- `ExtIndexDeleteFromClause`: Delete the clause from into index
- `ExtIndexFree`: Delete the clause from into index
- `CollectExtSupFromPos`: Put pairs (f_code, compact_pos) on pos_stack for all eligible from positions
- `CollectExtSupIntoPos`: Put pairs (f_code, compact_pos) on pos_stack for all eligible into positions

### Dependencies

- `"ccl_ext_index.h"`
- `<ccl_clausecpos.h>`
- `<ccl_clausepos_tree.h>`
- `<ccl_clauses.h>`
- `<clb_intmap.h>`

### Compile-Time Conditions

- `CCL_EXT_DEC_IDX`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_ext_index.h`, `CLAUSES/ccl_ext_index.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 466 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 14 structured function-comment blocks.
- A simple index mapping symbols to ClauseTPos trees. See .c file for details on functionality. <1> Thu Jun 3 11:30:36 CEST 2010 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- `ExtIndex_p` is an `IntMap` from function code to `ClauseTPosTree`; insertion creates the per-symbol tree lazily and deletion removes all positions for the clause under each collected symbol.
- Into-position collection scans every literal and both sides. It does not apply maximality, orientation, or sign gates; a term is pushed only when it is not arrow-typed, not a top-level any variable, not normalized as an applied pattern variable, and has an immediate or descendant Boolean/arrow-typed eligible subterm.
- From-position collection is narrower: it only considers positive literals whose left side is not arrow-typed, then checks the left side at the literal start position and the right side at the offset after the left side.
- The C right-side from-position gate also calls `MAYBE_NORMALIZE_APP_VAR(handle->lterm)`, not the right term. The Rust port preserves that left-term check for compatibility.
- Both insertion and deletion consume the collected `(f_code, compact_pos)` pairs by popping the stack, so observable duplicate-collapsing and tree insertion order are the reverse of collection order.
- Clause insertion is gated by `clause->proof_depth <= max_depth`; deletion has no depth gate.
- `MAYBE_NORMALIZE_APP_VAR` uses the term bank's eta-reducing `NormalizePatternAppVar` result only as a truth value in this index. Rust now computes the same decision when assigning shared-term pattern metadata, so the index skips both already-normalized and eta-normalizable applied pattern variables while still descending into non-pattern applications. The loose-DB eta-redex regression and full validation are retained in [experiment 335](../../../experiments/2026-07-25-034-eta-pattern-metadata/FINDINGS.md).

### Change Later

- C deletion obtains buckets with `IntMapGetRef`, which can create empty symbol slots during a delete. Rust drops empty `BTreeMap` entries; revisit this if storage accounting or debug tree shape needs to be C-identical.
- Extension indexes are allocated from `GlobalIndices` only for higher-order problems in C. Rust now wires them through an explicit problem-type initializer, and the supported executable caller-owned index path passes the parsed problem type; the future state-owned proof-session index owner must preserve that handoff.
<!-- END MANUAL REVIEW: c_source_docs -->
