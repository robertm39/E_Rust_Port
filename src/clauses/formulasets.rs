use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_print_lop_format_string, clause_print_tptp_format_string, clause_print_tstp_core_string,
    clause_tstp_string, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_IGNORE_PROPS, CP_INPUT_FORMULA, CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM,
    CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA, CP_TYPE_NEG_CONJECTURE,
    CP_TYPE_QUESTION,
};
use crate::clauses::clausefunc::{
    post_cnf_encode_clause_terms, tformula_add_quantor,
    tformula_app_encode_string_with_type_suffixes, tformula_clause_closed_encode,
    tformula_clause_encode, tformula_closure, tformula_collect_clause, tformula_collect_free_vars,
    tformula_conjunctive_nf3, tformula_copy_def, tformula_create_def, tformula_decode_polarity,
    tformula_encode_predicate_as_eqn, tformula_fcode_alloc, tformula_find_defs,
    tformula_has_free_vars, tformula_is_complex_bool, tformula_is_literal, tformula_is_prop_true,
    tformula_lift_ite, tformula_lift_lets, tformula_mark_polarity, tformula_preload_types,
    tformula_simplify, tformula_to_cnf, tformula_to_cnf_with_docs, tformula_tptp_string,
    tformula_unencode_root_eqn, tformula_unroll_fool_result, tformula_var_rename,
    TFormulaDefinitions, TFormulaToCnfDocContext, TFormulaToCnfInput, TFormulaTptpPrintOptions,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_push_formula_derivation, formula_dummy_quote_parent_ref, push_formula_derivation_stack,
    DerivationEntry, FormulaDerivationRef, DC_ANNO_QUESTION, DC_APPLY_DEF, DC_DIST_DISJUNCTIONS,
    DC_EQ_TO_EQ, DC_FNNF, DC_FOF_QUOTE, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL, DC_INTRO_DEF, DC_LIFT_ITE,
    DC_LIFT_LAMBDAS, DC_NEGATE_CONJECTURE, DC_SHIFT_QUANTORS, DC_SKOLEMIZE, DC_SPLIT_EQUIV,
    DC_VAR_RENAME,
};
use crate::clauses::eqn_props::{EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
use crate::clauses::garbage_coll::tb_gc_collect;
use crate::clauses::inferencedoc::{
    FormulaCreationInference, FormulaCreationParents, FormulaDocView, FormulaModificationInference,
    ProofDocSession, ProofDocWriteResult,
};
use crate::terms::functypes::FunCode;
use crate::terms::lambda::{
    abstract_vars, apply_terms, beta_normalize_db, decode_formulas_for_cnf, lambda_eta_reduce_db,
    lambda_normalize_db, lambda_to_forall, named_to_db, unfold_lambda, whnf_step,
};
use crate::terms::match_mgu::subst_compute_match;
use crate::terms::signature::{Signature, SIG_NAMED_LAMBDA_CODE};
use crate::terms::simpletypes::{arrow_type_flattened, type_is_predicate};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::tb_term_collect_subterms;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_compute_order, term_has_f_code, term_is_untyped, term_standard_weight,
};
use crate::terms::termtypes::{
    term_del_prop, term_has_interpreted_symbol, term_identity_id, DerefType, Term, TermProperties,
    TP_CHECK_FLAG, TP_NEG_POLARITY, TP_OP_FLAG, TP_POS_POLARITY,
};
use crate::terms::termvars::VarBank;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static WRAPPED_FORMULA_ENTRY_ID: AtomicU64 = AtomicU64::new(1);
static FORMULA_IDENT_COUNTER: AtomicI64 = AtomicI64::new(i64::MIN);
const TFORMULA_GC_LIMIT_NUMERATOR: i64 = 3;
const TFORMULA_GC_LIMIT_DENOMINATOR: i64 = 2;
const MAX_DEF_SYMBOL_REWRITE_STEPS: i32 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaPrintFormat {
    Lop,
    Tptp,
    Tstp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaTstpCompleteness {
    Complete,
    Open,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaTstpClauseMode {
    AsFormula,
    AsClauseCore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaTstpPrintOptions {
    pub full_terms: bool,
    pub completeness: FormulaTstpCompleteness,
    pub clause_mode: FormulaTstpClauseMode,
    pub keep_input_names: bool,
}

impl FormulaTstpPrintOptions {
    #[must_use]
    pub const fn complete_formula(full_terms: bool, keep_input_names: bool) -> Self {
        Self {
            full_terms,
            completeness: FormulaTstpCompleteness::Complete,
            clause_mode: FormulaTstpClauseMode::AsFormula,
            keep_input_names,
        }
    }

    #[must_use]
    pub const fn open_formula(full_terms: bool, keep_input_names: bool) -> Self {
        Self {
            full_terms,
            completeness: FormulaTstpCompleteness::Open,
            clause_mode: FormulaTstpClauseMode::AsFormula,
            keep_input_names,
        }
    }

    #[must_use]
    pub const fn with_clause_mode(mut self, clause_mode: FormulaTstpClauseMode) -> Self {
        self.clause_mode = clause_mode;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaProofDocRenderOptions {
    pub full_terms: bool,
    pub problem_type: ProblemType,
}

impl FormulaProofDocRenderOptions {
    #[must_use]
    pub const fn new(full_terms: bool, problem_type: ProblemType) -> Self {
        Self {
            full_terms,
            problem_type,
        }
    }
}

pub struct WrappedFormulaCnfDocContext<'a, W: fmt::Write> {
    output: &'a mut W,
    session: &'a mut ProofDocSession,
    render_options: FormulaProofDocRenderOptions,
}

impl<'a, W: fmt::Write> WrappedFormulaCnfDocContext<'a, W> {
    #[must_use]
    pub const fn new(
        output: &'a mut W,
        session: &'a mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
    ) -> Self {
        Self {
            output,
            session,
            render_options,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WrappedFormulaCnfResult {
    pub clauses_generated: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WrappedFormulaCnfDocResult {
    pub cnf: WrappedFormulaCnfResult,
    pub formula_write_results: Vec<ProofDocWriteResult>,
    pub clause_write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetCnfResult {
    pub clauses_generated: i64,
    pub original_formulas_archived: i64,
    pub cnf_formulas_archived: i64,
    pub term_garbage_collections: i64,
    pub terms_recovered_by_gc: i64,
    pub formulas_named_to_db: i64,
    pub formulas_ites_lifted: i64,
    pub formulas_lets_lifted: i64,
    pub formulas_def_symbols_unfolded: i64,
    pub unfolded_definition_rhs_rewritten: i64,
    pub unfolded_definitions_archived: i64,
    pub unfolded_original_definitions_archived: i64,
    pub definition_symbol_applications: i64,
    pub formulas_lambda_normalized: i64,
    pub clauses_lambdas_lifted: i64,
    pub lambda_lift_definitions_archived: i64,
    pub lambda_lift_definition_clauses_generated: i64,
    pub formulas_simplified: i64,
    pub boolean_equalities_replaced: i64,
    pub formulas_fool_unrolled: i64,
    pub definitions_introduced: i64,
    pub definition_applications: i64,
    pub definition_formulas_archived: i64,
    pub active_definition_formulas_inserted: i64,
    pub formulas_rewritten_by_defs: i64,
    pub quoted_formula_sources: Vec<FormulaDerivationRef>,
    pub formula_derivation_ops: Vec<i64>,
}

impl FormulaSetCnfResult {
    fn add_higher_order_preprocess(&mut self, result: &FormulaSetHigherOrderPreprocessResult) {
        self.formulas_named_to_db += result.formulas_named_to_db;
        self.formulas_ites_lifted += result.formulas_ites_lifted;
        self.formulas_lets_lifted += result.formulas_lets_lifted;
        self.formulas_def_symbols_unfolded += result.formulas_def_symbols_unfolded;
        self.unfolded_definition_rhs_rewritten += result.unfolded_definition_rhs_rewritten;
        self.unfolded_definitions_archived += result.unfolded_definitions_archived;
        self.unfolded_original_definitions_archived +=
            result.unfolded_original_definitions_archived;
        self.definition_symbol_applications += result.definition_symbol_applications;
        self.formulas_lambda_normalized += result.formulas_lambda_normalized;
        self.formula_derivation_ops
            .extend(result.formula_derivation_ops.iter().copied());
    }

    fn add_fool_unroll(&mut self, result: &FormulaSetFoolUnrollResult) {
        self.boolean_equalities_replaced += result.boolean_equalities_replaced;
        self.formulas_fool_unrolled += result.formulas_unrolled;
        self.formula_derivation_ops
            .extend(result.formula_derivation_ops.iter().copied());
    }

    fn add_simplify(&mut self, result: &FormulaSetSimplifyResult) {
        self.formulas_simplified += result.formulas_changed;
        self.term_garbage_collections += result.term_garbage_collections;
        self.terms_recovered_by_gc += result.terms_recovered_by_gc;
        self.formula_derivation_ops
            .extend(result.formula_derivation_ops.iter().copied());
    }

    fn add_introduce_defs(&mut self, result: &FormulaSetIntroduceDefsResult) {
        self.definitions_introduced += result.definitions_introduced;
        self.definition_applications += result.definition_applications;
        self.definition_formulas_archived += result.archived_definitions;
        self.active_definition_formulas_inserted += result.active_definitions_inserted;
        self.formulas_rewritten_by_defs += result.formulas_rewritten;
        self.formula_derivation_ops
            .extend(result.formula_derivation_ops.iter().copied());
    }

    fn add_wrapped_cnf(&mut self, result: &WrappedFormulaCnfResult) {
        self.clauses_generated += result.clauses_generated;
        self.formula_derivation_ops
            .extend(result.formula_derivation_ops.iter().copied());
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetCnfDocResult {
    pub cnf: FormulaSetCnfResult,
    pub preprocessing_write_results: Vec<ProofDocWriteResult>,
    pub simplification_write_results: Vec<ProofDocWriteResult>,
    pub definition_write_results: Vec<ProofDocWriteResult>,
    pub definition_application_write_results: Vec<ProofDocWriteResult>,
    pub cnf_formula_write_results: Vec<ProofDocWriteResult>,
    pub cnf_clause_write_results: Vec<ProofDocWriteResult>,
}

impl FormulaSetCnfDocResult {
    fn add_preprocess_doc(&mut self, result: FormulaSetHigherOrderPreprocessDocResult) {
        self.cnf.add_higher_order_preprocess(&result.preprocess);
        self.preprocessing_write_results
            .extend(result.write_results);
    }

    fn add_simplify_doc(&mut self, result: FormulaSetSimplifyDocResult) {
        self.cnf.add_simplify(&result.simplify);
        self.simplification_write_results
            .extend(result.write_results);
    }

    fn add_introduce_defs_doc(&mut self, result: FormulaSetIntroduceDefsDocResult) {
        self.cnf.add_introduce_defs(&result.introduce);
        self.definition_write_results
            .extend(result.definition_write_results);
        self.definition_application_write_results
            .extend(result.application_write_results);
    }

    fn add_wrapped_cnf_doc(&mut self, result: WrappedFormulaCnfDocResult) {
        self.cnf.add_wrapped_cnf(&result.cnf);
        self.cnf_formula_write_results
            .extend(result.formula_write_results);
        self.cnf_clause_write_results
            .extend(result.clause_write_results);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaSetCnfOptions {
    pub miniscope_limit: i64,
    pub def_limit: i64,
    pub fool_unroll: bool,
    pub higher_order: FormulaSetHigherOrderCnfOptions,
    pub problem_type: ProblemType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaSetHigherOrderCnfOptions {
    pub lambda_to_forall: bool,
    pub lift_lambdas: bool,
    pub unfold_only_forms: bool,
}

impl Default for FormulaSetHigherOrderCnfOptions {
    fn default() -> Self {
        Self {
            lambda_to_forall: true,
            lift_lambdas: true,
            unfold_only_forms: true,
        }
    }
}

impl FormulaSetCnfOptions {
    #[must_use]
    pub const fn new(miniscope_limit: i64, fool_unroll: bool, problem_type: ProblemType) -> Self {
        Self {
            miniscope_limit,
            def_limit: 0,
            fool_unroll,
            higher_order: FormulaSetHigherOrderCnfOptions {
                lambda_to_forall: true,
                lift_lambdas: true,
                unfold_only_forms: true,
            },
            problem_type,
        }
    }

    #[must_use]
    pub const fn with_def_limit(mut self, def_limit: i64) -> Self {
        self.def_limit = def_limit;
        self
    }

    #[must_use]
    pub const fn with_lambda_to_forall(mut self, lambda_to_forall: bool) -> Self {
        self.higher_order.lambda_to_forall = lambda_to_forall;
        self
    }

    #[must_use]
    pub const fn with_lift_lambdas(mut self, lift_lambdas: bool) -> Self {
        self.higher_order.lift_lambdas = lift_lambdas;
        self
    }

    #[must_use]
    pub const fn with_unfold_only_forms(mut self, unfold_only_forms: bool) -> Self {
        self.higher_order.unfold_only_forms = unfold_only_forms;
        self
    }
}

const fn formula_set_gc_threshold(old_nodes: i64) -> i64 {
    old_nodes.saturating_mul(TFORMULA_GC_LIMIT_NUMERATOR) / TFORMULA_GC_LIMIT_DENOMINATOR
}

fn cnf_phase_formula_inference(op: i64) -> Option<FormulaModificationInference> {
    match op {
        DC_FOF_SIMPLIFY => Some(FormulaModificationInference::Simplification),
        DC_FNNF => Some(FormulaModificationInference::Nnf),
        DC_SHIFT_QUANTORS => Some(FormulaModificationInference::ShiftQuantors),
        DC_VAR_RENAME => Some(FormulaModificationInference::VarRename),
        DC_SKOLEMIZE => Some(FormulaModificationInference::Skolemize),
        DC_DIST_DISJUNCTIONS => Some(FormulaModificationInference::Distribute),
        DC_FOOL_UNROLL => None,
        _ => panic!("unexpected CNF formula derivation opcode {op}"),
    }
}

fn term_has_named_lambda(term: &Term) -> bool {
    term.f_code() == SIG_NAMED_LAMBDA_CODE
        || term
            .argument_clones()
            .into_iter()
            .flatten()
            .any(|arg| term_has_named_lambda(&arg))
}

fn lambda_definition_sides(bank: &TermBank, formula: &Term) -> Option<(Term, Term)> {
    let signature = bank.signature();
    let mut body = formula.clone();
    while body.f_code() == signature.qall_code() && body.arity() == 2 {
        body = body.argument(1)?;
    }

    if body.f_code() == signature.eqn_code() && body.arity() == 2 {
        return Some((body.argument(0)?, body.argument(1)?));
    }

    if body.f_code() != signature.equiv_code() || body.arity() != 2 {
        return None;
    }
    let left = body.argument(0)?;
    if left.f_code() != signature.eqn_code()
        || left.arity() != 2
        || left.argument(1).as_ref() != Some(bank.true_term())
    {
        return None;
    }
    Some((left.argument(0)?, body.argument(1)?))
}

fn create_definition_symbol_map(
    set: &FormulaSet,
    bank: &mut TermBank,
    unfold_only_forms: bool,
) -> Result<DefinitionSymbolMap, Diagnostic> {
    let mut definitions = BTreeMap::new();
    let mut recognized_entry_ids = Vec::new();
    for formula in set.iter() {
        if !formula.query_prop(CP_IS_LAMBDA_DEF) {
            continue;
        }

        let Some((lhs, rhs)) = lambda_definition_sides(bank, formula.formula()) else {
            continue;
        };
        let mut bvars = Vec::new();
        let _lhs_matrix = unfold_lambda(&lhs, &mut bvars);
        for variable in &mut bvars {
            let type_ = variable
                .type_()
                .expect("lambda-definition binder must have a type");
            *variable = bank.vars().get_fresh_var(&type_);
        }
        let lhs_applied = apply_terms(bank, &lhs, &bvars)?;
        let lhs_body = beta_normalize_db(bank, &lhs_applied)?;
        let rhs_applied = apply_terms(bank, &rhs, &bvars)?;
        let rhs_applied = beta_normalize_db(bank, &rhs_applied)?;

        let lhs_is_predicate = lhs.type_().as_ref().is_some_and(type_is_predicate);
        if (unfold_only_forms && !lhs_is_predicate)
            || lhs.f_code() <= bank.signature().internal_symbols()
            || rhs == *bank.true_term()
        {
            continue;
        }

        let mut abstraction_vars = Vec::new();
        let mut seen_vars = BTreeSet::new();
        let mut is_definition = true;
        for arg in lhs_body.argument_clones().into_iter().flatten() {
            let arg = if arg.f_code() == bank.signature().eqn_code()
                && arg.arity() == 2
                && arg.argument(1).as_ref() == Some(bank.true_term())
            {
                arg.argument(0)
                    .expect("encoded definition argument left side is uninitialized")
            } else {
                arg
            };
            if !arg.is_free_var() || !seen_vars.insert(arg.f_code()) {
                is_definition = false;
                break;
            }
            abstraction_vars.push(arg);
        }
        if !is_definition || term_has_f_code(&rhs_applied, lhs_body.f_code()) {
            continue;
        }

        let rhs = abstract_vars(bank, &rhs_applied, &abstraction_vars)?;
        if tformula_has_free_vars(bank, &rhs).is_some() {
            continue;
        }

        let lhs_type = bank
            .signature()
            .get_type(lhs_body.f_code())
            .cloned()
            .expect("defined symbol must have a signature type");
        let lhs_symbol = Term::top_alloc(lhs_body.f_code(), 0);
        lhs_symbol.set_type(Some(lhs_type));
        let lhs_symbol = bank.term_top_insert(lhs_symbol)?;
        let eqn_code = bank.signature().eqn_code();
        let definition = tformula_fcode_alloc(bank, eqn_code, lhs_symbol, Some(rhs))?;
        let mut definition_wrapper = formula.flat_copy();
        definition_wrapper.set_formula(definition);
        let source = FormulaDerivationRef::new(formula.ident());
        definition_wrapper.push_formula_derivation(DC_FOF_QUOTE, Some(source), None);
        definitions.insert(lhs_body.f_code(), definition_wrapper);
        recognized_entry_ids.push(formula.entry_id());
    }

    Ok(DefinitionSymbolMap {
        definitions,
        recognized_entry_ids,
    })
}

fn refresh_qvars(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let mut bindings = BTreeMap::new();
    refresh_qvars_rek(bank, form, &mut bindings)
}

fn refresh_qvars_rek(
    bank: &mut TermBank,
    form: &Term,
    bindings: &mut BTreeMap<FunCode, Term>,
) -> Result<Term, Diagnostic> {
    if let Some(bound) = bindings.get(&form.f_code()) {
        if form.is_free_var() {
            return Ok(bound.clone());
        }
    }
    if form.is_db_var() || form.arity() == 0 {
        return Ok(form.clone());
    }

    let is_quantifier = {
        let signature = bank.signature();
        (form.f_code() == signature.qall_code() || form.f_code() == signature.qex_code())
            && form.arity() == 2
    };
    if is_quantifier {
        let variable = form
            .argument(0)
            .expect("quantifier variable is uninitialized");
        let body = form.argument(1).expect("quantifier body is uninitialized");
        let variable_type = variable
            .type_()
            .expect("quantified variable must have a type");
        let fresh_var = bank.vars().get_fresh_var(&variable_type);
        let previous = bindings.insert(variable.f_code(), fresh_var.clone());
        let refreshed_body = refresh_qvars_rek(bank, &body, bindings)?;
        if let Some(previous) = previous {
            bindings.insert(variable.f_code(), previous);
        } else {
            bindings.remove(&variable.f_code());
        }
        return tformula_fcode_alloc(bank, form.f_code(), fresh_var, Some(refreshed_body));
    }

    let copy = Term::top_copy_without_args(form);
    let mut changed = false;
    for (index, arg) in form.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let refreshed = refresh_qvars_rek(bank, &arg, bindings)?;
        if refreshed != arg {
            changed = true;
        }
        copy.set_argument(index, refreshed);
    }
    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(form.clone())
    }
}

fn do_rewrite_with_def_symbols(
    bank: &mut TermBank,
    term: &Term,
    def_map: &BTreeMap<FunCode, WrappedFormula>,
    used_defs: &mut BTreeSet<FunCode>,
    steps: &mut i32,
) -> Result<Term, Diagnostic> {
    if *steps <= 0 || term.is_any_var() {
        return Ok(term.clone());
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let rewritten = do_rewrite_with_def_symbols(bank, &arg, def_map, used_defs, steps)?;
        if rewritten != arg {
            changed = true;
        }
        copy.set_argument(index, rewritten);
    }

    let mut rewritten = if changed {
        bank.term_top_insert(copy)?
    } else {
        term.clone()
    };
    let Some(rhs) = def_map
        .get(&rewritten.f_code())
        .and_then(|definition| definition.formula().argument(1))
    else {
        return Ok(rewritten);
    };

    let rhs = refresh_qvars(bank, &rhs)?;
    let args = rewritten
        .argument_clones()
        .into_iter()
        .enumerate()
        .map(|(index, arg)| arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized")))
        .collect::<Vec<_>>();
    let applied = apply_terms(bank, &rhs, &args)?;
    rewritten = beta_normalize_db(bank, &applied)?;
    used_defs.insert(term.f_code());
    rewritten = do_rewrite_with_def_symbols(bank, &rewritten, def_map, used_defs, steps)?;
    *steps -= 1;
    Ok(rewritten)
}

fn definition_parent_refs(
    definitions: &BTreeMap<FunCode, WrappedFormula>,
    used_defs: &BTreeSet<FunCode>,
) -> Vec<FormulaDerivationRef> {
    used_defs
        .iter()
        .map(|code| {
            let definition = definitions
                .get(code)
                .unwrap_or_else(|| panic!("definition symbol {code} disappeared"));
            FormulaDerivationRef::new(definition.ident())
        })
        .collect()
}

fn unencode_eqns(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    bank.map_term(term, &mut |bank, candidate| {
        Ok(Some(tformula_unencode_root_eqn(bank, candidate)))
    })
}

#[derive(Clone)]
struct LooseDbReplacement {
    fresh_var: Term,
    db_var: Term,
}

fn unbind_loose_db_vars(
    bank: &mut TermBank,
    depth: FunCode,
    term: &Term,
    replacements: &mut BTreeMap<FunCode, LooseDbReplacement>,
) -> Result<Term, Diagnostic> {
    assert!(
        !term.is_lambda(),
        "loose DB unbinding expects lambda prefixes to be removed"
    );
    if !term.has_db_subterm() {
        return Ok(term.clone());
    }
    if term.is_db_var() {
        if term.f_code() < depth {
            return Ok(term.clone());
        }
        let entry = replacements.entry(term.f_code()).or_insert_with(|| {
            let type_ = term.type_().expect("loose DB variable must have a type");
            let fresh_var = bank.vars().get_fresh_var(&type_);
            let db_var = bank.request_db_var(&type_, term.f_code() - depth);
            LooseDbReplacement { fresh_var, db_var }
        });
        return Ok(entry.fresh_var.clone());
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let unbound = unbind_loose_db_vars(bank, depth, &arg, replacements)?;
        if unbound != arg {
            changed = true;
        }
        copy.set_argument(index, unbound);
    }
    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

fn bind_loose_db_replacements(replacements: &BTreeMap<FunCode, LooseDbReplacement>) {
    for replacement in replacements.values() {
        assert!(
            replacement.fresh_var.binding().is_none(),
            "fresh loose-DB replacement variable must be unbound"
        );
        replacement
            .fresh_var
            .set_binding(Some(replacement.db_var.clone()));
    }
}

fn clear_loose_db_replacements(replacements: &BTreeMap<FunCode, LooseDbReplacement>) {
    for replacement in replacements.values() {
        replacement.fresh_var.set_binding(None);
    }
}

fn lift_lambdas_in_term(
    bank: &mut TermBank,
    term: &Term,
    state: &mut LambdaLiftState,
    used_defs: &mut Vec<WrappedFormula>,
) -> Result<Term, Diagnostic> {
    let mut normalized = beta_normalize_db(bank, term)?;
    let mut bound_vars = Vec::new();
    let had_lambda_prefix = normalized.is_lambda();
    if had_lambda_prefix {
        normalized = unfold_lambda(&normalized, &mut bound_vars);
    }

    let lifted_body = if normalized.has_lambda_subterm() {
        let copy = Term::top_copy_without_args(&normalized);
        let mut changed = false;
        for (index, arg) in normalized.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let lifted = lift_lambdas_in_term(bank, &arg, state, used_defs)?;
            if lifted != arg {
                changed = true;
            }
            copy.set_argument(index, lifted);
        }
        if changed {
            bank.term_top_insert(copy)?
        } else {
            normalized.clone()
        }
    } else {
        normalized.clone()
    };

    if had_lambda_prefix {
        lift_lambda_prefix(bank, &bound_vars, &lifted_body, state, used_defs)
    } else {
        Ok(lifted_body)
    }
}

fn lift_lambda_prefix(
    bank: &mut TermBank,
    bound_vars: &[Term],
    body: &Term,
    state: &mut LambdaLiftState,
    used_defs: &mut Vec<WrappedFormula>,
) -> Result<Term, Diagnostic> {
    assert!(
        !body.has_lambda_subterm(),
        "lambda lifting expects nested lambdas to be lifted first"
    );
    let free_vars = tformula_collect_free_vars(bank, body);
    let bound_to_fresh = bound_vars
        .iter()
        .map(|variable| {
            let type_ = variable.type_().expect("lambda binder must have a type");
            bank.vars().get_fresh_var(&type_)
        })
        .collect::<Vec<_>>();

    let mut loose_replacements = BTreeMap::new();
    let body_no_loose = unbind_loose_db_vars(
        bank,
        FunCode::try_from(bound_vars.len()).expect("lambda prefix length fits FunCode"),
        body,
        &mut loose_replacements,
    )?;

    let mut closed = body_no_loose;
    for variable in bound_vars.iter().rev() {
        let type_ = variable.type_().expect("lambda binder must have a type");
        closed = crate::terms::lambda::close_with_db_var(bank, &type_, &closed)?;
    }
    bind_loose_db_replacements(&loose_replacements);
    let lifting_key = term_identity_id(&closed);
    if let Some(entry) = state.exact_liftings.get(&lifting_key).cloned() {
        clear_loose_db_replacements(&loose_replacements);
        used_defs.push(entry.definition);
        return Ok(entry.lifted);
    }
    if let Some(entry) = state.find_generalization(bank, &closed)? {
        clear_loose_db_replacements(&loose_replacements);
        used_defs.push(entry.definition);
        return Ok(entry.lifted);
    }
    clear_loose_db_replacements(&loose_replacements);

    let loose_fresh_vars = loose_replacements
        .values()
        .map(|replacement| replacement.fresh_var.clone())
        .collect::<Vec<_>>();
    let loose_db_vars = loose_replacements
        .values()
        .map(|replacement| replacement.db_var.clone())
        .collect::<Vec<_>>();
    let loose_db_types = loose_db_vars
        .iter()
        .map(|variable| {
            variable
                .type_()
                .expect("loose DB variable must have a type")
        })
        .collect::<Vec<_>>();
    let closed_type = closed.type_().expect("closed lambda body must have a type");
    let result_type = bank
        .signature_mut()
        .type_bank_mut()
        .insert_type_shared(arrow_type_flattened(&loose_db_types, &closed_type));

    let def_head = bank.alloc_new_skolem(&free_vars, Some(&result_type))?;
    let lifted = apply_terms(bank, &def_head, &loose_db_vars)?;
    let lhs_wo_bound = apply_terms(bank, &def_head, &loose_fresh_vars)?;
    let repl_lhs = apply_terms(bank, &lhs_wo_bound, &bound_to_fresh)?;
    let applied_rhs = apply_terms(bank, &closed, &bound_to_fresh)?;
    let repl_rhs = whnf_step(bank, &applied_rhs)?;

    let mut definition = if body
        .type_()
        .as_ref()
        .is_some_and(crate::terms::simpletypes::Type::is_bool)
    {
        let left = tformula_encode_predicate_as_eqn(bank, repl_lhs.clone())?;
        let right = tformula_encode_predicate_as_eqn(bank, repl_rhs)?;
        tformula_fcode_alloc(bank, bank.signature().equiv_code(), left, Some(right))?
    } else {
        tformula_fcode_alloc(
            bank,
            bank.signature().eqn_code(),
            repl_lhs.clone(),
            Some(repl_rhs),
        )?
    };
    for argument in repl_lhs.argument_clones().into_iter().flatten() {
        definition = tformula_add_quantor(bank, &definition, true, &argument)?;
    }

    let mut wrapped = WrappedFormula::wt_formula_alloc(definition);
    wrapped.push_formula_derivation(DC_INTRO_DEF, None, None);
    state.definitions.push(wrapped.clone());
    used_defs.push(wrapped.clone());
    state.exact_liftings.insert(
        lifting_key,
        LambdaLiftReuseEntry {
            lifted: lifted.clone(),
            definition: wrapped.clone(),
        },
    );
    state.general_liftings.push(LambdaLiftGeneralizationEntry {
        closed,
        lifted_template: lhs_wo_bound,
        definition: wrapped,
    });
    Ok(lifted)
}

fn cond_lift_clause_lambda(
    bank: &mut TermBank,
    term: &Term,
    state: &mut LambdaLiftState,
    used_defs: &mut Vec<WrappedFormula>,
) -> Result<Term, Diagnostic> {
    if term.is_lambda() || !term.has_lambda_subterm() {
        return Ok(term.clone());
    }
    let decoded = decode_formulas_for_cnf(bank, term)?;
    lift_lambdas_in_term(bank, &decoded, state, used_defs)
}

fn clause_set_lift_lambdas(
    set: &mut ClauseSet,
    archive: &mut FormulaSet,
    bank: &mut TermBank,
    fresh_vars: &VarBank,
    unroll_fool: bool,
) -> Result<ClauseSetLiftLambdasResult, Diagnostic> {
    let mut result = ClauseSetLiftLambdasResult::default();
    let mut state = LambdaLiftState::default();
    let mut all_defs = BTreeMap::new();

    bank.vars().set_v_counts_to_used();
    for clause in set.iter_mut() {
        let mut clause_changed = false;
        let mut clause_defs = Vec::new();
        for literal in clause.literals_mut().as_mut_slice() {
            let left = cond_lift_clause_lambda(bank, literal.left(), &mut state, &mut clause_defs)?;
            let right =
                cond_lift_clause_lambda(bank, literal.right(), &mut state, &mut clause_defs)?;
            if left != *literal.left() || right != *literal.right() {
                clause_changed = true;
                literal.set_left_raw(left);
                literal.set_right_raw(right);
                literal.del_prop(EP_MAX_IS_UP_TO_DATE | EP_IS_ORIENTED);
            }
        }

        if clause_changed {
            result.clauses_changed += 1;
            for definition in clause_defs {
                clause_push_formula_derivation(
                    clause,
                    DC_LIFT_LAMBDAS,
                    Some(FormulaDerivationRef::new(definition.ident())),
                    None,
                );
                result.clause_derivation_ops.push(DC_LIFT_LAMBDAS);
                all_defs.entry(definition.entry_id()).or_insert(definition);
            }
            clause.set_weight(clause.standard_weight());
        }
    }

    for definition in all_defs.into_values() {
        let mut copy = definition.flat_copy();
        archive.insert(definition);
        result.definitions_archived += 1;
        if unroll_fool {
            let _changed = copy.unroll_fool(bank)?;
        }
        let _changed = copy.simplify(bank)?;
        let cnf_result = copy.cnf2_into(
            bank,
            set,
            fresh_vars,
            100,
            unroll_fool,
            ProblemType::HigherOrder,
        )?;
        result.definition_clauses_generated += cnf_result.clauses_generated;
        archive.insert(copy);
    }

    Ok(result)
}

fn apply_post_cnf_clause_lambda_lifting(
    clauseset: &mut ClauseSet,
    archive: &mut FormulaSet,
    bank: &mut TermBank,
    fresh_vars: &VarBank,
    fool_unroll: bool,
    result: &mut FormulaSetCnfResult,
) -> Result<(), Diagnostic> {
    let lift_result = clause_set_lift_lambdas(clauseset, archive, bank, fresh_vars, fool_unroll)?;
    result.clauses_lambdas_lifted = lift_result.clauses_changed;
    result.lambda_lift_definitions_archived = lift_result.definitions_archived;
    result.lambda_lift_definition_clauses_generated = lift_result.definition_clauses_generated;
    Ok(())
}

struct FormulaSetCnfDrain<'a> {
    archive: &'a mut FormulaSet,
    clauseset: &'a mut ClauseSet,
    bank: &'a mut TermBank,
    fresh_vars: &'a VarBank,
    options: FormulaSetCnfOptions,
    old_nodes: &'a mut i64,
    gc_threshold: &'a mut i64,
    result: &'a mut FormulaSetCnfResult,
}

struct FormulaSetCnfDocDrain<'a, W: fmt::Write> {
    archive: &'a mut FormulaSet,
    clauseset: &'a mut ClauseSet,
    bank: &'a mut TermBank,
    fresh_vars: &'a VarBank,
    options: FormulaSetCnfOptions,
    old_nodes: &'a mut i64,
    gc_threshold: &'a mut i64,
    output: &'a mut W,
    session: &'a mut ProofDocSession,
    render_options: FormulaProofDocRenderOptions,
    result: &'a mut FormulaSetCnfDocResult,
}

fn drain_formula_set_to_cnf(
    set: &mut FormulaSet,
    drain: &mut FormulaSetCnfDrain<'_>,
) -> Result<(), Diagnostic> {
    while let Some(handle) = set.extract_first() {
        let source = FormulaDerivationRef::new(handle.ident());
        let mut form = handle.flat_copy();
        drain.archive.insert(handle);
        drain.result.original_formulas_archived += 1;
        drain.result.quoted_formula_sources.push(source);

        let cnf_result = form.cnf2_into(
            drain.bank,
            drain.clauseset,
            drain.fresh_vars,
            drain.options.miniscope_limit,
            drain.options.fool_unroll,
            drain.options.problem_type,
        )?;
        drain.result.clauses_generated += cnf_result.clauses_generated;
        drain
            .result
            .formula_derivation_ops
            .extend(cnf_result.formula_derivation_ops);

        let cnf_copy_has_formula = form.formula.is_some();
        drain.archive.insert(form);
        drain.result.cnf_formulas_archived += 1;
        if cnf_copy_has_formula && drain.bank.non_var_term_nodes() > *drain.gc_threshold {
            collect_formula_set_cnf_garbage(
                drain.bank,
                set,
                drain.archive,
                drain.clauseset,
                drain.result,
            );
            *drain.old_nodes = drain.bank.non_var_term_nodes();
            *drain.gc_threshold = formula_set_gc_threshold(*drain.old_nodes);
        }
    }

    Ok(())
}

fn drain_formula_set_to_cnf_with_docs<W: fmt::Write>(
    set: &mut FormulaSet,
    drain: &mut FormulaSetCnfDocDrain<'_, W>,
) -> Result<(), Diagnostic> {
    while let Some(handle) = set.extract_first() {
        let source = FormulaDerivationRef::new(handle.ident());
        let mut form = handle.flat_copy();
        drain.archive.insert(handle);
        drain.result.cnf.original_formulas_archived += 1;
        drain.result.cnf.quoted_formula_sources.push(source);

        let cnf_result = {
            let mut doc_context = WrappedFormulaCnfDocContext::new(
                &mut *drain.output,
                &mut *drain.session,
                drain.render_options,
            );
            form.cnf2_into_with_docs(
                &mut doc_context,
                drain.bank,
                drain.clauseset,
                drain.fresh_vars,
                drain.options.miniscope_limit,
                drain.options.fool_unroll,
            )?
        };
        drain.result.add_wrapped_cnf_doc(cnf_result);

        let cnf_copy_has_formula = form.formula.is_some();
        drain.archive.insert(form);
        drain.result.cnf.cnf_formulas_archived += 1;
        if cnf_copy_has_formula && drain.bank.non_var_term_nodes() > *drain.gc_threshold {
            collect_formula_set_cnf_garbage(
                drain.bank,
                set,
                drain.archive,
                drain.clauseset,
                &mut drain.result.cnf,
            );
            *drain.old_nodes = drain.bank.non_var_term_nodes();
            *drain.gc_threshold = formula_set_gc_threshold(*drain.old_nodes);
        }
    }

    Ok(())
}

fn collect_formula_set_cnf_garbage(
    bank: &mut TermBank,
    set: &FormulaSet,
    archive: &FormulaSet,
    clauseset: &ClauseSet,
    result: &mut FormulaSetCnfResult,
) {
    let recovered = tb_gc_collect(bank, &[clauseset], &[set, archive]);
    result.term_garbage_collections += 1;
    result.terms_recovered_by_gc += recovered;
}

fn collect_formula_set_simplify_garbage(
    bank: &mut TermBank,
    set: &FormulaSet,
    result: &mut FormulaSetSimplifyResult,
) {
    let clause_sets: [&ClauseSet; 0] = [];
    let recovered = tb_gc_collect(bank, &clause_sets, &[set]);
    result.term_garbage_collections += 1;
    result.terms_recovered_by_gc += recovered;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetSimplifyResult {
    pub formulas_changed: i64,
    pub term_garbage_collections: i64,
    pub terms_recovered_by_gc: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetSimplifyDocResult {
    pub simplify: FormulaSetSimplifyResult,
    pub write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetPreprocessConjecturesResult {
    pub conjectures_negated: i64,
    pub questions_annotated: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetPreprocessConjecturesDocResult {
    pub preprocess: FormulaSetPreprocessConjecturesResult,
    pub write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetFoolUnrollResult {
    pub boolean_equalities_replaced: i64,
    pub formulas_unrolled: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetIntroduceDefsResult {
    pub definitions_introduced: i64,
    pub archived_definitions: i64,
    pub active_definitions_inserted: i64,
    pub formulas_rewritten: i64,
    pub definition_applications: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetIntroduceDefsDocResult {
    pub introduce: FormulaSetIntroduceDefsResult,
    pub definition_write_results: Vec<ProofDocWriteResult>,
    pub application_write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetHigherOrderPreprocessResult {
    pub formulas_named_to_db: i64,
    pub formulas_ites_lifted: i64,
    pub formulas_lets_lifted: i64,
    pub formulas_lambdas_lifted: i64,
    pub formulas_def_symbols_unfolded: i64,
    pub unfolded_definition_rhs_rewritten: i64,
    pub unfolded_definitions_archived: i64,
    pub unfolded_original_definitions_archived: i64,
    pub definition_symbol_applications: i64,
    pub lambda_lift_definitions_inserted: i64,
    pub formulas_lambda_normalized: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetHigherOrderPreprocessDocResult {
    pub preprocess: FormulaSetHigherOrderPreprocessResult,
    pub write_results: Vec<ProofDocWriteResult>,
}

struct DefinitionSymbolMap {
    definitions: BTreeMap<FunCode, WrappedFormula>,
    recognized_entry_ids: Vec<u64>,
}

struct FormulaProofDocContext<'a, W: fmt::Write> {
    output: &'a mut W,
    session: &'a mut ProofDocSession,
    render_options: FormulaProofDocRenderOptions,
}

fn doc_introduced_definition<W: fmt::Write>(
    bank: &mut TermBank,
    formula: &mut WrappedFormula,
    doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
    result: &mut FormulaSetIntroduceDefsDocResult,
) -> Result<(), Diagnostic> {
    if let Some(context) = doc_context.as_mut() {
        let (write_result, new_ident, new_properties) = {
            let rendered = formula.proof_doc_formula_body_string(
                bank,
                context.render_options.full_terms,
                context.render_options.problem_type,
            )?;
            let mut view = formula.proof_doc_view(&rendered);
            let write_result = context.session.doc_formula_creation(
                context.output,
                &mut view,
                FormulaCreationInference::IntroDef,
                FormulaCreationParents::none(),
                None,
            )?;
            (write_result, view.ident(), view.properties())
        };
        formula.ident = new_ident;
        formula.set_properties(new_properties);
        result.definition_write_results.push(write_result);
    }
    Ok(())
}

fn doc_split_equiv_definition<W: fmt::Write>(
    bank: &mut TermBank,
    neutral_formula: &WrappedFormula,
    split_formula: &mut WrappedFormula,
    doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
    result: &mut FormulaSetIntroduceDefsDocResult,
) -> Result<(), Diagnostic> {
    if let Some(context) = doc_context.as_mut() {
        let (write_result, new_ident, new_properties) = {
            let parent_rendered = neutral_formula.proof_doc_formula_body_string(
                bank,
                context.render_options.full_terms,
                context.render_options.problem_type,
            )?;
            let parent_view = neutral_formula.proof_doc_view(&parent_rendered);
            let rendered = split_formula.proof_doc_formula_body_string(
                bank,
                context.render_options.full_terms,
                context.render_options.problem_type,
            )?;
            let mut view = split_formula.proof_doc_view(&rendered);
            let write_result = context.session.doc_formula_creation(
                context.output,
                &mut view,
                FormulaCreationInference::SplitEquiv,
                FormulaCreationParents::unary(&parent_view),
                None,
            )?;
            (write_result, view.ident(), view.properties())
        };
        split_formula.ident = new_ident;
        split_formula.set_properties(new_properties);
        result.definition_write_results.push(write_result);
    }
    Ok(())
}

fn doc_applied_definitions<W: fmt::Write>(
    bank: &mut TermBank,
    formula: &mut WrappedFormula,
    defs_used: &[FormulaDerivationRef],
    doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
    result: &mut FormulaSetIntroduceDefsDocResult,
) -> Result<(), Diagnostic> {
    if let Some(context) = doc_context.as_mut() {
        let (write_result, new_ident, new_properties) = {
            let def_ids = defs_used
                .iter()
                .map(|definition| definition.ident())
                .collect::<Vec<_>>();
            let rendered = formula.proof_doc_formula_body_string(
                bank,
                context.render_options.full_terms,
                context.render_options.problem_type,
            )?;
            let mut view = formula.proof_doc_view(&rendered);
            let write_result = context.session.doc_formula_intro_defs(
                context.output,
                &mut view,
                &def_ids,
                None,
            )?;
            (write_result, view.ident(), view.properties())
        };
        formula.ident = new_ident;
        formula.set_properties(new_properties);
        result.application_write_results.push(write_result);
    }
    Ok(())
}

fn intersimplify_definition_symbols(
    bank: &mut TermBank,
    definitions: &mut BTreeMap<FunCode, WrappedFormula>,
    result: &mut FormulaSetHigherOrderPreprocessDocResult,
) -> Result<(), Diagnostic> {
    let definition_codes = definitions.keys().copied().collect::<Vec<_>>();
    for definition_code in definition_codes {
        let (lhs, rhs) = {
            let definition = definitions
                .get(&definition_code)
                .expect("definition code disappeared");
            (
                definition
                    .formula()
                    .argument(0)
                    .expect("generated definition left side is uninitialized"),
                definition
                    .formula()
                    .argument(1)
                    .expect("generated definition right side is uninitialized"),
            )
        };
        let mut used_defs = BTreeSet::new();
        let mut max_steps = MAX_DEF_SYMBOL_REWRITE_STEPS;
        let new_rhs =
            do_rewrite_with_def_symbols(bank, &rhs, definitions, &mut used_defs, &mut max_steps)?;
        if new_rhs != rhs {
            let eqn_code = bank.signature().eqn_code();
            let new_definition = tformula_fcode_alloc(bank, eqn_code, lhs, Some(new_rhs))?;
            let parents = definition_parent_refs(definitions, &used_defs);
            let definition = definitions
                .get_mut(&definition_code)
                .expect("definition code disappeared");
            definition.set_formula(new_definition);
            for parent in parents {
                definition.push_formula_derivation(DC_APPLY_DEF, Some(parent), None);
            }
            result.preprocess.unfolded_definition_rhs_rewritten += 1;
            result.preprocess.definition_symbol_applications += usize_to_i64(used_defs.len());
            result
                .preprocess
                .formula_derivation_ops
                .extend(std::iter::repeat_n(DC_APPLY_DEF, used_defs.len()));
        }
    }
    Ok(())
}

fn rewrite_formulas_with_def_symbols<W: fmt::Write>(
    formulas: &mut [WrappedFormula],
    bank: &mut TermBank,
    definitions: &BTreeMap<FunCode, WrappedFormula>,
    recognized_entry_ids: &[u64],
    doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
    result: &mut FormulaSetHigherOrderPreprocessDocResult,
) -> Result<(), Diagnostic> {
    let recognized = recognized_entry_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for formula in formulas {
        if recognized.contains(&formula.entry_id()) {
            continue;
        }
        let mut used_defs = BTreeSet::new();
        let mut max_steps = MAX_DEF_SYMBOL_REWRITE_STEPS;
        let rewritten = do_rewrite_with_def_symbols(
            bank,
            formula.formula(),
            definitions,
            &mut used_defs,
            &mut max_steps,
        )?;
        if rewritten != *formula.formula() {
            let rewritten = unencode_eqns(bank, &rewritten)?;
            formula.set_formula(rewritten);
            if let Some(context) = doc_context.as_mut() {
                let (write_result, new_ident, new_properties) = {
                    let rendered = formula.proof_doc_formula_body_string(
                        bank,
                        context.render_options.full_terms,
                        context.render_options.problem_type,
                    )?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = context.session.doc_formula_modification(
                        context.output,
                        &mut view,
                        FormulaModificationInference::Simplification,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                formula.ident = new_ident;
                formula.set_properties(new_properties);
                result.write_results.push(write_result);
            }
            for parent in definition_parent_refs(definitions, &used_defs) {
                formula.push_formula_derivation(DC_APPLY_DEF, Some(parent), None);
            }
            result.preprocess.formulas_def_symbols_unfolded += 1;
            result.preprocess.definition_symbol_applications += usize_to_i64(used_defs.len());
            result
                .preprocess
                .formula_derivation_ops
                .extend(std::iter::repeat_n(DC_APPLY_DEF, used_defs.len()));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClauseSetLiftLambdasResult {
    pub clauses_changed: i64,
    pub definitions_archived: i64,
    pub definition_clauses_generated: i64,
    pub clause_derivation_ops: Vec<i64>,
}

#[derive(Default)]
struct LambdaLiftState {
    definitions: Vec<WrappedFormula>,
    exact_liftings: BTreeMap<usize, LambdaLiftReuseEntry>,
    general_liftings: Vec<LambdaLiftGeneralizationEntry>,
}

#[derive(Clone)]
struct LambdaLiftReuseEntry {
    lifted: Term,
    definition: WrappedFormula,
}

#[derive(Clone)]
struct LambdaLiftGeneralizationEntry {
    closed: Term,
    lifted_template: Term,
    definition: WrappedFormula,
}

impl LambdaLiftState {
    fn find_generalization(
        &self,
        bank: &mut TermBank,
        query: &Term,
    ) -> Result<Option<LambdaLiftReuseEntry>, Diagnostic> {
        let mut subst = Substitution::new();
        for entry in &self.general_liftings {
            let subst_start = subst.len();
            if !subst_compute_match(&entry.closed, query, &mut subst) {
                continue;
            }

            let matcher_derefed = match bank.insert_instantiated_ho(&entry.lifted_template, true) {
                Ok(term) => term,
                Err(err) => {
                    subst.backtrack_to_pos(subst_start);
                    return Err(err);
                }
            };
            let matched_vars = subst.bindings()[subst_start..].to_vec();
            let saved_bindings = matched_vars.iter().map(Term::binding).collect::<Vec<_>>();
            for var in &matched_vars {
                var.set_binding(None);
            }

            let candidate = (|| {
                let matcher_derefed = bank.insert_instantiated_ho(&matcher_derefed, true)?;
                let beta_normal = beta_normalize_db(bank, &matcher_derefed)?;
                lambda_eta_reduce_db(bank, &beta_normal)
            })();

            for (var, binding) in matched_vars.iter().zip(saved_bindings) {
                var.set_binding(binding);
            }
            subst.backtrack_to_pos(subst_start);

            let candidate = candidate?;
            if !candidate.has_lambda_subterm() {
                return Ok(Some(LambdaLiftReuseEntry {
                    lifted: candidate,
                    definition: entry.definition.clone(),
                }));
            }
        }
        Ok(None)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetArchiveResult {
    pub formulas_archived: i64,
    pub quoted_formula_sources: Vec<FormulaDerivationRef>,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetDocInitialResult {
    pub formulas_seen: i64,
    pub write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedFormula {
    entry_id: u64,
    properties: FormulaProperties,
    is_clause: bool,
    ident: i64,
    info: Option<ClauseInfo>,
    derivation: Option<PStack<DerivationEntry>>,
    formula: Option<Term>,
}

#[must_use]
pub fn wformula_dummy_quote_parent_ref(formula: &WrappedFormula) -> Option<FormulaDerivationRef> {
    formula_dummy_quote_parent_ref(formula.derivation())
}

#[must_use]
pub fn wformula_deriv_find_first<'a>(
    formula: &'a WrappedFormula,
    mut resolve_parent: impl FnMut(FormulaDerivationRef) -> Option<&'a WrappedFormula>,
) -> &'a WrappedFormula {
    let mut current = formula;
    let mut visited = Vec::new();

    while let Some(parent_ref) = wformula_dummy_quote_parent_ref(current) {
        let key = std::ptr::from_ref(current);
        if visited.contains(&key) {
            break;
        }
        visited.push(key);

        let Some(parent) = resolve_parent(parent_ref) else {
            break;
        };
        if std::ptr::eq(parent, current) {
            break;
        }
        current = parent;
    }

    current
}

impl WrappedFormula {
    #[must_use]
    pub fn default_alloc() -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: CP_IGNORE_PROPS,
            is_clause: false,
            ident: 0,
            info: None,
            derivation: None,
            formula: None,
        }
    }

    #[must_use]
    pub fn wt_formula_alloc(formula: Term) -> Self {
        Self {
            formula: Some(formula),
            ident: next_formula_ident(),
            ..Self::default_alloc()
        }
    }

    #[must_use]
    pub fn flat_copy(&self) -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: self.properties,
            is_clause: self.is_clause,
            ident: self.ident,
            info: None,
            derivation: None,
            formula: self.formula.clone(),
        }
    }

    #[must_use]
    pub const fn entry_id(&self) -> u64 {
        self.entry_id
    }

    #[must_use]
    pub const fn properties(&self) -> FormulaProperties {
        self.properties
    }

    pub fn set_properties(&mut self, properties: FormulaProperties) {
        self.properties = properties;
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        self.properties.delete(prop);
    }

    #[must_use]
    pub const fn query_prop(&self, prop: FormulaProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn give_props(&self, prop: FormulaProperties) -> FormulaProperties {
        self.properties.give(prop)
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        self.properties.set_tptp_type(type_);
    }

    #[must_use]
    pub const fn query_tptp_type(&self) -> FormulaProperties {
        self.properties.query_tptp_type()
    }

    #[must_use]
    pub const fn is_clause(&self) -> bool {
        self.is_clause
    }

    pub fn set_is_clause(&mut self, is_clause: bool) {
        self.is_clause = is_clause;
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    pub fn set_ident(&mut self, ident: i64) {
        self.ident = ident;
    }

    #[must_use]
    pub const fn info(&self) -> Option<&ClauseInfo> {
        self.info.as_ref()
    }

    pub fn set_info(&mut self, info: Option<ClauseInfo>) {
        self.info = info;
    }

    #[must_use]
    pub const fn derivation(&self) -> Option<&PStack<DerivationEntry>> {
        self.derivation.as_ref()
    }

    pub fn ensure_derivation(&mut self) -> &mut PStack<DerivationEntry> {
        self.derivation.get_or_insert_with(PStack::new)
    }

    pub fn set_derivation(&mut self, derivation: Option<PStack<DerivationEntry>>) {
        self.derivation = derivation;
    }

    pub fn take_derivation(&mut self) -> Option<PStack<DerivationEntry>> {
        self.derivation.take()
    }

    #[must_use]
    pub fn derivation_entries(&self) -> &[DerivationEntry] {
        self.derivation.as_ref().map_or(&[], PStack::as_slice)
    }

    pub fn push_formula_derivation(
        &mut self,
        op: i64,
        arg1: Option<FormulaDerivationRef>,
        arg2: Option<FormulaDerivationRef>,
    ) {
        let stack = self.ensure_derivation();
        push_formula_derivation_stack(stack, op, arg1, arg2);
    }

    /// Returns the wrapped formula term.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper was allocated without a formula term.
    #[must_use]
    pub fn formula(&self) -> &Term {
        self.formula
            .as_ref()
            .unwrap_or_else(|| panic!("wrapped formula has no term formula"))
    }

    pub fn set_formula(&mut self, formula: Term) {
        self.formula = Some(formula);
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            bank.gc_mark_term(formula);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            tformula_mark_polarity(bank, formula, 1);
        }
    }

    /// Applies C `WFormulaSimplify` to this wrapped formula.
    ///
    /// C calls `TFormulaSimplify` with a quantifier-optimization limit of zero.
    /// The staged Rust wrapper updates the formula term and reports whether it
    /// changed while pushing `DCFofSimplify` onto the formula derivation stack.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if simplifying or rebuilding the term-encoded
    /// formula fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if the formula is
    /// malformed.
    pub fn simplify(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let original = self.formula().clone();
        let simplified = tformula_simplify(bank, &original, 0)?;
        if simplified == original {
            return Ok(false);
        }
        self.set_formula(simplified);
        self.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        Ok(true)
    }

    /// Applies C `WFormulaConjectureNegate`.
    ///
    /// If the wrapper role is `conjecture`, this wraps the formula in a root
    /// negation and changes the role to `negated_conjecture`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the negation term cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn conjecture_negate(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        if self.query_tptp_type() != CP_TYPE_CONJECTURE {
            return Ok(false);
        }
        let negated = tformula_fcode_alloc(
            bank,
            bank.signature().not_code(),
            self.formula().clone(),
            None,
        )?;
        self.set_formula(negated);
        self.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        self.push_formula_derivation(DC_NEGATE_CONJECTURE, None, None);
        Ok(true)
    }

    /// Applies C `WFormulaAnnotateQuestion`.
    ///
    /// Questions, and optionally conjectures, become conjectures. When
    /// `add_answer_lits` is true, leading existential quantifiers receive the
    /// C-shaped `~$answer(esk(...))` conjunct.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if an answer literal or rebuilt formula cannot be
    /// allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a leading existential
    /// has uninitialized arguments.
    pub fn annotate_question(
        &mut self,
        bank: &mut TermBank,
        add_answer_lits: bool,
        conjectures_are_questions: bool,
    ) -> Result<bool, Diagnostic> {
        let role = self.query_tptp_type();
        if role != CP_TYPE_QUESTION && !(role == CP_TYPE_CONJECTURE && conjectures_are_questions) {
            return Ok(false);
        }
        if add_answer_lits {
            let annotated = tformula_annotate_question(bank, self.formula())?;
            self.set_formula(annotated);
        }
        self.set_tptp_type(CP_TYPE_CONJECTURE);
        self.push_formula_derivation(DC_ANNO_QUESTION, None, None);
        Ok(true)
    }

    /// Applies C `WFormulaReplaceEqnWithEquiv`.
    ///
    /// Complex Boolean equalities become equivalence formulas, complex Boolean
    /// disequalities become XOR formulas, and comparisons against `$true`
    /// collapse to the Boolean side or its negation.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a rebuilt formula cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a formula cell has
    /// uninitialized arguments.
    pub fn replace_eqn_with_equiv(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let replaced = tformula_replace_eqn_with_equiv(bank, self.formula())?;
        if replaced == *self.formula() {
            return Ok(false);
        }
        self.set_formula(replaced);
        self.push_formula_derivation(DC_EQ_TO_EQ, None, None);
        Ok(true)
    }

    /// Applies C `TFormulaUnrollFOOL` to this wrapper.
    ///
    /// Literal expansion always updates the wrapped formula when it changes.
    /// The returned flag reports only whether the FOOL unrolling mapper changed
    /// the expanded formula, matching C's `TFormulaUnrollFOOL` return value and
    /// `DCFoolUnroll` derivation condition.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal expansion, lambda eta-reduction, term
    /// replacement, or formula allocation fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, a located FOOL subterm is not
    /// Boolean, or formula cells are malformed.
    pub fn unroll_fool(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let result = tformula_unroll_fool_result(bank, self.formula())?;
        if result.formula() != self.formula() {
            self.set_formula(result.formula().clone());
        }
        if result.fool_unrolled() {
            self.push_formula_derivation(DC_FOOL_UNROLL, None, None);
        }
        Ok(result.fool_unrolled())
    }

    /// Applies C `NamedToDB` as used by `TFormulaSetNamedToDBLambdas`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if converting or beta-normalizing named lambdas
    /// cannot rebuild a shared term.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a named lambda cell is
    /// malformed.
    pub fn named_to_db_lambdas(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let original = self.formula().clone();
        if !term_has_named_lambda(&original) {
            return Ok(false);
        }
        let converted = named_to_db(bank, &original)?;
        if converted == original {
            return Ok(false);
        }
        self.set_formula(converted);
        self.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        Ok(true)
    }

    /// Applies C `do_ite_unroll` as used by `TFormulaSetLiftItes`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if ITE expansion or copied term insertion fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if an `$ite`, literal, or
    /// formula cell is malformed.
    pub fn lift_ites(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let original = self.formula().clone();
        let lifted = tformula_lift_ite(bank, &original)?;
        if lifted == original {
            return Ok(false);
        }
        self.set_formula(lifted);
        self.push_formula_derivation(DC_LIFT_ITE, None, None);
        Ok(true)
    }

    /// Applies C `lift_lets` as used by `TFormulaSetLiftLets`.
    ///
    /// The returned formulas are the generated global definitions that the set
    /// owner must wrap and append after the traversal.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if variable renaming, LET lifting, predicate
    /// encoding, closure, instantiation, app flattening, or term-bank insertion
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a `$let` cell,
    /// definition equality, or definition head is malformed.
    pub fn lift_lets(&mut self, bank: &mut TermBank) -> Result<Vec<Term>, Diagnostic> {
        let original = self.formula().clone();
        bank.vars().set_v_counts_to_used();
        let renamed = tformula_var_rename(bank, &original)?;
        let lifted = tformula_lift_lets(bank, &renamed)?;
        if lifted.definitions.is_empty() {
            return Ok(Vec::new());
        }
        self.set_formula(tformula_unencode_root_eqn(bank, &lifted.formula));
        Ok(lifted.definitions)
    }

    /// Applies C `TFormulaSetLambdaNormalize` to this wrapper's formula term.
    ///
    /// C beta-normalizes DB lambdas and then turns lambda equalities into
    /// quantified formulas through `LambdaToForall`; it records
    /// `DCFofSimplify` when the final formula differs from the original.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if beta normalization, lambda-to-forall conversion,
    /// or formula rebuilding fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if lambda/formula cells are
    /// malformed.
    pub fn lambda_normalize_forall(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let original = self.formula().clone();
        let beta_normal = beta_normalize_db(bank, &original)?;
        let normalized = lambda_to_forall(bank, &beta_normal)?;
        if normalized == original {
            return Ok(false);
        }
        self.set_formula(normalized);
        self.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        Ok(true)
    }

    /// Applies C `TFormulaApplyDefs` to this wrapper.
    ///
    /// Definition parents are reported as the archived neutral-definition
    /// formula refs.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if rebuilding the formula fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, a marked subformula has no
    /// definition entry, or definition metadata has not been populated.
    pub fn apply_defs(
        &mut self,
        bank: &mut TermBank,
        defs: &TFormulaDefinitions,
    ) -> Result<Vec<FormulaDerivationRef>, Diagnostic> {
        let mut defs_used = Vec::new();
        let reduced = tformula_copy_def(bank, self.formula(), self.ident, defs, &mut defs_used)?;
        if !defs_used.is_empty() {
            self.set_formula(reduced);
        }
        Ok(defs_used)
    }

    #[must_use]
    pub fn get_id(&self, keep_input_names: bool) -> String {
        if keep_input_names {
            if let Some(name) = self.info.as_ref().and_then(ClauseInfo::name) {
                return name.to_owned();
            }
        }
        if self.ident < 0 {
            format!("i_0_{}", i128::from(self.ident) - i128::from(i64::MIN))
        } else {
            format!("c_0_{}", self.ident)
        }
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        term_is_untyped(self.formula())
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        term_has_interpreted_symbol(self.formula())
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        term_standard_weight(self.formula())
    }

    #[must_use]
    pub fn is_hypothesis(&self) -> bool {
        self.properties.is_hypothesis()
    }

    #[must_use]
    pub fn is_conjecture(&self) -> bool {
        self.properties.is_conjecture()
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        term_compute_order(signature, self.formula())
    }

    #[must_use]
    pub fn contains_f_code(&self, f_code: FunCode) -> bool {
        term_has_f_code(self.formula(), f_code)
    }

    /// Pushes each distinct non-variable, non-phony-app f-code in traversal order.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if the formula is not
    /// shared by its term bank.
    pub fn return_f_codes(&self, f_codes: &mut Vec<FunCode>) -> i64 {
        let start = f_codes.len();
        let mut subterms = PStack::new();
        let _ = tb_term_collect_subterms(self.formula(), &mut subterms);
        let mut seen = BTreeSet::new();
        for term in subterms.as_slice() {
            term.del_prop(TP_OP_FLAG);
            if !term.is_any_var() && !term.is_phony_app() && seen.insert(term.f_code()) {
                f_codes.push(term.f_code());
            }
        }
        usize_to_i64(f_codes.len() - start)
    }

    /// Returns the count of distinct non-variable, non-phony-app f-codes.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::return_f_codes`].
    #[must_use]
    pub fn symbol_diversity(&self) -> i64 {
        let mut f_codes = Vec::new();
        self.return_f_codes(&mut f_codes)
    }

    /// Extracts the defined symbol from C's lambda-definition formula shapes.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper is tagged as a lambda definition but has no
    /// formula term.
    #[must_use]
    pub fn get_lambda_defined_symbol(&self, signature: &Signature) -> Option<FunCode> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        let left = if formula.f_code() == signature.eqn_code() {
            formula.argument(0)
        } else if formula.f_code() == signature.equiv_code() {
            let equivalence_left = formula.argument(0)?;
            if equivalence_left.f_code() == signature.eqn_code() {
                equivalence_left.argument(0)
            } else {
                None
            }
        } else {
            None
        }?;

        (left.f_code() > signature.internal_symbols()).then_some(left.f_code())
    }

    /// Renders C's `WFormulaAppEncode` shape for this wrapped formula.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if term application encoding fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or is marked as a clause.
    pub fn app_encode_string(
        &self,
        bank: &mut TermBank,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        self.app_encode_string_with_type_suffixes(bank, keep_input_names, false)
    }

    /// Renders C's `WFormulaAppEncode` shape with optional `TermPrintTypes` suffixes.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic under the same conditions as [`Self::app_encode_string`].
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::app_encode_string`].
    pub fn app_encode_string_with_type_suffixes(
        &self,
        bank: &mut TermBank,
        keep_input_names: bool,
        print_types: bool,
    ) -> Result<String, Diagnostic> {
        assert!(!self.is_clause, "WFormulaAppEncode expects a formula");
        let encoded =
            tformula_app_encode_string_with_type_suffixes(bank, self.formula(), print_types)?;
        Ok(format!(
            "tff({}, {}, {encoded}).",
            self.get_id(keep_input_names),
            self.app_encode_role_name()
        ))
    }

    /// Converts C's wrapped clause formula back into a `Clause`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if collecting a literal or allocating the resulting
    /// clause fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if an encoded literal has
    /// malformed arguments.
    pub fn form_clause_to_clause(&self, bank: &mut TermBank) -> Result<Clause, Diagnostic> {
        let mut clause = tformula_collect_clause(bank, self.formula(), None)?;
        clause.set_properties(self.properties);
        clause.set_info(self.info.clone());
        Ok(clause)
    }

    /// Encodes a parsed clause as a clause-backed wrapped formula, matching C
    /// `WFormClauseParse`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the clause cannot be encoded as a formula.
    pub fn form_clause_alloc(
        bank: &mut TermBank,
        mut clause: Clause,
        problem_type: ProblemType,
    ) -> Result<Self, Diagnostic> {
        let formula = tformula_clause_encode(bank, &clause, problem_type)?;
        let mut wrapped = Self::wt_formula_alloc(formula);
        wrapped.is_clause = true;
        wrapped.properties = clause.properties();
        wrapped.info = clause.take_info();
        Ok(wrapped)
    }

    /// Renders the formula body used by C formula proof documentation.
    ///
    /// This is the unwrapped `TFormulaTPTPPrint`/`TFormulaTSTPPrint` payload,
    /// not a complete `fof(...)`/`tff(...)` record.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if clause-backed closure construction or formula
    /// rendering fails.
    pub fn proof_doc_formula_body_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
    ) -> Result<String, Diagnostic> {
        if self.is_clause {
            let closure = tformula_closure(bank, self.formula(), true)?;
            tformula_tptp_string(
                bank,
                &closure,
                full_terms,
                TFormulaTptpPrintOptions::tstp(problem_type),
            )
        } else {
            tformula_tptp_string(
                bank,
                self.formula(),
                full_terms,
                TFormulaTptpPrintOptions::tstp(problem_type),
            )
        }
    }

    #[must_use]
    pub fn proof_doc_view<'a>(&'a self, rendered_formula: &'a str) -> FormulaDocView<'a> {
        let view = FormulaDocView::new(self.ident(), self.properties(), rendered_formula)
            .with_untyped(self.is_untyped());
        if let Some(info) = self.info() {
            view.with_info(info)
        } else {
            view
        }
    }

    /// Universally closes a clause encoding as C `WFormulaOfClause` does.
    ///
    /// C allocates a fresh formula wrapper and intentionally does not copy
    /// clause metadata onto it.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal encoding, closure allocation, or
    /// quantifier allocation fails.
    ///
    /// # Panics
    ///
    /// Panics if any clause literal violates term-bank sharing preconditions.
    pub fn of_clause(
        bank: &mut TermBank,
        clause: &Clause,
        problem_type: ProblemType,
    ) -> Result<Self, Diagnostic> {
        Ok(Self::wt_formula_alloc(tformula_clause_closed_encode(
            bank,
            clause,
            problem_type,
        )?))
    }

    /// Transforms this wrapped formula into CNF clauses, matching C
    /// `WFormulaCNF2` for the currently ported term-level CNF phases.
    ///
    /// The wrapper formula is first DB-lambda normalized. Clause-backed
    /// wrappers are converted directly into one clause with `DCFofQuote`
    /// provenance. Formula-backed wrappers are transformed through
    /// `WTFormulaConjunctiveNF3`, update this wrapper's formula payload, and
    /// then delegate to the staged `TFormulaToCNF` core.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if lambda normalization, clause conversion,
    /// higher-order post-CNF encoding, CNF transformation, or clause insertion
    /// preparation fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, if the miniscope limit is
    /// negative, or if a malformed encoded literal/formula violates the C CNF
    /// preconditions.
    pub fn cnf2_into(
        &mut self,
        bank: &mut TermBank,
        set: &mut ClauseSet,
        fresh_vars: &VarBank,
        miniscope_limit: i64,
        fool_unroll: bool,
        problem_type: ProblemType,
    ) -> Result<WrappedFormulaCnfResult, Diagnostic> {
        let normalized = lambda_normalize_db(bank, self.formula())?;
        self.set_formula(normalized);
        let source = FormulaDerivationRef::new(self.ident);

        if self.is_clause {
            let mut clause = self.form_clause_to_clause(bank)?;
            clause_push_formula_derivation(&mut clause, DC_FOF_QUOTE, Some(source), None);
            if problem_type == ProblemType::HigherOrder {
                post_cnf_encode_clause_terms(bank, &mut clause)?;
            }
            set.insert(clause);
            return Ok(WrappedFormulaCnfResult {
                clauses_generated: 1,
                formula_derivation_ops: Vec::new(),
            });
        }

        let cnf_result =
            tformula_conjunctive_nf3(bank, self.formula(), miniscope_limit, fool_unroll)?;
        let formula_derivation_ops = cnf_result.derivation_ops().to_vec();
        self.set_formula(cnf_result.formula().clone());
        for op in &formula_derivation_ops {
            self.push_formula_derivation(*op, None, None);
        }
        let clauses_generated = tformula_to_cnf(
            bank,
            self.formula(),
            self.query_tptp_type(),
            set,
            fresh_vars,
            source,
            problem_type,
        )?;
        Ok(WrappedFormulaCnfResult {
            clauses_generated,
            formula_derivation_ops,
        })
    }

    /// Transforms this wrapped formula into CNF clauses and emits C
    /// proof-documentation for represented formula phases and split clauses.
    ///
    /// This is the proof-documenting counterpart to [`Self::cnf2_into`].
    /// Formula-backed wrappers emit `DocFormulaModificationDefault` for each
    /// documented changed `WTFormulaConjunctiveNF3` phase, then
    /// `DocClauseFromForm` for each generated clause before the
    /// `DCSplitConjunct` derivation is pushed. The direct clause-wrapper
    /// shortcut remains output-free, matching C `WFormulaCNF2`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if lambda normalization, CNF transformation,
    /// formula rendering, proof-documentation rendering, clause conversion,
    /// higher-order post-CNF encoding, or clause insertion preparation fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, if the miniscope limit is
    /// negative, or if a malformed encoded literal/formula violates the C CNF
    /// preconditions.
    pub fn cnf2_into_with_docs<W: fmt::Write>(
        &mut self,
        doc: &mut WrappedFormulaCnfDocContext<'_, W>,
        bank: &mut TermBank,
        set: &mut ClauseSet,
        fresh_vars: &VarBank,
        miniscope_limit: i64,
        fool_unroll: bool,
    ) -> Result<WrappedFormulaCnfDocResult, Diagnostic> {
        let normalized = lambda_normalize_db(bank, self.formula())?;
        self.set_formula(normalized);
        let source = FormulaDerivationRef::new(self.ident);

        if self.is_clause {
            let mut clause = self.form_clause_to_clause(bank)?;
            clause_push_formula_derivation(&mut clause, DC_FOF_QUOTE, Some(source), None);
            if doc.render_options.problem_type == ProblemType::HigherOrder {
                post_cnf_encode_clause_terms(bank, &mut clause)?;
            }
            set.insert(clause);
            return Ok(WrappedFormulaCnfDocResult {
                cnf: WrappedFormulaCnfResult {
                    clauses_generated: 1,
                    formula_derivation_ops: Vec::new(),
                },
                formula_write_results: Vec::new(),
                clause_write_results: Vec::new(),
            });
        }

        let cnf_result =
            tformula_conjunctive_nf3(bank, self.formula(), miniscope_limit, fool_unroll)?;
        let formula_derivation_ops = cnf_result.derivation_ops().to_vec();
        let mut formula_write_results = Vec::new();
        for phase in cnf_result.changed_phases() {
            self.set_formula(phase.formula().clone());
            if let Some(inference) = cnf_phase_formula_inference(phase.op()) {
                let (write_result, new_ident, new_properties) = {
                    let rendered = self.proof_doc_formula_body_string(
                        bank,
                        doc.render_options.full_terms,
                        doc.render_options.problem_type,
                    )?;
                    let mut view = self.proof_doc_view(&rendered);
                    let write_result = doc.session.doc_formula_modification(
                        &mut *doc.output,
                        &mut view,
                        inference,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                self.ident = new_ident;
                self.set_properties(new_properties);
                formula_write_results.push(write_result);
            }
            self.push_formula_derivation(phase.op(), None, None);
        }
        self.set_formula(cnf_result.formula().clone());

        let source = FormulaDerivationRef::new(self.ident);
        let parent_rendered = self.proof_doc_formula_body_string(
            bank,
            doc.render_options.full_terms,
            doc.render_options.problem_type,
        )?;
        let parent_view = self.proof_doc_view(&parent_rendered);
        let clause_doc_result = tformula_to_cnf_with_docs(
            TFormulaToCnfDocContext::new(&mut *doc.output, &mut *doc.session, &parent_view),
            bank,
            set,
            TFormulaToCnfInput::new(
                self.formula(),
                self.query_tptp_type(),
                fresh_vars,
                source,
                doc.render_options.problem_type,
            ),
        )?;
        Ok(WrappedFormulaCnfDocResult {
            cnf: WrappedFormulaCnfResult {
                clauses_generated: clause_doc_result.clauses_generated,
                formula_derivation_ops,
            },
            formula_write_results,
            clause_write_results: clause_doc_result.write_results,
        })
    }

    /// Renders C's `WFormulaTPTPPrint` shape for a wrapped formula.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tptp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        let rendered = tformula_tptp_string(
            bank,
            self.formula(),
            full_terms,
            TFormulaTptpPrintOptions::tptp(problem_type),
        )?;
        Ok(format!(
            "input_formula({},{},{rendered}).",
            self.get_id(keep_input_names),
            self.tptp_role_name()
        ))
    }

    /// Renders C's `WFormulaTSTPPrintFlex` shape for a formula-backed wrapper.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tstp_string_flex(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        options: FormulaTstpPrintOptions,
    ) -> Result<String, Diagnostic> {
        let formula_kind = if problem_type == ProblemType::HigherOrder {
            "thf"
        } else if self.is_clause && options.clause_mode == FormulaTstpClauseMode::AsClauseCore {
            if self.is_untyped() {
                "cnf"
            } else {
                "tcf"
            }
        } else if self.is_untyped() {
            "fof"
        } else {
            "tff"
        };
        let rendered = if self.is_clause {
            match options.clause_mode {
                FormulaTstpClauseMode::AsFormula => {
                    let closure = tformula_closure(bank, self.formula(), true)?;
                    tformula_tptp_string(
                        bank,
                        &closure,
                        options.full_terms,
                        TFormulaTptpPrintOptions::tstp(problem_type),
                    )?
                }
                FormulaTstpClauseMode::AsClauseCore => {
                    let clause = self.form_clause_to_clause(bank)?;
                    clause_print_tstp_core_string(bank, &clause, options.full_terms, false)
                }
            }
        } else {
            tformula_tptp_string(
                bank,
                self.formula(),
                options.full_terms,
                TFormulaTptpPrintOptions::tstp(problem_type),
            )?
        };
        let mut output = format!(
            "{formula_kind}({}, {}, {rendered}",
            self.get_id(options.keep_input_names),
            self.tstp_role_name()
        );
        if options.completeness == FormulaTstpCompleteness::Complete {
            output.push_str(").");
        }
        Ok(output)
    }

    /// Renders C's `WFormulaTSTPPrint` macro shape with `as_formula=true`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tstp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        complete: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        self.tstp_string_flex(
            bank,
            problem_type,
            FormulaTstpPrintOptions {
                full_terms,
                completeness: if complete {
                    FormulaTstpCompleteness::Complete
                } else {
                    FormulaTstpCompleteness::Open
                },
                clause_mode: FormulaTstpClauseMode::AsFormula,
                keep_input_names,
            },
        )
    }

    /// Renders C's `WFormulaPrint` dispatch for formula-backed wrappers.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the selected output format or contained term
    /// formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn print_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        output_format: FormulaPrintFormat,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        if self.is_clause {
            let clause = self.form_clause_to_clause(bank)?;
            return match output_format {
                FormulaPrintFormat::Lop => {
                    Ok(clause_print_lop_format_string(bank, &clause, full_terms))
                }
                FormulaPrintFormat::Tptp => Ok(clause_print_tptp_format_string(bank, &clause)),
                FormulaPrintFormat::Tstp => {
                    clause_tstp_string(bank, &clause, full_terms, true, problem_type)
                }
            };
        }

        match output_format {
            FormulaPrintFormat::Lop | FormulaPrintFormat::Tptp => {
                self.tptp_string(bank, full_terms, problem_type, keep_input_names)
            }
            FormulaPrintFormat::Tstp => {
                self.tstp_string(bank, full_terms, true, problem_type, keep_input_names)
            }
        }
    }

    /// Counts lambda cells that occur below a non-logical/non-lambda formula node.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a malformed lambda or
    /// phony-application cell has uninitialized arguments.
    #[must_use]
    pub fn count_non_top_level_lambdas(&self, signature: &Signature) -> i32 {
        tformula_count_non_top_level_lambdas(signature, self.formula())
    }

    /// Returns whether any literal side is an applied free variable.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    #[must_use]
    pub fn has_app_var_literal(&self, bank: &TermBank) -> bool {
        tformula_has_app_var_literal(bank, self.formula())
    }

    fn lambda_definition_head_for_statistics(&self, bank: &TermBank) -> Option<Term> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let signature = bank.signature();
        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        if formula.f_code() == signature.eqn_code() {
            return formula.argument(0);
        }
        if formula.f_code() != signature.equiv_code() {
            return None;
        }

        let equivalence_left = formula.argument(0)?;
        if equivalence_left.f_code() != signature.eqn_code()
            || equivalence_left.argument(1).as_ref() != Some(bank.true_term())
        {
            return None;
        }
        equivalence_left.argument(0)
    }

    fn app_encode_role_name(&self) -> &'static str {
        self.tstp_role_name()
    }

    fn tstp_role_name(&self) -> &'static str {
        match self.query_tptp_type() {
            CP_TYPE_AXIOM if self.query_prop(CP_INPUT_FORMULA) => "axiom",
            CP_TYPE_HYPOTHESIS => "hypothesis",
            CP_TYPE_CONJECTURE => "conjecture",
            CP_TYPE_QUESTION => "question",
            CP_TYPE_LEMMA => "lemma",
            CP_TYPE_NEG_CONJECTURE => "negated_conjecture",
            _ => "plain",
        }
    }

    fn tptp_role_name(&self) -> &'static str {
        match self.query_tptp_type() {
            CP_TYPE_AXIOM => "axiom",
            CP_TYPE_HYPOTHESIS => "hypothesis",
            CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE => "conjecture",
            CP_TYPE_QUESTION => "question",
            _ => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormulaDefinitionStatistics {
    pub num_defs: i32,
    pub percentage_form_defs: f64,
    pub num_lams: i32,
    pub has_app_var_lits: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSet {
    formulas: Vec<WrappedFormula>,
    identifier: String,
}

impl FormulaSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        usize_to_i64(self.formulas.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = identifier.into();
    }

    pub fn iter(&self) -> impl Iterator<Item = &WrappedFormula> {
        self.formulas.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WrappedFormula> {
        self.formulas.iter_mut()
    }

    pub fn insert(&mut self, formula: WrappedFormula) -> u64 {
        let entry_id = formula.entry_id();
        self.formulas.push(formula);
        entry_id
    }

    pub fn insert_set(&mut self, from: &mut Self) -> i64 {
        let moved = from.cardinality();
        self.formulas.append(&mut from.formulas);
        moved
    }

    pub fn clear(&mut self) {
        self.formulas.clear();
    }

    pub fn extract_first(&mut self) -> Option<WrappedFormula> {
        if self.formulas.is_empty() {
            None
        } else {
            Some(self.formulas.remove(0))
        }
    }

    pub fn extract_entry(&mut self, entry_id: u64) -> Option<WrappedFormula> {
        let position = self
            .formulas
            .iter()
            .position(|formula| formula.entry_id() == entry_id)?;
        Some(self.formulas.remove(position))
    }

    #[must_use]
    pub fn get(&self, entry_id: u64) -> Option<&WrappedFormula> {
        self.formulas
            .iter()
            .find(|formula| formula.entry_id() == entry_id)
    }

    pub fn get_mut(&mut self, entry_id: u64) -> Option<&mut WrappedFormula> {
        self.formulas
            .iter_mut()
            .find(|formula| formula.entry_id() == entry_id)
    }

    pub fn delete_entry(&mut self, entry_id: u64) -> bool {
        self.extract_entry(entry_id).is_some()
    }

    pub fn move_formula_from(&mut self, from: &mut Self, entry_id: u64) -> bool {
        let Some(formula) = from.extract_entry(entry_id) else {
            return false;
        };
        self.insert(formula);
        true
    }

    /// Applies C `FormulaSetArchive`.
    ///
    /// Each formula is extracted in insertion order, the original wrapper is
    /// moved to `archive`, and a flat copy is inserted back into this set. The
    /// replacement copy receives the formula-level `DCFofQuote` derivation that
    /// quotes the archived original.
    #[must_use]
    pub fn archive_into(&mut self, archive: &mut Self) -> FormulaSetArchiveResult {
        let mut result = FormulaSetArchiveResult::default();
        let mut tmpset = Self::new();

        while let Some(handle) = self.extract_first() {
            let source = FormulaDerivationRef::new(handle.ident());
            let mut newform = handle.flat_copy();
            newform.push_formula_derivation(DC_FOF_QUOTE, Some(source), None);
            tmpset.insert(newform);
            archive.insert(handle);
            result.formulas_archived += 1;
            result.quoted_formula_sources.push(source);
            result.formula_derivation_ops.push(DC_FOF_QUOTE);
        }

        self.insert_set(&mut tmpset);
        result
    }

    /// Applies C `FormulaSetDocInital`.
    ///
    /// The misspelled C helper documents each formula as an initial formula
    /// when proof-document output level is at least two. The session owns that
    /// level gate and id assignment; this wrapper preserves insertion-order
    /// traversal and uses each formula's rendered proof-document body.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if rendering a wrapped formula body or writing
    /// proof documentation fails.
    pub fn doc_initial<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        full_terms: bool,
        problem_type: ProblemType,
    ) -> Result<FormulaSetDocInitialResult, Diagnostic> {
        let mut result = FormulaSetDocInitialResult::default();
        for formula in &mut self.formulas {
            let rendered = formula.proof_doc_formula_body_string(bank, full_terms, problem_type)?;
            let (write_result, new_ident) = {
                let mut view = formula.proof_doc_view(&rendered);
                let write_result = session.doc_formula_creation(
                    output,
                    &mut view,
                    FormulaCreationInference::Initial,
                    FormulaCreationParents::none(),
                    None,
                )?;
                (write_result, view.ident())
            };
            formula.ident = new_ident;
            result.formulas_seen += 1;
            result.write_results.push(write_result);
        }
        Ok(result)
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.formulas.iter().all(WrappedFormula::is_untyped)
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        self.formulas
            .iter()
            .any(WrappedFormula::has_interpreted_symbol)
    }

    #[must_use]
    pub fn split_conjectures(&self) -> (i64, Vec<&WrappedFormula>, Vec<&WrappedFormula>) {
        let mut conjectures = Vec::new();
        let mut rest = Vec::new();
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures.push(formula);
            } else {
                rest.push(formula);
            }
        }
        (usize_to_i64(conjectures.len()), conjectures, rest)
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.formulas
            .iter()
            .filter(|formula| !formula.query_prop(CP_IS_LAMBDA_DEF))
            .map(WrappedFormula::standard_weight)
            .sum()
    }

    pub fn count_conjectures(&self, hypotheses: &mut i64) -> i64 {
        let mut conjectures = 0;
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures += 1;
            }
            if formula.is_hypothesis() {
                *hypotheses += 1;
            }
        }
        conjectures
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        self.formulas
            .iter()
            .filter(|formula| formula.is_conjecture() || formula.is_hypothesis())
            .map(|formula| formula.conjecture_order(signature))
            .max()
            .unwrap_or(0)
    }

    pub fn collect_f_code(&self, f_code: FunCode, result: &mut Vec<u64>) -> i64 {
        let start = result.len();
        result.extend(
            self.formulas
                .iter()
                .filter(|formula| formula.contains_f_code(f_code))
                .map(WrappedFormula::entry_id),
        );
        usize_to_i64(result.len() - start)
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.gc_mark_cells(bank);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.mark_polarity(bank);
        }
    }

    /// Applies C `TFormulaSetDelTermpProp`.
    ///
    /// Walks formulas in set order, ignores wrappers without a formula payload,
    /// and deletes `props` recursively using C's `DEREF_NEVER` behavior.
    pub fn del_term_props(&self, props: TermProperties) {
        for formula in &self.formulas {
            if let Some(term) = &formula.formula {
                term_del_prop(term, DerefType::Never, props);
            }
        }
    }

    /// Applies C `FormulaSetSimplify` to each formula in insertion order
    /// without the optional term-bank garbage collection side effect.
    ///
    /// This stages the mutating simplification and changed-count behavior.
    /// Changed formulas receive `DCFofSimplify` stack entries and are also
    /// represented by opcodes in the result metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be simplified.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify(
        &mut self,
        bank: &mut TermBank,
    ) -> Result<FormulaSetSimplifyResult, Diagnostic> {
        self.simplify_with_garbage_collection(bank, false)
    }

    /// Applies C `FormulaSetSimplify` while also emitting C
    /// `DocFormulaModificationDefault(..., inf_fof_simpl)` output for changed
    /// formulas.
    ///
    /// This is the proof-documenting counterpart to [`Self::simplify`].
    /// Changed formulas keep the staged `DCFofSimplify` derivation entries and
    /// also receive the proof-document id/property side effects that C applies
    /// to the modified `WFormula`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if simplification, proof-document formula
    /// rendering, or proof-document writing fails.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify_with_docs<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        full_terms: bool,
        problem_type: ProblemType,
    ) -> Result<FormulaSetSimplifyDocResult, Diagnostic> {
        self.simplify_with_garbage_collection_and_docs(
            output,
            bank,
            session,
            full_terms,
            problem_type,
            false,
        )
    }

    /// Applies C `FormulaSetSimplify` to each formula in insertion order.
    ///
    /// When `do_garbage_collect` is true, this mirrors C's thresholded
    /// `TBGCCollect` checks using this set as the formula root set. Changed
    /// formulas receive `DCFofSimplify` stack entries and are also represented
    /// by opcodes in the result metadata; proof-document output remains
    /// deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be simplified.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify_with_garbage_collection(
        &mut self,
        bank: &mut TermBank,
        do_garbage_collect: bool,
    ) -> Result<FormulaSetSimplifyResult, Diagnostic> {
        let mut result = FormulaSetSimplifyResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);
        let mut index = 0;

        while index < self.formulas.len() {
            let changed = {
                let formula = &mut self.formulas[index];
                formula.simplify(bank)?
            };
            if changed {
                result.formulas_changed += 1;
                result.formula_derivation_ops.push(DC_FOF_SIMPLIFY);
                if do_garbage_collect && bank.non_var_term_nodes() > gc_threshold {
                    collect_formula_set_simplify_garbage(bank, self, &mut result);
                    old_nodes = bank.non_var_term_nodes();
                    gc_threshold = formula_set_gc_threshold(old_nodes);
                }
            }
            index += 1;
        }

        if do_garbage_collect && bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_simplify_garbage(bank, self, &mut result);
        }
        Ok(result)
    }

    /// Applies C `FormulaSetSimplify` with optional term-bank garbage
    /// collection and proof-documenting formula modification output.
    ///
    /// This mirrors the C ordering at the formula-set level: each changed
    /// formula is documented before any threshold-triggered garbage collection
    /// for that iteration.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if simplification, proof-document formula
    /// rendering, proof-document writing, or garbage collection bookkeeping
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify_with_garbage_collection_and_docs<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        full_terms: bool,
        problem_type: ProblemType,
        do_garbage_collect: bool,
    ) -> Result<FormulaSetSimplifyDocResult, Diagnostic> {
        let mut result = FormulaSetSimplifyDocResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);
        let mut index = 0;

        while index < self.formulas.len() {
            let changed = {
                let formula = &mut self.formulas[index];
                formula.simplify(bank)?
            };
            if changed {
                result.simplify.formulas_changed += 1;
                result.simplify.formula_derivation_ops.push(DC_FOF_SIMPLIFY);
                let (write_result, new_ident, new_properties) = {
                    let formula = &self.formulas[index];
                    let rendered =
                        formula.proof_doc_formula_body_string(bank, full_terms, problem_type)?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = session.doc_formula_modification(
                        output,
                        &mut view,
                        FormulaModificationInference::Simplification,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                {
                    let formula = &mut self.formulas[index];
                    formula.ident = new_ident;
                    formula.set_properties(new_properties);
                }
                result.write_results.push(write_result);

                if do_garbage_collect && bank.non_var_term_nodes() > gc_threshold {
                    collect_formula_set_simplify_garbage(bank, self, &mut result.simplify);
                    old_nodes = bank.non_var_term_nodes();
                    gc_threshold = formula_set_gc_threshold(old_nodes);
                }
            }
            index += 1;
        }

        if do_garbage_collect && bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_simplify_garbage(bank, self, &mut result.simplify);
        }
        Ok(result)
    }

    /// Applies C `FormulaSetPreprocConjectures` in insertion order.
    ///
    /// Each formula is first annotated as a question when applicable, then
    /// conjectures are negated. Mutated formulas receive the corresponding
    /// formula-owned derivation entries and this also returns the C derivation
    /// opcodes as result metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if answer-literal allocation or conjecture negation
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if any preprocessed wrapper has no formula term.
    pub fn preproc_conjectures(
        &mut self,
        bank: &mut TermBank,
        add_answer_lits: bool,
        conjectures_are_questions: bool,
    ) -> Result<FormulaSetPreprocessConjecturesResult, Diagnostic> {
        let mut result = FormulaSetPreprocessConjecturesResult::default();
        for formula in &mut self.formulas {
            if formula.annotate_question(bank, add_answer_lits, conjectures_are_questions)? {
                result.questions_annotated += 1;
                result.formula_derivation_ops.push(DC_ANNO_QUESTION);
            }
            if formula.conjecture_negate(bank)? {
                result.conjectures_negated += 1;
                result.formula_derivation_ops.push(DC_NEGATE_CONJECTURE);
            }
        }
        Ok(result)
    }

    /// Applies C `FormulaSetPreprocConjectures` and emits its proof-document
    /// modification steps.
    ///
    /// This mirrors `preproc_conjectures` but also ports the
    /// `DocFormulaModificationDefault` calls performed by
    /// `WFormulaAnnotateQuestion` and `WFormulaConjectureNegate`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if answer-literal allocation, conjecture negation,
    /// formula rendering, or proof-document writing fails.
    ///
    /// # Panics
    ///
    /// Panics if any preprocessed wrapper has no formula term.
    pub fn preproc_conjectures_with_docs<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
        add_answer_lits: bool,
        conjectures_are_questions: bool,
    ) -> Result<FormulaSetPreprocessConjecturesDocResult, Diagnostic> {
        let mut result = FormulaSetPreprocessConjecturesDocResult::default();

        for index in 0..self.formulas.len() {
            let annotated = {
                let formula = &mut self.formulas[index];
                formula.annotate_question(bank, add_answer_lits, conjectures_are_questions)?
            };
            if annotated {
                result.preprocess.questions_annotated += 1;
                result
                    .preprocess
                    .formula_derivation_ops
                    .push(DC_ANNO_QUESTION);
                let (write_result, new_ident, new_properties) = {
                    let formula = &self.formulas[index];
                    let rendered = formula.proof_doc_formula_body_string(
                        bank,
                        render_options.full_terms,
                        render_options.problem_type,
                    )?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = session.doc_formula_modification(
                        output,
                        &mut view,
                        FormulaModificationInference::AnnotateQuestion,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                {
                    let formula = &mut self.formulas[index];
                    formula.ident = new_ident;
                    formula.set_properties(new_properties);
                }
                result.write_results.push(write_result);
            }

            let negated = {
                let formula = &mut self.formulas[index];
                formula.conjecture_negate(bank)?
            };
            if negated {
                result.preprocess.conjectures_negated += 1;
                result
                    .preprocess
                    .formula_derivation_ops
                    .push(DC_NEGATE_CONJECTURE);
                let (write_result, new_ident, new_properties) = {
                    let formula = &self.formulas[index];
                    let rendered = formula.proof_doc_formula_body_string(
                        bank,
                        render_options.full_terms,
                        render_options.problem_type,
                    )?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = session.doc_formula_modification(
                        output,
                        &mut view,
                        FormulaModificationInference::NegConjecture,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                {
                    let formula = &mut self.formulas[index];
                    formula.ident = new_ident;
                    formula.set_properties(new_properties);
                }
                result.write_results.push(write_result);
            }
        }

        Ok(result)
    }

    /// Applies C `WFormulaSetUnrollFOOL` in insertion order.
    ///
    /// Each formula first runs `WFormulaReplaceEqnWithEquiv`, then
    /// `TFormulaUnrollFOOL`. Mutated formulas receive the corresponding
    /// formula-owned derivation entries and this also returns the C derivation
    /// opcodes as result metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if formula rebuilding, literal expansion, lambda
    /// eta-reduction, or term replacement fails.
    ///
    /// # Panics
    ///
    /// Panics if any preprocessed wrapper has no formula term or a malformed
    /// formula payload.
    pub fn unroll_fool(
        &mut self,
        bank: &mut TermBank,
    ) -> Result<FormulaSetFoolUnrollResult, Diagnostic> {
        let mut result = FormulaSetFoolUnrollResult::default();
        for formula in &mut self.formulas {
            if formula.replace_eqn_with_equiv(bank)? {
                result.boolean_equalities_replaced += 1;
                result.formula_derivation_ops.push(DC_EQ_TO_EQ);
            }
            if formula.unroll_fool(bank)? {
                result.formulas_unrolled += 1;
                result.formula_derivation_ops.push(DC_FOOL_UNROLL);
            }
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetNamedToDBLambdas` in insertion order.
    ///
    /// This is gated to higher-order problems, matching the `FormulaSetCNF2`
    /// `ENABLE_LFHO` branch. Changed formulas receive `DCFofSimplify` stack
    /// entries and this also returns the opcodes as result metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if named-lambda conversion or beta normalization
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or a malformed
    /// named-lambda payload.
    pub fn named_to_db_lambdas(
        &mut self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }
        for formula in &mut self.formulas {
            if formula.named_to_db_lambdas(bank)? {
                result.formulas_named_to_db += 1;
                result.formula_derivation_ops.push(DC_FOF_SIMPLIFY);
            }
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetNamedToDBLambdas` and emits its proof-document
    /// simplification steps.
    ///
    /// This is the proof-documenting counterpart to
    /// [`Self::named_to_db_lambdas`]. Changed formulas keep the staged
    /// `DCFofSimplify` derivation entry and also receive the proof-document
    /// id/property side effects that C applies through
    /// `DocFormulaModificationDefault(..., inf_fof_simpl)`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if named-lambda conversion, beta normalization,
    /// formula rendering, or proof-document writing fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or a malformed
    /// named-lambda payload.
    pub fn named_to_db_lambdas_with_docs<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
    ) -> Result<FormulaSetHigherOrderPreprocessDocResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessDocResult::default();
        if render_options.problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }

        for index in 0..self.formulas.len() {
            let changed = {
                let formula = &mut self.formulas[index];
                formula.named_to_db_lambdas(bank)?
            };
            if changed {
                result.preprocess.formulas_named_to_db += 1;
                result
                    .preprocess
                    .formula_derivation_ops
                    .push(DC_FOF_SIMPLIFY);
                let (write_result, new_ident, new_properties) = {
                    let formula = &self.formulas[index];
                    let rendered = formula.proof_doc_formula_body_string(
                        bank,
                        render_options.full_terms,
                        render_options.problem_type,
                    )?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = session.doc_formula_modification(
                        output,
                        &mut view,
                        FormulaModificationInference::Simplification,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                {
                    let formula = &mut self.formulas[index];
                    formula.ident = new_ident;
                    formula.set_properties(new_properties);
                }
                result.write_results.push(write_result);
            }
        }

        Ok(result)
    }

    /// Applies C `TFormulaSetLiftItes` in insertion order.
    ///
    /// This is gated to higher-order problems, matching the `FormulaSetCNF2`
    /// `ENABLE_LFHO` branch. Changed formulas receive `DCLiftIte` stack entries
    /// and this also returns the opcodes as result metadata; proof output
    /// remains deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if ITE expansion or copied term insertion fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or if an `$ite`,
    /// literal, or formula cell is malformed.
    pub fn lift_ites(
        &mut self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }
        for formula in &mut self.formulas {
            if formula.lift_ites(bank)? {
                result.formulas_ites_lifted += 1;
                result.formula_derivation_ops.push(DC_LIFT_ITE);
            }
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetLiftLets` in insertion order.
    ///
    /// Generated definition wrappers are appended after the traversal in the
    /// same stack-pop order C uses. Generated definitions receive
    /// `DCIntroDef`, and each rewritten source formula receives `DCApplyDef`
    /// parented by the generated definition. This also returns those opcodes as
    /// result metadata; proof output remains deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if variable renaming, LET lifting, predicate
    /// encoding, closure, instantiation, app flattening, or term-bank insertion
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or if a `$let` cell,
    /// definition equality, or definition head is malformed.
    pub fn lift_lets(
        &mut self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }

        let mut lifted_definitions = Vec::new();
        for formula in &mut self.formulas {
            let definitions = formula.lift_lets(bank)?;
            if !definitions.is_empty() {
                result.formulas_lets_lifted += 1;
                for definition in definitions {
                    let mut definition_wrapper = WrappedFormula::wt_formula_alloc(definition);
                    definition_wrapper.push_formula_derivation(DC_INTRO_DEF, None, None);
                    formula.push_formula_derivation(
                        DC_APPLY_DEF,
                        Some(FormulaDerivationRef::new(definition_wrapper.ident())),
                        None,
                    );
                    lifted_definitions.push(definition_wrapper);
                    result.formula_derivation_ops.push(DC_INTRO_DEF);
                    result.formula_derivation_ops.push(DC_APPLY_DEF);
                }
            }
        }

        while let Some(definition) = lifted_definitions.pop() {
            self.insert(definition);
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetLiftLambdas` in insertion order.
    ///
    /// Generated definition wrappers are appended after the original traversal,
    /// so the definitions are not lifted again by the same pass.
    /// Proof-document output is deferred. Rewritten source formulas store the
    /// validated no-parent `DCIntroDef` entries and this still returns those
    /// opcodes as staged metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if beta normalization, lambda lifting, predicate
    /// encoding, or term-bank insertion fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or if a lambda,
    /// formula, or definition payload is malformed.
    pub fn lift_lambdas(
        &mut self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }

        bank.vars().set_v_counts_to_used();
        let mut state = LambdaLiftState::default();
        let mut lifted_definitions = BTreeMap::new();
        for formula in &mut self.formulas {
            let original = formula.formula().clone();
            let mut formula_defs = Vec::new();
            let lifted = lift_lambdas_in_term(bank, &original, &mut state, &mut formula_defs)?;
            if lifted != original {
                formula.set_formula(lifted);
                result.formulas_lambdas_lifted += 1;
                for definition in formula_defs.into_iter().rev() {
                    result.lambda_lift_definitions_inserted += 1;
                    result.formula_derivation_ops.push(DC_INTRO_DEF);
                    formula.push_formula_derivation(DC_INTRO_DEF, None, None);
                    lifted_definitions
                        .entry(definition.entry_id())
                        .or_insert(definition);
                }
            }
        }

        for definition in lifted_definitions.into_values() {
            self.insert(definition);
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetUnfoldDefSymbols`.
    ///
    /// The pass recognizes `CP_IS_LAMBDA_DEF` wrappers in C's definition
    /// shapes, archives simplified `symbol = lambda` definitions, rewrites the
    /// remaining formulas with those definitions, and moves recognized
    /// original definition wrappers to the archive. Proof-document output is
    /// deferred. Generated definitions quote the original definitions, and
    /// rewritten generated definitions and source formulas store `DCApplyDef`
    /// parent entries while this still returns the opcodes as staged metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if lambda application, beta normalization,
    /// abstraction, quantified-variable refresh, term rewriting, or term-bank
    /// insertion fails.
    ///
    /// # Panics
    ///
    /// Panics if a recognized lambda-definition formula is malformed, if a
    /// definition symbol lacks a signature type, or if a rewritten formula has
    /// malformed term arguments.
    pub fn unfold_def_symbols(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        unfold_only_forms: bool,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        Ok(self
            .unfold_def_symbols_impl::<String>(
                archive,
                bank,
                problem_type,
                unfold_only_forms,
                None,
            )?
            .preprocess)
    }

    /// Applies C `TFormulaSetUnfoldDefSymbols` and emits its proof-document
    /// simplification steps for rewritten source formulas.
    ///
    /// This is the proof-documenting counterpart to
    /// [`Self::unfold_def_symbols`]. It mirrors the C phase boundary where
    /// non-definition source formulas rewritten by unfolded definitions receive
    /// `DocFormulaModificationDefault(..., inf_fof_simpl)` before their
    /// `DCApplyDef` parent entries are pushed. Generated definition
    /// intersimplification remains derivation-only, matching C.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if definition-symbol unfolding, formula rendering,
    /// or proof-document writing fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or malformed term
    /// arguments.
    pub fn unfold_def_symbols_with_docs<W: fmt::Write>(
        &mut self,
        archive: &mut Self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
        unfold_only_forms: bool,
    ) -> Result<FormulaSetHigherOrderPreprocessDocResult, Diagnostic> {
        let problem_type = render_options.problem_type;
        self.unfold_def_symbols_impl(
            archive,
            bank,
            problem_type,
            unfold_only_forms,
            Some(FormulaProofDocContext {
                output,
                session,
                render_options,
            }),
        )
    }

    fn unfold_def_symbols_impl<W: fmt::Write>(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        unfold_only_forms: bool,
        mut doc_context: Option<FormulaProofDocContext<'_, W>>,
    ) -> Result<FormulaSetHigherOrderPreprocessDocResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessDocResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }

        bank.vars().set_v_counts_to_used();
        let DefinitionSymbolMap {
            mut definitions,
            recognized_entry_ids,
        } = create_definition_symbol_map(self, bank, unfold_only_forms)?;
        result
            .preprocess
            .formula_derivation_ops
            .extend(std::iter::repeat_n(DC_FOF_QUOTE, definitions.len()));

        intersimplify_definition_symbols(bank, &mut definitions, &mut result)?;
        rewrite_formulas_with_def_symbols(
            &mut self.formulas,
            bank,
            &definitions,
            &recognized_entry_ids,
            &mut doc_context,
            &mut result,
        )?;

        for definition in definitions.into_values() {
            archive.insert(definition);
            result.preprocess.unfolded_definitions_archived += 1;
        }

        for entry_id in recognized_entry_ids {
            if let Some(original) = self.extract_entry(entry_id) {
                archive.insert(original);
                result.preprocess.unfolded_original_definitions_archived += 1;
            }
        }

        Ok(result)
    }

    /// Applies C `TFormulaSetLambdaNormalize` in insertion order.
    ///
    /// This is gated to higher-order problems and mirrors C's
    /// `BetaNormalizeDB` followed by `LambdaToForall`. Changed formulas receive
    /// `DCFofSimplify` stack entries and this also returns the opcodes as
    /// result metadata; proof output remains deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if beta normalization or lambda-to-forall
    /// conversion fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or a malformed
    /// lambda/formula payload.
    pub fn lambda_normalize_forall(
        &mut self,
        bank: &mut TermBank,
        problem_type: ProblemType,
    ) -> Result<FormulaSetHigherOrderPreprocessResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessResult::default();
        if problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }
        for formula in &mut self.formulas {
            if formula.lambda_normalize_forall(bank)? {
                result.formulas_lambda_normalized += 1;
                result.formula_derivation_ops.push(DC_FOF_SIMPLIFY);
            }
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetLambdaNormalize` and emits its proof-document
    /// simplification steps.
    ///
    /// This is the proof-documenting counterpart to
    /// [`Self::lambda_normalize_forall`]. Changed formulas keep the staged
    /// `DCFofSimplify` derivation entry and also receive the proof-document
    /// id/property side effects that C applies through
    /// `DocFormulaModificationDefault(..., inf_fof_simpl)`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if beta normalization, lambda-to-forall
    /// conversion, formula rendering, or proof-document writing fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or a malformed
    /// lambda/formula payload.
    pub fn lambda_normalize_forall_with_docs<W: fmt::Write>(
        &mut self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
    ) -> Result<FormulaSetHigherOrderPreprocessDocResult, Diagnostic> {
        let mut result = FormulaSetHigherOrderPreprocessDocResult::default();
        if render_options.problem_type != ProblemType::HigherOrder {
            return Ok(result);
        }

        for index in 0..self.formulas.len() {
            let changed = {
                let formula = &mut self.formulas[index];
                formula.lambda_normalize_forall(bank)?
            };
            if changed {
                result.preprocess.formulas_lambda_normalized += 1;
                result
                    .preprocess
                    .formula_derivation_ops
                    .push(DC_FOF_SIMPLIFY);
                let (write_result, new_ident, new_properties) = {
                    let formula = &self.formulas[index];
                    let rendered = formula.proof_doc_formula_body_string(
                        bank,
                        render_options.full_terms,
                        render_options.problem_type,
                    )?;
                    let mut view = formula.proof_doc_view(&rendered);
                    let write_result = session.doc_formula_modification(
                        output,
                        &mut view,
                        FormulaModificationInference::Simplification,
                        None,
                    )?;
                    (write_result, view.ident(), view.properties())
                };
                {
                    let formula = &mut self.formulas[index];
                    formula.ident = new_ident;
                    formula.set_properties(new_properties);
                }
                result.write_results.push(write_result);
            }
        }

        Ok(result)
    }

    /// Applies C `TFormulaSetIntroduceDefs`.
    ///
    /// This stages the set-level definition introduction pipeline: clear the
    /// definition/polarity term flags, mark formula polarities, find expensive
    /// subformulas in non-clause wrappers, archive neutral definitions, insert
    /// active definitions into the set, and apply the definitions across the
    /// resulting set in insertion order.
    ///
    /// Proof-document output is deferred. The owner stores formula-owned
    /// derivation entries that can be represented with stable formula ids and
    /// still returns the C opcodes as staged metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a definition atom, definition formula, or copied
    /// formula cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term, if formula cells are
    /// malformed, or if definition metadata invariants are violated.
    pub fn introduce_defs(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        limit: i64,
    ) -> Result<FormulaSetIntroduceDefsResult, Diagnostic> {
        Ok(self
            .introduce_defs_impl::<String>(archive, bank, limit, None)?
            .introduce)
    }

    /// Applies C `TFormulaSetIntroduceDefs` and emits its formula proof docs.
    ///
    /// This is the proof-documenting counterpart to [`Self::introduce_defs`].
    /// It emits `DocFormulaCreationDefault` for introduced definitions and
    /// split-equivalence wrappers, then `DocFormulaIntroDefsDefault` for
    /// formulas rewritten by those definitions.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a definition atom, definition formula, copied
    /// formula, or proof-document render cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term, if formula cells are
    /// malformed, or if definition metadata invariants are violated.
    pub fn introduce_defs_with_docs<W: fmt::Write>(
        &mut self,
        archive: &mut Self,
        output: &mut W,
        bank: &mut TermBank,
        session: &mut ProofDocSession,
        render_options: FormulaProofDocRenderOptions,
        limit: i64,
    ) -> Result<FormulaSetIntroduceDefsDocResult, Diagnostic> {
        self.introduce_defs_impl(
            archive,
            bank,
            limit,
            Some(FormulaProofDocContext {
                output,
                session,
                render_options,
            }),
        )
    }

    fn introduce_defs_impl<W: fmt::Write>(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        limit: i64,
        mut doc_context: Option<FormulaProofDocContext<'_, W>>,
    ) -> Result<FormulaSetIntroduceDefsDocResult, Diagnostic> {
        let mut result = FormulaSetIntroduceDefsDocResult::default();
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();

        self.del_term_props(TP_CHECK_FLAG | TP_POS_POLARITY | TP_NEG_POLARITY);
        self.mark_polarity(bank);
        for formula in &self.formulas {
            if limit != 0 && !formula.is_clause {
                tformula_find_defs(
                    bank,
                    formula.formula(),
                    1,
                    limit,
                    &mut defs,
                    &mut renamed_forms,
                )?;
            }
        }

        result.introduce.definitions_introduced = usize_to_i64(renamed_forms.len());
        self.create_introduced_definitions(
            archive,
            bank,
            &mut defs,
            renamed_forms,
            &mut doc_context,
            &mut result,
        )?;
        self.apply_introduced_definitions(bank, &defs, &mut doc_context, &mut result)?;

        Ok(result)
    }

    fn create_introduced_definitions<W: fmt::Write>(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        defs: &mut TFormulaDefinitions,
        renamed_forms: Vec<Term>,
        doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
        result: &mut FormulaSetIntroduceDefsDocResult,
    ) -> Result<(), Diagnostic> {
        for form in renamed_forms {
            let entry_no = form.entry_no();
            let polarity = tformula_decode_polarity(&form);
            let def_atom = defs
                .get(&entry_no)
                .unwrap_or_else(|| panic!("renamed formula {entry_no} must have a definition"))
                .rename_atom()
                .clone();
            let neutral_def = tformula_create_def(bank, &def_atom, &form, 0)?;
            let mut neutral_wrapper = WrappedFormula::wt_formula_alloc(neutral_def);
            doc_introduced_definition(bank, &mut neutral_wrapper, doc_context, result)?;
            let mut archived_wrapper = neutral_wrapper.flat_copy();
            let archived_ref = FormulaDerivationRef::new(archived_wrapper.ident());
            archived_wrapper.push_formula_derivation(DC_INTRO_DEF, None, None);
            let archived_formula = archived_wrapper.formula().clone();
            archive.insert(archived_wrapper);
            result.introduce.archived_definitions += 1;
            result.introduce.formula_derivation_ops.push(DC_INTRO_DEF);

            if polarity == 0 {
                let real_definition_id = neutral_wrapper.ident();
                defs.get_mut(&entry_no)
                    .unwrap_or_else(|| panic!("definition {entry_no} disappeared"))
                    .set_definition_metadata(real_definition_id, archived_formula, archived_ref);
                neutral_wrapper.push_formula_derivation(DC_FOF_QUOTE, Some(archived_ref), None);
                self.insert(neutral_wrapper);
                result.introduce.active_definitions_inserted += 1;
                result.introduce.formula_derivation_ops.push(DC_FOF_QUOTE);
            } else {
                let active_def = tformula_create_def(bank, &def_atom, &form, polarity)?;
                let mut active_wrapper = WrappedFormula::wt_formula_alloc(active_def);
                doc_split_equiv_definition(
                    bank,
                    &neutral_wrapper,
                    &mut active_wrapper,
                    doc_context,
                    result,
                )?;
                let real_definition_id = active_wrapper.ident();
                defs.get_mut(&entry_no)
                    .unwrap_or_else(|| panic!("definition {entry_no} disappeared"))
                    .set_definition_metadata(real_definition_id, archived_formula, archived_ref);
                active_wrapper.push_formula_derivation(DC_SPLIT_EQUIV, Some(archived_ref), None);
                self.insert(active_wrapper);
                result.introduce.active_definitions_inserted += 1;
                result.introduce.formula_derivation_ops.push(DC_SPLIT_EQUIV);
            }
        }
        Ok(())
    }

    fn apply_introduced_definitions<W: fmt::Write>(
        &mut self,
        bank: &mut TermBank,
        defs: &TFormulaDefinitions,
        doc_context: &mut Option<FormulaProofDocContext<'_, W>>,
        result: &mut FormulaSetIntroduceDefsDocResult,
    ) -> Result<(), Diagnostic> {
        for formula in &mut self.formulas {
            let defs_used = formula.apply_defs(bank, defs)?;
            if !defs_used.is_empty() {
                doc_applied_definitions(bank, formula, &defs_used, doc_context, result)?;
                result.introduce.formulas_rewritten += 1;
                let used_count = usize_to_i64(defs_used.len());
                result.introduce.definition_applications += used_count;
                for parent in &defs_used {
                    formula.push_formula_derivation(DC_APPLY_DEF, Some(*parent), None);
                }
                result
                    .introduce
                    .formula_derivation_ops
                    .extend(std::iter::repeat_n(DC_APPLY_DEF, defs_used.len()));
            }
        }
        Ok(())
    }

    /// Drains this set into CNF clauses using the staged core of C
    /// `FormulaSetCNF2`.
    ///
    /// This preserves the supported C phase order: supported higher-order
    /// preprocessing, optional set-level FOOL unrolling, formula
    /// simplification, definition introduction, then the archive/copy/CNF
    /// drain loop, and optional post-CNF clause lambda lifting. Higher-order
    /// formula-set lift-lambda preprocessing, proof-document output, and some
    /// term-bank GC side effects from full `FormulaSetCNF2` are still
    /// deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be transformed into
    /// clauses.
    ///
    /// # Panics
    ///
    /// Panics under the same malformed-formula preconditions as
    /// [`WrappedFormula::cnf2_into`].
    pub fn cnf2_into(
        &mut self,
        archive: &mut Self,
        clauseset: &mut ClauseSet,
        bank: &mut TermBank,
        fresh_vars: &VarBank,
        options: FormulaSetCnfOptions,
    ) -> Result<FormulaSetCnfResult, Diagnostic> {
        let mut result = FormulaSetCnfResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);

        let named_result = self.named_to_db_lambdas(bank, options.problem_type)?;
        result.formulas_named_to_db = named_result.formulas_named_to_db;
        result
            .formula_derivation_ops
            .extend(named_result.formula_derivation_ops);

        let lift_ite_result = self.lift_ites(bank, options.problem_type)?;
        result.formulas_ites_lifted = lift_ite_result.formulas_ites_lifted;
        result
            .formula_derivation_ops
            .extend(lift_ite_result.formula_derivation_ops);

        let lift_let_result = self.lift_lets(bank, options.problem_type)?;
        result.formulas_lets_lifted = lift_let_result.formulas_lets_lifted;
        result
            .formula_derivation_ops
            .extend(lift_let_result.formula_derivation_ops);

        let unfold_def_result = self.unfold_def_symbols(
            archive,
            bank,
            options.problem_type,
            options.higher_order.unfold_only_forms,
        )?;
        result.formulas_def_symbols_unfolded = unfold_def_result.formulas_def_symbols_unfolded;
        result.unfolded_definition_rhs_rewritten =
            unfold_def_result.unfolded_definition_rhs_rewritten;
        result.unfolded_definitions_archived = unfold_def_result.unfolded_definitions_archived;
        result.unfolded_original_definitions_archived =
            unfold_def_result.unfolded_original_definitions_archived;
        result.definition_symbol_applications = unfold_def_result.definition_symbol_applications;
        result
            .formula_derivation_ops
            .extend(unfold_def_result.formula_derivation_ops);

        if options.higher_order.lambda_to_forall {
            let normalize_result = self.lambda_normalize_forall(bank, options.problem_type)?;
            result.formulas_lambda_normalized = normalize_result.formulas_lambda_normalized;
            result
                .formula_derivation_ops
                .extend(normalize_result.formula_derivation_ops);
        }

        if options.fool_unroll {
            let unroll_result = self.unroll_fool(bank)?;
            result.boolean_equalities_replaced = unroll_result.boolean_equalities_replaced;
            result.formulas_fool_unrolled = unroll_result.formulas_unrolled;
            result
                .formula_derivation_ops
                .extend(unroll_result.formula_derivation_ops);
        }

        let simplify_result = self.simplify_with_garbage_collection(bank, true)?;
        result.formulas_simplified = simplify_result.formulas_changed;
        result.term_garbage_collections += simplify_result.term_garbage_collections;
        result.terms_recovered_by_gc += simplify_result.terms_recovered_by_gc;
        result
            .formula_derivation_ops
            .extend(simplify_result.formula_derivation_ops);

        let intro_result = self.introduce_defs(archive, bank, options.def_limit)?;
        result.definitions_introduced = intro_result.definitions_introduced;
        result.definition_applications = intro_result.definition_applications;
        result.definition_formulas_archived = intro_result.archived_definitions;
        result.active_definition_formulas_inserted = intro_result.active_definitions_inserted;
        result.formulas_rewritten_by_defs = intro_result.formulas_rewritten;
        result
            .formula_derivation_ops
            .extend(intro_result.formula_derivation_ops);

        drain_formula_set_to_cnf(
            self,
            &mut FormulaSetCnfDrain {
                archive,
                clauseset,
                bank,
                fresh_vars,
                options,
                old_nodes: &mut old_nodes,
                gc_threshold: &mut gc_threshold,
                result: &mut result,
            },
        )?;

        if options.higher_order.lift_lambdas {
            apply_post_cnf_clause_lambda_lifting(
                clauseset,
                archive,
                bank,
                fresh_vars,
                options.fool_unroll,
                &mut result,
            )?;
        }

        if bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_cnf_garbage(bank, self, archive, clauseset, &mut result);
        }

        Ok(result)
    }

    /// Applies C `FormulaSetCNF2` and emits represented formula proof docs.
    ///
    /// This is the proof-documenting counterpart to [`Self::cnf2_into`]. It
    /// preserves the same supported phase order while threading one
    /// [`ProofDocSession`] through represented formula-level documentation for
    /// named-to-DB conversion, definition-symbol unfolding, lambda
    /// normalization, simplification, definition introduction/application, and
    /// wrapped CNF conversion.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any CNF phase, proof-document rendering, proof
    /// writing, garbage collection bookkeeping, or clause insertion fails.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term or if a malformed
    /// formula violates the same preconditions as the underlying phase wrappers.
    pub fn cnf2_into_with_docs<W: fmt::Write>(
        &mut self,
        doc: &mut WrappedFormulaCnfDocContext<'_, W>,
        archive: &mut Self,
        clauseset: &mut ClauseSet,
        bank: &mut TermBank,
        fresh_vars: &VarBank,
        options: FormulaSetCnfOptions,
    ) -> Result<FormulaSetCnfDocResult, Diagnostic> {
        let mut result = FormulaSetCnfDocResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);
        doc.render_options.problem_type = options.problem_type;

        let named_result = self.named_to_db_lambdas_with_docs(
            &mut *doc.output,
            bank,
            &mut *doc.session,
            doc.render_options,
        )?;
        result.add_preprocess_doc(named_result);

        let lift_ite_result = self.lift_ites(bank, options.problem_type)?;
        result.cnf.add_higher_order_preprocess(&lift_ite_result);

        let lift_let_result = self.lift_lets(bank, options.problem_type)?;
        result.cnf.add_higher_order_preprocess(&lift_let_result);

        let unfold_def_result = self.unfold_def_symbols_with_docs(
            archive,
            &mut *doc.output,
            bank,
            &mut *doc.session,
            doc.render_options,
            options.higher_order.unfold_only_forms,
        )?;
        result.add_preprocess_doc(unfold_def_result);

        if options.higher_order.lambda_to_forall {
            let normalize_result = self.lambda_normalize_forall_with_docs(
                &mut *doc.output,
                bank,
                &mut *doc.session,
                doc.render_options,
            )?;
            result.add_preprocess_doc(normalize_result);
        }

        if options.fool_unroll {
            let unroll_result = self.unroll_fool(bank)?;
            result.cnf.add_fool_unroll(&unroll_result);
        }

        let simplify_result = self.simplify_with_garbage_collection_and_docs(
            &mut *doc.output,
            bank,
            &mut *doc.session,
            doc.render_options.full_terms,
            doc.render_options.problem_type,
            true,
        )?;
        result.add_simplify_doc(simplify_result);

        let intro_result = self.introduce_defs_with_docs(
            archive,
            &mut *doc.output,
            bank,
            &mut *doc.session,
            doc.render_options,
            options.def_limit,
        )?;
        result.add_introduce_defs_doc(intro_result);

        drain_formula_set_to_cnf_with_docs(
            self,
            &mut FormulaSetCnfDocDrain {
                archive,
                clauseset,
                bank,
                fresh_vars,
                options,
                old_nodes: &mut old_nodes,
                gc_threshold: &mut gc_threshold,
                output: &mut *doc.output,
                session: &mut *doc.session,
                render_options: doc.render_options,
                result: &mut result,
            },
        )?;

        if options.higher_order.lift_lambdas {
            apply_post_cnf_clause_lambda_lifting(
                clauseset,
                archive,
                bank,
                fresh_vars,
                options.fool_unroll,
                &mut result.cnf,
            )?;
        }

        if bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_cnf_garbage(bank, self, archive, clauseset, &mut result.cnf);
        }

        Ok(result)
    }

    /// Renders C's `FormulaSetPrint` dispatch in formula insertion order.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a wrapped formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term.
    pub fn print_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        output_format: FormulaPrintFormat,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        for formula in &self.formulas {
            output.push_str(&formula.print_string(
                bank,
                full_terms,
                problem_type,
                output_format,
                keep_input_names,
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Renders C's `FormulaSetPrettyPrintTSTP` in formula insertion order.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if type declarations or a wrapped formula cannot
    /// be rendered.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term.
    pub fn pretty_print_tstp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = if self.is_untyped() {
            String::new()
        } else {
            let mut declarations = Vec::new();
            bank.signature()
                .print_type_decls_tstp(&mut declarations, problem_type)
                .map_err(|_| formula_set_write_error("failed to write TSTP type declarations"))?;
            String::from_utf8(declarations).map_err(|_| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "TSTP type declarations are not valid UTF-8",
                )
            })?
        };

        for formula in &self.formulas {
            output.push_str(&formula.tstp_string_flex(
                bank,
                problem_type,
                FormulaTstpPrintOptions::complete_formula(full_terms, keep_input_names),
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Renders C's `FormulaSetAppEncode` output for this formula set.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if type preloading, declaration printing, or
    /// formula application encoding fails.
    ///
    /// # Panics
    ///
    /// Panics under the same malformed-formula conditions as the underlying
    /// term-formula app-encoding helpers.
    pub fn app_encode_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        self.app_encode_string_with_type_suffixes(bank, problem_type, keep_input_names, false)
    }

    /// Renders C's `FormulaSetAppEncode` output with optional `TermPrintTypes` suffixes.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic under the same conditions as [`Self::app_encode_string`].
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::app_encode_string`].
    pub fn app_encode_string_with_type_suffixes(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        keep_input_names: bool,
        print_types: bool,
    ) -> Result<String, Diagnostic> {
        if self.formulas.is_empty() {
            return Ok(String::new());
        }

        for formula in &self.formulas {
            if formula.formula() != bank.true_term()
                && !tformula_is_prop_true(bank, formula.formula())
            {
                tformula_preload_types(bank, formula.formula())?;
            }
        }

        let mut output = Vec::new();
        bank.signature()
            .type_bank()
            .app_encode_types(&mut output, problem_type, true)
            .map_err(|_| {
                formula_set_write_error("failed to write app-encoded type declarations")
            })?;
        bank.signature()
            .print_app_encoded_decls(&mut output)
            .map_err(|_| {
                formula_set_write_error("failed to write app-encoded symbol declarations")
            })?;
        let mut output = String::from_utf8(output).map_err(|_| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "app-encoded declarations are not valid UTF-8",
            )
        })?;

        for formula in &self.formulas {
            if formula.formula() != bank.true_term()
                && !tformula_is_prop_true(bank, formula.formula())
            {
                output.push_str(&formula.app_encode_string_with_type_suffixes(
                    bank,
                    keep_input_names,
                    print_types,
                )?);
                output.push('\n');
            }
        }
        Ok(output)
    }
}

#[must_use]
pub fn formula_set_stack_cardinality(stack: &[&FormulaSet]) -> i64 {
    stack.iter().map(|set| set.cardinality()).sum()
}

pub fn formula_stack_cond_set_type(stack: &mut [WrappedFormula], type_: FormulaProperties) {
    for formula in stack {
        if formula.query_tptp_type() != CP_TYPE_CONJECTURE || type_ == CP_TYPE_CONJECTURE {
            formula.set_tptp_type(type_);
        }
    }
}

#[must_use]
pub fn formula_set_definition_statistics(
    orig: &FormulaSet,
    arch: &FormulaSet,
    bank: &TermBank,
) -> FormulaDefinitionStatistics {
    let mut num_defs = 0_i32;
    let mut form_defs = 0_i32;
    let mut num_lams = 0_i32;
    let mut has_app_var_lits = false;

    for set in [orig, arch] {
        for formula in set.iter() {
            has_app_var_lits |= formula.has_app_var_literal(bank);
            num_lams =
                num_lams.saturating_add(formula.count_non_top_level_lambdas(bank.signature()));

            if let Some(head) = formula.lambda_definition_head_for_statistics(bank) {
                num_defs = num_defs.saturating_add(1);
                if head.type_().as_ref().is_some_and(type_is_predicate) {
                    form_defs = form_defs.saturating_add(1);
                }
            }
        }
    }

    FormulaDefinitionStatistics {
        num_defs,
        percentage_form_defs: if num_defs == 0 {
            0.0
        } else {
            f64::from(form_defs) / f64::from(num_defs)
        },
        num_lams,
        has_app_var_lits,
    }
}

fn next_entry_id() -> u64 {
    WRAPPED_FORMULA_ENTRY_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_formula_ident() -> i64 {
    loop {
        let current = FORMULA_IDENT_COUNTER.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .unwrap_or_else(|| panic!("formula ident counter overflow"));
        if FORMULA_IDENT_COUNTER
            .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn tformula_count_non_top_level_lambdas(signature: &Signature, formula: &Term) -> i32 {
    let mut stack = vec![(formula.clone(), true)];
    let mut result = 0_i32;

    while let Some((formula, mut is_at_top)) = stack.pop() {
        if !formula.has_lambda_subterm() {
            continue;
        }

        if is_at_top {
            is_at_top = !formula.is_free_var()
                && ((formula.f_code() > 0 && signature.is_logical_symbol(formula.f_code()))
                    || formula.is_lambda());
        } else if formula.is_lambda() {
            result = result.saturating_add(1);
        }

        let start = usize::from(formula.is_phony_app() || formula.is_lambda());
        for index in start..formula.arity() {
            let child = formula
                .argument(index)
                .unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
            if child.has_lambda_subterm() {
                stack.push((child, is_at_top));
            }
        }
    }

    result
}

fn tformula_has_app_var_literal(bank: &TermBank, formula: &Term) -> bool {
    let mut stack = vec![formula.clone()];

    while let Some(formula) = stack.pop() {
        if tformula_is_literal(bank, &formula) {
            let left_has_app_var = formula
                .argument(0)
                .is_some_and(|term| term.is_applied_free_var());
            let right_has_app_var = formula
                .argument(1)
                .is_some_and(|term| term.is_applied_free_var());
            if left_has_app_var || right_has_app_var {
                return true;
            }
        } else if formula.f_code() > 0 && bank.signature().is_logical_symbol(formula.f_code()) {
            stack.extend(formula.argument_clones().into_iter().flatten());
        }
    }

    false
}

fn tformula_annotate_question(bank: &mut TermBank, formula: &Term) -> Result<Term, Diagnostic> {
    let qex_code = bank.signature().qex_code();
    let mut variables = Vec::new();
    let mut handle = formula.clone();
    while handle.f_code() == qex_code && handle.arity() == 2 {
        variables.push(
            handle
                .argument(0)
                .unwrap_or_else(|| panic!("existential quantifier variable is uninitialized")),
        );
        handle = handle
            .argument(1)
            .unwrap_or_else(|| panic!("existential quantifier body is uninitialized"));
    }
    if variables.is_empty() {
        return Ok(formula.clone());
    }

    let answer = answer_lit_alloc(bank, &variables)?;
    let mut result = tformula_fcode_alloc(bank, bank.signature().and_code(), handle, Some(answer))?;
    while let Some(variable) = variables.pop() {
        result = tformula_fcode_alloc(bank, qex_code, variable, Some(result))?;
    }
    Ok(result)
}

fn answer_lit_alloc(bank: &mut TermBank, variables: &[Term]) -> Result<Term, Diagnostic> {
    let answer_payload = bank.alloc_new_skolem(variables, None)?;
    let answer = Term::top_alloc(bank.signature().answer_code(), 1);
    answer.set_type(Some(bank.signature().type_bank().bool_type()));
    answer.set_argument(0, answer_payload);
    let answer = bank.term_top_insert(answer)?;
    let true_term = bank.true_term().clone();
    let neqn_code = bank.signature_mut().get_eqn_code(false);
    tformula_fcode_alloc(bank, neqn_code, answer, Some(true_term))
}

fn tformula_replace_eqn_with_equiv(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if form.is_db_var() || !form.has_eq_neq() || form.is_any_var() {
        return Ok(form.clone());
    }

    let copy = Term::top_copy_without_args(form);
    for (index, arg) in form.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        copy.set_argument(index, tformula_replace_eqn_with_equiv(bank, &arg)?);
    }
    let current = bank.term_top_insert(copy)?;
    rewrite_bool_eqn_root(bank, &current)
}

fn rewrite_bool_eqn_root(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let (eqn_code, neqn_code, equiv_code, xor_code, not_code) = {
        let signature = bank.signature();
        (
            signature.eqn_code(),
            signature.neqn_code(),
            signature.equiv_code(),
            signature.xor_code(),
            signature.not_code(),
        )
    };
    if form.arity() != 2 || (form.f_code() != eqn_code && form.f_code() != neqn_code) {
        return Ok(form.clone());
    }

    let left = form
        .argument(0)
        .unwrap_or_else(|| panic!("Boolean equality left argument is uninitialized"));
    let right = form
        .argument(1)
        .unwrap_or_else(|| panic!("Boolean equality right argument is uninitialized"));
    if !tformula_is_complex_bool(bank, &left) || !tformula_is_complex_bool(bank, &right) {
        return Ok(form.clone());
    }

    let true_term = bank.true_term().clone();
    if form.f_code() == eqn_code {
        if right != true_term {
            return tformula_fcode_alloc(bank, equiv_code, left, Some(right));
        }
        if left != true_term {
            return Ok(left);
        }
    } else if right != true_term {
        return tformula_fcode_alloc(bank, xor_code, left, Some(right));
    } else if left != true_term {
        return tformula_fcode_alloc(bank, not_code, left, None);
    }
    Ok(form.clone())
}

fn formula_set_write_error(message: &'static str) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_lift_lambdas, formula_set_definition_statistics, formula_set_stack_cardinality,
        formula_stack_cond_set_type, wformula_deriv_find_first, wformula_dummy_quote_parent_ref,
        FormulaDefinitionStatistics, FormulaPrintFormat, FormulaProofDocRenderOptions, FormulaSet,
        FormulaSetCnfOptions, FormulaTstpClauseMode, FormulaTstpPrintOptions, WrappedFormula,
        WrappedFormulaCnfDocContext,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_IGNORE_PROPS, CP_INPUT_FORMULA, CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
        CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION,
    };
    use crate::clauses::clausefunc::{
        tformula_clause_encode, tformula_decode_polarity, tformula_quantor_alloc,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        DerivationEntry, FormulaDerivationRef, DC_ANNO_QUESTION, DC_APPLY_DEF,
        DC_DIST_DISJUNCTIONS, DC_EQ_TO_EQ, DC_FOF_QUOTE, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL,
        DC_INTRO_DEF, DC_LIFT_ITE, DC_LIFT_LAMBDAS, DC_NEGATE_CONJECTURE, DC_SPLIT_CONJUNCT,
        DC_SPLIT_EQUIV,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{
        ProofDocOutputFormat, ProofDocSession, ProofDocWriteResult,
    };
    use crate::terms::lambda::{apply_terms as lambda_apply_terms, close_with_db_var};
    use crate::terms::signature::{
        Signature, SIG_DB_LAMBDA_CODE, SIG_ITE_CODE, SIG_LET_CODE, SIG_NAMED_LAMBDA_CODE,
        SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
    };
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type, ST_INTEGER};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::{term_has_f_code, term_standard_weight};
    use crate::terms::termtypes::{
        DerefType, Term, TP_CHECK_FLAG, TP_NEG_POLARITY, TP_POS_POLARITY,
    };
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_with_type(bank, name, &type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_types(bank, name, arg, &type_, &type_)
    }

    fn typed_unary_with_types(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        arg_type: &crate::terms::simpletypes::Type,
        ret_type: &crate::terms::simpletypes::Type,
    ) -> Term {
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), ret_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(ret_type.clone()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let arg_type = arg.type_().expect("predicate argument must have a type");
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, predicate_type)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn typed_predicate_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn bool_ite(bank: &mut TermBank, condition: &Term, if_true: &Term, if_false: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(SIG_ITE_CODE, 3);
        term.set_type(Some(type_));
        term.set_argument(0, condition.clone());
        term.set_argument(1, if_true.clone());
        term.set_argument(2, if_false.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn let_term(bank: &mut TermBank, definitions: &[Term], body: &Term) -> Term {
        let type_ = body.type_().expect("$let body must have a type");
        let term = Term::top_alloc(SIG_LET_CODE, definitions.len() + 1);
        term.set_type(Some(type_));
        for (index, definition) in definitions.iter().enumerate() {
            term.set_argument(index, definition.clone());
        }
        term.set_argument(definitions.len(), body.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn db_lambda_equality(bank: &mut TermBank, prefix: &str) -> Term {
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(bank, &format!("{prefix}_f"), &unary_type);
        let g = typed_const_with_type(bank, &format!("{prefix}_g"), &unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let left_body = lambda_apply_terms(bank, &f, std::slice::from_ref(&db0)).unwrap();
        let left_lambda = close_with_db_var(bank, &i_type, &left_body).unwrap();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        bool_binary_with_code(bank, eqn_code, &left_lambda, &g)
    }

    fn named_lambda_equality(bank: &mut TermBank, prefix: &str) -> Term {
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(bank, &format!("{prefix}_f"), &unary_type);
        let g = typed_const_with_type(bank, &format!("{prefix}_g"), &unary_type);
        let x = typed_var(bank, -509);
        let left_body = lambda_apply_terms(bank, &f, std::slice::from_ref(&x)).unwrap();
        let left_lambda =
            tformula_quantor_alloc(bank, SIG_NAMED_LAMBDA_CODE, &x, &left_body).unwrap();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        bool_binary_with_code(bank, eqn_code, &left_lambda, &g)
    }

    fn unfolding_definition_fixture(
        bank: &mut TermBank,
        prefix: &str,
    ) -> (WrappedFormula, Term, Term) {
        let x = typed_var(bank, -711);
        let a = typed_const(bank, &format!("{prefix}_a"));
        let p_x = typed_unary_predicate(bank, &format!("{prefix}_p"), &x);
        let q_x = typed_unary_predicate(bank, &format!("{prefix}_q"), &x);
        let p_a = typed_unary_predicate(bank, &format!("{prefix}_p"), &a);
        let q_a = typed_unary_predicate(bank, &format!("{prefix}_q"), &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let true_term = bank.true_term().clone();
        let lhs_formula = bool_binary_with_code(bank, eqn_code, &p_x, &true_term);
        let definition_formula = bool_binary_with_code(bank, equiv_code, &lhs_formula, &q_x);
        let mut definition = WrappedFormula::wt_formula_alloc(definition_formula);
        definition.set_prop(CP_IS_LAMBDA_DEF);
        (definition, p_a, q_a)
    }

    fn c_complex_bool_shape(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let payload = typed_const(bank, name);
        let term = Term::top_alloc(SIG_TRUE_CODE, 1);
        term.set_type(Some(type_));
        term.set_argument(0, payload);
        bank.term_top_insert(term).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn quantified_with_code(
        bank: &mut TermBank,
        f_code: i64,
        variable: &Term,
        body: &Term,
    ) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, variable.clone());
        term.set_argument(1, body.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn phony_app(bank: &mut TermBank, head: &Term, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(type_));
        term.set_argument(0, head.clone());
        term.set_argument(1, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    #[test]
    fn wrapped_formula_allocation_defaults_and_ids_match_c_shape() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_atom");
        let default = WrappedFormula::default_alloc();

        assert_eq!(default.properties(), CP_IGNORE_PROPS);
        assert!(!default.is_clause());
        assert_eq!(default.ident(), 0);
        assert_eq!(default.info(), None);
        assert_eq!(default.get_id(false), "c_0_0");

        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        assert!(formula.get_id(false).starts_with("i_0_"));
        formula.set_info(Some(ClauseInfo::new(
            Some("input_formula_name"),
            Some("problem.p"),
            7,
            3,
        )));
        assert_eq!(formula.get_id(true), "input_formula_name");
        assert!(formula.get_id(false).starts_with("i_0_"));
    }

    #[test]
    fn wrapped_formula_flat_copy_preserves_formula_state_but_not_source_info() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_copy_atom");
        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        formula.set_prop(CP_INPUT_FORMULA);
        formula.set_is_clause(true);
        formula.set_info(Some(ClauseInfo::new(Some("copy_source"), None, 1, 1)));

        let copied = formula.flat_copy();

        assert_ne!(copied.entry_id(), formula.entry_id());
        assert_eq!(copied.ident(), formula.ident());
        assert_eq!(copied.properties(), formula.properties());
        assert!(copied.is_clause());
        assert_eq!(copied.info(), None);
        assert_eq!(copied.formula(), formula.formula());
    }

    #[test]
    fn formula_set_insert_extract_and_move_preserve_c_list_order() {
        let mut bank = test_bank();
        let first = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_first"));
        let second = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_second"));
        let third = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_third"));
        let first_id = first.entry_id();
        let second_id = second.entry_id();
        let third_id = third.entry_id();
        let mut set = FormulaSet::new();

        assert!(set.is_empty());
        assert_eq!(set.insert(first), first_id);
        assert_eq!(set.insert(second), second_id);
        assert_eq!(set.cardinality(), 2);
        assert_eq!(
            set.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );

        assert_eq!(set.extract_first().unwrap().entry_id(), first_id);
        assert_eq!(set.cardinality(), 1);
        assert_eq!(set.extract_entry(second_id).unwrap().entry_id(), second_id);
        assert!(set.extract_first().is_none());

        let mut from = FormulaSet::new();
        from.insert(WrappedFormula::wt_formula_alloc(typed_const(
            &mut bank,
            "move_first",
        )));
        from.insert(third);
        let moved_first_id = from.iter().next().unwrap().entry_id();
        let mut to = FormulaSet::new();

        assert!(to.move_formula_from(&mut from, third_id));
        assert_eq!(from.cardinality(), 1);
        assert_eq!(to.cardinality(), 1);
        assert_eq!(to.iter().next().unwrap().entry_id(), third_id);

        assert_eq!(to.insert_set(&mut from), 1);
        assert!(from.is_empty());
        assert_eq!(
            to.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![third_id, moved_first_id]
        );
        assert_eq!(formula_set_stack_cardinality(&[&to, &from]), 2);
    }

    #[test]
    fn wformula_deriv_find_first_follows_dummy_quote_cascade() {
        let mut bank = test_bank();
        let original_term = typed_const(&mut bank, "wform_quote_original");
        let quote_term = typed_const(&mut bank, "wform_quote_copy");
        let second_quote_term = typed_const(&mut bank, "wform_second_quote_copy");

        let original = WrappedFormula::wt_formula_alloc(original_term);
        let original_ref = FormulaDerivationRef::new(original.ident());
        let mut quote = WrappedFormula::wt_formula_alloc(quote_term);
        quote.push_formula_derivation(DC_FOF_QUOTE, Some(original_ref), None);
        let quote_ref = FormulaDerivationRef::new(quote.ident());
        let mut second_quote = WrappedFormula::wt_formula_alloc(second_quote_term);
        second_quote.push_formula_derivation(DC_FOF_QUOTE, Some(quote_ref), None);

        assert_eq!(wformula_dummy_quote_parent_ref(&quote), Some(original_ref));
        let formulas = [&original, &quote, &second_quote];
        let first = wformula_deriv_find_first(&second_quote, |parent| {
            formulas
                .iter()
                .copied()
                .find(|formula| FormulaDerivationRef::new(formula.ident()) == parent)
        });

        assert!(std::ptr::eq(
            std::ptr::from_ref(first),
            std::ptr::from_ref(&original)
        ));
    }

    #[test]
    fn wformula_deriv_find_first_stops_when_parent_is_missing_or_cyclic() {
        let mut bank = test_bank();
        let mut missing_parent_quote =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "wform_missing_parent_quote"));
        missing_parent_quote.push_formula_derivation(
            DC_FOF_QUOTE,
            Some(FormulaDerivationRef::new(9_999)),
            None,
        );
        let first = wformula_deriv_find_first(&missing_parent_quote, |_| None);
        assert!(std::ptr::eq(
            std::ptr::from_ref(first),
            std::ptr::from_ref(&missing_parent_quote)
        ));

        let mut cyclic = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "wform_cyclic"));
        let cyclic_ref = FormulaDerivationRef::new(cyclic.ident());
        cyclic.push_formula_derivation(DC_FOF_QUOTE, Some(cyclic_ref), None);

        let first =
            wformula_deriv_find_first(&cyclic, |parent| (parent == cyclic_ref).then_some(&cyclic));
        assert!(std::ptr::eq(
            std::ptr::from_ref(first),
            std::ptr::from_ref(&cyclic)
        ));
    }

    #[test]
    fn formula_set_archive_moves_originals_and_replaces_flat_copies() {
        let mut bank = test_bank();
        let first_term = typed_const(&mut bank, "archive_first");
        let second_term = typed_const(&mut bank, "archive_second");
        let mut first = WrappedFormula::wt_formula_alloc(first_term.clone());
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_info(Some(ClauseInfo::new(Some("archive_name"), None, 1, 1)));
        first.push_formula_derivation(DC_FOF_SIMPLIFY, None, None);
        let first_entry = first.entry_id();
        let first_source = FormulaDerivationRef::new(first.ident());
        let mut second = WrappedFormula::wt_formula_alloc(second_term.clone());
        second.set_is_clause(true);
        let second_entry = second.entry_id();
        let second_source = FormulaDerivationRef::new(second.ident());
        let mut set = FormulaSet::new();
        set.insert(first);
        set.insert(second);
        let mut archive = FormulaSet::new();

        let result = set.archive_into(&mut archive);

        assert_eq!(result.formulas_archived, 2);
        assert_eq!(
            result.quoted_formula_sources,
            vec![first_source, second_source]
        );
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_FOF_QUOTE, DC_FOF_QUOTE]
        );
        assert_eq!(
            archive
                .iter()
                .map(WrappedFormula::entry_id)
                .collect::<Vec<_>>(),
            vec![first_entry, second_entry]
        );
        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(
            archived[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(archived[1].derivation_entries(), &[]);
        let copied = set.iter().collect::<Vec<_>>();
        assert_eq!(copied.len(), 2);
        assert_ne!(copied[0].entry_id(), first_entry);
        assert_ne!(copied[1].entry_id(), second_entry);
        assert_eq!(copied[0].ident(), first_source.ident());
        assert_eq!(copied[1].ident(), second_source.ident());
        assert_eq!(copied[0].query_tptp_type(), CP_TYPE_AXIOM);
        assert!(copied[1].is_clause());
        assert_eq!(copied[0].info(), None);
        assert_eq!(copied[0].formula(), &first_term);
        assert_eq!(copied[1].formula(), &second_term);
        assert_eq!(
            copied[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(first_source)
            ]
        );
        assert_eq!(
            copied[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(second_source)
            ]
        );
    }

    #[test]
    fn formula_set_del_term_props_clears_nested_terms_and_ignores_default_wrappers() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "del_props_left");
        let right = typed_const(&mut bank, "del_props_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        formula.set_prop(TP_CHECK_FLAG);
        left.set_prop(TP_CHECK_FLAG | TP_POS_POLARITY);
        right.set_prop(TP_NEG_POLARITY);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::default_alloc());
        set.insert(WrappedFormula::wt_formula_alloc(formula.clone()));

        set.del_term_props(TP_CHECK_FLAG | TP_POS_POLARITY);

        assert!(!formula.query_prop(TP_CHECK_FLAG));
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!left.query_prop(TP_POS_POLARITY));
        assert!(right.query_prop(TP_NEG_POLARITY));
        assert!(!right.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn formula_set_doc_initial_suppresses_below_level_two_without_reidentifying() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "doc_suppress_left");
        let right = typed_const(&mut bank, "doc_suppress_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let wrapped = WrappedFormula::wt_formula_alloc(formula);
        let original_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .doc_initial(
                &mut rendered,
                &mut bank,
                &mut session,
                true,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.formulas_seen, 1);
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        assert_eq!(set.iter().next().unwrap().ident(), original_ident);
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn formula_set_doc_initial_prints_pcl_initials_and_assigns_ids_in_order() {
        let mut bank = test_bank();
        let first_left = typed_const(&mut bank, "doc_first_left");
        let first_right = typed_const(&mut bank, "doc_first_right");
        let second_left = typed_const(&mut bank, "doc_second_left");
        let second_right = typed_const(&mut bank, "doc_second_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first_formula = bool_binary_with_code(&mut bank, eqn_code, &first_left, &first_right);
        let second_formula =
            bool_binary_with_code(&mut bank, eqn_code, &second_left, &second_right);
        let mut first = WrappedFormula::wt_formula_alloc(first_formula);
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_info(Some(ClauseInfo::new(
            Some("doc_first"),
            Some("doc.p"),
            2,
            3,
        )));
        let first_body = first
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let mut second = WrappedFormula::wt_formula_alloc(second_formula);
        second.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        second.set_info(Some(ClauseInfo::new(Some("doc_second"), None, 4, 5)));
        let second_body = second
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let mut set = FormulaSet::new();
        set.insert(first);
        set.insert(second);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .doc_initial(
                &mut rendered,
                &mut bank,
                &mut session,
                true,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.formulas_seen, 2);
        assert_eq!(
            result.write_results,
            vec![
                ProofDocWriteResult::printed(),
                ProofDocWriteResult::printed()
            ]
        );
        assert_eq!(
            rendered,
            format!(
                "     1 : :{first_body} : initial(\"doc.p\", doc_first)\n     2 : neg:{second_body} : initial(unknown, doc_second)\n"
            )
        );
        assert_eq!(
            set.iter().map(WrappedFormula::ident).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(session.id_source.current_ident(), 2);
    }

    #[test]
    fn formula_set_doc_initial_prints_tstp_initials() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "doc_tstp_left");
        let right = typed_const(&mut bank, "doc_tstp_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(Some("doc_tstp"), Some("doc.p"), 8, 9)));
        let body = wrapped
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .doc_initial(
                &mut rendered,
                &mut bank,
                &mut session,
                true,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.formulas_seen, 1);
        assert_eq!(result.write_results, vec![ProofDocWriteResult::printed()]);
        assert_eq!(
            rendered,
            format!("fof(c_0_1, axiom, {body}, file('doc.p', doc_tstp)).\n")
        );
        assert_eq!(set.iter().next().unwrap().ident(), 1);
    }

    #[test]
    fn formula_set_type_queries_and_counts_match_c_macros() {
        let mut bank = test_bank();
        let mut axiom = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_axiom"));
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut hypothesis = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_hyp"));
        hypothesis.set_tptp_type(CP_TYPE_HYPOTHESIS);
        let mut conjecture = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_conj"));
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut question =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_question"));
        question.set_tptp_type(CP_TYPE_QUESTION);
        let mut set = FormulaSet::new();
        set.insert(axiom);
        set.insert(hypothesis);
        set.insert(conjecture);
        set.insert(question);

        let (count, conjectures, rest) = set.split_conjectures();
        assert_eq!(count, 2);
        assert_eq!(conjectures.len(), 2);
        assert_eq!(rest.len(), 2);

        let mut hypothesis_count = 5;
        assert_eq!(set.count_conjectures(&mut hypothesis_count), 2);
        assert_eq!(hypothesis_count, 6);

        let mut stack = set.iter().cloned().collect::<Vec<_>>();
        formula_stack_cond_set_type(&mut stack, CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[0].query_tptp_type(), CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[2].query_tptp_type(), CP_TYPE_CONJECTURE);
        formula_stack_cond_set_type(&mut stack, CP_TYPE_CONJECTURE);
        assert!(stack
            .iter()
            .all(|formula| formula.query_tptp_type() == CP_TYPE_CONJECTURE));
    }

    #[test]
    fn formula_set_weight_fcode_and_symbol_queries_use_formula_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "formula_set_a");
        let f_a = typed_unary(&mut bank, "formula_set_f", &a);
        let mut normal = WrappedFormula::wt_formula_alloc(f_a.clone());
        normal.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut lambda_def = WrappedFormula::wt_formula_alloc(a.clone());
        lambda_def.set_prop(CP_IS_LAMBDA_DEF);
        let normal_id = normal.entry_id();
        let mut set = FormulaSet::new();
        set.insert(normal);
        set.insert(lambda_def);

        assert_eq!(set.standard_weight(), term_standard_weight(&f_a));
        let mut result = Vec::new();
        assert_eq!(set.collect_f_code(f_a.f_code(), &mut result), 1);
        assert_eq!(result, vec![normal_id]);
        assert_eq!(set.conjecture_order(bank.signature()), 0);
        assert!(!set.has_interpreted_symbol());
        assert!(set.is_untyped());

        let int_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(ST_INTEGER));
        let int_code = bank.signature_mut().insert_id("formula_set_int", 0, false);
        bank.signature_mut()
            .declare_final_type(int_code, int_type)
            .unwrap();
        let int_term = bank.create_const_term(int_code).unwrap();
        set.insert(WrappedFormula::wt_formula_alloc(int_term));
        assert!(set.has_interpreted_symbol());
        assert!(!set.is_untyped());
    }

    #[test]
    fn formula_set_mark_polarity_marks_each_wrapped_formula_like_c() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pol_first");
        let second = typed_const(&mut bank, "pol_second");
        let third = typed_const(&mut bank, "pol_third");
        let fourth = typed_const(&mut bank, "pol_fourth");
        let fifth = typed_const(&mut bank, "pol_fifth");
        let sixth = typed_const(&mut bank, "pol_sixth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let impl_code = bank.signature().impl_code();
        let or_code = bank.signature().or_code();
        let first_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let second_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let third_atom = bool_binary_with_code(&mut bank, eqn_code, &fifth, &sixth);
        let left_disjunction = bool_binary_with_code(&mut bank, or_code, &first_atom, &second_atom);
        let implication =
            bool_binary_with_code(&mut bank, impl_code, &left_disjunction, &third_atom);
        let disjunction = bool_binary_with_code(&mut bank, or_code, &second_atom, &third_atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::default_alloc());
        set.insert(WrappedFormula::wt_formula_alloc(implication.clone()));
        set.insert(WrappedFormula::wt_formula_alloc(disjunction.clone()));

        set.mark_polarity(&bank);

        assert_eq!(tformula_decode_polarity(&implication), 1);
        assert_eq!(tformula_decode_polarity(&left_disjunction), -1);
        assert_eq!(tformula_decode_polarity(&disjunction), 1);
    }

    #[test]
    fn wrapped_formula_return_f_codes_preserves_order_and_skips_phony_apps() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fc_first");
        let second = typed_const(&mut bank, "fc_second");
        let third = typed_const(&mut bank, "fc_third");
        let var = typed_var(&bank, -41);
        let applied = phony_app(&mut bank, &var, &third);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let pair = bool_binary_with_code(&mut bank, and_code, &first, &applied);
        let formula = bool_binary_with_code(&mut bank, or_code, &pair, &second);
        let wrapped = WrappedFormula::wt_formula_alloc(formula);
        let mut f_codes = vec![999_999];

        assert_eq!(wrapped.return_f_codes(&mut f_codes), 5);
        assert_eq!(
            f_codes,
            vec![
                999_999,
                or_code,
                and_code,
                first.f_code(),
                third.f_code(),
                second.f_code(),
            ]
        );
        assert_eq!(wrapped.symbol_diversity(), 5);
        assert!(!f_codes.contains(&SIG_PHONY_APP_CODE));
    }

    #[test]
    fn wrapped_formula_get_lambda_defined_symbol_matches_c_shapes() {
        let mut bank = test_bank();
        let head = typed_const(&mut bank, "lambda_defined_head");
        let rhs = typed_const(&mut bank, "lambda_defined_rhs");
        let quantified_var = typed_var(&bank, -42);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let qall_code = bank.signature().qall_code();
        let equiv_code = bank.signature().equiv_code();
        let equation = bool_binary_with_code(&mut bank, eqn_code, &head, &rhs);
        let quantified = quantified_with_code(&mut bank, qall_code, &quantified_var, &equation);
        let mut equation_definition = WrappedFormula::wt_formula_alloc(quantified);
        equation_definition.set_prop(CP_IS_LAMBDA_DEF);

        assert_eq!(
            equation_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let true_term = bank.true_term().clone();
        let predicate_equation = bool_binary_with_code(&mut bank, eqn_code, &head, &true_term);
        let true_formula = bank.true_term().clone();
        let equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut equivalence_definition = WrappedFormula::wt_formula_alloc(equivalence);
        equivalence_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            equivalence_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let mut untagged = equivalence_definition.flat_copy();
        untagged.del_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(untagged.get_lambda_defined_symbol(bank.signature()), None);
    }

    #[test]
    fn wrapped_formula_app_encode_renders_c_role_and_id_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_app_a");
        let f_a = typed_unary(&mut bank, "wf_app_f", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &f_a, &a);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(Some("wf_app_named"), None, 1, 1)));

        let rendered = wrapped.app_encode_string(&mut bank, true).unwrap();

        assert!(rendered.starts_with("tff(wf_app_named, axiom, "));
        assert!(rendered.ends_with(")."));
        assert!(rendered.contains("wf_app_a"));
    }

    #[test]
    fn wrapped_formula_tptp_and_tstp_printers_match_c_roles_and_spacing() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_print_a");
        let b = typed_const(&mut bank, "wf_print_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(Some("wf_print_neg"), None, 7, 3)));

        let legacy_output = wrapped
            .tptp_string(&mut bank, true, ProblemType::FirstOrder, true)
            .unwrap();
        let tstp_output = wrapped
            .tstp_string(&mut bank, true, true, ProblemType::FirstOrder, true)
            .unwrap();
        let incomplete_tstp = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::open_formula(true, true),
            )
            .unwrap();

        assert!(legacy_output.starts_with("input_formula(wf_print_neg,conjecture,"));
        assert!(legacy_output.contains("wf_print_a"));
        assert!(legacy_output.contains("wf_print_b"));
        assert!(legacy_output.ends_with(")."));
        assert!(tstp_output.starts_with("fof(wf_print_neg, negated_conjecture, "));
        assert!(tstp_output.contains("wf_print_a=wf_print_b"));
        assert!(tstp_output.ends_with(")."));
        assert!(incomplete_tstp.starts_with("fof(wf_print_neg, negated_conjecture, "));
        assert!(!incomplete_tstp.ends_with(")."));
    }

    #[test]
    fn wrapped_formula_tptp_prints_clause_backed_formula_payload() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_tptp_clause_a");
        let b = typed_const(&mut bank, "wf_tptp_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(Some("wf_tptp_clause"), None, 1, 1)));

        let rendered = wrapped
            .tptp_string(&mut bank, true, ProblemType::FirstOrder, true)
            .unwrap();

        assert_eq!(
            rendered,
            "input_formula(wf_tptp_clause,conjecture,(equal(wf_tptp_clause_a, wf_tptp_clause_b)|~equal(wf_tptp_clause_b, wf_tptp_clause_a)))."
        );
    }

    #[test]
    fn wrapped_formula_clause_conversion_preserves_properties_and_source_info() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_clause_a");
        let b = typed_const(&mut bank, "wf_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(
            Some("wrapped_clause"),
            Some("source.p"),
            4,
            2,
        )));

        let converted = wrapped.form_clause_to_clause(&mut bank).unwrap();

        assert_eq!(converted.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(converted.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            converted.info().and_then(ClauseInfo::name),
            Some("wrapped_clause")
        );
        assert_eq!(
            converted.info().and_then(ClauseInfo::source),
            Some("source.p")
        );
        assert_eq!(converted.literal_number(), 2);
        assert_eq!(converted.positive_literal_count(), 1);
        assert_eq!(converted.negative_literal_count(), 1);
    }

    #[test]
    fn wrapped_formula_form_clause_alloc_preserves_clause_shape_and_metadata() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_alloc_clause_a");
        let b = typed_const(&mut bank, "wf_alloc_clause_b");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &b, true)]));
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause.set_prop(CP_INPUT_FORMULA);
        clause.set_info(Some(ClauseInfo::new(
            Some("allocated_clause"),
            Some("input.p"),
            8,
            5,
        )));

        let wrapped =
            WrappedFormula::form_clause_alloc(&mut bank, clause, ProblemType::FirstOrder).unwrap();
        let converted = wrapped.form_clause_to_clause(&mut bank).unwrap();

        assert!(wrapped.is_clause());
        assert_eq!(wrapped.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(wrapped.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            wrapped.info().and_then(ClauseInfo::name),
            Some("allocated_clause")
        );
        assert_eq!(converted.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            converted.info().and_then(ClauseInfo::source),
            Some("input.p")
        );
        assert_eq!(converted.literal_number(), 1);
    }

    #[test]
    fn wrapped_formula_of_clause_closes_formula_and_drops_clause_metadata() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "wf_of_clause_a");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &x, &a, true)]));
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause.set_prop(CP_INPUT_FORMULA);
        clause.set_info(Some(ClauseInfo::new(Some("source_clause"), None, 1, 1)));

        let wrapped = WrappedFormula::of_clause(&mut bank, &clause, ProblemType::FirstOrder)
            .expect("clause can be encoded as a closed formula");
        let rendered = wrapped
            .tstp_string(&mut bank, true, true, ProblemType::FirstOrder, true)
            .unwrap();

        assert!(!wrapped.is_clause());
        assert_eq!(wrapped.properties(), CP_IGNORE_PROPS);
        assert_eq!(wrapped.info(), None);
        assert!(rendered.contains("plain, ![X1]:(X1=wf_of_clause_a)"));
    }

    #[test]
    fn wrapped_formula_print_string_dispatches_clause_wrappers_like_clause_print() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_print_clause_a");
        let b = typed_const(&mut bank, "wf_print_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(
            Some("wrapped_print_clause"),
            None,
            1,
            1,
        )));

        let lop = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Lop,
                true,
            )
            .unwrap();
        let old_tptp_output = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tptp,
                true,
            )
            .unwrap();
        let structured_output = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tstp,
                true,
            )
            .unwrap();

        assert_eq!(
            lop,
            "?- wf_print_clause_a!=wf_print_clause_b, wf_print_clause_b=wf_print_clause_a."
        );
        assert!(old_tptp_output.starts_with("input_clause("));
        assert!(old_tptp_output.contains(",conjecture,["));
        assert!(old_tptp_output.contains("++equal(wf_print_clause_a, wf_print_clause_b)"));
        assert!(old_tptp_output.contains("--equal(wf_print_clause_b, wf_print_clause_a)"));
        assert!(structured_output.starts_with("cnf("));
        assert!(structured_output.contains(", negated_conjecture, "));
        assert!(structured_output.contains("(wf_print_clause_a=wf_print_clause_b|"));
        assert!(structured_output.ends_with(")."));
    }

    #[test]
    fn wrapped_formula_tstp_flex_handles_clause_as_formula_and_core_modes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_flex_clause_a");
        let b = typed_const(&mut bank, "wf_flex_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(Some("flex_clause"), None, 1, 1)));

        let as_formula = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::complete_formula(true, true),
            )
            .unwrap();
        let as_clause_core = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::complete_formula(true, true)
                    .with_clause_mode(FormulaTstpClauseMode::AsClauseCore),
            )
            .unwrap();

        assert!(as_formula.starts_with("fof(flex_clause, negated_conjecture, "));
        assert!(as_formula.contains("wf_flex_clause_a=wf_flex_clause_b"));
        assert!(as_formula.ends_with(")."));
        assert!(as_clause_core.starts_with("cnf(flex_clause, negated_conjecture, ("));
        assert!(as_clause_core.contains("wf_flex_clause_a=wf_flex_clause_b|"));
        assert!(as_clause_core.ends_with("))."));
    }

    #[test]
    fn wrapped_formula_simplify_updates_formula_like_c() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "wf_simpl_left");
        let atom_right = typed_const(&mut bank, "wf_simpl_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let disjunction = bool_binary_with_code(&mut bank, or_code, &false_formula, &atom);
        let mut wrapped = WrappedFormula::wt_formula_alloc(disjunction);

        assert!(wrapped.simplify(&mut bank).unwrap());
        assert_eq!(wrapped.formula(), &atom);
        assert_eq!(
            wrapped.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert!(!wrapped.simplify(&mut bank).unwrap());
    }

    #[test]
    fn formula_set_simplify_counts_changes_and_preserves_order() {
        let mut bank = test_bank();
        let changed_left = typed_const(&mut bank, "set_simpl_changed_left");
        let changed_right = typed_const(&mut bank, "set_simpl_changed_right");
        let stable_left = typed_const(&mut bank, "set_simpl_stable_left");
        let stable_right = typed_const(&mut bank, "set_simpl_stable_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let changed_atom =
            bool_binary_with_code(&mut bank, eqn_code, &changed_left, &changed_right);
        let stable_atom = bool_binary_with_code(&mut bank, eqn_code, &stable_left, &stable_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let changed_formula =
            bool_binary_with_code(&mut bank, or_code, &false_formula, &changed_atom);
        let mut changed = WrappedFormula::wt_formula_alloc(changed_formula);
        let changed_entry = changed.entry_id();
        changed.set_tptp_type(CP_TYPE_AXIOM);
        let stable = WrappedFormula::wt_formula_alloc(stable_atom.clone());
        let stable_entry = stable.entry_id();
        let mut set = FormulaSet::new();
        set.insert(changed);
        set.insert(stable);

        let result = set.simplify(&mut bank).unwrap();

        assert_eq!(result.formulas_changed, 1);
        assert_eq!(result.term_garbage_collections, 0);
        assert_eq!(result.terms_recovered_by_gc, 0);
        assert_eq!(result.formula_derivation_ops, vec![DC_FOF_SIMPLIFY]);
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].entry_id(), changed_entry);
        assert_eq!(formulas[0].query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(formulas[0].formula(), &changed_atom);
        assert_eq!(
            formulas[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(formulas[1].entry_id(), stable_entry);
        assert_eq!(formulas[1].formula(), &stable_atom);
        assert_eq!(formulas[1].derivation_entries(), &[]);
    }

    #[test]
    fn formula_set_simplify_with_docs_prints_changed_formula_modifications() {
        let mut bank = test_bank();
        let changed_left = typed_const(&mut bank, "set_simpl_doc_changed_left");
        let changed_right = typed_const(&mut bank, "set_simpl_doc_changed_right");
        let stable_left = typed_const(&mut bank, "set_simpl_doc_stable_left");
        let stable_right = typed_const(&mut bank, "set_simpl_doc_stable_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let changed_atom =
            bool_binary_with_code(&mut bank, eqn_code, &changed_left, &changed_right);
        let stable_atom = bool_binary_with_code(&mut bank, eqn_code, &stable_left, &stable_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let changed_formula =
            bool_binary_with_code(&mut bank, or_code, &false_formula, &changed_atom);
        let mut expected_body = WrappedFormula::wt_formula_alloc(changed_atom.clone());
        expected_body.set_tptp_type(CP_TYPE_AXIOM);
        let expected_body = expected_body
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let mut changed = WrappedFormula::wt_formula_alloc(changed_formula);
        changed.set_tptp_type(CP_TYPE_AXIOM);
        changed.set_prop(CP_INPUT_FORMULA);
        let old_changed_ident = changed.ident();
        let stable = WrappedFormula::wt_formula_alloc(stable_atom.clone());
        let old_stable_ident = stable.ident();
        let mut set = FormulaSet::new();
        set.insert(changed);
        set.insert(stable);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .simplify_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                true,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.simplify.formulas_changed, 1);
        assert_eq!(
            result.simplify.formula_derivation_ops,
            vec![DC_FOF_SIMPLIFY]
        );
        assert_eq!(result.write_results, vec![ProofDocWriteResult::printed()]);
        assert_eq!(
            rendered,
            format!("     1 : :{expected_body} : fof_simplification({old_changed_ident})\n")
        );
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].ident(), 1);
        assert!(!formulas[0].query_prop(CP_INPUT_FORMULA));
        assert_eq!(formulas[0].formula(), &changed_atom);
        assert_eq!(
            formulas[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(formulas[1].ident(), old_stable_ident);
        assert_eq!(formulas[1].formula(), &stable_atom);
        assert_eq!(formulas[1].derivation_entries(), &[]);
        assert_eq!(session.id_source.current_ident(), 1);
    }

    #[test]
    fn formula_set_simplify_with_docs_suppresses_but_keeps_c_property_side_effects() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "set_simpl_doc_suppress_left");
        let right = typed_const(&mut bank, "set_simpl_doc_suppress_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let changed_formula = bool_binary_with_code(&mut bank, or_code, &false_formula, &atom);
        let mut changed = WrappedFormula::wt_formula_alloc(changed_formula);
        changed.set_tptp_type(CP_TYPE_AXIOM);
        changed.set_prop(CP_INPUT_FORMULA);
        let old_ident = changed.ident();
        let mut set = FormulaSet::new();
        set.insert(changed);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .simplify_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                true,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.simplify.formulas_changed, 1);
        assert_eq!(
            result.simplify.formula_derivation_ops,
            vec![DC_FOF_SIMPLIFY]
        );
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        let formula = set.iter().next().unwrap();
        assert_eq!(formula.ident(), old_ident);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.formula(), &atom);
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn formula_set_simplify_with_gc_collects_after_node_growth() {
        let mut bank = test_bank();
        let dropped = typed_const(&mut bank, "set_simpl_gc_dropped");
        let a = typed_const(&mut bank, "set_simpl_gc_a");
        let b = typed_const(&mut bank, "set_simpl_gc_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let equiv_code = bank.signature().equiv_code();
        let equiv_false = bool_binary_with_code(&mut bank, equiv_code, &atom, &false_prop);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(equiv_false));

        let result = set
            .simplify_with_garbage_collection(&mut bank, true)
            .unwrap();

        assert_eq!(result.formulas_changed, 1);
        assert!(result.term_garbage_collections >= 1);
        assert!(result.terms_recovered_by_gc >= 1);
        assert!(bank.find(&dropped).is_none());
        let simplified = set.iter().next().unwrap().formula().clone();
        assert_eq!(simplified.f_code(), neqn_code);
        assert!(bank.find(&simplified).is_some());
        assert_eq!(
            set.iter().next().unwrap().derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
    }

    fn assert_answer_annotation_shape(bank: &TermBank, annotated: &Term, expected_body: &Term) {
        let qex_code = bank.signature().qex_code();
        assert_eq!(annotated.f_code(), qex_code);
        let first_var = annotated.argument(0).unwrap();
        let second_quantifier = annotated.argument(1).unwrap();
        assert_eq!(second_quantifier.f_code(), qex_code);
        let second_var = second_quantifier.argument(0).unwrap();
        let conjunction = second_quantifier.argument(1).unwrap();
        assert_eq!(conjunction.f_code(), bank.signature().and_code());
        assert_eq!(conjunction.argument(0).as_ref(), Some(expected_body));
        let answer_literal = conjunction.argument(1).unwrap();
        assert_eq!(answer_literal.f_code(), bank.signature().neqn_code());
        assert_eq!(answer_literal.argument(1).as_ref(), Some(bank.true_term()));
        let answer = answer_literal.argument(0).unwrap();
        assert_eq!(answer.f_code(), bank.signature().answer_code());
        let answer_payload = answer.argument(0).unwrap();
        assert_eq!(answer_payload.arity(), 2);
        assert_eq!(answer_payload.argument(0).as_ref(), Some(&first_var));
        assert_eq!(answer_payload.argument(1).as_ref(), Some(&second_var));
    }

    #[test]
    fn wrapped_formula_annotate_question_adds_answer_literal_to_leading_existentials() {
        let mut bank = test_bank();
        let first_var = typed_var(&bank, -401);
        let second_var = typed_var(&bank, -402);
        let left = typed_const(&mut bank, "wf_question_left");
        let right = typed_const(&mut bank, "wf_question_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let qex_code = bank.signature().qex_code();
        let inner = bool_binary_with_code(&mut bank, qex_code, &second_var, &body);
        let formula = bool_binary_with_code(&mut bank, qex_code, &first_var, &inner);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_QUESTION);

        assert!(wrapped.annotate_question(&mut bank, true, false).unwrap());

        assert_eq!(wrapped.query_tptp_type(), CP_TYPE_CONJECTURE);
        assert_answer_annotation_shape(&bank, wrapped.formula(), &body);
        assert_eq!(
            wrapped.derivation_entries(),
            &[DerivationEntry::Operation(DC_ANNO_QUESTION)]
        );
    }

    #[test]
    fn formula_set_preproc_conjectures_annotates_then_negates_in_order() {
        let mut bank = test_bank();
        let q_left = typed_const(&mut bank, "set_preproc_question_left");
        let q_right = typed_const(&mut bank, "set_preproc_question_right");
        let c_left = typed_const(&mut bank, "set_preproc_conj_left");
        let c_right = typed_const(&mut bank, "set_preproc_conj_right");
        let a_left = typed_const(&mut bank, "set_preproc_axiom_left");
        let a_right = typed_const(&mut bank, "set_preproc_axiom_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let question_formula = bool_binary_with_code(&mut bank, eqn_code, &q_left, &q_right);
        let conjecture_formula = bool_binary_with_code(&mut bank, eqn_code, &c_left, &c_right);
        let axiom_formula = bool_binary_with_code(&mut bank, eqn_code, &a_left, &a_right);
        let mut question = WrappedFormula::wt_formula_alloc(question_formula.clone());
        question.set_tptp_type(CP_TYPE_QUESTION);
        let mut conjecture = WrappedFormula::wt_formula_alloc(conjecture_formula.clone());
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut axiom = WrappedFormula::wt_formula_alloc(axiom_formula.clone());
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut set = FormulaSet::new();
        set.insert(question);
        set.insert(conjecture);
        set.insert(axiom);

        let result = set.preproc_conjectures(&mut bank, false, true).unwrap();

        assert_eq!(result.questions_annotated, 2);
        assert_eq!(result.conjectures_negated, 2);
        assert_eq!(
            result.formula_derivation_ops,
            vec![
                DC_ANNO_QUESTION,
                DC_NEGATE_CONJECTURE,
                DC_ANNO_QUESTION,
                DC_NEGATE_CONJECTURE,
            ]
        );
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(formulas[0].formula().f_code(), bank.signature().not_code());
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_ANNO_QUESTION),
                DerivationEntry::Operation(DC_NEGATE_CONJECTURE)
            ]
        );
        assert_eq!(
            formulas[0].formula().argument(0).as_ref(),
            Some(&question_formula)
        );
        assert_eq!(formulas[1].query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            formulas[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_ANNO_QUESTION),
                DerivationEntry::Operation(DC_NEGATE_CONJECTURE)
            ]
        );
        assert_eq!(
            formulas[1].formula().argument(0).as_ref(),
            Some(&conjecture_formula)
        );
        assert_eq!(formulas[2].query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(formulas[2].formula(), &axiom_formula);
        assert_eq!(formulas[2].derivation_entries(), &[]);
    }

    #[test]
    fn formula_set_preproc_conjectures_with_docs_prints_annotation_then_negation() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "set_preproc_doc_question_left");
        let right = typed_const(&mut bank, "set_preproc_doc_question_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let question_formula = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut question = WrappedFormula::wt_formula_alloc(question_formula.clone());
        question.set_tptp_type(CP_TYPE_QUESTION);
        question.set_prop(CP_INPUT_FORMULA);
        let old_ident = question.ident();
        let question_body = question
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let mut set = FormulaSet::new();
        set.insert(question);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .preproc_conjectures_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
                false,
                true,
            )
            .unwrap();

        assert_eq!(result.preprocess.questions_annotated, 1);
        assert_eq!(result.preprocess.conjectures_negated, 1);
        assert_eq!(
            result.preprocess.formula_derivation_ops,
            vec![DC_ANNO_QUESTION, DC_NEGATE_CONJECTURE]
        );
        assert_eq!(
            result.write_results,
            vec![
                ProofDocWriteResult::printed(),
                ProofDocWriteResult::printed()
            ]
        );
        let formula = set.iter().next().unwrap();
        let final_body = formula
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(
            rendered,
            format!(
                "     1 : conj:{question_body} : add_answer_literal({old_ident})\n     2 : neg:{final_body} : assume_negation(1)\n"
            )
        );
        assert_eq!(formula.ident(), 2);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            formula.derivation_entries(),
            &[
                DerivationEntry::Operation(DC_ANNO_QUESTION),
                DerivationEntry::Operation(DC_NEGATE_CONJECTURE)
            ]
        );
        assert_eq!(
            formula.formula().argument(0).as_ref(),
            Some(&question_formula)
        );
        assert_eq!(session.id_source.current_ident(), 2);
    }

    #[test]
    fn formula_set_preproc_conjectures_with_docs_suppresses_but_keeps_property_side_effects() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "set_preproc_doc_suppress_left");
        let right = typed_const(&mut bank, "set_preproc_doc_suppress_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let conjecture_formula = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut conjecture = WrappedFormula::wt_formula_alloc(conjecture_formula.clone());
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        conjecture.set_prop(CP_INPUT_FORMULA);
        let old_ident = conjecture.ident();
        let mut set = FormulaSet::new();
        set.insert(conjecture);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .preproc_conjectures_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
                false,
                false,
            )
            .unwrap();

        assert_eq!(result.preprocess.questions_annotated, 0);
        assert_eq!(result.preprocess.conjectures_negated, 1);
        assert_eq!(
            result.preprocess.formula_derivation_ops,
            vec![DC_NEGATE_CONJECTURE]
        );
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        let formula = set.iter().next().unwrap();
        assert_eq!(formula.ident(), old_ident);
        assert!(!formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            formula.derivation_entries(),
            &[DerivationEntry::Operation(DC_NEGATE_CONJECTURE)]
        );
        assert_eq!(
            formula.formula().argument(0).as_ref(),
            Some(&conjecture_formula)
        );
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn wrapped_formula_replace_eqn_with_equiv_matches_c_complex_bool_shape() {
        let mut bank = test_bank();
        let left = c_complex_bool_shape(&mut bank, "wf_eq_to_eq_left");
        let right = c_complex_bool_shape(&mut bank, "wf_eq_to_eq_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut wrapped = WrappedFormula::wt_formula_alloc(equality);

        assert!(wrapped.replace_eqn_with_equiv(&mut bank).unwrap());

        assert_eq!(wrapped.formula().f_code(), bank.signature().equiv_code());
        assert_eq!(wrapped.formula().argument(0).as_ref(), Some(&left));
        assert_eq!(wrapped.formula().argument(1).as_ref(), Some(&right));
        assert_eq!(
            wrapped.derivation_entries(),
            &[DerivationEntry::Operation(DC_EQ_TO_EQ)]
        );
    }

    #[test]
    fn wrapped_formula_unroll_fool_expands_literals_without_counting_mapper_noops() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "wf_fool_expand_left");
        let right = typed_const(&mut bank, "wf_fool_expand_right");
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let disequality = bool_binary_with_code(&mut bank, neqn_code, &left, &right);
        let mut wrapped = WrappedFormula::wt_formula_alloc(disequality);

        assert!(!wrapped.unroll_fool(&mut bank).unwrap());

        assert_eq!(wrapped.formula().f_code(), bank.signature().not_code());
        let equality = wrapped.formula().argument(0).unwrap();
        assert_eq!(equality.f_code(), bank.signature().eqn_code());
        assert_eq!(equality.argument(0).as_ref(), Some(&left));
        assert_eq!(equality.argument(1).as_ref(), Some(&right));
        assert_eq!(wrapped.derivation_entries(), &[]);
    }

    #[test]
    fn formula_set_unroll_fool_replaces_then_unrolls_in_order() {
        let mut bank = test_bank();
        let eq_left = c_complex_bool_shape(&mut bank, "set_fool_eq_left");
        let eq_right = c_complex_bool_shape(&mut bank, "set_fool_eq_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &eq_left, &eq_right);
        let replacement_only = WrappedFormula::wt_formula_alloc(equality);

        let a = typed_const(&mut bank, "set_fool_a");
        let b = typed_const(&mut bank, "set_fool_b");
        let c = typed_const(&mut bank, "set_fool_c");
        let d = typed_const(&mut bank, "set_fool_d");
        let target = typed_const(&mut bank, "set_fool_target");
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let bool_arg = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let applied = typed_unary_with_types(
            &mut bank,
            "set_fool_fun",
            &bool_arg,
            &bool_type,
            &default_type,
        );
        let unroll_formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);
        let unroll = WrappedFormula::wt_formula_alloc(unroll_formula);

        let stable_left = typed_const(&mut bank, "set_fool_stable_left");
        let stable_right = typed_const(&mut bank, "set_fool_stable_right");
        let stable_formula =
            bool_binary_with_code(&mut bank, eqn_code, &stable_left, &stable_right);
        let stable = WrappedFormula::wt_formula_alloc(stable_formula.clone());

        let mut set = FormulaSet::new();
        set.insert(replacement_only);
        set.insert(unroll);
        set.insert(stable);

        let result = set.unroll_fool(&mut bank).unwrap();

        assert_eq!(result.boolean_equalities_replaced, 1);
        assert_eq!(result.formulas_unrolled, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_EQ_TO_EQ, DC_FOOL_UNROLL]
        );
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(
            formulas[0].formula().f_code(),
            bank.signature().equiv_code()
        );
        assert_eq!(
            formulas[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_EQ_TO_EQ)]
        );
        assert_eq!(formulas[1].formula().f_code(), bank.signature().and_code());
        assert_eq!(
            formulas[1].derivation_entries(),
            &[DerivationEntry::Operation(DC_FOOL_UNROLL)]
        );
        assert_eq!(formulas[2].formula(), &stable_formula);
        assert_eq!(formulas[2].derivation_entries(), &[]);
    }

    #[test]
    fn formula_set_introduce_defs_archives_split_def_and_applies_definition() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_def_first");
        let second = typed_const(&mut bank, "set_intro_def_second");
        let third = typed_const(&mut bank, "set_intro_def_third");
        let fourth = typed_const(&mut bank, "set_intro_def_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();

        let result = set.introduce_defs(&mut archive, &mut bank, 1).unwrap();

        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(result.archived_definitions, 1);
        assert_eq!(result.active_definitions_inserted, 1);
        assert_eq!(result.formulas_rewritten, 1);
        assert_eq!(result.definition_applications, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_SPLIT_EQUIV, DC_APPLY_DEF]
        );

        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas.len(), 2);
        assert_eq!(archive.cardinality(), 1);
        let archived_wrapper = archive.iter().next().unwrap();
        let archived_ref = FormulaDerivationRef::new(archived_wrapper.ident());
        assert_eq!(
            archived_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );

        let rewritten = formulas[0].formula();
        assert_eq!(rewritten.f_code(), bank.signature().or_code());
        let rename_atom = rewritten.argument(0).unwrap();
        assert_eq!(rename_atom.f_code(), bank.signature().eqn_code());
        assert_eq!(rewritten.argument(1).as_ref(), Some(&tail));
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );

        let active_definition = formulas[1].formula();
        assert_eq!(active_definition.f_code(), bank.signature().impl_code());
        assert_eq!(active_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(active_definition.argument(1).as_ref(), Some(&expensive));
        assert_eq!(
            formulas[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_SPLIT_EQUIV),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );

        let archived_definition = archived_wrapper.formula();
        assert_eq!(archived_definition.f_code(), bank.signature().equiv_code());
        assert_eq!(archived_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(archived_definition.argument(1).as_ref(), Some(&expensive));
    }

    #[test]
    fn formula_set_introduce_defs_keeps_zero_polarity_definition_as_equivalence() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_zero_first");
        let second = typed_const(&mut bank, "set_intro_zero_second");
        let third = typed_const(&mut bank, "set_intro_zero_third");
        let fourth = typed_const(&mut bank, "set_intro_zero_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let or_code = bank.signature().or_code();
        let expensive = bool_binary_with_code(&mut bank, or_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let formula = bool_binary_with_code(&mut bank, equiv_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();

        let result = set.introduce_defs(&mut archive, &mut bank, 1).unwrap();

        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_FOF_QUOTE, DC_APPLY_DEF]
        );

        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas.len(), 2);
        assert_eq!(archive.cardinality(), 1);
        let archived_wrapper = archive.iter().next().unwrap();
        let archived_ref = FormulaDerivationRef::new(archived_wrapper.ident());
        assert_eq!(
            archived_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );

        let rewritten = formulas[0].formula();
        let rename_atom = rewritten.argument(0).unwrap();
        assert_eq!(rename_atom.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );
        let active_definition = formulas[1].formula();
        assert_eq!(active_definition.f_code(), bank.signature().equiv_code());
        assert_eq!(active_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(active_definition.argument(1).as_ref(), Some(&expensive));
        assert_eq!(
            formulas[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );
    }

    #[test]
    fn formula_set_introduce_defs_with_docs_prints_creation_and_apply_steps() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_doc_first");
        let second = typed_const(&mut bank, "set_intro_doc_second");
        let third = typed_const(&mut bank, "set_intro_doc_third");
        let fourth = typed_const(&mut bank, "set_intro_doc_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut archive = FormulaSet::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .introduce_defs_with_docs(
                &mut archive,
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
                1,
            )
            .unwrap();

        assert_eq!(result.introduce.definitions_introduced, 1);
        assert_eq!(result.introduce.archived_definitions, 1);
        assert_eq!(result.introduce.active_definitions_inserted, 1);
        assert_eq!(result.introduce.formulas_rewritten, 1);
        assert_eq!(result.introduce.definition_applications, 1);
        assert_eq!(
            result.introduce.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_SPLIT_EQUIV, DC_APPLY_DEF]
        );
        assert_eq!(
            result.definition_write_results,
            vec![
                ProofDocWriteResult::printed(),
                ProofDocWriteResult::printed()
            ]
        );
        assert_eq!(
            result.application_write_results,
            vec![ProofDocWriteResult::printed()]
        );

        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas.len(), 2);
        assert_eq!(archive.cardinality(), 1);
        let archived = archive.iter().next().unwrap();
        let archived_ref = FormulaDerivationRef::new(archived.ident());
        assert_eq!(archived.ident(), 1);
        assert_eq!(formulas[1].ident(), 2);
        assert_eq!(formulas[0].ident(), 3);
        assert_eq!(session.id_source.current_ident(), 3);
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );
        assert_eq!(
            formulas[1].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_SPLIT_EQUIV),
                DerivationEntry::FormulaParent(archived_ref)
            ]
        );

        let archived_body = archived
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let active_body = formulas[1]
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        let rewritten_body = formulas[0]
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        assert_eq!(
            rendered,
            format!(
                "     1 : :{archived_body} : introduced\n     2 : :{active_body} : split_equiv(1)\n     3 : :{rewritten_body} : apply_def({old_ident},1)\n"
            )
        );
    }

    #[test]
    fn formula_set_introduce_defs_with_docs_suppresses_but_keeps_side_effects() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_doc_suppress_first");
        let second = typed_const(&mut bank, "set_intro_doc_suppress_second");
        let third = typed_const(&mut bank, "set_intro_doc_suppress_third");
        let fourth = typed_const(&mut bank, "set_intro_doc_suppress_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let wrapped = WrappedFormula::wt_formula_alloc(formula);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut archive = FormulaSet::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = set
            .introduce_defs_with_docs(
                &mut archive,
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
                1,
            )
            .unwrap();

        assert_eq!(result.introduce.definitions_introduced, 1);
        assert_eq!(
            result.definition_write_results,
            vec![
                ProofDocWriteResult::suppressed(),
                ProofDocWriteResult::suppressed()
            ]
        );
        assert_eq!(
            result.application_write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        assert_eq!(session.id_source.current_ident(), 0);

        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].ident(), old_ident);
        assert_eq!(
            result.introduce.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_SPLIT_EQUIV, DC_APPLY_DEF]
        );
        assert_eq!(archive.cardinality(), 1);
    }

    #[test]
    fn formula_set_named_to_db_lambdas_is_higher_order_gated() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -501);
        let a = typed_const(&mut bank, "set_named_db_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let named_lambda =
            tformula_quantor_alloc(&mut bank, SIG_NAMED_LAMBDA_CODE, &x, &body).unwrap();
        let mut first_order = FormulaSet::new();
        first_order.insert(WrappedFormula::wt_formula_alloc(named_lambda.clone()));

        let first_order_result = first_order
            .named_to_db_lambdas(&mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(first_order_result.formulas_named_to_db, 0);
        assert!(first_order_result.formula_derivation_ops.is_empty());
        assert_eq!(first_order.iter().next().unwrap().formula(), &named_lambda);
        assert_eq!(first_order.iter().next().unwrap().derivation_entries(), &[]);

        let mut higher_order = FormulaSet::new();
        higher_order.insert(WrappedFormula::wt_formula_alloc(named_lambda));

        let result = higher_order
            .named_to_db_lambdas(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_named_to_db, 1);
        assert_eq!(result.formula_derivation_ops, vec![DC_FOF_SIMPLIFY]);
        let converted_wrapper = higher_order.iter().next().unwrap();
        assert_eq!(
            converted_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        let converted = converted_wrapper.formula();
        assert_eq!(converted.f_code(), SIG_DB_LAMBDA_CODE);
        let matrix = converted.argument(1).unwrap();
        assert_eq!(matrix.f_code(), eqn_code);
        assert!(matrix.argument(0).unwrap().is_db_var());
        assert_eq!(matrix.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn formula_set_named_to_db_lambdas_with_docs_prints_changed_formula() {
        let mut bank = test_bank();
        let named_lambda_formula = named_lambda_equality(&mut bank, "set_named_db_doc");
        let mut wrapped = WrappedFormula::wt_formula_alloc(named_lambda_formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .named_to_db_lambdas_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_named_to_db, 1);
        assert_eq!(
            result.preprocess.formula_derivation_ops,
            vec![DC_FOF_SIMPLIFY]
        );
        assert_eq!(result.write_results, vec![ProofDocWriteResult::printed()]);
        let converted_wrapper = set.iter().next().unwrap();
        let converted_body = converted_wrapper
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(
            rendered,
            format!("     1 : :{converted_body} : fof_simplification({old_ident})\n")
        );
        assert_eq!(converted_wrapper.ident(), 1);
        assert!(!converted_wrapper.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            converted_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(
            converted_wrapper.formula().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            converted_wrapper.formula().argument(0).unwrap().f_code(),
            SIG_DB_LAMBDA_CODE
        );
        assert_eq!(session.id_source.current_ident(), 1);
    }

    #[test]
    fn formula_set_named_to_db_lambdas_with_docs_suppresses_but_keeps_property_side_effects() {
        let mut bank = test_bank();
        let named_lambda_formula = named_lambda_equality(&mut bank, "set_named_db_doc_suppress");
        let mut wrapped = WrappedFormula::wt_formula_alloc(named_lambda_formula);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .named_to_db_lambdas_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_named_to_db, 1);
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        let converted_wrapper = set.iter().next().unwrap();
        assert_eq!(converted_wrapper.ident(), old_ident);
        assert!(!converted_wrapper.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            converted_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(
            converted_wrapper.formula().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            converted_wrapper.formula().argument(0).unwrap().f_code(),
            SIG_DB_LAMBDA_CODE
        );
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn formula_set_lift_ites_is_higher_order_gated() {
        let mut bank = test_bank();
        let condition_left = typed_const(&mut bank, "set_lift_ite_condition_left");
        let condition_right = typed_const(&mut bank, "set_lift_ite_condition_right");
        let then_left = typed_const(&mut bank, "set_lift_ite_then_left");
        let then_right = typed_const(&mut bank, "set_lift_ite_then_right");
        let else_left = typed_const(&mut bank, "set_lift_ite_else_left");
        let else_right = typed_const(&mut bank, "set_lift_ite_else_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let and_code = bank.signature().and_code();
        let condition =
            bool_binary_with_code(&mut bank, eqn_code, &condition_left, &condition_right);
        let then_atom = bool_binary_with_code(&mut bank, eqn_code, &then_left, &then_right);
        let else_atom = bool_binary_with_code(&mut bank, eqn_code, &else_left, &else_right);
        let ite = bool_ite(&mut bank, &condition, &then_atom, &else_atom);
        let mut first_order = FormulaSet::new();
        first_order.insert(WrappedFormula::wt_formula_alloc(ite.clone()));

        let first_order_result = first_order
            .lift_ites(&mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(first_order_result.formulas_ites_lifted, 0);
        assert!(first_order_result.formula_derivation_ops.is_empty());
        assert_eq!(first_order.iter().next().unwrap().formula(), &ite);
        assert_eq!(first_order.iter().next().unwrap().derivation_entries(), &[]);

        let mut higher_order = FormulaSet::new();
        higher_order.insert(WrappedFormula::wt_formula_alloc(ite));

        let result = higher_order
            .lift_ites(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_ites_lifted, 1);
        assert_eq!(result.formula_derivation_ops, vec![DC_LIFT_ITE]);
        let lifted = higher_order.iter().next().unwrap();
        assert_eq!(
            lifted.derivation_entries(),
            &[DerivationEntry::Operation(DC_LIFT_ITE)]
        );
        assert_eq!(lifted.formula().f_code(), and_code);
    }

    #[test]
    fn formula_set_lift_lets_is_higher_order_gated() {
        let mut bank = test_bank();
        let local_symbol = typed_const(&mut bank, "set_lift_let_local_symbol");
        let definition_value = typed_const(&mut bank, "set_lift_let_definition_value");
        let target = typed_const(&mut bank, "set_lift_let_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition =
            bool_binary_with_code(&mut bank, eqn_code, &local_symbol, &definition_value);
        let let_term = let_term(&mut bank, &[definition], &local_symbol);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &let_term, &target);
        let mut first_order = FormulaSet::new();
        first_order.insert(WrappedFormula::wt_formula_alloc(formula.clone()));

        let first_order_result = first_order
            .lift_lets(&mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(first_order_result.formulas_lets_lifted, 0);
        assert!(first_order_result.formula_derivation_ops.is_empty());
        assert_eq!(first_order.cardinality(), 1);
        assert_eq!(first_order.iter().next().unwrap().formula(), &formula);
        assert_eq!(first_order.iter().next().unwrap().derivation_entries(), &[]);

        let mut higher_order = FormulaSet::new();
        higher_order.insert(WrappedFormula::wt_formula_alloc(formula));

        let result = higher_order
            .lift_lets(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_lets_lifted, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_APPLY_DEF]
        );
        assert_eq!(higher_order.cardinality(), 2);
        let formulas = higher_order.iter().collect::<Vec<_>>();
        let rewritten = formulas[0].formula();
        assert_eq!(rewritten.f_code(), eqn_code);
        assert_ne!(rewritten.argument(0).unwrap().f_code(), SIG_LET_CODE);
        let generated_definition = formulas[1].formula();
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(FormulaDerivationRef::new(formulas[1].ident()))
            ]
        );
        assert_eq!(
            formulas[1].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
        assert_eq!(generated_definition.f_code(), eqn_code);
        assert_eq!(
            generated_definition.argument(1).as_ref(),
            Some(&definition_value)
        );
    }

    #[test]
    fn formula_set_lift_lambdas_is_higher_order_gated_and_inserts_definitions() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let inner = typed_unary(&mut bank, "set_lift_lambda_body_f", &db0);
        let body = typed_unary(&mut bank, "set_lift_lambda_body_g", &inner);
        let lambda = close_with_db_var(&mut bank, &i_type, &body).unwrap();
        let lambda_type = lambda.type_().expect("lambda term is typed");
        let wrapped_lambda = typed_unary_with_types(
            &mut bank,
            "set_lift_lambda_wrapper",
            &lambda,
            &lambda_type,
            &i_type,
        );
        let target = typed_const(&mut bank, "set_lift_lambda_target");
        let eqn_code = bank.signature().eqn_code();
        let formula = bool_binary_with_code(&mut bank, eqn_code, &wrapped_lambda, &target);
        let mut first_order = FormulaSet::new();
        first_order.insert(WrappedFormula::wt_formula_alloc(formula.clone()));

        let first_order_result = first_order
            .lift_lambdas(&mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(first_order_result.formulas_lambdas_lifted, 0);
        assert_eq!(first_order_result.lambda_lift_definitions_inserted, 0);
        assert!(first_order_result.formula_derivation_ops.is_empty());
        assert_eq!(first_order.cardinality(), 1);
        assert_eq!(first_order.iter().next().unwrap().formula(), &formula);

        let mut higher_order = FormulaSet::new();
        higher_order.insert(WrappedFormula::wt_formula_alloc(formula));

        let result = higher_order
            .lift_lambdas(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_lambdas_lifted, 1);
        assert_eq!(result.lambda_lift_definitions_inserted, 1);
        assert_eq!(result.formula_derivation_ops, vec![DC_INTRO_DEF]);
        assert_eq!(higher_order.cardinality(), 2);
        let formulas = higher_order.iter().collect::<Vec<_>>();
        assert!(!formulas[0].formula().has_lambda_subterm());
        assert_eq!(
            formulas[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
        assert_eq!(formulas[1].formula().f_code(), bank.signature().qall_code());
        assert!(!formulas[1].formula().has_lambda_subterm());
        assert_eq!(
            formulas[1].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
    }

    #[test]
    fn formula_set_lift_lambdas_reuses_exact_closed_liftings() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let body = typed_unary(&mut bank, "set_lift_reuse_body", &db0);
        let lambda = close_with_db_var(&mut bank, &i_type, &body).unwrap();
        let lambda_type = lambda.type_().expect("lambda term is typed");
        let wrapped_lambda = typed_unary_with_types(
            &mut bank,
            "set_lift_reuse_wrapper",
            &lambda,
            &lambda_type,
            &i_type,
        );
        let eqn_code = bank.signature().eqn_code();
        let formula = bool_binary_with_code(&mut bank, eqn_code, &wrapped_lambda, &wrapped_lambda);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));

        let result = set
            .lift_lambdas(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_lambdas_lifted, 1);
        assert_eq!(result.lambda_lift_definitions_inserted, 2);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_INTRO_DEF]
        );
        assert_eq!(set.cardinality(), 2);
        let formulas = set.iter().collect::<Vec<_>>();
        let rewritten = formulas[0].formula();
        assert_eq!(rewritten.argument(0), rewritten.argument(1));
        assert_eq!(
            formulas[0].derivation_entries(),
            &[
                DerivationEntry::Operation(DC_INTRO_DEF),
                DerivationEntry::Operation(DC_INTRO_DEF),
            ]
        );
        assert_eq!(
            formulas[1].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
    }

    #[test]
    fn formula_set_lift_lambdas_reuses_generalized_liftings() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let x = typed_var(&bank, -901);
        let a = typed_const(&mut bank, "set_lift_generalize_a");
        let generic_body = typed_unary(&mut bank, "set_lift_generalize_body", &x);
        let specific_body = typed_unary(&mut bank, "set_lift_generalize_body", &a);
        let generic_lambda = close_with_db_var(&mut bank, &i_type, &generic_body).unwrap();
        let specific_lambda = close_with_db_var(&mut bank, &i_type, &specific_body).unwrap();
        let lambda_type = generic_lambda.type_().expect("lambda term is typed");
        let generic_wrapped = typed_unary_with_types(
            &mut bank,
            "set_lift_generalize_wrapper",
            &generic_lambda,
            &lambda_type,
            &i_type,
        );
        let specific_wrapped = typed_unary_with_types(
            &mut bank,
            "set_lift_generalize_wrapper",
            &specific_lambda,
            &lambda_type,
            &i_type,
        );
        let eqn_code = bank.signature().eqn_code();
        let formula =
            bool_binary_with_code(&mut bank, eqn_code, &generic_wrapped, &specific_wrapped);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));

        let result = set
            .lift_lambdas(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_lambdas_lifted, 1);
        assert_eq!(result.lambda_lift_definitions_inserted, 2);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_INTRO_DEF]
        );
        assert_eq!(set.cardinality(), 2);
        let formulas = set.iter().collect::<Vec<_>>();
        let rewritten = formulas[0].formula();
        let left_lifted = rewritten
            .argument(0)
            .and_then(|term| term.argument(0))
            .expect("left wrapper contains lifted lambda replacement");
        let right_lifted = rewritten
            .argument(1)
            .and_then(|term| term.argument(0))
            .expect("right wrapper contains lifted lambda replacement");
        assert_eq!(left_lifted.f_code(), right_lifted.f_code());
        assert_eq!(left_lifted.argument(0).as_ref(), Some(&x));
        assert_eq!(right_lifted.argument(0).as_ref(), Some(&a));
        assert_eq!(
            formulas[1].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
    }

    #[test]
    fn formula_set_lambda_normalize_forall_is_higher_order_gated() {
        let mut bank = test_bank();
        let formula = db_lambda_equality(&mut bank, "set_lambda_norm");
        let eqn_code = bank.signature().eqn_code();
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula.clone()));

        let first_order_result = set
            .lambda_normalize_forall(&mut bank, ProblemType::FirstOrder)
            .unwrap();

        assert_eq!(first_order_result.formulas_lambda_normalized, 0);
        assert!(first_order_result.formula_derivation_ops.is_empty());
        assert_eq!(set.iter().next().unwrap().formula(), &formula);
        assert_eq!(set.iter().next().unwrap().derivation_entries(), &[]);

        let result = set
            .lambda_normalize_forall(&mut bank, ProblemType::HigherOrder)
            .unwrap();

        assert_eq!(result.formulas_lambda_normalized, 1);
        assert_eq!(result.formula_derivation_ops, vec![DC_FOF_SIMPLIFY]);
        let normalized_wrapper = set.iter().next().unwrap();
        assert_eq!(
            normalized_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        let normalized = normalized_wrapper.formula();
        assert_eq!(normalized.f_code(), bank.signature().qall_code());
        assert_eq!(normalized.argument(1).unwrap().f_code(), eqn_code);
    }

    #[test]
    fn formula_set_lambda_normalize_forall_with_docs_prints_changed_formula() {
        let mut bank = test_bank();
        let formula = db_lambda_equality(&mut bank, "set_lambda_norm_doc");
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .lambda_normalize_forall_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_lambda_normalized, 1);
        assert_eq!(
            result.preprocess.formula_derivation_ops,
            vec![DC_FOF_SIMPLIFY]
        );
        assert_eq!(result.write_results, vec![ProofDocWriteResult::printed()]);
        let normalized_wrapper = set.iter().next().unwrap();
        let normalized_body = normalized_wrapper
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(
            rendered,
            format!("     1 : :{normalized_body} : fof_simplification({old_ident})\n")
        );
        assert_eq!(normalized_wrapper.ident(), 1);
        assert!(!normalized_wrapper.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            normalized_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(session.id_source.current_ident(), 1);
    }

    #[test]
    fn formula_set_lambda_normalize_forall_with_docs_suppresses_but_keeps_property_side_effects() {
        let mut bank = test_bank();
        let formula = db_lambda_equality(&mut bank, "set_lambda_norm_doc_suppress");
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .lambda_normalize_forall_with_docs(
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_lambda_normalized, 1);
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        let normalized_wrapper = set.iter().next().unwrap();
        assert_eq!(normalized_wrapper.ident(), old_ident);
        assert!(!normalized_wrapper.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            normalized_wrapper.derivation_entries(),
            &[DerivationEntry::Operation(DC_FOF_SIMPLIFY)]
        );
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn wrapped_formula_cnf2_quotes_clause_wrappers_directly() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_clause_a");
        let b = typed_const(&mut bank, "wf_cnf_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &b, true)]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let source = FormulaDerivationRef::new(wrapped.ident());
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = wrapped
            .cnf2_into(
                &mut bank,
                &mut set,
                &fresh_vars,
                100,
                false,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.clauses_generated, 1);
        assert_eq!(result.formula_derivation_ops, Vec::<i64>::new());
        assert_eq!(wrapped.derivation_entries(), &[]);
        let generated = set.iter().next().unwrap();
        assert_eq!(generated.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(generated.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            generated.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(source),
            ]
        );
    }

    #[test]
    fn wrapped_formula_cnf2_runs_formula_pipeline_and_extracts_clauses() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_a");
        let b = typed_const(&mut bank, "wf_cnf_b");
        let c = typed_const(&mut bank, "wf_cnf_c");
        let d = typed_const(&mut bank, "wf_cnf_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let source = FormulaDerivationRef::new(wrapped.ident());
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = wrapped
            .cnf2_into(
                &mut bank,
                &mut set,
                &fresh_vars,
                100,
                false,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.clauses_generated, 2);
        assert!(result
            .formula_derivation_ops
            .contains(&DC_DIST_DISJUNCTIONS));
        assert_eq!(
            wrapped.derivation_entries(),
            result
                .formula_derivation_ops
                .iter()
                .copied()
                .map(DerivationEntry::Operation)
                .collect::<Vec<_>>()
                .as_slice()
        );
        assert_eq!(set.members(), 2);
        for clause in set.iter() {
            assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
            assert_eq!(
                &clause.derivation().unwrap().as_slice()[..2],
                &[
                    DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                    DerivationEntry::FormulaParent(source),
                ]
            );
        }
    }

    #[test]
    fn wrapped_formula_cnf2_with_docs_prints_formula_and_split_steps() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_doc_a");
        let b = typed_const(&mut bank, "wf_cnf_doc_b");
        let c = typed_const(&mut bank, "wf_cnf_doc_c");
        let d = typed_const(&mut bank, "wf_cnf_doc_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = {
            let mut doc_context = WrappedFormulaCnfDocContext::new(
                &mut rendered,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
            );
            wrapped
                .cnf2_into_with_docs(
                    &mut doc_context,
                    &mut bank,
                    &mut set,
                    &fresh_vars,
                    100,
                    false,
                )
                .unwrap()
        };

        assert_eq!(result.cnf.clauses_generated, 2);
        assert_eq!(
            result.cnf.formula_derivation_ops,
            vec![DC_DIST_DISJUNCTIONS]
        );
        assert_eq!(
            result.formula_write_results,
            vec![ProofDocWriteResult::printed()]
        );
        assert_eq!(
            result.clause_write_results,
            vec![
                ProofDocWriteResult::printed(),
                ProofDocWriteResult::printed()
            ]
        );
        let distributed_body = wrapped
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::FirstOrder)
            .unwrap();
        assert!(rendered.starts_with(&format!(
            "     1 : neg:{distributed_body} : distribute({old_ident})\n"
        )));
        assert_eq!(rendered.matches("split_conjunct(1)").count(), 2);
        assert_eq!(rendered.lines().count(), 3);
        assert_eq!(wrapped.ident(), 1);
        assert!(!wrapped.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            wrapped.derivation_entries(),
            &[DerivationEntry::Operation(DC_DIST_DISJUNCTIONS)]
        );
        assert_eq!(session.id_source.current_ident(), 3);

        let split_source = FormulaDerivationRef::new(1);
        let generated_idents = set.iter().map(Clause::ident).collect::<Vec<_>>();
        assert_eq!(generated_idents, vec![2, 3]);
        for clause in set.iter() {
            assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
            assert!(!clause.query_prop(CP_INPUT_FORMULA));
            assert_eq!(
                &clause.derivation().unwrap().as_slice()[..2],
                &[
                    DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                    DerivationEntry::FormulaParent(split_source),
                ]
            );
        }
    }

    #[test]
    fn wrapped_formula_cnf2_with_docs_suppresses_but_keeps_property_side_effects() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_doc_suppress_a");
        let b = typed_const(&mut bank, "wf_cnf_doc_suppress_b");
        let c = typed_const(&mut bank, "wf_cnf_doc_suppress_c");
        let d = typed_const(&mut bank, "wf_cnf_doc_suppress_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = {
            let mut doc_context = WrappedFormulaCnfDocContext::new(
                &mut rendered,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
            );
            wrapped
                .cnf2_into_with_docs(
                    &mut doc_context,
                    &mut bank,
                    &mut set,
                    &fresh_vars,
                    100,
                    false,
                )
                .unwrap()
        };

        assert_eq!(result.cnf.clauses_generated, 2);
        assert_eq!(
            result.formula_write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert_eq!(
            result.clause_write_results,
            vec![
                ProofDocWriteResult::suppressed(),
                ProofDocWriteResult::suppressed()
            ]
        );
        assert!(rendered.is_empty());
        assert_eq!(wrapped.ident(), old_ident);
        assert!(!wrapped.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);

        let split_source = FormulaDerivationRef::new(old_ident);
        for clause in set.iter() {
            assert!(!clause.query_prop(CP_INPUT_FORMULA));
            assert_eq!(
                &clause.derivation().unwrap().as_slice()[..2],
                &[
                    DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                    DerivationEntry::FormulaParent(split_source),
                ]
            );
        }
    }

    fn count_formula_set_cnf_derivations(
        clauses: &ClauseSet,
        split_source: FormulaDerivationRef,
        quote_source: FormulaDerivationRef,
    ) -> (i32, i32) {
        let mut split_clauses = 0;
        let mut quoted_clauses = 0;
        for clause in clauses.iter() {
            let derivation = clause.derivation().unwrap().as_slice();
            let operation = match derivation.first() {
                Some(DerivationEntry::Operation(operation)) => *operation,
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            };
            let parent = match derivation.get(1) {
                Some(DerivationEntry::FormulaParent(parent)) => *parent,
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            };
            match operation {
                DC_SPLIT_CONJUNCT => {
                    split_clauses += 1;
                    assert_eq!(parent, split_source);
                    assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
                }
                DC_FOF_QUOTE => {
                    quoted_clauses += 1;
                    assert_eq!(parent, quote_source);
                    assert_eq!(clause.query_tptp_type(), CP_TYPE_AXIOM);
                }
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            }
        }
        (split_clauses, quoted_clauses)
    }

    #[test]
    fn formula_set_cnf2_runs_supported_ho_preprocessing_before_archive_drain() {
        let mut bank = test_bank();
        let formula = db_lambda_equality(&mut bank, "set_cnf_ho_lambda");
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::HigherOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_named_to_db, 0);
        assert_eq!(result.formulas_lambda_normalized, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOF_SIMPLIFY));
        assert_eq!(result.original_formulas_archived, 1);
        assert_eq!(result.cnf_formulas_archived, 1);
        assert!(result.clauses_generated > 0);
        assert_eq!(
            archive.iter().next().unwrap().formula().f_code(),
            bank.signature().qall_code()
        );
    }

    #[test]
    fn formula_set_cnf2_lifts_ites_before_archive_drain() {
        let mut bank = test_bank();
        let condition_left = typed_const(&mut bank, "set_cnf_lift_ite_condition_left");
        let condition_right = typed_const(&mut bank, "set_cnf_lift_ite_condition_right");
        let then_left = typed_const(&mut bank, "set_cnf_lift_ite_then_left");
        let then_right = typed_const(&mut bank, "set_cnf_lift_ite_then_right");
        let else_left = typed_const(&mut bank, "set_cnf_lift_ite_else_left");
        let else_right = typed_const(&mut bank, "set_cnf_lift_ite_else_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let and_code = bank.signature().and_code();
        let condition =
            bool_binary_with_code(&mut bank, eqn_code, &condition_left, &condition_right);
        let then_atom = bool_binary_with_code(&mut bank, eqn_code, &then_left, &then_right);
        let else_atom = bool_binary_with_code(&mut bank, eqn_code, &else_left, &else_right);
        let ite = bool_ite(&mut bank, &condition, &then_atom, &else_atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(ite));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::HigherOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_ites_lifted, 1);
        assert!(result.formula_derivation_ops.contains(&DC_LIFT_ITE));
        assert_eq!(result.original_formulas_archived, 1);
        assert_eq!(archive.iter().next().unwrap().formula().f_code(), and_code);
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_lifts_lets_before_archive_drain() {
        let mut bank = test_bank();
        let local_symbol = typed_const(&mut bank, "set_cnf_lift_let_local_symbol");
        let definition_value = typed_const(&mut bank, "set_cnf_lift_let_definition_value");
        let target = typed_const(&mut bank, "set_cnf_lift_let_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition =
            bool_binary_with_code(&mut bank, eqn_code, &local_symbol, &definition_value);
        let let_term = let_term(&mut bank, &[definition], &local_symbol);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &let_term, &target);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::HigherOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_lets_lifted, 1);
        assert!(result.formula_derivation_ops.contains(&DC_INTRO_DEF));
        assert!(result.formula_derivation_ops.contains(&DC_APPLY_DEF));
        assert_eq!(result.original_formulas_archived, 2);
        assert!(archive.iter().all(|formula| {
            formula.formula().f_code() != SIG_LET_CODE
                && formula
                    .formula()
                    .argument(0)
                    .is_none_or(|argument| argument.f_code() != SIG_LET_CODE)
        }));
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_handles_lifted_term_let_equality_with_fool_unroll() {
        let mut bank = test_bank();
        let local_symbol = typed_const(&mut bank, "set_cnf_lift_fool_let_local_symbol");
        let definition_value = typed_const(&mut bank, "set_cnf_lift_fool_let_definition_value");
        let target = typed_const(&mut bank, "set_cnf_lift_fool_let_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition =
            bool_binary_with_code(&mut bank, eqn_code, &local_symbol, &definition_value);
        let let_term = let_term(&mut bank, &[definition], &local_symbol);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &let_term, &target);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, true, ProblemType::HigherOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_lets_lifted, 1);
        assert!(result.formula_derivation_ops.contains(&DC_INTRO_DEF));
        assert!(result.formula_derivation_ops.contains(&DC_APPLY_DEF));
        assert_eq!(result.original_formulas_archived, 2);
        assert!(archive.iter().all(|formula| {
            formula.formula().f_code() != SIG_LET_CODE
                && formula
                    .formula()
                    .argument(0)
                    .is_none_or(|argument| argument.f_code() != SIG_LET_CODE)
        }));
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_unfold_def_symbols_rewrites_and_archives_definitions() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -71);
        let a = typed_const(&mut bank, "set_unfold_def_a");
        let p_x = typed_unary_predicate(&mut bank, "set_unfold_def_p", &x);
        let q_x = typed_unary_predicate(&mut bank, "set_unfold_def_q", &x);
        let p_a = typed_unary_predicate(&mut bank, "set_unfold_def_p", &a);
        let q_a = typed_unary_predicate(&mut bank, "set_unfold_def_q", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let true_term = bank.true_term().clone();
        let lhs_formula = bool_binary_with_code(&mut bank, eqn_code, &p_x, &true_term);
        let definition_formula = bool_binary_with_code(&mut bank, equiv_code, &lhs_formula, &q_x);
        let mut definition = WrappedFormula::wt_formula_alloc(definition_formula.clone());
        definition.set_prop(CP_IS_LAMBDA_DEF);
        let definition_entry = definition.entry_id();

        let mut first_order = FormulaSet::new();
        first_order.insert(definition.flat_copy());
        first_order.insert(WrappedFormula::wt_formula_alloc(p_a.clone()));
        let mut first_order_archive = FormulaSet::new();
        let first_order_result = first_order
            .unfold_def_symbols(
                &mut first_order_archive,
                &mut bank,
                ProblemType::FirstOrder,
                true,
            )
            .unwrap();
        assert_eq!(first_order_result.formulas_def_symbols_unfolded, 0);
        assert!(first_order_archive.is_empty());

        let mut set = FormulaSet::new();
        set.insert(definition);
        set.insert(WrappedFormula::wt_formula_alloc(p_a));
        let mut archive = FormulaSet::new();

        let result = set
            .unfold_def_symbols(&mut archive, &mut bank, ProblemType::HigherOrder, true)
            .unwrap();

        assert_eq!(result.formulas_def_symbols_unfolded, 1);
        assert_eq!(result.unfolded_definitions_archived, 1);
        assert_eq!(result.unfolded_original_definitions_archived, 1);
        assert_eq!(result.definition_symbol_applications, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOF_QUOTE));
        assert!(result.formula_derivation_ops.contains(&DC_APPLY_DEF));
        assert_eq!(set.cardinality(), 1);
        let rewritten = set.iter().next().unwrap();
        assert_eq!(rewritten.formula(), &q_a);
        assert_eq!(archive.cardinality(), 2);
        let generated_definition = archive.iter().next().unwrap();
        let original_definition = archive.get(definition_entry).unwrap();
        let generated_ref = FormulaDerivationRef::new(generated_definition.ident());
        let original_ref = FormulaDerivationRef::new(original_definition.ident());
        assert_eq!(
            rewritten.derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(generated_ref)
            ]
        );
        assert_eq!(generated_definition.formula().f_code(), eqn_code);
        assert_eq!(
            generated_definition.derivation_entries(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(original_ref)
            ]
        );
        assert_eq!(
            archive.iter().nth(1).unwrap().formula(),
            &definition_formula
        );
        assert_eq!(original_definition.derivation_entries(), &[]);
    }

    #[test]
    fn formula_set_unfold_def_symbols_honors_unfold_only_forms_predicate_gate() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -7111);
        let a = typed_const(&mut bank, "set_unfold_gate_a");
        let f_x = typed_unary(&mut bank, "set_unfold_gate_f", &x);
        let g_x = typed_unary(&mut bank, "set_unfold_gate_g", &x);
        let f_a = typed_unary(&mut bank, "set_unfold_gate_f", &a);
        let g_a = typed_unary(&mut bank, "set_unfold_gate_g", &a);
        let target_before = typed_unary_predicate(&mut bank, "set_unfold_gate_p", &f_a);
        let target_after = typed_unary_predicate(&mut bank, "set_unfold_gate_p", &g_a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition_formula = bool_binary_with_code(&mut bank, eqn_code, &f_x, &g_x);
        let mut definition = WrappedFormula::wt_formula_alloc(definition_formula.clone());
        definition.set_prop(CP_IS_LAMBDA_DEF);

        let mut gated = FormulaSet::new();
        gated.insert(definition.flat_copy());
        gated.insert(WrappedFormula::wt_formula_alloc(target_before.clone()));
        let mut gated_archive = FormulaSet::new();

        let gated_result = gated
            .unfold_def_symbols(
                &mut gated_archive,
                &mut bank,
                ProblemType::HigherOrder,
                true,
            )
            .unwrap();

        assert_eq!(gated_result.formulas_def_symbols_unfolded, 0);
        assert_eq!(gated_result.unfolded_definitions_archived, 0);
        assert_eq!(gated_result.unfolded_original_definitions_archived, 0);
        assert!(gated_archive.is_empty());
        assert_eq!(gated.cardinality(), 2);
        assert!(gated
            .iter()
            .any(|formula| formula.formula() == &target_before));
        assert!(gated
            .iter()
            .any(|formula| formula.formula() == &definition_formula));

        let mut ungated = FormulaSet::new();
        ungated.insert(definition);
        ungated.insert(WrappedFormula::wt_formula_alloc(target_before));
        let mut ungated_archive = FormulaSet::new();

        let ungated_result = ungated
            .unfold_def_symbols(
                &mut ungated_archive,
                &mut bank,
                ProblemType::HigherOrder,
                false,
            )
            .unwrap();

        assert_eq!(ungated_result.formulas_def_symbols_unfolded, 1);
        assert_eq!(ungated_result.unfolded_definitions_archived, 1);
        assert_eq!(ungated_result.unfolded_original_definitions_archived, 1);
        assert_eq!(ungated_result.definition_symbol_applications, 1);
        assert_eq!(ungated.cardinality(), 1);
        assert_eq!(ungated.iter().next().unwrap().formula(), &target_after);
        assert_eq!(ungated_archive.cardinality(), 2);
    }

    #[test]
    fn formula_set_unfold_def_symbols_duplicate_head_uses_later_definition() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -7112);
        let a = typed_const(&mut bank, "set_unfold_duplicate_a");
        let p_x = typed_unary_predicate(&mut bank, "set_unfold_duplicate_p", &x);
        let q_x = typed_unary_predicate(&mut bank, "set_unfold_duplicate_q", &x);
        let r_x = typed_unary_predicate(&mut bank, "set_unfold_duplicate_r", &x);
        let p_a = typed_unary_predicate(&mut bank, "set_unfold_duplicate_p", &a);
        let r_a = typed_unary_predicate(&mut bank, "set_unfold_duplicate_r", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let true_term = bank.true_term().clone();
        let lhs_formula = bool_binary_with_code(&mut bank, eqn_code, &p_x, &true_term);

        let first_definition_formula =
            bool_binary_with_code(&mut bank, equiv_code, &lhs_formula, &q_x);
        let mut first_definition = WrappedFormula::wt_formula_alloc(first_definition_formula);
        first_definition.set_prop(CP_IS_LAMBDA_DEF);
        let first_definition_entry = first_definition.entry_id();

        let second_definition_formula =
            bool_binary_with_code(&mut bank, equiv_code, &lhs_formula, &r_x);
        let mut second_definition =
            WrappedFormula::wt_formula_alloc(second_definition_formula.clone());
        second_definition.set_prop(CP_IS_LAMBDA_DEF);
        let second_definition_entry = second_definition.entry_id();

        let mut set = FormulaSet::new();
        set.insert(first_definition);
        set.insert(second_definition);
        set.insert(WrappedFormula::wt_formula_alloc(p_a));
        let mut archive = FormulaSet::new();

        let result = set
            .unfold_def_symbols(&mut archive, &mut bank, ProblemType::HigherOrder, true)
            .unwrap();

        assert_eq!(result.formulas_def_symbols_unfolded, 1);
        assert_eq!(result.unfolded_definitions_archived, 1);
        assert_eq!(result.unfolded_original_definitions_archived, 2);
        assert_eq!(result.definition_symbol_applications, 1);
        assert_eq!(set.cardinality(), 1);
        let rewritten = set.iter().next().unwrap();
        assert_eq!(rewritten.formula(), &r_a);
        assert_eq!(archive.cardinality(), 3);

        let generated_definition = archive.iter().next().unwrap();
        let generated_rhs = generated_definition
            .formula()
            .argument(1)
            .expect("generated definition rhs is initialized");
        assert!(term_has_f_code(&generated_rhs, r_x.f_code()));
        assert!(!term_has_f_code(&generated_rhs, q_x.f_code()));
        assert_eq!(
            archive
                .get(first_definition_entry)
                .unwrap()
                .derivation_entries(),
            &[]
        );
        assert_eq!(
            archive.get(second_definition_entry).unwrap().formula(),
            &second_definition_formula
        );
    }

    #[test]
    fn formula_set_unfold_def_symbols_with_docs_prints_changed_formula() {
        let mut bank = test_bank();
        let (definition, target_formula, expected_formula) =
            unfolding_definition_fixture(&mut bank, "set_unfold_def_doc");
        let mut target = WrappedFormula::wt_formula_alloc(target_formula);
        target.set_prop(CP_INPUT_FORMULA);
        let old_ident = target.ident();
        let mut set = FormulaSet::new();
        set.insert(definition);
        set.insert(target);
        let mut archive = FormulaSet::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .unfold_def_symbols_with_docs(
                &mut archive,
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
                true,
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_def_symbols_unfolded, 1);
        assert_eq!(result.preprocess.unfolded_definitions_archived, 1);
        assert_eq!(result.preprocess.unfolded_original_definitions_archived, 1);
        assert_eq!(result.preprocess.definition_symbol_applications, 1);
        assert_eq!(result.write_results, vec![ProofDocWriteResult::printed()]);
        let rewritten = set.iter().next().unwrap();
        let rewritten_body = rewritten
            .proof_doc_formula_body_string(&mut bank, true, ProblemType::HigherOrder)
            .unwrap();
        assert_eq!(
            rendered,
            format!("     1 : :{rewritten_body} : fof_simplification({old_ident})\n")
        );
        assert_eq!(rewritten.formula(), &expected_formula);
        assert_eq!(rewritten.ident(), 1);
        assert!(!rewritten.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 1);
        let generated_definition = archive.iter().next().unwrap();
        let generated_ref = FormulaDerivationRef::new(generated_definition.ident());
        assert_eq!(
            rewritten.derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(generated_ref)
            ]
        );
    }

    #[test]
    fn formula_set_unfold_def_symbols_with_docs_suppresses_but_keeps_property_side_effects() {
        let mut bank = test_bank();
        let (definition, target_formula, expected_formula) =
            unfolding_definition_fixture(&mut bank, "set_unfold_def_doc_suppress");
        let mut target = WrappedFormula::wt_formula_alloc(target_formula);
        target.set_prop(CP_INPUT_FORMULA);
        let old_ident = target.ident();
        let mut set = FormulaSet::new();
        set.insert(definition);
        set.insert(target);
        let mut archive = FormulaSet::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let result = set
            .unfold_def_symbols_with_docs(
                &mut archive,
                &mut rendered,
                &mut bank,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::HigherOrder),
                true,
            )
            .unwrap();

        assert_eq!(result.preprocess.formulas_def_symbols_unfolded, 1);
        assert_eq!(
            result.write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert!(rendered.is_empty());
        let rewritten = set.iter().next().unwrap();
        assert_eq!(rewritten.formula(), &expected_formula);
        assert_eq!(rewritten.ident(), old_ident);
        assert!(!rewritten.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
        let generated_definition = archive.iter().next().unwrap();
        let generated_ref = FormulaDerivationRef::new(generated_definition.ident());
        assert_eq!(
            rewritten.derivation_entries(),
            &[
                DerivationEntry::Operation(DC_APPLY_DEF),
                DerivationEntry::FormulaParent(generated_ref)
            ]
        );
    }

    #[test]
    fn formula_set_cnf2_unfolds_def_symbols_before_archive_drain() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -72);
        let a = typed_const(&mut bank, "set_cnf_unfold_def_a");
        let p_x = typed_unary_predicate(&mut bank, "set_cnf_unfold_def_p", &x);
        let q_x = typed_unary_predicate(&mut bank, "set_cnf_unfold_def_q", &x);
        let p_a = typed_unary_predicate(&mut bank, "set_cnf_unfold_def_p", &a);
        let q_a = typed_unary_predicate(&mut bank, "set_cnf_unfold_def_q", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let true_term = bank.true_term().clone();
        let lhs_formula = bool_binary_with_code(&mut bank, eqn_code, &p_x, &true_term);
        let definition_formula = bool_binary_with_code(&mut bank, equiv_code, &lhs_formula, &q_x);
        let mut definition = WrappedFormula::wt_formula_alloc(definition_formula.clone());
        definition.set_prop(CP_IS_LAMBDA_DEF);
        let definition_entry = definition.entry_id();
        let mut set = FormulaSet::new();
        set.insert(definition);
        set.insert(WrappedFormula::wt_formula_alloc(p_a));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::HigherOrder)
                    .with_lambda_to_forall(false),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_def_symbols_unfolded, 1);
        assert_eq!(result.unfolded_definitions_archived, 1);
        assert_eq!(result.unfolded_original_definitions_archived, 1);
        assert_eq!(result.definition_symbol_applications, 1);
        assert_eq!(result.original_formulas_archived, 1);
        assert!(archive.get(definition_entry).is_some());
        assert!(archive.iter().any(|formula| formula.formula() == &q_a));
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn clause_set_lift_lambdas_archives_and_clausifies_definitions() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let inner = typed_unary(&mut bank, "clause_lift_lambda_body_f", &db0);
        let body = typed_unary(&mut bank, "clause_lift_lambda_body_g", &inner);
        let lambda = close_with_db_var(&mut bank, &i_type, &body).unwrap();
        let lambda_type = lambda.type_().expect("lambda term is typed");
        let wrapped_lambda = typed_unary_with_types(
            &mut bank,
            "clause_lift_lambda_wrapper",
            &lambda,
            &lambda_type,
            &i_type,
        );
        let target = typed_const(&mut bank, "clause_lift_lambda_target");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &wrapped_lambda,
            &target,
            true,
        )]));
        let mut clauses = ClauseSet::from_clauses([clause]);
        let mut archive = FormulaSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result =
            clause_set_lift_lambdas(&mut clauses, &mut archive, &mut bank, &fresh_vars, false)
                .unwrap();

        assert_eq!(result.clauses_changed, 1);
        assert_eq!(result.definitions_archived, 1);
        assert!(result.definition_clauses_generated > 0);
        assert_eq!(result.clause_derivation_ops, vec![DC_LIFT_LAMBDAS]);
        assert_eq!(archive.cardinality(), 2);
        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(
            archived[0].derivation_entries(),
            &[DerivationEntry::Operation(DC_INTRO_DEF)]
        );
        assert_eq!(
            clauses
                .iter()
                .next()
                .unwrap()
                .derivation()
                .unwrap()
                .as_slice()[0],
            DerivationEntry::Operation(DC_LIFT_LAMBDAS)
        );
        let lifted_left = clauses
            .iter()
            .next()
            .unwrap()
            .literals()
            .as_slice()
            .first()
            .unwrap()
            .left()
            .clone();
        assert!(!lifted_left.has_lambda_subterm());
        assert!(clauses.members() > 1);
    }

    #[test]
    fn formula_set_cnf2_lifts_clause_lambdas_after_archive_drain() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let inner = typed_unary(&mut bank, "set_cnf_clause_lift_lambda_body_f", &db0);
        let body = typed_unary(&mut bank, "set_cnf_clause_lift_lambda_body_g", &inner);
        let lambda = close_with_db_var(&mut bank, &i_type, &body).unwrap();
        let lambda_type = lambda.type_().expect("lambda term is typed");
        let wrapped_lambda = typed_unary_with_types(
            &mut bank,
            "set_cnf_clause_lift_lambda_wrapper",
            &lambda,
            &lambda_type,
            &i_type,
        );
        let target = typed_const(&mut bank, "set_cnf_clause_lift_lambda_target");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &wrapped_lambda,
            &target,
            true,
        )]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::HigherOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        let mut formulas = FormulaSet::new();
        formulas.insert(wrapped);
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = formulas
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::HigherOrder),
            )
            .unwrap();

        assert!(formulas.is_empty());
        assert_eq!(result.clauses_generated, 1);
        assert_eq!(result.clauses_lambdas_lifted, 1);
        assert_eq!(result.lambda_lift_definitions_archived, 1);
        assert!(result.lambda_lift_definition_clauses_generated > 0);
        assert!(clauses.members() > result.clauses_generated);
        assert!(archive.cardinality() >= 4);
    }

    #[test]
    fn clause_set_lift_lambdas_reuses_exact_closed_liftings() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let body = typed_unary(&mut bank, "clause_lift_reuse_body", &db0);
        let lambda = close_with_db_var(&mut bank, &i_type, &body).unwrap();
        let lambda_type = lambda.type_().expect("lambda term is typed");
        let wrapped_lambda = typed_unary_with_types(
            &mut bank,
            "clause_lift_reuse_wrapper",
            &lambda,
            &lambda_type,
            &i_type,
        );
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &wrapped_lambda,
            &wrapped_lambda,
            true,
        )]));
        let mut clauses = ClauseSet::new();
        clauses.insert(clause);
        let mut archive = FormulaSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result =
            clause_set_lift_lambdas(&mut clauses, &mut archive, &mut bank, &fresh_vars, false)
                .unwrap();

        assert_eq!(result.clauses_changed, 1);
        assert_eq!(result.definitions_archived, 1);
        assert_eq!(
            result.clause_derivation_ops,
            vec![DC_LIFT_LAMBDAS, DC_LIFT_LAMBDAS]
        );
        assert!(result.definition_clauses_generated > 0);
        assert_eq!(archive.cardinality(), 2);
        let lifted_clause = clauses.iter().next().unwrap();
        let lifted_literal = lifted_clause.literals().as_slice().first().unwrap();
        assert_eq!(lifted_literal.left(), lifted_literal.right());
    }

    #[test]
    fn clause_set_lift_lambdas_reuses_generalized_liftings() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let x = typed_var(&bank, -902);
        let a = typed_const(&mut bank, "clause_lift_generalize_a");
        let generic_body = typed_unary(&mut bank, "clause_lift_generalize_body", &x);
        let specific_body = typed_unary(&mut bank, "clause_lift_generalize_body", &a);
        let generic_lambda = close_with_db_var(&mut bank, &i_type, &generic_body).unwrap();
        let specific_lambda = close_with_db_var(&mut bank, &i_type, &specific_body).unwrap();
        let lambda_type = generic_lambda.type_().expect("lambda term is typed");
        let generic_wrapped = typed_unary_with_types(
            &mut bank,
            "clause_lift_generalize_wrapper",
            &generic_lambda,
            &lambda_type,
            &i_type,
        );
        let specific_wrapped = typed_unary_with_types(
            &mut bank,
            "clause_lift_generalize_wrapper",
            &specific_lambda,
            &lambda_type,
            &i_type,
        );
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &generic_wrapped,
            &specific_wrapped,
            true,
        )]));
        let mut clauses = ClauseSet::new();
        clauses.insert(clause);
        let mut archive = FormulaSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result =
            clause_set_lift_lambdas(&mut clauses, &mut archive, &mut bank, &fresh_vars, false)
                .unwrap();

        assert_eq!(result.clauses_changed, 1);
        assert_eq!(result.definitions_archived, 1);
        assert_eq!(
            result.clause_derivation_ops,
            vec![DC_LIFT_LAMBDAS, DC_LIFT_LAMBDAS]
        );
        assert_eq!(archive.cardinality(), 2);
        let lifted_clause = clauses.iter().next().unwrap();
        let lifted_literal = lifted_clause.literals().as_slice().first().unwrap();
        let left_lifted = lifted_literal
            .left()
            .argument(0)
            .expect("left wrapper contains lifted lambda replacement");
        let right_lifted = lifted_literal
            .right()
            .argument(0)
            .expect("right wrapper contains lifted lambda replacement");
        assert_eq!(left_lifted.f_code(), right_lifted.f_code());
        assert_eq!(left_lifted.argument(0).as_ref(), Some(&x));
        assert_eq!(right_lifted.argument(0).as_ref(), Some(&a));
    }

    #[test]
    fn formula_set_cnf2_drains_inputs_and_archives_originals_then_cnf_copies() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "set_cnf_a");
        let atom_middle = typed_const(&mut bank, "set_cnf_b");
        let atom_right = typed_const(&mut bank, "set_cnf_c");
        let atom_tail = typed_const(&mut bank, "set_cnf_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_middle);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &atom_middle, &atom_right);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &atom_right, &atom_tail);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let original_formula = formula.clone();
        let mut first = WrappedFormula::wt_formula_alloc(formula);
        first.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        first.set_info(Some(ClauseInfo::new(Some("set_cnf_formula"), None, 1, 1)));
        let first_entry = first.entry_id();
        let first_source = FormulaDerivationRef::new(first.ident());

        let clause_left = typed_const(&mut bank, "set_cnf_e");
        let clause_right = typed_const(&mut bank, "set_cnf_f");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &clause_left,
            &clause_right,
            true,
        )]));
        let clause_formula =
            tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let original_clause_formula = clause_formula.clone();
        let mut second = WrappedFormula::wt_formula_alloc(clause_formula);
        second.set_is_clause(true);
        second.set_tptp_type(CP_TYPE_AXIOM);
        let second_entry = second.entry_id();
        let second_source = FormulaDerivationRef::new(second.ident());

        let mut formulas = FormulaSet::new();
        formulas.insert(first);
        formulas.insert(second);
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = formulas
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder),
            )
            .unwrap();

        assert!(formulas.is_empty());
        assert_eq!(archive.cardinality(), 4);
        assert_eq!(clauses.members(), 3);
        assert_eq!(result.clauses_generated, 3);
        assert_eq!(result.original_formulas_archived, 2);
        assert_eq!(result.cnf_formulas_archived, 2);
        assert_eq!(
            result.quoted_formula_sources,
            vec![first_source, second_source]
        );
        assert!(result
            .formula_derivation_ops
            .contains(&DC_DIST_DISJUNCTIONS));

        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(archived[0].entry_id(), first_entry);
        assert_eq!(archived[0].formula(), &original_formula);
        assert_eq!(archived[1].ident(), first_source.ident());
        assert_ne!(archived[1].entry_id(), first_entry);
        assert_eq!(archived[1].info(), None);
        assert_ne!(archived[1].formula(), &original_formula);
        assert_eq!(archived[2].entry_id(), second_entry);
        assert_eq!(archived[2].formula(), &original_clause_formula);
        assert_eq!(archived[3].ident(), second_source.ident());
        assert!(archived[3].is_clause());

        assert_eq!(
            count_formula_set_cnf_derivations(&clauses, first_source, second_source),
            (2, 1)
        );
    }

    #[test]
    fn formula_set_cnf2_simplifies_before_archiving_inputs() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "set_cnf_simpl_left");
        let atom_right = typed_const(&mut bank, "set_cnf_simpl_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let unsimplified = bool_binary_with_code(&mut bank, or_code, &false_formula, &atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(unsimplified.clone()));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_simplified, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOF_SIMPLIFY));
        assert_eq!(result.clauses_generated, 1);
        assert_eq!(clauses.members(), 1);
        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(archived[0].formula(), &atom);
        assert_ne!(archived[0].formula(), &unsimplified);
    }

    #[test]
    fn formula_set_cnf2_unrolls_fool_before_archive_drain() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_cnf_fool_a");
        let b = typed_const(&mut bank, "set_cnf_fool_b");
        let c = typed_const(&mut bank, "set_cnf_fool_c");
        let d = typed_const(&mut bank, "set_cnf_fool_d");
        let target = typed_const(&mut bank, "set_cnf_fool_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let bool_arg = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let applied = typed_unary_with_types(
            &mut bank,
            "set_cnf_fool_fun",
            &bool_arg,
            &bool_type,
            &default_type,
        );
        let formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, true, ProblemType::FirstOrder),
            )
            .unwrap();

        assert_eq!(result.formulas_fool_unrolled, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOOL_UNROLL));
        assert_eq!(archive.iter().next().unwrap().formula().f_code(), and_code);
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_introduces_defs_before_archive_drain() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_cnf_intro_first");
        let second = typed_const(&mut bank, "set_cnf_intro_second");
        let third = typed_const(&mut bank, "set_cnf_intro_third");
        let fourth = typed_const(&mut bank, "set_cnf_intro_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder).with_def_limit(1),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(result.definition_formulas_archived, 1);
        assert_eq!(result.active_definition_formulas_inserted, 1);
        assert_eq!(result.formulas_rewritten_by_defs, 1);
        assert_eq!(result.definition_applications, 1);
        assert_eq!(result.original_formulas_archived, 2);
        assert_eq!(result.cnf_formulas_archived, 2);
        assert!(result.formula_derivation_ops.contains(&DC_INTRO_DEF));
        assert!(result.formula_derivation_ops.contains(&DC_SPLIT_EQUIV));
        assert!(result.formula_derivation_ops.contains(&DC_APPLY_DEF));
        assert_eq!(archive.cardinality(), 5);

        let archived = archive.iter().collect::<Vec<_>>();
        let neutral_definition = archived[0].formula();
        assert_eq!(neutral_definition.f_code(), bank.signature().equiv_code());
        let rename_atom = neutral_definition.argument(0).unwrap();
        let rewritten_original = archived[1].formula();
        assert_eq!(rewritten_original.f_code(), bank.signature().or_code());
        assert_eq!(rewritten_original.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(rewritten_original.argument(1).as_ref(), Some(&tail));
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_with_docs_threads_phase_definition_and_split_output() {
        let mut bank = test_bank();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let simpl_left = typed_const(&mut bank, "set_cnf_doc_simpl_left");
        let simpl_right = typed_const(&mut bank, "set_cnf_doc_simpl_right");
        let simpl_atom = bool_binary_with_code(&mut bank, eqn_code, &simpl_left, &simpl_right);
        let or_code = bank.signature().or_code();
        let simpl_formula = bool_binary_with_code(&mut bank, or_code, &false_formula, &simpl_atom);
        let mut simpl_wrapper = WrappedFormula::wt_formula_alloc(simpl_formula);
        simpl_wrapper.set_prop(CP_INPUT_FORMULA);

        let first = typed_const(&mut bank, "set_cnf_doc_intro_first");
        let second = typed_const(&mut bank, "set_cnf_doc_intro_second");
        let third = typed_const(&mut bank, "set_cnf_doc_intro_third");
        let fourth = typed_const(&mut bank, "set_cnf_doc_intro_fourth");
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let intro_formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut intro_wrapper = WrappedFormula::wt_formula_alloc(intro_formula);
        intro_wrapper.set_prop(CP_INPUT_FORMULA);

        let mut set = FormulaSet::new();
        set.insert(simpl_wrapper);
        set.insert(intro_wrapper);
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = {
            let mut doc_context = WrappedFormulaCnfDocContext::new(
                &mut rendered,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
            );
            set.cnf2_into_with_docs(
                &mut doc_context,
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder).with_def_limit(1),
            )
            .unwrap()
        };

        assert!(set.is_empty());
        assert_eq!(result.cnf.formulas_simplified, 1);
        assert_eq!(result.cnf.definitions_introduced, 1);
        assert_eq!(result.cnf.definition_applications, 1);
        assert_eq!(result.cnf.original_formulas_archived, 3);
        assert_eq!(result.cnf.cnf_formulas_archived, 3);
        assert_eq!(clauses.members(), result.cnf.clauses_generated);
        assert_eq!(
            result.simplification_write_results,
            vec![ProofDocWriteResult::printed()]
        );
        assert_eq!(
            result.definition_write_results,
            vec![
                ProofDocWriteResult::printed(),
                ProofDocWriteResult::printed()
            ]
        );
        assert_eq!(
            result.definition_application_write_results,
            vec![ProofDocWriteResult::printed()]
        );
        assert_eq!(
            result.cnf_clause_write_results.len(),
            usize::try_from(result.cnf.clauses_generated).unwrap()
        );
        assert!(result
            .cnf_clause_write_results
            .iter()
            .all(|write_result| *write_result == ProofDocWriteResult::printed()));

        let simpl_pos = rendered.find("fof_simplification(").unwrap();
        let intro_pos = rendered.find("introduced").unwrap();
        let split_pos = rendered.find("split_equiv(").unwrap();
        let apply_pos = rendered.find("apply_def(").unwrap();
        let clause_pos = rendered.rfind("split_conjunct(").unwrap();
        assert!(simpl_pos < intro_pos);
        assert!(intro_pos < split_pos);
        assert!(split_pos < apply_pos);
        assert!(apply_pos < clause_pos);
        assert!(session.id_source.current_ident() > 0);
    }

    #[test]
    fn formula_set_cnf2_with_docs_suppresses_output_but_keeps_c_side_effects() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_cnf_doc_suppress_a");
        let b = typed_const(&mut bank, "set_cnf_doc_suppress_b");
        let c = typed_const(&mut bank, "set_cnf_doc_suppress_c");
        let d = typed_const(&mut bank, "set_cnf_doc_suppress_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let old_ident = wrapped.ident();
        let mut set = FormulaSet::new();
        set.insert(wrapped);
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let result = {
            let mut doc_context = WrappedFormulaCnfDocContext::new(
                &mut rendered,
                &mut session,
                FormulaProofDocRenderOptions::new(true, ProblemType::FirstOrder),
            );
            set.cnf2_into_with_docs(
                &mut doc_context,
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder),
            )
            .unwrap()
        };

        assert!(rendered.is_empty());
        assert_eq!(session.id_source.current_ident(), 0);
        assert_eq!(result.cnf.original_formulas_archived, 1);
        assert_eq!(result.cnf.cnf_formulas_archived, 1);
        assert_eq!(result.cnf.clauses_generated, 2);
        assert_eq!(
            result.cnf_formula_write_results,
            vec![ProofDocWriteResult::suppressed()]
        );
        assert_eq!(
            result.cnf_clause_write_results,
            vec![
                ProofDocWriteResult::suppressed(),
                ProofDocWriteResult::suppressed()
            ]
        );
        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(archived[0].ident(), old_ident);
        assert!(!archived[1].query_prop(CP_INPUT_FORMULA));
    }

    #[test]
    fn formula_set_cnf2_collects_term_garbage_after_cnf_growth() {
        let mut bank = test_bank();
        let dropped = typed_const(&mut bank, "set_cnf_gc_dropped");
        let first = typed_const(&mut bank, "set_cnf_gc_first");
        let second = typed_const(&mut bank, "set_cnf_gc_second");
        let third = typed_const(&mut bank, "set_cnf_gc_third");
        let fourth = typed_const(&mut bank, "set_cnf_gc_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder).with_def_limit(1),
            )
            .unwrap();

        assert!(set.is_empty());
        assert!(result.term_garbage_collections >= 1);
        assert!(result.terms_recovered_by_gc >= 1);
        assert!(bank.find(&dropped).is_none());
        let archived_formula = archive.iter().next().unwrap().formula().clone();
        assert!(bank.find(&archived_formula).is_some());
    }

    #[test]
    fn formula_set_print_string_preserves_order_and_selected_format() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_print_a");
        let b = typed_const(&mut bank, "set_print_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first_formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let second_formula = bool_binary_with_code(&mut bank, eqn_code, &b, &a);
        let mut first = WrappedFormula::wt_formula_alloc(first_formula);
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_prop(CP_INPUT_FORMULA);
        first.set_info(Some(ClauseInfo::new(Some("set_print_first"), None, 1, 1)));
        let mut second = WrappedFormula::wt_formula_alloc(second_formula);
        second.set_tptp_type(CP_TYPE_QUESTION);
        second.set_info(Some(ClauseInfo::new(Some("set_print_second"), None, 2, 1)));
        let mut set = FormulaSet::new();
        set.insert(first);
        set.insert(second);

        let set_tstp_output = set
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tstp,
                true,
            )
            .unwrap();
        let set_legacy_output = set
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tptp,
                true,
            )
            .unwrap();

        let mut tstp_lines = set_tstp_output.lines();
        assert!(tstp_lines
            .next()
            .is_some_and(|line| line.starts_with("fof(set_print_first, axiom, ")));
        assert!(tstp_lines
            .next()
            .is_some_and(|line| line.starts_with("fof(set_print_second, question, ")));
        assert_eq!(tstp_lines.next(), None);
        assert!(set_legacy_output.starts_with("input_formula(set_print_first,axiom,"));
        assert!(set_legacy_output.contains("\ninput_formula(set_print_second,question,"));
        assert!(set_legacy_output.ends_with('\n'));
    }

    #[test]
    fn formula_set_pretty_print_tstp_declares_typed_symbols_first() {
        let mut bank = test_bank();
        let integer = bank.signature().type_bank().integer_type();
        let int_const = typed_const_with_type(&mut bank, "pretty_int", &integer);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &int_const, &int_const);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(Some("pretty_typed"), None, 1, 1)));
        let mut set = FormulaSet::new();
        set.insert(wrapped);

        let rendered = set
            .pretty_print_tstp_string(&mut bank, true, ProblemType::FirstOrder, true)
            .unwrap();

        let declaration = rendered
            .find("tff(decl_")
            .expect("typed pretty-print emits declarations");
        let formula = rendered
            .find("tff(pretty_typed, axiom, ")
            .expect("typed pretty-print emits formula after declarations");
        assert!(declaration < formula);
        assert!(rendered.contains("pretty_int: $int"));
    }

    #[test]
    fn formula_set_app_encode_preloads_declarations_and_skips_true_formula() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_app_a");
        let b = typed_const(&mut bank, "set_app_b");
        let f_a = typed_unary(&mut bank, "set_app_f", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &f_a, &b);
        let mut axiom = WrappedFormula::wt_formula_alloc(formula);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        axiom.set_prop(CP_INPUT_FORMULA);
        axiom.set_info(Some(ClauseInfo::new(Some("set_app_axiom"), None, 1, 1)));

        let conjecture_formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let mut conjecture = WrappedFormula::wt_formula_alloc(conjecture_formula);
        conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        conjecture.set_info(Some(ClauseInfo::new(Some("set_app_neg_conj"), None, 2, 1)));

        let true_term = bank.true_term().clone();
        let true_formula = bool_binary_with_code(&mut bank, eqn_code, &true_term, &true_term);
        let mut skipped = WrappedFormula::wt_formula_alloc(true_formula);
        skipped.set_info(Some(ClauseInfo::new(Some("set_app_true"), None, 3, 1)));
        let mut skipped_raw = WrappedFormula::wt_formula_alloc(true_term);
        skipped_raw.set_info(Some(ClauseInfo::new(Some("set_app_raw_true"), None, 4, 1)));

        let mut set = FormulaSet::new();
        set.insert(axiom);
        set.insert(skipped);
        set.insert(skipped_raw);
        set.insert(conjecture);

        let rendered = set
            .app_encode_string(&mut bank, ProblemType::FirstOrder, true)
            .unwrap();

        let type_decl = rendered
            .find("tff(typedecl")
            .expect("type declarations are printed before formulas");
        let symbol_decl = rendered
            .find("tff(symboltypedecl")
            .expect("symbol declarations are printed before formulas");
        let axiom_line = rendered
            .find("tff(set_app_axiom, axiom, ")
            .expect("input axiom formula is rendered");
        assert!(type_decl < symbol_decl);
        assert!(symbol_decl < axiom_line);
        assert!(rendered.contains("tff(set_app_neg_conj, negated_conjecture, "));
        assert!(!rendered.contains("set_app_true"));
        assert!(!rendered.contains("set_app_raw_true"));
        assert_eq!(
            FormulaSet::new()
                .app_encode_string(&mut bank, ProblemType::FirstOrder, true)
                .unwrap(),
            ""
        );
    }

    #[test]
    fn formula_set_definition_statistics_matches_c_scan_shapes() {
        let mut bank = test_bank();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let or_code = bank.signature().or_code();

        let func_head = typed_const(&mut bank, "stats_function_head");
        let func_rhs = typed_const(&mut bank, "stats_function_rhs");
        let function_equation = bool_binary_with_code(&mut bank, eqn_code, &func_head, &func_rhs);
        let mut function_definition = WrappedFormula::wt_formula_alloc(function_equation);
        function_definition.set_prop(CP_IS_LAMBDA_DEF);

        let predicate_head = typed_predicate_const(&mut bank, "stats_predicate_head");
        let true_term = bank.true_term().clone();
        let predicate_equation =
            bool_binary_with_code(&mut bank, eqn_code, &predicate_head, &true_term);
        let true_formula = bank.true_term().clone();
        let predicate_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut predicate_definition = WrappedFormula::wt_formula_alloc(predicate_equivalence);
        predicate_definition.set_prop(CP_IS_LAMBDA_DEF);

        let loose_head = typed_predicate_const(&mut bank, "stats_loose_predicate_head");
        let loose_rhs = typed_predicate_const(&mut bank, "stats_loose_predicate_rhs");
        let loose_equation = bool_binary_with_code(&mut bank, eqn_code, &loose_head, &loose_rhs);
        let true_formula = bank.true_term().clone();
        let loose_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &loose_equation, &true_formula);
        let mut loose_definition = WrappedFormula::wt_formula_alloc(loose_equivalence);
        loose_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            loose_definition.get_lambda_defined_symbol(bank.signature()),
            Some(loose_head.f_code())
        );

        let binder_type = bank.signature().type_bank().default_type();
        let lambda_body = typed_const(&mut bank, "stats_lambda_body");
        let lambda = close_with_db_var(&mut bank, &binder_type, &lambda_body).unwrap();
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let lambda_type = lambda.type_().expect("lambda is typed");
        let container_ret_type = bank.signature().type_bank().default_type();
        let lambda_container = typed_unary_with_types(
            &mut bank,
            "stats_lambda_container",
            &lambda,
            &lambda_type,
            &container_ret_type,
        );
        let lambda_rhs = typed_const(&mut bank, "stats_lambda_rhs");
        let lambda_literal =
            bool_binary_with_code(&mut bank, eqn_code, &lambda_container, &lambda_rhs);

        let app_head = typed_var(&bank, -101);
        let app_arg = typed_const(&mut bank, "stats_app_arg");
        let app_var = phony_app(&mut bank, &app_head, &app_arg);
        let app_literal = bool_binary_with_code(&mut bank, eqn_code, &app_var, &true_term);
        let normal_literal = bool_binary_with_code(&mut bank, eqn_code, &func_rhs, &func_rhs);
        let app_formula = bool_binary_with_code(&mut bank, or_code, &app_literal, &normal_literal);

        let mut orig = FormulaSet::new();
        orig.insert(function_definition);
        orig.insert(loose_definition);
        orig.insert(WrappedFormula::wt_formula_alloc(lambda_literal));
        let mut arch = FormulaSet::new();
        arch.insert(predicate_definition);
        arch.insert(WrappedFormula::wt_formula_alloc(app_formula));

        let stats = formula_set_definition_statistics(&orig, &arch, &bank);

        assert_eq!(stats.num_defs, 2);
        assert!((stats.percentage_form_defs - 0.5).abs() < f64::EPSILON);
        assert_eq!(stats.num_lams, 1);
        assert!(stats.has_app_var_lits);

        let empty = FormulaSet::new();
        assert_eq!(
            formula_set_definition_statistics(&empty, &empty, &bank),
            FormulaDefinitionStatistics::default()
        );
    }
}
