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
- Change-later candidate: C CNF conversion relies on destructive formula mutation, shared term/formula structure, polarity markers, and side-effecting definition archives. Rust should mirror the observable clause output and proof metadata first, but the final clausifier should expose ownership and mutation phases explicitly so temporary bridges do not duplicate ad hoc fragments of this pipeline.
- Change-later candidate: `TFormulaSkolemizeOutermost` seeds recursive Skolemization with globally free variables and `tformula_rek_skolemize` mutates variable bindings while pushing/popping universal variables through a raw `PStack`. Rust's temporary FOF bridge mirrors that stack order only for occurring dependencies in supported fragments; the final formula owner should keep the dependency stack explicit and avoid exposing temporary term bindings outside the clausification phase.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
