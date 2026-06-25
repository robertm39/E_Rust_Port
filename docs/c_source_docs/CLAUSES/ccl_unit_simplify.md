<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_unit_simplify

## Source Files

- [CLAUSES/ccl_unit_simplify.h](../../../eprover/CLAUSES/ccl_unit_simplify.h)
- [CLAUSES/ccl_unit_simplify.c](../../../eprover/CLAUSES/ccl_unit_simplify.c)

## Purpose

Functions and datatypes for performing unit-cuts and unit-simplifications with a mixed clause set where units are indexed. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `UnitSimplifyType`

### Macros And Constants

- `CCL_UNIT_SIMPLIFY`
- `SimplifyFailed(res)`
- `TransUnitSimplifyString(str)`

### Globals

- `extern char* UnitSimplifyNames[]`

### Exported Functions

- `ClausePos_p FindSignedTopSimplifyingUnit(ClauseSet_p units, Term_p t1, Term_p t2, bool sign)`
- `ClausePos_p FindSimplifyingUnit(ClauseSet_p set, Term_p t1, Term_p t2, bool positive_only)`
- `ClausePos_p FindTopSimplifyingUnit(ClauseSet_p units, Term_p t1, Term_p t2)`
- `bool ClauseSimplifyWithUnitSet(Clause_p clause, ClauseSet_p unit_set, UnitSimplifyType how)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FindTopSimplifyingUnit`: Find a unit s=t (or s!=t) in units such that sigma(s)=t1 and sigma(t)=t2 for some sigma.
- `FindSignedTopSimplifyingUnit`: Find a unit s=t (or s!=t) in units such that sigma(s)=t1 and sigma(t)=t2 for some sigma. Return only clauses with sign sign.
- `FindSimplifyingUnit`: Return a unit clause with from set that can simplify or subsume t1=t2.
- `ClauseSimplifyWithUnitSet`: Simplify a clause with the (indexed) units from set. Performs simplify-reflect and subsumption steps. simplify-reflect is controlled by the value of how. If clause is subsumed by a unit, return false, otherwise return true.

### Dependencies

- `"ccl_unit_simplify.h"`
- `<ccl_clausefunc.h>`
- `<ccl_clausepos.h>`

### Compile-Time Conditions

- `CCL_UNIT_SIMPLIFY`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_unit_simplify.h`, `CLAUSES/ccl_unit_simplify.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 383 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Functions and datatypes for performing unit-cuts and unit-simplifications with a mixed clause set where units are indexed. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ClauseSimplifyWithUnitSet` first tries top-level lookup for `TopLevelUnitSimplify`; `FullUnitSimplify` calls `FindSimplifyingUnit`, which tries top-level units of either sign, then descends through exactly one differing argument pair and only accepts positive units for deeper simplification.
- Opposite-signed unit simplification removes the current literal, clears only `CPLimitedRW`, and keeps iterating at the same list handle. Same-signed simplification returns `false` immediately to signal subsumption, optionally marks the unit `CPIsProtected` for equal standard weight, and does not remove the target literal.
- Change-later candidate: same-signed subsumption calls `ClauseSetProp(pos->clause, ClauseQueryProp(clause, CPIsSOS))`. Since `ClauseQueryProp` returns boolean `0` or `1`, an SOS target marks the unit `CPInitial` instead of `CPIsSOS`. Preserve this until proof-search reference tests can decide whether it is relied on.
- `FindSimplifyingUnit` has a higher-order-specific early return after finding a descended positive unit, but the first-order path returns the same result immediately after the loop condition observes success. Treat that branch as redundant unless later higher-order indexed matching gives it a distinct effect.
- Rust currently ports a plain-set lookup for this behavior. The C unit relies on `unit_set->demod_index`, `PDTreeSearchInit`, and live `ClausePos` results, so indexed demodulator integration remains a future ownership task.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
