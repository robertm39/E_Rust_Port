<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_unfold_defs

## Source Files

- [CLAUSES/ccl_unfold_defs.h](../../../eprover/CLAUSES/ccl_unfold_defs.h)
- [CLAUSES/ccl_unfold_defs.c](../../../eprover/CLAUSES/ccl_unfold_defs.c)

## Purpose

Functions used for unfolding equational definitions (sometimes also called "demodulating", but that term seems to be seriously overloaded). This is basically a special case of rewriting. However, the application is sufficiently different to warrant separate implementation. It also is not shared (shame on me), but then it also is quite cheap and will be applied very rarely.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_UNFOLD_DEFS`
- `DEFAULT_EQDEF_INCRLIMIT`
- `DEFAULT_EQDEF_MAXCLAUSES`

### Globals

- None found in the source scan.

### Exported Functions

- `bool ClauseSetUnfoldEqDef(ClauseSet_p set, ClausePos_p demod)`
- `bool ClauseUnfoldEqDef(Clause_p clause, ClausePos_p demod, Term_p lside, Term_p rside)`
- `long ClauseSetPreprocess(ClauseSet_p set, ClauseSet_p passive, ClauseSet_p archive, TB_p tmp_terms, TB_p terms, bool replace_inj_defs, int eqdef_incrlimit, long eqdef_maxclauses)`
- `long ClauseSetUnfoldAllEqDefs(ClauseSet_p set, ClauseSet_p passive, ClauseSet_p archive, int min_arity, long eqdef_incrlimit)`
- `long ClauseSetUnfoldEqDefNormalize(ClauseSet_p set, ClauseSet_p passive, ClauseSet_p archive, TB_p tmp_terms, long eqdef_incrlimit, long eqdef_maxclauses)`

## Implementation Notes

### Internal Functions

- `eqn_unfold_def`
- `term_top_unfold_def_fo`
- `term_top_unfold_def_ho`
- `term_unfold_def`

### Source-Level Behavior

- `term_top_unfold_def_fo`: If possible, return the term that results from applying the demodulator at top position, otherwise return term.
- `term_top_unfold_def_ho`: Like term_top_unfold_def_fo, but assumes that all definitions have been transformed into symbol = lambda expression, so it does not do matching, but lambda normalization.
- `term_unfold_def`: Apply demod everywhere in term. One-traversal leftmost-innermost is complete, because we know that the top symbol of the demodulator cannot occur in demodulated terms. pos_stack is intended to keep positions, at the moment it just counts applications in a very expensive way ;-)
- `eqn_unfold_def`: Apply demod everywhere in the literal. See above. Return true if one term changes.
- `ClauseUnfoldEqDef`: Apply demod to normalize clause. Print unfolding as (annotated) rewrite steps. Return true if clause changed. NB: In case of HO unfolding lside and rside can be transformed into a lambda equation. Thus, they might be different in shape from what is stored in demod. Demod is still used to build proof object.
- `ClauseSetUnfoldEqDef`: Apply demod to all clauses in set.
- `ClauseSetUnfoldAllEqDefs`: While there are equational definitions where the right hand side is not to much (eqdef_incrlimit) bigger than the left hand side, apply them and remove them. Returns number of removed clauses.
- `ClauseSetPreprocess`: Perform preprocessing on the clause set: Removing tautologies, definition unfolding and canonization. Returns number of clauses removed. If passive is true, potential unfolding is applied to clauses in that set as well.
- `ClauseSetUnfoldEqDefNormalize`: Unfold definitions and renormalize clause set.

### Dependencies

- `"ccl_clausefunc.h"`
- `"ccl_unfold_defs.h"`
- `<cte_lambda.h>`

### Compile-Time Conditions

- `CCL_UNFOLD_DEFS`

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

Source files reviewed: `CLAUSES/ccl_unfold_defs.h`, `CLAUSES/ccl_unfold_defs.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 494 lines, 5 scanned public declarations, 4 scanned internal function definitions, and 9 structured function-comment blocks.
- Functions used for unfolding equational definitions (sometimes also called "demodulating", but that term seems to be seriously overloaded). This is basically a special case of rewriting. However, the application is sufficiently different to warrant separate implementation. It also is not shared (shame on me), but then it also is quite ch...
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/clauses/unfold_defs.rs` currently ports the `ClauseSetPreprocess` subset: `ClauseSetRemoveSuperfluousLiterals`, `ClauseSetFilterTautologies`, optional `ClauseSetReplaceInjectivityDefs`, and `ClauseSetCanonize`, returning the number of clauses removed by tautology filtering just as the C helper does.
- The same Rust unit now ports the first-order equality-definition unfolding path: `ClauseUnfoldEqDef`, `ClauseSetUnfoldEqDef`, `ClauseSetUnfoldAllEqDefs`, and `ClauseSetUnfoldEqDefNormalize`, including definition removal into the archive, `DCUnfold` derivation entries, tautology refiltering, and canonization after successful unfolding.
- Supported executable prune/proof-search paths apply the clause-set preprocessing subset before BCE and goal-definition preprocessing when `--no-preprocessing` is absent, then apply first-order `ClauseSetUnfoldEqDefNormalize` regardless of `--no-preprocessing` unless the unfolding gates disable it. `--eq-unfold-limit`, `--eq-unfold-maxclauses`, and `--no-eq-unfolding` now drive the supported first-order normalization path, and proof-search statistics use the combined removed count for `% Removed in clause preprocessing`.
- Higher-order lambda-definition unfolding and executable passive/watchlist unfolding at this preprocessing boundary remain pending.

### Change-Later Observations

- `ClauseSetPreprocess` keeps `eqdef_incrlimit` and `eqdef_maxclauses` parameters even though this current C body does not use them; Rust preserves the public helper shape with ignored parameters, while the separate Rust `ClauseSetUnfoldEqDefNormalize` bridge consumes those gates just as the C caller does after preprocessing.
- C `ProofStateClausalPreproc` archives copies of all input axioms before clause preprocessing, then later moves eliminated/unfolded definitions to the regular archive. Rust has not added that initial `ax_archive` copy at this executable boundary yet because represented proof-output parent resolution is still compact-id based; revisit this when stable clause handles make archive-copy identity less fragile.
- The C source comments say `ClauseSetPreprocess` performs definition unfolding, but the current function body only does literal cleanup, tautology filtering, optional injectivity replacement, and canonization. The actual equality-definition unfolding lives in `ClauseSetUnfoldEqDefNormalize`, which C calls separately after `ClauseSetPreprocess`.
- `ClauseUnfoldEqDef` emits live `DocClauseEqUnfold` preprocessing documentation when proof output is active. Rust records `DCUnfold` derivation metadata for unfolded clauses, but the executable path does not yet emit the live unfolding documentation stream because proof-document session ownership is still outside this preprocessing boundary.
- Higher-order unfolding uses `ClauseExtractHODefinition`, lambda-shaped definitions, and top-level lambda normalization instead of the first-order matching path. Rust currently no-ops this helper outside first-order mode so it does not accidentally apply first-order matching to higher-order lambda definitions.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
