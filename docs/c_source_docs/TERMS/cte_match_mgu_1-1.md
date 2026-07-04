<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_match_mgu_1-1

## Source Files

- [TERMS/cte_match_mgu_1-1.h](../../../eprover/TERMS/cte_match_mgu_1-1.h)
- [TERMS/cte_match_mgu_1-1.c](../../../eprover/TERMS/cte_match_mgu_1-1.c)

## Purpose

Interface to simple, non-indexed 1-1 match and unification routines on shared terms (and unshared terms with shared variables). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OracleUnifResult`
- `UnifTermSide`
- `UnificationResult`

### Macros And Constants

- `CTE_MATCH_MGU_1_1`
- `FAIL_AND_BREAK(res, val)`
- `MATCH_FAILED`
- `MATCH_SUCC`
- `SubstMatchComplete(t, s, subst)`
- `SubstMguComplete(t, s, subst)`
- `UnifFailed(u_res)`
- `VerifyMatch(matcher, to_match)`

### Globals

- `extern const UnificationResult UNIF_FAILED`
- `extern const UnificationResult UNIF_SUCC`
- `extern long UnifAttempts`
- `extern long UnifSuccesses`

### Exported Functions

- `PERF_CTR_DECL(MguTimer)`
- `UnificationResult SubstComputeMguHO(Term_p t1, Term_p t2, Subst_p subst)`
- `bool OccurCheck(restrict Term_p term, restrict Term_p var)`
- `bool SubstComputeMatch(Term_p matcher, Term_p to_match, Subst_p subst)`
- `bool SubstComputeMgu(Term_p t1, Term_p t2, Subst_p subst)`
- `bool SubstMatchComplete(Term_p t, Term_p s, Subst_p subst)`
- `bool SubstMguComplete(Term_p t, Term_p s, Subst_p subst)`
- `int PartiallyMatchVar(Term_p var_matcher, Term_p to_match, Sig_p sig, bool perform_OccursCheck)`
- `int SubstComputeMatchHO(Term_p matcher, Term_p to_match, Subst_p subst)`

## Implementation Notes

### Internal Functions

- `reorientation_needed`

### Source-Level Behavior

- `reorientation_needed`: Determines whether terms have to be reoriented in HO unification algorithm. Generalizes FO reorientation (rhs var, lhs non-var).
- `OccurCheck`: Occur check for variables, possibly more efficient than the general TermIsSubterm()
- `PartiallyMatchVar`: Given a variable var_matcher, determine the number of arguments of to_match that are actually matched. Performs occur check if needed.
- `SubstComputeMatch`: Try to compute a match from matcher onto to_match and record it in subst. Return true if match exits (in this case subst is changed and needs to be backtracked by the caller), false otherwise (subst is unchanged). Both terms are assumed to contain no bindings except those stored in subst. The routine will work and compute a valid match if the two terms shar...
- `SubstComputeMatchHO`: Generalization of SubstComputeMatch(). Behaves exactly the same, except for the fact that it matches HO terms and can match prefix of to_match. For details, see SubstComputeMatch().
- `SubstComputeMgu`: Compute an mgu between two terms. Currently without any special optimization (double entry checking in the to-solve stack has been deleted as ineficient). Returns true and modifies subst if sucessful, false otherwise (as for match, see above). Terms have to be variable disjoint, otherwise behaviour is unpredictable! Solution with stacks is more efficient th...
- `SubstComputeMguHO`: Generalization of SubstComputeMgu(). Behaves exactly the same, except for the fact that it unifies HO terms and can unify a prefix of either t1 or t2. The number of (possible) remaining arguments is stored in UnificationResult. For other details, see SubstComputeMgu().
- `SubstMatchComplete`: Determines whether pattern matches target so that no arguments remain in the target. If so, it adds bindings to subst and returns true. Otherwise, leaves subst unchanged and returns false.
- `SubstMguComplete`: Determines whether t unifies with s so that no arguments remain in either t or s. If so, it adds bindings to subst and returns true. Otherwise, leaves subst unchanged and returns false.

### Dependencies

- `"clb_plocalstacks.h"`
- `"cte_match_mgu_1-1.h"`
- `"cte_pattern_match_mgu.h"`
- `<clb_os_wrapper.h>`
- `<cte_lambda.h>`
- `<cte_subst.h>`

### Compile-Time Conditions

- `CTE_MATCH_MGU_1_1`
- `ENABLE_LFHO`
- `MEASURE_UNIFICATION`
- `NDEBUG`

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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for higher-order `SubstMguComplete` porting boundaries and applied-variable dereference coverage on 2026-07-04.

Source files reviewed: `TERMS/cte_match_mgu_1-1.h`, `TERMS/cte_match_mgu_1-1.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 936 lines, 17 scanned public declarations, 1 scanned internal function definitions, and 9 structured function-comment blocks.
- First-order matching/MGU routines; variable binding order and occurs-check behavior must match existing callers.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `OccurCheck`, `SubstComputeMgu`, and `VerifyMatch` reach applied free-variable expansion through ordinary dereference/equality helpers. Rust now has regression coverage that the first-order match/MGU boundary follows bound applied free-variable heads instead of treating that as an unported path.
- `MEASURE_UNIFICATION` owns process-global `UnifAttempts`/`UnifSuccesses` counters around `SubstComputeMgu` and `SubstComputeMguHO`. Rust maps the first-order `SubstComputeMgu` side to the non-default `measure-unification` Cargo feature and exposes the same executable statistics lines over those counters.
- In higher-order problem mode, `SubstMguComplete` eta-reduces both inputs, calls `SubstComputeMguHO`, and falls back to the higher-order pattern MGU when both original inputs are non-first-order patterns. Rust now ports that bank-aware complete-MGU dispatch for CSU single-mode callers by taking the owning `TermBank` explicitly, preserving C's same-head different-arity failure behavior instead of falling into the first-order assertion path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `SubstComputeMguHO` reports leftover-argument information through `UnificationResult`, while `SubstMguComplete` collapses that to a boolean after checking no arguments remain. A cleaned API should expose the argument-prefix result type directly at callers that need HO constraints, rather than threading a C-style integer result plus a separate `CheckHOUnificationConstraints` hook.
- The higher-order pattern fallback is guarded by `TermIsNonFOPattern` checks on the unreduced original inputs after the first HO MGU attempt fails. Preserve that order for compatibility until trace tests prove eta-reduced pattern detection is equivalent.
- The C first-order helpers inherit LFHO `TermDeref` binding-cache refreshes when applied-variable dereferencing is active. Rust currently matches the expansion shape through no-cache term handles; add owner-bank/cache behavior only after profiling or trace tests show the cache side effects matter.
- C terms can recover their owning bank through `TermGetBank`, so `SubstMguComplete` can remain a three-argument macro/function while secretly reaching eta-reduction, type-bank sharing, and app-variable prefix construction. Rust currently exposes a bank-aware wrapper for the HO branch; after compatibility is secured, prefer explicit proof/unification-session context over C-style owner-bank recovery.
- C uses signed `long` globals for `UnifAttempts` and `UnifSuccesses`; Rust uses feature-gated atomic signed counters so tests and future threaded callers can read them safely. Revisit the exact overflow and reset story only if compatibility diagnostics depend on very long-running process-global counter behavior.

<!-- END MANUAL REVIEW: c_source_docs -->
