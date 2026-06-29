<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_inferencedoc

## Source Files

- [CLAUSES/ccl_inferencedoc.h](../../../eprover/CLAUSES/ccl_inferencedoc.h)
- [CLAUSES/ccl_inferencedoc.c](../../../eprover/CLAUSES/ccl_inferencedoc.c)

## Purpose

Functions and constants for reporting on the proof process. the GNU Lesser General Public License. <1> Tue Jan 5 15:27:37 MET 1999 Partially new, partially lifted from ccl_clauses.[ch]

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `InfType`
- `OutputFormatType`

### Macros And Constants

- `CCL_INFERENCEDOC`
- `DocClauseApplyDefsDefault(clause, parent_id, def_ids)`
- `DocClauseCreationDefault(clause, op, parent1, parent2)`
- `DocClauseModificationDefault(clause, op, partner)`
- `DocClauseQuoteDefault(target_level, clause, comment)`
- `DocClauseRewriteDefault(rewritten, old_term)`
- `DocFormulaCreationDefault(formula, op, parent1, parent2)`
- `DocFormulaIntroDefsDefault(form, def_list)`
- `DocFormulaModificationDefault(form, op)`
- `DocIntroSplitDefDefault(form)`
- `DocIntroSplitDefRestDefault(clause, parent)`
- `PCL_ACRES`
- `PCL_AD`
- `PCL_ANNOQ`
- `PCL_ARG_CONG`
- `PCL_CHOICE_AX`
- `PCL_CHOICE_INST`
- `PCL_CN`
- `PCL_CONDENSE`
- `PCL_CSR`
- `PCL_DDC`
- `PCL_DSTR`
- `PCL_DYN_CNF`
- `PCL_EBV`
- `PCL_EF`
- `PCL_EQ_TO_EQ`
- `PCL_ER`
- `PCL_EVALGC`
- `PCL_EVANS`
- `PCL_EXPDISTICT`
- `PCL_EXT_EQFACT`
- `PCL_EXT_EQRES`
- `PCL_EXT_SUP`
- `PCL_FLEX_RESOLVE`
- `PCL_FS`
- `PCL_FU`
- `PCL_ID`
- `PCL_ID_DEF`
- `PCL_INV_REC`
- `PCL_LEIBNIZ_ELIM`
- `PCL_LIFT_ITE`
- `PCL_LL`
- `PCL_LOCAL_RW`
- `PCL_NC`
- `PCL_NEG_EXT`
- `PCL_NNF`
- `PCL_OF`
- `PCL_PE_RESOLVE`
- `PCL_PM`
- `PCL_POS_EXT`
- `PCL_PRIM_ENUM`
- `PCL_PRUNE_ARG`
- `PCL_QUOTE`
- `PCL_RW`
- `PCL_SAT`
- `PCL_SC`
- `PCL_SE`
- `PCL_SK`
- `PCL_SPLIT`
- `PCL_SPM`
- `PCL_SQ`
- `PCL_SR`
- `PCL_TRIGGER`
- `PCL_VR`
- `TSTP_SPLIT_BASE`
- `TSTP_SPLIT_REFINED`

### Globals

- `extern OutputFormatType DocOutputFormat`
- `extern bool PCLFullTerms`
- `extern bool PCLStepCompact`
- `extern int PCLShellLevel`

### Exported Functions

- `DocClauseCreation(GlobalOut, OutputLevel, (clause),\ (op), (parent1), (parent2), NULL) void DocClauseFromForm(FILE* out, long level, Clause_p clause, WFormula_p parent)`
- `DocClauseModification(GlobalOut, OutputLevel, (clause), (op),\ (partner), NULL, NULL) void DocClauseQuote(FILE* out, long level, long target_level, Clause_p clause, char* comment, Clause_p opt_partner)`
- `DocClauseQuote(GlobalOut, OutputLevel, (target_level),\ (clause), (comment), NULL) void DocClauseRewrite(FILE* out, long level, ClausePos_p rewritten, Term_p old_term, char* comment)`
- `DocClauseRewrite(GlobalOut, OutputLevel, (rewritten),\ (old_term), NULL)`
- `DocFormulaCreation(GlobalOut, OutputLevel, (formula),\ (op), (parent1), (parent2), NULL) void DocFormulaModification(FILE* out, long level, WFormula_p form, InfType op, char* comment)`
- `DocFormulaIntroDefs(GlobalOut, OutputLevel, (form), (def_list), NULL) void DocIntroSplitDef(FILE* out, long level, WFormula_p form)`
- `DocFormulaModification(GlobalOut, OutputLevel, (form), (op), NULL) void DocFormulaIntroDefs(FILE* out, long level, WFormula_p form, PStack_p def_list, char* comment)`
- `DocIntroSplitDef(GlobalOut, OutputLevel, (form)) void DocIntroSplitDefRest(FILE* out, long level, Clause_p clause, WFormula_p parent, char* comment)`
- `DocIntroSplitDefRest(GlobalOut, OutputLevel, (clause), (parent), NULL) void DocClauseApplyDefs(FILE* out, long level, Clause_p clause, long parent_id, PStack_p def_ids, char* comment)`
- `char* PCLTypeStr(FormulaProperties type)`
- `void DocClauseCreation(FILE* out, long level, Clause_p clause, InfType op, Clause_p parent1, Clause_p parent2, char* comment)`
- `void DocClauseEqUnfold(FILE* out, long level, Clause_p rewritten, ClausePos_p demod, PStack_p demod_pos)`
- `void DocClauseModification(FILE* out, long level, Clause_p clause, InfType op, Clause_p partner, Sig_p sig, char* comment)`
- `void DocFormulaCreation(FILE* out, long level, WFormula_p formula, InfType op, WFormula_p parent1, WFormula_p parent2, char* comment)`

## Implementation Notes

### Internal Functions

- `pcl_formula_print_end`
- `pcl_formula_print_start`
- `pcl_print_end`
- `pcl_print_start`
- `print_ac_res`
- `print_annotate_question`
- `print_condense`
- `print_context_simplify_reflect`
- `print_des_eres`
- `print_distribute`
- `print_efactor`
- `print_eq_unfold`
- `print_eres`
- `print_eval_answer`
- `print_factor`
- `print_fof_intro_def`
- `print_fof_nnf`
- `print_fof_simpl`
- `print_fof_split_equiv`
- `print_formula_initial`
- `print_initial`
- `print_minimize`
- `print_neg_conj`
- `print_paramod`
- `print_rewrite`
- `print_shift_quantors`
- `print_simplify_reflect`
- `print_skolemize`
- `print_split`
- `print_var_rename`
- `tstp_formula_print_end`
- `tstp_print_end`

### Source-Level Behavior

- `PCLTypeStr`: Given an E-internal type of clause, return a string describing the type (default type is plain/ax and is represented by the empty string).
- `pcl_print_start`: Print the "<id> :<type> : <clause> : " part of a pcl step.
- `pcl_print_end`: Print the optional comment and new line
- `tstp_print_end`: Print the optional comment and new line
- `print_initial`: Print an initial clause (axiom).
- `print_paramod`: Print a clause creation by (simultaneous) paramodulation (or superposition).
- `print_eres`: Print a clause creation by equality resolution.
- `print_des_eres`: Print a clause modification by destructive equality resolution.
- `print_efactor`: Print a clause creation by equality factoring.
- `print_factor`: Print a clause creation by (ordinary) factoring.
- `print_split`: Print a clause creation by splitting.
- `print_simplify_reflect`: Print a clause modification by simplify-reflect.
- `print_context_simplify_reflect`: Print a clause modification by contextual simplify-reflect.
- `print_ac_res`: Print a clause modification by AC-resolution.
- `print_minimize`: Print a clause modification by clause-internal simplification (elemination of redundant literals)
- `print_condense`: Print a clause modification by condensation.
- `print_eval_answer`: Print a clause modification by answer-literal-elimination.
- `print_rewrite`: Print a series of rewrite steps.
- `print_eq_unfold`: Print a series of eq-unfoldings with demod.
- `pcl_formula_print_start`: Print the "<id> : <clause> : " part of a pcl step.
- `pcl_formula_print_end`: Print the optional comment and new line
- `tstp_formula_print_end`: Print the optional comment and new line
- `print_formula_initial`: Print an initial formula.
- `print_fof_intro_def`: Print the introduction of a formula definition.
- `print_fof_split_equiv`: Print the introduction of a formula by splitting <=> into => or <=.
- `print_fof_simpl`: Print a fof simplification step.
- `print_neg_conj`: Print a conjecture negation step ("assume opposite")
- `print_fof_nnf`: Print a fof negation normal form step.
- `print_shift_quantors`: Print a shift quantor (in for miniskoping, out for final CNF'ing) inference.
- `print_skolemize`: Print a Skolemization step.
- `print_distribute`: Print a distributivity step (or steps).
- `print_annotate_question`: Print a step adding answer literals.
- `print_var_rename`: Print a variable renaming step.
- `DocClauseCreation`: Document the creation of a new clause if level >=2
- `DocClauseFromForm`: Document the creation of a clause from a conjunct of a formula.
- `DocClauseModification`: Document the modification of a clause.
- `DocClauseQuote`: Print the clause with a new id as a descendent of itself only. Useful for getting the comment out.
- `DocClauseRewrite`: Document a series of rewrite steps performed on the literal position described in pos, on the original term old_term.
- `DocClauseEqUnfold`: Document rewrite steps caused by definition unfolding. Ugly and incomplete.
- `DocFormulaCreation`: Document the creation of a full FOF formula.
- `DocFormulaModification`: Document general clause modifications.
- `DocFormulaIntroDefs`: Print the application of a set of definitions to a formula.
- `DocIntroSplitDef`: Print a split definition that defines the constant predicate represented by def_pred as the (universal closure of) clause_part.
- `DocIntroSplitDefRest`: Print the clause representation of the expanding implication of a definition.
- `DocClauseApplyDefs`: Print the clause derivation describing the application of the definitions in def_ids to parent.

### Dependencies

- `"ccl_inferencedoc.h"`
- `<ccl_clausepos.h>`
- `<ccl_formula_wrapper.h>`

### Compile-Time Conditions

- `CCL_INFERENCEDOC`

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

Source files reviewed: `CLAUSES/ccl_inferencedoc.h`, `CLAUSES/ccl_inferencedoc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 2111 lines, 20 scanned public declarations, 32 scanned internal function definitions, and 45 structured function-comment blocks.
- Functions and constants for reporting on the proof process. the GNU Lesser General Public License. <1> Tue Jan 5 15:27:37 MET 1999 Partially new, partially lifted from ccl_clauses.[ch]
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- The PCL `print_initial` branch contains unconditional `printf("XX\n")` debug markers around `pcl_print_start`, and those writes target stdout rather than the passed documentation stream. Keep this accidental behavior visible for compatibility; a future cleanup should decide whether to remove the markers or make all proof-documentation output use one stream.
- `PCLTypeStr` deliberately returns `""` for every clause type except conjecture, question, and negated conjecture, so PCL step prefixes collapse plain/axiom, hypothesis, lemma, watchlist, and unknown roles. Rust ports this helper for compatibility; a later cleaned proof-documentation API may expose fuller role names outside C-compatible PCL formatting.
- `pcl_print_end` has an asymmetric non-compact spacing rule: watchlist comments use `": 'wl,...'"`, while ordinary comments and watchlist-only output use the leading-space form (`" : '...'"` / `" : 'wl'"`). Preserve it for byte-compatible PCL output; normalize only in a future non-compatibility renderer.
- `tstp_print_end` writes watchlist status and the optional comment as a single quoted list entry (`['wl,comment']`) instead of structured separate annotations. Preserve it for compatibility; split those fields only in a cleaned proof-documentation model.
- The formula terminators mirror ordinary clause comment suffixes but omit the clause watchlist branch. Keep the distinction while matching C; a cleaned proof-documentation API should represent clause/formula targets explicitly before choosing a suffix shape.
- Clause proof-documentation inference strings are also assembled by hand and use per-helper spacing and annotation shapes: `split` uses a TSTP split annotation instead of `status(thm)`, `split_conjunct`/rewrite/apply-def use spaces that the formula apply-def path omits, and answer-literal elimination appends `theory(answers)`. Rust ports these strings as isolated helpers; a future proof-object renderer should normalize the data model first and only then decide whether to offer a non-byte-compatible pretty-printer.
- `print_ac_res` asserts that the signature AC-axiom stack is non-empty and then expands every AC axiom into the parent list. Keep this assertion visible until AC axiom ownership and proof-output parent selection are fully represented; a cleaned API should report a structured missing-parent error rather than formatting a partial inference.
- `DocClauseQuote`'s optional-partner branch bypasses the normal PCL/TSTP suffix helpers, asserts that a comment exists, and renders the comment as `comment(partner)` while ignoring watchlist annotations. Keep this byte-compatible branch separate from ordinary comments until a structured proof-note model exists.
- `DocIntroSplitDefRest` accepts but ignores its comment argument, and `DocClauseApplyDefs` hard-codes `split` regardless of its comment argument. In the same family, several clause proof-documentation helpers assign fresh ids only inside the PCL/TSTP cases while `DocFormulaIntroDefs` assigns before the format switch. Preserve these timing differences until output-level compatibility no longer depends on them.
- `DocClauseRewrite` asserts that the selected clause position has a literal and that `TermComputeRWSequence` can recover a rewrite path from `old_term` to the selected side; Rust mirrors this as a panic for now. A future proof-documentation API may want a structured missing-rewrite diagnostic once callers are no longer expected to satisfy the C invariant.
- `DocClauseEqUnfold` is described in the C source as "ugly and incomplete": it ignores the actual entries in `demod_pos` and repeats the same demodulator clause id once per stack element. Rust preserves that visible proof term shape, but a later proof model should represent the unfolded positions or deliberately collapse them with an explicit reason.
- Formula proof-documentation inference strings are assembled by hand and preserve dialect-specific differences: formula definition introduction is `introduced` in PCL but `introduced(definition)` in TSTP, and TSTP inference terms use inconsistent spaces around parent lists across operations. Keep these byte-compatible renderings isolated from the eventual formula-proof data model.
- `WFormulaGetId` can return source formula names under the mutable global `FormulasKeepInputNames`; otherwise it formats generated `c_0_...`/`i_0_...` ids. Rust formula proof-documentation currently uses a pre-rendered formula view with the session id source; revisit input-name preservation once the real `WFormula` owner and formula source-name policy are ported.
- `DocClauseCreation` and `DocClauseQuote` both assign ids from the mutable global `ClauseIdentCounter`. Rust ports creation documentation with a session-owned id source for now, and mirrors the proof-success quote by rendering from a clone for the supported executable path; full proof-documentation support should decide whether mutable global documentation ids remain part of the ported model or stay session-owned state.

### Rust Port Status Notes

- `src/clauses/inferencedoc.rs` now ports the C `OutputFormatType` discriminants, a `ClauseIdentCounter`-style id source, and the represented `DocClauseCreation` dispatch for initial clauses, paramodulation, simultaneous paramodulation, equality resolution, equality factoring, factoring, and splitting.
- The creation helper renders C-shaped PCL and TSTP proof-documentation steps, including level gating, id assignment, parent arity assertions, source-info rendering, optional TSTP term type suffixes, unsupported-format fallback output, and the accidental PCL initial-clause `XX` stdout markers with the C interleaving point reported as explicit side-channel metadata.
- The clause-modification session helper now covers the represented `DocClauseModification` branches for simplify-reflect, contextual simplify-reflect, AC-resolution, condensation, minimization, answer-literal evaluation, and destructive equality resolution. It preserves C's `CPInputFormula` clearing before the output-level gate, id reassignment from the session counter, unsupported-format fallback after id assignment, the destructive equality-resolution partner assertion even though the partner id is not printed, and AC-resolution's nonempty axiom-list assertion over caller-provided signature axiom ids.
- A formula-documentation session helper now covers the represented `DocFormulaCreation` branches for initial formulas, formula definition introduction, and equivalence splitting, plus represented `DocFormulaModification` branches for simplification, conjecture negation, NNF conversion, quantifier shifting, variable renaming, Skolemization, distribution, and answer-literal annotation. It uses an explicit pre-rendered formula view until `WFormula` exists, and preserves C's creation-side lack of `CPInputFormula` clearing, modification-side clearing before the output-level gate, id reassignment from the session counter, unsupported-format fallback after id assignment, and default modification branch that assigns an id but prints nothing.
- Additional session helpers now cover the represented `DocClauseFromForm`, `DocClauseQuote`, `DocClauseRewrite`, `DocClauseEqUnfold`, `DocIntroSplitDef`, `DocIntroSplitDefRest`, `DocClauseApplyDefs`, and `DocFormulaIntroDefs` surfaces, including their C-specific comment handling, target-level gate, id-assignment timing, rewrite-chain/demodulator-parent rendering, and unsupported-format fallback behavior.
- Executable initial-clause and watchlist initial-clause documentation now use the shared creation helper. Tests cover the creation helper's level gate, id assignment, PCL/TSTP surfaces, unsupported-format fallback, exact output-format discriminants, output-file stdout marker leaks, formula creation/modification and exported helper session behavior, and executable initial-documentation output; generated/modification proof-control call sites remain separate work.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
