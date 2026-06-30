<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_tformulae

## Source Files

- [CLAUSES/ccl_tformulae.h](../../../eprover/CLAUSES/ccl_tformulae.h)
- [CLAUSES/ccl_tformulae.c](../../../eprover/CLAUSES/ccl_tformulae.c)

## Purpose

Declarations and definitions for full first-order formulae encoded as terms. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TFormula_p`

### Macros And Constants

- `CCL_TFORMULAE`
- `TFORM_MINISCOPE_LIMIT_STR`
- `TFORM_RENAME_LIMIT`
- `TFORM_RENAME_LIMIT_STR`
- `TFormulaCopy(bank, form)`
- `TFormulaEqual(f1,f2)`
- `TFormulaFindMaxVarCode(form)`
- `TFormulaGCMarkCells(bank, form)`
- `TFormulaHasSubForm1(sig, form)`
- `TFormulaHasSubForm2(sig, form)`
- `TFormulaIsBinary(form)`
- `TFormulaIsComplexBool(sig, form)`
- `TFormulaIsLiteral(sig,form)`
- `TFormulaIsPropFalse(sig, form)`
- `TFormulaIsPropTrue(sig, form)`
- `TFormulaIsQuantified(sig,form)`
- `TFormulaIsQuantifiedNL(sig,form)`
- `TFormulaIsUnary(form)`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p TFormulaCollectClause(TFormula_p form, TB_p terms, VarBank_p fresh_vars)`
- `TFormula_p LambdaToForall(TB_p terms, TFormula_p t)`
- `TFormula_p LiftLambdas(TB_p terms, TFormula_p t, PStack_p definitions, PDTree_p liftings)`
- `TFormula_p TFormulaAddQuantor(TB_p bank, TFormula_p form, bool universal, Term_p var)`
- `TFormula_p TFormulaAddQuantors(TB_p bank, TFormula_p form, bool universal, PTree_p vars)`
- `TFormula_p TFormulaClauseClosedEncode(TB_p bank, Clause_p clause)`
- `TFormula_p TFormulaClauseEncode(TB_p bank, Clause_p clause)`
- `TFormula_p TFormulaClosure(TB_p bank, TFormula_p form, bool universal)`
- `TFormula_p TFormulaCreateDef(TB_p bank, TFormula_p def_atom, TFormula_p defined, int polarity)`
- `TFormula_p TFormulaExpandDistinct(TB_p bank, TFormula_p distinct)`
- `TFormula_p TFormulaFCodeAlloc(TB_p bank, FunCode op, TFormula_p arg1, TFormula_p arg2)`
- `TFormula_p TFormulaHasFreeVars(TB_p bank, TFormula_p form)`
- `TFormula_p TFormulaLitAlloc(Eqn_p literal)`
- `TFormula_p TFormulaNegate(TFormula_p form, TB_p terms)`
- `TFormula_p TFormulaPropConstantAlloc(TB_p bank, bool positive)`
- `TFormula_p TFormulaQuantorAlloc(TB_p bank, FunCode quantor, Term_p var, TFormula_p arg)`
- `TFormula_p TFormulaStackToForm(TB_p bank, PStack_p stack, FunCode op)`
- `TFormula_p TFormulaTPTPParse(Scanner_p in, TB_p terms)`
- `TFormula_p TFormulaTSTPParse(Scanner_p in, TB_p terms)`
- `TFormula_p TSTPDistinctParse(Scanner_p in, TB_p terms)`
- `TFormula_p TcfTSTPParse(Scanner_p in, TB_p terms)`
- `Term_p EncodePredicateAsEqn(TB_p bank, TFormula_p f)`
- `bool TFormulaIsClosed(TB_p bank, TFormula_p form)`
- `bool TFormulaIsUntyped(TFormula_p form)`
- `bool TFormulaVarIsFree(TB_p bank, TFormula_p form, Term_p var)`
- `bool TFormulaVarIsFreeCached(TB_p bank, TFormula_p form, Term_p var)`
- `int TFormulaDecodePolarity(TB_p bank, TFormula_p form)`
- `void PreloadTypes(TB_p bank, TFormula_p form)`
- `void TFormulaAppEncode(FILE* out, TB_p bank, TFormula_p form)`
- `void TFormulaCollectFreeVars(TB_p bank, TFormula_p form, PTree_p *vars)`
- `void TFormulaMarkPolarity(TB_p bank, TFormula_p form, int polarity)`
- `void TFormulaTPTPPrint(FILE* out, TB_p bank, TFormula_p form, bool fullterms, bool pcl)`

## Implementation Notes

### Internal Functions

- `applied_tform_tstp_parse`
- `assoc_tform_tstp_parse`
- `clause_tform_tstp_parse`
- `elem_tform_tptp_parse`
- `lambda_eq_to_forall`
- `literal_tform_tstp_parse`
- `make_head`
- `normalize_head`
- `parse_atom`
- `parse_ho_atom`
- `quantified_tform_tptp_parse`
- `quantified_tform_tstp_parse`
- `tformula_collect_freevars`
- `tptp_operator_convert`
- `tptp_operator_parse`
- `tptp_quantor_parse`

### Source-Level Behavior

- `make_head`: Makes term that has function code that corresponds to f_name and no arguments. NB: Term is unshared at this point!
- `parse_ho_atom`: Parses one HO symbol.
- `normalize_head`: Makes sure that term is represented in a flattened representation.
- `tptp_operator_convert`: Return the f_code corresponding to a given token. Rather trivial ;-)
- `tptp_operator_parse`: Parse a TPTP operator and return the corresponding f_code. Rather trivial ;-)
- `tptp_quantor_parse`: Parse and return a TPTP quantor. Rather trivial ;-)
- `quantified_tform_tptp_parse`: Parse a quantified TPTP/TSTP formula. At this point, the quantor has already been read (and is passed into the function), and we are at the first (or current) variable.
- `parse_atom`: Parse an elementary formula in TPTP/TSTP format. New: takes care of complicated forms such as $let and $ite
- `elem_tform_tptp_parse`: Parse an elementary formula in TPTP/TSTP format.
- `clause_tform_tstp_parse`: Parse a sequence of literals connected by a | operator and return it.
- `quantified_tform_tstp_parse`: Parse a quantified TSTP formula. At this point, the quantor has already been read (and is passed into the function), and we are at the first (or current) variable.
- `assoc_tform_tstp_parse`: Parse a sequence of formulas connected by a single AC operator and return it.
- `applied_tform_tstp_parse`: Parse a sequence of formulas connected by application operator and normalize the term according to the invariant maintained by @: If the head is a single constant F then simply apply F to arguments. Otherwise, apply the head using SIG_PHONY_APP_CODE
- `literal_tform_tstp_parse`: Parse an elementary formula in TSTP format. Parses: (1) quantified formulas (includes lambda in HO) (2) '(' full formula ')' (3) ~ full formula FO: (4) equation / predicate term HO: (4) variable or constant
- `tform_compute_freevars`: Return the set of free variables in form. If necessary, compute it and update bank->freevars.
- `tformula_collect_freevars`: Collect the _free_ variables in form in *vars. This is somewhat tricky. We require that initially all variables have TPIsFreeVar set.
- `lambda_eq_to_forall`: If the term is an equation between terms where at least one is a lambda, then turn it into equation of non-lambdas
- `find_generalization`: Check if there is already a name for lambda term query. If so, return the defining formula and store the name in *name. Assumes that in query fresh variables that represent loosely bound vars are bound to their corresponding DB vars.
- `store_lifting`: Check if there is already a name for lambda term query. If so, return the defining formula and store the name in *name.
- `lift_lambda`: Convert lambda term: ^[...bound vars...]:s[...free vars...] into a definiton f ..free vars.. ..bound vars.. = s
- `EncodePredicateAsEqn`: If a term is of the from p(s) where p is an uninterpreted predicate symbol it will be converted to equation p(s) = T, to maintain E's interal invariants
- `TFormulaIsPropConst`: Return true iff the formula is the encoding of one of the propositional constants i.e. $eqn($true,$true)$ (if posive is true) or $neqn($true, $true).
- `TFormulaFCodeAlloc`: Allocate a formula given an f_code and two subformulas (the second one may be NULL).
- `TFormulaLitAlloc`: Allocate a literal term formula. The equation is _not_ freed!
- `TFormulaPropConstantAlloc`: Allocate a formula representing a propositional constant (true or false).
- `TFormulaQuantorAlloc`: Allocate a formula with a quantor.
- `tformula_print_or_chain`: Print a formula of |-connectect subformula as a flat list without parentheses.
- `tformula_appencode_or_chain`: Prints app-encoded version of the formula form to out. Original formula is not chagned.
- `TFormulaTPTPPrint`: Print a formula in TPTP/TSTP format.
- `TFormulaAppEncode`: Appencodes TFormula and prints result to out.
- `PreloadTypes`: Make sure that all intermediate types needed for app-encoding of the formula are already inserted in the type bank. For example if type a > b > c > d appears in the type bank insert types b > c > d and c > d to the type bank.
- `TFormulaTPTPParse`: Parse a formula in TPTP format.
- `TFormulaTSTPParse`: Parse a formula in TSTP formuat.
- `TcfTSTPParse`: Parse a TCF formula (potentially typed clause) in TSTP format.
- `parse_constant_term`: Parse a constant term (only constants allowed).
- `TSTPDistinctParse`: Parse a $distinct()-pseudo-term.
- `TFormulaVarIsFree`: Return true iff var is a free variable in form.
- `TFormulaVarIsFreeCached`: Return true iff var is a free variable in form. Also cache the local variable set in bank->freevarset. Not really an improvement in the original use case, kept as a historical recode...
- `TFormulaCollectFreeVars`: Collect the _free_ variables in form in *vars.
- `TFormulaIsClosed`: Returns true if forula has no free vars.
- `TFormulaHasFreeVars`: Check if the formula has at least one free variable. If so, return one of them, otherweise NULL.
- `TFormulaAddQuantor`: Given F and X, create !X.F or ?X.F. Requires F and X to be in the term bank!
- `TFormulaAddQuantors`: Given F and X1...Xn, create Q[X1...Xn]:F, where Q is ? or ! as requested.
- `TFormulaClosure`: Create the existential or universal closure of form.
- `TFormulaCreateDef`: Given an fresh, suitable atom, a formula, and the polarity, return the correct defining formula.
- `TFormulaClauseEncode`: Given a clause, return a TFormula representing it. Quantors are not added for the universal closure!
- `TFormulaMarkPolarity`: For all subformulas of form, mark if they occur with positive and/or negative polarity. Assumes that the properties are properly reset!
- `TFormulaDecodePolarity`: Return the polarity indicated by the polarity properties.
- `TFormulaClauseClosedEncode`: Generate a tform-representation of clause with explicit universal quantification.
- `TFormulaCollectClause`: Given a term-encoded formula that is a disjunction of literals, transform it into a clause. If the optional parameter fresh_vars is given, variables in the result will be normalized.
- `TFormulaStackToForm`: Given a stack of formulas, combine them into a formula conjoined by the given op. I'm to tired to think about structure, so better use only conjunction and disjunction here ;-)
- `TFormulaExpandDistinct`: Create a conjunction of disequations expressing a $distinct statment: $distinct(a,b,c) => neqn(a,b)&neqn(b,c)&neqn(a,c).
- `LiftLambdas`: Turns equation (^[X]:s)=t into ![X]:(s = (t @ X))
- `Lambda2Forall`: Turns equation (^[X]:s)=t into ![X]:(s = (t @ X))

### Dependencies

- `"ccl_derivation.h"`
- `"ccl_inferencedoc.h"`
- `"ccl_tformulae.h"`
- `<ccl_clauses.h>`
- `<ccl_formula_wrapper.h>`
- `<ccl_pdtrees.h>`
- `<cte_lambda.h>`
- `<cte_typecheck.h>`

### Compile-Time Conditions

- `CCL_TFORMULAE`
- `previously`

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

Source files reviewed: `CLAUSES/ccl_tformulae.h`, `CLAUSES/ccl_tformulae.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 2975 lines, 35 scanned public declarations, 16 scanned internal function definitions, and 55 structured function-comment blocks.
- Declarations and definitions for full first-order formulae encoded as terms. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- Change-later candidate: the C TSTP/FOF parser encodes formulas as terms and shares scanner, term-bank, and formula-owner state across parsing, simplification, CNF conversion, and proof output. Rust should preserve the observable token flow first, but the final owner API should separate parsing from clausification enough to make unsupported fragments and source metadata explicit.
- Change-later candidate: old TPTP `input_formula(...)` uses `TFormulaTPTPParse`, which treats every FOF binary operator as the same-precedence right-recursive operator and lets a quantifier bind only the next elementary formula unless the body is parenthesized. Rust preserves that dialect quirk for compatibility, but the final parser API should make TPTP-vs-TSTP precedence and quantifier-scope rules explicit rather than hiding them behind shared helper names.
- Change-later candidate: `quantified_tform_tstp_parse` gives an unparenthesized quantifier only the next literal as its body, uses recursive one-variable nesting for comma lists, and relies on variable-bank push/pop side effects around parsing. Rust mirrors that behavior for Boolean term arguments; a full formula owner should make binder scope and variable lifetime explicit instead of coupling them to parser-global external-name state.
- Change-later candidate: `tptp_quantor_parse` maps `^` to `SIG_NAMED_LAMBDA_CODE`, and `TFormulaFCodeAlloc` deliberately skips assigning `$o` to that operator so the term bank can infer an arrow type from binder to body. Rust mirrors this in the temporary Boolean-argument bridge; a full formula/term owner should make the boundary between formulas and higher-order term-valued lambdas explicit.
- Change-later candidate: `applied_tform_tstp_parse` sizes a temporary C argument array from the original head type, parses each `@` operand through the literal-formula parser, re-applies predicate-as-equality encoding for logical heads, and then relies on `normalize_head` to choose ordinary head extension versus `SIG_PHONY_APP_CODE`. Rust mirrors the normalized shape for Boolean term arguments with explicit type checks; a full formula owner should make application argument expectations and logical-head lowering explicit instead of coupling them to parser side effects.
- Change-later candidate: formula-level `$ite` enters through `TBTermParseReal`/`ParseIte`, parses all three arms with `TFormulaTSTPParse`, asserts the condition is Boolean and the branches share one sort, then lets `literal_tform_tstp_parse` encode the Boolean `$ite` as a predicate literal. Rust mirrors this for Boolean term arguments and supported top-level Boolean formula atoms in the temporary bridge; the full parser should expose `$ite` as a typed formula/term node before literal encoding rather than relying on this term-parser detour.
- Change-later candidate: `TFormulaTSTPParse` parses formula-level `=` and `!=` as ordinary equality operators first, then rewrites them to equivalence and XOR when both operands have Boolean type. Rust mirrors that behavior for supported compound formula operands and for supported Boolean `$ite`/`$let` primary operands in the temporary bridge; a full formula owner should keep the source spelling and the semantic normalized connective as separate, explicit facts.
- Change-later candidate: `parse_atom` returns the left term unchanged when `EqnParseInfix` finds no right side, which lets term-valued atoms flow through formula parsing for constructs such as non-Boolean `$ite` and `$let`. Rust mirrors the fixed non-predicate atom and compound cases plus supported top-level Boolean `$let` atoms in the temporary bridge; a full formula owner should expose this term/formula distinction explicitly.
- Change-later candidate: `literal_tform_tstp_parse` always returns through `EncodePredicateAsEqn`, so ordinary Boolean predicates become `$eq(p(...),$true)` and `$false` becomes `$neq($true,$true)` before later simplification can normalize them. Rust mirrors this shape for Boolean term arguments in the temporary term-bank bridge; a full formula owner should make predicate-as-literal encoding an explicit lowering step instead of hiding it in the parser.
- Change-later candidate: `TFormulaAppEncode` prints the formula-level operator represented in the term encoding, so reverse implication, XOR, NAND, and NOR remain visible as `<=`, `<~>`, `~&`, and `~|` in the app-encoded output even if later proof lowering can normalize them semantically. Rust now keeps these variants through the supported bridge for compatibility; a full formula owner should make this source-spelling-preserving render path explicit rather than depending on normalized clause forms.
- Change-later candidate: C's `TFormulaAppEncode` and `PreloadTypes` recurse over formula literals, quantifiers, unary nodes, and binary nodes, while `$ite` and `$let` arrive through the term parser and literal encoding path. Rust's temporary app-encode bridge now recognizes supported top-level Boolean `$ite`/`$let` atoms, recursively renders their encoded Boolean subformulas, including `$let` definitions and bodies, and keeps their Boolean equality/disequality output on the equivalence/XOR formula path, but the final `WFormula`/`FormulaSet` port should own this traversal instead of reconstructing it from term-encoded bridge nodes.

### Rust Port Status Notes

- `src/terms/lambda.rs` now stages `LambdaToForall` for a single term-encoded formula, including the C `lambda_eq_to_forall` mapping that applies fresh variables to lambda equality sides, beta-normalizes, converts Boolean equality/disequality to equivalence/XOR, encodes Boolean atoms as predicate equalities, and closes the result with universal or existential quantifiers. Formula-set/archive integration through `TFormulaSetLambdaNormalize` remains tied to full formula-owner plumbing.
- `src/clauses/clausefunc.rs` now stages the `TFormulaCollectFreeVars` behavior needed by `TFormulaSkolemizeOutermost`: `$let` contributes only its body, DB variables are ignored, quantified/named-lambda binders mask their variable while the body is traversed, and the resulting dependency stack is sorted by term-handle identity to mirror C's pointer-keyed `PTree`.
- `src/clauses/clausefunc.rs` now stages `TFormulaLitAlloc` and `TFormulaClauseEncode`, including first-order `$eq`/`$neq` literal encoding, higher-order formula decoding for `$true`-sided literals, Boolean equality-to-equivalence/XOR lowering for clausifiable literals, decoded equality fallback, empty-clause false formulas, and left-to-right OR folding without universal closure.
- `src/clauses/clausefunc.rs` now stages `TFormulaCollectClause`, including C's top-level disjunction stack traversal, encoded equality/disequality decoding, `$true` true-literal insertion, `$false` dropping, optional fresh-variable normalization, and final `ClauseAlloc` positive-before-negative literal partitioning.

### Change-Later Observations

- C `LambdaToForall` begins by calling `VarBankSetVCountsToUsed` and then relies on `TermMap`'s NULL-return optimization to stop descending into subterms that have no equality/disequality. Rust mirrors that with `TermBank::map_term` and explicit fresh-variable count initialization; keep this ordering when the formula-set wrapper is added.
- `TFormulaLitAlloc` changes behavior through the process-global `problemType`: first-order mode emits ordinary equality terms, while higher-order mode decodes formulas before deciding whether to build negation, equivalence/XOR, or a decoded equality. Rust passes `ProblemType` explicitly at the call site; keep the C branch order visible until global problem-type access is fully removed.
- `TFormulaLitAlloc` treats any literal with right side `$true` as a formula literal before checking `EqnIsClausifiable`; only non-`$true` Boolean-left literals use the equivalence/XOR path. Rust mirrors this ordering because changing it can alter how logical symbols on the left are decoded and printed.
- `TFormulaClauseEncode` returns propositional false for an empty clause and otherwise folds OR from the literal list without adding universal closure. Rust mirrors that separation; a later formula owner should keep clause encoding and closure explicit instead of hiding closure inside a convenience constructor.
- `TFormulaCollectFreeVars` sets `TPIsFreeVar` on every variable in the bank, temporarily deletes that property on bound variables while recursing, and leaves those property side effects visible on free variables. Rust's staged collector uses a local bound-variable stack and does not mutate term properties; a later formula owner should make free-variable collection a pure query unless a caller is proven to depend on the transient C flag state.
- `TFormulaCollectClause` silently ignores any disjunction leaf that is neither an encoded literal nor `$true`/`$false`, drops `$false`, inserts `$true` as a true literal, and then lets `ClauseAlloc` regroup positives before negatives. Rust mirrors those construction artifacts for compatibility; a full formula owner should decide whether unexpected leaves become explicit diagnostics once reference traces cover the C fallthrough.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
