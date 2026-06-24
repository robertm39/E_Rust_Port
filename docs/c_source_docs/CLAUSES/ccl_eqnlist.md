<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_eqnlist

## Source Files

- [CLAUSES/ccl_eqnlist.h](../../../eprover/CLAUSES/ccl_eqnlist.h)
- [CLAUSES/ccl_eqnlist.c](../../../eprover/CLAUSES/ccl_eqnlist.c)

## Purpose

Functions for dealing with (singly linked) lists of equations as used in clauses. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermPredicateFun_p`

### Macros And Constants

- `CCL_EQNLIST`
- `EQN_LIST_LONG_LIMIT`
- `EqnListDeleteFirst(list)`
- `EqnListExistsTerm(list, predicate)`
- `EqnListExtractFirst(list)`
- `EqnListInsertFirst(list, element)`
- `EqnListTermDelProp(list, props)`
- `EqnListTermSetProp(list, props)`
- `NormSubstEqnList(list, subst, vars)`

### Globals

- None found in the source scan.

### Exported Functions

- `EqnListDeleteElement(list) void EqnListInsertElement(EqnRef pos, Eqn_p element)`
- `EqnListExtractElement(list) Eqn_p EqnListExtractByProps(EqnRef list, EqnProperties props, bool negate)`
- `EqnListInsertElement((list), (element)) Eqn_p EqnListAppend(EqnRef list, Eqn_p newpart)`
- `EqnListSignedTermDelProp((list),(props), true,true) long EqnListTBTermDelPropCount(Eqn_p list, TermProperties props)`
- `EqnListSignedTermSetProp((list),(props), true,true) void EqnListSignedTermDelProp(Eqn_p list, TermProperties props, bool pos, bool neg)`
- `Eqn_p EqnListCopy(Eqn_p list, TB_p bank)`
- `Eqn_p EqnListCopyDisjoint(Eqn_p list)`
- `Eqn_p EqnListCopyExcept(Eqn_p list, Eqn_p except, TB_p bank)`
- `Eqn_p EqnListCopyOpt(Eqn_p list)`
- `Eqn_p EqnListCopyOptExcept(Eqn_p list, Eqn_p except)`
- `Eqn_p EqnListCopyRepl(Eqn_p list, TB_p bank, Term_p old, Term_p repl)`
- `Eqn_p EqnListCopyReplPlain(Eqn_p list, TB_p bank, Term_p old, Term_p repl)`
- `Eqn_p EqnListExtractElement(EqnRef element)`
- `Eqn_p EqnListFindNegPureVarLit(Eqn_p list)`
- `Eqn_p EqnListFindTrue(Eqn_p list)`
- `Eqn_p EqnListFlatCopy(Eqn_p list)`
- `Eqn_p EqnListFromArray(Eqn_p* array, int lenght)`
- `Eqn_p EqnListFromStack(PStack_p stack)`
- `Eqn_p EqnListNegateEqns(Eqn_p list)`
- `Eqn_p EqnListParse(Scanner_p in, TB_p bank, TokenType sep)`
- `FunCode NormSubstEqnListExcept(Eqn_p list, Eqn_p except, Subst_p subst, VarBank_p vars)`
- `NormSubstEqnListExcept((list), NULL, (subst), (vars)) long EqnListDepth(Eqn_p list)`
- `PStack_p EqnListToStack(Eqn_p list)`
- `bool EqnListEqnIsMaximal(OCB_p ocb, Eqn_p list, Eqn_p eqn)`
- `bool EqnListEqnIsStrictlyMaximal(OCB_p ocb, Eqn_p list, Eqn_p eqn)`
- `bool EqnListExistsTermExcept(Eqn_p list, Eqn_p except, TermPredicateFun_p predicate)`
- `bool EqnListFindCompLitExcept(Eqn_p xs, Eqn_p exc, Eqn_p ys, DerefType d_x, DerefType d_y)`
- `bool EqnListIsACTrivial(Eqn_p list)`
- `bool EqnListIsEquational(Eqn_p list)`
- `bool EqnListIsGround(Eqn_p list)`
- `bool EqnListIsPureEquational(Eqn_p list)`
- `bool EqnListIsTrivial(Eqn_p list)`
- `bool EqnLongListIsTrivial(Eqn_p list)`
- `int EqnListDelProp(Eqn_p list, EqnProperties prop)`
- `int EqnListFlipProp(Eqn_p list, EqnProperties prop)`
- `int EqnListLength(Eqn_p list)`
- `int EqnListMaximalLiterals(OCB_p ocb, Eqn_p list)`
- `int EqnListOrient(OCB_p ocb, Eqn_p list)`
- `int EqnListQueryPropNumber(Eqn_p list, EqnProperties prop)`
- `int EqnListRemoveACResolved(EqnRef list)`
- `int EqnListRemoveDuplicates(Eqn_p list)`
- `int EqnListRemoveResolved(EqnRef list)`
- `int EqnListRemoveSimpleAnswers(EqnRef list)`
- `int EqnListSetProp(Eqn_p list, EqnProperties prop)`
- `long EqnListAddFunOccs(Eqn_p list, PDArray_p f_occur, PStack_p res_stack)`
- `long EqnListCollectFCodes(Eqn_p list, NumTree_p *tree)`
- `long EqnListCollectGroundTerms(Eqn_p list, PTree_p *res, bool pos_lits, bool neg_lits, bool all_subterms)`
- `long EqnListCollectSubterms(Eqn_p list, PStack_p collector)`
- `long EqnListCollectVariables(Eqn_p list, PTree_p *tree)`
- `void EqnListAddSymbolDistExist(Eqn_p list, long *dist_array, PStack_p exist)`
- `void EqnListAddSymbolDistribution(Eqn_p list, long *dist_array)`
- `void EqnListAddSymbolFeatures(Eqn_p list, PStack_p mod_stack, long *feature_array)`
- `void EqnListAddTypeDistribution(Eqn_p list, long *type_array)`
- `void EqnListComputeFunctionRanks(Eqn_p list, long *rank_array, long* count)`
- `void EqnListDeleteElement(EqnRef element)`
- `void EqnListFree(Eqn_p list)`
- `void EqnListGCMarkTerms(Eqn_p list)`
- `void EqnListLambdaNormalize(Eqn_p list)`
- `void EqnListMapTerms(Eqn_p list, TermMapper_p f, void* arg)`
- `void EqnListPrint(FILE* out, Eqn_p list, char* sep, bool negated, bool fullterms)`
- `void EqnListPrintDeref(FILE* out, Eqn_p list, char* sep, DerefType deref)`
- `void EqnListSignedTermSetProp(Eqn_p list, TermProperties props, bool pos, bool neg)`
- `void EqnListTSTPPrint(FILE* out, Eqn_p list, char* sep, bool fullterms)`

## Implementation Notes

### Internal Functions

- `comp_stack_eqns`

### Source-Level Behavior

- `eqn_list_find_last`: Find the last EqnRef in *list (may be list itself).
- `EqnListFree`: Deallocate the list.
- `EqnListGCMarkTerms`: Mark all terms in the eqnlist for the Garbage Collection.
- `EqnListSetProp`: Set the properties prop in all literals from list. Return the lenght of the list.
- `EqnListDelProp`: Delete the properties prop in all literals from list. Return lenght of the list.
- `EqnListFlipProp`: Delete the properties prop in all literals from list. Return lenght of the list.
- `EqnListQueryPropNumber`: Return number of equations with props set.
- `EqnListExistsTermExcept`: Return number of equations with props set.
- `EqnListLength`: Return number of equations in the list.
- `EqnListFromArray`: Convert an array of Eqn_p's into a list.
- `EqnListToStack`: Push the literals onto a newly created stack and return it. Does not copy anything! The caller has to free the stack.
- `EqnListFromStack`: Create a list from a stack of equations. The stack is destroyed and freed!
- `EqnListSplitToStacks`: Push the literals onto the provided stacks - those with prop set onto "pos", the others onto "neg".
- `EqnListExtractElement`: Take the given element out of the list and return a pointer to it.
- `EqnListExtractByProp`: Extract all equations with properties props (not) set (depending on negate).
- `EqnListDeleteElement`: Delete the given element from the list.
- `EqnListInsertElement`: Insert the element at the position defined by pos.
- `EqnListAppend`: Append newpart at the end of *list.
- `EqnListFlatCopy`: Return a flat copy of the given list, reusing the existing terms.
- `EqnListCopy`: Return a copy of the given list, with new terms from the term bank. Instantiated terms are copied as instantiations.
- `EqnListCopyExcept`: Return a copy of the given list, except for the equation given in except, with new terms from the term bank. Instantiated terms are copied as instantiations.
- `EqnListCopyOpt`: Copy an Eqnlist with the optimizations possible if all terms (source and target) are from the same term bank.
- `EqnListCopyOptExcept`: Copy an Eqnlist with one exception using the optimizations possible if all terms (source and target) are from the same term bank.
- `EqnListCopyDisjoint`: Create a copy of list with disjoint variables (using the even/odd convention).
- `EqnListCopyRepl`: Return a copy of the list with terms from bank, except that all occurrences of "old" are replaced with repl (which has to be in bank).
- `EqnListCopyReplPlain`: Return a copy of the list with terms from bank, except that all occurrences of "old" are replaced with repl (which has to be in bank). Terma are not instantiated.
- `EqnListNegateEqns`: Negate all signs in the list.
- `EqnListRemoveDuplicates`: Remove all but one copy of identical (modulo commutativity) elements from the list. Return number of removed literals.
- `EqnListRemoveResolved`: Remove trivially false equations.
- `EqnListRemoveACResolved`: Remove negative equations implied by the current AC theory.
- `EqnListRemoveSimpleAnswers`: Remove all simple answer literals from the list
- `EqnListFindNegPureVarLit`: Return a pointer to the first negative literal of the form X!=Y (or NULL if no such literal exists).
- `EqnListFindTrue`: Return the first "always true" literal, if any. Return false otherwise.
- `EqnListIsTrivial`: Return true if the list contains two equal literals with opposing signs.
- `EqnLongListIsTrivial`: As EqnListIsTrivial(), but with an algorithm optimised for long lists.
- `EqnListIsACTrivial`: Return true if the list contains a positive AC-trivial equation.
- `EqnListIsGround`: Return true if all equations in list are true, false otherwise.
- `EqnListIsEquational`: Return true if any literal in the list is a true equations.
- `EqnListIsPureEquational`: Return true if all literals in the list are true equations.
- `EqnListOrient`: Orient all the equations in list. Equations already oriented are not reoriented! Return number of swapped equations.
- `EqnListMaximalLiterals`: Determine for each literal wether it is maximal or not. Returns number of maximal literals. Also determines strictly maximal literals. Returns number of maximal literals (although nobody seems to care ;-).
- `EqnListEqnIsMaximal`: Return true if eqn is maximal with respect to list (i.e. if there are no equations that dominate it), false otherwise. As above, details of this may need change if the calculus changes.
- `EqnListEqnIsStrictlyMaximal`: Return true if eqn is strictly maximal with respect to list (i.e. if there are no equations that dominate it), false otherwise. As above, details of this may need change if the calculus changes.
- `EqnListDeleteTermProperties`: Delete the given properties for all term occurences in the eqnlist.
- `EqnListPrint`: Print the list. Separate elements with the given separator (usually "," oder ";"). If negated is true, negate equations before printing (to allow for easy printing of clauses in implicational form).
- `EqnListPrintDeref`: Print an instantiated list of equations (mostly for debugging).
- `EqnListTSTPPrint`: Same as above, but without negation and uses TSTP literal format.
- `EqnListParse`: Parse a list of equations, separated by Tokens of type sep.
- `NormSubstEqnListExcept`: Instantiate all variables in eqnlist (except for terms from except) with fresh variables from vars. Returns the current position in subst.
- `EqnListDepth`: Return the depth of an eqn-list (i.e. the maximal depth of a term).
- `EqnListAddSymbolDistribution`: Count the number of occurences of function symbols in list and add them to dist_array, which has to be a pointer to an array of long that is sufficiently long (and preferably adequatly initialized).
- `EqnListAddTypeDistribution`: Count the number of occurrences of types of function symbols in list and add them to type_array, which has to be a pointer to an array of long that is sufficiently long (and preferably adequatly initialized).
- `EqnListAddSymbolDistribExist`: Count the number of occurences of function symbols in list and add them to dist_array, which has to be a pointer to an array of long that is sufficiently long (and preferably adequatly initialized). Push occuring symbols onto exists (once).
- `EqnListAddSymbolFeatures`: Update features in feature_array with all equation in list.
- `EqnListComputeFunctionRanks`: Compute the occurrence rank for all function symbols in list.
- `EqnListCollectVariables`: Add all variables in list to tree. Return number of distinct variables.
- `EqnListCollectFCodes`: Add all FCodes in list to tree. Return number of distinct FCodes.
- `EqnListAddFunOccs`: For each symbol in literals that is not already marked in f_occur, push it onto res_stack and mark its entry. Return number of symbols found.
- `EqnListSignedTermSetProp`: Set prop in all terms in the literals in list selected via the pos/neg parameters.
- `EqnListSignedTermDelProp`: Delete prop in all terms in the literals in list selected via the pos/neg parameters.
- `EqnListTBTermDelPropCount`: Delete prop in all terms in list, return number of termcells in which prop was set.
- `EqnListCollectSubterms`: Collect all subterms of list onto collector. Assumes that TPOpFlag is set if and only if the term is already in the collection. Returns the number of new terms found.
- `EqnListCollectGroundTerms`: Collect the non-constant ground terms of (positive/negative) equations in list into res.
- `EqnListMapTerms`: Map all terms in the equation list using f.
- `EqnListLambdaNormalize`: Map all terms in the equation list using f.
- `EqnListFindCompLitExcept`: Try to find if there are literal complementary to y in xs (ignoring exc in it) and ys. Follow dereference as d_x and d_y.

### Dependencies

- `"ccl_eqnlist.h"`
- `"cte_typecheck.h"`
- `<ccl_eqn.h>`
- `<clb_objtrees.h>`
- `<cte_lambda.h>`

### Compile-Time Conditions

- `CCL_EQNLIST`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
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

Source files reviewed: `CLAUSES/ccl_eqnlist.h`, `CLAUSES/ccl_eqnlist.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 2241 lines, 64 scanned public declarations, 1 scanned internal function definitions, and 66 structured function-comment blocks.
- Functions for dealing with (singly linked) lists of equations as used in clauses. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `EqnListPrint` has no format state of its own: it prints nothing for an empty list, writes the first literal without a leading separator, then writes the caller's separator before each remaining literal while forwarding `negated` and `fullterms` directly to `EqnPrint`. Rust preserves this exact list assembly over an owned vector.
- `EqnListPrintDeref` uses the same no-leading-separator loop and forwards one `DerefType` to every literal. Rust preserves the separator behavior while keeping dereference expansion explicit in the literal helper.
- `EqnListTSTPPrint` reuses the same first-literal/no-leading-separator loop but always delegates to `EqnTSTPPrint` without a negation argument. Rust keeps the separator behavior and forwards explicit `fullterms` and oriented-output choices to the bank-explicit TSTP literal writer.
- `EqnListParse` first checks for a format-specific literal start and returns an empty list without consuming input if none is present; otherwise it parses the first literal and then consumes the caller-supplied separator before each following literal. Rust preserves that control flow over the currently ported equation/simple-term parser.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
