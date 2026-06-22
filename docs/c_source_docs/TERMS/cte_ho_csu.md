<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_ho_csu

## Source Files

- [TERMS/cte_ho_csu.h](../../../eprover/TERMS/cte_ho_csu.h)
- [TERMS/cte_ho_csu.c](../../../eprover/TERMS/cte_ho_csu.c)

## Purpose

Interface to algorithm for enumerating (potentially) infinite complete set of unifiers. the GNU Lesser General Public License. <1> do 21 okt 2021 13:40:13 CEST

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CSUIterator_p`
- `CSUIterator_t`
- `Limits_t`
- `StateTag_t`

### Macros And Constants

- `BT_STEP_SIZE`
- `BUILD_CONSTR(c, s)`
- `BURY_KIND`
- `CONSTRAINT_COUNTER(c)`
- `CONSTRAINT_STATE(c)`
- `CSUIterAlloc()`
- `CTE_FULL_UNIF`
- `GET_HEAD_ID(t)`
- `STORE_KIND`

### Globals

- `extern const StateTag_t DECOMPOSED_VAR`

### Exported Functions

- `CSUIterator_p CSUIterInit(Term_p lhs, Term_p rhs, Subst_p subst, TB_p bank)`
- `Subst_p CSUIterGetCurrentSubst(CSUIterator_p iter)`
- `bool NextCSUElement(CSUIterator_p iter)`
- `void CSUIterDestroy(CSUIterator_p iter)`
- `void InitUnifLimits(HeuristicParms_p p)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `dbg_print_state`: Print the state in human-readeable format.
- `whnf_and_prune`: Normalize heads and remove the possible lambda prefixes.
- `build_new_queue`: Builds a copy of the queue that is to be used for backtracking.
- `prepare_backtrack`: Prepare the backtracking state.
- `unroll_fcode`: Go under lamdbas and follow binding pointers until we either hit rigid symbol or there are no more bindings
- `move_stack`: Push one stack to the other one and reset the original one.
- `schedule_args`: Put the arguments on the constraints stack in the order which improves the performance of unif algorithm: first rigid-rigid of different values, then other rigid-rigid and then flex-flex-pairs
- `forward_iter`: After the iterator has successfully been backtracked, try to find the solution.
- `backtrack_iter`: After the call to CSUIterInit or successful call to NextCSUElement, set the state of the iterator so that it is ready to advance to the next iterator. If false is returned, there are no more solutions and the iterator shall be destroyed.
- `NextCSUElement`: Given a (previously initialized) iterator if there exists a next unifier return true and set the substitution of the iterator to the unifier. If there is no unifier, all the variables are unbound and false is returned. When false is returned, CSUIterator is destroyed and is no longer to be used.
- `CSUIterInit`: Given a (previously initialized) iterator if there exists a next unifier return true and set the substitution of the iterator to the unifier. If there is no unifier, all the variables are unbound and false is returned. When false is returned, CSUIterator is destroyed an is no longer to be used.
- `CSUIterGetCurrentSubst`: Returns the substitution stored in the iterator. NB: User needs to take care that substitution is only observed in the correct states.
- `InitUnifLimits`: Store heuristic parameters locally and use them to pick up the limits for unification.
- `CSUIterDestroy`: Destroys the iter and frees all the memory EXCEPT for the initial substitution.

### Dependencies

- `"cte_ho_bindings.h"`
- `"cte_ho_csu.h"`
- `<che_hcb.h>`
- `<cte_fixpoint_unif.h>`
- `<cte_lambda.h>`
- `<cte_pattern_match_mgu.h>`
- `<cte_subst.h>`
- `<cte_termtypes.h>`
- `<stdint.h>`

### Compile-Time Conditions

- `CTE_FULL_UNIF`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_ho_csu.h`, `TERMS/cte_ho_csu.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 727 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 14 structured function-comment blocks.
- Higher-order complete set of unifiers support. Search bounds and binding generation are subtle and performance-sensitive.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
