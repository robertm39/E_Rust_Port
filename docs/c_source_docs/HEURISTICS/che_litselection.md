<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_litselection

## Source Files

- [HEURISTICS/che_litselection.h](../../../eprover/HEURISTICS/che_litselection.h)
- [HEURISTICS/che_litselection.c](../../../eprover/HEURISTICS/che_litselection.c)

## Purpose

Functions for selection certain literals (and hence superposition strategies). the GNU Lesser General Public License. <1> Fri May 21 22:17:06 GMT 1999

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LitEvalCell`
- `LitEval_p`
- `LitSelNameFunAssocCell`
- `LiteralSelectionFun`

### Macros And Constants

- `CHE_LITSELECTION`
- `LitEvalInit(cell)`
- `VAR_FACTOR`
- `lit_sel_diff_weight(handle)`
- `pred_dist_array_free(array)`

### Globals

- None found in the source scan.

### Exported Functions

- `LiteralSelectionFun GetLitSelFun(char* name)`
- `char* GetLitSelName(LiteralSelectionFun fun)`
- `void GSelectMinInfpos(OCB_p ocb, Clause_p clause)`
- `void HSelectMinInfpos(OCB_p ocb, Clause_p clause)`
- `void LitSelAppendNames(DStr_p str)`
- `void MSelectComplexExceptUniqMaxHorn(OCB_p ocb, Clause_p clause)`
- `void MSelectLargestOrientableLiteral(OCB_p ocb, Clause_p clause)`
- `void MSelectSmallestOrientableLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectAllCondOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectAntiRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectComplex(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexAHP(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexAHPExceptRRHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexExceptRRHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexExceptUniqMaxHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexExceptUniqMaxPosHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexPreferEQ(OCB_p ocb, Clause_p clause)`
- `void PSelectComplexPreferNEQ(OCB_p ocb, Clause_p clause)`
- `void PSelectCondOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectDepth2OptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectDiffNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectFirstVariableLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectGroundNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectLComplex(OCB_p ocb, Clause_p clause)`
- `void PSelectLargestNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectLargestOrientableLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectMaxLComplex(OCB_p ocb, Clause_p clause)`
- `void PSelectMaxLComplexNoTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectMaxLComplexNoXTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectMin2Infpos(OCB_p ocb, Clause_p clause)`
- `void PSelectMinInfpos(OCB_p ocb, Clause_p clause)`
- `void PSelectMinInfposNoTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectMinOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectMinOptimalNoRXTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectMinOptimalNoTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectMinOptimalNoXTypePred(OCB_p ocb, Clause_p clause)`
- `void PSelectNDepth2OptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectNegativeLiterals(OCB_p ocb, Clause_p clause)`
- `void PSelectNewComplex(OCB_p ocb, Clause_p clause)`
- `void PSelectNewComplexAHP(OCB_p ocb, Clause_p clause)`
- `void PSelectNewComplexAHPExceptRRHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectNewComplexAHPExceptUniqMaxHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectNewComplexExceptUniqMaxHorn(OCB_p ocb, Clause_p clause)`
- `void PSelectNonAntiRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectNonRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectNonStrongRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectPDepth2OptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectSmallestNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectSmallestOrientableLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectStrongRRNonRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectUnlessPosMaxOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectUnlessUniqMaxOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectUnlessUniqMaxPosOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void PSelectUnlessUniqMaxSmallestOrientable(OCB_p ocb, Clause_p clause)`
- `void PSelectUnlessUniqPosMaxOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectAllCondOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectAntiRROptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectCQAr(OCB_p ocb, Clause_p clause)`
- `void SelectCQArEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQArEqLast(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNT(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNTEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNTEqFirstUnlessPDom(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNTNp(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNTNpEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNXTEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNp(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNpEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQArNpEqFirstUnlessPDom(OCB_p ocb, Clause_p clause)`
- `void SelectCQGrArEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIAr(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArEqLast(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNT(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNTEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNTNp(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNTNpEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNXTEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNp(OCB_p ocb, Clause_p clause)`
- `void SelectCQIArNpEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectCQIPrecW(OCB_p ocb, Clause_p clause)`
- `void SelectCQIPrecWNTNp(OCB_p ocb, Clause_p clause)`
- `void SelectCQPrecW(OCB_p ocb, Clause_p clause)`
- `void SelectCQPrecWNTNp(OCB_p ocb, Clause_p clause)`
- `void SelectComplex(OCB_p ocb, Clause_p clause)`
- `void SelectComplexAHP(OCB_p ocb, Clause_p clause)`
- `void SelectComplexAHPExceptRRHorn(OCB_p ocb, Clause_p clause)`
- `void SelectComplexExceptRRHorn(OCB_p ocb, Clause_p clause)`
- `void SelectComplexExceptUniqMaxHorn(OCB_p ocb, Clause_p clause)`
- `void SelectComplexExceptUniqMaxPosHorn(OCB_p ocb, Clause_p clause)`
- `void SelectComplexG(OCB_p ocb, Clause_p clause)`
- `void SelectComplexPreferEQ(OCB_p ocb, Clause_p clause)`
- `void SelectComplexPreferNEQ(OCB_p ocb, Clause_p clause)`
- `void SelectCondOptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectDepth2OptimalLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectDiffNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectDiversificationLiterals(OCB_p ocb, Clause_p clause)`
- `void SelectDiversificationPreferIntoLiterals(OCB_p ocb, Clause_p clause)`
- `void SelectFirstVariableLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectGrCQArEqFirst(OCB_p ocb, Clause_p clause)`
- `void SelectGroundNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectLComplex(OCB_p ocb, Clause_p clause)`
- `void SelectLargestNegativeLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectLargestOrientableLiteral(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplex(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexAPPNTNp(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexAPPNoType(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexAvoidAppVar(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexAvoidPosPred(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexAvoidPosUPred(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexG(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexNoTypePred(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexNoXTypePred(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexPreferAppVar(OCB_p ocb, Clause_p clause)`
- `void SelectMaxLComplexStronglyAvoidAppVar(OCB_p ocb, Clause_p clause)`
- `void SelectMin2Infpos(OCB_p ocb, Clause_p clause)`
- `void SelectMinInfpos(OCB_p ocb, Clause_p clause)`
- `void SelectMinInfposNoTypePred(OCB_p ocb, Clause_p clause)`
- `void SelectMinOptimalLiteral(OCB_p ocb, Clause_p clause)`
- ... 27 more

## Implementation Notes

### Internal Functions

- `clause_select_pos`
- `complex_weight`
- `complex_weight_ahp`
- `diversification_prefer_into_weight`
- `diversification_weight`
- `find_lcomplex_literal`
- `find_max_xtype_no_type_lit`
- `find_maxlcomplex_literal`
- `find_ng_min11_infpos_no_xtype_lit`
- `find_smallest_max_neg_ground_lit`
- `find_smallest_neg_ground_lit`
- `maxlcomplex_weight`
- `maxlcomplexappNTNp_weight`
- `maxlcomplexavoidappvar_weight`
- `maxlcomplexavoidpred_weight`
- `maxlcomplexavoidprednotype_weight`
- `maxlcomplexstronglyavoidappvar_weight`
- `maxlcomplexstronglypreferappvar_weight`
- `new_complex_notp_ahp`
- `new_complex_notp_ahp_ns`
- `pos_pred_dist_array_compute`
- `select_cq_ar_eqf_weight`
- `select_cq_ar_eql_weight`
- `select_cq_ar_weight`
- `select_cq_arnp_eqf_weight`
- `select_cq_arnp_weight`
- `select_cq_arnt_eqf_weight`
- `select_cq_arnt_weight`
- `select_cq_arntnp_eqf_weight`
- `select_cq_arntnp_weight`
- `select_cq_arnxt_eqf_weight`
- `select_cq_iar_eqf_weight`
- `select_cq_iar_eql_weight`
- `select_cq_iar_weight`
- `select_cq_iarnp_eqf_weight`
- `select_cq_iarnp_weight`
- `select_cq_iarnt_eqf_weight`
- `select_cq_iarnt_weight`
- `select_cq_iarntnp_eqf_weight`
- `select_cq_iarntnp_weight`
- `select_cq_iarnxt_eqf_weight`
- `select_cq_iprecw_weight`
- `select_cq_iprecwntnp_weight`
- `select_cq_precw_weight`
- `select_cq_precwntnp_weight`
- `select_cqgr_ar_eqf_weight`
- `select_grcq_ar_eqf_weight`

### Source-Level Behavior

- `find_maxlcomplex_literal`: Find a maximal negative literal to select (see SelectMaxLComplex() below.
- `find_lcomplex_literal`: Find a non-maximal negative literal to select (see SelectComplex() below.
- `find_smallest_neg_ground_lit`: Return smallest negative ground literal, or NULL if no negative ground literal exists.
- `find_smallest_max_neg_ground_lit`: Return the ground literal with the smallest maximal side. Assumes that all literals have been oriented (if possible).
- `find_ng_min11_infpos_no_xtype_lit`: Return the non-ground, non-xpos literal with the smallest number of inference positions.
- `find_max_xtype_no_type_lit`: Return the biggest xtype literal (but never a type literal).
- `clause_select_pos`: Select all positive literals in clause.
- `lit_eval_compare`: Return integer smaller than 0, 0, or int > than zero if le1 is smaller, equal to, or larger than le2 (by weight). Highest priority is implicit sign!
- `generic_uniq_selection`: Function implementing generic weight-based selection for cases where at most one negative literal is selected (the one which is assigned minimal weight by weight_fun).
- `pos_pred_dist_array_compute`: Compute a distribution array of predicate symbols (or uninterpreted predicate symbols in positive literals of clause.
- `generic_app_var_sel`: Factors out computation needed for Avoid/PrefferAppVar family of functions.
- `GetLitSelFun`: Given an external name, return a literal selection function or NULL if the name does not match any known function.
- `GetLitSelName`: Given a LiteralSelectionFun, return the corresponding name. Fails/Undefined, if function is not found.
- `LitSelAppendNames`: Append all valid literal selection function names (comma-separated) to str.
- `SelectNoLiterals`: Unselect all literals (now a dummy, this is done further up).
- `SelectNoGeneration`: Do nothing with a clause.
- `SelectNegativeLiterals`: If the clause has negative literals, mark them as selected.
- `PSelectNegativeLiterals`: If the clause has negative literals, mark all literals as selected.
- `SelectFirstVariableLiteral`: Select first literal of the form X!=Y.
- `PSelectFirstVariableLiteral`: If a literal of the form X!=Y exist, select it and all positive literals. Otherwise unselect all literals.
- `SelectLargestNegativeLiteral`: Select the largest of the clauses negative literals.
- `PSelectLargestNegativeLiteral`: If clause has negative literals, select the largest of the clauses negative literals and positive literals.
- `SelectSmallestNegativeLiteral`: Select the smallest of the clauses negative literals.
- `PSelectSmallestNegativeLiteral`: If clause has negative literals, select the smallest of the clauses negative literals and positive literals.
- `SelectLargestOrientableLiteral`: If there is at least one negative orientable literal, select the largest one, otherwise select the largest one.
- `PSelectLargestOrientableLiteral`: If there is at least one negative orientable literal, select the largest one, otherwise select the largest one. Also select positive literals.
- `MSelectLargestOrientableLiteral`: For horn clauses, call PSelectLargestOrientableLiteral, otherwise call SelectLargestOrientableLiteral.
- `SelectSmallestOrientableLiteral`: If there is at least one negative orientable literal, select the smallest one, otherwise select the largest one.
- `PSelectSmallestOrientableLiteral`: If there is at least one negative orientable literal, select the smallest one, otherwise select the largest one. Also select positive literals.
- `MSelectSmallestOrientableLiteral`: For horn clauses, call PSelectSmallestOrientableLiteral, otherwise call SelectSmallestOrientableLiteral.
- `SelectDiffNegativeLiteral`: Select the most unbalanced of the clauses negative literals.
- `PSelectDiffNegativeLiteral`: If clause has negative literals, select the most unbalanced one of the clauses negative literals and all positive literals.
- `SelectGroundNegativeLiteral`: If there are negative ground literals, select the one with maximal lit_sel_diff_weight.
- `PSelectGroundNegativeLiteral`: If there are negative ground literals, select the one with maximal lit_sel_diff_weight and select all positive literals.
- `SelectOptimalLiteral`: (Hah! Believe it at your peril ;-). If there is a ground negative literal, select it, otherwise select the negative literal with the largest size difference.
- `PSelectOptimalLiteral`: (Hah! Believe it at your peril ;-). If there is a ground negative literal, select it, otherwise select the negative literal with the largest size difference and all positive literals.
- `SelectMinOptimalLiteral`: If there is a ground negative literal, select it, otherwise select the smallest negative literal.
- `PSelectMinOptimalLiteral`: If there is a ground negative literal, select it, otherwise select the smallest negative literal and positive literals.
- `SelectMinOptimalNoTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select type literals.
- `PSelectMinOptimalNoTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select type literals. If a negative literal is selected, also select positive ones.
- `SelectMinOptimalNoXTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select extendet type literals.
- `PSelectMinOptimalNoXTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select extendet type literals. If a negative literal is selected, also select positive ones.
- `SelectMinOptimalNoRXTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select real extended type literals.
- `PSelectMinOptimalNoRXTypePred`: If there is a ground negative literal, select it, otherwise select the smallest negative literal, but never select real extendet type literals. If a negative literal is selected, also select positive ones.
- `SelectCondOptimalLiteral`: As above, but if the clause has a positive literal that is very uninstantiated, select no literal at all.
- `PSelectCondOptimalLiteral`: As above, but select positive literals as well.
- `SelectAllCondOptimalLiteral`: As above, but if the clause has only positive literals that are very uninstantiated, select no literal at all.
- `PSelectAllCondOptimalLiteral`: As above, but select positive literals as well.
- `SelectDepth2OptimalLiteral`: Select optimal literal unless there is a literal with depth <= 2, then select no literal.
- `PSelectDepth2OptimalLiteral`: As above, but select positive literals as well.
- `SelectPDepth2OptimalLiteral`: Select optimal literal unless there is a positive literal with depth <= 2, then select no literal.
- `PSelectPDepth2OptimalLiteral`: As above, with positive literals.
- `SelectNDepth2OptimalLiteral`: Select optimal literal unless there is a negative literal with depth <= 2, then select no literal.
- `PSelectNDepth2OptimalLiteral`: As above, with positive literals.
- `SelectNonRROptimalLiteral`: If a clause is not range-restricted, select the optimal literal, otherwise select no literal.
- `PSelectNonRROptimalLiteral`: If a clause is not range-restricted, select the optimal literal and positive literals, otherwise select no literal.
- `SelectNonStrongRROptimalLiteral`: If a clause is not strongly range-restricted, select the optimal literal, otherwise select no literal.
- `PSelectNonStrongRROptimalLiteral`: If a clause is not Strong range-restricted, select the optimal literal and positive literals, otherwise select no literal.
- `SelectAntiRROptimalLiteral`: If a clause is anti-range-restricted, select the optimal literal, otherwise select no literal.
- `PSelectAntiRROptimalLiteral`: If a clause is anti-range-restricted, select the optimal literal and positive literals, otherwise select no literal.
- `SelectNonAntiRROptimalLiteral`: If a clause is not anti-range-restricted, select the optimal literal, otherwise select no literal.
- `PSelectNonAntiRROptimalLiteral`: If a clause is not anti-range-restricted, select the optimal literal and positive literals, otherwise select no literal.
- `SelectStrongRRNonRROptimalLiteral`: If a clause is not range-restricted or strongly range-restricted select the optimal literal, otherwise select no literal.
- `PSelectStrongRRNonRROptimalLiteral`: If a clause is not range-restricted or strongly range-restricted select the optimal literal and positive literals, otherwise select no literal.
- `SelectUnlessUniqMaxOptimalLiteral`: If a clause has a single maximal literal, do not select, otherwise select the optimal literal.
- `PSelectUnlessUniqMaxOptimalLiteral`: If a clause has a single maximal literal, do not select, otherwise select the optimal literal.
- `SelectUnlessUniqMaxSmallestOrientable`: If a clause has a single maximal literal, do not select, otherwise select the smallest orientable literal.
- `PSelectUnlessUniqMaxSmallestOrientable`: If a clause has a single maximal literal, do not select, otherwise select the smallest orientable literal and all positive ones.
- `SelectUnlessPosMaxOptimalLiteral`: If a clause has a positive maximal literal (i.e. is potentially reductive), do not select, otherwise select optimal literal.
- `PSelectUnlessPosMaxOptimalLiteral`: If a clause has a positive maximal literal (i.e. is potentially reductive), do not select, otherwise select optimal literal and positive literals.
- `SelectUnlessUniqPosMaxOptimalLiteral`: If a clause has a uniqe positive maximal literal do not select, otherwise select optimal literal.
- `PSelectUnlessUniqPosMaxOptimalLiteral`: If a clause has a uniqe positive maximal literal do not select, otherwise select optimal literal and positive literals.
- `SelectUnlessUniqMaxPosOptimalLiteral`: If a clause has a maximal literal that is positive, do not select, otherwise select optimal literal.
- `PSelectUnlessUniqMaxPosOptimalLiteral`: If a clause has a maximal literal that is positive, do not select, otherwise select optimal literal.
- `SelectComplex`: If there is a pure variable literal, select it. Otherwise, if there is at least one ground literal, select the smallest one. Otherwise, select the literal with the largest size difference.
- `PSelectComplex`: If there is a pure variable literal, select it. Otherwise, if there is at least one ground literal, select the smallest one. Otherwise, select the literal with the largest size difference and the positive literals.
- `SelectComplexExceptRRHorn`: If a clause is Horn and range-restricted, do no select. Otherwise use SelectComplex() (above).
- `PSelectComplexExceptRRHorn`: If a clause is Horn and range-restricted, do no select. Otherwise use PSelectComplex() (above).
- `SelectLComplex`: Similar to SelectComplex, but always select largest diff literals first.
- `PSelectLComplex`: Similar to PSelectComplex, but always select largest diff literals first.
- `SelectMaxLComplex`: If there is more than one maximal literal, select a negative literal, with the following priority: Maximal, pure variable Maximal, largest difference ground Maximal, largest difference non-ground pure variable largest difference ground largest difference non-ground
- `PSelectMaxLComplex`: As above, but in the default case select positive literals as well.
- `SelectMaxLComplexNoTypePred`: If there is more than one maximal literal, select a negative literal, with the following priority: Maximal, pure variable Maximal, largest difference ground Maximal, largest difference non-ground pure variable largest difference ground largest difference non-ground Never select a type literal. If all negative literals are type literals, select nothing.
- `PSelectMaxLComplexNoTypePred`: If there is more than one maximal literal, select a negative literal, with the following priority: Maximal, pure variable Maximal, largest difference ground Maximal, largest difference non-ground pure variable largest difference ground largest difference non-ground Never select a type literal. If all negative literals are type literals, select nothing. If s...
- `SelectMaxLComplexNoXTypePred`: If there is more than one maximal literal, select a negative literal, with the following priority: Maximal, pure variable Maximal, largest difference ground Maximal, largest difference non-ground pure variable largest difference ground largest difference non-ground Never select an extended type literal P(X1,...,Xn). If all negative literals are extended typ...
- `PSelectMaxLComplexNoXTypePred`: If there is more than one maximal literal, select a negative literal, with the following priority: Maximal, pure variable Maximal, largest difference ground Maximal, largest difference non-ground pure variable largest difference ground largest difference non-ground Never select an extended type literal. If all negative literals are extended type literals, s...
- `SelectComplexPreferNEQ`: Select a negative literal as in SelectComplex, but prefer non-equational literals.
- `PSelectComplexPreferNEQ`: Select a negative literal as in PSelectComplex, but prefer non-equational literals.
- `SelectComplexPreferEQ`: Select a negative literal as in SelectComplex, but prefer equational literals.
- `PSelectComplexPreferEQ`: Select a negative literal as in PSelectComplex, but prefer equational literals.
- `SelectComplexExceptUniqMaxHorn`: Select literal as in SelectComplex unless the clause is a Horn clause with a unique maximal literal.
- `PSelectComplexExceptUniqMaxHorn`: Select literal as in PSelectComplex unless the clause is a Horn clause with a unique maximal literal.
- `MSelectComplexExceptUniqMaxHorn`: For horn clauses, call PSelectComplexExceptUniqMaxHorn, otherwise call SelectComplexExceptUniqMaxHorn.
- `SelectNewComplex`: If there is a negative ground literal, select the one with the smallest maximal side. Else: Select the minimal inference position non-XType orientable negative literal. Else: Select the lagest XType literal. Never select a Type literal - if all negative literals are type literals, do not select at all.
- `PSelectNewComplex`: If there is a negative ground literal, select the one with the smallest maximal side. Else: Select the minimal inference position non-XType orientable negative literal. Else: Select the lagest XType literal. Never select a Type literal - if all negative literals are type literals, do not select at all. If anything is selected, select positive literals as we...
- `SelectNewComplexExceptUniqMaxHorn`: Select literal as in SelectNewsComplex unless the clause is a Horn clause with a unique maximal literal.
- `PSelectNewComplexExceptUniqMaxHorn`: Select literal as in PSelectNewsComplex unless the clause is a Horn clause with a unique maximal literal.
- `SelectMinInfpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights.
- `PSelectMinInfpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights, and select positive literals as well.
- `HSelectMinInfpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights. If this is not ground, select positive ones as well.
- `GSelectMinInfpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights. If this is ground, select positive ones as well.
- `SelectMinInfposNoTypePred`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights, but never select type predicates.
- `PSelectMinInfposNoTypePred`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights, but never select type predicates. If literal is selected, also select positive ones.
- `SelectMin2Infpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights for f_weight = 1, v_weight = 2
- `PSelectMin2Infpos`: Select the literal with the smallest number of potential inference positions, i.e. smallest sum of maximal size weights (as above), and select positive literals as well.
- `SelectComplexExceptUniqMaxPosHorn`: Select literal as in SelectComplex unless the clause is a Horn clause with a unique maximal positive literal.
- `PSelectComplexExceptUniqMaxPosHorn`: Select literal as in PSelectComplex unless the clause is a Horn clause with a unique maximal positive literal.
- `diversification_weight`: Assign pseudo-random weight to negative literals, 0 to positive ones.
- `SelectDiversificationLiterals`: Systematically select a pseudo-random literal in clause (where pseudo is large and random in small).
- `diversification_prefer_into_weight`: Assing pseudo-random weight to negative literals, 0 to positive ones. However, always prefer literals comming from the into clause of a paramodulation to those of a from clause.
- `SelectDiversificationPreferIntoLiterals`: Systematically select a pseudo-random literal in clause (where pseudo is large and random in small), but prefer into-literals.
- `maxlcomplex_weight`: Initialize weights to mimic SelectMaxLComplexWeight()
- `SelectMaxLComplexG`: Reimplementation of SelectMaxLComplex() using the generic literal selection framework.
- `maxlcomplexavoidpred_weight`: Initialize weights to mimic SelectMaxLComplexWeight(), but defer literals with which occur often in pred_dist.
- `maxlcomplexavoidappvar_weight`: Initialize weights to mimic SelectMaxLComplexWeight(), but defer literals with applied variables, and put them right after pure vars and defer literals which occur often in pred_dist.
- `maxlcomplexstronglyavoidappvar_weight`: Initialize weights to mimic SelectMaxLComplexWeight(), but defer literals with applied variables, and put them right after maximal lits and defer literals which occur often in pred_dist.
- `maxlcomplexstronglypreferappvar_weight`: Initialize weights to mimic SelectMaxLComplexWeight(), but prefer literals with applied variables and defer literals which occur often in pred_dist.
- `SelectMaxLComplexAvoidPosPred`: As SelectMaxLComplex, but preferably select literals that do not share the predicate symbol with a positive literal.
- `SelectMaxLComplexAvoidAppVar`: As SelectMaxLComplex, but preferably select literals that do not have applied variables and the ones that do not share the predicate symbol with a positive literal.
- `SelectMaxLComplexStronglyAvoidAppVar`: As above, but avoids app vars stronger (considers them even before maximality of literals).
- ... 47 more

### Dependencies

- `"che_litselection.h"`
- `<ccl_clauses.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CHE_LITSELECTION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_litselection.h`, `HEURISTICS/che_litselection.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 7371 lines, 151 scanned public declarations, 47 scanned internal function definitions, and 167 structured function-comment blocks.
- Literal-selection policy affects completeness and inference generation; match default and named policies carefully.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `SelectNoLiterals` and `SelectNoGeneration` assert that no literals are selected and otherwise do nothing. Their bodies rely on `DoLiteralSelection` in `che_proofcontrol.c` to clear `EPIsSelected` before any selector function is called.
- `NoSelection` and `NoGeneration` are distinct selector names and distinct function pointers in C, even though both selector bodies are no-ops. `NoGeneration` is also checked elsewhere in the proof process to suppress generating inferences, so Rust should not collapse the two strategy names.
- The simple non-orienting selectors do not require `OCB` state. `SelectNegativeLiterals`, `PSelectNegativeLiterals`, pure-variable, largest/smallest negative, diff-weight, and ground-negative selectors mutate only literal `EPIsSelected` bits. Largest/smallest/diff variants use strict `>` or `<`, so the first literal with the best score wins ties.
- `PSelectFirstVariableLiteral` does not select the pure-variable literal itself despite its comment saying it does; it only calls `clause_select_pos()` after finding a negative `X!=Y`, so only positive literals are selected.
- `PSelectGroundNegativeLiteral` first selects positive literals while scanning, but if no negative ground literal is found it clears `EPIsSelected` from the whole literal list. That makes the positive selections transient in the no-ground case.
- The largest/smallest orientable selectors call `ClauseCondMarkMaximalTerms`, then prefer any oriented negative literal over all unoriented negative literals. Within the chosen orientation class they use strict standard-weight comparisons, so ties keep the first candidate. The `P` variants mark positives during the scan before selecting the negative candidate, and the `M` variants use the positive variant only for Horn clauses.
- `lit_sel_diff_weight(handle)` is a macro equal to `100*EqnStandardDiff(handle)+EqnStandardWeight(handle)`. Several selectors recompute it when comparing and then storing the current best value rather than caching side weights separately.
- `PSelectOptimalLiteral` and `PSelectMinOptimalLiteral` select positive literals only through their fallback calls (`PSelectDiffNegativeLiteral` and `PSelectSmallestNegativeLiteral`). When their own ground-negative branch succeeds, the C bodies select only the chosen negative literal despite comments saying that positives are selected too.
- `SelectCondOptimalLiteral` and `PSelectCondOptimalLiteral` clear every selected bit if any positive literal satisfies `TermStandardWeight <= TermWeight(vweight=0, fweight=3)` over the left side, plus the right side for equational literals. The `AllCond` variants invert the gate: they select only if some positive literal fails that condition; with no positive literals, the initial `found=true` value makes them clear selected bits.
- The depth-restricted optimal selectors are exported under table names `SelectOptimalRestrDepth2`, `PSelectOptimalRestrDepth2`, `SelectOptimalRestrPDepth2`, `PSelectOptimalRestrPDepth2`, `SelectOptimalRestrNDepth2`, and `PSelectOptimalRestrNDepth2`, not under the C function names. They clear selected bits when the relevant literal scope has depth `<= 2`.
- The `SelectUnless...` optimal selectors first call `ClauseCondMarkMaximalTerms`, then either leave selected bits untouched or delegate to the optimal selector and clear `CPIsOriented`. `SelectUnlessUniqMaxSmallestOrientable` uses the same more-than-one-maximal gate but delegates to the smallest-orientable selector instead. `SelectUnlessUniqPosMax` blocks whenever exactly one positive maximal literal exists, even if another negative literal is also maximal. `SelectUnlessUniqMaxPos` blocks only when the unique maximal literal is positive; a positive maximal plus a negative maximal falls through to selection.
- `PSelectComplex` and `PSelectLComplex` select positive literals only when they fall through to `PSelectDiffNegativeLiteral`. If a pure-variable or ground negative literal is found first, the C bodies select only that negative literal despite the function comments saying positives are selected too.
- `SelectComplexExceptRRHorn` and `PSelectComplexExceptRRHorn` are pure no-op gates for Horn range-restricted clauses; direct calls leave pre-existing selected bits untouched. `SelectComplexPreferNEQ`/`EQ` use a single pass that `break`s at the first lower-priority or non-improving negative literal, so later better literals are ignored once the scan breaks.
- `SelectComplexExceptUniqMaxHorn` and `SelectComplexExceptUniqMaxPosHorn` call `ClauseCondMarkMaximalTerms` only for Horn clauses, skip selection when their unique-maximal gate succeeds, and otherwise delegate to `SelectComplex`/`PSelectComplex` before clearing `CPIsOriented`. The `M` variant uses the positive wrapper only for Horn clauses. There is no table-visible `MSelectComplexExceptUniqMaxPosHorn`.
- The old `SelectMaxLComplex`/`PSelectMaxLComplex` pair selects only when more than one literal is marked maximal. If all maximal literals are positive, it clears `CPIsOriented` and delegates to `SelectLComplex`/`PSelectLComplex`; otherwise it searches maximal negative literals by pure-variable, ground largest-difference, then non-ground largest-difference priority. The `NoTypePred`/`NoXTypePred` variants do not delegate to the public `SelectLComplex` fallback; they run the private non-maximal `find_lcomplex_literal()` helper and apply the type filter only to the single selected candidate.
- `SelectMaxLComplexG` reimplements the old MaxLComplex priority through `generic_uniq_selection()`: maximal literals beat non-maximal by `w1`, pure variables beat other literals, ground beats non-ground, larger `lit_sel_diff_weight` wins through negative `w2`, and the process-global `literal_weight_counter` is used as a final `w3` tiebreaker modulo `clause->neg_lit_no`. The avoid-positive-predicate variants add the leading-positive predicate distribution as `w3`; `SelectMaxLComplexAvoidPosUPred` first forces distribution slot `0` to zero. The app-var variants add `20` or `200` to `w1` when avoiding app vars, while `SelectMaxLComplexPreferAppVar` adds `200` to non-app-var literals. `SelectMaxLComplexAPPNTNp` gives propositional/type literals a very large `w1` and marks them forbidden, while `SelectMaxLComplexAPPNoType` runs ordinary avoid-positive-predicate scoring and then only marks type predicates forbidden.
- `SelectNewComplex` first orients/maximal-marks the clause, then selects a negative ground literal with the smallest cached left-term weight, else a non-ground non-XType negative with minimal `TermWeight(...,1,1)` inference-position estimate, else the largest cached-weight XType negative that is not also a type predicate. It clears `CPIsOriented` only after actually selecting. The `P` variant selects positives only after a negative literal is selected, and the unique-max Horn wrappers return before delegation when the Horn gate finds exactly one maximal literal.
- `SelectMinInfpos` and its `P`/`H`/`G` variants call `ClauseCondMarkMaximalTerms`, then choose the first negative literal with the strict smallest `TermStandardWeight(lterm)` plus `TermStandardWeight(rterm)` only when the literal is not oriented. The ordinary and `H`/`G` bodies assert a selected negative and always clear `CPIsOriented`; `PSelectMinInfpos` marks positives during its scan, while `H` marks positives only when the selected negative is non-ground and `G` only when it is ground. The `NoTypePred` variants filter every negative candidate before scoring and do nothing, including preserving `CPIsOriented`, when no non-type negative exists.
- `SelectComplexAHP`/`PSelectComplexAHP` and the NewComplex AHP family use `generic_uniq_selection()` with ordering enabled, so literal sign, `w1`, `w2`, and `w3` are compared in ascending order and ties keep the first candidate. `pos_pred_dist_array_compute()` counts only leading positive literals, and AHP `w3` penalizes a candidate whose left-head predicate code occurs in that distribution. The NewComplex AHP non-ground non-XType branch uses `EqnMaxTermPositions`, not the `TermWeight(...,1,1)` estimate used by base NewComplex. `SelectNewComplexAHPNS` checks split literals before the ground branch, and the AHP RR-Horn and unique-max Horn wrappers are pure no-op gates when their C predicates match.
- `SelectVGNonCR` first selects a negative pure-variable literal without maximal marking, then after `ClauseCondMarkMaximalTerms` selects the smallest standard-weight negative ground literal, then blocks only on a unique positive maximal literal, and otherwise delegates to the `SelectNewComplexAHPNS` scoring path. The pure-variable and ground branches do not clear `CPIsOriented`; only the delegated generic path clears it after selection.
- The `SelectCQ...` arity family uses `generic_uniq_selection()` with ordering enabled. Equality literals and free-variable-left literals get the special equality weights (`-100000`, `-1000000`, `100000`, or `1000000` depending on variant) or the normal equality weights `-2`/`2`; ordinary predicates use signed arity in `w1`, `SigGetAlphaRank` in `w2`, and `lit_sel_diff_weight` in `w3`. Type/propositional/XType filters set a large `w1` and `forbidden=true`, so a best-but-forbidden candidate can block selection. `SelectCQGrArEqFirst` prefers ground literals only by subtracting `2000000` from `w2`; the private non-table `SelectGrCQArEqFirst` instead biases `w1`.
- `SelectCQPrecW`/`SelectCQIPrecW` use `OCBFunPrecWeight(lterm->f_code)` or its negation for non-free-variable left sides, with alpha rank as the next tiebreaker. Their `NTNp` variants check the type/propositional filter before setting alpha rank, leaving rejected candidates with only the large forbidden `w1` plus `w3`.
- `SelectCQArNpEqFirstUnlessPDom` and `SelectCQArNTEqFirstUnlessPDom` mark maximal terms, collect predicate codes from positive maximal literals, and skip selection when any negative literal shares one of those predicate codes. A blocked direct call leaves selected bits and `CPIsOriented` untouched.
- The range-restriction optimal wrappers are thin gates over `ClauseIsRangeRestricted`, `ClauseIsAntiRangeRestricted`, and `ClauseIsStronglyRangeRestricted`. `SelectAntiRROptimalLiteral` has an explicit zero-negative-literal return, while `PSelectAntiRROptimalLiteral` relies on the outer `DoLiteralSelection` negative-literal gate. `PSelectStrongRRNonRROptimalLiteral` actively clears selected bits in the range-restricted-but-not-strongly-range-restricted case.
- `SelectDiversificationLiterals` and `SelectDiversificationPreferIntoLiterals` use `generic_uniq_selection()` with `needs_ordering=false`, so they do not mark maximal terms. The file-static `literal_weight_counter` increments once per literal, including positives, and negative candidates use the counter modulo `clause->neg_lit_no`. The prefer-into variant puts `-ClauseQueryProp(literal, EPIsPMIntoLit)` in `w1` before using the diversification value in `w2`.
- `GetLitSelFun` and `GetLitSelName` are table-driven. Reverse lookup scans function pointers and returns the first matching name, so table order is part of the printable strategy surface.

### Change-Later Observations

- The literal-selection unit mixes several families of selectors with repeated "P" positive-literal variants and many near-duplicate weight helpers. Keep the initial Rust port close to the table and wrappers, but consider factoring shared scoring code only after reference tests cover representative selectors from each family.
- Many selector functions call `ClauseDelProp(clause, CPIsOriented)` after selecting a literal even though `DoLiteralSelection` already cleared the clause-oriented property before dispatch. Preserve the redundant invalidation while porting selector bodies; it may still document orientation-cache assumptions for selectors that orient or inspect maximality internally.
- Several comments describe older behavior that no longer matches the code, notably `PSelectFirstVariableLiteral`, `PSelectGroundNegativeLiteral`, `PSelectOptimalLiteral`, and `PSelectMinOptimalLiteral`. Prefer source-code behavior over comments for compatibility, and revisit the comments only after reference strategy tests prove the difference is unobservable.
- The `SelectSmallestOrientableLiteral` and `PSelectSmallestOrientableLiteral` comments say the no-orientable fallback selects the largest literal, but the implementation uses `< select_weight` and therefore selects the smallest standard-weight negative. Keep the implementation behavior until reference traces show whether the prose was intended.
- The positive-variant optimal wrappers repeat mostly identical gate/fallback structure with small selected-bit differences. Keep them literal during the port, but after selector-level reference tests exist, consider consolidating the wrappers behind explicit gate and fallback helpers.
- The conditional selector comments describe "very uninstantiated" literals, but the actual threshold is the raw `TermStandardWeight <= TermWeight(...,0,3)` comparison and the all-positive variant treats the absence of positive literals as a blocked case. Keep that source behavior for now; a future user-facing strategy explanation should probably spell out the formula.
- The unless-maximal gates expose two cleanup candidates: direct calls that block do not clear stale `EPIsSelected` bits, and the similarly named `UniqPosMax`/`UniqMaxPos` variants have materially different semantics. The smallest-orientable gate also maximal-marks before delegating to another selector that repeats maximal marking and orientation-cache invalidation. Keep these shapes for compatibility until full proof-search traces cover the public strategy names.
- The complex-selector comments also overstate positive-literal selection for `PSelectComplex` and `PSelectLComplex`. The prefer-EQ/NEQ variants' early `break` behavior may be an optimization for expected literal ordering, but it is observable; revisit only after reference traces show whether continuing the scan would be safe.
- The unique-max Horn complex wrappers have asymmetric public names and side effects: the direct gate return leaves stale selected bits and orientation flags untouched, but the delegated path clears `CPIsOriented` after `SelectComplex`/`PSelectComplex`. Keep that distinction while compatibility is the priority, but consider whether a clearer post-port API should separate "gate matched" from "selector delegated".
- The `SelectMaxLComplexNoTypePred` and `SelectMaxLComplexNoXTypePred` comments imply they select nothing only when all negative literals are filtered type predicates, but the C code filters after choosing one maximal or non-maximal candidate. A later non-type candidate is ignored if the chosen candidate is filtered out; preserve this for compatibility and reconsider after strategy-level traces exist.
- The generic MaxLComplex variants add two more compatibility wrinkles: `SelectMaxLComplexAPPNoType` can select nothing when a normally best type predicate is forbidden even if a later non-type literal is available, and `SelectMaxLComplexAvoidPosUPred` lacks the zero-negative-literal guard used by sibling generic MaxLComplex selectors. Keep both while matching C, but a later Rust API could make "best allowed candidate" and "requires a negative literal" explicit.
- `SelectNewComplex` says it never selects type literals, but the first ground-literal branch does not call `EqnIsTypePred`; the later XType fallback also depends on the overlapping weight-based `EqnIsXTypePred`/`EqnIsTypePred` macros, where low-arity XType-shaped predicates can be filtered as type predicates. Keep the macro behavior and revisit user-facing strategy wording later.
- The min-infpos family contains another positive-variant asymmetry: `PSelectMinInfpos` marks positives before the negative-selection assertion, but `PSelectMinInfposNoTypePred` marks positives only after a non-type negative was found. Preserve that direct-call behavior for now, but a later API could separate "score a negative" from "decorate positives" and let the proof-control wrapper own the no-negative precondition.
- `generic_uniq_selection()` chooses the best candidate before checking `forbidden`, so a best-but-forbidden literal blocks selection even when a later allowed negative exists. The AHP selector family also relies on positive literals being clustered at the front because `pos_pred_dist_array_compute()` stops at the first non-positive. Preserve both behaviors for compatibility, but a post-port API could score only allowed candidates and make the literal-order precondition explicit.
- The CQ selector family has many near-duplicate C weight helpers with magic constants and subtle differences in whether filtering preserves or skips alpha-rank assignment. Rust should keep the declarative spec close to C for now, but after strategy traces exist the repeated helpers are good candidates for a clearer scoring API with explicit "forbidden candidate blocks" behavior.
- `SelectVGNonCR` and the `UnlessPDom` CQ wrappers rely on the outer proof-control wrapper to start from a clean selected-bit state, yet direct no-op branches preserve stale selected bits and orientation flags. Keep this direct-call compatibility, but a later Rust API should make "gate blocked" observable instead of encoding it only through unchanged side effects.
- `select_unless_pdom()` uses `EqnGetPredCode`, which is problem-type dependent in C. The current Rust port uses first-order predicate codes consistently with the rest of the literal-selection helpers; revisit this if the higher-order problem-type global starts affecting selector behavior.
- Diversification selector state is process-global (`static long literal_weight_counter`) and the C evaluation weights are `int`. Preserve the sequence for compatibility, but revisit counter ownership, deterministic reset hooks, and native-width overflow behavior if Rust later supports parallel proof search or target-specific C reference builds.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
