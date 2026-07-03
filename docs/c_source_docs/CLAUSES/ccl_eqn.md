<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_eqn

## Source Files

- [CLAUSES/ccl_eqn.h](../../../eprover/CLAUSES/ccl_eqn.h)
- [CLAUSES/ccl_eqn.c](../../../eprover/CLAUSES/ccl_eqn.c)

## Purpose

The termpair datatype: Rules, Equations, positive and negative literals. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `EqnCell`
- `EqnProperties`
- `EqnRef`
- `EqnSide`
- `Eqn_p`
- `PatEqnDirection`

### Macros And Constants

- `BOOL_TERM_NORMALIZE(t)`
- `CCL_EQN`
- `EQN_CELL_MEM`
- `EQUAL_PREDICATE`
- `EqnAddFunOccs(eqn, f_occur, res_stack)`
- `EqnAddSymbolDistExist(eqn, dist_array, exist)`
- `EqnAddSymbolDistribution(eqn, dist_array)`
- `EqnAddSymbolDistributionLimited(eqn, dist_array, limit)`
- `EqnAddSymbolFeaturesLimited(eqn, freq_array, depth_array, limit)`
- `EqnAddTypeDistribution(eqn, type_array)`
- `EqnCellAlloc()`
- `EqnCellFree(junk)`
- `EqnCollectFCodes(eqn, tree)`
- `EqnCollectGroundTerms(eqn, res, all_subterms)`
- `EqnCollectPropVariables(eqn, tree, prop)`
- `EqnCollectVariables(eqn, tree)`
- `EqnComputeFunctionRanks(eqn, rank_array, count)`
- `EqnCountMaximalLiterals(eqn)`
- `EqnCreateTrueLit(bank)`
- `EqnDelProp(eqn, prop)`
- `EqnDominates(eq)`
- `EqnEqual(eq1, eq2)`
- `EqnEqualDirected(eq1, eq2)`
- `EqnEqualDirectedDeref(eq1, eq2, d1, d2)`
- `EqnEquiv(eq1, eq2)`
- `EqnFlipProp(eqn, prop)`
- `EqnGCMarkTerms(eqn)`
- `EqnGetPredCode(eq)`
- `EqnGetPredCodeFO(eq)`
- `EqnGetPredCodeHO(eq)`
- `EqnHasEquiv(eq)`
- `EqnIsAnyPropSet(eqn, prop)`
- `EqnIsBoolVar(eq)`
- `EqnIsClausifiable(eq)`
- `EqnIsDominated(eq)`
- `EqnIsEquLit(eq)`
- `EqnIsGround(eq)`
- `EqnIsMaximal(eq)`
- `EqnIsNegative(eq)`
- `EqnIsOriented(eq)`
- `EqnIsPartVar(eq)`
- `EqnIsPositive(eq)`
- `EqnIsPropFalse(eq)`
- `EqnIsPropTrue(eq)`
- `EqnIsPropositional(eq)`
- `EqnIsPureVar(eq)`
- `EqnIsRealXTypePred(eq)`
- `EqnIsSelected(eq)`
- `EqnIsSimpleAnswer(eq)`
- `EqnIsSplitLit(eq)`
- `EqnIsStrictlyMaximal(eq)`
- `EqnIsTrivial(eq)`
- `EqnIsTypePred(eq)`
- `EqnIsXTypePred(eq)`
- `EqnPrintOriginal(out, eq)`
- `EqnQueryProp(eqn, prop)`
- `EqnSetProp(eqn, prop)`
- `EqnSkolemSubst(handle, subst, sig)`
- `EqnSplitModStandardWeight(eqn)`
- `EqnStandardDiff(eqn)`
- `EqnStandardWeight(eqn)`
- `EqnTBTermDelPropCount(eq,prop)`
- `EqnTBTermEncode(eqn, dir)`
- `EqnTermDelProp(eqn, prop)`
- `EqnTermSetProp(eq,prop)`
- `LiteralEqual(eq1, eq2)`
- `LiteralEquiv(eq1, eq2)`

### Globals

- `extern IOFormat OutputFormat`
- `extern bool EqnFullEquationalRep`
- `extern bool EqnPrintOriented`
- `extern bool EqnUseInfix`

### Exported Functions

- `(MAX(TermStandardWeight((eqn)->lterm), \ TermStandardWeight((eqn)->rterm)) - \ MIN(TermStandardWeight((eqn)->lterm), \ TermStandardWeight((eqn)->rterm))) long EqnMaxTermPositions(Eqn_p eqn)`
- `(PropsAreEquiv((eq1),(eq2),EPIsPositive) && EqnEqual((eq1),(eq2))) bool EqnSubsumeDirected(Eqn_p subsumer, Eqn_p subsumed, Subst_p subst)`
- `(TermCollectGroundTerms((eqn)->lterm, (res), (all_subterms))+ \ TermCollectGroundTerms((eqn)->rterm, (res), (all_subterms))) void EqnAppEncode(FILE* out, Eqn_p eq, bool negated)`
- `(TermStandardWeight((eqn)->lterm)+ \ TermStandardWeight((eqn)->rterm)) EqnQueryProp(eqn,EPIsSplitLit|EPIsPositive)? \ SigGetSpecialWeight(eqn->bank->sig, eqn->lterm->f_code): \ EqnStandardWeight(eqn) double EqnFunWeight(Eqn_p eq, double max_multiplier, long vweight, long flimit, long *fweights, long default_fweight, double app_var_mult, long* typefreqs)`
- `CompareResult EqnCompare(OCB_p ocb, Eqn_p eq1, Eqn_p eq2)`
- `CompareResult LiteralCompare(OCB_p ocb, Eqn_p eq1, Eqn_p eq2)`
- `EqnPrint((out), (eq), normal, true) void EqnPrintDeref(FILE* out, Eqn_p eq, DerefType deref)`
- `EqnSide EqnIsDefinition(Eqn_p eq, int min_arity)`
- `EqnTermsTBTermEncode((eqn)->bank, (eqn)->lterm, \ (eqn)->rterm, EqnIsPositive(eqn), (dir)) Eqn_p EqnTBTermDecode(TB_p terms, Term_p eqn)`
- `Eqn_p EqnAlloc(Term_p lterm, Term_p rterm, TB_p bank, bool positive)`
- `Eqn_p EqnAllocFlatten(Term_p lterm, TB_p bank, bool sign)`
- `Eqn_p EqnCanonize(Eqn_p eq)`
- `Eqn_p EqnCopy(Eqn_p eq, TB_p bank)`
- `Eqn_p EqnCopyDisjoint(Eqn_p eq)`
- `Eqn_p EqnCopyRepl(Eqn_p eq, TB_p bank, Term_p old, Term_p repl)`
- `Eqn_p EqnCopyReplPlain(Eqn_p eq, TB_p bank, Term_p old, Term_p repl)`
- `Eqn_p EqnFOFParse(Scanner_p in, TB_p bank)`
- `Eqn_p EqnFlatCopy(Eqn_p eq)`
- `Eqn_p EqnHOFParse(Scanner_p in, TB_p terms, bool *continue_parsing)`
- `Eqn_p EqnParse(Scanner_p in, TB_p bank)`
- `PStackPointer SubstNormEqn(Eqn_p eq, Subst_p subst, VarBank_p vars)`
- `SubstSkolemizeTerm((handle)->lterm, (subst), (sig)); \ SubstSkolemizeTerm((handle)->rterm, (subst), (sig)) Eqn_p EqnCopyOpt(Eqn_p eq)`
- `TermAddSymbolDistribution((eqn)->lterm, (dist_array)); \ TermAddSymbolDistribution((eqn)->rterm, (dist_array)) TermAddSymbolDistExist((eqn)->lterm, (dist_array), (exist)); \ TermAddSymbolDistExist((eqn)->rterm, (dist_array), (exist)) TermAddTypeDistribution((eqn)->lterm, (eqn)->bank->sig, type_array);\ TermAddTypeDistribution((eqn)->rterm, (eqn)->bank->sig, type_array) TermAddSymbolDistributionLimited((eqn)->lterm, (dist_array), (limit)); \ TermAddSymbolDistributionLimited((eqn)->rterm, (dist_array), (limit)) TermAddSymbolFeaturesLimited((eqn)->lterm, 0, (freq_array), \ (depth_array), (limit)); \ TermAddSymbolFeaturesLimited((eqn)->rterm, 0, (freq_array), \ (depth_array), (limit)) void EqnAddSymbolFeatures(Eqn_p eq, PStack_p mod_stack, long *feature_array)`
- `TermComputeFunctionRanks((eqn)->lterm, (rank_array), (count)); \ TermComputeFunctionRanks((eqn)->rterm, (rank_array), (count)) (TermCollectVariables((eqn)->lterm,(tree))+ \ TermCollectVariables((eqn)->rterm,(tree))) (TermCollectFCodes((eqn)->lterm,(tree))+ \ TermCollectFCodes((eqn)->rterm,(tree))) (TermCollectPropVariables((eqn)->lterm,(tree), (prop))+ \ TermCollectPropVariables((eqn)->rterm,(tree), (prop))) (TermAddFunOcc((eqn)->lterm,(f_occur), (res_stack))+ \ TermAddFunOcc((eqn)->rterm, (f_occur), (res_stack))) long EqnCollectSubterms(Eqn_p eqn, PStack_p collector)`
- `Term_p EqnTBTermParse(Scanner_p in, TB_p bank)`
- `Term_p EqnTermsTBTermEncode(TB_p bank, Term_p lterm, Term_p rterm, bool positive, PatEqnDirection dir)`
- `bool EqnGreater(OCB_p ocb, Eqn_p eq1, Eqn_p eq2)`
- `bool EqnHasAppVar(Eqn_p eq)`
- `bool EqnHasUnboundVars(Eqn_p eq, EqnSide dom_side)`
- `bool EqnIsACTrivial(Eqn_p eq)`
- `bool EqnIsFalse(Eqn_p eq)`
- `bool EqnIsTrue(Eqn_p eq)`
- `bool EqnOrient(OCB_p ocb, Eqn_p eq)`
- `bool EqnSubsume(Eqn_p subsumer, Eqn_p subsumed, Subst_p subst)`
- `bool EqnSubsumeP(Eqn_p subsumer, Eqn_p subsumed)`
- `bool EqnTermsAreDistinct(Eqn_p eq)`
- `bool EqnUnify(Eqn_p eq1, Eqn_p eq2, Subst_p subst)`
- `bool EqnUnifyP(Eqn_p eq1, Eqn_p eq2)`
- `bool LiteralGreater(OCB_p ocb, Eqn_p eq1, Eqn_p eq2)`
- `bool LiteralSubsumeP(Eqn_p subsumer, Eqn_p subsumed)`
- `bool LiteralUnifyOneWay(Eqn_p eq1, Eqn_p eq2, Subst_p subst, bool swapped)`
- `double EqnDAGWeight(Eqn_p eq, double uniqmax_multiplier, double max_multiplier, long vweight, long fweight, long dup_weight, bool new_eqn, bool new_terms)`
- `double EqnDAGWeight2(Eqn_p eq, double maxw_multiplier, long vweight, long fweight, long dup_weight)`
- `double EqnMaxWeight(Eqn_p eq, long vweight, long fweight, double app_var_mult)`
- `double EqnNonLinearWeight(Eqn_p eq, double max_multiplier, long vlweight, long vweight, long fweight, double app_var_mult)`
- `double EqnSymTypeWeight(Eqn_p eq, double max_multiplier, long vweight, long fweight, long cweight, long pweight, double app_var_mult)`
- `double EqnWeight(Eqn_p eq, double max_multiplier, long vweight, long fweight, double app_var_mult)`
- `double LiteralFunWeight(Eqn_p eq, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long flimit, long *fweights, long default_fweight, double app_var_mult, long* typefreqs)`
- `double LiteralNonLinearWeight(Eqn_p eq, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vlweight, long vweight, long fweight,double app_var_mult,bool count_eq_encoding)`
- `double LiteralSymTypeWeight(Eqn_p eq, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, long cweight, long pweight, double app_var_mult)`
- `double LiteralTermExtWeight(Eqn_p eq, TermWeightExtension_p twe)`
- `double LiteralWeight(Eqn_p eq, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, long vweight, long fweight, double app_var_mult, bool count_eq_encoding)`
- `int EqnCanonCompareRef(const void* lit1ref, const void* l2ref)`
- `int EqnSubsumeCompare(Eqn_p l1, Eqn_p l2)`
- `int EqnSubsumeInverseCompareRef(const void* lit1ref, const void* lit2ref)`
- `int EqnSubsumeInverseRefinedCompareRef(const void* lit1ref, const void* lit2ref)`
- `int EqnSubsumeQOrderCompare(const void* lit1, const void* lit2)`
- `int EqnSyntaxCompare(const void* l1, const void* l2)`
- `int LiteralCompareFun(Eqn_p lit1, Eqn_p lit2)`
- `int LiteralSyntaxCompare(const void* l1, const void* l2)`
- `long EqnInferencePositions(Eqn_p eqn)`
- `long EqnStructWeightCompare(Eqn_p l1, Eqn_p l2)`
- `long EqnStructWeightLexCompare(Eqn_p l1, Eqn_p lit2)`
- `static inline long EqnDepth(Eqn_p eqn)`
- `void EqnFOFPrint(FILE* out, Eqn_p eq, bool negated, bool fullterms, bool pcl)`
- `void EqnFree(Eqn_p junk)`
- `void EqnMap(Eqn_p eq, TermMapper_p f, void* arg)`
- `void EqnPrint(FILE* out, Eqn_p eq, bool negated, bool fullterms)`
- `void EqnPrintDBG(FILE* out, Eqn_p eq)`
- `void EqnRecordTermDates(Eqn_p eq)`
- `void EqnSwapSides(Eqn_p eq)`
- `void EqnSwapSidesSimple(Eqn_p eq)`
- `void EqnTSTPPrint(FILE* out, Eqn_p eq, bool fullterms)`

## Implementation Notes

### Internal Functions

- `EqnDepth`
- `EqnIsUntyped`
- `compare_pos_eqns`
- `compare_poseqn_negeqn`
- `eqn_parse_mixfix`
- `eqn_parse_prefix`

### Source-Level Behavior

- `EqnDepth`: Return the depth of an equation
- `compare_pos_eqns`: Compare two positive equations l1=r1 and l2=r2: (1) {l1,r1} == {l2,r2} <==> (l1=l2 & r1=r2) v (l1=r2 & r1=l2) Assume that {l1,r1} =/= {l2,r2}. Then, (2) {l1,r1} >> {l2,r2} <==> (l1>l2 & l1>r2) v (l1>=l2 & r1>=r2) v (r1>=l2 & l1>=r2) v (r1>l2 & r1>r2) (3) {l1,r1} << {l2,r2} <==> (l1<l2 & r1<l2) v (l1<=l2 & r1<=r2) v (r1<=l2 & l1<=r2) v (l1<r2 & r1<r2) (4) Ot...
- `compare_poseqn_negeqn`: Compare a positive equations l1=r1 and a negative equation l2=/=r2: (1) {{l1},{r1}} == {{l2,r2}}: This case is impossible! (2) {{l1},{r1}} >> {{l2,r2}} <==> (l1>l2 & l1>r2) v (r1>l2 & r1>r2) (3) {{l1},{r1}} << {{l2,r2}} <==> (l1<=l2 v l1<=r2) & (r1<=l2 v r1<=r2) (4) Otherwise, {{l1},{r1}} and {{l2,r2}} are incomparable. Assume that l1>r1 holds. Then the abo...
- `eqn_parse_prefix`: Parse a literal without external sign assuming that _all_ equational literals are prefix. Return sign. This is for TPTP format and old-style LOP.
- `eqn_parse_mixfix`: Parse a literal without external sign, allowing both infix and prefix notations (this is for mixed LOP).
- `eqn_parse_real`: Parse an equation with optional external sign and depending on wether FOF or CNF is being parsed.
- `EqnParseInfix`: Parse a literal without external sign assuming that _all_ equational literals are infix. Return sign. This is for TSTP syntax and E-LOP style.
- `EqnAlloc`: Allocate a literal with the given (shared terms). References for the terms will be added to the bank.
- `EqnAllocFlatten`: Allocates a predicate literal but makes sure that if it is a formula of the form (~)$eq(s,t) then s and t are lifted to the literal level.
- `EqnFree`: Free the space taken by an equation. Does not free the terms any more - this is left to GC.
- `EqnParse`: Parse a CNF style equation according to the current input format and return a pointer to the resulting cell.
- `EqnFOFParse`: Parse a literal in FOF format (changes syntax for TPTP literals).
- `EqnHOFParse`: Parse a literal in THF format. Because of many peculiarities with parentheses in THF, we might have to continue on parsing the formula from the point where the function has been called and then read the closing bracket :(
- `EqnTBTermEncode`: Take two terms (from bank) and a positive value and return a pointer to a TermBank-Term corresponding to the term encoding of the equation.
- `EqnTBTermDecode`: Given a term encoding of an equation, create and return a suitable Equation.
- `EqnTBTermParse`: Parse an equation, encode it as a term bank term and return a pointer to it.
- `EqnPrint`: Print an equation. If negated is true, print the negated equation. If TPTPFormatPrint is true, print TPTPFormat.
- `EqnPrintDBG`: Debug printing of the equation.
- `EqnPrintDeref`: Print a (potentially instantiated) equation (in standard infix).
- `EqnFOFPrint`: Print an equation in FOF format. For LOP/TSTP that is infix, for TPTP/PCL it is prefix.
- `EqnAppEncode`: Encodes both sides of the equation using applicative encoding. Does not change original equation.
- `EqnTSTPPrint`: Print a literal in TSTP format.
- `EqnSwapSidesSimple`: Exchange the two sides of the equation. This will lead to inconsistent states if not used carefully (i.e. only temporary or via EqnSwapSides() (which takes care of the attached strings).
- `EqnSwapSides`: Exchange the two sides of an equation. Will update type and references.
- `EqnCopy`: Create a copy of eq with terms from bank. Does not copy the next pointer. Properties of the original terms are not copied.
- `EqnFlatCopy`: Create a flat copy of eq.
- `EqnCopyRepl`: As EqnCopy(), but replace occurrences of old with repl.
- `EqnCopyReplPlain`: As EqnCopyRepl(), but copy terms uninstantiated.
- `EqnCopyOpt`: Copy an instantiated equation into the same term bank (using the common optimizations possible in that case).
- `EqnCopyDisjoint`: Copy an equation into the same term bank, but with disjoint (odd->even or vice versa) variable.
- `EqnIsACTrivial`: Return true iff the two terms are AC-equal (with respect to the AC symbols specified in the signatrue).
- `EqnTermsAreDistinct`: Return true if terms are forced distinct by built-in semi-interpretation of numbers and objects.
- `EqnIsTrue`: Return true if the equation is guranteed to evaluate to true (s=s or s!=t where s and t are objects/numbers)
- `EqnIsFalse`: Return true if the equation is guaranteed to evaluate to false.
- `EqnHasUnboundVars`: Return false if Vars(dom_side) is a superset of var(other_side), true otherwise.
- `EqnIsDefinition`: Return true if eqn is a definition, i.e. positive, and of the form f(X1....Xn)=t with f not occuring in t and no other variables in t.
- `EqnSubsumeQOrderCompare`: Compare two equations with a quasi-ordering that ensures that only equivalent equations can subsume each other.
- `EqnSubsumeInverseCompareRef`: Compute a refinement of the inverse of the previous ordering such that a smaller literal can never subsume a larger ond
- `EqnSubsumeInverseRefinedCompareRef`: A refined version of the above, made total for search control purposes, but not longer strictly compatible with subsumption!
- `EqnSubsumeCompare`: Compute the inverse of the previous order, taking pointers as arguments.
- `EqnCanonize`: Bring equation into canonical form: If there is at least one $true-term, RHS is a true term. Otherwise, the bigger term (by standard weight) is the LHS. If they are equal, the one with the smaller top symbol arity is LHS. Otherwise, compare lexicographically.
- `EqnStructWeightCompare`: Compare two equation (literals) based on structural criteria only: Sign, Equality, Size, LHS structure, RHS structure. Assumes that the literals are in canonical form (see above).
- `EqnCanonCompareRef`: Compare two pointed to equations with EqnStructWeightLexCompare().
- `EqnStructWeightLexCompare`: Compare equations first by structure, then by lexical f_codes.
- `EqnEqualDeref`: Test wether two equations are equivalent (modulo commutativity). Treats equations as _unsigned_ term sets. Follows variable binding pointers as denoted by d1 and d2.
- `EqnSubsumeDirected`: Test wether an equation subsumes another one. If yes, return true and extend subst to give the substitution, otherwise just return false and let subst unmodified. Don't deal with commutativity of equality.
- `EqnSubsume`: Test wether an equation subsumes another one. If yes, return true and extend subst to give the substitution, otherwise just return true and let subst unmodifies. Equations are treated as 2-sets of terms unless both are oriented.
- `EqnSubsumeP`: Test wether subsumer subsumes subsumed, undo all side effects.
- `LiteralSubsumeP`: Return true if subsumer subsumes subsumed, false / otherwise. Checks signs!
- `EqnUnifyDirected`: Test wether two equations can be unified. If yes, return true and extend subst to give the substitution, otherwise just return false and let subst unmodified. Don't deal with commutativity of equality.
- `EqnUnify`: Test wether two equations are unifyable. If yes, return true and extend subst to give the substitution, otherwise just return true and let subst unmodifies. Equations are treated as 2-sets of terms unless both are oriented.
- `EqnUnifyP`: Test wether two equations are unifiable, undo all side effects.
- `LiteralUnifyOneWay`: Test wether two equations are unifyable, taking into account sign and direction. If yes, return true and extend subst to give the substitution, otherwise just return false and let subst unmodifies.
- `EqnSyntaxCompare`: Induce a total ordering on equations (modulo commutativity, but ignoring properties, including polarity). Assumes that terms are perfectly shared. Equality literals are smaller than non-equational literals, the rest is done by comparing term bank entry_no.
- `LiteralSyntaxCompare`: Induce a total ordering on literals (modulo commutativity). Assumes that terms are perfectly shared. Negative literals are bigger than positive ones, equality literals are smaller than non-equational literals, the rest is done by comparing term bank entry_no.
- `EqnOrient`: Orient an equation. Return true, if sides are exchanged, false otherwise.
- `EqnCompare`: Compare two equations (as multisets of terms) and return the result.
- `EqnGreater`: Return true if eq1 is greater than eq2, false otherwise.
- `LiteralCompare`: Compare two signed literals L1 and L2: L1 > L2 <==> rep(L1) >> rep(L2) where >> stands for the extension of an ordering on terms to multisets of (multisets of) terms and rep is a function mapping (negative or positive) equations to multisets (of multisets) of terms in the following way: o rep(s=t) = rep(s=/=t) = {s,t} if L1 and L2 both are positive or both...
- `LiteralGreater`: Return true if eq1 is greater than eq2, takes as (signed) literals, false otherwise.
- `SubstNormEqn`: Instantiate all variables in eq with normed variables. Returns the previous value of vars->v_count, i.e. the number of the first fresh variable used.
- `EqnWeight`: Compute the weight of an equation. Weights of potentially maximal sides are multiplied by max_multiplier. Weight of applied variables is multiplied with app_var_mult.
- `EqnDAGWeight`: Compute the DAG weight of an equation. Weights of potentially maximal sides are cpmputed first, and are multiplied by max_multiplier. If new_eqn is set, the equation is treated as a stand-alone structure. If new_terms is set, the two terms are treated as stand-alone structures.
- `EqnDAGWeight2`: Alternative DAG weight of an equation, inspired by Twee (Smallbone:CADE-202, but details via personal email): Terms are treated as individual DAGs, the bigger weight of both terms is boosted by a multiplier. Term order is not considered.
- `EqnFunWeight`: As EqnWeight(), but use weighted FSum instead of plain term weight. Weight of applied variables is multiplied with app_var_mult.
- `EqnTermExtWeight`: Compute the weight of a literal as an extension of an arbitrary term weight function. Modifiers are applied, several extensions are supported (standard - sum literal/term weights, subterms - sum weights of all subterms, or take the maximum subterm weight).
- `EqnNonLinearWeight`: Compute the non-linear weight of an equation. Weights of potentially maximal sides are multiplied by max_multiplier. Weight of applied variables is multiplied with app_var_mult.
- `EqnSymTypeWeight`: Compute the symbol type weight of an equation. Weight of applied variables is multiplied with app_var_mult.
- `EqnMaxWeight`: Compute the maximum of the weighs of the terms of an equation. Weight of applied variables is multiplied with app_var_mult.
- `EqnCorrectedWeight`: Compute the weight of an equation. Weights of potentially maximal sides are multiplied by max_multiplier. The equal-Predicate is counted with weight fweight, $true is not counted. Applied variable terms are multiplied with app_var_mult.
- `EqnCorrectedNonLinearWeight`: Compute the weight of an equation. Weights of potentially maximal sides are multiplied by max_multiplier. The equal-Predicate is counted with weight fweight, $true is not counted. Applied variable's weight is multiplied with app_var_mult.
- `EqnMaxTermPositions`: Return the number of positions in maximal terms of eqn.
- `EqnInferencePositions`: Return the number of potential inference positions in maximal terms of eqn. Variables are not inference positions.
- `LiteralWeight`: Return weight of a literal. max_term_multipler is applied to maximal sides, max_literal_multiplier to maxinal literals, pos_multiplier is applied to positive literals. If count_eq_encoding is true, count $true and ignore the equal-predicate, otherwise ignore $true and count the equal-predicate for equations only. Applied variable's weights are multiplied by...
- `LiteralFunWeight`: As LiteralWeight(), but use individual functgion symbol weights. The eq encoding is always counted. Weight of applied variables is multiplied with app_var_mult.
- `LiteralTermExtWeight`: Compute the weight of a literal as an extension of an arbitrary term weight function. Modifiers are applied, several extensions are supported (standard - sum literal/term weights, subterms - sum weights of all subterms, or take the maximum subterm weight).
- `LiteralNonLinearWeight`: Return weight of a literal. max_term_multipler is applied to maximal sides, max_literal_multiplier to maxinal literals, pos_multiplier is applied to positive literals. If count_eq_encoding is true, count $true and ignore the equal-predicate, otherwise ignore $true and count the equal-predicate for equations only. Weight of applied variables is multiplied wi...
- `LiteralSymTypeWeight`: Return weight of a literal. max_term_multipler is applied to maximal sides, max_literal_multiplier to maxinal literals, pos_multiplier is applied to positive literals. Different weights are used for predicate symbols, constants, function symbols and variables. Weight of applied variables is multiplied with app_var_mult.
- `LiteralCompareFun`: Comparison function for technical stuff, i.e. trees and so on.
- `EqnAddSymbolFeatures`: Add symbol features to the feature array.
- `EqnCollectSubterms`: Collect all subterms of eqn onto collector. Assumes that TPOpFlag is set if and only if the term is already in the collection. Returns the number of new terms found.
- `EqnHasAppVar`: Does eq have an applied variable at any side?
- `EqnListMapTerms`: Map all terms in the equation list using f.

### Dependencies

- `"ccl_eqn.h"`
- `"ccl_tformulae.h"`
- `"cte_typecheck.h"`
- `<cte_acterms.h>`
- `<cte_match_mgu_1-1.h>`
- `<cte_replace.h>`
- `<cte_termweightext.h>`
- `<cto_orderings.h>`

### Compile-Time Conditions

- `CCL_EQN`
- `CONSTANT_MEM_ESTIMATE`
- `ENABLE_LFHO`
- `MARK_MAX_EQNS`

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

Source files reviewed: `CLAUSES/ccl_eqn.h`, `CLAUSES/ccl_eqn.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 3970 lines, 83 scanned public declarations, 6 scanned internal function definitions, and 83 structured function-comment blocks.
- Literal/equation representation. Polarity, orientation, term replacement, and rewrite-status behavior must stay synchronized with clause indexes.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.

### Compatibility Notes

- `EqnPrint` reads mutable process-global `OutputFormat`, `EqnUseInfix`, `EqnFullEquationalRep`, and `EqnPrintOriented`, and it indirectly honors `TermPrintTypes` through full-term printing. Rust keeps the rendered LOP/TPTP behavior but passes those choices through explicit `EqnPrintOptions`; prefer that explicit API unless executable-level compatibility requires recreating process globals.
- In the `TPTPFormat` branch, `EqnPrint` ignores `EqnFullEquationalRep` and always prints an external `++`/`--` sign plus prefix `equal(...)` only for equational literals. Rust preserves that dialect split in the TPTP option.
- `EqnParse`/`EqnFOFParse` dispatch through `ScannerGetFormat`, with LOP accepting optional `~` plus mixfix `equal(...)`/infix syntax, TPTP CNF requiring doubled external `++`/`--`, TPTP FOF accepting optional `~`, and TSTP using optional `~` plus infix syntax. Rust now preserves those control-flow branches for the currently ported shared simple term parser; full `TBTermParse` syntax parity remains a term-bank/parser follow-up.
- `EqnPrintDBG` delegates to `TermPrintDbg` with `DEREF_NEVER`, then appends maximal/oriented/equational markers. The equational marker uses `COMCHAR` through a `%s` argument, so the default non-`UNIX_COMMENTS` build prints the doubled string `%%`; Rust preserves that default through an explicit debug helper.
- `EqnPrintDeref` ignores output-format globals and always prints standard infix `=`/`!=` after passing the same `DerefType` to both sides' `TermPrint`. Rust preserves this as a bank-explicit helper; broader applied-variable dereference expansion remains a term-bank concern.
- `EqnAppEncode` creates temporary app-encoded term copies and frees them after printing, but it may still mutate the signature by allocating typed application symbols. Rust preserves source-literal immutability and exposes the signature mutation through a mutable `TermBank` argument.
- `EqnFOFPrint` chooses infix output only for `TSTPFormat` and non-PCL `LOPFormat`; `TPTPFormat` and LOP/PCL use prefix `equal(...)`, and unlike `EqnPrint` this helper does not emit external `++`/`--` signs. Rust preserves those branches through explicit FOF print options and keeps the higher-order parenthesis global as a caller-provided switch.
- `EqnTSTPPrint` special-cases any negative `lterm == rterm` literal as `$false` before checking equational shape, and consults the process-global `EqnPrintOriented` for `->`/`!->` output. Rust preserves the same spellings through an explicit TSTP writer and a `print_oriented` argument; the C global is a good candidate to keep explicit in future Rust call paths.
- `EqnOrient` trusts `EPMaxIsUpToDate` as a complete cache-validity guard and returns before checking whether the current side terms still match the stored orientation bit. Rust preserves that behavior and now exposes both the original immutable-bank ordering path and a bank-backed path for callers that can supply the mutable owner `TermBank` needed by KBO6 `LAMBDA_ORDER` beta/eta preparation; later side-mutation APIs should clear the flag explicitly instead of making orientation recompute defensively.
- `EqnMap` applies the mapper to both sides, rewrites mapped `$false` to `$true` with polarity flips, swaps `$true` away from the left side, recomputes `EPIsEquLiteral`, and clears `EPMaxIsUpToDate`/`EPIsOriented` only when the final left side changed. Rust preserves this through `Eqn::map_terms`; the same behavior is used by the ported equation-list lambda-normalization helper.
- `compare_poseqn_negeqn` labels one mixed positive/negative lesser branch as `Buggy, changed by StS` and uses a broad disjunction over both equation sides. Rust preserves the implementation; defer cleanup until maximal-literal and proof-search comparisons can show the change is unobservable.
- `EqnTermExtWeight` always applies `max_term_multiplier` to the left term, and applies it to the right term only when the literal is not oriented. `LiteralTermExtWeight` then applies the maximal-literal multiplier before the positive-equation multiplier. Rust preserves this order; later heuristic APIs may want names that make the left-side-as-potentially-maximal convention explicit.
- `EqnSplitModStandardWeight` checks the full `EPIsSplitLit|EPIsPositive` property mask before using the left head symbol's special weight; it does not call `EqnIsSplitLit`, so merely marking the predicate symbol with `FPClSplitDef` is not enough. The checked C snapshot references `SigGetSpecialWeight` only from this macro and does not define it elsewhere, so later cleanup should decide whether the special weight belongs to signature state, ordering-control state, or a caller-supplied policy.

### Change Later

- `EqnMap`'s left-side-only orientation/maximality invalidation can leave metadata untouched after a right-side-only rewrite. Keep this while matching C traces; after drop-in compatibility is stable, consider making any side replacement clear ordering metadata or splitting literal side mutation from truth/polarity normalization.
- C equations carry their owner bank implicitly, so `EqnOrient`, `EqnCompare`, and `LiteralCompare` can reach owner-bank normalization through `TOCompare` without changing signatures. Rust currently keeps explicit immutable-bank and mutable-bank comparison variants; collapse that split only after term-owner metadata or proof-state ownership can provide the C context without hidden global coupling.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
