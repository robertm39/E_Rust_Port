<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_context_sr

## Source Files

- [CLAUSES/ccl_context_sr.h](../../../eprover/CLAUSES/ccl_context_sr.h)
- [CLAUSES/ccl_context_sr.c](../../../eprover/CLAUSES/ccl_context_sr.c)

## Purpose

Declarations for functions implementing contextual simplify-reflect (or subsumption resolution in Vampire's terminology). C v L C' v -L v R --------------------- if s(C v L) = C' v L for some subst. s C' v R

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_CONTEXT_SR`

### Globals

- None found in the source scan.

### Exported Functions

- `int ClauseContextualSimplifyReflect(ClauseSet_p set, Clause_p clause)`
- `long ClauseSetFindContextSRClauses(ClauseSet_p set, Clause_p clause, PStack_p res)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ClauseContextualSimplifyReflect`: Perform contextial-simplify-reflect with all clauses in set on clause. Return number of literals deleted.
- `ClauseSetFindContextSRClauses`: Find all clauses in set that can be contextually simplify-reflected ;-) with clause and push them onto res. ATTENTION! A clause that can be simplified in more than one way will be pushed more than once onto the stack! Returns number of clauses pushed.

### Dependencies

- `"ccl_context_sr.h"`
- `<ccl_subsumption.h>`

### Compile-Time Conditions

- `CCL_CONTEXT_SR`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for indexed contextual contraction parity on 2026-07-13 and executable proof-session ownership on 2026-07-17.

Source files reviewed: `CLAUSES/ccl_context_sr.h`, `CLAUSES/ccl_context_sr.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 206 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Declarations for functions implementing contextual simplify-reflect (or subsumption resolution in Vampire's terminology). C v L C' v -L v R --------------------- if s(C v L) = C' v L for some subst. s C' v R
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ClauseContextualSimplifyReflect` first snapshots the clause literals into a stack, sets the cached weight to `ClauseStandardWeight`, then pops literals in stack order. For each literal it flips the sign, sorts by subsumption order, and removes the flipped literal only if the modified clause is subsumed by the set.
- When a contextual subsumer is found, C inherits `CPIsSOS`, clears `CPInitial|CPLimitedRW`, removes the literal, documents the modification, and pushes a `DCContextSR` derivation entry. The Rust plain helper preserves the mutation/property changes and records `DCContextSR` with a compact subsumer reference; an opt-in documenting helper now emits represented `DocClauseModification(inf_context_simplify_reflect, subsumer)` steps for proof-control callers with a `ProofDocSession`.
- `ClauseSetFindContextSRClauses` flips and sorts the query for each literal and pushes every subsumed set clause, including duplicate pushes for the same clause if multiple flipped literals work.
- Rust now exposes mutable-bank contextual simplify-reflect and subsumed-clause discovery variants, and proof control uses them so forward and backward contextual contraction reaches C's complete higher-order matcher. Both directions now route through the owning clause set's feature-vector anchor when present, matching C's automatic `set->fvindex` dispatch; plain standalone sets retain a linear fallback.
- C's sole production mutation call is the forward-contraction path; backward contextual simplify-reflect only discovers and requeues candidates, which are mutated when selected again. Rust now gives the selected-clause `ProcessClause` owner its live `ProofDocSession` when it invokes forward contraction, so an executable proof run emits the `csr` modification before the survivor's `new_given` quote. The focused C/Rust owner evidence is retained in [`experiments/2026-07-17-075-context-sr-doc-owner`](../../../experiments/2026-07-17-075-context-sr-doc-owner/FINDINGS.md).

### Change Later

- C relies on raw `Eqn_p` stack entries remaining valid across literal-list sorting. Rust matches by literal properties and term handles while ignoring the mutable position field; revisit if duplicate literal identity becomes observable outside cleanup-normalized clauses.
- C stores the raw contextual subsumer pointer in the derivation stack. Rust currently records a compact clause reference; replace it with a stable clause handle before proof-object traversal needs the full parent object.
- C couples contextual simplify-reflect proof documentation to the same mutation loop and process-global output/id state. Rust keeps the compatibility behavior behind an explicit session/output wrapper, now selected by the production `ProcessClause` owner whenever a proof session exists. A later unified proof-control output owner would be API cleanup rather than missing executable compatibility behavior.
- C hides indexed-versus-linear behavior behind nullable mutable `ClauseSet` index state. Rust preserves automatic owned-anchor dispatch for compatibility, but a cleaned API should require an explicit query view or stable indexed owner so performance-critical callers cannot accidentally select the linear fallback.
- After a successful deletion, contextual simplify-reflect can query a now-unit target. C's indexed `ClauseSetSubsumesClause` path accepts that query while its plain fallback asserts that the target has more than one literal. Rust preserves this asymmetric contract; later code should make the unit policy uniform instead of encoding it in index availability.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
