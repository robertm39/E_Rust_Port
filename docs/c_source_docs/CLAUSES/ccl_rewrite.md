<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_rewrite

## Source Files

- [CLAUSES/ccl_rewrite.h](../../../eprover/CLAUSES/ccl_rewrite.h)
- [CLAUSES/ccl_rewrite.c](../../../eprover/CLAUSES/ccl_rewrite.c)

## Purpose

Functions for rewriting terms and clauses with clause sets. the GNU Lesser General Public License. <1> Tue May 26 19:47:52 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RWDescCell`
- `RWDesc_p`

### Macros And Constants

- `CCL_REWRITE`
- `RWDescCellAlloc()`
- `RWDescCellFree(junk)`

### Globals

- `extern unsigned long BWRWMatchAttempts`
- `extern unsigned long BWRWMatchSuccesses`
- `extern unsigned long RewriteAttempts`
- `extern unsigned long RewriteSuccesses`
- `extern unsigned long RewriteUnboundVarFails`
- `extern unsigned long RewriteUncached`

### Exported Functions

- `Term_p TermComputeLINormalform(OCB_p ocb, TB_p bank, Term_p term, ClauseSet_p *demodulators, RewriteLevel level, bool prefer_general, bool restricted_rw, bool lambda_demod)`
- `bool ClauseLocalRW(OCB_p ocb, Clause_p clause)`
- `bool FindRewritableClauses(OCB_p ocb, ClauseSet_p set, PStack_p results, Clause_p new_demod, SysDate nf_date)`
- `long ClauseComputeLINormalform(OCB_p ocb, TB_p bank, Clause_p clause, ClauseSet_p *demodulators, RewriteLevel level, bool prefer_general, bool lambda_demod)`
- `long ClauseSetComputeLINormalform(OCB_p ocb, TB_p bank, ClauseSet_p set, ClauseSet_p *demodulators, RewriteLevel level, bool prefer_general, bool lambda_demod)`
- `long FindRewritableClausesIndexed(OCB_p ocb, SubtermIndex_p index, PStack_p stack, Clause_p new_demod, SysDate nf_date)`

## Implementation Notes

### Internal Functions

- `clause_is_rewritable`
- `eqn_has_rw_side`
- `find_rewritable_clauses`
- `find_rewritable_clauses_indexed`
- `instance_is_rule`
- `rewrite_with_clause_set`
- `rewrite_with_clause_set_list`
- `rw_desc_cell_alloc`
- `subst_complete_min_instance`
- `term_find_rw_clauses`
- `term_is_rewritable`
- `term_is_top_rewritable`
- `term_li_normalform`
- `term_subterm_rewrite`
- `tree_find_rw_clauses`

### Source-Level Behavior

- `subst_complete_min_instance`: Complete the substitution by binding any unbound variable to the minimum term of the appropriate type.
- `instance_is_rule`: Return true if lside->rside is a rule, i.e. lside>rside (for the instantiated terms) and rside contains no unbound variables. Assumes that uninstantated terms lside and rside are uncomparable!
- `term_follow_top_RW_chain`: Return the last term in an existing rewrite link chain, following only top rewrite links. If one of those is induced by a SoS clause, set desc->sos.
- `term_is_top_rewritable`: Return true if the term is rewritable with new_demod at the top position, false otherwise.
- `term_is_rewritable`: Return true if the term is rewritable with new_demod, false otherwise. Set nf_date[0,1] on non-rewritable terms to nf_date (i.e. assumes thate term is in normal for with respect to earlier systems). I keep this like it is for the moment despite the new rewriting. We may loose a few cycles by not immediately adding a rewrite link if we detected a possible re...
- `eqn_has_rw_side`: Return NoSide, MaxSide, MinSide depending on wether eqn does or doesn't have a rewritable (maximal) side.
- `clause_is_rewritable`: Return true if clause is rewriteable with new_demod.
- `find_rewritable_clauses`: A non-index-using implementation of FindRewritableClause(). Returns true if any clause is rewritable
- `replace_term`: Replace all subterms stored in the rw_sys by their respective associated partners.
- `indexed_find_demodulator`: Find a demodulator via demodulators->demod_index.
- `rewrite_with_clauseset`: Rewrite the given term at root position with the first matching, orientable rule from demodulators. Return new term.
- `rewrite_with_clause_set_list`: Rewrite a term at top level with the sets of demodulators. Returns new term.
- `term_subterm_rewrite`: Normalize the subterms of the given term and propagate the result to term. Returns modification, result per *term.
- `term_li_normalform`: Compute a leftmost-innermost normal form of term. This uses dates to minimize rewrite-attempts: If the normal form of the term is younger than the clause sets, no further rewrite-attempt on this term is made.
- `eqn_li_normalform`: Compute the normal form of maximal, minimal or both terms in an equation. Return rewritten sides (truth value is true if any side was rewritten).
- `rw_desc_cell_alloc`: Allocate an initialized RWDescCell.
- `clause_tree_push`: Push all clauses in the tree onto stack (unless already there, indicated by CPRWDetected).
- `term_find_rw_clauses`: Push all clauses stored at termocc that are rewritable with lterm->rterm onto stack. Return number if there is at least one.
- `tree_find_rw_clauses`: Push all clauses in termtree that are rewritable with lterm->rterm onto stack. Return number of clauses.
- `find_rewritable_clauses_indexed`: Push all clauses in index that are rewritable with lterm->rterm onto stack. Return true if there is at least one.
- `TermComputeLINormalform`: Compute a leftmost-innermost normal form of term and return it. This uses dates to minimize rewrite-attempts: If the normal form of the term is younger than the clause sets, no further rewrite-attempt on this term is made.
- `ClauseComputeLINormalform`: Compute the normal form of terms in a clause. Return number of rewrite steps performed.
- `ClauseSetComputeLINormalform`: Compute a normal form for terms in all clauses in set with respect to clauses in demodulators up to level. Returns number of rewrite steps. Updates weights of rewritten clauses.
- `FindRewritableClauses`: New version - find all clauses that are rewritable with new_demod.
- `FindRewritableClausesIndexed`: New version - find all clauses that are rewritable with new_demod using the subterm index. Returns true if any rewritable clause was found.
- `ClauseLocalRW`: Find negative literals s != t such that s > t and replace all occurrences of s with t in the clause.

### Dependencies

- `"ccl_rewrite.h"`
- `<ccl_clausefunc.h>`
- `<ccl_pdtrees.h>`
- `<ccl_subterm_index.h>`
- `<cte_lambda.h>`
- `<cte_replace.h>`

### Compile-Time Conditions

- `CCL_REWRITE`
- `NDEBUG`

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

Source files reviewed: `CLAUSES/ccl_rewrite.h`, `CLAUSES/ccl_rewrite.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1621 lines, 16 scanned public declarations, 15 scanned internal function definitions, and 26 structured function-comment blocks.
- Rewrite/demodulation code; orientation status, limited rewriting, and index updates are subtle compatibility points.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `ClauseLocalRW` is ported in `src/clauses/rewrite.rs` as a clause-local mutation helper. It preserves the C order: orient literals, collect local rules, skip rule-source literals while rewriting, then recompute literal counts, remove superfluous literals, clear the clause orientation cache, and record `DCLocalRewrite` on modification. Rust uses the bank-backed orientation wrapper here when the active ordering needs owner-bank preparation.
- The temporary local rewrite system is keyed by shared-term identity, matching C's pointer-keyed `PObjMap`; duplicate rule keys overwrite earlier values like `PObjMapStore`.
- C treats any negative oriented literal as a local rule source, not just equational literals. This means an oriented negative predicate literal can be skipped as a source instead of rewritten by a positive-atom rule. Rust preserves that classification.
- `replace_term` recursively follows rule replacements and rebuilds changed top cells through the term bank. This matches `TermMap` restart behavior when the mapper returns a different shared term.
- `term_follow_top_RW_chain` is represented by `term_follow_top_rw_chain` in `src/terms/replace.rs`. It follows only rewrite links carrying a demodulator handle, honors the restricted-rewrite bit before stepping, and reports whether any traversed link was SoS-derived.
- Plain `ClauseComputeLINormalform` behavior is ported in `src/clauses/rewrite.rs` as `clause_compute_li_normalform_plain`. It normalizes literal sides with the same limited-rewrite retry condition, recovers compact rewrite derivation entries with `term_compute_rw_sequence`, clears `CP_INITIAL` only when the derivation delta records rewrite steps, and propagates `CP_IS_SOS` from followed rewrite links.
- `clause_compute_li_normalform_plain_with_docs` adds the C `OutputLevel >= 4` `DocClauseRewriteDefault` side effect for callers that provide a proof-documentation session and output sink. It emits a documentation step immediately after each rewritten side, preserves the live-clause identifier mutation chain, and leaves lower output levels identical to the non-documenting normalizer.
- Plain `ClauseSetComputeLINormalform` behavior is ported as `clause_set_compute_li_normalform_plain`. It preserves set iteration order, sums per-clause rewrite counts, refreshes cached standard weight only on clauses with nonzero rewrite steps, and records the demodulator-index `match_count` when the plain selector stands in for C `indexed_find_demodulator` on an indexed set. The stand-in now also applies the represented demodulator index's per-node age/size pruning predicate, conservative trie path pruning, and direct compact clause-id/side candidate consumption in recorded branch order when the index covers the set's demodulators.
- The C global rewrite counters are represented by Rust atomics. `REWRITE_UNCACHED` feeds proof-state cached rewrite-step statistics with C's `MAX(0, rw_count - RewriteUncached)` correction, and executable statistics now read the maintained unbound-RHS and backward-rewrite match attempt/success counters.

### Change Later

- `ClauseLocalRW` and clause normal-form rewriting now preserve C's compact derivation stack shape with numeric operation codes. Replace those raw constants with a typed derivation-code module when the broader proof-object and proof-output port lands.
- C does not directly refresh the cached clause weight after `ClauseLocalRW` unless `ClauseRemoveSuperfluousLiterals` removes something. Rust refreshes after any local rewrite to preserve the current Rust cached-weight invariant; revisit this when forward-contraction reference tests cover stale-weight observability.
- Positive-atom local rewriting uses `$false` as an intermediate replacement and relies on `EqnMap` to normalize `$false`/`$true` and flip polarity. A later typed API could expose this as a Boolean-literal transformation, but compatibility code should keep the C normalization path visible.
- The top-chain helper checks whether a link is followable before reading its SoS flag, so a skipped limited link under restricted rewriting does not contribute to SoS status. Preserve this for compatibility unless proof accounting tests show the C order is accidental.
- Backward rewrite discovery mutates term rewrite flags, rewrite links, global counters, and normal-form dates while only collecting candidate clauses. This makes discovery and later simplification tightly coupled; a later Rust API may want to split "detect" from "cache/link" once compatibility tests can prove the separation is behavior-neutral.
- `rewrite_strong_rhs_inst` completes unbound RHS variables with the ordering's designated minimum term by type. This may create fresh minimal constants during a match attempt; the side effect is compatibility-relevant but would be worth isolating behind a clearer typed-minimum cache after the ordering and signature ownership model is fully ported.
- Higher-order root replacement still passes through `MakeRewrittenTerm` in C after `TBInsertInstantiated`. Rust now ports the `TBTermPosReplace` retained-argument construction with the available DB-lambda beta-normalization subset, but the broader rewrite normalizer should still revisit this boundary once full LFHO eta/WHNF normalization and applied-variable rewrite construction are complete.
- Indexed backward rewrite discovery deduplicates by setting `CPRWDetected` on live C clauses while traversing occurrence trees. Rust currently suppresses duplicates locally by the clause key stored in the subterm occurrence maps because the index stores clause clones; revisit this when global clause ownership/indexing is unified.
- `term_find_rw_clauses` accepts `nf_date` but does not use it, and pushes candidate clauses before checking whether the instantiated replacement is identical to the matched term. This differs from the plain scan's final `RWNotRewritable` result for self-replacements; keep tests around both paths before changing either behavior.
- `indexed_find_demodulator` relies on a perfect discrimination tree search for traversal order, query age filtering, subtree-consuming variable alternatives, and the `prefer_general` heuristic. Rust currently ports the selection rules through direct compact clause-id/side candidate consumption in recorded branch order for fully indexed sets, with conservative represented-index per-node constraint/trie-path pruning and a set-order fallback when the index is absent or partial; clause sets can own demodulator indexes for insertion/deletion/storage and record `PDTreeSearchInit` query code/spans/weight/date, match-attempt accounting, and `prefer_general` traversal-order initialization. Revisit performance and exact semantics once `PDTreeFindNextDemodulator`-style traversal over live clause positions, dynamic substitution state, variable-binding weight adjustment, and backtracking exists.
- The C right-side branch for unoriented demodulators intentionally omits the restricted-renaming rejection used on the left-side branch, with an inline note that the older condition looked wrong. Rust preserves that asymmetry for compatibility until C/Rust comparison tests prove a different rule is observable.
- `term_li_normalform` is driven by `RWDesc`, which packages the OCB, bank, demodulator list, level, age cutoff, `prefer_general`, and `sos_rewritten` flag. Rust currently passes those pieces explicitly plus a private trace object; this should be collapsed into a Rust descriptor once indexed demodulator selection and clause-set normalization share the path.
- `eqn_li_normalform` performs verbose proof documentation through `DocClauseRewriteDefault` only when the global output level is at least 4, and that documentation mutates the live clause identifier while normalization is still in progress. Rust now exposes the same behavior through an opt-in wrapper; a later structured proof API should separate normalization, proof-step id allocation, and rendering once compatibility call sites are fully wired.
- `ClauseComputeLINormalform` counts rewrite steps as `(new_deriv_sp - old_deriv_sp) / 2`, relying on each demodulation entry being stored as a `DCRewrite` operation plus one clause argument. Rust preserves that compact stack shape; keep this arithmetic covered by tests when replacing raw derivation constants.
- The C set-level normal-form wrapper updates clause weights in place without refreshing evaluation indexes. Rust preserves the direct weight update; revisit this if weighted evaluation indexes become live during simplification.
<!-- END MANUAL REVIEW: c_source_docs -->
