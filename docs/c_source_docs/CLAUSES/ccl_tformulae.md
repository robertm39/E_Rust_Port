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

### Rust Port Status Notes

- `src/terms/lambda.rs` now stages `LambdaToForall` for a single term-encoded formula, including the C `lambda_eq_to_forall` mapping that applies fresh variables to lambda equality sides, beta-normalizes, converts Boolean equality/disequality to equivalence/XOR, encodes Boolean atoms as predicate equalities, and closes the result with universal or existential quantifiers. The executable's temporary TSTP application/THF Boolean formula bridge now runs that staged conversion after `NamedToDB` for lambda/DB-bearing equality or disequality formulas that are already known to be Boolean, so supported `fof`, `tff`, and `thf` wrappers can lower those formulas to the existing first-order-shaped clause path. Formula-set/archive integration through `TFormulaSetLambdaNormalize` remains tied to full formula-owner plumbing.
- The executable bridge also applies the single-formula lambda normalization when an ordinary FOF/TFF equality has a lambda-valued operand and a bare declared arrow-typed symbol on the opposite side, recovering the declared symbol from the first-order arity-fixed placeholder before encoding the equality.
- `src/clauses/clausefunc.rs` now stages the formula macro surface for `TFormulaHasSubForm1`, `TFormulaHasSubForm2`, `TFormulaIsBinary`, `TFormulaIsUnary`, `TFormulaIsQuantifiedNL`, `TFormulaIsQuantified`, `TFormulaIsLiteral`, `TFormulaIsComplexBool`, `TFormulaEqual`, `TFormulaCopy`, `TFormulaGCMarkCells`, and `TFormulaFindMaxVarCode`, plus the direct `TFormulaVarIsFree` query, the header-declared `TFormulaVarIsFreeCached` compatibility alias, and the `TFormulaCollectFreeVars`, `TFormulaIsClosed`, `TFormulaHasFreeVars`, `TFormulaIsUntyped`, `TFormulaIsPropConst`, and `EncodePredicateAsEqn` behavior needed by closure, skolemization, and predicate-literal lowering helpers: the direct query trusts `v_count` and masks only `$qex`/`$qall` binders; collection treats `$let` as contributing only its body, ignores DB variables, masks quantified/named-lambda binders while traversing their bodies, keeps encoded propositional constants as equality literals, and sorts the resulting dependency stack by term-handle identity to mirror C's pointer-keyed `PTree`.
- `src/clauses/clausefunc.rs` and `src/terms/termbanks.rs` now stage `TFormulaFCodeAlloc`, `TFormulaPropConstantAlloc`, `TFormulaQuantorAlloc`, `TFormulaAddQuantor`, `TFormulaAddQuantors`, `TFormulaClosure`, `TFormulaNegate`, `TFormulaStackToForm`, `TFormulaExpandDistinct`, `TFormulaTPTPParse`, `TFormulaTSTPParse`, `TcfTSTPParse`, `TSTPDistinctParse`, `TFormulaTPTPPrint`, `TFormulaAppEncode`, `PreloadTypes`, `TFormulaLitAlloc`, `TFormulaClauseEncode`, and `TFormulaClauseClosedEncode`, including first-order `$eq`/`$neq` literal encoding, higher-order formula decoding for `$true`-sided literals, Boolean equality-to-equivalence/XOR lowering for clausifiable literals and TSTP formula-level equality, decoded equality fallback, empty-clause false formulas, C stack-pop conjunction/disjunction folding, pairwise `$distinct` disequality expansion, old-TPTP right-recursive formula parsing, TSTP formula parsing over the existing term-bank subset, TCF `|`-clause folding for unquantified bodies and parenthesized universal bodies, TCF recursive one-variable universal binder nesting, raw `$distinct` pseudo-formula allocation, TPTP/TSTP and app-encoded literal/quantifier/connective rendering with left-spine OR flattening, app-encoding preload side effects, left-to-right OR folding without universal closure, and explicit universal closure for closed clause encoding.
- `src/terms/termbanks.rs` now parses staged TSTP quantified binder annotations with the higher-order type grammar and keeps quantified free-variable application heads out of signature lookup, matching C's `TBTermParse` behavior for arrow-typed THF binders such as `![F: person > person]: ...`.
- `src/terms/termbanks.rs` now keeps non-`$o` partial-application equality on the term-literal path, matching LFHOL encodings that use a user sort named `bool`.
- `src/prover/eprover.rs` now routes executable `tcf(...)` formula bodies through the staged `TcfTSTPParse` helper before the supported parser surface lowers them to clauses or stores them for app-encoded formula-owner output, so the supported executable path preserves C's TCF `|`-only clause-body restriction.
- `src/clauses/clausefunc.rs` now stages `TFormulaCollectClause`, including C's top-level disjunction stack traversal, encoded equality/disequality decoding, `$true` true-literal insertion, `$false` dropping, optional fresh-variable normalization, and final `ClauseAlloc` positive-before-negative literal partitioning.

### Change Later

- C `LambdaToForall` begins by calling `VarBankSetVCountsToUsed` and then relies on `TermMap`'s NULL-return optimization to stop descending into subterms that have no equality/disequality. Rust mirrors that with `TermBank::map_term` and explicit fresh-variable count initialization; keep this ordering when the formula-set wrapper is added.
- C `LambdaToForall` is a broad `TermMap` over equality/disequality subterms after the formula-set preprocessing sequence decides to enable it. Rust's executable bridge deliberately applies the staged single-formula helper only inside the supported Boolean TSTP application and THF term-formula fallbacks until the formula owner can run the full `TFormulaSetLambdaNormalize` sequence with source metadata and archive ownership intact.
- `TFormulaLitAlloc` changes behavior through the process-global `problemType`: first-order mode emits ordinary equality terms, while higher-order mode decodes formulas before deciding whether to build negation, equivalence/XOR, or a decoded equality. Rust passes `ProblemType` explicitly at the call site; keep the C branch order visible until global problem-type access is fully removed.
- `TFormulaLitAlloc` treats any literal with right side `$true` as a formula literal before checking `EqnIsClausifiable`; only non-`$true` Boolean-left literals use the equivalence/XOR path. Rust mirrors this ordering because changing it can alter how logical symbols on the left are decoded and printed.
- `TFormulaClauseEncode` returns propositional false for an empty clause and otherwise folds OR from the literal list without adding universal closure. Rust mirrors that separation; a later formula owner should keep clause encoding and closure explicit instead of hiding closure inside a convenience constructor.
- `TFormulaCollectFreeVars` sets `TPIsFreeVar` on every variable in the bank, temporarily deletes that property on bound variables while recursing, and leaves those property side effects visible on free variables. Rust's staged collector uses a local bound-variable stack and does not mutate term properties; a later formula owner should make free-variable collection a pure query unless a caller is proven to depend on the transient C flag state.
- `TFormulaVarIsFree` trusts the cached `v_count` and treats only `$qex`/`$qall` as binders. A binary `SIG_NAMED_LAMBDA_CODE` cell is traversed as an ordinary formula, so its binder variable can be reported free even when the body does not use it, while `TFormulaCollectFreeVars` treats named lambdas as binders. Rust mirrors both paths; a full formula owner should split fast occurrence queries from binder-aware free-variable collection.
- `TFormulaVarIsFreeCached` is declared in the header, but the only implementation in this checkout is a fully commented-out historical body that would have used `tform_compute_freevars` and asserted equality with `TFormulaVarIsFree`. Rust exposes it as a compatibility alias for the direct query rather than inventing a term-bank cache. A full formula owner should decide whether this facade needs real cached free-variable sets or should remain a documented historical alias.
- Several exported formula operations are macros over lower-level term APIs: `TFormulaEqual` is raw pointer identity, `TFormulaCopy` is `TBInsertNoPropsCached(..., DEREF_ALWAYS)`, `TFormulaGCMarkCells` is term-bank GC marking, and `TFormulaFindMaxVarCode` is term traversal. Rust mirrors these wrappers explicitly; a full formula owner should make handle identity, dereferencing copy, and GC root marking explicit instead of hiding them behind formula-shaped names.
- `TFormulaIsComplexBool` checks `TypeIsBool(form)` rather than `TypeIsBool(form->type)`, so the macro tests the term cell's f-code against the Boolean type code after checking that the symbol is logical. Rust preserves that artifact; a full formula owner should replace this with separate syntactic-logical and typed-Boolean predicates once reference behavior proves where the macro result matters.
- `TFormulaAddQuantors` converts the pointer-keyed free-variable `PTree` with `PTreeToPStack`, whose own C documentation calls the order arbitrary, before wrapping quantifiers. Rust's staged closure uses term-handle identity order from the local collector, while direct add-quantors callers provide an explicit slice order; a full formula owner should decide whether user-facing output wants deterministic quantifier ordering or exact allocator-shaped compatibility.
- `TFormulaStackToForm` destructively pops the last pushed formula as the initial result, folds remaining popped formulas on the left, and returns `$true` for an empty stack regardless of the requested connective. This makes empty or singleton `$distinct` expansions true and makes the pair order depend on the pair-push order plus destructive stack fold; keep that construction artifact explicit until formula-set `$distinct` processing has reference output coverage.
- `TFormulaTPTPPrint` performs rendering through the same literal allocation and term printer side effects as C, including left-spine-only disjunction flattening, adjacent-only quantifier coalescing, named-lambda binder printing for binary cells, and optional duplicated variable type suffixes. Rust mirrors those artifacts in the direct helper; a full formula owner should keep compatibility printing separate from any cleaner internal pretty-printer.
- `TFormulaAppEncode` coalesces only adjacent repeated quantifiers of the same f-code, flattens only the left spine of `|`, and shares `PreloadTypes`' side-effectful dependence on term app-encoding to populate type declarations. Rust mirrors those artifacts in the direct helper; a full formula owner should make quantifier grouping, disjunction rendering, and declaration preloading independently testable phases.
- `TFormulaCollectClause` silently ignores any disjunction leaf that is neither an encoded literal nor `$true`/`$false`, drops `$false`, inserts `$true` as a true literal, and then lets `ClauseAlloc` regroup positives before negatives. Rust mirrors those construction artifacts for compatibility; a full formula owner should decide whether unexpected leaves become explicit diagnostics once reference traces cover the C fallthrough.

- The C TSTP/FOF parser encodes formulas as terms and shares scanner, term-bank, and formula-owner state across parsing, simplification, CNF conversion, and proof output. Rust should preserve the observable token flow first, but the final owner API should separate parsing from clausification enough to make unsupported fragments and source metadata explicit.
- Old TPTP `input_formula(...)` uses `TFormulaTPTPParse`, which treats every FOF binary operator as the same-precedence right-recursive operator and lets a quantifier bind only the next elementary formula unless the body is parenthesized. Rust preserves that dialect quirk, including supported FOOL-primary existential bodies through the elementary-formula path, for compatibility, but the final parser API should make TPTP-vs-TSTP precedence and quantifier-scope rules explicit rather than hiding them behind shared helper names.
- Old TPTP `TFormulaTPTPParse` routes atoms through `EqnParseInfix` and does not run the TSTP Boolean formula equality-to-equivalence rewrite at the binary-operator layer. Rust mirrors this in the direct parser and now uses that represented parser for executable `--app-encode` `input_formula(...)` entries, so formula-level `=`/`!=` remain `$eq`/`$neq` formula operators and print as `=`/`!=` in that dialect; a future formula owner should keep source dialect and normalized Boolean semantics as separate facts.
- `TFormulaTSTPParse` handles associative `&`/`|` chains with a loop, but non-associative binary operators parse only one `literal_tform_tstp_parse` operand on the right; unparenthesized chained implications/equivalences or unparenthesized connective operands therefore fall out as trailing tokens. Rust now rejects repeated unparenthesized non-associative chains in the executable bridge while still leaving the broader temporary connective-operand bridge visible for later full formula-owner work. A cleaned parser should make that precedence boundary explicit, either as a compatibility grammar mode or as a deliberate modern TPTP grammar choice after reference tests decide which spellings must remain accepted.
- After `~`, `literal_tform_tstp_parse` recurses into the literal parser rather than the outer binary parser. As a result, `~p(a) = (q(a)&r(a))` is parsed as a negated literal/equality attempt and can type-error, while `(~p(a)) = (q(a)&r(a))` reaches the formula-level equality-to-equivalence rewrite. Rust preserves this token-flow quirk; a cleaned parser should make negation precedence explicit behind compatibility tests.
- `quantified_tform_tstp_parse` gives an unparenthesized quantifier only the next literal as its body, uses recursive one-variable nesting for comma lists, and relies on variable-bank push/pop side effects around parsing. Rust mirrors that behavior for Boolean term arguments and the supported FOOL-primary existential-body bridge; a full formula owner should make binder scope and variable lifetime explicit instead of coupling them to parser-global external-name state.
- `quantified_tform_tstp_parse` reads each binder with `TBTermParse`, so typed binder annotations such as `F: person > person` depend on the active `problemType` to make the full higher-order type grammar available, and the resulting free variable can later be used as an application head. Rust now mirrors this in the staged TSTP formula parser by parsing binder annotations with the higher-order type grammar and avoiding signature lookup for free-variable heads; a full formula/parser owner should carry wrapper/problem-type context explicitly rather than relying on process-global parser state.
- `TcfTSTPParse` accepts only a leading universal quantifier, then reuses `quantified_tform_tstp_parse` with the `tcf` flag so comma-separated variables become recursive one-variable quantifier nodes, a parenthesized body is parsed as a `|`-only clause, and an unparenthesized body is parsed as one atom. Rust mirrors this direct helper behavior; a full typed-clause parser should make the distinction between clause bodies and general formulas explicit instead of hiding it in a Boolean flag.
- `WFormulaTSTPParse` special-cases top-level `$distinct(...)`, routes `tcf(...)` bodies through `TcfTSTPParse`, and routes the remaining FOF/TFF/THF wrappers through `TFormulaTSTPParse`. Rust now uses the represented formula-owner route for direct top-level `$distinct(...)` executable `--app-encode` bodies, expanding them through proof-state `$distinct` processing before `FormulaSetAppEncode`, for ordinary FOF app-encode bodies that do not need bridge-only negated/nested-helper formula-operand equality, embedded non-Boolean FOOL term equality, or lambda handling, including ordinary TSTP application syntax, declared-Boolean predicate equality, top-level and connective-operand untyped atomic-left formula equality/disequality, direct formula-level `$ite`/`$let` FOOL bodies, parenthesized Boolean FOOL equality/disequality, and top-level non-Boolean `$ite`/`$let` term equality on either side, and for non-`$distinct` executable `--app-encode` TFF/TCF/THF formulas with the same `tcf` parser split. FOF app-encode negated/nested-helper formula-operand equality, embedded non-Boolean FOOL term equality, and lambda shim entries plus negated or wrapped `$distinct` app-encode bodies still use the bridge expansion path. A full formula owner should audit whether the remaining FOF bridge shims can move to `TFormulaTSTPParse` without losing compatibility behavior, and whether non-direct raw `$distinct` spellings should render through `TFormulaAppEncode` like C or remain a compatibility expansion.
- `tptp_quantor_parse` maps `^` to `SIG_NAMED_LAMBDA_CODE`, and `TFormulaFCodeAlloc` deliberately skips assigning `$o` to that operator so the term bank can infer an arrow type from binder to body. Rust mirrors this in the temporary Boolean-argument bridge and in the executable bridge for beta-reducible parenthesized lambda applications; a full formula/term owner should make the boundary between formulas and higher-order term-valued lambdas explicit.
- `applied_tform_tstp_parse` sizes a temporary C argument array from the original head type, parses each `@` operand through the literal-formula parser before returning to the outer application loop, re-applies predicate-as-equality encoding for logical heads, and then relies on `normalize_head` to choose ordinary head extension versus `SIG_PHONY_APP_CODE`. `literal_tform_tstp_parse` can also manufacture a zero-arity logical head such as `(&)` for immediate application, relying on the signature-declared logical-operator type rather than the temporary head arity. That operand parsing keeps bare applied arguments left-associative, carries the residual arrow type when a partially applied head is applied again, keeps parenthesized operands bounded by the closing parenthesis, and requires explicit parentheses when the argument itself is an application. Rust mirrors the normalized shape for Boolean term arguments with explicit type checks, and the temporary executable FOF/TFF bridge now detects typed `p @ a`, `(p) @ a`, `(h @ a) @ b`, transparent parenthesized wrappers around such applications, right-hand equality terms such as `c = (h @ a) @ b`, `c = (((h @ a) @ b))`, and `c = ((g)) @ a`, beta-reducible lambda equality terms such as `((^[X: $i]: f @ X) @ a) = b`, and `(<logical-op>) @ ...` formula starts before detouring through the staged term-bank parser. A full formula owner should make application associativity, argument expectations, predicate/logical-head/lambda lowering, and typed application parsing explicit instead of coupling them to parser side effects and executable-level lookahead.
- C's application parser uses the same literal-formula operand parser for Boolean operands and for term-valued operands whose expected type is an arrow, so a THF atom such as `p @ (^[X: person]: X)` is valid when `p` expects a function argument, and a lambda head can still be the prefix of an app-encoded application such as `(^[X: person]: p @ X) @ a`. Rust now routes proof-search/CNF/prune, non-printing syntax-only, print-formulas, and app-encode formula owners for those shapes through represented `WFormula` storage and the formula-set lambda-aware owner path; a cleaned parser should model Boolean formula arguments, term-valued lambda arguments, and lambda-headed application prefixes as distinct typed operands instead of relying on the shared literal-parser detour.
- First-order arity fixing can temporarily split a bare typed function symbol from the declared arrow symbol until the application path normalizes the head. Rust now has a narrow bridge recovery for lambda equality operands paired with a bare arrow symbol, such as `f = (^[X: $i]: g @ X)` or `(^[X: $i]: g @ X) = f`; the full formula owner should remove that executable shim by making typed application and equality parsing use one source of symbol identity.
- `literal_tform_tstp_parse` accepts an optional `Application` token immediately after `~` before parsing the negated literal, so C accepts first-order-looking syntax such as `fof(a, axiom, ~ @ p(a)).` through the same branch used by higher-order application syntax. Rust mirrors this in the temporary executable FOF/TFF bridge and in the term-bank TSTP parser; a cleaned parser should decide whether this compatibility spelling remains accepted only in a legacy mode.
- Formula-level `$ite` enters through `TBTermParseReal`/`ParseIte`, parses all three arms with `TFormulaTSTPParse`, asserts the condition is Boolean and the branches share one sort, then lets `literal_tform_tstp_parse` encode the Boolean `$ite` as a predicate literal. Rust mirrors this for Boolean term arguments, supported top-level Boolean formula atoms in the remaining temporary bridge, and the represented first-order proof-search/CNF/prune formula-owner route through `TFormulaSetLiftItes`; the full parser should expose `$ite` as a typed formula/term node before literal encoding rather than relying on this term-parser detour.
- C's `$let` formula path keeps definition-head variables as local formals through term-encoded formula parsing and later let lifting, while captured variables become dependencies of the generated definitions. Because `TFormulaSetLiftLets` runs after conjecture negation and inserts fresh definitions as separate wrappers, generated definitions are not negated with the conjecture body and start from default/plain wrapper metadata rather than inheriting input source annotations. Rust now mirrors this for supported direct parameterized first-order `$let` axioms and conjectures through the represented proof-search/CNF/prune formula-owner route, while the temporary executable bridge remains for actual THF/helper paths; a full `WFormula`/`FormulaSet` owner should represent let-local formals, captured variables, generated-definition derivation/archive metadata, and conjecture-side lifting explicitly instead of relying on bridge-local term scans.
- `TFormulaTSTPParse` parses formula-level `=` and `!=` as ordinary equality operators first, then rewrites them to equivalence and XOR when both operands have Boolean type while non-Boolean operands remain term equality. Because `literal_tform_tstp_parse` returns ordinary predicate atoms through `EncodePredicateAsEqn` before the outer operator parse, forms like `p(a) = (q(a)|r(a))` and `p(a) != ![X]:q(X)` are formula equivalence/XOR, not term equality attempts. `EqnParseInfix` has a related Boolean-left shortcut for typed predicate atoms: after consuming `=`, it parses the right side with `TFormulaTSTPParse`, so `p(a) = q(a)`, `p(a) = $ite(...)`, and `p(a) != $let(...)` behave as Boolean formula equality when `p(a)` is already typed as `$o`. The C checks are type-based, not name-based, so an LFHOL user sort named `bool` is still an ordinary term sort; partial applications returning that sort, such as `col @ (lam @ A)`, must remain term-equality operands. Rust mirrors that behavior for supported compound formula operands, represented typed atomic-left Boolean equality/disequality, represented top-level and connective-operand atomic-left formula-only right operands, supported Boolean `$ite`/`$let` primary operands, non-Boolean `$ite`/`$let` equality operands, non-Boolean `@` application equality RHS terms such as `c = (h @ a) @ b`, and non-`$o` partial-application equality such as `(col @ (lam @ A)) = A`; a full formula owner should keep the source spelling, semantic normalized connective, real `$o` versus user-sort distinction, and term-literal fallback as separate, explicit facts.
- `parse_atom` returns the left term unchanged when `EqnParseInfix` finds no right side, which lets term-valued atoms flow through formula parsing for constructs such as non-Boolean `$ite` and `$let`. Rust mirrors the fixed non-predicate atom and compound cases plus supported top-level and existential-body Boolean `$ite`/`$let` atoms and non-Boolean `$ite`/`$let` equality literals in the temporary bridge; a full formula owner should expose this term/formula distinction explicitly.
- `TSTPDistinctParse` parses each `$distinct(...)` argument with `FuncSymbParse` rather than the variable-aware term parser, so uppercase or underscore-starting identifiers become zero-arity constants in this context. Rust mirrors that direct parser entry point, and the executable formula bridge now reuses it before expanding supported `$distinct(...)` formulas to disequalities. A cleaned formula parser should make the constant-only rule explicit and reserve variable-looking names only behind a compatibility mode.
- `literal_tform_tstp_parse` always returns through `EncodePredicateAsEqn`, so ordinary Boolean predicates become `$eq(p(...),$true)` and `$false` becomes `$neq($true,$true)` before later simplification can normalize them. Rust mirrors this shape for Boolean term arguments in the temporary term-bank bridge; a full formula owner should make predicate-as-literal encoding an explicit lowering step instead of hiding it in the parser.
- `TFormulaTPTPPrint` allocates temporary `Eqn` cells for literal formulas before printing, so printing can still update predicate-symbol state through `EqnAlloc`. Rust mirrors that side effect in the direct term-formula helper; a full formula owner should decide whether rendering remains allowed to mutate signature metadata or whether compatibility callers get an explicit pre-normalization phase.
- `TFormulaTPTPPrint` flattens only the left spine of disjunctions through `tformula_print_or_chain`, coalesces only adjacent repeated quantifiers with the same f-code, and treats `SIG_NAMED_LAMBDA_CODE` as a `^[...]` binder when the cell has binary quantifier shape. Rust mirrors these output artifacts; a full formula owner should make disjunction rendering, quantifier grouping, and lambda-valued terms explicit before considering cleaner formatting.
- `TFormulaTPTPPrint` prints quantified variable terms first and then appends a required quantifier type annotation, so the process-global `TermPrintTypes` flag can duplicate type text on non-`$i` first-order variables. Rust mirrors the shape through explicit print options; non-compatibility renderers should avoid carrying that global-formatting artifact forward.
- `TFormulaAppEncode` prints the formula-level operator represented in the term encoding, so reverse implication, XOR, NAND, and NOR remain visible as `<=`, `<~>`, `~&`, and `~|` in the app-encoded output even if later proof lowering can normalize them semantically. Rust now keeps these variants in the reusable term-formula helper and routes supported executable `--app-encode` formulas through proof-state `f_axioms` and the `FormulaSet` owner; a full parser owner should keep this source-spelling-preserving render path explicit rather than depending on normalized clause forms.
- C's `TFormulaAppEncode` flattens only the left spine of disjunctions through `tformula_appencode_or_chain`; right-nested disjunctions still print as nested parenthesized formulas. Rust mirrors this left-spine-only rendering artifact; a full formula owner should decide whether byte-compatible app-encoded output or structurally normalized disjunction printing is the long-term API.
- C's `TFormulaIsQuantified` includes `SIG_NAMED_LAMBDA_CODE`, but `TFormulaAppEncode` then assumes a free-variable first argument and prints every non-existential quantified code with `!`. Rust mirrors the direct helper behavior for term-encoded formulas; a typed formula owner should separate quantifiers from lambda-valued terms before app-encoded rendering.
- C's `PreloadTypes` app-encodes literal sides solely for side effects, discards the temporary terms, and thereby mutates the type bank and signature before type/symbol declaration printing. Rust mirrors those side effects through the term-formula preload helper, including FOOL-aware traversal before declaration output; keep declaration preloading as an explicit phase instead of depending on a discarded rendering traversal at each executable call site.
- C's `TFormulaAppEncode` and `PreloadTypes` recurse over formula literals, quantifiers, unary nodes, and binary nodes, while `$ite` and `$let` arrive through the term parser and literal encoding path. Rust now ports the direct term-formula app-encode/preload helpers with recursive Boolean and term-valued `$ite`/`$let` rendering, including `$let` definitions and bodies, keeps Boolean equality/disequality output on the equivalence/XOR formula path, preserves non-Boolean `$ite`/`$let` equality as term-valued app-encoded equality, and routes supported executable `--app-encode` formulas through proof-state `f_axioms` and `FormulaSetAppEncode`; the remaining parser bridge should be replaced by a full `WFormula` parser owner rather than by reconstructing formula traversal in executable code.
- `LiftLambdas` uses `unbind_loose`, `find_generalization`, and a `PDTree` of previous liftings to reuse more general generated lambda definitions while temporarily rebinding fresh variables to loose DB variables, and C's `lift_lambda` gives each generated definition formula its own `DCIntroDef` before the set-level wrapper records source-formula provenance. Rust now stages formula-set lambda lifting, post-CNF clause lambda lifting, generated-definition owner side effects including `DCIntroDef` metadata, and exact plus generalized closed-body reuse; the exact `PDTree` index structure remains a compatibility/performance item for a later term-level lifting pass.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
