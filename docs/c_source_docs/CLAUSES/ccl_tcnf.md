<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_tcnf

## Source Files

- [CLAUSES/ccl_tcnf.h](../../../eprover/CLAUSES/ccl_tcnf.h)
- [CLAUSES/ccl_tcnf.c](../../../eprover/CLAUSES/ccl_tcnf.c)

## Purpose

Functions implementing the CNF conversion of first order formulae encoded as terms. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_TCNF`
- `RETURN_IF_LARGE(param)`
- `TFORM_MANY_CLAUSES`
- `TFORM_MANY_LIMIT`

### Globals

- None found in the source scan.

### Exported Functions

- `TFormula_p TFormulaCopyDef(TB_p bank, TFormula_p form, long blocked, NumXTree_p *defs, PStack_p defs_used)`
- `TFormula_p TFormulaDefRename(TB_p bank, TFormula_p form, int polarity, NumXTree_p *defs, PStack_p renamed_forms)`
- `TFormula_p TFormulaDistributeDisjunctions(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaExpandLiterals(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaMiniScope(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaNNF(TB_p terms, TFormula_p form, int polarity)`
- `TFormula_p TFormulaNegAlloc(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaSimplify(TB_p terms, TFormula_p form, long quopt_limit)`
- `TFormula_p TFormulaSimplifyDecoded(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaSkolemizeOutermost(TB_p terms, TFormula_p form)`
- `TFormula_p TFormulaVarRename(TB_p terms, TFormula_p form)`
- `long TFormulaEstimateClauses(TB_p bank, TFormula_p form, bool pos)`
- `void TFormulaFindDefs(TB_p bank, TFormula_p form, int polarity, long def_limit, NumXTree_p *defs, PStack_p renamed_forms)`
- `void WTFormulaConjunctiveNF(WFormula_p form, TB_p terms)`
- `void WTFormulaConjunctiveNF3(WFormula_p form, TB_p terms, long miniscope_limit, bool fool_unroll)`

## Implementation Notes

### Internal Functions

- `fold_and_or`
- `miniscope_qall`
- `miniscope_qex`
- `negate_form`
- `simplify_args`
- `term_compare`
- `tform_mark_varocc`
- `unroll_binary`

### Source-Level Behavior

- `tprop_arg_return_other`: If one of the args is a propositional formula of the desired type, return the other one, else return NULL.
- `tprop_arg_return`: If one of the args is a propositional formula of the desired type, return it, else return NULL.
- `negate_form`: Negate the formula, and flatten the negation (only one step).
- `fold_and_or`: Make a formula which applies args to a binary symbol fc.
- `unroll_binary`: Puts all the arguments of binary fcode fc to args.
- `troot_nnf`: Apply all NNF-transformation rules that can be applied at the root level form and return it.
- `tformula_rec_skolemize`: Recursively Skolemize form. Note that it is not quite trivial that it this works, as it works on a shared structure, and the same subformula may occur in different contexts. It _does_ work (I hope) because we require that every quantor binds a distinct variable, and hence terms that were originally equal are either invariant with respect to context (i.e. th...
- `tformula_rename_test`: Return true if the formula at argument position i should be renamed, false otherwise. Polarity is the polarity of root, not root|i. def_limit determines how often a subformula can be replicated before it is renamed.
- `extract_formula_core`: Remove all (universal) quantifiers from Skolemized form in NNF and push the corresponding variables onto varstack.
- `extract_formula_core2`: Remove all quantifiers from form in NNF and push the corresponding quantifier/variable pairs onto varstack.
- `tform_mark_varocc`: Mark all subforms/subterms in form in which var occurs.
- `miniscope_qex`: Assume var is existentially quantified in var and move the quantifier inward as far as possible. Assumes that form is in NNF and that TPCheckFlag is set in all subformulas of form in which var occurs.
- `miniscope_qall`: Assume var is universally quantified in var and move the quantifier inward as far as possible. Assumes that form is in NNF and that TPCheckFlag is set in all subformulas of form in which var occurs.
- `tform_find_miniscopeable`: Find all maximal miniscopable subformulas. A formula is miniscopable, if it has at most limit subformulae, starts with a universal quantifier, and contains an existential quantifier (we only miniscope with the goal of moving existential quantifier out of the scope of universal quantifiers - and that not at all costs ;-). Return value is the size, candidates...
- `tform_copy_mod`: Copy a formula, following "binding" when it is set. Ground formumas, variables, and literals are returned as-is.
- `do_simplify_decoded`: Function that actually performs the simplification on decoded formulas.
- `TFormulaEstimateClauses`: Given a formula, estimate how many clauses would be generated by it. Assumes that formulas with TPCheckFlag are renamed into atoms. If too many clauses result, just return TFORM_MANY_CLAUSES. Variables: -
- `TFormulaDefRename`: Given a tformula, return a renaming atom for it and register the (potential) need for a renaming formula of the proper polarity in defs. In defs, the key is the entry_no, val1 is the most general polarity, and val2 is the renaming atom.
- `TFormulaFindDefs`: Find all useful definitions in form and enter them in defs and renamed_forms. def_limit determines when a formula is replicated sufficiently often to warrant renaming. Remember: TPCheckFlag means we already have a definition for this formula (though possibly not of the right polarity).
- `TFormulaCopyDef`: Copy a formula, replacing all defined subformulas (except for the blocked one, if any) with the proper definition). Record _all_ definitions (but not sub-definitions) on the stack (by pushing the definition numbers onto the stack).
- `TFormulaNegAlloc`: Return a formula equivalent to ~form. If form is of the form ~f, return f, otherwise ~form.
- `TFormulaExpandLiterals`: - Make all negation signs explicit - $neqn(a,b)=> ~$eqn(a,b) - Expand literals with Boolean operands - $eqn(x,y) => x <=> y. This is used before FOOL-Unrolling, to clearly identify Boolean positions and make sure that unrolling happens at the right polarity.
- `TFormulaReEncodeLiterals`: Find non-trivial Boolean terms and re-encode them as equations/disequations.
- `TFormulaSimplify`: Maximally simplify a formula using (primarily) the simplification rules (from [NW:SmallCNF-2001]). P | P => P P | T => T P | F => P P & P => F P & T => P P & F -> F ~T = F ~F = T P <-> P => T P <-> F => ~P P <-> T => P P <~> P => ~(P<->P) P -> P => T P -> T => T P -> F => ~P ... We only check for redundant quantifiers in "small" formulas (weight less than q...
- `TFormulaSimplifyDecoded`: Like TFromulaSimplify, but works on decoded formulas and performs some more simplifications [http://ceur-ws.org/Vol-2752/paper11.pdf]
- `TFormulaNNF`: Destructively transform a (simplified) formula into NNF.
- `TFormulaMiniScope`: Perform mini-scoping, i.e. move quantors inward as far as possible.
- `TFormulaMiniScope2`: Perform mini-scoping, i.e. move quantors inward as far as possible. Assumes that variables for each quantifier are unique, and that the formula is in NNF. Only a finite amount of work is spent on the formula (as defined by miniscope_limit).
- `TFormulaMiniScope3`: Perform (conditional) mini-scoping, i.e. move quantors inward as far as possible if there are "small" subformulas that might profit from miniscoping. Assumes that variables for each quantifier are unique, and that the formula is in NNF.
- `TFormulaVarRename`: Convert the formula into one where all the bound variables have been replaced by fresh one. IMPORTANT PRECONDITION: terms->vars->f_count _must_ point to a variable bigger than all in form.
- `FormulaSkolemizeOutermost`: Skolemize a formula in an outermost manner. Interpretes the formula as its universal closure, i.e. globally free variables in form are used as Skolem function arguments. Also assumes that every quantor binds a new variable.
- `TFormulaShiftQuantors`: Shift all remaining all-quantors outward. This has several premises: - All quantified variables are disjoint from each other and from the free variables. - The formula is in negation normal form. - All quantifiers are universal.
- `TFormulaShiftQuantors2`: Shift all all-quantors outward. This has several premises: - All quantified variables are disjoint from each other and from the free variables. - The formula is in negation normal form.
- `TFormulaDistributeDisjunctions`: Apply distributivity law to transform a suitably preprocessed formula into conjunctive normal form.
- `WTFormulaConjunctiveNF`: Transform a formula into Conjunctive Normal Form.
- `WTFormulaConjunctiveNF3`: Transform a formula into Conjunctive Normal Form.

### Dependencies

- `"ccl_formulafunc.h"`
- `"ccl_tcnf.h"`
- `"cte_lambda.h"`
- `<ccl_clausesets.h>`
- `<ccl_tformulae.h>`
- `<clb_numxtrees.h>`

### Compile-Time Conditions

- `CCL_TCNF`
- `NDEBUG`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_tcnf.h`, `CLAUSES/ccl_tcnf.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 3003 lines, 16 scanned public declarations, 8 scanned internal function definitions, and 36 structured function-comment blocks.
- Functions implementing the CNF conversion of first order formulae encoded as terms. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- A focused Rust port of `TFormulaSimplifyDecoded` is staged for `BooleanSimplification` in forward contraction. It covers decoded two-argument `and`/`or`, unary decoded `and`/`or` constant-to-DB-lambda cases, closed-lambda decoded quantifier matrix removal, `not`, implication, equivalence/xor, `$eq`/`$neq`, recursive argument simplification, neutral/absorbing constants, duplicate removal, complement detection, and literal remapping through the term bank.
- `src/clauses/clausefunc.rs` now stages `TFormulaSimplify` for a single term-encoded formula, including recursive child simplification, root-loop simplification for `not`, `or`, `and`, equivalence, implication, XOR, reverse implication, NAND, NOR, propositional-constant encodings, handle-identity duplicate checks, and small redundant-quantifier removal through `TFormulaVarIsFree`.
- `src/clauses/clausefunc.rs` now stages `TFormulaNegAlloc` for a single term-encoded formula: it returns the child of a root `$not`, otherwise allocates a root `$not` formula without applying constant, equality, equivalence, or XOR-specific negation rules.
- `src/clauses/clausefunc.rs` now stages `TFormulaExpandLiterals` for a single term-encoded formula, including C's disequality-to-negated-equality rewrite, recursive argument rebuild, Boolean equality-to-equivalence rewrite, internal Boolean `$eq(F,$true)` unwrapping, and the free-variable exception.
- `src/clauses/clausefunc.rs` now stages `TFormulaEstimateClauses` for a single term-encoded formula, including positive/negative polarity arithmetic for `and`, `or`, implication, equivalence, negation, and quantifiers; `TPCheckFlag` definition atoms; `$true`/`$false`; applied free variables; arrow-typed formulas; and the C `TFORM_MANY_LIMIT`/`TFORM_MANY_CLAUSES` cutoff.
- `src/clauses/clausefunc.rs` now stages the definition-renaming helpers `TFormulaDefRename`, `TFormulaFindDefs`, and `TFormulaCopyDef` for single term-encoded formulas, including entry-number keyed definition records, `TPCheckFlag` marking, polarity generalization, free-variable-dependent Boolean definition atoms in C's pointer-splay stack order, depth-first candidate discovery, and blocked self-definition copying.
- `src/clauses/clausefunc.rs` now stages `TFormulaCreateDef`, `TFormulaMarkPolarity`, and `TFormulaDecodePolarity` for single term-encoded formulas, including C's polarity-direction choice for definitions, universal closure over definition-atom variables after `TermCollectVariables`' reverse explicit-stack traversal and a second pointer-splay conversion, literal-skipping polarity marking, implication/negation polarity inversion, equivalence both-polarity marking, quantifier-body propagation, and polarity flag decoding.
- `src/clauses/clausefunc.rs` now stages `TFormulaNNF` for a single simplified term-encoded formula, including C's root negation rewrites, implication elimination, polarity-aware equivalence expansion, recursive normalization below `$not`, quantifiers, `and`, and `or`, truth-constant literal encoding, applied-free-variable encoding, and predicate-as-equality fallback.
- `src/clauses/clausefunc.rs` now stages `TFormulaMiniScope` for a single term-encoded NNF formula, including C's left-first free-variable test, universal-over-conjunction and existential-over-disjunction splitting, recursive reprocessing after changed children, and quantifier shadowing in `TFormulaVarIsFree`.
- `src/clauses/clausefunc.rs` now stages `TFormulaMiniScope3` for a single term-encoded NNF formula, including C's conditional universal/existential candidate scan, miniscope-limit gate, maximal-candidate replacement through temporary formula bindings, and identity-ordered candidate traversal.
- `src/clauses/clausefunc.rs` now stages `TFormulaVarRename` for a single term-encoded formula, including fresh replacement of bound variables, nested quantifier shadowing through temporary bindings, `DEREF_ALWAYS` copying for literals and Boolean terms, restoration of previous variable bindings, and the special `$let`/`$ite` path that recursively rewrites every argument.
- `src/clauses/clausefunc.rs` now stages `TFormulaSkolemizeOutermost` for a single term-encoded formula, including global-free-variable dependencies, universal-variable dependency stacking, existential removal through typed Skolem terms/predicates, temporary binding restoration, literal and non-logical Boolean-term copying with `DEREF_ALWAYS`, and connective rebuilds only when a child changes.
- `src/clauses/clausefunc.rs` now stages `TFormulaShiftQuantors` and `TFormulaShiftQuantors2` for single term-encoded NNF formulas. They strip leading quantifiers, descend through `and`/`or`, rebuild changed connectives through the term bank, and then wrap the collected variables back around the formula in the C stack order; the `2` variant preserves mixed universal/existential quantifier codes.
- `src/clauses/clausefunc.rs` now stages `TFormulaDistributeDisjunctions` for a single suitably preprocessed NNF formula, including recursive body rebuilding under quantifiers and C's left/right operand order when distributing an `or` over either side's `and`.
- `src/clauses/clausefunc.rs` now stages term-level `WTFormulaConjunctiveNF` and `WTFormulaConjunctiveNF3` orchestration wrappers as `tformula_conjunctive_nf` and `tformula_conjunctive_nf3`. They preserve the C phase order, C's `1000` simplification budget, the fresh-variable seeding boundary before bound-variable renaming, conditional miniscope limit handling, optional FOOL unrolling before the final NNF pass, distribution, and the derivation opcodes that a later `WFormula` owner should attach when each phase changes the formula.

### Change Later

- `TFormulaSimplify` represents Boolean constants as equality/disequality formulas over `$true`, uses pointer identity (`TFormulaEqual`) rather than structural equality for duplicate formula rewrites, and repeats root rewrites until stable. Rust preserves handle identity; a later formula owner should make this identity-vs-structure distinction explicit before introducing canonical formula equality.
- `TFormulaSimplify` only removes redundant quantified variables unconditionally when `form->v_count` is zero, otherwise it runs `TFormulaVarIsFree` only when the whole quantified formula's cached weight is at or below `quopt_limit`. Rust preserves that performance gate; later cleanup should make redundant-quantifier removal a separate budgeted simplification pass.
- `do_simplify_decoded` sorts flattened decoded `and`/`or` arguments by raw term pointer (`PCmp`) before deduplicating and folding the formula back into binary shape. This makes the resulting shared formula shape allocation-order dependent. Rust mirrors handle-identity ordering for compatibility; a later cleaned formula canonicalizer should switch to structural ordering only behind proof-search and proof-output reference tests.
- `do_simplify_decoded` simplifies the child of unary decoded `and`/`or`, but when that child is neither the neutral nor absorbing Boolean constant it returns the original unary formula cell rather than the recursively simplified child. Rust preserves that visible behavior; revisit whether this is an accidental missed simplification once higher-order decoded-formula tests cover non-constant unary bodies.
- `do_simplify_decoded` recognizes decoded quantifiers as unary `$qex`/`$qall` cells whose single argument is a DB lambda, despite the normal signature arity for those symbols being binary. Rust preserves this accepted shape; a later formula layer should model decoded binder forms directly instead of overloading ordinary function-symbol arity.
- `TFormulaNegAlloc` deliberately performs only root `$not` cancellation/allocation even though nearby `negate_form` simplifies Boolean constants, equality/disequality, and equivalence/XOR. Rust keeps these as separate helpers; a later formula layer should make the distinction between syntactic negation allocation and semantic negation normalization explicit.
- `TFormulaExpandLiterals` treats Boolean equality asymmetrically: only the left operand's type and free-variable status decide whether `$eq(left,right)` becomes `$equiv(left,right)` or, for non-answer internal Boolean formulas, unwraps `$eq(left,$true)` to `left`. Rust preserves this left-biased behavior; a later formula layer should make Boolean literal expansion symmetric only if compatibility tests allow it.
- `TFormulaEstimateClauses` is deliberately approximate: `TPCheckFlag` subforms, arrow-typed formulas, literals, and applied free variables all collapse to one clause, and estimates above `1024` are represented by `LONG_MAX` rather than a precise count. Rust preserves the sentinel model; a later formula owner should expose this as an explicit growth budget instead of mixing marker flags with structural estimation.
- `TFormulaDefRename` keys definitions by `form->entry_no`, mutates `TPCheckFlag` on shared formula terms, and generalizes polarity to `0` on the first conflicting request. `TFormulaCopyDef` later interprets the same `NumXTree` value slots differently after definition introduction. Rust preserves this with an explicit definition-entry cell; a later formula owner should replace the marker flag and slot reuse with phase-specific definition handles.
- `TFormulaDefRename` obtains definition-predicate arguments by recursively inserting free variables into a raw-pointer splay tree, while `TFormulaCreateDef` recollects variables from that generated atom through `TermCollectVariables`' reverse argument-stack visit and another splay tree before wrapping quantifiers. Definition argument and binder order therefore depend on temporary tree shape and allocator identity. Rust preserves both traversals for compatibility; a cleaned definition API should carry one explicit stable dependency list through atom creation and universal closure.
- `tformula_rename_test` computes implication subformula polarity with `subform_sign = (pos==2?true:false)`, but the only passed positions are `0` and `1`, so positive implication consequents are tested with negative polarity. Rust preserves that artifact; revisit it only behind definitional-CNF reference tests because changing it can alter which formulas are renamed.
- `TFormulaCreateDef` relies on earlier `TFormulaMarkPolarity` side effects on shared term cells, then asserts that one-sided definitions are not used with an incompatible marker. It also universally closes definitions over variables collected from the generated definition atom, not directly from the defined formula. Rust mirrors this marker-and-definition-atom contract; a later formula owner should use explicit polarity maps and definition dependency lists instead of shared term-cell flags.
- `TFormulaNNF` combines root connective rewriting with Boolean predicate-to-equality encoding, and the `polarity` argument is relevant only when expanding equivalence roots. Rust preserves this phase coupling for compatibility; a later formula owner should make the polarity contract and the predicate-encoding boundary explicit before exposing NNF as a general-purpose formula transform.
- `TFormulaMiniScope` tests the left branch before the right branch; if a quantified variable is absent from both sides of a binary body, the quantifier is pushed onto the right branch instead of being removed. Rust preserves this branch order; after compatibility, redundant-quantifier removal should be handled by an explicit simplification pass rather than miniscope's incidental ordering.
- `TFormulaMiniScope3` discovers candidates with `tform_find_miniscopeable`, whose non-quantifier size update is `size += size + child_size` and whose candidate tree is keyed by raw formula pointers. It then stores mini-scoped replacements in `Term.binding` on the selected formula cells before `tform_copy_mod` follows those bindings. Rust preserves the size accounting, identity ordering, and temporary binding phase; a later formula owner should replace this with explicit candidate handles and a documented work budget.
- `TFormulaVarRename` mutates `Term.binding` on quantified variables while copying literals and Boolean terms with `DEREF_ALWAYS`, and relies on the caller to advance the fresh-variable bank past every variable in the source formula. Rust preserves the temporary binding side channel with an explicit restore guard; a later formula owner should replace it with a scoped variable-renaming map and make the freshness precondition a typed phase boundary.
- `tformula_rek_skolemize` mutates `Term.binding` on existential variables while recursively copying affected literals and Boolean terms, and it relies on prior variable-renaming so shared subformulas are not context-sensitive except for ground cells. Rust preserves the binding side channel with restore guards; a later formula owner should use an explicit substitution/dependency map instead of term-cell mutation.
- In `tformula_rek_skolemize`, the non-logical Boolean-term branch assigns `TFormulaCopy` to a local `handle` but never assigns it back to `form`. Rust returns the copied/dereferenced Boolean term because that matches the surrounding literal-copy and `TFormulaVarRename` behavior; keep this C quirk under review if strict bug compatibility becomes necessary.
- `extract_formula_core` and `extract_formula_core2`, the helpers behind C `TFormulaShiftQuantors` and `TFormulaShiftQuantors2`, rely on callers having already produced suitable NNF: they strip leading quantifiers and only search below `and`/`or`, leaving quantifiers under other connectives untouched. Rust mirrors that precondition-heavy shape; a later formula owner should make the NNF/quantifier phase boundary explicit before allowing these helpers on arbitrary formulas.
- `TFormulaDistributeDisjunctions` recursively expands `or(and(...), ...)` and `or(..., and(...))` without a local clause-growth guard; C relies on earlier estimation/definition-renaming phases and preserves operand order rather than canonicalizing the generated disjunctions. Rust mirrors that helper-level behavior, but the final clausifier should make growth controls and canonicalization boundaries explicit.
- `WTFormulaConjunctiveNF` and `WTFormulaConjunctiveNF3` bundle destructive formula replacement, proof-documentation calls, and derivation-stack pushes behind pointer-identity change checks. Rust keeps the lower-level term transformation and emitted phase opcodes separate, and `WFormulaCNF2` now stores those opcodes on the wrapper derivation stack; proof-document side effects should remain explicit instead of hiding inside mutation helpers.

- C CNF conversion relies on destructive formula mutation, shared term/formula structure, polarity markers, and side-effecting definition archives. Rust should mirror the observable clause output and proof metadata first, but the final clausifier should expose ownership and mutation phases explicitly so temporary bridges do not duplicate ad hoc fragments of this pipeline.
- `TFormulaSkolemizeOutermost` seeds recursive Skolemization with globally free variables and `tformula_rek_skolemize` mutates variable bindings while pushing/popping universal variables through a raw `PStack`. Rust now mirrors that stack order with explicit vectors and restore guards; the final formula owner should keep the dependency stack explicit and avoid exposing temporary term bindings outside the clausification phase.

<!-- END MANUAL REVIEW: c_source_docs -->
