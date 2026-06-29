<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_gd_transformation

## Source Files

- [CLAUSES/ccl_gd_transformation.h](../../../eprover/CLAUSES/ccl_gd_transformation.h)
- [CLAUSES/ccl_gd_transformation.c](../../../eprover/CLAUSES/ccl_gd_transformation.c)

## Purpose

Definitions for function implementing a TWEE-style direct goal transformation (by adding equational definitions that reduce goal ground terms to (usually new) constants. This goes from clause level to signature level - I put it together here to keep things under control...

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_GD_TRANSFORMATION`

### Globals

- None found in the source scan.

### Exported Functions

- `long ClauseSetGDTransform(TB_p terms, ClauseSet_p clauses, bool add_goal_defs_pos, bool add_goal_defs_neg, bool add_goal_defs_subterms)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `gd_def_nf`: Compute the normal-form of term with respect to defs.
- `gd_term_rek_define`: Conditionally (if it does not already exist) add a definiton for term -> New Constant. Definitions are stored in defs, with cell->key being the entry_no of the LHS, and cell->val1.pval pointing to the RHS. The defining clause is added to clauses.
- `ClauseSetGDTransform`: Perform a Twee-style goal-direct transformation, by adding equations (unit clauses) that will reduce some or all ground subterms from conjecture clauses to fresh constants. Returns the number of new defintions introduced.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_gd_transformation.h>`

### Compile-Time Conditions

- `CCL_GD_TRANSFORMATION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_gd_transformation.h`, `CLAUSES/ccl_gd_transformation.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 306 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Definitions for function implementing a TWEE-style direct goal transformation (by adding equational definitions that reduce goal ground terms to (usually new) constants. This goes from clause level to signature level - I put it together here to keep things under control...
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/clauses/gd_transformation.rs` ports the clause-level goal-definition transform over the represented Rust `ClauseSet`, including conjecture-clause filtering, selected positive/negative literal collection, non-constant ground-term definition, recursive subterm definition mode, definition normal forms through already introduced definitions, fresh typed `edef` constant creation, positive unit equality clauses, and `DCIntroDef` derivation entries.
- Supported executable prune/proof-search paths now apply `--goal-defs` and `--goal-subterm-defs` after represented SInE/relevance pruning and BCE, and before initial clause documentation, watchlist loading, proof-control initialization, or saturation. Generated definitions are inserted into the represented axiom set, so initial docs and the saturation initial-clause count can observe them.
- Full formula-owner preprocessing and exact integration with later predicate elimination remain pending; the current call site matches the C ordering relative to BCE and the currently unported predicate-elimination pass by running after BCE.

### Change-Later Observations

- C traverses collected goal terms through a `PTree`, so generated definition names and insertion order can depend on raw pointer order. Rust reuses the existing deterministic `BTreeMap` keyed by term identity for ground-term collection, which is stable inside the Rust port but may differ from C definition order. Preserve this until reference traces decide whether exact pointer-order compatibility matters.
- C keys the definition table by term-bank `entry_no` and can define a normalized left-hand side that is already an introduced definition constant if an original compound term normalizes to it. Rust mirrors that entry-number map and the resulting behavior, even though a later cleaned transform might avoid constant-to-constant follow-up definitions.
- `ClauseSetGDTransform` mutates the same clause set that it scans, but only after collecting all candidate terms. Rust keeps the same two-phase shape to avoid borrowing aliases; a future proof-state owner with stable clause handles should keep that mutation boundary explicit.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
