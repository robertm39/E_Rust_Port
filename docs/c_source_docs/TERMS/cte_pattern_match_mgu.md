<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_pattern_match_mgu

## Source Files

- [TERMS/cte_pattern_match_mgu.h](../../../eprover/TERMS/cte_pattern_match_mgu.h)
- [TERMS/cte_pattern_match_mgu.c](../../../eprover/TERMS/cte_pattern_match_mgu.c)

## Purpose

Interface to simple, non-indexed 1-1 match and unification routines on shared *higher-order pattern* terms. the GNU Lesser General Public License. <1> di 20 jul 2021 9:06:46 UTC

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic, Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTE_PATTERN_UNIF`
- `IS_RIGID(t)`
- `NUM_ACTUAL_ARGS(t)`
- `UNIF_FAIL(res)`

### Globals

- None found in the source scan.

### Exported Functions

- `OracleUnifResult SubstComputeMatchPattern(Term_p matcher, Term_p to_match, Subst_p subst)`
- `OracleUnifResult SubstComputeMguPattern(Term_p t1, Term_p t2, Subst_p subst)`
- `Term_p FreshVarWArgs(TB_p bank, PStack_p args, Type_p ret_ty)`
- `bool PruneLambdaPrefix(TB_p bank, Term_p *t1_ref, Term_p *t2_ref)`

## Implementation Notes

### Internal Functions

- `schedule_jobs`

### Source-Level Behavior

- `db_var_map`: For each DB var which is the argument of a free variable create a corresponding DB var which denotes which argument of the free variable DB var corresponds to. For example: X 0 5 1 2 --> { 0 -> DB(3), 5 -> DB(2), 1 -> DB(1), 2 -> DB(0) }
- `solve_flex_rigid`: Solve flex rigid
- `flex_rigid`: Solve pattern unification problem of the form X s = t, where t has a rigid head.
- `flex_flex_diff`: Solve pattern unification problem of the form X s = X t.
- `flex_flex_same`: Solve pattern unification problem of the form X s = X t.
- `schedule_jobs`: Store the jobs represented by argument pairs to queue, prefering the easier ones first.
- `eta_expand_otf`: Assuming that the first arugment is a lambda and t2 is not, and that the types of t1 and t2 are the same, eta-expand t2 so that it has the same lambda prefix as t1 and then trim the lambda prefix of t2.
- `do_remap`: The actual driver that does the remapping.
- `remap_variables`: Given a matcher applied variable remap bound variables in to_match to match the ones that are arguments of the matcher. If this is not possible return NULL.
- `match_var`: Given an (applied) pattern variable matcher, compute the substitution that binds it to to_match. If no such substitution exists, or to_match is not a pattern, return the corresponding value.
- `PruneLambdaPrefix`: Make sure that terms are eta-expanded enough that they have the lambda-prefix of the same size and then trim this prefix, revealing only the bodies of the terms. References of those trimmed bodies are assigned to arguments t1_ref and t2_ref.
- `SubstComputeMguPattern`: Compute MGU of two terms which might not be patterns. If the terms are not patterns, NOT_IN_FRAGMENT is returned. Otherwise, the the answer to are terms unifiable is returned and subst is extended in the obvious way.
- `SubstComputeMatchPattern`: Computes the matcher of two pattern terms. NB: In HO logic, we cannot use the weight trick as substitution can possibly remove some of variable arguments.
- `FreshVarWArgs`: Make fresh variable applied to args with the appropriate return type.

### Dependencies

- `"cte_pattern_match_mgu.h"`
- `<clb_plocalstacks.h>`
- `<cte_lambda.h>`
- `<cte_match_mgu_1-1.h>`
- `<cte_subst.h>`
- `<cte_termtypes.h>`

### Compile-Time Conditions

- `CTE_PATTERN_UNIF`

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

Source files reviewed: `TERMS/cte_pattern_match_mgu.h`, `TERMS/cte_pattern_match_mgu.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1126 lines, 4 scanned public declarations, 1 scanned internal function definitions, and 14 structured function-comment blocks.
- Pattern matching/unification variant used by higher-order reasoning; keep pattern restrictions explicit.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- Rust now ports the shared `FreshVarWArgs` helper as `fresh_var_with_args` in `src/terms/lambda.rs`. It derives the fresh head type from the supplied argument term types and requested return type, inserts the fresh head through the term bank, and uses the C-shaped `ApplyTerms` path for non-empty arguments.

### Change-Later Observations

- `FreshVarWArgs` is implemented in the pattern-match MGU unit but is also used by higher-order binding constructors. Rust centralizes the helper with lambda application utilities; after full higher-order unification is ported, consider whether the C header/source boundary should be reflected as a dedicated higher-order construction module instead of keeping this cross-unit dependency.
<!-- END MANUAL REVIEW: c_source_docs -->
