<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_formulafunc

## Source Files

- [CLAUSES/ccl_formulafunc.h](../../../eprover/CLAUSES/ccl_formulafunc.h)
- [CLAUSES/ccl_formulafunc.c](../../../eprover/CLAUSES/ccl_formulafunc.c)

## Purpose

Higher level Formula functions that need to know about sets (and CNFing). the GNU Lesser General Public License. <1> Sun Apr 4 14:10:19 CEST 2004

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_FORMULAFUNC`
- `MAX_RW_STEPS`
- `TFORMULA_GC_LIMIT`
- `WFormulaTSTPPrintDeriv(out, form)`

### Globals

- None found in the source scan.

### Exported Functions

- `TFormula_p TFormulaAnnotateQuestion(TB_p terms, TFormula_p form, NumTree_p *question_assoc)`
- `WFormulaTSTPPrint((out),(form), true, false); \ fprintf((out), ", "); \ DerivationStackTSTPPrint((out), (form)->terms->sig, (form)->derivation); \ fprintf(out, ").")`
- `bool FormulaHasAppVarLit(Sig_p sig, TFormula_p form)`
- `bool TFormulaUnrollFOOL(WFormula_p form, TB_p terms)`
- `bool WFormulaAnnotateQuestion(WFormula_p wform, bool add_answer_lits, bool conjectures_are_questions, NumTree_p *question_assoc)`
- `bool WFormulaConjectureNegate(WFormula_p wform)`
- `bool WFormulaReplaceEqnWithEquiv(WFormula_p form, TB_p terms)`
- `bool WFormulaSimplify(WFormula_p form, TB_p terms)`
- `int FormulaCountNonTopLevelLambdas(Sig_p sig, TFormula_p form)`
- `long FormulaAndClauseSetParse(Scanner_p in, FormulaSet_p fset, ClauseSet_p wlset, TB_p terms, StrTree_p *name_selector, StrTree_p *skip_includes)`
- `long FormulaSetCNF2(FormulaSet_p set, FormulaSet_p archive, ClauseSet_p clauseset, TB_p terms, VarBank_p fresh_vars, long miniscope_limit, long def_limit, bool lift_lambdas, bool lambda_to_forall, bool unfold_only_form, bool unroll_fool)`
- `long FormulaSetPreprocConjectures(FormulaSet_p set, FormulaSet_p archive, bool add_answer_lits, bool conjectures_are_questions)`
- `long FormulaSetSimplify(FormulaSet_p set, TB_p terms, bool gc)`
- `long FormulaToCNF(WFormula_p form, FormulaProperties type, ClauseSet_p set, TB_p terms, VarBank_p fresh_vars)`
- `long TFormulaApplyDefs(WFormula_p form, TB_p terms, NumXTree_p *defs)`
- `long TFormulaSetIntroduceDefs(FormulaSet_p set, FormulaSet_p archive, TB_p terms, long limit)`
- `long TFormulaSetLambdaNormalize(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `long TFormulaSetLiftItes(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `long TFormulaSetLiftLambdas(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `long TFormulaSetLiftLets(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `long TFormulaSetNamedToDBLambdas(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `long TFormulaSetUnfoldDefSymbols(FormulaSet_p set, FormulaSet_p archive, TB_p terms, bool only_forms)`
- `long TFormulaToCNF(WFormula_p form, FormulaProperties type, ClauseSet_p set, TB_p terms, VarBank_p fresh_vars)`
- `long WFormulaCNF2(WFormula_p form, ClauseSet_p set, TB_p terms, VarBank_p fresh_vars, long miniscope_limit, bool fool_unroll)`
- `long WFormulaSetUnrollFOOL(FormulaSet_p set, FormulaSet_p archive, TB_p terms)`
- `void ClauseSetLiftLambdas(ClauseSet_p set, FormulaSet_p archive, TB_p terms, VarBank_p fresh_vars, bool unroll_fool)`
- `void FormulaSetArchive(FormulaSet_p set, FormulaSet_p archive)`
- `void FormulaSetDocInital(FILE* out, long level, FormulaSet_p set)`
- `void TFormulaSetDelTermpProp(FormulaSet_p set, TermProperties prop)`
- `void TFormulaSetFindDefs(FormulaSet_p set, TB_p terms, NumXTree_p *defs, PStack_p renamed_forms, long limit)`

## Implementation Notes

### Internal Functions

- `check_all_found`
- `close_let_def`
- `deleter`
- `fool_should_ignore`
- `insert_to_set`
- `replace_body`
- `verify_name`

### Source-Level Behavior

- `FlattenApps_driver`: Apply additional arguments to hd assuming hd needs to be flattened.
- `close_let_def`: For each defined symbol f(bound vars) = s, finds what are free variables in s and creates a fresh symbol f_fresh(free_vars, bound_vars)
- `replace_body`: Replace all occurences of old symbols with new definitions.
- `make_fresh_defs`: Make a formula introducing a new name for a local let definition
- `lift_lets`: Does the actual lifting of let terms
- `unencode_eqns`: Undo encoding of the form **formula** = $true to **formula**
- `refresh_qvars`: Bind all quantified variables in form to fresh free variables.
- `do_rw_with_defs`: Actually performs rewriting on a term using definition map. Stores used formulas in used_defs.
- `create_sym_map`: Creates a map from symbol to WFormula describing the simplified definition f = \xyz. body
- `intersimplify_definitions`: Make sure that the definitions themselves are rewritten using terms in sym_def_map.
- `map_formula`: Applies processor to form. If formula is changed it alters the proof object by applying the correct derivation code.
- `ignore_include`: Ignore includes and echoes the ignored declaration. Used for app encoding only.
- `answer_lit_alloc`: Allocate a FOF literal of the form ~$answer(skn(x1, ... xn)), where the xi are the variables on varstack and skn is a new skolem symbol.
- `verify_name`: If name_selector is NULL, return true. Otherwise, check if info->name is in name_selector. Return true if yes, false otherwise.
- `check_all_found`: Check if all names in name_selector are marked as found. Print a useful error message and terminate otherwise.
- `fool_should_ignore`: Is the term a variable encoded as X = true, X!=true or a negation thereof. Or is it of the form $eq(true, true) or $eq(false, false).
- `find_fool_subterm`: Returns true if it finds a formula subterm in t. pos is the position corresponding to this subterm if it is found, empty otherwise.
- `do_fool_unroll`: Unroll boolean arguments of terms. For example, subformula "f(a, p&q) = a" is replaced with "(~(p&q)|f(a,$true)=a) & (p&q)|f(a, $false)=a".
- `do_ite_unroll`: If $ite(c, it, if) occurs at formula position p, replace f|_p with f[c -> it /\ ~c -> if]_p. If it occurs at subterm position p, find the first above formula position q and do the replacement f[c -> f[it]_p /\ ~c -> f[if]_p]_q.
- `do_bool_eqn_replace1`: Replace boolean equations with equivalences. Goes inside literals as well. For example, "f(a, p = q) = b" will be translated to "f(a, p <=> q) = b".
- `do_bool_eqn_replace`: Replace boolean equations with equivalences. Goes inside literals as well. For example, "f(a, p = q) = b" will be translated to "f(a, p <=> q) = b". We don't want to lift "true" atoms, but we do want to lift proper Boolean formulas. So with t as a non-logical term, f as a non-trivial formula: eq(t, $true) => eq(t, $true) eq(f, $true) => f We also don't want...
- `TformulaCollectClause`: Given a term-encoded formula that is a disjunction of literals, transform it into a clause.
- `WFormulaConjectureNegate`: If formula is a conjecture, negate it and delete that property (but set WPInitialConjecture). Returns true if formula was a conjecture.
- `TFormulaAnnotateQuestion`: Take a formula of the form ((\exists X)*.F) and convert it to ((\exists Xi)*.(F&~$answer(skn(X1,...Xn))), i.e. add an answer literal encoding all leading existentially quantified variables.
- `WFormulaAnnotateQuestion`: If formula is a question, convert it into the equivalent conjecture with answer annotation. Returns true if formula was a question. Add the association of the new skolem symbol in the answer literal to the clause id.
- `FormulaSetPreprocConjectures`: Negate all conjectures to make the implication to prove into an formula set that is inconsistent if the implication is true. Note that multiple conjectures are implicitely disjunctively connected! Returns number of conjectures.
- `WFormulaSimplify`: Apply standard simplifications to the wrapped formula. Return true if the formula has changed.
- `WFormulaCNF2`: Transform the formula of a wrapped formula into CNF.
- `FormulaSetSimplify`: Apply standard FOF simplification rules to all formulae in the set. Returns number of changed formulas.
- `FormulaSetCNF2`: Transform all formulae in set into CNF. Return number of clauses generated.
- `FormulaAndClauseSetParse`: Parse a mixture of clauses and formulas (if the syntax supports it). Return number of elements parsed (even if discarded by filter). Watch list clauses are parsed as clauses in wlset, everything else (even clauses) is parsed as a formula and put into fset.
- `TFormulaToCNF`: Convert a term-encoded formula from conjunctive normal form into a set of (variable-normalized) clauses. Return number of clauses generated.
- `TFormulaSetDelTermpProp`: Go through a set of term-encoded formulas and delete prop in all term and formula cells.
- `TFormulaSetFindDefs`: Go through a set of formulas and generate and record all necessary definitions. Assumes that the formulas are simplified!
- `TFormulaApplyDefs`: Given a formula and a number of definitions represented by defs and tags in bank, apply all apropriate definitions to simplify the formula. Return the number of definitions used. Note that defs has to contain the defined atoms in val2 and the ident of the corresponding definition in val1 of its cells.
- `TFormulaUnrollFOOL`: Translate FOOL features into FOL. Performs following translations: - Takes formulas as arguments out of the term, leaving only $true, $false and boolean vars as the argument of the term - TODO: Unfolds ite expressions used as terms - TODO: Unfolds ite expressions used as formulas
- `WFormulaReplaceEqnWithEquiv`: If input formula contains subformulas of type \alpha = \beta, replace those subformulas with \alpha <=> \beta and alter proof object accordingly.
- `WFormulaSetUnrollFOOL`: Unrolls FOOL features for the set of formulas.
- `TFormulaSetLiftLets`: Rewrites all formulas so that all occurrences of the let symbols are replaced by global definitions.
- `TFormulaSetLiftItes`: Rewrites all formulas so that all occurrences of the ite symbols are replaced by appropriate implications
- `TFormulaSetLambdaNormalize`: Beta normalizes the input problem and turns every equation (^[X]:s) = t into ![X]: (s = (t @ X))
- `TFormulaSetUnfoldDefSymbols`: Rewrites all formulas using defined symbols of the form sym = \vars. body where return type of sym is Bool
- `TFormulaSetLiftLambdas`: Lifts lambdas from the formula set. Inserts new definitons into set.
- `TFormulaSetNamedToDBLambdas`: Convert all lambdas in the proof state from named to de Bruijn representation ()
- `TFormulaSetIntroduceDefs`: Transform a formula set by renaming certain subformulae and adding the necessary definitions. Returns the number of definitions. Note that NumXTree cells are used as follows: key is the term ident of the formula to be replaced vals[0].i_val starts as the polarity of that formula, but turns into the id of the "virtual" definition used for output vals[1].p_va...
- `FormulaSetArchive`: Move each formula from set to archive, replace it by a copy that quoted the archived formula as the parent.
- `FormulaSetDocInital`: If level >= 2, print all formula as initials.
- `FormulaCountNonTopLevelLambdas`: Count non-top level lambdas in the formulas
- `FormulaHasAppVarLit`: Does formula have a literal that is an applied variable?
- `cond_lift_lambda`: If a term-(encoded formula) is not a lambda-term, but has a proper lambda-subterm, bring it into the encoding needed for CNFization (e.q. encode literals as $eq(t1, t2)) and lift the lambda into a universal.
- `ClauseSetLiftLambdas`: Lift lambdas in clauses, change them in place, modify the proof object and store the lambda definitions in archive. New lambda definitions are clausified in turn.

### Dependencies

- `"ccl_clausefunc.h"`
- `"ccl_formulafunc.h"`
- `"cte_lambda.h"`
- `<ccl_garbage_coll.h>`
- `<ccl_tcnf.h>`

### Compile-Time Conditions

- `CCL_FORMULAFUNC`
- `ENABLE_LFHO`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_formulafunc.h`, `CLAUSES/ccl_formulafunc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 3048 lines, 32 scanned public declarations, 7 scanned internal function definitions, and 51 structured function-comment blocks.
- Higher level Formula functions that need to know about sets (and CNFing). the GNU Lesser General Public License. <1> Sun Apr 4 14:10:19 CEST 2004
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change-Later Observations

- `WFormulaAnnotateQuestion` mutates `question` formulas, and optionally `conjecture` formulas under `--conjectures-are-questions`, into `conjecture` before conjecture negation. `TFormulaAnnotateQuestion` only adds a `~$answer(esk(...))` literal when the formula starts with one or more leading existential quantifiers; non-leading existential variables are not included in the answer tuple. Rust now mirrors that behavior in the supported temporary formula bridge for proof/CNF/prune parsing while keeping syntax-only printing role-preserving. A later full `WFormula` port should keep this preprocessing step explicit instead of hiding role mutation in raw parsing.
- `FormulaAndClauseSetParse` routes every parsed entry whose resulting type is `CPTypeWatchClause` into the caller-provided `wlset`, including recursively parsed includes after selector filtering, while all non-watchlist entries go to the normal formula set. Rust now mirrors this for supported TPTP/TSTP input by splitting lowered clauses into normal input and inline watchlist sets; a later parser API should make the two output channels explicit instead of coupling watchlist storage to formula-role classification.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
