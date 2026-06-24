<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clauses

## Source Files

- [CLAUSES/ccl_clauses.h](../../../eprover/CLAUSES/ccl_clauses.h)
- [CLAUSES/ccl_clauses.c](../../../eprover/CLAUSES/ccl_clauses.c)

## Purpose

Clauses - Infrastructure functions the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClauseCell`
- `Clause_p`
- `FormulaProperties`

### Macros And Constants

- `CCL_CLAUSES`
- `CLAUSECELL_MEM`
- `CLAUSE_ENSURE_DERIVATION(clause)`
- `ClauseAddFunOccs(clause, f_occur, res_stack)`
- `ClauseAddSymbolDistExist(clause, dist_array, exists)`
- `ClauseAddSymbolDistribution(clause, dist_array)`
- `ClauseAddSymbolFeatures(clause, mod_stack, feature_array)`
- `ClauseAddTypeDistribution(clause, type_array)`
- `ClauseCellAllocRaw()`
- `ClauseCellFree(junk)`
- `ClauseCollectFCodes(clause,tree)`
- `ClauseCollectVariables(clause,tree)`
- `ClauseComputeFunctionRanks(clause, rank_array, count)`
- `ClauseCondMarkMaximalTerms(ocb, clause)`
- `ClauseDelProp(clause, prop)`
- `ClauseDepth(clause)`
- `ClauseFindNegPureVarLit(clause)`
- `ClauseGCMarkTerms(clause)`
- `ClauseGiveProps(clause, prop)`
- `ClauseIsAnyPropSet(clause, prop)`
- `ClauseIsConjecture(clause)`
- `ClauseIsDemodulator(clause)`
- `ClauseIsEmpty(clause)`
- `ClauseIsEquational(clause)`
- `ClauseIsGoal(clause)`
- `ClauseIsGround(clause)`
- `ClauseIsHorn(clause)`
- `ClauseIsHypothesis(clause)`
- `ClauseIsMixed(clause)`
- `ClauseIsNegative(clause)`
- `ClauseIsPositive(clause)`
- `ClauseIsPureEquational(clause)`
- `ClauseIsRWRule(clause)`
- `ClauseIsSOS(clause)`
- `ClauseIsSubsumeOrdered(clause)`
- `ClauseIsUnit(clause)`
- `ClauseLiteralNumber(clause)`
- `ClauseMarkMaximalLiterals(ocb, clause)`
- `ClauseOrientLiterals(ocb, clause)`
- `ClausePropLitNumber(clause, prop)`
- `ClauseQueryCSSCPASource(clause)`
- `ClauseQueryProp(clause, prop)`
- `ClauseQueryTPTPType(clause)`
- `ClauseSetCSSCPASource(clause,prop)`
- `ClauseSetProp(clause, prop)`
- `ClauseSubsumeOrderSortLits(clause)`
- `ClauseTBTermDelPropCount(clause, prop)`
- `ClauseTermDelProp(clause, prop)`
- `ClauseTermSetProp(clause, prop)`
- `ClauseToStack(clause)`
- `FAIL_ON(cond)`
- `NormSubstClause(clause, subst, vars)`
- `TPTPTypesCombine(type1, type2)`

### Globals

- `extern bool ClausesHaveDisjointVariables`
- `extern bool ClausesHaveLocalVariables`
- `extern long ClauseIdentCounter`

### Exported Functions

- `((clause)->pos_lit_no+(clause)->neg_lit_no) EqnListQueryPropNumber((clause)->literals,(prop)) bool ClauseIsSemFalse(Clause_p clause)`
- `ClauseIsSorted((clause), \ (ComparisonFunctionType)EqnSubsumeInverseCompareRef) long ClauseStructWeightCompare(Clause_p c1, Clause_p c2)`
- `ClauseSortLiterals((clause), \ (ComparisonFunctionType)EqnSubsumeInverseRefinedCompareRef) bool ClauseIsSorted(Clause_p clause, ComparisonFunctionType cmp_fun)`
- `Clause_p ClauseAlloc(Eqn_p literals)`
- `Clause_p ClauseCanonize(Clause_p clause)`
- `Clause_p ClauseCopy(Clause_p clause, TB_p bank)`
- `Clause_p ClauseCopyDisjoint(Clause_p clause)`
- `Clause_p ClauseCopyOpt(Clause_p clause)`
- `Clause_p ClauseFlatCopy(Clause_p clause)`
- `Clause_p ClausePCLParse(Scanner_p in, TB_p bank)`
- `Clause_p ClauseParse(Scanner_p in, TB_p bank)`
- `Clause_p ClauseSkolemize(Clause_p clause, TB_p bank)`
- `Clause_p ClauseSortLiterals(Clause_p clause, ComparisonFunctionType cmp_fun)`
- `Clause_p EmptyClauseAlloc(void)`
- `EqnListAddSymbolDistribution((clause)->literals, (dist_array)) EqnListAddTypeDistribution((clause)->literals, (type_array)) EqnListAddSymbolDistExist((clause)->literals, (dist_array), (exists)) EqnListAddSymbolFeatures((clause)->literals, (mod_stack), (feature_array)) EqnListComputeFunctionRanks((clause)->literals, (rank_array), (count)) EqnListCollectVariables((clause)->literals,(tree)) EqnListCollectFCodes((clause)->literals,(tree)) EqnListAddFunOccs((clause)->literals, (f_occur), (res_stack)) long ClauseCollectSubterms(Clause_p clause, PStack_p collector)`
- `EqnListIsEquational(clause->literals) EqnListIsPureEquational(clause->literals) EqnListTermSetProp((clause)->literals, (prop)) EqnListTBTermDelPropCount((clause)->literals, (prop)) EqnListTermDelProp((clause)->literals, (prop)) bool ClauseIsRangeRestricted(Clause_p clause)`
- `EqnListOrient((ocb), (clause)->literals) EqnListMaximalLiterals((ocb), (clause)->literals) void ClauseAddEvalCell(Clause_p clause, Eval_p evaluation)`
- `EqnSide ClauseIsEqDefinition(Clause_p clause, int min_arity)`
- `FormulaProperties ClauseTypeParse(Scanner_p in, char *legal_types)`
- `NormSubstEqnListExcept((clause)->literals, \ NULL, (subst), (vars)) Clause_p ClauseNormalizeVars(Clause_p clause, VarBank_p fresh_vars)`
- `bool ClauseHasMaxPosEqLit(Clause_p clause)`
- `bool ClauseIsACRedundant(Clause_p clause)`
- `bool ClauseIsAntiRangeRestricted(Clause_p clause)`
- `bool ClauseIsSemEmpty(Clause_p clause)`
- `bool ClauseIsStronglyRangeRestricted(Clause_p clause)`
- `bool ClauseIsUntyped(Clause_p clause)`
- `bool ClauseNotGreaterEqual(OCB_p ocb, Clause_p clause1, Clause_p clause2)`
- `bool ClauseQueryLiteral(Clause_p clause, bool (*query_fun)(Eqn_p))`
- `bool ClauseRecognizeChoice(IntMap_p choice_symbols_map, Clause_p cl)`
- `bool ClauseStartsMaybe(Scanner_p in)`
- `double ClauseFunWeight(Clause_p clause, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long flimit, long *fweights, long default_fweight, double app_var_mult, long* typefreqs)`
- `double ClauseNonLinearWeight(Clause_p clause, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vlweight, long vweight, long fweight, double app_var_mult, bool count_eq_encoding)`
- `double ClauseOrientWeight(Clause_p clause, double unorientable_literal_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, double app_var_mult, bool count_eq_encoding)`
- `double ClauseStandardWeight(Clause_p clause)`
- `double ClauseSymTypeWeight(Clause_p clause, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, long cweight, long pweight, double app_var_mult)`
- `double ClauseTermExtWeight(Clause_p clause, TermWeightExtension_p twe)`
- `double ClauseWeight(Clause_p clause, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, double app_var_mult, bool count_eq_encoding)`
- `int ClauseCmpById(const void* clause1, const void* clause2)`
- `int ClauseCmpByPermId(const void* clause1, const void* clause2)`
- `int ClauseCmpByPermIdR(const void* clause1, const void* clause2)`
- `int ClauseCmpByPtr(const void* clause1, const void* clause2)`
- `int ClauseCmpByStructWeight(const void* clause1, const void* clause2)`
- `int ClauseCompareFun(const void *c1, const void* c2)`
- `long ClauseCollectGroundTerms(Clause_p clause, PTree_p *res, bool pos_lits, bool neg_lits, bool all_subterms)`
- `long ClauseReturnFCodes(Clause_p clause, PStack_p f_codes)`
- `long ClauseStructWeightLexCompare(Clause_p c1, Clause_p c2)`
- `void ClauseExtractHODefinition(Clause_p clause, EqnSide def_side, Term_p *lside, Term_p* rside)`
- `void ClauseFree(Clause_p junk)`
- `void ClauseMarkMaximalTerms(OCB_p ocb, Clause_p clause)`
- `void ClausePCLPrint(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrint(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintAxiom(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintDBG(FILE* out, Clause_p clause)`
- `void ClausePrintGoal(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintLOPFormat(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintList(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintQuery(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintRule(FILE* out, Clause_p clause, bool fullterms)`
- `void ClausePrintTPTPFormat(FILE* out, Clause_p clause)`
- `void ClauseRecomputeLitCounts(Clause_p clause)`
- `void ClauseRemoveEvaluations(Clause_p clause)`
- `void ClauseSetTPTPType(Clause_p clause, FormulaProperties type)`
- `void ClauseTSTPCorePrint(FILE* out, Clause_p clause, bool fullterms)`
- `void ClauseTSTPPrint(FILE* out, Clause_p clause, bool fullterms, bool complete)`
- `void TSTPSkipSource(Scanner_p in)`

## Implementation Notes

### Internal Functions

- `clause_collect_posneg_vars`
- `foundEqLitLater`

### Source-Level Behavior

- `clause_copy_meta`: Return a copy of the clause cell, but without literals.
- `clause_collect_posneg_vars`: Collect all the variables in positive literals of clause in pos_vars, the ones of negative literals in neg_vars.
- `TSTPSkipSource`: Skip a TSTP source field.
- `ClauseSetTPTPType`: Set the TPTP type of a clause.
- `ClauseCellAlloc`: Allocate a clause cell. This is a thin wrapper only relevant when perm-idents are enabled for debugging.
- `EmptyClauseAlloc`: Return a pointer to an empty clause initialized with rational values.
- `ClauseAlloc`: Create a new clause with the literal list list. Does sort literal list by pos/neg-literals for easier comparison, does not use EqnList functions because I'm a stupid performance freak.
- `ClauseRecomputeLitCounts`: Recompute the literal counts in clause.
- `ClauseIsTrivial`: Return true if the clause is trivial (because it has a trivial true literal or propositionally conflicting literals).
- `ClauseHasMaxPosEqLit`: Return true if the clause has a maximal positive equational literal.
- `ClauseSortLiterals`: Sort literal order in clause according to the given comparison function.
- `ClauseCanonize`: Try to bring the clause into a canonical representation. Terms are ordered by standard weight (except that $true is always minimal), literals are ordered by sign, equality, standard weight. Clauses should have no trivial literals!
- `ClauseIsSorted`: Return true if clause is in order with respect to the (quasi-)order defined by cmpfun.
- `ClauseStructWeightCompare`: Compare two clauses based on structure. Clauses are assumed to be canonized and have correct weight. The ordering is: Positive < mixed < negative smaller number of neg literals < greater number of neg literals smaller number of literals < greater number of literals StandardWeight Lexical extension of structural
- `ClauseStructWeightLexCompare`: Compare two clauses based on structure, break ties by lexical comparison, then by clause id.
- `ClauseIsACRedundant`: Return true if clause is redundant with respect to AC symbols. It is redundant if it is non-unit and has an AC-trivial literal or if it is an AC-trivial unit with more than two function symbols.
- `ClauseFree`: Free a clause. Does not take care of parents, children, etc., but releases the memory directly taken by the clause.
- `ClauseIsSemFalse`: Return true if the clause has only PseudoLiterals.
- `ClauseIsSemEmpty`: Return true if the clause has only simple answer literals.
- `ClauseIsRangeRestricted`: Return true if clause is range-restricted, i.e. if all variables occuring in the tail (negative literals) also occur in the head (positive literals).
- `ClauseIsAntiRangeRestricted`: Return true if clause is anti-range-restricted, i.e. if all variables occuring in the head also occur in the tail.
- `ClauseIsStronglyRangeRestricted`: Return true if clause is strongly range-restricted, i.e. if exactly the same variables occur in the tail and in the head.
- `ClauseIsEqDefinition`: If clause is a positive unit definition f(X1....Xn)=t (f not in t), return the definitional side, otherwise NoSide.
- `ClauseExtractHODefinition`: Given a
- `ClauseCopy`: Create a semantically equivalent clause and return a pointer to it. Does not copy parents, children, etc. Terms in the original clause are interpreted as instantiated, and are created in or retrived from the new bank. Evaluations are not copied, and neither is info.
- `ClauseFlatCopy`: As ClauseCopy(), but use the same bank as in clause, and ignore instantiations.
- `ClauseCopyOpt`: Copy a (possibly instantiated) clause using the "same term bank" optimizations.
- `ClauseCopyDisjoint`: Create a variable-disjoint copy of clause.
- `ClauseSkolemize`: ; Return a skolemized copy of clause.
- `ClausePrintList`: Print a clause as a declarative list of literals.
- `ClausePrintAxiom`: Print a clause as a declarative axiom in normal form, i.e. print positive literals first, then <-, then negative literals.
- `ClausePrintRule`: Print a clause as a rule, with the head literal as the conclusion and the remaining literals as preconditions. If a clause has a single literal only, print it as a fact.
- `ClausePrintGoal`: Print a clause as a goal, i.e. put all literals behind <-.
- `ClausePrintQuery`: Print a clause as a procedural query, i.e. put all literals behind ?-.
- `ClausePrintTPTPFormat`: Print a clause in TPTP format.
- `ClausePrintLOPFormat`: Print a clause in LOP format.
- `ClausePrint`: Print a clause in the most canonical representation.
- `ClausePrintDBG`: Print a clause in the form useful for debugging.
- `ClausePCLPrint`: Print a clause in PCL format.
- `ClauseTSTPCorePrint`: Print a core clause in TSTP format.
- `ClauseTSTPPrint`: Print a clause in TSTP format. If complete is true, terminate clause properly, otherwise stop after the logical part.
- `ClauseStartsMaybe`: Return true if a clause possibly starts on the current position in the input, i.e. if TermStartToken, TildeSign, ?-, or <- are present on the input stream.
- `ClauseTypeParse`: Parse a clause type specifier and return a matching type.
- `ClauseParse`: Parse a clause.
- `ClausePCLParse`: Parse a clause in PCL format, i.e. as TPTP literal list.
- `ClauseMarkMaximalTerms`: Orient literals, mark maximal literals.
- `ClauseParentsAreSubset`: Return true if parents of clause1 are a subset of clause2.
- `ClauseAddEvalCell`: Add an evaluation cell (as the first evaluation) to a clause.
- `ClauseRemoveEvaluations`: Remove the evaluations from a clause and free the EvalCells.
- `ClauseWeight`: Compute the weight of a clause by counting function symbols and variables and applying various modifiers.
- `ClauseFunWeight`: Compute the weight of a clause by summing weights for individual function symbols and variables and applying various modifiers.
- `ClauseTermExtWeight`: Compute the weight of a clause as an extension of an arbitrary term weight function. Modifiers are applied, several extensions are supported (standard - sum literal/term weights, subterms - sum weights of all subterms, or take the maximum subterm weight).
- `ClauseNonLinearWeight`: Compute the weight of a clause by counting function symbols and variables and applying various modifiers.
- `ClauseSymTypeWeight`: Compute the weight of a clause by counting function, predicate and constant symbols, and variables, and apply various modifiers.
- `ClauseStandardWeight`: Compute the standard weight of a clause (Vars = 1, Funs = 2, everything counts equally.
- `ClauseOrientWeight`: Compute the weight of a clause by counting function symbols and variables and applying various other modifiers ;-).
- `ClauseCompareFun`: Compare two clauses, induce a total ordering on all clauses. If ClauseCompareFun(clause1, clause2) == 0, then clause1==clause2 modulo symmetry of = and AC of the disjunction (In the current implementation, only symmetry is taken into account.
- `ClauseCmpById`: Compare two clauses by identifier.
- `ClauseCmpByPermId`: Compare two clauses by permanent identifier.
- `ClauseCmpByPermIdR`: Compare two clauses by reverse permanent identifier.
- `ClauseCmpByStructWeight`: Compare by a total syntactical order.
- `ClauseCmpByPtr`: Compare two clauses by address. This is rarely useful outside debugging!
- `ClauseNormalizeVars`: Destructively normalize variables in clause.
- `ClauseCollectSubterms`: Collect all subterms of clause onto collector. Assumes that TPOpFlag is unset in all subterms. Returns the number of new terms found.
- `ClauseReturnFCodes`: Push all function symbol codes from clause onto f_codes. Return number of symbols found.
- `ClauseQueryLiteral`: Return true if there is a literal that satisfies query_fun predicate
- `ClauseRecognizeChoice`: If the clause is of the form ~P X | P (f P) it will recognize that f is a defined choice operatior, store f in choice_symbols map and return true.
- `ClauseCollectGroundTerms`: Add no-constant ground subterms of the terms in certain literals (positive and/or negative, as per the selection parameters) in the clause to result. If top_only is set, only add maximal (in the subterm relation sense) terms, otherwise add all non-constant ground terms. Returns number of terms newly added.

### Dependencies

- `"ccl_clauses.h"`
- `"ccl_clausesets.h"`
- `"ccl_tformulae.h"`
- `"cte_lambda.h"`
- `<ccl_clauseinfo.h>`
- `<ccl_eqnlist.h>`
- `<ccl_neweval.h>`
- `<clb_fixdarrays.h>`
- `<clb_properties.h>`

### Compile-Time Conditions

- `CCL_CLAUSES`
- `CLAUSE_PERM_IDENT`
- `CONSTANT_MEM_ESTIMATE`
- `PCLPRINTDEGBUG`
- `PRINT_SOS_PROP`

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

Source files reviewed: `CLAUSES/ccl_clauses.h`, `CLAUSES/ccl_clauses.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 3147 lines, 71 scanned public declarations, 2 scanned internal function definitions, and 68 structured function-comment blocks.
- Primary clause object definition. Field ownership, property bits, derivation storage, and literal-list mutation affect almost every inference module.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.

### Compatibility Notes

- `ClauseAddEvalCell` stores the evaluation cell directly on the clause and sets `evaluation->object` to the owning clause pointer. Rust clause-owned evaluations should keep the object slot explicit until clause sets provide stable handles for eval-index lookups.
- `ClauseCopy`, `ClauseFlatCopy`, `ClauseCopyOpt`, and `ClauseCopyDisjoint` copy metadata but intentionally do not copy evaluations or source info. Rust copy helpers should continue to drop optional evaluation storage.
- `ClausePCLPrint` temporarily mutates the process-global `OutputFormat` to `TPTPFormat` to reuse `EqnListPrint`, then restores it. Rust preserves the bracketed PCL text with explicit TPTP equation-print options instead of hidden global mutation.
- `ClausePrintTPTPFormat` maps both `CPTypeConjecture` and `CPTypeNegConjecture` to the old TPTP role string `conjecture`; TSTP printing distinguishes `negated_conjecture`. Rust preserves this dialect-specific role mapping in the first-order TPTP helper.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
