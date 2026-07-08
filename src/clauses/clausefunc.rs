use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_print_lop_format_string, clause_print_tptp_format_string, clause_tstp_string, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_D_INDEXED, CP_IS_PURE_INJECTIVITY,
    CP_IS_SOS, CP_IS_S_INDEXED, CP_LIMITED_RW,
};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_push_derivation, clause_push_formula_derivation, op_has_cnf_arg1, op_has_cnf_arg2,
    op_is_generating, ClauseDerivationRef, DerivationEntry, DerivationParentRef,
    FormulaDerivationRef, DC_CNF_ADD_ARG, DC_CNF_QUOTE, DC_DIST_DISJUNCTIONS, DC_ELIMINATE_BVAR,
    DC_FLEX_RESOLVE, DC_FNNF, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL, DC_INV_REC, DC_NORMALIZE,
    DC_PRUNE_ARG, DC_SHIFT_QUANTORS, DC_SKOLEMIZE, DC_SPLIT_CONJUNCT, DC_VAR_RENAME,
};
use crate::clauses::eqn::{
    eqn_fof_parse, eqn_write_app_encode, eqn_write_app_encode_with_type_suffixes, eqn_write_fof,
    Eqn, EqnFofPrintOptions,
};
use crate::clauses::eqn_props::{
    PatEqnDirection, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE,
};
use crate::clauses::eqnlist::EqnList;
use crate::clauses::inferencedoc::{FormulaDocView, ProofDocSession, ProofDocWriteResult};
use crate::inout::scanner::{token_pos_rep, IoFormat, Scanner, TokenType};
use crate::terms::functypes::{func_symb_start_token, FunCode};
use crate::terms::lambda::{
    apply_terms, beta_normalize_db, close_with_db_var, close_with_type_prefix,
    decode_formulas_for_cnf, flatten_apps, lambda_eta_reduce_db, post_cnf_encode_formulas,
};
use crate::terms::match_mgu::subst_mgu_complete;
use crate::terms::replace::tb_term_pos_replace;
use crate::terms::signature::{
    FP_FOF_OP, FP_IS_INJ_DEF_SKOLEM, SIG_FALSE_CODE, SIG_ITE_CODE, SIG_LET_CODE,
    SIG_NAMED_LAMBDA_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
};
use crate::terms::simpletypes::{
    arrow_type_flattened, type_app_encoded_name, type_get_max_arity, type_is_predicate, Type,
    ST_BOOL,
};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_app_encode, term_find_max_var_code, term_is_db_closed, term_is_ground, term_is_untyped,
    term_standard_weight,
};
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{
    term_del_prop, term_identity_id, DerefType, Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT,
    TP_CHECK_FLAG, TP_NEG_POLARITY, TP_OP_FLAG, TP_POS_POLARITY, TP_PRED_POS,
};
use crate::terms::termvars::VarBank;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const TFORM_MANY_CLAUSES: i64 = i64::MAX;
const TFORM_MANY_LIMIT: i64 = 1024;
const TFORM_CNF_SIMPLIFY_LIMIT: i64 = 1000;

pub type TFormulaDefinitions = BTreeMap<i64, TFormulaDefEntry>;

#[derive(Clone)]
pub struct TFormulaDefEntry {
    polarity: i32,
    rename_atom: Term,
    real_definition_id: Option<i64>,
    archived_definition: Option<Term>,
    archived_definition_ref: Option<FormulaDerivationRef>,
}

impl TFormulaDefEntry {
    #[must_use]
    pub const fn polarity(&self) -> i32 {
        self.polarity
    }

    #[must_use]
    pub fn rename_atom(&self) -> &Term {
        &self.rename_atom
    }

    #[must_use]
    pub const fn real_definition_id(&self) -> Option<i64> {
        self.real_definition_id
    }

    #[must_use]
    pub fn archived_definition(&self) -> Option<&Term> {
        self.archived_definition.as_ref()
    }

    #[must_use]
    pub const fn archived_definition_ref(&self) -> Option<FormulaDerivationRef> {
        self.archived_definition_ref
    }

    pub fn set_definition_metadata(
        &mut self,
        real_definition_id: i64,
        archived_definition: Term,
        archived_definition_ref: FormulaDerivationRef,
    ) {
        self.real_definition_id = Some(real_definition_id);
        self.archived_definition = Some(archived_definition);
        self.archived_definition_ref = Some(archived_definition_ref);
    }
}

/// Term-level result of a staged formula CNF pipeline.
///
/// The term is the transformed formula. `derivation_ops` records the C
/// derivation opcodes that `WTFormulaConjunctiveNF*` would push when a phase
/// changes the wrapped formula. `changed_phases` records the formula snapshot
/// after each phase that contributes a derivation opcode, so a represented
/// wrapper can reproduce C's proof-documentation order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TFormulaCnfResult {
    formula: Term,
    derivation_ops: Vec<i64>,
    changed_phases: Vec<TFormulaCnfPhase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TFormulaCnfPhase {
    op: i64,
    formula: Term,
}

impl TFormulaCnfPhase {
    #[must_use]
    pub const fn op(&self) -> i64 {
        self.op
    }

    #[must_use]
    pub fn formula(&self) -> &Term {
        &self.formula
    }
}

impl TFormulaCnfResult {
    #[must_use]
    pub fn formula(&self) -> &Term {
        &self.formula
    }

    #[must_use]
    pub fn into_formula(self) -> Term {
        self.formula
    }

    #[must_use]
    pub fn derivation_ops(&self) -> &[i64] {
        &self.derivation_ops
    }

    #[must_use]
    pub fn changed_phases(&self) -> &[TFormulaCnfPhase] {
        &self.changed_phases
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TFormulaToCnfDocResult {
    pub clauses_generated: i64,
    pub write_results: Vec<ProofDocWriteResult>,
}

#[derive(Clone, Copy, Debug)]
pub struct TFormulaToCnfInput<'a> {
    form: &'a Term,
    type_: FormulaProperties,
    fresh_vars: &'a VarBank,
    source: FormulaDerivationRef,
    problem_type: ProblemType,
}

impl<'a> TFormulaToCnfInput<'a> {
    #[must_use]
    pub const fn new(
        form: &'a Term,
        type_: FormulaProperties,
        fresh_vars: &'a VarBank,
        source: FormulaDerivationRef,
        problem_type: ProblemType,
    ) -> Self {
        Self {
            form,
            type_,
            fresh_vars,
            source,
            problem_type,
        }
    }
}

pub struct TFormulaToCnfDocContext<'a, 'view, W: fmt::Write> {
    output: &'a mut W,
    session: &'a mut ProofDocSession,
    parent: &'a FormulaDocView<'view>,
}

impl<'a, 'view, W: fmt::Write> TFormulaToCnfDocContext<'a, 'view, W> {
    #[must_use]
    pub const fn new(
        output: &'a mut W,
        session: &'a mut ProofDocSession,
        parent: &'a FormulaDocView<'view>,
    ) -> Self {
        Self {
            output,
            session,
            parent,
        }
    }
}

/// Term-level result of C `TFormulaUnrollFOOL`.
///
/// `formula` is the final formula after literal expansion and FOOL unrolling.
/// `fool_unrolled` reports only whether the `do_fool_unroll` mapper changed
/// the already-expanded formula, matching the boolean returned by C
/// `TFormulaUnrollFOOL`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TFormulaFoolUnrollResult {
    formula: Term,
    fool_unrolled: bool,
}

impl TFormulaFoolUnrollResult {
    #[must_use]
    pub fn formula(&self) -> &Term {
        &self.formula
    }

    #[must_use]
    pub fn into_formula(self) -> Term {
        self.formula
    }

    #[must_use]
    pub const fn fool_unrolled(&self) -> bool {
        self.fool_unrolled
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TFormulaTptpPrintOptions {
    pub problem_type: ProblemType,
    pub eqn_options: EqnFofPrintOptions,
}

impl TFormulaTptpPrintOptions {
    #[must_use]
    pub const fn new(problem_type: ProblemType, eqn_options: EqnFofPrintOptions) -> Self {
        Self {
            problem_type,
            eqn_options,
        }
    }

    #[must_use]
    pub const fn tptp(problem_type: ProblemType) -> Self {
        Self::new(problem_type, EqnFofPrintOptions::tptp())
    }

    #[must_use]
    pub const fn tstp(problem_type: ProblemType) -> Self {
        Self::new(problem_type, EqnFofPrintOptions::tstp())
    }
}

#[must_use]
pub fn pstack_clause_print_lop_string(
    bank: &TermBank,
    stack: &PStack<&Clause>,
    extra: Option<&str>,
) -> String {
    let mut output = String::new();
    for clause in stack.as_slice() {
        output.push_str(&clause_print_lop_format_string(bank, clause, true));
        if let Some(extra) = extra {
            output.push_str(extra);
        }
        output.push('\n');
    }
    output
}

/// Returns the C `PStackClausePrint` shape with explicit `ClausePrint` dispatch.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects a selected clause.
pub fn pstack_clause_print_format_string(
    bank: &TermBank,
    stack: &PStack<&Clause>,
    extra: Option<&str>,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    for clause in stack.as_slice() {
        output.push_str(&pstack_clause_rendered_string(
            bank,
            clause,
            output_format,
            problem_type,
        )?);
        if let Some(extra) = extra {
            output.push_str(extra);
        }
        output.push('\n');
    }
    Ok(output)
}

fn pstack_clause_rendered_string(
    bank: &TermBank,
    clause: &Clause,
    output_format: IoFormat,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string(bank, clause)),
        IoFormat::Tstp => clause_tstp_string(bank, clause, true, true, problem_type),
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string(bank, clause, true)),
    }
}

pub fn clause_archive(
    archive: &mut ClauseSet,
    clause: Clause,
    bank: &mut TermBank,
) -> Result<Clause, Diagnostic> {
    let mut new_clause = clause.flat_copy(bank)?;
    clause_push_derivation(&mut new_clause, DC_CNF_QUOTE, Some(&clause), None);
    archive.insert(clause);
    Ok(new_clause)
}

pub fn clause_archive_copy(
    archive: &mut ClauseSet,
    clause: &mut Clause,
    bank: &mut TermBank,
) -> Result<ClauseDerivationRef, Diagnostic> {
    let mut archived = clause.flat_copy(bank)?;
    archived.set_info(clause.take_info());
    archived.set_derivation(clause.take_derivation());
    let archived_ref = ClauseDerivationRef::from(&archived);

    clause_push_derivation(clause, DC_CNF_QUOTE, Some(&archived), None);
    archive.insert(archived);
    Ok(archived_ref)
}

pub fn clause_set_archive_copy(
    archive: &mut ClauseSet,
    set: &mut ClauseSet,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let mut archived = 0;
    for clause in set.iter_mut() {
        let _ = clause_archive_copy(archive, clause, bank)?;
        archived += 1;
    }
    Ok(archived)
}

pub fn clause_is_orphaned_with(
    clause: &Clause,
    mut parent_is_dead: impl FnMut(DerivationParentRef) -> bool,
) -> bool {
    let Some(derivation) = clause.derivation() else {
        return false;
    };
    let entries = derivation.as_slice();
    let Some(DerivationEntry::Operation(op)) = entries.first() else {
        return false;
    };
    if !op_is_generating(*op) {
        return false;
    }

    let mut index = 1;
    if op_has_cnf_arg1(*op) {
        if derivation_parent_is_dead(entries, index, &mut parent_is_dead) {
            return true;
        }
        index += 1;
    }
    if op_has_cnf_arg2(*op) {
        if derivation_parent_is_dead(entries, index, &mut parent_is_dead) {
            return true;
        }
        index += 1;
    }

    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            break;
        };
        if op != DC_CNF_ADD_ARG {
            break;
        }
        index += 1;
        if derivation_parent_is_dead(entries, index, &mut parent_is_dead) {
            return true;
        }
        index += 1;
    }

    false
}

pub fn clause_set_delete_orphans_with(
    set: &mut ClauseSet,
    mut parent_is_dead: impl FnMut(DerivationParentRef) -> bool,
) -> i64 {
    for clause in set.iter_mut() {
        if clause_is_orphaned_with(clause, &mut parent_is_dead) {
            clause.set_prop(CP_DELETE_CLAUSE);
        } else {
            clause.del_prop(CP_DELETE_CLAUSE);
        }
    }
    set.delete_marked_entries()
}

fn derivation_parent_is_dead(
    entries: &[DerivationEntry],
    index: usize,
    parent_is_dead: &mut impl FnMut(DerivationParentRef) -> bool,
) -> bool {
    let entry = entries
        .get(index)
        .unwrap_or_else(|| panic!("orphan-check derivation parent is missing"));
    let parent = match entry {
        DerivationEntry::ClauseParent(parent) => DerivationParentRef::Clause(*parent),
        DerivationEntry::FormulaParent(parent) => DerivationParentRef::Formula(*parent),
        DerivationEntry::Demodulator(demodulator) => DerivationParentRef::Demodulator(*demodulator),
        DerivationEntry::Operation(_) | DerivationEntry::NumericArg(_) => {
            panic!("orphan-check derivation parent has the wrong entry shape")
        }
    };
    parent_is_dead(parent)
}

pub fn clause_remove_literal_index(clause: &mut Clause, index: usize) -> Option<Eqn> {
    let literal = clause.literals_mut().extract_element(index)?;
    clause.recompute_lit_counts();
    clause.set_weight(clause.weight() - literal.standard_weight());
    Some(literal)
}

pub fn clause_remove_literal(clause: &mut Clause, literal: &Eqn) -> Option<Eqn> {
    let index = clause
        .literals()
        .as_slice()
        .iter()
        .position(|candidate| candidate == literal)?;
    clause_remove_literal_index(clause, index)
}

pub fn clause_flip_literal_sign_index(clause: &mut Clause, index: usize) -> bool {
    let Some(literal) = clause.literals_mut().as_mut_slice().get_mut(index) else {
        return false;
    };
    literal.flip_prop(crate::clauses::eqn_props::EP_IS_POSITIVE);
    clause.recompute_lit_counts();
    true
}

/// Removes duplicate literals and literals already resolved by reflexivity.
///
/// # Panics
///
/// Panics if `clause` is currently discrimination- or subsumption-indexed, matching the C
/// assertion that indexed clauses must be removed from their indexes before mutation.
pub fn clause_remove_superfluous_literals(clause: &mut Clause, bank: &TermBank) -> usize {
    assert!(
        !clause.is_any_prop_set(CP_IS_D_INDEXED | CP_IS_S_INDEXED),
        "indexed clauses must be removed from indexes before literal cleanup"
    );

    let removed_resolved = clause.literals_mut().remove_resolved(bank);
    let removed_duplicates = clause.literals_mut().remove_duplicates(bank);
    let removed = removed_resolved + removed_duplicates;
    if removed != 0 {
        clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
        clause.recompute_lit_counts();
        clause.set_weight(clause.standard_weight());
        clause_push_derivation(clause, DC_NORMALIZE, None, None);
    }
    removed
}

pub fn clause_set_remove_superfluous_literals(set: &mut ClauseSet, bank: &TermBank) -> i64 {
    let removed: usize = set
        .iter_mut()
        .map(|clause| clause_remove_superfluous_literals(clause, bank))
        .sum();
    if removed != 0 {
        set.recompute_literals();
    }
    usize_to_i64(removed)
}

pub fn clause_set_canonize(set: &mut ClauseSet, bank: &TermBank) {
    for clause in set.iter_mut() {
        let _ = clause_remove_superfluous_literals(clause, bank);
        clause.canonize(bank);
    }
    set.recompute_literals();
    set.sort_by(|left, right| cmp_i64_to_order(left.struct_weight_lex_compare(right, bank)));
}

pub fn clause_remove_ac_resolved(clause: &mut Clause, bank: &TermBank) -> usize {
    if clause.negative_literal_count() == 0 {
        return 0;
    }
    let removed = clause.literals_mut().remove_ac_resolved(bank);
    if removed != 0 {
        clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
        clause.recompute_lit_counts();
        clause.set_weight(clause.standard_weight());
    }
    removed
}

#[must_use]
/// Tests whether a unit clause can simplify-reflect a target clause.
///
/// # Panics
///
/// Panics if `simplifier` is not unit, or if its sole literal is a positive oriented equation.
pub fn clause_unit_simplify_test(clause: &Clause, simplifier: &Clause) -> bool {
    assert!(simplifier.is_unit(), "simplifier must be unit");
    let simplifier_literal = &simplifier.literals().as_slice()[0];
    assert!(
        simplifier_literal.is_negative() || !simplifier_literal.is_oriented(),
        "positive unit simplifier must not be oriented"
    );

    let positive = simplifier_literal.is_positive();
    if positive == clause.is_positive() {
        return false;
    }

    clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| (positive != literal.is_positive()) && simplifier_literal.subsume_p(literal))
}

/// Eliminates naked Boolean-variable literals by substituting the variable with
/// the opposite truth value and simplifying the resulting clause.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding the substituted literal list through the
/// term bank fails.
///
/// # Panics
///
/// Panics if a literal reports the C `EqnIsBoolVar` shape but does not have a
/// free variable on the left-hand side, matching the C assertion.
pub fn clause_eliminate_naked_boolean_variables(
    clause: &mut Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    if clause.is_empty() {
        return Ok(false);
    }

    let true_term = bank.true_term().clone();
    let false_term = bank.false_term().clone();
    let mut substitution = Substitution::new();
    let mut eliminated_var = false;
    let mut became_tautology = false;

    for literal in clause.literals_mut().as_mut_slice() {
        if !literal.is_bool_var(bank) {
            continue;
        }

        let variable = literal.left().clone();
        assert!(
            variable.is_free_var(),
            "Boolean literal left side must be a free variable"
        );

        if literal.is_positive() {
            if variable
                .binding()
                .as_ref()
                .is_some_and(|binding| binding == &true_term)
            {
                became_tautology = true;
                break;
            }
            if variable.binding().is_none() {
                substitution.add_binding(&variable, &false_term);
            }
            literal.del_prop(EP_IS_POSITIVE);
        } else {
            if variable
                .binding()
                .as_ref()
                .is_some_and(|binding| binding == &false_term)
            {
                became_tautology = true;
                break;
            }
            if variable.binding().is_none() {
                substitution.add_binding(&variable, &true_term);
            }
        }

        literal.set_left_raw(true_term.clone());
        eliminated_var = true;
    }

    if became_tautology {
        clause.replace_literals(EqnList::from_vec(vec![Eqn::create_true_lit(bank)?]));
    }

    if eliminated_var {
        let copied = clause.literals().copy_opt(bank)?;
        clause.replace_literals(copied);
        let removed = clause_remove_superfluous_literals(clause, bank);
        clause.recompute_lit_counts();
        if removed == 0 {
            clause_push_derivation(clause, DC_NORMALIZE, None, None);
        }
    }
    if eliminated_var || became_tautology {
        clause.set_weight(clause.standard_weight());
    }

    let result = clause.literals().find_true(bank).is_some();
    substitution.delete();
    Ok(result)
}

/// Applies C `NormalizeEquations`.
///
/// This lifts encoded `$eq`/`$neq` Boolean terms and strips encoded `$not`
/// prefixes from predicate-literal left sides.
///
/// # Panics
///
/// Panics if an encoded `$not`, `$eq`, or `$neq` term has uninitialized
/// arguments, matching the C direct argument access.
pub fn clause_normalize_equations(clause: &mut Clause, bank: &TermBank) -> bool {
    let mut normalized = false;

    for literal in clause.literals_mut().as_mut_slice() {
        if normalize_encoded_equation_literal(literal, bank) {
            normalized = true;
        }
    }

    if normalized {
        clause.recompute_lit_counts();
        let _ = clause_remove_superfluous_literals(clause, bank);
        clause.set_weight(clause.standard_weight());
        clause_push_derivation(clause, DC_NORMALIZE, None, None);
    }

    normalized
}

fn normalize_encoded_equation_literal(literal: &mut Eqn, bank: &TermBank) -> bool {
    let true_term = bank.true_term().clone();
    let false_term = bank.false_term().clone();
    let eqn_code = bank.signature().eqn_code();
    let neqn_code = bank.signature().neqn_code();
    let not_code = bank.signature().not_code();
    let mut normalized = false;

    if literal.left() == &true_term && literal.right() != &true_term {
        literal.swap_sides_simple();
        literal.del_prop(EP_IS_EQU_LITERAL | EP_MAX_IS_UP_TO_DATE | EP_IS_ORIENTED);
        normalized = true;
    }

    if literal.right() == &true_term
        && matches!(literal.left().f_code(), code if code == eqn_code || code == neqn_code || code == not_code)
    {
        let mut negate = false;
        let mut left = literal.left().clone();
        while left.f_code() == not_code {
            assert_eq!(left.arity(), 1, "encoded $not term must be unary");
            negate = !negate;
            left = formula_argument(&left, 0);
        }

        let mut right = true_term.clone();
        if left.f_code() == eqn_code || left.f_code() == neqn_code {
            let encoded = left;
            left = formula_argument(&encoded, 0);
            right = formula_argument(&encoded, 1);
            if encoded.f_code() == neqn_code {
                negate = !negate;
            }
        }

        if left == false_term {
            left = true_term.clone();
            negate = !negate;
        }
        if right == false_term {
            right = true_term.clone();
            negate = !negate;
        }
        if left == true_term {
            std::mem::swap(&mut left, &mut right);
        }

        literal.set_left_raw(left);
        literal.set_right_raw(right);
        if literal.right() == &true_term {
            literal.del_prop(EP_IS_EQU_LITERAL);
        } else {
            literal.set_prop(EP_IS_EQU_LITERAL);
        }
        if negate {
            literal.flip_prop(EP_IS_POSITIVE);
        }
        literal.del_prop(EP_MAX_IS_UP_TO_DATE | EP_IS_ORIENTED);
        normalized = true;
    }

    normalized
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FlexResolveVarSign {
    Positive,
    Negative,
    InEquality,
}

/// Applies C `ResolveFlexClause`.
///
/// A resolvable clause is replaced by the empty clause and marked with the
/// `flex_resolve` derivation operation.
///
/// # Panics
///
/// Panics if a non-equational literal has `$true` as its left term, matching
/// the C assertion in `ResolveFlexClause`.
pub fn clause_resolve_flex_clause(clause: &mut Clause, bank: &TermBank) -> bool {
    let mut variable_signs = BTreeMap::new();

    let is_resolvable = clause
        .literals()
        .as_slice()
        .iter()
        .all(|literal| flex_literal_is_resolvable(literal, bank, &mut variable_signs));

    if is_resolvable {
        clause.replace_literals(EqnList::new());
        clause.set_weight(clause.standard_weight());
        clause_push_derivation(clause, DC_FLEX_RESOLVE, None, None);
    }

    is_resolvable
}

fn flex_literal_is_resolvable(
    literal: &Eqn,
    bank: &TermBank,
    variable_signs: &mut BTreeMap<i64, FlexResolveVarSign>,
) -> bool {
    if literal.is_equ_lit(bank) {
        return flex_equ_literal_is_resolvable(literal, variable_signs);
    }

    assert!(
        literal.left() != bank.true_term(),
        "non-equational flex literal must not be $true"
    );

    let Some(variable_code) = top_level_free_var_code(literal.left()) else {
        return false;
    };
    let sign = if literal.is_positive() {
        FlexResolveVarSign::Positive
    } else {
        FlexResolveVarSign::Negative
    };

    if let Some(previous) = variable_signs.get(&variable_code).copied() {
        previous == sign
    } else {
        variable_signs.insert(variable_code, sign);
        true
    }
}

fn flex_equ_literal_is_resolvable(
    literal: &Eqn,
    variable_signs: &mut BTreeMap<i64, FlexResolveVarSign>,
) -> bool {
    if !literal.is_negative()
        || !literal.left().is_top_level_free_var()
        || !literal.right().is_top_level_free_var()
    {
        return false;
    }

    if !literal
        .left()
        .type_()
        .is_some_and(|type_| type_is_predicate(&type_))
    {
        return true;
    }

    let left_code = top_level_free_var_code(literal.left())
        .unwrap_or_else(|| panic!("left flex equality term must have a free-variable head"));
    let right_code = top_level_free_var_code(literal.right())
        .unwrap_or_else(|| panic!("right flex equality term must have a free-variable head"));

    if variable_signs.contains_key(&left_code) || variable_signs.contains_key(&right_code) {
        return false;
    }

    variable_signs.insert(left_code, FlexResolveVarSign::InEquality);
    variable_signs.insert(right_code, FlexResolveVarSign::InEquality);
    true
}

fn top_level_free_var_code(term: &Term) -> Option<i64> {
    if term.is_free_var() {
        Some(term.f_code())
    } else if term.is_applied_free_var() {
        Some(
            term.argument(0)
                .unwrap_or_else(|| panic!("applied free variable must have a head"))
                .f_code(),
        )
    } else {
        None
    }
}

/// Applies C `BooleanSimplification` to a clause.
///
/// The mapped term-level simplifier follows C `TFormulaSimplifyDecoded` for
/// decoded Boolean formulas used by forward contraction.
///
/// # Errors
///
/// Returns a diagnostic if the formula rebuild needs an unavailable signature
/// arity, if term-bank insertion fails, or if a lambda body is unexpectedly
/// untyped.
pub fn clause_boolean_simplification(
    clause: &mut Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let mut changed = false;
    let mut is_tautology = false;

    for literal in clause.literals_mut().as_mut_slice() {
        let old_left = literal.left().clone();
        let old_right = literal.right().clone();
        let new_left = tformula_simplify_decoded(bank, &old_left, true)?;
        let new_right = tformula_simplify_decoded(bank, &old_right, true)?;
        if new_left != old_left || new_right != old_right {
            changed = true;
        }

        literal.map_terms(bank, |term| {
            if *term == old_left {
                new_left.clone()
            } else if *term == old_right {
                new_right.clone()
            } else {
                term.clone()
            }
        });
        if literal.is_true(bank) {
            is_tautology = true;
            break;
        }
    }

    if changed {
        clause.recompute_lit_counts();
        let removed_resolved = clause.literals_mut().remove_resolved(bank);
        let removed_duplicates = clause.literals_mut().remove_duplicates(bank);
        if removed_resolved + removed_duplicates != 0 {
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            clause.recompute_lit_counts();
        }
        clause.set_weight(clause.standard_weight());
        clause_push_derivation(clause, DC_NORMALIZE, None, None);
    }

    Ok(is_tautology)
}

fn tformula_simplify_decoded(
    bank: &mut TermBank,
    formula: &Term,
    unroll_implications: bool,
) -> Result<Term, Diagnostic> {
    if formula.is_db_var() {
        return Ok(formula.clone());
    }

    let sig = bank.signature();
    if matches!(formula.f_code(), code if code == sig.or_code() || code == sig.and_code()) {
        return simplify_decoded_and_or(bank, formula, unroll_implications);
    }
    if formula.f_code() == sig.not_code() {
        return match formula.arity() {
            1 => {
                let arg = formula_argument(formula, 0);
                negate_decoded_formula(bank, &arg)
            }
            _ => Ok(formula.clone()),
        };
    }
    if formula.f_code() == sig.impl_code() {
        return simplify_decoded_implication(bank, formula, unroll_implications);
    }
    if matches!(formula.f_code(), code if code == sig.equiv_code()
        || code == sig.xor_code()
        || code == sig.eqn_code()
        || code == sig.neqn_code())
    {
        return simplify_decoded_equivalence_like(bank, formula);
    }
    if matches!(formula.f_code(), code if code == sig.qex_code() || code == sig.qall_code()) {
        return simplify_decoded_quantifier(bank, formula);
    }

    simplify_decoded_args(bank, formula, true)
}

fn simplify_decoded_args(
    bank: &mut TermBank,
    formula: &Term,
    unroll_implications: bool,
) -> Result<Term, Diagnostic> {
    if formula.is_any_var() || formula.arity() == 0 {
        return Ok(formula.clone());
    }

    let copy = Term::top_copy_without_args(formula);
    let mut changed = false;
    for (index, arg) in formula.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        let simplified = tformula_simplify_decoded(bank, &arg, unroll_implications)?;
        if simplified != arg {
            changed = true;
        }
        copy.set_argument(index, simplified);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(formula.clone())
    }
}

fn simplify_decoded_and_or(
    bank: &mut TermBank,
    formula: &Term,
    unroll_implications: bool,
) -> Result<Term, Diagnostic> {
    let is_or = formula.f_code() == bank.signature().or_code();
    let neutral_element = if is_or {
        bank.false_term().clone()
    } else {
        bank.true_term().clone()
    };
    let absorbing_element = if is_or {
        bank.true_term().clone()
    } else {
        bank.false_term().clone()
    };

    match formula.arity() {
        1 => {
            let simplified = simplify_decoded_args(bank, formula, true)?;
            let arg = formula_argument(&simplified, 0);
            let bool_type = bank.signature().type_bank().bool_type();
            if arg == neutral_element {
                let body = bank.request_db_var(&bool_type, 0);
                close_with_db_var(bank, &bool_type, &body)
            } else if arg == absorbing_element {
                close_with_db_var(bank, &bool_type, &arg)
            } else {
                Ok(formula.clone())
            }
        }
        2 => {
            let mut changed = false;
            let mut args = Vec::new();
            unroll_binary_formula(formula, formula.f_code(), &mut args);

            let mut simplified_args = Vec::new();
            for arg in args {
                let simplified = tformula_simplify_decoded(bank, &arg, unroll_implications)?;
                if simplified != arg {
                    changed = true;
                }
                if simplified == neutral_element {
                    changed = true;
                } else if simplified == absorbing_element {
                    return Ok(absorbing_element);
                } else {
                    simplified_args.push(simplified);
                }
            }

            simplified_args.sort_by_key(term_identity_id);
            let deduped = dedup_sorted_terms(simplified_args);
            if deduped.removed_duplicate {
                changed = true;
            }

            if contains_decoded_complement(bank, &deduped.terms)? {
                return Ok(absorbing_element);
            }

            if !changed {
                Ok(formula.clone())
            } else if deduped.terms.is_empty() {
                Ok(neutral_element)
            } else {
                fold_and_or(bank, deduped.terms, formula.f_code())
            }
        }
        _ => Ok(formula.clone()),
    }
}

fn simplify_decoded_implication(
    bank: &mut TermBank,
    formula: &Term,
    unroll_implications: bool,
) -> Result<Term, Diagnostic> {
    let nested_implication = formula.arity() == 2
        && formula_argument(formula, 1).f_code() == bank.signature().impl_code();
    let formula = simplify_decoded_args(bank, formula, unroll_implications && !nested_implication)?;
    if formula.arity() != 2 {
        return Ok(formula);
    }

    if unroll_implications {
        let mut precedent = Vec::new();
        let mut consequent = Vec::new();
        let mut current = formula.clone();
        while current.f_code() == bank.signature().impl_code() && current.arity() == 2 {
            unroll_binary_formula(
                &formula_argument(&current, 0),
                bank.signature().and_code(),
                &mut precedent,
            );
            current = formula_argument(&current, 1);
        }
        unroll_binary_formula(&current, bank.signature().or_code(), &mut consequent);
        precedent.sort_by_key(term_identity_id);
        for term in consequent {
            if precedent
                .binary_search_by_key(&term_identity_id(&term), term_identity_id)
                .is_ok()
            {
                return Ok(bank.true_term().clone());
            }
        }
    }

    let antecedent = formula_argument(&formula, 0);
    let consequent = formula_argument(&formula, 1);
    if antecedent == consequent
        || antecedent == *bank.false_term()
        || consequent == *bank.true_term()
    {
        return Ok(bank.true_term().clone());
    }

    let neg_antecedent = negate_decoded_formula(bank, &antecedent)?;
    let neg_consequent = negate_decoded_formula(bank, &consequent)?;
    if consequent == neg_antecedent
        || antecedent == neg_consequent
        || antecedent == *bank.true_term()
    {
        return Ok(consequent);
    }
    if consequent == *bank.false_term() {
        return negate_decoded_formula(bank, &antecedent);
    }

    Ok(formula)
}

fn simplify_decoded_equivalence_like(
    bank: &mut TermBank,
    formula: &Term,
) -> Result<Term, Diagnostic> {
    let formula = simplify_decoded_args(bank, formula, true)?;
    if formula.arity() != 2 {
        return Ok(formula);
    }

    let sig = bank.signature();
    let negative =
        matches!(formula.f_code(), code if code == sig.xor_code() || code == sig.neqn_code());
    let left = formula_argument(&formula, 0);
    let right = formula_argument(&formula, 1);

    if left == right {
        return Ok(if negative {
            bank.false_term().clone()
        } else {
            bank.true_term().clone()
        });
    }
    if left == *bank.true_term() {
        return if negative {
            negate_decoded_formula(bank, &right)
        } else {
            Ok(right)
        };
    }
    if right == *bank.true_term() {
        return if negative {
            negate_decoded_formula(bank, &left)
        } else {
            Ok(left)
        };
    }
    if left == *bank.false_term() {
        return if negative {
            Ok(right)
        } else {
            negate_decoded_formula(bank, &right)
        };
    }
    if right == *bank.false_term() {
        return if negative {
            Ok(left)
        } else {
            negate_decoded_formula(bank, &left)
        };
    }

    Ok(formula)
}

fn simplify_decoded_quantifier(bank: &mut TermBank, formula: &Term) -> Result<Term, Diagnostic> {
    let formula = simplify_decoded_args(bank, formula, true)?;
    if formula.arity() == 1 && formula_argument(&formula, 0).is_lambda() {
        let matrix = formula_argument(&formula_argument(&formula, 0), 1);
        assert!(
            matrix.type_().is_some_and(|type_| type_.is_bool()),
            "decoded quantified lambda matrix must be Boolean"
        );
        if term_is_db_closed(&matrix) {
            return Ok(matrix);
        }
    }
    Ok(formula)
}

fn negate_decoded_formula(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if !term.type_().is_some_and(|type_| type_.is_bool()) {
        return Ok(term.clone());
    }
    if term.is_db_var() {
        return tformula_fcode_alloc(bank, bank.signature().not_code(), term.clone(), None);
    }

    let sig = bank.signature();
    if term == bank.true_term() {
        return Ok(bank.false_term().clone());
    }
    if term == bank.false_term() {
        return Ok(bank.true_term().clone());
    }
    if term.f_code() == sig.not_code() {
        return Ok(formula_argument(term, 0));
    }
    if term.f_code() == sig.eqn_code() {
        return tformula_fcode_alloc(
            bank,
            bank.signature().neqn_code(),
            formula_argument(term, 0),
            Some(formula_argument(term, 1)),
        );
    }
    if term.f_code() == sig.neqn_code() {
        return tformula_fcode_alloc(
            bank,
            bank.signature().eqn_code(),
            formula_argument(term, 0),
            Some(formula_argument(term, 1)),
        );
    }
    if term.f_code() == sig.equiv_code() {
        return tformula_fcode_alloc(
            bank,
            bank.signature().xor_code(),
            formula_argument(term, 0),
            Some(formula_argument(term, 1)),
        );
    }
    if term.f_code() == sig.xor_code() {
        return tformula_fcode_alloc(
            bank,
            bank.signature().equiv_code(),
            formula_argument(term, 0),
            Some(formula_argument(term, 1)),
        );
    }

    tformula_fcode_alloc(bank, bank.signature().not_code(), term.clone(), None)
}

/// Allocates a unary or binary formula node with the given function code.
///
/// This matches C `TFormulaFCodeAlloc`: the operator arity comes from the
/// signature, non-lambda formula nodes receive Boolean type, predicate
/// formulas receive `TPPredPos`, and the result is inserted into the term bank.
///
/// # Errors
///
/// Returns a diagnostic if `op` is unknown, is not unary or binary, is missing
/// a required second argument, or cannot be inserted into the term bank.
pub fn tformula_fcode_alloc(
    bank: &mut TermBank,
    op: i64,
    arg1: Term,
    arg2: Option<Term>,
) -> Result<Term, Diagnostic> {
    let arity = bank.signature().find_arity(op).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "TFormulaFCodeAlloc requires a known signature arity",
        )
    })?;
    let arity = usize::try_from(arity).map_err(|_| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "TFormulaFCodeAlloc requires unary or binary formula arity",
        )
    })?;
    if arity != 1 && arity != 2 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "TFormulaFCodeAlloc requires unary or binary formula arity",
        ));
    }
    if arity == 2 && arg2.is_none() {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "TFormulaFCodeAlloc binary formula is missing its second argument",
        ));
    }

    let term = Term::top_alloc(op, arity);
    if op != SIG_NAMED_LAMBDA_CODE {
        term.set_type(Some(bank.signature().type_bank().bool_type()));
    }
    if bank.signature().is_predicate(op) {
        term.set_prop(TP_PRED_POS);
    }
    term.set_argument(0, arg1);
    if let Some(arg2) = arg2 {
        term.set_argument(1, arg2);
    }
    bank.term_top_insert(term)
}

/// Returns a formula equivalent to the negation of `form`.
///
/// This matches C `TFormulaNegAlloc`: it removes a single root negation when
/// present and otherwise allocates `$not(form)`.
///
/// # Errors
///
/// Returns a diagnostic if allocating the negated formula fails.
///
/// # Panics
///
/// Panics if a root negation cell is malformed.
pub fn tformula_neg_alloc(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if form.f_code() == bank.signature().not_code() {
        return Ok(formula_argument(form, 0));
    }
    tformula_fcode_alloc(bank, bank.signature().not_code(), form.clone(), None)
}

/// Expands literal encodings before FOOL/CNF lowering.
///
/// This matches C `TFormulaExpandLiterals` for a single term-encoded formula:
/// disequality becomes an explicit negated equality, Boolean equality may
/// become equivalence, and selected `$eq(F,$true)` wrappers around internal
/// Boolean formulas are removed.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a changed formula or allocating an
/// expanded formula fails.
///
/// # Panics
///
/// Panics if the C precondition for unwrapping an internal Boolean formula is
/// violated.
pub fn tformula_expand_literals(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if form.is_any_var() || form.arity() == 0 {
        return Ok(form.clone());
    }

    let (eqn_code, neqn_code, not_code, equiv_code) = {
        let sig = bank.signature();
        (
            sig.eqn_code(),
            sig.neqn_code(),
            sig.not_code(),
            sig.equiv_code(),
        )
    };

    let mut current = if form.f_code() == neqn_code {
        let equality = tformula_fcode_alloc(
            bank,
            eqn_code,
            formula_argument(form, 0),
            Some(formula_argument(form, 1)),
        )?;
        tformula_fcode_alloc(bank, not_code, equality, None)?
    } else {
        form.clone()
    };

    let copy = Term::top_copy_without_args(&current);
    let mut changed = false;
    for (index, arg) in current.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        let expanded = tformula_expand_literals(bank, &arg)?;
        changed |= expanded != arg;
        copy.set_argument(index, expanded);
    }
    if changed {
        current = bank.term_top_insert(copy)?;
    }

    if current.arity() == 2 && current.f_code() == eqn_code {
        let left = formula_argument(&current, 0);
        if left.type_() == Some(bank.signature().type_bank().bool_type()) && !left.is_free_var() {
            let right = formula_argument(&current, 1);
            if right != *bank.true_term() {
                current = tformula_fcode_alloc(bank, equiv_code, left, Some(right))?;
            } else if left.f_code() < bank.signature().internal_symbols()
                && left.f_code() != bank.signature().answer_code()
            {
                assert_eq!(
                    right,
                    bank.true_term().clone(),
                    "internal Boolean equality must be compared to true"
                );
                current = left;
            }
        }
    }

    Ok(current)
}

/// Unrolls FOOL Boolean subterms that occur as ordinary term arguments.
///
/// This matches C `TFormulaUnrollFOOL` at the term-formula level: encoded
/// literals are expanded first, then each nontrivial Boolean subterm inside a
/// literal side is split into `$true` and `$false` cases.
///
/// # Errors
///
/// Returns a diagnostic if literal expansion, lambda eta-reduction, term
/// replacement, or formula allocation fails.
///
/// # Panics
///
/// Panics if a located FOOL subterm is not Boolean, or if formula cells are
/// malformed.
pub fn tformula_unroll_fool(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    Ok(tformula_unroll_fool_result(bank, form)?.into_formula())
}

struct TFormulaIteExpansion {
    condition: Term,
    if_true: Term,
    if_false: Term,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TFormulaLetLiftResult {
    pub formula: Term,
    pub definitions: Vec<Term>,
}

struct TFormulaLetDefinition {
    old_lhs: Term,
    fresh_lhs: Term,
}

/// Applies C `do_ite_unroll` to a term-encoded formula.
///
/// Formula-position `$ite(C,T,F)` becomes `(~C|T)&(C|F)`. Literal-side
/// term-position `$ite` uses the first occurrence on the left side, then the
/// first occurrence on the right side, and replaces the enclosing literal by
/// the two conditional literal cases. The recursive traversal skips nested
/// lambda terms for term-position searches, matching C `TermFindIteSubterm`.
///
/// # Errors
///
/// Returns a diagnostic if an implication/conjunction or copied term cannot be
/// inserted into the term bank.
///
/// # Panics
///
/// Panics if an `$ite`, literal, or formula cell is malformed.
pub fn tformula_lift_ite(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    do_ite_unroll(bank, form)
}

fn do_ite_unroll(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if form.f_code() == SIG_ITE_CODE {
        assert_eq!(form.arity(), 3, "$ite formula must have three arguments");
        let condition = formula_argument(form, 0);
        let not_condition = tformula_negate(bank, &condition)?;
        let true_part = tformula_fcode_alloc(
            bank,
            bank.signature().or_code(),
            not_condition,
            Some(formula_argument(form, 1)),
        )?;
        let false_part = tformula_fcode_alloc(
            bank,
            bank.signature().or_code(),
            condition,
            Some(formula_argument(form, 2)),
        )?;
        let unrolled = tformula_fcode_alloc(
            bank,
            bank.signature().and_code(),
            true_part,
            Some(false_part),
        )?;
        return do_ite_unroll(bank, &unrolled);
    }

    if tformula_is_literal(bank, form) {
        if let Some(unrolled) = do_ite_unroll_literal(bank, form)? {
            return Ok(unrolled);
        }
        return Ok(form.clone());
    }

    if tformula_is_quantified(bank, form) && !form.is_lambda() {
        let original_body = formula_argument(form, 1);
        let unrolled_body = do_ite_unroll(bank, &original_body)?;
        if unrolled_body != original_body {
            return tformula_fcode_alloc(
                bank,
                form.f_code(),
                formula_argument(form, 0),
                Some(unrolled_body),
            );
        }
        return Ok(form.clone());
    }

    let copy = Term::top_copy_without_args(form);
    let mut changed = false;
    for (index, arg) in form.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        let unrolled = do_ite_unroll(bank, &arg)?;
        changed |= unrolled != arg;
        copy.set_argument(index, unrolled);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(form.clone())
    }
}

fn do_ite_unroll_literal(bank: &mut TermBank, form: &Term) -> Result<Option<Term>, Diagnostic> {
    if let Some(expansion) = tformula_first_ite_expansion(&formula_argument(form, 0), bank)? {
        let if_true = tformula_fcode_alloc(
            bank,
            form.f_code(),
            expansion.if_true,
            Some(formula_argument(form, 1)),
        )?;
        let if_false = tformula_fcode_alloc(
            bank,
            form.f_code(),
            expansion.if_false,
            Some(formula_argument(form, 1)),
        )?;
        return tformula_ite_expansion_to_formula(bank, expansion.condition, if_true, if_false)
            .map(Some);
    }

    if let Some(expansion) = tformula_first_ite_expansion(&formula_argument(form, 1), bank)? {
        let if_true = tformula_fcode_alloc(
            bank,
            form.f_code(),
            formula_argument(form, 0),
            Some(expansion.if_true),
        )?;
        let if_false = tformula_fcode_alloc(
            bank,
            form.f_code(),
            formula_argument(form, 0),
            Some(expansion.if_false),
        )?;
        return tformula_ite_expansion_to_formula(bank, expansion.condition, if_true, if_false)
            .map(Some);
    }

    Ok(None)
}

fn tformula_ite_expansion_to_formula(
    bank: &mut TermBank,
    condition: Term,
    if_true: Term,
    if_false: Term,
) -> Result<Term, Diagnostic> {
    let negated_condition = tformula_negate(bank, &condition)?;
    let if_true_impl = tformula_fcode_alloc(
        bank,
        bank.signature().or_code(),
        negated_condition,
        Some(if_true),
    )?;
    let if_false_impl =
        tformula_fcode_alloc(bank, bank.signature().or_code(), condition, Some(if_false))?;
    let left = do_ite_unroll(bank, &if_true_impl)?;
    let right = do_ite_unroll(bank, &if_false_impl)?;
    tformula_fcode_alloc(bank, bank.signature().and_code(), left, Some(right))
}

fn tformula_first_ite_expansion(
    term: &Term,
    bank: &mut TermBank,
) -> Result<Option<TFormulaIteExpansion>, Diagnostic> {
    if term.f_code() == SIG_ITE_CODE {
        return Ok(Some(TFormulaIteExpansion {
            condition: formula_argument(term, 0),
            if_true: formula_argument(term, 1),
            if_false: formula_argument(term, 2),
        }));
    }
    if term.is_lambda() {
        return Ok(None);
    }

    for index in 0..term.arity() {
        let argument = term
            .argument(index)
            .unwrap_or_else(|| panic!("term-position $ite argument {index} is uninitialized"));
        if let Some(expansion) = tformula_first_ite_expansion(&argument, bank)? {
            return Ok(Some(TFormulaIteExpansion {
                condition: expansion.condition,
                if_true: tformula_term_replace_argument(term, index, &expansion.if_true, bank)?,
                if_false: tformula_term_replace_argument(term, index, &expansion.if_false, bank)?,
            }));
        }
    }

    Ok(None)
}

fn tformula_term_replace_argument(
    term: &Term,
    target_index: usize,
    replacement: &Term,
    bank: &mut TermBank,
) -> Result<Term, Diagnostic> {
    let replaced = Term::top_copy_without_args(term);
    for index in 0..term.arity() {
        let argument = if index == target_index {
            replacement.clone()
        } else {
            term.argument(index)
                .unwrap_or_else(|| panic!("term-position $ite argument {index} is uninitialized"))
        };
        replaced.set_argument(index, argument);
    }
    bank.term_top_insert(replaced)
}

/// Applies C `lift_lets` to a variable-renamed term-encoded formula.
///
/// The returned definitions are the globally closed formulas introduced for
/// local `$let` definitions, in the same push order C stores in `lifted_lets`.
/// The caller owns the surrounding `TFormulaSetLiftLets` responsibilities:
/// fresh-variable-bank seeding, `TFormulaVarRename`, root `unencode_eqns`, and
/// appending the generated wrappers to the formula set.
///
/// # Errors
///
/// Returns a diagnostic if fresh symbol allocation, predicate encoding,
/// quantifier closure, replacement instantiation, app flattening, or term-bank
/// insertion fails.
///
/// # Panics
///
/// Panics if a `$let` cell, let definition equality, or definition head is
/// malformed.
pub fn tformula_lift_lets(
    bank: &mut TermBank,
    form: &Term,
) -> Result<TFormulaLetLiftResult, Diagnostic> {
    let mut definitions = Vec::new();
    let formula = lift_lets(bank, form, &mut definitions)?;
    Ok(TFormulaLetLiftResult {
        formula,
        definitions,
    })
}

fn lift_lets(
    bank: &mut TermBank,
    term: &Term,
    fresh_defs: &mut Vec<Term>,
) -> Result<Term, Diagnostic> {
    if term.is_any_var() {
        return Ok(term.clone());
    }

    if term.f_code() == SIG_LET_CODE {
        assert!(term.arity() >= 1, "$let must have at least a body");
        let body_index = term.arity() - 1;
        let mut closed_defs = BTreeMap::new();
        let mut lifted_defs = Vec::with_capacity(body_index);
        for index in 0..body_index {
            let lifted_def = lift_lets(bank, &formula_argument(term, index), fresh_defs)?;
            close_let_def(bank, &mut closed_defs, &lifted_def)?;
            lifted_defs.push(lifted_def);
        }
        make_fresh_let_defs(bank, &lifted_defs, &closed_defs, fresh_defs)?;
        let body = replace_let_body(bank, &closed_defs, &formula_argument(term, body_index))?;
        return lift_lets(bank, &body, fresh_defs);
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        let lifted = lift_lets(bank, &arg, fresh_defs)?;
        changed |= lifted != arg;
        copy.set_argument(index, lifted);
    }

    if !changed {
        return Ok(term.clone());
    }

    if copy.is_phony_app()
        && copy
            .argument(0)
            .is_some_and(|head| !head.is_phony_app_target())
    {
        let head = copy
            .argument(0)
            .expect("changed phony application head is uninitialized");
        let args = (1..copy.arity())
            .map(|index| {
                copy.argument(index).unwrap_or_else(|| {
                    panic!("phony application argument {index} is uninitialized")
                })
            })
            .collect::<Vec<_>>();
        let result_type = copy
            .type_()
            .expect("changed phony application must have a type");
        return flatten_apps(bank, &head, &args, &result_type);
    }

    bank.term_top_insert(copy)
}

fn close_let_def(
    bank: &mut TermBank,
    closed_defs: &mut BTreeMap<i64, TFormulaLetDefinition>,
    definition: &Term,
) -> Result<(), Diagnostic> {
    assert_eq!(
        definition.f_code(),
        bank.signature().eqn_code(),
        "$let definition must be an equality"
    );
    let old_lhs = formula_argument(definition, 0);
    let rhs = formula_argument(definition, 1);
    let formal_args = let_definition_formal_args(&old_lhs);
    let formal_ids = formal_args
        .iter()
        .map(term_identity_id)
        .collect::<BTreeSet<_>>();
    let mut all_vars = tformula_collect_free_vars(bank, &rhs)
        .into_iter()
        .filter(|var| !formal_ids.contains(&term_identity_id(var)))
        .collect::<Vec<_>>();
    all_vars.extend(formal_args);

    let lhs_type = old_lhs
        .type_()
        .expect("$let definition left side must have a type");
    let fresh_lhs = bank.alloc_new_skolem(&all_vars, Some(&lhs_type))?;
    closed_defs.insert(
        old_lhs.f_code(),
        TFormulaLetDefinition { old_lhs, fresh_lhs },
    );
    Ok(())
}

fn let_definition_formal_args(old_lhs: &Term) -> Vec<Term> {
    let mut args = Vec::with_capacity(old_lhs.arity());
    for index in 0..old_lhs.arity() {
        let arg = old_lhs
            .argument(index)
            .unwrap_or_else(|| panic!("$let definition head argument {index} is uninitialized"));
        assert!(
            arg.is_free_var(),
            "$let definition head arguments must be free variables"
        );
        args.push(arg);
    }
    args
}

fn make_fresh_let_defs(
    bank: &mut TermBank,
    definitions: &[Term],
    closed_defs: &BTreeMap<i64, TFormulaLetDefinition>,
    fresh_defs: &mut Vec<Term>,
) -> Result<(), Diagnostic> {
    for definition in definitions {
        assert_eq!(
            definition.f_code(),
            bank.signature().eqn_code(),
            "$let definition must be an equality"
        );
        let old_lhs = formula_argument(definition, 0);
        let rhs = formula_argument(definition, 1);
        let fresh_lhs = closed_defs
            .get(&old_lhs.f_code())
            .unwrap_or_else(|| panic!("missing closed $let definition for {}", old_lhs.f_code()))
            .fresh_lhs
            .clone();

        let matrix = if rhs.type_().as_ref().is_some_and(Type::is_bool) {
            let left = tformula_encode_predicate_as_eqn(bank, fresh_lhs)?;
            let right = tformula_encode_predicate_as_eqn(bank, rhs)?;
            tformula_fcode_alloc(bank, bank.signature().equiv_code(), left, Some(right))?
        } else {
            tformula_fcode_alloc(bank, bank.signature().eqn_code(), fresh_lhs, Some(rhs))?
        };
        fresh_defs.push(tformula_closure(bank, &matrix, true)?);
    }
    Ok(())
}

fn replace_let_body(
    bank: &mut TermBank,
    closed_defs: &BTreeMap<i64, TFormulaLetDefinition>,
    term: &Term,
) -> Result<Term, Diagnostic> {
    let mut result = if term.is_any_var() {
        term.clone()
    } else {
        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("$let body argument {index} is uninitialized"));
            let replaced = replace_let_body(bank, closed_defs, &arg)?;
            changed |= replaced != arg;
            copy.set_argument(index, replaced);
        }
        if changed {
            bank.term_top_insert(copy)?
        } else {
            term.clone()
        }
    };

    if let Some(definition) = closed_defs.get(&result.f_code()) {
        result = instantiate_let_definition(bank, definition, &result)?;
    }
    Ok(result)
}

fn instantiate_let_definition(
    bank: &mut TermBank,
    definition: &TFormulaLetDefinition,
    occurrence: &Term,
) -> Result<Term, Diagnostic> {
    assert_eq!(
        occurrence.arity(),
        definition.old_lhs.arity(),
        "$let body application arity must match its definition"
    );
    let mut substitution = Substitution::new();
    let instantiated = {
        for index in 0..occurrence.arity() {
            let variable = definition
                .old_lhs
                .argument(index)
                .unwrap_or_else(|| panic!("$let definition argument {index} is uninitialized"));
            let argument = occurrence
                .argument(index)
                .unwrap_or_else(|| panic!("$let occurrence argument {index} is uninitialized"));
            substitution.add_binding(&variable, &argument);
        }
        bank.insert_no_props_cached(&definition.fresh_lhs, DerefType::Always)
    };
    substitution.backtrack();
    instantiated
}

/// Undo C's root `unencode_eqns` rewrite for formula-shaped equality-to-true.
#[must_use]
pub fn tformula_unencode_root_eqn(bank: &TermBank, term: &Term) -> Term {
    let eqn_code = bank.signature().eqn_code();
    if term.f_code() != eqn_code || term.arity() != 2 {
        return term.clone();
    }
    let left = formula_argument(term, 0);
    if term.argument(1).as_ref() != Some(bank.true_term())
        || left.is_any_var()
        || !tformula_unencode_eqn_left_is_formula(bank, &left)
    {
        return term.clone();
    }
    left
}

fn tformula_unencode_eqn_left_is_formula(bank: &TermBank, left: &Term) -> bool {
    let sig = bank.signature();
    sig.query_prop(left.f_code(), FP_FOF_OP)
        || left.f_code() == sig.qex_code()
        || left.f_code() == sig.qall_code()
        || left.f_code() == sig.eqn_code()
        || left.f_code() == sig.neqn_code()
}

/// Applies C `TFormulaUnrollFOOL` and preserves its change flag.
///
/// The C helper first expands literals unconditionally, then calls the
/// `do_fool_unroll` mapper through `map_formula`. Its boolean return value is
/// true only when the mapper changes the expanded formula; expansion-only
/// changes do not count as `DCFoolUnroll`.
///
/// # Errors
///
/// Returns a diagnostic if literal expansion, lambda eta-reduction, term
/// replacement, or formula allocation fails.
///
/// # Panics
///
/// Panics if a located FOOL subterm is not Boolean, or if formula cells are
/// malformed.
pub fn tformula_unroll_fool_result(
    bank: &mut TermBank,
    form: &Term,
) -> Result<TFormulaFoolUnrollResult, Diagnostic> {
    let expanded = tformula_expand_literals(bank, form)?;
    let unrolled = do_fool_unroll(bank, &expanded)?;
    Ok(TFormulaFoolUnrollResult {
        fool_unrolled: unrolled != expanded,
        formula: unrolled,
    })
}

fn do_fool_unroll(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if tformula_is_literal(bank, form) {
        let reduced = lambda_eta_reduce_db(bank, form)?;
        let position = find_fool_subterm_in_literal(bank, &reduced);
        if let Some(position) = position {
            let raw_subformula = position.get_subterm(&reduced);
            assert!(
                raw_subformula.type_().as_ref().is_some_and(Type::is_bool),
                "FOOL subterm must be Boolean"
            );
            let subformula = tformula_encode_predicate_as_eqn(bank, raw_subformula)?;
            let true_case = tb_term_pos_replace(
                bank,
                &bank.true_term().clone(),
                &position,
                DerefType::Never,
                0,
                None,
            )?;
            let false_case = tb_term_pos_replace(
                bank,
                &bank.false_term().clone(),
                &position,
                DerefType::Never,
                0,
                None,
            )?;
            let negated_subformula = tformula_negate(bank, &subformula)?;
            let or_code = bank.signature().or_code();
            let and_code = bank.signature().and_code();
            let first_implication =
                tformula_fcode_alloc(bank, or_code, negated_subformula, Some(true_case))?;
            let second_implication =
                tformula_fcode_alloc(bank, or_code, subformula, Some(false_case))?;
            let left = do_fool_unroll(bank, &first_implication)?;
            let right = do_fool_unroll(bank, &second_implication)?;
            return tformula_fcode_alloc(bank, and_code, left, Some(right));
        }
        return Ok(reduced);
    }

    if tformula_is_quantified(bank, form) && !form.is_lambda() {
        let original_body = formula_argument(form, 1);
        let unrolled_body = do_fool_unroll(bank, &original_body)?;
        if unrolled_body != original_body {
            return tformula_fcode_alloc(
                bank,
                form.f_code(),
                formula_argument(form, 0),
                Some(unrolled_body),
            );
        }
        return Ok(form.clone());
    }

    if form.is_lambda() {
        return Ok(form.clone());
    }

    let mut left = None;
    let mut right = None;
    let mut changed = false;
    if tformula_has_subform1(bank, form) {
        let original = formula_argument(form, 0);
        let unrolled = do_fool_unroll(bank, &original)?;
        changed = unrolled != original;
        left = Some(unrolled);
    }
    if tformula_has_subform2(bank, form) {
        let original = formula_argument(form, 1);
        let unrolled = do_fool_unroll(bank, &original)?;
        changed |= unrolled != original;
        right = Some(unrolled);
    }

    if changed {
        return tformula_fcode_alloc(
            bank,
            form.f_code(),
            left.expect("changed formula must have a first subformula"),
            right,
        );
    }

    Ok(form.clone())
}

fn find_fool_subterm_in_literal(bank: &TermBank, form: &Term) -> Option<TermPos> {
    let left = formula_argument(form, 0);
    let mut left_position = TermPos::new();
    left_position.push_component(form.clone(), 0);
    if let Some(position) = find_fool_subterm(bank, &left, &left_position) {
        return Some(position);
    }

    let right = formula_argument(form, 1);
    let mut right_position = TermPos::new();
    right_position.push_component(form.clone(), 1);
    find_fool_subterm(bank, &right, &right_position)
}

fn find_fool_subterm(bank: &TermBank, term: &Term, prefix: &TermPos) -> Option<TermPos> {
    if term.is_lambda() || !term.has_bool_subterm() {
        return None;
    }

    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("FOOL subterm search expects initialized arguments"));
        let mut nested = prefix.clone();
        nested.push_component(term.clone(), index);
        if arg.type_().as_ref().is_some_and(Type::is_bool) {
            if !fool_should_ignore(&arg, bank)
                && arg.f_code() != SIG_TRUE_CODE
                && arg.f_code() != SIG_FALSE_CODE
            {
                return Some(nested);
            }
        } else if let Some(found) = find_fool_subterm(bank, &arg, &nested) {
            return Some(found);
        }
    }

    None
}

fn fool_should_ignore(term: &Term, bank: &TermBank) -> bool {
    if !term.type_().as_ref().is_some_and(Type::is_bool) {
        return false;
    }

    let sig = bank.signature();
    if (term.f_code() == sig.eqn_code() || term.f_code() == sig.neqn_code()) && term.arity() == 2 {
        let left = formula_argument(term, 0);
        let right = formula_argument(term, 1);
        if right == *bank.true_term() {
            return left.is_free_var() || left == *bank.true_term();
        }
    }

    term.is_free_var()
}

/// Returns the logical negation of a term-encoded formula.
///
/// This matches C `TFormulaNegate`: literal roots have their equality code
/// toggled, while non-literals are wrapped in a formula negation. Unlike
/// [`tformula_neg_alloc`], an existing root negation is not flattened.
///
/// # Errors
///
/// Returns a diagnostic if allocating the toggled literal or negation fails.
///
/// # Panics
///
/// Panics if a literal formula lacks either C-required argument.
pub fn tformula_negate(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if tformula_is_literal(bank, form) {
        let f_code = bank.signature().get_other_eqn_code(form.f_code());
        return tformula_fcode_alloc(
            bank,
            f_code,
            formula_argument(form, 0),
            Some(formula_argument(form, 1)),
        );
    }

    tformula_fcode_alloc(bank, bank.signature().not_code(), form.clone(), None)
}

/// Maximally simplifies a term-encoded formula.
///
/// This matches C `TFormulaSimplify`: children are simplified first, then C's
/// root rewrite loop is applied until no more root rewrite fires. The
/// `quopt_limit` gate controls the expensive free-variable check before
/// removing redundant quantifiers.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a formula or allocating a propositional
/// constant fails.
///
/// # Panics
///
/// Panics if formula cells are malformed or if `quopt_limit` is negative.
pub fn tformula_simplify(
    bank: &mut TermBank,
    form: &Term,
    quopt_limit: i64,
) -> Result<Term, Diagnostic> {
    assert!(
        quopt_limit >= 0,
        "TFormulaSimplify expects a nonnegative limit"
    );

    if tformula_is_literal(bank, form) || form.type_().as_ref().is_some_and(Type::is_arrow) {
        return Ok(form.clone());
    }

    let mut arg1 = None;
    let mut arg2 = None;
    let mut modified = false;
    if tformula_has_subform1(bank, form) {
        let original = formula_argument(form, 0);
        let simplified = tformula_simplify(bank, &original, quopt_limit)?;
        modified = simplified != original;
        arg1 = Some(simplified);
    } else if tformula_is_quantified(bank, form) {
        arg1 = Some(formula_argument(form, 0));
    }

    if tformula_has_subform2(bank, form) || tformula_is_quantified(bank, form) {
        let original = formula_argument(form, 1);
        let simplified = tformula_simplify(bank, &original, quopt_limit)?;
        modified |= simplified != original;
        arg2 = Some(simplified);
    }

    let mut current = if modified {
        tformula_fcode_alloc(
            bank,
            form.f_code(),
            arg1.expect("changed formula must have a first argument"),
            arg2,
        )?
    } else {
        form.clone()
    };

    loop {
        let simplified = tformula_simplify_root_once(bank, &current, quopt_limit)?;
        if simplified == current {
            return Ok(current);
        }
        current = simplified;
    }
}

fn tformula_simplify_root_once(
    bank: &mut TermBank,
    form: &Term,
    quopt_limit: i64,
) -> Result<Term, Diagnostic> {
    let signature = bank.signature();
    let negation_code = signature.not_code();
    let disjunction_code = signature.or_code();
    let conjunction_code = signature.and_code();
    let equivalence_code = signature.equiv_code();
    let implication_code = signature.impl_code();
    let exclusive_or_code = signature.xor_code();
    let reverse_implication_code = signature.bimpl_code();
    let negated_disjunction_code = signature.nor_code();
    let negated_conjunction_code = signature.nand_code();
    let existential_code = signature.qex_code();
    let universal_code = signature.qall_code();

    if form.f_code() == negation_code {
        return simplify_not_root(bank, form);
    }
    if form.f_code() == disjunction_code {
        return Ok(simplify_or_root(bank, form).unwrap_or_else(|| form.clone()));
    }
    if form.f_code() == conjunction_code {
        return Ok(simplify_and_root(bank, form).unwrap_or_else(|| form.clone()));
    }
    if form.f_code() == equivalence_code {
        return simplify_equivalence_root(bank, form);
    }
    if form.f_code() == implication_code {
        return simplify_implication_root(bank, form);
    }
    if form.f_code() == exclusive_or_code {
        return simplify_negated_binary_root(bank, form, equivalence_code, quopt_limit);
    }
    if form.f_code() == reverse_implication_code {
        let implication = tformula_fcode_alloc(
            bank,
            implication_code,
            formula_argument(form, 1),
            Some(formula_argument(form, 0)),
        )?;
        return tformula_simplify(bank, &implication, quopt_limit);
    }
    if form.f_code() == negated_disjunction_code {
        return simplify_negated_binary_root(bank, form, disjunction_code, quopt_limit);
    }
    if form.f_code() == negated_conjunction_code {
        return simplify_negated_binary_root(bank, form, conjunction_code, quopt_limit);
    }
    if form.f_code() == existential_code || form.f_code() == universal_code {
        return Ok(
            simplify_quantifier_root(bank, form, quopt_limit).unwrap_or_else(|| form.clone())
        );
    }

    Ok(form.clone())
}

fn simplify_not_root(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let child = formula_argument(form, 0);
    if tformula_is_literal(bank, &child) {
        return tformula_fcode_alloc(
            bank,
            bank.signature().get_other_eqn_code(child.f_code()),
            formula_argument(&child, 0),
            Some(formula_argument(&child, 1)),
        );
    }
    Ok(form.clone())
}

fn simplify_or_root(bank: &TermBank, form: &Term) -> Option<Term> {
    let left = formula_argument(form, 0);
    let right = formula_argument(form, 1);
    tprop_arg_return_other(bank, &left, &right, false)
        .or_else(|| tprop_arg_return(bank, &left, &right, true))
        .or_else(|| (left == right).then_some(left))
}

fn simplify_and_root(bank: &TermBank, form: &Term) -> Option<Term> {
    let left = formula_argument(form, 0);
    let right = formula_argument(form, 1);
    tprop_arg_return_other(bank, &left, &right, true)
        .or_else(|| tprop_arg_return(bank, &left, &right, false))
        .or_else(|| (left == right).then_some(left))
}

fn simplify_equivalence_root(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let left = formula_argument(form, 0);
    let right = formula_argument(form, 1);
    if let Some(handle) = tprop_arg_return_other(bank, &left, &right, true) {
        return Ok(handle);
    }
    if let Some(handle) = tprop_arg_return_other(bank, &left, &right, false) {
        return tformula_neg_alloc(bank, &handle);
    }
    if left == right {
        return tformula_prop_constant_alloc(bank, true);
    }
    Ok(form.clone())
}

fn simplify_implication_root(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let left = formula_argument(form, 0);
    let right = formula_argument(form, 1);
    if tformula_is_prop_true(bank, &left) {
        return Ok(right);
    }
    if tformula_is_prop_false(bank, &left) {
        return tformula_prop_constant_alloc(bank, true);
    }
    if tformula_is_prop_false(bank, &right) {
        return tformula_neg_alloc(bank, &left);
    }
    if tformula_is_prop_true(bank, &right) || left == right {
        return tformula_prop_constant_alloc(bank, true);
    }
    Ok(form.clone())
}

fn simplify_negated_binary_root(
    bank: &mut TermBank,
    form: &Term,
    inner_code: i64,
    quopt_limit: i64,
) -> Result<Term, Diagnostic> {
    let inner = tformula_fcode_alloc(
        bank,
        inner_code,
        formula_argument(form, 0),
        Some(formula_argument(form, 1)),
    )?;
    let negated = tformula_fcode_alloc(bank, bank.signature().not_code(), inner, None)?;
    tformula_simplify(bank, &negated, quopt_limit)
}

fn simplify_quantifier_root(bank: &TermBank, form: &Term, quopt_limit: i64) -> Option<Term> {
    let body = formula_argument(form, 1);
    let variable = formula_argument(form, 0);
    (form.v_count() == 0
        || (form.weight() <= quopt_limit && !tformula_var_is_free(bank, &body, &variable)))
    .then_some(body)
}

fn tprop_arg_return_other(
    bank: &TermBank,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Option<Term> {
    if tformula_is_prop_const(bank, left, positive) {
        Some(right.clone())
    } else if tformula_is_prop_const(bank, right, positive) {
        Some(left.clone())
    } else {
        None
    }
}

fn tprop_arg_return(bank: &TermBank, left: &Term, right: &Term, positive: bool) -> Option<Term> {
    if tformula_is_prop_const(bank, left, positive) {
        Some(left.clone())
    } else if tformula_is_prop_const(bank, right, positive) {
        Some(right.clone())
    } else {
        None
    }
}

#[must_use]
pub fn tformula_is_prop_true(bank: &TermBank, form: &Term) -> bool {
    tformula_is_prop_const(bank, form, true)
}

#[must_use]
pub fn tformula_is_prop_false(bank: &TermBank, form: &Term) -> bool {
    tformula_is_prop_const(bank, form, false)
}

/// Returns whether `form` is C's encoded propositional constant.
///
/// This matches C `TFormulaIsPropConst`: `$true` is represented as
/// `$eq($true,$true)` and `$false` as `$neq($true,$true)`.
#[must_use]
pub fn tformula_is_prop_const(bank: &TermBank, form: &Term, positive: bool) -> bool {
    let expected_code = if positive {
        bank.signature().eqn_code()
    } else {
        bank.signature().neqn_code()
    };
    form.f_code() == expected_code
        && form.arity() == 2
        && form.argument(0).as_ref() == Some(bank.true_term())
        && form.argument(1).as_ref() == Some(bank.true_term())
}

/// Allocates C's encoded propositional true or false formula.
///
/// This matches C `TFormulaPropConstantAlloc`.
///
/// # Errors
///
/// Returns a diagnostic if the encoded literal cannot be allocated.
pub fn tformula_prop_constant_alloc(
    bank: &mut TermBank,
    positive: bool,
) -> Result<Term, Diagnostic> {
    let f_code = bank.signature_mut().get_eqn_code(positive);
    let true_term = bank.true_term().clone();
    tformula_fcode_alloc(bank, f_code, true_term.clone(), Some(true_term))
}

/// Allocates a term-encoded formula for a literal.
///
/// This matches C `TFormulaLitAlloc`: first-order mode keeps the ordinary
/// `$eq`/`$neq` literal encoding, while higher-order mode decodes formula
/// terms and turns Boolean equalities into equivalence or XOR formulas.
///
/// # Errors
///
/// Returns a diagnostic if formula decoding or term-bank allocation fails.
///
/// # Panics
///
/// Panics if encoded equality allocation sees terms that are not shared in the
/// term bank, matching the C term-bank precondition.
pub fn tformula_lit_alloc(
    bank: &mut TermBank,
    literal: &Eqn,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    if problem_type == ProblemType::FirstOrder {
        return literal.tb_term_encode(bank, PatEqnDirection::Normal);
    }

    let right_is_true = literal.right() == bank.true_term();
    if right_is_true {
        let mut formula = decode_formulas_for_cnf(bank, literal.left())?;
        if literal.is_negative() {
            let not_code = bank.signature().not_code();
            formula = tformula_fcode_alloc(bank, not_code, formula, None)?;
        }
        return Ok(formula);
    }

    if literal.is_clausifiable(bank) {
        let left = decode_formulas_for_cnf(bank, literal.left())?;
        let right = decode_formulas_for_cnf(bank, literal.right())?;
        let op = if literal.is_positive() {
            bank.signature().equiv_code()
        } else {
            bank.signature().xor_code()
        };
        return tformula_fcode_alloc(bank, op, left, Some(right));
    }

    let left = decode_formulas_for_cnf(bank, literal.left())?;
    let right = decode_formulas_for_cnf(bank, literal.right())?;
    Eqn::terms_tb_term_encode(
        bank,
        &left,
        &right,
        literal.is_positive(),
        PatEqnDirection::Normal,
    )
}

/// Parses a TSTP term-encoded formula.
///
/// This forwards to the term-bank implementation of C `TFormulaTSTPParse`.
pub fn tformula_tstp_parse(scanner: &mut Scanner, bank: &mut TermBank) -> Result<Term, Diagnostic> {
    bank.parse_tformula_tstp(scanner)
}

/// Parses an old-TPTP term-encoded formula.
///
/// This forwards to the term-bank implementation of C `TFormulaTPTPParse`.
pub fn tformula_tptp_parse(scanner: &mut Scanner, bank: &mut TermBank) -> Result<Term, Diagnostic> {
    bank.parse_tformula_tptp(scanner)
}

/// Parses a TCF formula in TSTP syntax.
///
/// This matches C `TcfTSTPParse`: unquantified clause bodies fold
/// `EqnFOFParse` literals over `|`, while leading universal quantifiers use the
/// TCF-specific quantified parser that gives parenthesized bodies the same
/// clause-only treatment and unparenthesized bodies a single atom.
pub fn tcf_tstp_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let start = func_symb_start_token()
        | TokenType::ITE_TOKEN
        | TokenType::LET_TOKEN
        | TokenType::TILDE_SIGN
        | TokenType::UNIV_QUANTOR
        | TokenType::OPEN_BRACKET;
    scanner.check_tok(start)?;

    let in_parens = scanner.test_tok(TokenType::OPEN_BRACKET);
    if in_parens {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    }

    let formula = if scanner.test_tok(TokenType::UNIV_QUANTOR) {
        scanner.accept_tok(TokenType::UNIV_QUANTOR)?;
        scanner.accept_tok(TokenType::OPEN_SQUARE)?;
        let quantor = bank.signature().qall_code();
        tcf_quantified_tform_tstp_parse(scanner, bank, quantor, problem_type)?
    } else {
        tcf_clause_tform_tstp_parse(scanner, bank, problem_type)?
    };

    if in_parens {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    }
    Ok(formula)
}

/// Parses a top-level TSTP `$distinct(...)` pseudo-formula spelling.
///
/// This preserves the C-shaped formula-owner surface used around
/// `TSTPDistinctParse`: a bare or parenthesized `$distinct(...)` stays as the
/// pseudo-formula term for later `$distinct` processing, while a top-level
/// negated form is expanded to pairwise disequalities before wrapping it in
/// formula negation.
///
/// # Errors
///
/// Returns a diagnostic if a recognized `$distinct` spelling is malformed.
pub fn parse_tstp_top_level_distinct_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<Term>, Diagnostic> {
    if scanner.test_id("$distinct") {
        return bank.parse_tstp_distinct(scanner).map(Some);
    }
    if let Some(distinct) = parse_tstp_negated_distinct_formula(scanner, bank)? {
        return Ok(Some(distinct));
    }
    if let Some(distinct) = parse_tstp_parenthesized_negated_distinct_formula(scanner, bank)? {
        return Ok(Some(distinct));
    }
    parse_tstp_parenthesized_distinct_formula(scanner, bank)
}

/// Parses `~$distinct(...)`, `~ @ $distinct(...)`, or negated parenthesized
/// top-level `$distinct` forms.
///
/// # Errors
///
/// Returns a diagnostic if a recognized negated `$distinct` spelling is
/// malformed.
pub fn parse_tstp_negated_distinct_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<Term>, Diagnostic> {
    if !scanner.test_tok(TokenType::TILDE_SIGN) {
        return Ok(None);
    }

    let mut lookahead = scanner.clone();
    lookahead.accept_tok(TokenType::TILDE_SIGN)?;
    if lookahead.test_tok(TokenType::APPLICATION) {
        lookahead.accept_tok(TokenType::APPLICATION)?;
    }
    let mut probe = bank.clone();
    let is_distinct = if lookahead.test_id("$distinct") {
        true
    } else {
        parse_tstp_parenthesized_distinct_formula(&mut lookahead, &mut probe)?.is_some()
    };
    if !is_distinct {
        return Ok(None);
    }

    scanner.accept_tok(TokenType::TILDE_SIGN)?;
    if scanner.test_tok(TokenType::APPLICATION) {
        scanner.accept_tok(TokenType::APPLICATION)?;
    }
    let distinct = if scanner.test_id("$distinct") {
        bank.parse_tstp_distinct(scanner)?
    } else {
        parse_tstp_parenthesized_distinct_formula(scanner, bank)?.ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "expected parenthesized $distinct after negation",
            )
        })?
    };
    let expanded = tformula_expand_distinct(bank, &distinct)?;
    tformula_fcode_alloc(bank, bank.signature().not_code(), expanded, None).map(Some)
}

/// Parses one or more parenthesized wrappers around a negated top-level
/// `$distinct` formula.
///
/// # Errors
///
/// Returns a diagnostic if a recognized parenthesized negated `$distinct`
/// spelling is malformed.
pub fn parse_tstp_parenthesized_negated_distinct_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<Term>, Diagnostic> {
    let mut lookahead = scanner.clone();
    let mut wrappers = 0;
    while lookahead.test_tok(TokenType::OPEN_BRACKET) {
        lookahead.accept_tok(TokenType::OPEN_BRACKET)?;
        wrappers += 1;
    }
    if wrappers == 0 {
        return Ok(None);
    }
    let mut probe = bank.clone();
    if parse_tstp_negated_distinct_formula(&mut lookahead, &mut probe)?.is_none() {
        return Ok(None);
    }

    for _ in 0..wrappers {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    }
    let distinct = parse_tstp_negated_distinct_formula(scanner, bank)?.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "expected parenthesized negated $distinct formula",
        )
    })?;
    for _ in 0..wrappers {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    }
    Ok(Some(distinct))
}

/// Parses one or more parenthesized wrappers around a top-level `$distinct`
/// formula.
///
/// # Errors
///
/// Returns a diagnostic if a recognized parenthesized `$distinct` spelling is
/// malformed.
pub fn parse_tstp_parenthesized_distinct_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<Term>, Diagnostic> {
    let mut lookahead = scanner.clone();
    let mut wrappers = 0;
    while lookahead.test_tok(TokenType::OPEN_BRACKET) {
        lookahead.accept_tok(TokenType::OPEN_BRACKET)?;
        wrappers += 1;
    }
    if wrappers == 0 || !lookahead.test_id("$distinct") {
        return Ok(None);
    }

    for _ in 0..wrappers {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    }
    let distinct = bank.parse_tstp_distinct(scanner)?;
    for _ in 0..wrappers {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    }
    Ok(Some(distinct))
}

fn tcf_quantified_tform_tstp_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    quantor: FunCode,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let variable_position = token_pos_rep(scanner.current_token());
    bank.vars().push_env();
    let parsed = (|| {
        let variable = bank.parse_term_with_distinct_checks(scanner)?;
        if !variable.is_free_var() {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!("{variable_position} Variable expected, non-variable term found"),
            ));
        }

        let rest = if scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            tcf_quantified_tform_tstp_parse(scanner, bank, quantor, problem_type)?
        } else {
            scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
            scanner.accept_tok(TokenType::COLON)?;
            if scanner.test_tok(TokenType::OPEN_BRACKET) {
                scanner.accept_tok(TokenType::OPEN_BRACKET)?;
                let rest = tcf_clause_tform_tstp_parse(scanner, bank, problem_type)?;
                scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
                rest
            } else {
                tcf_atom_tform_tstp_parse(scanner, bank, problem_type)?
            }
        };

        tformula_fcode_alloc(bank, quantor, variable, Some(rest))
    })();
    bank.vars().pop_env();
    parsed
}

fn tcf_atom_tform_tstp_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let atom = eqn_fof_parse(scanner, bank, problem_type)?;
    tformula_lit_alloc(bank, &atom, problem_type)
}

fn tcf_clause_tform_tstp_parse(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let first = eqn_fof_parse(scanner, bank, problem_type)?;
    let mut formula = tformula_lit_alloc(bank, &first, problem_type)?;
    let or_code = bank.signature().or_code();
    while scanner.test_tok(TokenType::FOF_OR) {
        scanner.accept_tok(TokenType::FOF_OR)?;
        let next_literal = eqn_fof_parse(scanner, bank, problem_type)?;
        let next = tformula_lit_alloc(bank, &next_literal, problem_type)?;
        formula = tformula_fcode_alloc(bank, or_code, formula, Some(next))?;
    }
    Ok(formula)
}

/// Encodes a clause as a disjunction-shaped term formula.
///
/// This matches C `TFormulaClauseEncode`: empty clauses become `$false`, and
/// non-empty clauses fold encoded literals from left to right with formula OR.
/// Universal closure is intentionally left to `TFormulaClauseClosedEncode`.
///
/// # Errors
///
/// Returns a diagnostic if literal encoding or formula allocation fails.
///
/// # Panics
///
/// Panics if a literal violates the term-bank sharing preconditions inherited
/// from [`tformula_lit_alloc`].
pub fn tformula_clause_encode(
    bank: &mut TermBank,
    clause: &Clause,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let mut literals = clause.literals().as_slice().iter();
    let Some(first) = literals.next() else {
        return tformula_prop_constant_alloc(bank, false);
    };

    let mut result = tformula_lit_alloc(bank, first, problem_type)?;
    let or_code = bank.signature().or_code();
    for literal in literals {
        let next = tformula_lit_alloc(bank, literal, problem_type)?;
        result = tformula_fcode_alloc(bank, or_code, result, Some(next))?;
    }
    Ok(result)
}

/// Encodes a clause as a universally closed term formula.
///
/// This matches C `TFormulaClauseClosedEncode`: first build the unclosed
/// disjunction with [`tformula_clause_encode`], then add the universal closure.
///
/// # Errors
///
/// Returns a diagnostic if literal encoding, formula allocation, or quantifier
/// allocation fails.
///
/// # Panics
///
/// Panics if a literal violates the term-bank sharing preconditions inherited
/// from [`tformula_lit_alloc`].
pub fn tformula_clause_closed_encode(
    bank: &mut TermBank,
    clause: &Clause,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    let formula = tformula_clause_encode(bank, clause, problem_type)?;
    tformula_closure(bank, &formula, true)
}

/// Returns whether a term-encoded formula has no free variables.
///
/// This matches C `TFormulaIsClosed` without mutating the bank-wide
/// `TPIsFreeVar` flags used by the C implementation.
#[must_use]
pub fn tformula_is_closed(bank: &TermBank, form: &Term) -> bool {
    tformula_collect_free_vars(bank, form).is_empty()
}

/// Returns one free variable from a term-encoded formula, if any exists.
///
/// This matches C `TFormulaHasFreeVars` at the API level. The selected
/// variable follows Rust's term-identity collection order rather than C's
/// current pointer-tree root.
#[must_use]
pub fn tformula_has_free_vars(bank: &TermBank, form: &Term) -> Option<Term> {
    tformula_collect_free_vars(bank, form).into_iter().next()
}

/// Returns whether two term-encoded formula handles are identical.
///
/// This matches C `TFormulaEqual`, which is a pointer-identity macro rather
/// than structural equality.
#[must_use]
pub fn tformula_equal(left: &Term, right: &Term) -> bool {
    left == right
}

/// Copies a term-encoded formula through the term bank without copied
/// top-cell properties.
///
/// This matches C `TFormulaCopy`, which expands to
/// `TBInsertNoPropsCached(bank, form, DEREF_ALWAYS)`.
///
/// # Errors
///
/// Returns a diagnostic if the copied formula cannot be inserted.
///
/// # Panics
///
/// Panics if a free or DB variable lacks a type, matching the C term-bank
/// preconditions.
pub fn tformula_copy(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    bank.insert_no_props_cached(form, DerefType::Always)
}

/// Marks every term cell reachable from a term-encoded formula for term-bank
/// garbage collection.
///
/// This matches C `TFormulaGCMarkCells`.
pub fn tformula_gc_mark_cells(bank: &TermBank, form: &Term) {
    bank.gc_mark_term(form);
}

/// Returns the maximum variable code used by a term-encoded formula.
///
/// This matches C `TFormulaFindMaxVarCode`, including E's negative free
/// variable-code convention where the most negative code is the maximum used
/// variable index.
#[must_use]
pub fn tformula_find_max_var_code(form: &Term) -> i64 {
    term_find_max_var_code(form)
}

/// Returns whether `var` occurs free in a term-encoded formula.
///
/// This matches C `TFormulaVarIsFree`: it first trusts the formula `v_count`
/// cache, treats term identity as an occurrence, masks only `$qex` and `$qall`
/// binder variables, and otherwise recursively scans every argument. In
/// particular, named-lambda cells are not binder-aware in this direct query;
/// they are traversed like ordinary binary cells.
#[must_use]
pub fn tformula_var_is_free(bank: &TermBank, form: &Term, var: &Term) -> bool {
    if form.v_count() == 0 {
        return false;
    }
    if form == var {
        return true;
    }
    if form.f_code() == bank.signature().qex_code() || form.f_code() == bank.signature().qall_code()
    {
        if formula_argument(form, 0) == *var {
            false
        } else {
            tformula_var_is_free(bank, &formula_argument(form, 1), var)
        }
    } else {
        form.argument_clones()
            .into_iter()
            .flatten()
            .any(|arg| tformula_var_is_free(bank, &arg, var))
    }
}

/// Compatibility facade for C `TFormulaVarIsFreeCached`.
///
/// The checked C source declares this function in the header, but its only
/// implementation body is commented out and asserts the same result as
/// `TFormulaVarIsFree`. Rust therefore exposes the public surface as an alias
/// to the direct query rather than inventing a cache owned by the term bank.
#[must_use]
pub fn tformula_var_is_free_cached(bank: &TermBank, form: &Term, var: &Term) -> bool {
    tformula_var_is_free(bank, form, var)
}

/// Wraps a formula in one universal or existential quantifier.
///
/// This matches C `TFormulaAddQuantor`: `universal` selects `!`, otherwise
/// `?`, and the variable and formula are expected to already be bank-shared.
///
/// # Errors
///
/// Returns a diagnostic if the quantifier symbol is unavailable or the
/// resulting formula cannot be inserted into the bank.
pub fn tformula_add_quantor(
    bank: &mut TermBank,
    form: &Term,
    universal: bool,
    variable: &Term,
) -> Result<Term, Diagnostic> {
    let quantifier = if universal {
        bank.signature().qall_code()
    } else {
        bank.signature().qex_code()
    };
    tformula_fcode_alloc(bank, quantifier, variable.clone(), Some(form.clone()))
}

/// Allocates a formula with an explicit quantifier symbol.
///
/// This matches C `TFormulaQuantorAlloc`.
///
/// # Errors
///
/// Returns a diagnostic if the quantified formula cannot be allocated.
///
/// # Panics
///
/// Panics if `variable` is not a free variable.
pub fn tformula_quantor_alloc(
    bank: &mut TermBank,
    quantifier: i64,
    variable: &Term,
    arg: &Term,
) -> Result<Term, Diagnostic> {
    assert!(
        variable.is_free_var(),
        "TFormulaQuantorAlloc expects a free variable"
    );
    tformula_fcode_alloc(bank, quantifier, variable.clone(), Some(arg.clone()))
}

/// Wraps a formula in universal or existential quantifiers for `variables`.
///
/// This matches C `TFormulaAddQuantors` for a caller-provided variable order:
/// each variable is wrapped around the current formula in slice order.
///
/// # Errors
///
/// Returns a diagnostic if any quantifier allocation fails.
pub fn tformula_add_quantors(
    bank: &mut TermBank,
    form: &Term,
    universal: bool,
    variables: &[Term],
) -> Result<Term, Diagnostic> {
    let mut result = form.clone();
    for variable in variables {
        result = tformula_add_quantor(bank, &result, universal, variable)?;
    }
    Ok(result)
}

/// Returns the universal or existential closure of a term-encoded formula.
///
/// This matches C `TFormulaClosure`, with free variables collected by
/// [`tformula_collect_free_vars`] and then wrapped through
/// [`tformula_add_quantors`].
///
/// # Errors
///
/// Returns a diagnostic if any quantifier allocation fails.
pub fn tformula_closure(
    bank: &mut TermBank,
    form: &Term,
    universal: bool,
) -> Result<Term, Diagnostic> {
    let variables = tformula_collect_free_vars(bank, form);
    tformula_add_quantors(bank, form, universal, &variables)
}

/// Combines a stack of formulas with `op`, destructively popping the stack.
///
/// This matches C `TFormulaStackToForm`: an empty stack returns `$true`; a
/// non-empty stack starts from the last pushed formula and wraps remaining
/// popped formulas on the left.
///
/// # Errors
///
/// Returns a diagnostic if a binary formula allocation fails.
pub fn tformula_stack_to_form(
    bank: &mut TermBank,
    stack: &mut Vec<Term>,
    op: i64,
) -> Result<Term, Diagnostic> {
    let Some(mut result) = stack.pop() else {
        return Ok(bank.true_term().clone());
    };

    while let Some(handle) = stack.pop() {
        result = tformula_fcode_alloc(bank, op, handle, Some(result))?;
    }
    Ok(result)
}

/// Expands a `$distinct` pseudo-formula into pairwise disequalities.
///
/// This matches C `TFormulaExpandDistinct`: all `i < j` argument pairs are
/// converted to `$neq(arg_i,arg_j)`, then combined with
/// [`tformula_stack_to_form`] using conjunction.
///
/// # Errors
///
/// Returns a diagnostic if allocating a disequality or conjunction fails.
///
/// # Panics
///
/// Panics if `distinct` has uninitialized argument slots.
pub fn tformula_expand_distinct(bank: &mut TermBank, distinct: &Term) -> Result<Term, Diagnostic> {
    let disequality_code = bank.signature_mut().get_eqn_code(false);
    let mut disequalities = Vec::new();
    for left_index in 0..distinct.arity() {
        let left = formula_argument(distinct, left_index);
        for right_index in (left_index + 1)..distinct.arity() {
            let right = formula_argument(distinct, right_index);
            let disequality =
                tformula_fcode_alloc(bank, disequality_code, left.clone(), Some(right))?;
            disequalities.push(disequality);
        }
    }

    let and_code = bank.signature().and_code();
    tformula_stack_to_form(bank, &mut disequalities, and_code)
}

/// Returns whether a formula tree contains only individual and Boolean types.
///
/// This matches C `TFormulaIsUntyped`, delegated to the term-level query.
#[must_use]
pub fn tformula_is_untyped(form: &Term) -> bool {
    term_is_untyped(form)
}

/// Writes the C `TFormulaTPTPPrint` rendering for a term-encoded formula.
///
/// This covers the direct term-formula printer used for TPTP/TSTP-style
/// formula output: literals delegate to `EqnFOFPrint`, left-spine
/// disjunctions are flattened, repeated adjacent quantifiers are coalesced,
/// `$ite`/`$let` and phony applications are printed through the term printer,
/// and malformed quantified cells fall back to debug term output.
///
/// # Errors
///
/// Returns a diagnostic if temporary equation allocation, term/type rendering,
/// or output formatting fails.
///
/// # Panics
///
/// Panics if a formula cell has the C-required shape but uninitialized child
/// arguments, or if a binary connective has an unexpected operator.
pub fn tformula_write_tptp(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    full_terms: bool,
    options: TFormulaTptpPrintOptions,
) -> Result<(), Diagnostic> {
    if tformula_is_literal(bank, form) {
        let literal = Eqn::alloc(
            formula_argument(form, 0),
            formula_argument(form, 1),
            bank,
            true,
        )?;
        return eqn_write_fof(
            output,
            bank,
            &literal,
            form.f_code() == bank.signature().neqn_code(),
            full_terms,
            options.eqn_options,
        )
        .map_err(tformula_write_error);
    }

    if form.is_free_var() {
        return tformula_write_term_print(output, bank, form, options);
    }

    if form.f_code() == SIG_PHONY_APP_CODE {
        output.write_char('(').map_err(tformula_write_error)?;
        tformula_write_term_print(output, bank, form, options)?;
        output.write_char(')').map_err(tformula_write_error)?;
        return Ok(());
    }

    if tformula_is_quantified(bank, form) {
        return tformula_write_tptp_quantifier(output, bank, form, full_terms, options);
    }

    if form.arity() == 1 {
        output.write_str("~(").map_err(tformula_write_error)?;
        tformula_write_tptp(
            output,
            bank,
            &formula_argument(form, 0),
            full_terms,
            options,
        )?;
        output.write_char(')').map_err(tformula_write_error)?;
        return Ok(());
    }

    if form.arity() == 0 {
        let name = bank.signature().find_name(form.f_code()).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "TFormulaTPTPPrint arity-zero formula symbol has no name",
            )
        })?;
        output.write_str(name).map_err(tformula_write_error)?;
        return Ok(());
    }

    if form.f_code() == bank.signature().distinct_code() {
        return bank
            .write_term_with_type_suffixes(
                output,
                form,
                full_terms,
                options.eqn_options.print_types,
            )
            .map_err(tformula_write_error);
    }

    assert!(
        matches!(form.f_code(), SIG_LET_CODE | SIG_ITE_CODE) || form.arity() == 2,
        "TFormulaTPTPPrint expects $let, $ite, or a binary formula"
    );
    output.write_char('(').map_err(tformula_write_error)?;
    if matches!(form.f_code(), SIG_LET_CODE | SIG_ITE_CODE) {
        tformula_write_term_print(output, bank, form, options)?;
    } else if form.f_code() == bank.signature().or_code() {
        tformula_write_tptp_or_chain(output, bank, form, full_terms, options)?;
    } else {
        tformula_write_tptp(
            output,
            bank,
            &formula_argument(form, 0),
            full_terms,
            options,
        )?;
        let operator = tformula_app_encoded_binary_operator(bank, form.f_code())
            .unwrap_or_else(|| panic!("TFormulaTPTPPrint binary formula has the wrong operator"));
        output.write_str(operator).map_err(tformula_write_error)?;
        tformula_write_tptp(
            output,
            bank,
            &formula_argument(form, 1),
            full_terms,
            options,
        )?;
    }
    output.write_char(')').map_err(tformula_write_error)?;
    Ok(())
}

/// Returns the C `TFormulaTPTPPrint` rendering for a term-encoded formula.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`tformula_write_tptp`].
pub fn tformula_tptp_string(
    bank: &mut TermBank,
    form: &Term,
    full_terms: bool,
    options: TFormulaTptpPrintOptions,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    tformula_write_tptp(&mut output, bank, form, full_terms, options)?;
    Ok(output)
}

fn tformula_write_tptp_quantifier(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    full_terms: bool,
    options: TFormulaTptpPrintOptions,
) -> Result<(), Diagnostic> {
    if form.arity() != 2 {
        return bank
            .write_term_debug(output, form, options.problem_type)
            .map_err(tformula_write_error);
    }

    let quantifier = form.f_code();
    output
        .write_str(if quantifier == bank.signature().qex_code() {
            "?["
        } else if quantifier == bank.signature().qall_code() {
            "!["
        } else {
            "^["
        })
        .map_err(tformula_write_error)?;

    let mut current = form.clone();
    loop {
        let variable = formula_argument(&current, 0);
        tformula_write_quantified_variable(output, bank, &variable, options)?;

        let body = formula_argument(&current, 1);
        if body.f_code() != quantifier {
            output.write_str("]:(").map_err(tformula_write_error)?;
            tformula_write_tptp(output, bank, &body, full_terms, options)?;
            output.write_char(')').map_err(tformula_write_error)?;
            return Ok(());
        }

        output.write_str(", ").map_err(tformula_write_error)?;
        current = body;
    }
}

fn tformula_write_quantified_variable(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    variable: &Term,
    options: TFormulaTptpPrintOptions,
) -> Result<(), Diagnostic> {
    tformula_write_term_print(output, bank, variable, options)?;
    let type_ = variable.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            "TFormulaTPTPPrint quantified variable has no type",
        )
    })?;
    if options.problem_type == ProblemType::HigherOrder || !type_.is_individual() {
        output.write_char(':').map_err(tformula_write_error)?;
        tformula_write_tstp_type(output, bank, &type_, options.problem_type)?;
    }
    Ok(())
}

fn tformula_write_tptp_or_chain(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    full_terms: bool,
    options: TFormulaTptpPrintOptions,
) -> Result<(), Diagnostic> {
    if form.f_code() != bank.signature().or_code() {
        return tformula_write_tptp(output, bank, form, full_terms, options);
    }

    tformula_write_tptp_or_chain(
        output,
        bank,
        &formula_argument(form, 0),
        full_terms,
        options,
    )?;
    output.write_char('|').map_err(tformula_write_error)?;
    tformula_write_tptp(
        output,
        bank,
        &formula_argument(form, 1),
        full_terms,
        options,
    )
}

fn tformula_write_term_print(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    term: &Term,
    options: TFormulaTptpPrintOptions,
) -> Result<(), Diagnostic> {
    if options.problem_type == ProblemType::FirstOrder && options.eqn_options.print_types {
        bank.write_term_with_type_suffixes(output, term, true, true)
    } else {
        bank.write_term_deref_for_problem(output, term, options.problem_type, DerefType::Never)
    }
    .map_err(tformula_write_error)
}

fn tformula_write_tstp_type(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    type_: &Type,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    let mut rendered = Vec::new();
    bank.signature()
        .type_bank()
        .print_tstp(&mut rendered, type_, problem_type)
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write type"))?;
    let rendered = String::from_utf8(rendered)
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write type"))?;
    output.write_str(&rendered).map_err(tformula_write_error)
}

/// Writes the C `TFormulaAppEncode` rendering for a term-encoded formula.
///
/// This uses the same temporary term application encoding as `EqnAppEncode`.
/// The source formula is not mutated, but the term bank signature may gain
/// typed-application symbols and intermediate app-encoded types.
///
/// # Errors
///
/// Returns a diagnostic if term app-encoding, type-name rendering, equation
/// allocation, or output formatting fails.
///
/// # Panics
///
/// Panics if a formula cell violates the C shape preconditions: literals and
/// binary nodes must have initialized arguments, quantified variables must be
/// free variables, unary formula nodes must be negations, and binary formula
/// nodes must use one of the known logical connectives.
pub fn tformula_write_app_encode(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
) -> Result<(), Diagnostic> {
    tformula_write_app_encode_with_type_suffixes(output, bank, form, false)
}

/// Writes the C `TFormulaAppEncode` rendering with optional `TermPrintTypes` suffixes.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`tformula_write_app_encode`].
///
/// # Panics
///
/// Panics under the same conditions as [`tformula_write_app_encode`].
pub fn tformula_write_app_encode_with_type_suffixes(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    if tformula_is_literal(bank, form) {
        let literal = Eqn::alloc(
            formula_argument(form, 0),
            formula_argument(form, 1),
            bank,
            true,
        )?;
        return tformula_write_app_encoded_literal(
            output,
            bank,
            &literal,
            form.f_code() == bank.signature().neqn_code(),
            print_types,
        );
    }

    if tformula_is_quantified(bank, form) {
        return tformula_write_app_encoded_quantifier(output, bank, form, print_types);
    }

    if form.f_code() == SIG_ITE_CODE {
        return tformula_write_app_encoded_ite(output, bank, form, print_types);
    }

    if form.f_code() == SIG_LET_CODE {
        return tformula_write_app_encoded_let(output, bank, form, print_types);
    }

    if form.arity() == 1 {
        assert_eq!(
            form.f_code(),
            bank.signature().not_code(),
            "TFormulaAppEncode unary formula must be negation"
        );
        output.write_str("~(").map_err(tformula_write_error)?;
        tformula_write_app_encode_with_type_suffixes(
            output,
            bank,
            &formula_argument(form, 0),
            print_types,
        )?;
        output.write_char(')').map_err(tformula_write_error)?;
        return Ok(());
    }

    assert_eq!(
        form.arity(),
        2,
        "TFormulaAppEncode expects a binary formula"
    );
    output.write_char('(').map_err(tformula_write_error)?;
    if form.f_code() == bank.signature().or_code() {
        tformula_write_app_encoded_or_chain(output, bank, form, print_types)?;
    } else {
        tformula_write_app_encode_with_type_suffixes(
            output,
            bank,
            &formula_argument(form, 0),
            print_types,
        )?;
        let operator = tformula_app_encoded_binary_operator(bank, form.f_code())
            .unwrap_or_else(|| panic!("TFormulaAppEncode binary formula has the wrong operator"));
        output.write_str(operator).map_err(tformula_write_error)?;
        tformula_write_app_encode_with_type_suffixes(
            output,
            bank,
            &formula_argument(form, 1),
            print_types,
        )?;
    }
    output.write_char(')').map_err(tformula_write_error)?;
    Ok(())
}

/// Returns the C `TFormulaAppEncode` rendering for a term-encoded formula.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`tformula_write_app_encode`].
pub fn tformula_app_encode_string(bank: &mut TermBank, form: &Term) -> Result<String, Diagnostic> {
    tformula_app_encode_string_with_type_suffixes(bank, form, false)
}

/// Returns the C `TFormulaAppEncode` rendering with optional type suffixes.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as [`tformula_app_encode_string`].
pub fn tformula_app_encode_string_with_type_suffixes(
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    tformula_write_app_encode_with_type_suffixes(&mut output, bank, form, print_types)?;
    Ok(output)
}

/// Preloads the intermediate types needed by app-encoding a term formula.
///
/// This matches C `PreloadTypes`: literal sides are app-encoded and immediately
/// discarded so that typed-application symbols and suffix arrow types have
/// already been inserted before declaration printing.
///
/// # Errors
///
/// Returns a diagnostic if term app-encoding fails.
///
/// # Panics
///
/// Panics if a formula node has an unexpected arity or uninitialized
/// arguments, matching the C assertions and direct argument access.
pub fn tformula_preload_types(bank: &mut TermBank, form: &Term) -> Result<(), Diagnostic> {
    if tformula_is_literal(bank, form) {
        tformula_preload_app_encoded_formula_or_term(bank, &formula_argument(form, 0))?;
        tformula_preload_app_encoded_formula_or_term(bank, &formula_argument(form, 1))?;
    } else if tformula_is_quantified(bank, form) {
        tformula_preload_types(bank, &formula_argument(form, 1))?;
    } else if form.f_code() == SIG_ITE_CODE {
        for index in 0..form.arity() {
            tformula_preload_app_encoded_formula_or_term(bank, &formula_argument(form, index))?;
        }
    } else if form.f_code() == SIG_LET_CODE {
        tformula_preload_app_encoded_let(bank, form)?;
    } else if form.arity() == 1 {
        tformula_preload_types(bank, &formula_argument(form, 0))?;
    } else {
        assert_eq!(form.arity(), 2, "PreloadTypes expects a binary formula");
        tformula_preload_types(bank, &formula_argument(form, 0))?;
        tformula_preload_types(bank, &formula_argument(form, 1))?;
    }
    Ok(())
}

fn tformula_write_app_encoded_literal(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    literal: &Eqn,
    negated: bool,
    print_types: bool,
) -> Result<(), Diagnostic> {
    let positive = literal.is_positive() ^ negated;
    if literal.is_equ_lit(bank) {
        tformula_write_app_encoded_formula_or_term(output, bank, literal.left(), print_types)?;
        if !positive {
            output
                .write_char('!')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        }
        output
            .write_char('=')
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        tformula_write_app_encoded_formula_or_term(output, bank, literal.right(), print_types)?;
    } else {
        if !positive {
            output
                .write_char('~')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))?;
        }
        tformula_write_app_encoded_formula_or_term(output, bank, literal.left(), print_types)?;
    }
    Ok(())
}

fn tformula_preload_app_encoded_formula_or_term(
    bank: &mut TermBank,
    term: &Term,
) -> Result<(), Diagnostic> {
    if term.f_code() == SIG_ITE_CODE {
        for index in 0..term.arity() {
            tformula_preload_app_encoded_formula_or_term(bank, &formula_argument(term, index))?;
        }
        return Ok(());
    }

    if term.f_code() == SIG_LET_CODE {
        return tformula_preload_app_encoded_let(bank, term);
    }

    if term.type_().as_ref().is_some_and(Type::is_bool) {
        if tformula_is_app_encoded_formula_node(bank, term) {
            return tformula_preload_types(bank, term);
        }
        let literal = Eqn::alloc(term.clone(), bank.true_term().clone(), bank, true)?;
        let mut sink = String::new();
        return eqn_write_app_encode(&mut sink, bank, &literal, false);
    }
    let _encoded = term_app_encode(term, bank.signature_mut())?;
    Ok(())
}

fn tformula_write_app_encoded_formula_or_term(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    term: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    if term.f_code() == SIG_ITE_CODE {
        return tformula_write_app_encoded_ite(output, bank, term, print_types);
    }

    if term.f_code() == SIG_LET_CODE {
        return tformula_write_app_encoded_let(output, bank, term, print_types);
    }

    if term.type_().as_ref().is_some_and(Type::is_bool) {
        if tformula_is_app_encoded_formula_node(bank, term) {
            return tformula_write_app_encode_with_type_suffixes(output, bank, term, print_types);
        }
        let literal = Eqn::alloc(term.clone(), bank.true_term().clone(), bank, true)?;
        return eqn_write_app_encode_with_type_suffixes(output, bank, &literal, false, print_types);
    }

    let encoded = term_app_encode(term, bank.signature_mut())?;
    bank.write_term_with_type_suffixes(output, &encoded, true, print_types)
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write equation"))
}

fn tformula_is_app_encoded_formula_node(bank: &TermBank, term: &Term) -> bool {
    tformula_is_literal(bank, term)
        || tformula_is_quantified(bank, term)
        || term.f_code() == bank.signature().not_code()
        || term.f_code() == SIG_ITE_CODE
        || term.f_code() == SIG_LET_CODE
        || tformula_app_encoded_binary_operator(bank, term.f_code()).is_some()
}

fn tformula_write_app_encoded_ite(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    assert_eq!(form.arity(), 3, "$ite formula must have three arguments");
    output
        .write_str("$ite(")
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    for index in 0..form.arity() {
        if index != 0 {
            output
                .write_char(',')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        }
        tformula_write_app_encoded_formula_or_term(
            output,
            bank,
            &formula_argument(form, index),
            print_types,
        )?;
    }
    output
        .write_char(')')
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))
}

fn tformula_preload_app_encoded_let(bank: &mut TermBank, form: &Term) -> Result<(), Diagnostic> {
    assert!(form.arity() >= 1, "$let formula must have at least a body");
    for index in 0..form.arity().saturating_sub(1) {
        let definition = formula_argument(form, index);
        let (left, right) = tformula_app_encoded_let_definition_parts(&definition);
        tformula_preload_app_encoded_formula_or_term(bank, &left)?;
        tformula_preload_app_encoded_formula_or_term(bank, &right)?;
    }
    tformula_preload_app_encoded_formula_or_term(bank, &formula_argument(form, form.arity() - 1))
}

fn tformula_write_app_encoded_let(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    assert!(form.arity() >= 1, "$let formula must have at least a body");
    let mut definitions = Vec::with_capacity(form.arity().saturating_sub(1));
    for index in 0..form.arity().saturating_sub(1) {
        definitions.push(tformula_app_encoded_let_definition_parts(
            &formula_argument(form, index),
        ));
    }

    output
        .write_str("$let(")
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    tformula_write_app_encoded_let_declarations(output, bank, &definitions)?;
    output
        .write_char(',')
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    tformula_write_app_encoded_let_definitions(output, bank, &definitions, print_types)?;
    output
        .write_char(',')
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    tformula_write_app_encoded_formula_or_term(
        output,
        bank,
        &formula_argument(form, form.arity() - 1),
        print_types,
    )?;
    output
        .write_char(')')
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))
}

fn tformula_app_encoded_let_definition_parts(definition: &Term) -> (Term, Term) {
    assert!(definition.arity() == 2, "$let definition must be binary");
    (
        formula_argument(definition, 0),
        formula_argument(definition, 1),
    )
}

fn tformula_write_app_encoded_let_declarations(
    output: &mut impl fmt::Write,
    bank: &TermBank,
    definitions: &[(Term, Term)],
) -> Result<(), Diagnostic> {
    for (index, (left, _right)) in definitions.iter().enumerate() {
        if index != 0 {
            output
                .write_char(',')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        }
        bank.write_term(output, left, true)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        output
            .write_char(':')
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        let type_ = left
            .type_()
            .unwrap_or_else(|| panic!("$let definition left side must have a type"));
        let type_name = type_app_encoded_name(&type_)?;
        output
            .write_str(&type_name)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    }
    Ok(())
}

fn tformula_write_app_encoded_let_definitions(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    definitions: &[(Term, Term)],
    print_types: bool,
) -> Result<(), Diagnostic> {
    for (index, (left, right)) in definitions.iter().enumerate() {
        if index != 0 {
            output
                .write_char(',')
                .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        }
        tformula_write_app_encoded_formula_or_term(output, bank, left, print_types)?;
        output
            .write_str(":=")
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        tformula_write_app_encoded_formula_or_term(output, bank, right, print_types)?;
    }
    Ok(())
}

fn tformula_write_app_encoded_quantifier(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    let quantifier = form.f_code();
    output
        .write_str(if quantifier == bank.signature().qex_code() {
            "?["
        } else {
            "!["
        })
        .map_err(tformula_write_error)?;

    let mut current = form.clone();
    loop {
        let variable = formula_argument(&current, 0);
        assert!(
            variable.is_free_var(),
            "TFormulaAppEncode quantified variable must be free"
        );
        let type_ = variable.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "app-encoded quantified variable has no type",
            )
        })?;
        let type_name = type_app_encoded_name(&type_)?;
        write!(output, "{}:{type_name}", bank.term_string(&variable, true))
            .map_err(tformula_write_error)?;

        let body = formula_argument(&current, 1);
        if body.f_code() != quantifier {
            output.write_str("]:").map_err(tformula_write_error)?;
            return tformula_write_app_encode_with_type_suffixes(output, bank, &body, print_types);
        }

        output.write_str(", ").map_err(tformula_write_error)?;
        current = body;
    }
}

fn tformula_write_app_encoded_or_chain(
    output: &mut impl fmt::Write,
    bank: &mut TermBank,
    form: &Term,
    print_types: bool,
) -> Result<(), Diagnostic> {
    if form.f_code() != bank.signature().or_code() {
        return tformula_write_app_encode_with_type_suffixes(output, bank, form, print_types);
    }

    tformula_write_app_encoded_or_chain(output, bank, &formula_argument(form, 0), print_types)?;
    output.write_char('|').map_err(tformula_write_error)?;
    tformula_write_app_encode_with_type_suffixes(
        output,
        bank,
        &formula_argument(form, 1),
        print_types,
    )
}

fn tformula_app_encoded_binary_operator(bank: &TermBank, f_code: i64) -> Option<&'static str> {
    if f_code == bank.signature().and_code() {
        Some("&")
    } else if f_code == bank.signature().or_code() {
        Some("|")
    } else if f_code == bank.signature().impl_code() {
        Some("=>")
    } else if f_code == bank.signature().equiv_code() {
        Some("<=>")
    } else if f_code == bank.signature().nand_code() {
        Some("~&")
    } else if f_code == bank.signature().nor_code() {
        Some("~|")
    } else if f_code == bank.signature().bimpl_code() {
        Some("<=")
    } else if f_code == bank.signature().xor_code() {
        Some("<~>")
    } else {
        None
    }
}

fn tformula_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula")
}

/// Estimates the number of clauses produced by clausifying a formula.
///
/// This matches C `TFormulaEstimateClauses` for a single term-encoded formula:
/// literals, marked definition subforms, applied free variables, and
/// higher-order/partially applied formulas count as one clause; `$true` counts
/// as zero; and estimates above C's `TFORM_MANY_LIMIT` return
/// [`TFORM_MANY_CLAUSES`].
///
/// # Panics
///
/// Panics if a logical formula cell is malformed and lacks the C-required
/// child arguments for its connective.
#[must_use]
pub fn tformula_estimate_clauses(bank: &TermBank, form: &Term, pos: bool) -> i64 {
    if form.query_prop(TP_CHECK_FLAG)
        || tformula_is_literal(bank, form)
        || form.type_().as_ref().is_some_and(Type::is_arrow)
    {
        return 1;
    }
    if form == bank.true_term() {
        return 0;
    }
    if form == bank.false_term() || form.is_applied_free_var() {
        return 1;
    }

    let result = if pos {
        estimate_positive_clauses(bank, form)
    } else {
        estimate_negative_clauses(bank, form)
    };
    if result > TFORM_MANY_LIMIT {
        TFORM_MANY_CLAUSES
    } else {
        result
    }
}

/// Returns or creates the definition atom for a formula.
///
/// This matches C `TFormulaDefRename`: definitions are keyed by the formula
/// cell's `entry_no`, repeated requests with different polarities generalize
/// the stored polarity to `0`, and a new definition atom is a fresh Boolean
/// Skolem/predicate equated to `$true`.
///
/// # Errors
///
/// Returns a diagnostic if allocating or encoding the fresh definition atom
/// fails.
///
/// # Panics
///
/// Panics if `polarity` is outside C's `-1..=1` range.
pub fn tformula_def_rename(
    bank: &mut TermBank,
    form: &Term,
    polarity: i32,
    defs: &mut TFormulaDefinitions,
    renamed_forms: &mut Vec<Term>,
) -> Result<Term, Diagnostic> {
    assert!(
        (-1..=1).contains(&polarity),
        "TFormulaDefRename polarity must be -1, 0, or 1"
    );

    if let Some(definition) = defs.get_mut(&form.entry_no()) {
        if polarity != definition.polarity {
            definition.polarity = 0;
        }
        return Ok(definition.rename_atom.clone());
    }

    let free_vars = tformula_collect_free_vars(bank, form);
    let bool_type = bank.signature().type_bank().bool_type();
    let skolem = bank.alloc_new_skolem(&free_vars, Some(&bool_type))?;
    let true_term = bank.true_term().clone();
    let rename_atom =
        Eqn::terms_tb_term_encode(bank, &skolem, &true_term, true, PatEqnDirection::Normal)?;

    defs.insert(
        form.entry_no(),
        TFormulaDefEntry {
            polarity,
            rename_atom: rename_atom.clone(),
            real_definition_id: None,
            archived_definition: None,
            archived_definition_ref: None,
        },
    );
    form.set_prop(TP_CHECK_FLAG);
    renamed_forms.push(form.clone());

    Ok(rename_atom)
}

/// Finds subformulas that should receive definitional CNF atoms.
///
/// This matches C `TFormulaFindDefs` for a single term-encoded formula. The
/// traversal is depth-first, preserves C polarity propagation, and deliberately
/// continues below an already marked formula because C may need to generalize
/// subformula polarities after re-adding the marked root.
///
/// # Errors
///
/// Returns a diagnostic if creating a definition atom fails.
///
/// # Panics
///
/// Panics if `polarity` is outside C's `-1..=1` range.
pub fn tformula_find_defs(
    bank: &mut TermBank,
    form: &Term,
    polarity: i32,
    def_limit: i64,
    defs: &mut TFormulaDefinitions,
    renamed_forms: &mut Vec<Term>,
) -> Result<(), Diagnostic> {
    assert!(
        (-1..=1).contains(&polarity),
        "TFormulaFindDefs polarity must be -1, 0, or 1"
    );

    if tformula_is_literal(bank, form) || form.type_().as_ref().is_some_and(Type::is_arrow) {
        return Ok(());
    }

    if form.query_prop(TP_CHECK_FLAG) {
        tformula_def_rename(bank, form, polarity, defs, renamed_forms)?;
    }

    let (and_code, or_code, not_code, implication_code, equivalence_code, qex_code, qall_code) = {
        let sig = bank.signature();
        (
            sig.and_code(),
            sig.or_code(),
            sig.not_code(),
            sig.impl_code(),
            sig.equiv_code(),
            sig.qex_code(),
            sig.qall_code(),
        )
    };

    let f_code = form.f_code();
    if f_code == and_code || f_code == or_code {
        let left = formula_argument(form, 0);
        tformula_find_defs(bank, &left, polarity, def_limit, defs, renamed_forms)?;
        if tformula_rename_test(bank, form, 0, polarity, def_limit) {
            tformula_def_rename(bank, &left, polarity, defs, renamed_forms)?;
        }
    } else if f_code == not_code || f_code == implication_code {
        let left = formula_argument(form, 0);
        let child_polarity = -polarity;
        tformula_find_defs(bank, &left, child_polarity, def_limit, defs, renamed_forms)?;
        if tformula_rename_test(bank, form, 0, child_polarity, def_limit) {
            tformula_def_rename(bank, &left, child_polarity, defs, renamed_forms)?;
        }
    } else if f_code == equivalence_code {
        let left = formula_argument(form, 0);
        tformula_find_defs(bank, &left, 0, def_limit, defs, renamed_forms)?;
        if tformula_rename_test(bank, form, 0, polarity, def_limit) {
            tformula_def_rename(bank, &left, 0, defs, renamed_forms)?;
        }
    }

    if f_code == and_code
        || f_code == or_code
        || f_code == implication_code
        || f_code == qex_code
        || f_code == qall_code
    {
        let right = formula_argument(form, 1);
        tformula_find_defs(bank, &right, polarity, def_limit, defs, renamed_forms)?;
        if tformula_rename_test(bank, form, 1, polarity, def_limit) {
            tformula_def_rename(bank, &right, polarity, defs, renamed_forms)?;
        }
    } else if f_code == equivalence_code {
        let right = formula_argument(form, 1);
        tformula_find_defs(bank, &right, 0, def_limit, defs, renamed_forms)?;
        if tformula_rename_test(bank, form, 1, polarity, def_limit) {
            tformula_def_rename(bank, &right, 0, defs, renamed_forms)?;
        }
    }

    Ok(())
}

/// Copies a formula while replacing marked definition subformulas.
///
/// This matches C `TFormulaCopyDef`: marked subformulas are replaced by their
/// definition atom unless the caller is currently building the corresponding
/// real definition. Used definitions are recorded through their archived
/// polarity-zero definition formula term, mirroring C's `vals[3]` pointer until
/// full `WFormula` ownership exists.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a changed formula fails.
///
/// # Panics
///
/// Panics if a marked formula has no definition entry, or if a used definition
/// entry has not yet been populated with the metadata produced by definition
/// introduction.
pub fn tformula_copy_def(
    bank: &mut TermBank,
    form: &Term,
    blocked: i64,
    defs: &TFormulaDefinitions,
    defs_used: &mut Vec<FormulaDerivationRef>,
) -> Result<Term, Diagnostic> {
    if tformula_is_literal(bank, form)
        || form.is_applied_free_var()
        || form.type_().as_ref().is_some_and(Type::is_arrow)
        || form == bank.true_term()
        || form == bank.false_term()
        || form.is_any_var()
        || form.f_code() <= 0
        || !bank.signature().is_logical_symbol(form.f_code())
    {
        return Ok(form.clone());
    }

    if form.query_prop(TP_CHECK_FLAG) {
        let definition = defs
            .get(&form.entry_no())
            .unwrap_or_else(|| panic!("marked formula {} must have a definition", form.entry_no()));
        let real_definition_id = definition
            .real_definition_id
            .unwrap_or_else(|| panic!("definition {} must have a real id", form.entry_no()));
        if real_definition_id != blocked {
            let archived_definition_ref = definition.archived_definition_ref.unwrap_or_else(|| {
                panic!(
                    "definition {} must have an archived formula ref",
                    form.entry_no()
                )
            });
            defs_used.push(archived_definition_ref);
            return Ok(definition.rename_atom.clone());
        }
    }

    let (
        and_code,
        or_code,
        implication_code,
        equivalence_code,
        negated_conjunction_code,
        negated_disjunction_code,
        reverse_implication_code,
        exclusive_or_code,
        negation_code,
        qex_code,
        qall_code,
    ) = {
        let sig = bank.signature();
        (
            sig.and_code(),
            sig.or_code(),
            sig.impl_code(),
            sig.equiv_code(),
            sig.nand_code(),
            sig.nor_code(),
            sig.bimpl_code(),
            sig.xor_code(),
            sig.not_code(),
            sig.qex_code(),
            sig.qall_code(),
        )
    };

    let f_code = form.f_code();
    let left = if matches!(
        f_code,
        code if code == and_code
            || code == or_code
            || code == implication_code
            || code == equivalence_code
            || code == negated_conjunction_code
            || code == negated_disjunction_code
            || code == reverse_implication_code
            || code == exclusive_or_code
            || code == negation_code
    ) {
        tformula_copy_def(bank, &formula_argument(form, 0), blocked, defs, defs_used)?
    } else {
        assert!(
            f_code == qex_code || f_code == qall_code,
            "TFormulaCopyDef expects a connective or quantifier"
        );
        formula_argument(form, 0)
    };

    let right = if f_code == negation_code {
        None
    } else {
        Some(tformula_copy_def(
            bank,
            &formula_argument(form, 1),
            blocked,
            defs,
            defs_used,
        )?)
    };

    tformula_fcode_alloc(bank, f_code, left, right)
}

fn tformula_rename_test(
    bank: &TermBank,
    root: &Term,
    position: usize,
    polarity: i32,
    def_limit: i64,
) -> bool {
    assert!(
        (-1..=1).contains(&polarity),
        "tformula_rename_test polarity must be -1, 0, or 1"
    );

    let sig = bank.signature();
    if root.f_code() == sig.qex_code() || root.f_code() == sig.qall_code() {
        return false;
    }

    let child = formula_argument(root, position);
    if root.f_code() == sig.equiv_code() {
        return tformula_estimate_clauses(bank, &child, true) > def_limit
            || tformula_estimate_clauses(bank, &child, false) > def_limit;
    }

    match polarity {
        1 => {
            if root.f_code() == sig.or_code()
                && tformula_estimate_clauses(bank, &child, true) > def_limit
            {
                return true;
            }
            let subform_sign = position == 2;
            root.f_code() == sig.impl_code()
                && tformula_estimate_clauses(bank, &child, subform_sign) > def_limit
        }
        -1 => {
            root.f_code() == sig.and_code()
                && tformula_estimate_clauses(bank, &child, false) > def_limit
        }
        0 => {
            (root.f_code() == sig.and_code()
                || root.f_code() == sig.or_code()
                || root.f_code() == sig.impl_code())
                && (tformula_estimate_clauses(bank, &child, true) > def_limit
                    || tformula_estimate_clauses(bank, &child, false) > def_limit)
        }
        _ => unreachable!("polarity assertion above covers all cases"),
    }
}

/// Builds a definition formula for a definition atom and a defined formula.
///
/// This matches C `TFormulaCreateDef`: negative polarity creates
/// `defined -> def_atom`, positive polarity creates `def_atom -> defined`,
/// polarity `0` creates `def_atom <=> defined`, and the result is universally
/// closed over the variables occurring in `def_atom`.
///
/// # Errors
///
/// Returns a diagnostic if allocating a connective or quantifier fails.
///
/// # Panics
///
/// Panics if `polarity` is outside C's `-1..=1` range, or if the C polarity
/// marker assertions fail.
pub fn tformula_create_def(
    bank: &mut TermBank,
    def_atom: &Term,
    defined: &Term,
    polarity: i32,
) -> Result<Term, Diagnostic> {
    let (implication_code, equivalence_code, universal_code) = {
        let sig = bank.signature();
        (sig.impl_code(), sig.equiv_code(), sig.qall_code())
    };

    let mut result = match polarity {
        -1 => {
            assert!(
                !defined.query_prop(TP_POS_POLARITY),
                "negative definition must not be marked positive"
            );
            tformula_fcode_alloc(
                bank,
                implication_code,
                defined.clone(),
                Some(def_atom.clone()),
            )?
        }
        0 => tformula_fcode_alloc(
            bank,
            equivalence_code,
            def_atom.clone(),
            Some(defined.clone()),
        )?,
        1 => {
            assert!(
                !defined.query_prop(TP_NEG_POLARITY),
                "positive definition must not be marked negative"
            );
            tformula_fcode_alloc(
                bank,
                implication_code,
                def_atom.clone(),
                Some(defined.clone()),
            )?
        }
        _ => panic!("TFormulaCreateDef polarity must be -1, 0, or 1"),
    };

    for variable in tformula_collect_free_vars(bank, def_atom) {
        result = tformula_fcode_alloc(bank, universal_code, variable, Some(result))?;
    }

    Ok(result)
}

/// Marks formula polarity flags on a term-encoded formula tree.
///
/// This matches C `TFormulaMarkPolarity`: literals are not marked; `not` and
/// the left side of implication invert polarity; equivalence children are
/// marked with both polarities; and quantifier bodies inherit root polarity.
///
/// # Panics
///
/// Panics if `polarity` is outside C's `-1..=1` range or if a traversed formula
/// cell is malformed.
pub fn tformula_mark_polarity(bank: &TermBank, form: &Term, polarity: i32) {
    assert!(
        (-1..=1).contains(&polarity),
        "TFormulaMarkPolarity polarity must be -1, 0, or 1"
    );

    if tformula_is_literal(bank, form) {
        return;
    }

    match polarity {
        -1 => form.set_prop(TP_NEG_POLARITY),
        0 => form.set_prop(TP_POS_POLARITY | TP_NEG_POLARITY),
        1 => form.set_prop(TP_POS_POLARITY),
        _ => unreachable!("polarity assertion above covers all cases"),
    }

    let (and_code, or_code, not_code, implication_code, equivalence_code, qex_code, qall_code) = {
        let sig = bank.signature();
        (
            sig.and_code(),
            sig.or_code(),
            sig.not_code(),
            sig.impl_code(),
            sig.equiv_code(),
            sig.qex_code(),
            sig.qall_code(),
        )
    };

    let f_code = form.f_code();
    if f_code == and_code || f_code == or_code {
        tformula_mark_polarity(bank, &formula_argument(form, 0), polarity);
    } else if f_code == not_code || f_code == implication_code {
        tformula_mark_polarity(bank, &formula_argument(form, 0), -polarity);
    } else if f_code == equivalence_code {
        tformula_mark_polarity(bank, &formula_argument(form, 0), 0);
    }

    if f_code == and_code
        || f_code == or_code
        || f_code == implication_code
        || f_code == qex_code
        || f_code == qall_code
    {
        tformula_mark_polarity(bank, &formula_argument(form, 1), polarity);
    } else if f_code == equivalence_code {
        tformula_mark_polarity(bank, &formula_argument(form, 1), 0);
    }
}

/// Decodes the polarity flags on a formula.
///
/// This matches C `TFormulaDecodePolarity`: both flags mean `0`, only positive
/// means `1`, only negative means `-1`.
///
/// # Panics
///
/// Panics if neither polarity flag is set.
#[must_use]
pub fn tformula_decode_polarity(form: &Term) -> i32 {
    if form.query_prop(TP_POS_POLARITY | TP_NEG_POLARITY) {
        return 0;
    }
    if form.query_prop(TP_POS_POLARITY) {
        return 1;
    }
    if form.query_prop(TP_NEG_POLARITY) {
        return -1;
    }
    panic!("formula has no polarity marker")
}

fn estimate_positive_clauses(bank: &TermBank, form: &Term) -> i64 {
    let sig = bank.signature();
    if form.f_code() == sig.and_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
            if_many(right, || left + right)
        });
    }
    if form.f_code() == sig.or_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
            if_many(right, || left * right)
        });
    }
    if form.f_code() == sig.impl_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
            if_many(right, || left * right)
        });
    }
    if form.f_code() == sig.equiv_code() {
        let pos_left = tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
        return if_many(pos_left, || {
            let pos_right = tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
            if_many(pos_right, || {
                let neg_left = tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
                if_many(neg_left, || {
                    let neg_right =
                        tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
                    if_many(neg_right, || pos_left * neg_right + neg_left * pos_right)
                })
            })
        });
    }
    if form.f_code() == sig.not_code() {
        return tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
    }
    if tformula_is_quantified(bank, form) {
        return tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
    }
    1
}

fn estimate_negative_clauses(bank: &TermBank, form: &Term) -> i64 {
    let sig = bank.signature();
    if form.f_code() == sig.and_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
            if_many(right, || left * right)
        });
    }
    if form.f_code() == sig.or_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
            if_many(right, || left + right)
        });
    }
    if form.f_code() == sig.impl_code() {
        let left = tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
        return if_many(left, || {
            let right = tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
            if_many(right, || left + right)
        });
    }
    if form.f_code() == sig.equiv_code() {
        let pos_left = tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
        return if_many(pos_left, || {
            let pos_right = tformula_estimate_clauses(bank, &formula_argument(form, 1), true);
            if_many(pos_right, || {
                let neg_left = tformula_estimate_clauses(bank, &formula_argument(form, 0), false);
                if_many(neg_left, || {
                    let neg_right =
                        tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
                    if_many(neg_right, || pos_left * pos_right + neg_left * neg_right)
                })
            })
        });
    }
    if form.f_code() == sig.not_code() {
        return tformula_estimate_clauses(bank, &formula_argument(form, 0), true);
    }
    if tformula_is_quantified(bank, form) {
        return tformula_estimate_clauses(bank, &formula_argument(form, 1), false);
    }
    1
}

fn if_many(value: i64, next: impl FnOnce() -> i64) -> i64 {
    if value == TFORM_MANY_CLAUSES {
        TFORM_MANY_CLAUSES
    } else {
        next()
    }
}

/// Transforms a simplified term-encoded formula into negation normal form.
///
/// This matches C `TFormulaNNF` for a single formula. `polarity` must be `-1`,
/// `0`, or `1`; equivalence expansion requires a nonzero polarity just like C.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding or predicate-as-equality encoding fails.
///
/// # Panics
///
/// Panics if `polarity` is outside C's accepted range, if equivalence is
/// expanded with zero polarity, or if formula cells are malformed.
pub fn tformula_nnf(bank: &mut TermBank, form: &Term, polarity: i32) -> Result<Term, Diagnostic> {
    assert!(
        (-1..=1).contains(&polarity),
        "TFormulaNNF polarity must be -1, 0, or 1"
    );

    let mut current = form.clone();
    let mut normal_form = false;
    while !normal_form {
        normal_form = true;
        current = troot_nnf(bank, &current, polarity)?;
        let not_code = bank.signature().not_code();
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();

        if current.f_code() == not_code {
            let child = formula_argument(&current, 0);
            let rewritten = tformula_nnf(bank, &child, -polarity)?;
            if rewritten != child {
                normal_form = false;
                current = tformula_fcode_alloc(bank, not_code, rewritten, None)?;
            }
        } else if current.f_code() == qex_code || current.f_code() == qall_code {
            let body = formula_argument(&current, 1);
            let rewritten = tformula_nnf(bank, &body, polarity)?;
            if rewritten != body {
                normal_form = false;
                current = tformula_fcode_alloc(
                    bank,
                    current.f_code(),
                    formula_argument(&current, 0),
                    Some(rewritten),
                )?;
            }
        } else if current.f_code() == and_code || current.f_code() == or_code {
            let left = formula_argument(&current, 0);
            let right = formula_argument(&current, 1);
            let rewritten_left = tformula_nnf(bank, &left, polarity)?;
            let rewritten_right = tformula_nnf(bank, &right, polarity)?;
            if rewritten_left != left || rewritten_right != right {
                normal_form = false;
                current = tformula_fcode_alloc(
                    bank,
                    current.f_code(),
                    rewritten_left,
                    Some(rewritten_right),
                )?;
            }
        } else if current == *bank.true_term() {
            current = tformula_fcode_alloc(
                bank,
                bank.signature().eqn_code(),
                current.clone(),
                Some(current.clone()),
            )?;
        } else if current == *bank.false_term() {
            current = tformula_fcode_alloc(
                bank,
                bank.signature().neqn_code(),
                current.clone(),
                Some(current.clone()),
            )?;
        } else if current.is_applied_free_var() {
            current = tformula_fcode_alloc(
                bank,
                bank.signature().eqn_code(),
                current,
                Some(bank.true_term().clone()),
            )?;
        } else {
            current = tformula_encode_predicate_as_eqn(bank, current)?;
        }
    }

    Ok(current)
}

fn troot_nnf(bank: &mut TermBank, form: &Term, polarity: i32) -> Result<Term, Diagnostic> {
    assert!(
        (-1..=1).contains(&polarity),
        "troot_nnf polarity must be -1, 0, or 1"
    );

    let mut current = form.clone();
    loop {
        let rewritten = troot_nnf_once(bank, &current, polarity)?;
        if rewritten == current {
            return Ok(current);
        }
        current = rewritten;
    }
}

fn troot_nnf_once(bank: &mut TermBank, form: &Term, polarity: i32) -> Result<Term, Diagnostic> {
    let not_code = bank.signature().not_code();
    let eqn_code = bank.signature().eqn_code();
    let neqn_code = bank.signature().neqn_code();
    let and_code = bank.signature().and_code();
    let or_code = bank.signature().or_code();
    let impl_code = bank.signature().impl_code();
    let equiv_code = bank.signature().equiv_code();
    let qex_code = bank.signature().qex_code();
    let qall_code = bank.signature().qall_code();

    if form.f_code() == not_code {
        let child = formula_argument(form, 0);
        if tformula_is_literal(bank, &child) {
            return tformula_fcode_alloc(
                bank,
                bank.signature().get_other_eqn_code(child.f_code()),
                formula_argument(&child, 0),
                Some(formula_argument(&child, 1)),
            );
        }
        if child == *bank.true_term() {
            return tformula_fcode_alloc(bank, neqn_code, child.clone(), Some(child));
        }
        if child == *bank.false_term() {
            return tformula_fcode_alloc(bank, eqn_code, child.clone(), Some(child));
        }
        if child.f_code() == not_code {
            return Ok(formula_argument(&child, 0));
        }
        if child.f_code() == or_code {
            let left = tformula_fcode_alloc(bank, not_code, formula_argument(&child, 0), None)?;
            let right = tformula_fcode_alloc(bank, not_code, formula_argument(&child, 1), None)?;
            return tformula_fcode_alloc(bank, and_code, left, Some(right));
        }
        if child.f_code() == and_code {
            let left = tformula_fcode_alloc(bank, not_code, formula_argument(&child, 0), None)?;
            let right = tformula_fcode_alloc(bank, not_code, formula_argument(&child, 1), None)?;
            return tformula_fcode_alloc(bank, or_code, left, Some(right));
        }
        if child.f_code() == qall_code {
            let negated_body =
                tformula_fcode_alloc(bank, not_code, formula_argument(&child, 1), None)?;
            return tformula_fcode_alloc(
                bank,
                qex_code,
                formula_argument(&child, 0),
                Some(negated_body),
            );
        }
        if child.f_code() == qex_code {
            let negated_body =
                tformula_fcode_alloc(bank, not_code, formula_argument(&child, 1), None)?;
            return tformula_fcode_alloc(
                bank,
                qall_code,
                formula_argument(&child, 0),
                Some(negated_body),
            );
        }
    } else if form.f_code() == impl_code {
        let negated_left = tformula_fcode_alloc(bank, not_code, formula_argument(form, 0), None)?;
        return tformula_fcode_alloc(bank, or_code, negated_left, Some(formula_argument(form, 1)));
    } else if form.f_code() == equiv_code {
        assert!(
            polarity == 1 || polarity == -1,
            "TFormulaNNF equivalence expansion requires nonzero polarity"
        );
        if polarity == 1 {
            let left_impl = tformula_fcode_alloc(
                bank,
                impl_code,
                formula_argument(form, 0),
                Some(formula_argument(form, 1)),
            )?;
            let right_impl = tformula_fcode_alloc(
                bank,
                impl_code,
                formula_argument(form, 1),
                Some(formula_argument(form, 0)),
            )?;
            return tformula_fcode_alloc(bank, and_code, left_impl, Some(right_impl));
        }
        let not_left = tformula_fcode_alloc(bank, not_code, formula_argument(form, 0), None)?;
        let not_right = tformula_fcode_alloc(bank, not_code, formula_argument(form, 1), None)?;
        let negative_pair = tformula_fcode_alloc(bank, and_code, not_left, Some(not_right))?;
        let positive_pair = tformula_fcode_alloc(
            bank,
            and_code,
            formula_argument(form, 0),
            Some(formula_argument(form, 1)),
        )?;
        return tformula_fcode_alloc(bank, or_code, positive_pair, Some(negative_pair));
    }

    Ok(form.clone())
}

/// Encodes Boolean predicate-like terms as equality or disequality formulas.
///
/// This matches C `EncodePredicateAsEqn`: Boolean variables, non-logical
/// Boolean terms, answers, `$true`, `$false`, `$ite`, `$let`, and phony
/// applications become comparisons with `$true`; `$false` is encoded as
/// `$true != $true`.
///
/// # Errors
///
/// Returns a diagnostic if the encoded equality cannot be allocated.
pub fn tformula_encode_predicate_as_eqn(
    bank: &mut TermBank,
    formula: Term,
) -> Result<Term, Diagnostic> {
    let f_code = formula.f_code();
    let is_encodable = (formula.is_any_var()
        || !bank.signature().is_logical_symbol(f_code)
        || f_code == bank.signature().answer_code()
        || matches!(
            f_code,
            SIG_TRUE_CODE | SIG_FALSE_CODE | SIG_ITE_CODE | SIG_LET_CODE
        )
        || formula.is_phony_app())
        && formula.type_().as_ref().is_some_and(Type::is_bool);
    if !is_encodable {
        return Ok(formula);
    }

    let positive = formula.is_any_var() || f_code != SIG_FALSE_CODE;
    let left = if f_code == SIG_FALSE_CODE && !formula.is_any_var() {
        bank.true_term().clone()
    } else {
        formula
    };
    let right = bank.true_term().clone();
    let eqn_code = bank.signature_mut().get_eqn_code(positive);
    tformula_fcode_alloc(bank, eqn_code, left, Some(right))
}

/// Moves quantifiers inward where possible.
///
/// This matches C `TFormulaMiniScope` for a single term-encoded formula after
/// earlier CNF preprocessing has produced suitable NNF-shaped input.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a moved quantifier or changed connective
/// fails.
///
/// # Panics
///
/// Panics if formula cells are malformed or if the input violates C's
/// precondition that miniscope sees formulas with the expected binary shape.
pub fn tformula_mini_scope(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let (and_code, or_code, qex_code, qall_code) = {
        let sig = bank.signature();
        (
            sig.and_code(),
            sig.or_code(),
            sig.qex_code(),
            sig.qall_code(),
        )
    };

    let mut current = form.clone();
    if tformula_is_quantified(bank, &current) {
        let var = formula_argument(&current, 0);
        let body = formula_argument(&current, 1);
        let op = body.f_code();
        let quantifier = current.f_code();

        if op == and_code || op == or_code {
            let left = formula_argument(&body, 0);
            let right = formula_argument(&body, 1);
            if !tformula_var_is_free(bank, &left, &var) {
                let scoped_right =
                    tformula_fcode_alloc(bank, quantifier, var, Some(right.clone()))?;
                current = tformula_fcode_alloc(bank, op, left, Some(scoped_right))?;
            } else if !tformula_var_is_free(bank, &right, &var) {
                let scoped_left = tformula_fcode_alloc(bank, quantifier, var, Some(left.clone()))?;
                current = tformula_fcode_alloc(bank, op, scoped_left, Some(right))?;
            } else if op == and_code && quantifier == qall_code {
                let scoped_left =
                    tformula_fcode_alloc(bank, qall_code, var.clone(), Some(left.clone()))?;
                let scoped_right = tformula_fcode_alloc(bank, qall_code, var, Some(right))?;
                current = tformula_fcode_alloc(bank, and_code, scoped_left, Some(scoped_right))?;
            } else if op == or_code && quantifier == qex_code {
                let scoped_left =
                    tformula_fcode_alloc(bank, qex_code, var.clone(), Some(left.clone()))?;
                let scoped_right = tformula_fcode_alloc(bank, qex_code, var, Some(right))?;
                current = tformula_fcode_alloc(bank, or_code, scoped_left, Some(scoped_right))?;
            }
        }
    }

    let mut left = formula_argument(&current, 0);
    let mut right = formula_argument(&current, 1);
    let mut modified = false;
    if tformula_has_subform1(bank, &current) {
        let scoped = tformula_mini_scope(bank, &left)?;
        modified = scoped != left;
        left = scoped;
    }
    if tformula_has_subform2(bank, &current) || tformula_is_quantified(bank, &current) {
        let scoped = tformula_mini_scope(bank, &right)?;
        modified |= scoped != right;
        right = scoped;
    }
    if modified {
        let rebuilt = tformula_fcode_alloc(bank, current.f_code(), left, Some(right))?;
        return tformula_mini_scope(bank, &rebuilt);
    }

    Ok(current)
}

/// Conditionally mini-scopes small universal/existential subformulas.
///
/// This matches C `TFormulaMiniScope3`: it first finds maximal subformulas that
/// start with a universal quantifier, contain an existential quantifier, and are
/// within the given size limit. Those subformulas are mini-scoped with
/// `TFormulaMiniScope`, then the original formula is copied while following the
/// selected formula-cell bindings.
///
/// # Errors
///
/// Returns a diagnostic if mini-scoping a selected subformula or rebuilding the
/// copied formula fails.
///
/// # Panics
///
/// Panics if a selected candidate already has a temporary binding.
pub fn tformula_mini_scope3(
    bank: &mut TermBank,
    form: &Term,
    miniscope_limit: i64,
) -> Result<Term, Diagnostic> {
    let mut candidates = BTreeMap::new();
    let scan = tform_find_miniscopeable(bank, form, miniscope_limit, &mut candidates);

    if candidates.is_empty() {
        return Ok(form.clone());
    }
    assert!(
        scan.has_existential,
        "TFormulaMiniScope3 candidates imply an existential quantifier"
    );

    let mut bindings = Vec::with_capacity(candidates.len());
    for candidate in candidates.into_values() {
        assert!(
            candidate.binding().is_none(),
            "TFormulaMiniScope3 candidate must not already be bound"
        );
        let scoped = tformula_mini_scope(bank, &candidate)?;
        bindings.push(BindingRestore::install(candidate, scoped));
    }

    let copied = tform_copy_mod(bank, form)?;
    drop(bindings);
    Ok(copied)
}

struct MiniscopeScan {
    size: i64,
    has_existential: bool,
}

fn tform_find_miniscopeable(
    bank: &TermBank,
    form: &Term,
    limit: i64,
    candidates: &mut BTreeMap<usize, Term>,
) -> MiniscopeScan {
    assert!(
        !form.is_free_var(),
        "tform_find_miniscopeable expects a formula root"
    );

    if form.v_count() == 0 {
        return MiniscopeScan {
            size: i64::MAX,
            has_existential: false,
        };
    }
    if tformula_is_literal(bank, form) || form.type_().as_ref().is_some_and(Type::is_arrow) {
        return MiniscopeScan {
            size: 1,
            has_existential: false,
        };
    }

    let qex_code = bank.signature().qex_code();
    if tformula_is_quantified(bank, form) {
        let mut nested_candidates = BTreeMap::new();
        let body_scan = tform_find_miniscopeable(
            bank,
            &formula_argument(form, 1),
            limit,
            &mut nested_candidates,
        );
        let size = tform_size_add(1, body_scan.size);

        if form.f_code() == qex_code {
            candidates.extend(nested_candidates);
            return MiniscopeScan {
                size,
                has_existential: true,
            };
        }

        if size <= limit && body_scan.has_existential {
            candidates.insert(term_identity_id(form), form.clone());
        } else {
            candidates.extend(nested_candidates);
        }
        return MiniscopeScan {
            size,
            has_existential: body_scan.has_existential,
        };
    }

    let mut size = 1;
    let mut has_existential = false;
    if tformula_has_subform1(bank, form) {
        let scan = tform_find_miniscopeable(bank, &formula_argument(form, 0), limit, candidates);
        size = tform_size_add(size, tform_size_add(size, scan.size));
        has_existential |= scan.has_existential;
    }
    if tformula_has_subform2(bank, form) {
        let scan = tform_find_miniscopeable(bank, &formula_argument(form, 1), limit, candidates);
        size = tform_size_add(size, tform_size_add(size, scan.size));
        has_existential |= scan.has_existential;
    }

    MiniscopeScan {
        size,
        has_existential,
    }
}

fn tform_size_add(left: i64, right: i64) -> i64 {
    left.checked_add(right).unwrap_or(i64::MAX)
}

fn tform_copy_mod(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if tformula_is_literal(bank, form)
        || form.type_().as_ref().is_some_and(Type::is_arrow)
        || form.v_count() == 0
        || form.is_free_var()
    {
        return Ok(form.clone());
    }
    if let Some(binding) = form.binding() {
        return Ok(binding);
    }

    let mut left = None;
    let mut right = None;
    let mut changed = false;

    if tformula_is_quantified(bank, form) {
        left = Some(formula_argument(form, 0));
        let original = formula_argument(form, 1);
        let copied = tform_copy_mod(bank, &original)?;
        changed = copied != original;
        right = Some(copied);
    } else if tformula_has_subform1(bank, form) {
        let original = formula_argument(form, 0);
        let copied = tform_copy_mod(bank, &original)?;
        changed = copied != original;
        left = Some(copied);
    }
    if tformula_has_subform2(bank, form) {
        let original = formula_argument(form, 1);
        let copied = tform_copy_mod(bank, &original)?;
        changed |= copied != original;
        right = Some(copied);
    }

    if changed {
        tformula_fcode_alloc(
            bank,
            form.f_code(),
            left.expect("changed copied formula must have a first argument"),
            right,
        )
    } else {
        Ok(form.clone())
    }
}

/// Replaces every bound variable in a term-encoded formula with a fresh one.
///
/// This matches C `TFormulaVarRename`: quantified variables are temporarily
/// bound to fresh variables, and literal/Boolean term copying follows those
/// bindings with `DEREF_ALWAYS`.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding or copying the formula through the term
/// bank fails.
///
/// # Panics
///
/// Panics if the input violates the C preconditions: no applied free-variable
/// root, well-formed quantified cells, typed quantified variables, and a fresh
/// variable bank state that cannot return the original quantified variable.
pub fn tformula_var_rename(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    assert!(
        !form.is_applied_free_var(),
        "TFormulaVarRename expects no applied free-variable root"
    );

    let quantified = if tformula_is_quantified(bank, form) {
        let quantified_var = formula_argument(form, 0);
        let fresh_type = quantified_var
            .type_()
            .expect("quantified variable must have a type");
        let fresh_var = bank.vars().get_fresh_var(&fresh_type);
        assert_ne!(
            fresh_var, quantified_var,
            "fresh quantified variable must differ from original"
        );
        Some((
            BindingRestore::install(quantified_var, fresh_var.clone()),
            fresh_var,
        ))
    } else {
        None
    };
    let fresh_quantified_var = quantified.as_ref().map(|(_, fresh_var)| fresh_var.clone());

    if matches!(form.f_code(), SIG_LET_CODE | SIG_ITE_CODE) {
        let copy = Term::top_copy_without_args(form);
        for (index, arg) in form.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
            copy.set_argument(index, tformula_var_rename(bank, &arg)?);
        }
        return bank.term_top_insert(copy);
    }

    if tformula_is_literal(bank, form) || form.type_().as_ref().is_some_and(Type::is_arrow) {
        return bank.insert_no_props_cached(form, DerefType::Always);
    }

    let copies_as_bool_term = {
        let sig = bank.signature();
        !sig.is_logical_symbol(form.f_code())
            && form
                .type_()
                .is_some_and(|type_| type_ == sig.type_bank().bool_type())
    };
    if copies_as_bool_term {
        return bank.insert_no_props_cached(form, DerefType::Always);
    }

    if !tformula_is_quantified(bank, form)
        && !tformula_has_subform1(bank, form)
        && !tformula_has_subform2(bank, form)
    {
        return bank.insert_no_props_cached(form, DerefType::Always);
    }

    let mut arg1 = None;
    let mut arg2 = None;
    if tformula_is_quantified(bank, form) {
        arg1 = fresh_quantified_var;
        arg2 = Some(tformula_var_rename(bank, &formula_argument(form, 1))?);
    } else if tformula_has_subform1(bank, form) {
        arg1 = Some(tformula_var_rename(bank, &formula_argument(form, 0))?);
    }
    if tformula_has_subform2(bank, form) {
        arg2 = Some(tformula_var_rename(bank, &formula_argument(form, 1))?);
    }

    tformula_fcode_alloc(
        bank,
        form.f_code(),
        arg1.expect("formula operator must have a first argument"),
        arg2,
    )
}

/// Skolemizes a term-encoded formula as its universal closure.
///
/// This matches C `TFormulaSkolemizeOutermost`: globally free variables seed
/// the Skolem dependency stack, universal variables are pushed while descending,
/// and existential variables are temporarily bound to fresh Skolem terms while
/// the body is copied with `DEREF_ALWAYS`.
///
/// # Errors
///
/// Returns a diagnostic if allocating a Skolem symbol or rebuilding/copying a
/// changed formula fails.
///
/// # Panics
///
/// Panics if the input violates the C preconditions: well-formed quantified
/// cells, free and typed quantified variables, distinct quantified variables,
/// and no pre-existing binding on variables being Skolemized.
pub fn tformula_skolemize_outermost(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let mut free_vars = tformula_collect_free_vars(bank, form);
    tformula_rek_skolemize(bank, form, &mut free_vars)
}

fn tformula_rek_skolemize(
    bank: &mut TermBank,
    form: &Term,
    free_vars: &mut Vec<Term>,
) -> Result<Term, Diagnostic> {
    if term_is_ground(form) {
        return Ok(form.clone());
    }

    if tformula_is_literal(bank, form) || form.type_().as_ref().is_some_and(Type::is_arrow) {
        return bank.insert_no_props_cached(form, DerefType::Always);
    }

    let copies_as_bool_term = {
        let sig = bank.signature();
        !sig.is_logical_symbol(form.f_code())
            && form
                .type_()
                .is_some_and(|type_| type_ == sig.type_bank().bool_type())
    };
    if copies_as_bool_term {
        return bank.insert_no_props_cached(form, DerefType::Always);
    }

    let (qall_code, qex_code) = {
        let sig = bank.signature();
        (sig.qall_code(), sig.qex_code())
    };

    if form.f_code() == qex_code {
        assert_eq!(
            form.arity(),
            2,
            "existential formula must have variable and body arguments"
        );
        let variable = formula_argument(form, 0);
        assert!(
            variable.is_free_var(),
            "existential quantifier must bind a free variable"
        );
        assert!(
            variable.binding().is_none(),
            "existential variable must not already be bound"
        );
        let variable_type = variable
            .type_()
            .expect("existential variable must have a type");
        let skolem = bank.alloc_new_skolem(free_vars.as_slice(), Some(&variable_type))?;
        let _binding = BindingRestore::install(variable, skolem);
        return tformula_rek_skolemize(bank, &formula_argument(form, 1), free_vars);
    }

    if form.f_code() == qall_code {
        assert_eq!(
            form.arity(),
            2,
            "universal formula must have variable and body arguments"
        );
        let variable = formula_argument(form, 0);
        assert!(
            variable.is_free_var(),
            "universal quantifier must bind a free variable"
        );
        assert!(
            variable.binding().is_none(),
            "universal variable must not already be bound"
        );
        free_vars.push(variable.clone());
        let body_result = tformula_rek_skolemize(bank, &formula_argument(form, 1), free_vars);
        let popped = free_vars.pop().expect("universal variable stack underflow");
        assert_eq!(
            popped, variable,
            "universal variable stack must unwind in LIFO order"
        );
        let body = body_result?;
        return tformula_fcode_alloc(bank, qall_code, variable, Some(body));
    }

    assert!(
        tformula_has_subform1(bank, form),
        "compound formula must have a first subformula"
    );
    let original_left = formula_argument(form, 0);
    let left = tformula_rek_skolemize(bank, &original_left, free_vars)?;
    let mut modified = left != original_left;
    let mut right = None;
    if tformula_has_subform2(bank, form) {
        let original_right = formula_argument(form, 1);
        let new_right = tformula_rek_skolemize(bank, &original_right, free_vars)?;
        modified |= new_right != original_right;
        right = Some(new_right);
    }

    if modified {
        tformula_fcode_alloc(bank, form.f_code(), left, right)
    } else {
        Ok(form.clone())
    }
}

/// Collects free variables from a term-encoded formula.
///
/// This matches C `TFormulaCollectFreeVars` for the represented formula shapes:
/// `$let` contributes only its body, DB variables are ignored, and quantifiers
/// plus named lambdas bind their first argument while traversing their body.
/// Unlike C, this staged Rust helper does not mutate `TPIsFreeVar`; it returns
/// variables in term-identity order.
#[must_use]
pub fn tformula_collect_free_vars(bank: &TermBank, form: &Term) -> Vec<Term> {
    let mut vars = BTreeMap::new();
    let mut bound = Vec::new();
    tformula_collect_free_vars_rek(bank, form, &mut bound, &mut vars);
    vars.into_values().collect()
}

fn tformula_collect_free_vars_rek(
    bank: &TermBank,
    form: &Term,
    bound: &mut Vec<usize>,
    vars: &mut BTreeMap<usize, Term>,
) {
    if form.f_code() == SIG_LET_CODE {
        if form.arity() > 0 {
            tformula_collect_free_vars_rek(
                bank,
                &formula_argument(form, form.arity() - 1),
                bound,
                vars,
            );
        }
        return;
    }

    if form.is_db_var() {
        return;
    }

    if tformula_is_quantified(bank, form) && form.arity() == 2 {
        let variable = formula_argument(form, 0);
        let variable_id = term_identity_id(&variable);
        bound.push(variable_id);
        tformula_collect_free_vars_rek(bank, &formula_argument(form, 1), bound, vars);
        let popped = bound.pop().expect("bound variable stack underflow");
        assert_eq!(
            popped, variable_id,
            "bound variable stack must unwind in LIFO order"
        );
        return;
    }

    if form.is_free_var() {
        let variable_id = term_identity_id(form);
        if !bound.contains(&variable_id) {
            vars.insert(variable_id, form.clone());
        }
        return;
    }

    for arg in form.argument_clones().into_iter().flatten() {
        if arg.is_free_var() {
            let variable_id = term_identity_id(&arg);
            if !bound.contains(&variable_id) {
                vars.insert(variable_id, arg);
            }
        } else {
            tformula_collect_free_vars_rek(bank, &arg, bound, vars);
        }
    }
}

/// Shifts universal quantifiers in a term-encoded NNF formula outward.
///
/// This matches C `TFormulaShiftQuantors` for a single formula. The input is
/// expected to satisfy the C preconditions: quantified variables are disjoint,
/// the formula is in negation normal form, and every remaining quantifier is
/// universal.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a shifted connective or quantifier fails.
///
/// # Panics
///
/// Panics if the formula violates the C precondition that shifted quantifiers
/// are universal, or if formula cells are malformed.
pub fn tformula_shift_quantors(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let mut vars = Vec::new();
    let mut shifted = extract_formula_core(bank, form, &mut vars)?;
    while let Some(var) = vars.pop() {
        shifted = tformula_fcode_alloc(bank, bank.signature().qall_code(), var, Some(shifted))?;
    }
    Ok(shifted)
}

/// Shifts all quantifiers in a term-encoded NNF formula outward.
///
/// This matches C `TFormulaShiftQuantors2` for a single formula. Unlike
/// `tformula_shift_quantors`, it preserves both universal and existential
/// quantifier codes.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a shifted connective or quantifier fails.
///
/// # Panics
///
/// Panics if quantified formula cells are malformed.
pub fn tformula_shift_quantors2(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let mut quantifiers = Vec::new();
    let mut shifted = extract_formula_core2(bank, form, &mut quantifiers)?;
    while let Some((quantifier, var)) = quantifiers.pop() {
        shifted = tformula_fcode_alloc(bank, quantifier, var, Some(shifted))?;
    }
    Ok(shifted)
}

/// Distributes disjunction over conjunction in a term-encoded NNF formula.
///
/// This matches C `TFormulaDistributeDisjunctions` for a single suitably
/// preprocessed formula.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding a changed formula or distributed
/// connective fails.
///
/// # Panics
///
/// Panics if the input formula violates the C precondition that it is an NNF
/// formula containing only quantifiers, conjunctions, disjunctions, literals, or
/// Boolean constants, or if required formula arguments are malformed.
pub fn tformula_distribute_disjunctions(
    bank: &mut TermBank,
    form: &Term,
) -> Result<Term, Diagnostic> {
    if form.is_db_var() {
        return Ok(form.clone());
    }

    let (and_code, or_code) = {
        let sig = bank.signature();
        (sig.and_code(), sig.or_code())
    };
    assert!(
        tformula_is_quantified(bank, form)
            || form.f_code() == or_code
            || form.f_code() == and_code
            || tformula_is_literal(bank, form)
            || form == bank.true_term()
            || form == bank.false_term(),
        "TFormulaDistributeDisjunctions expects a preprocessed NNF formula"
    );

    let mut left = None;
    let mut right = None;
    let mut changed = false;
    if tformula_has_subform1(bank, form) {
        let original = formula_argument(form, 0);
        let distributed = tformula_distribute_disjunctions(bank, &original)?;
        changed = distributed != original;
        left = Some(distributed);
    } else if tformula_is_quantified(bank, form) {
        left = Some(formula_argument(form, 0));
    }

    if tformula_has_subform2(bank, form) || tformula_is_quantified(bank, form) {
        let original = formula_argument(form, 1);
        let distributed = tformula_distribute_disjunctions(bank, &original)?;
        changed |= distributed != original;
        right = Some(distributed);
    }

    let mut current = if changed {
        tformula_fcode_alloc(
            bank,
            form.f_code(),
            left.expect("changed formula must have a left argument"),
            right,
        )?
    } else {
        form.clone()
    };

    if current.f_code() == or_code && current.arity() == 2 {
        let left_arg = formula_argument(&current, 0);
        let right_arg = formula_argument(&current, 1);
        if !left_arg.is_db_var() && left_arg.f_code() == and_code {
            let distributed_left = tformula_fcode_alloc(
                bank,
                or_code,
                formula_argument(&left_arg, 0),
                Some(right_arg.clone()),
            )?;
            let distributed_right = tformula_fcode_alloc(
                bank,
                or_code,
                formula_argument(&left_arg, 1),
                Some(right_arg),
            )?;
            let conjunction =
                tformula_fcode_alloc(bank, and_code, distributed_left, Some(distributed_right))?;
            current = tformula_distribute_disjunctions(bank, &conjunction)?;
        } else if !right_arg.is_db_var() && right_arg.f_code() == and_code {
            let distributed_right = tformula_fcode_alloc(
                bank,
                or_code,
                formula_argument(&right_arg, 1),
                Some(left_arg.clone()),
            )?;
            let distributed_left = tformula_fcode_alloc(
                bank,
                or_code,
                formula_argument(&right_arg, 0),
                Some(left_arg),
            )?;
            let conjunction =
                tformula_fcode_alloc(bank, and_code, distributed_left, Some(distributed_right))?;
            current = tformula_distribute_disjunctions(bank, &conjunction)?;
        }
    }

    Ok(current)
}

/// Transforms a term-encoded formula into CNF using C `WTFormulaConjunctiveNF`
/// phase order.
///
/// This is the term-level staged wrapper: it returns the transformed formula
/// and the derivation opcodes that a future `WFormula` owner should attach when
/// the corresponding phase changed the formula.
///
/// # Errors
///
/// Returns a diagnostic if any composed formula transform fails.
///
/// # Panics
///
/// Panics if the input violates the C preconditions of a composed transform.
pub fn tformula_conjunctive_nf(
    bank: &mut TermBank,
    form: &Term,
) -> Result<TFormulaCnfResult, Diagnostic> {
    let mut current = form.clone();
    let mut derivation_ops = Vec::new();
    let mut changed_phases = Vec::new();

    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_FOF_SIMPLIFY,
        |bank, form| tformula_simplify(bank, form, TFORM_CNF_SIMPLIFY_LIMIT),
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_FNNF,
        tformula_nnf_positive,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SHIFT_QUANTORS,
        tformula_mini_scope,
    )?;
    seed_formula_cnf_fresh_vars(bank);
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_VAR_RENAME,
        tformula_var_rename,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SKOLEMIZE,
        tformula_skolemize_outermost,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SHIFT_QUANTORS,
        tformula_shift_quantors,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_DIST_DISJUNCTIONS,
        tformula_distribute_disjunctions,
    )?;

    Ok(TFormulaCnfResult {
        formula: current,
        derivation_ops,
        changed_phases,
    })
}

/// Transforms a term-encoded formula into CNF using C
/// `WTFormulaConjunctiveNF3` phase order.
///
/// This variant uses conditional miniscoping and optionally performs the C
/// FOOL-unrolling step before the final NNF/distribution phases.
///
/// # Errors
///
/// Returns a diagnostic if any composed formula transform fails.
///
/// # Panics
///
/// Panics if `miniscope_limit` is negative, or if the input violates the C
/// preconditions of a composed transform.
pub fn tformula_conjunctive_nf3(
    bank: &mut TermBank,
    form: &Term,
    miniscope_limit: i64,
    fool_unroll: bool,
) -> Result<TFormulaCnfResult, Diagnostic> {
    assert!(
        miniscope_limit >= 0,
        "WTFormulaConjunctiveNF3 expects a nonnegative miniscope limit"
    );

    let mut current = form.clone();
    let mut derivation_ops = Vec::new();
    let mut changed_phases = Vec::new();

    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_FOF_SIMPLIFY,
        |bank, form| tformula_simplify(bank, form, TFORM_CNF_SIMPLIFY_LIMIT),
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_FNNF,
        tformula_nnf_positive,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SHIFT_QUANTORS,
        |bank, form| tformula_mini_scope3(bank, form, miniscope_limit),
    )?;
    seed_formula_cnf_fresh_vars(bank);
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_VAR_RENAME,
        tformula_var_rename,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SKOLEMIZE,
        tformula_skolemize_outermost,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_SHIFT_QUANTORS,
        tformula_shift_quantors,
    )?;

    if fool_unroll {
        apply_cnf_fool_unroll(bank, &mut current, &mut derivation_ops, &mut changed_phases)?;
    }

    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_FNNF,
        tformula_nnf_positive,
    )?;
    apply_cnf_phase(
        bank,
        &mut current,
        &mut derivation_ops,
        &mut changed_phases,
        DC_DIST_DISJUNCTIONS,
        tformula_distribute_disjunctions,
    )?;

    Ok(TFormulaCnfResult {
        formula: current,
        derivation_ops,
        changed_phases,
    })
}

fn apply_cnf_phase(
    bank: &mut TermBank,
    current: &mut Term,
    derivation_ops: &mut Vec<i64>,
    changed_phases: &mut Vec<TFormulaCnfPhase>,
    op: i64,
    transform: impl FnOnce(&mut TermBank, &Term) -> Result<Term, Diagnostic>,
) -> Result<(), Diagnostic> {
    let transformed = transform(bank, current)?;
    if transformed != *current {
        *current = transformed;
        derivation_ops.push(op);
        changed_phases.push(TFormulaCnfPhase {
            op,
            formula: current.clone(),
        });
    }
    Ok(())
}

fn tformula_nnf_positive(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    tformula_nnf(bank, form, 1)
}

fn seed_formula_cnf_fresh_vars(bank: &TermBank) {
    bank.vars().set_v_counts_to_used();
    bank.vars().set_fresh_count_to_used();
}

fn apply_cnf_fool_unroll(
    bank: &mut TermBank,
    current: &mut Term,
    derivation_ops: &mut Vec<i64>,
    changed_phases: &mut Vec<TFormulaCnfPhase>,
) -> Result<(), Diagnostic> {
    let expanded = tformula_expand_literals(bank, current)?;
    let unrolled = do_fool_unroll(bank, &expanded)?;
    if unrolled != expanded {
        derivation_ops.push(DC_FOOL_UNROLL);
        changed_phases.push(TFormulaCnfPhase {
            op: DC_FOOL_UNROLL,
            formula: unrolled.clone(),
        });
    }
    *current = unrolled;
    Ok(())
}

/// Collects a disjunction-shaped term formula into a clause.
///
/// This matches C `TFormulaCollectClause`: top-level disjunctions are flattened
/// with C's stack order, encoded equality/disequality terms become literals,
/// `$true` becomes a true literal, and `$false` is dropped. If `fresh_vars` is
/// present, variables in the collected clause are normalized through that
/// caller-owned variable bank.
///
/// # Errors
///
/// Returns a diagnostic if decoding a literal, allocating a true literal, or
/// normalizing/copying variables through the term bank fails.
///
/// # Panics
///
/// Panics if an encoded literal has malformed arguments.
pub fn tformula_collect_clause(
    bank: &mut TermBank,
    form: &Term,
    fresh_vars: Option<&VarBank>,
) -> Result<Clause, Diagnostic> {
    let or_code = bank.signature().or_code();
    let mut tasks = vec![form.clone()];
    let mut literal_stack = Vec::new();

    while let Some(current) = tasks.pop() {
        if current.f_code() == or_code && current.arity() == 2 {
            tasks.push(formula_argument(&current, 0));
            tasks.push(formula_argument(&current, 1));
            continue;
        }
        if tformula_is_literal(bank, &current) {
            literal_stack.push(Eqn::tb_term_decode(bank, &current)?);
            continue;
        }
        if current == *bank.true_term() {
            literal_stack.push(Eqn::create_true_lit(bank)?);
        }
        // C drops $false and silently ignores any unexpected non-literal leaf.
    }

    let literals = literal_stack.into_iter().rev().collect();
    let mut clause = Clause::alloc(EqnList::from_vec(literals));
    if let Some(fresh_vars) = fresh_vars {
        clause.normalize_vars(bank, fresh_vars)?;
    }
    clause.set_weight(clause.standard_weight());
    Ok(clause)
}

/// Splits a term-encoded CNF formula into variable-normalized clauses.
///
/// This is the staged term-level core of C `TFormulaToCNF`. It skips leading
/// universal quantifiers, splits top-level conjunctions with C's stack order,
/// collects each conjunct as a clause, sets the requested TPTP role, records the
/// `DCSplitConjunct` formula-parent derivation, runs naked Boolean-variable
/// elimination, optionally post-encodes higher-order formula terms, inserts the
/// clauses into `set`, and returns the number inserted.
///
/// Full `DocClauseFromForm` output is deferred until a real `WFormula` owner and
/// proof-document session are available.
///
/// # Errors
///
/// Returns a diagnostic if clause collection, naked Boolean-variable
/// elimination, or higher-order post-CNF encoding fails.
///
/// # Panics
///
/// Panics if the input violates the C preconditions of the collected formula
/// shape or if an encoded literal is malformed.
pub fn tformula_to_cnf(
    bank: &mut TermBank,
    form: &Term,
    type_: FormulaProperties,
    set: &mut ClauseSet,
    fresh_vars: &VarBank,
    source: FormulaDerivationRef,
    problem_type: ProblemType,
) -> Result<i64, Diagnostic> {
    Ok(tformula_to_cnf_impl::<String>(
        bank,
        set,
        TFormulaToCnfInput::new(form, type_, fresh_vars, source, problem_type),
        None,
    )?
    .clauses_generated)
}

/// Splits a term-encoded CNF formula into clauses and emits C
/// `DocClauseFromForm` output for each generated clause.
///
/// This is the proof-documenting counterpart to [`tformula_to_cnf`].
/// The documentation call happens before the `DCSplitConjunct` derivation is
/// pushed, matching C `TFormulaToCNF`.
///
/// # Errors
///
/// Returns a diagnostic if clause collection, proof-documentation rendering,
/// naked Boolean-variable elimination, or higher-order post-CNF encoding fails.
///
/// # Panics
///
/// Panics if the input violates the C preconditions of the collected formula
/// shape or if an encoded literal is malformed.
pub fn tformula_to_cnf_with_docs<W: fmt::Write>(
    doc_context: TFormulaToCnfDocContext<'_, '_, W>,
    bank: &mut TermBank,
    set: &mut ClauseSet,
    input: TFormulaToCnfInput<'_>,
) -> Result<TFormulaToCnfDocResult, Diagnostic> {
    tformula_to_cnf_impl(bank, set, input, Some(doc_context))
}

fn tformula_to_cnf_impl<W: fmt::Write>(
    bank: &mut TermBank,
    set: &mut ClauseSet,
    input: TFormulaToCnfInput<'_>,
    mut doc_context: Option<TFormulaToCnfDocContext<'_, '_, W>>,
) -> Result<TFormulaToCnfDocResult, Diagnostic> {
    let old_clause_number = set.members();
    let mut result = TFormulaToCnfDocResult::default();
    let qall_code = bank.signature().qall_code();
    let and_code = bank.signature().and_code();

    let mut handle = input.form.clone();
    while handle.f_code() == qall_code {
        handle = formula_argument(&handle, 1);
    }

    let mut tasks = vec![handle];
    while let Some(current) = tasks.pop() {
        if current.f_code() == and_code && current.arity() == 2 {
            tasks.push(formula_argument(&current, 0));
            tasks.push(formula_argument(&current, 1));
            continue;
        }

        let mut clause = tformula_collect_clause(bank, &current, Some(input.fresh_vars))?;
        clause.set_tptp_type(input.type_);
        if let Some(doc) = doc_context.as_mut() {
            result.write_results.push(doc.session.doc_clause_from_form(
                &mut *doc.output,
                bank,
                &mut clause,
                doc.parent,
            )?);
        }
        clause_push_formula_derivation(&mut clause, DC_SPLIT_CONJUNCT, Some(input.source), None);

        if clause_eliminate_naked_boolean_variables(&mut clause, bank)? {
            clause_push_derivation(&mut clause, DC_ELIMINATE_BVAR, None, None);
        }

        if input.problem_type == ProblemType::HigherOrder {
            post_cnf_encode_clause_terms(bank, &mut clause)?;
        }

        set.insert(clause);
    }

    result.clauses_generated = set.members() - old_clause_number;
    Ok(result)
}

/// Applies C `PostCNFEncodeFormulas` to both sides of each clause literal.
///
/// This is used after CNF extraction in higher-order mode and by the staged
/// `WFormulaCNF2` clause-wrapper shortcut.
///
/// # Errors
///
/// Returns a diagnostic if encoding any literal side fails.
///
/// # Panics
///
/// Panics if a literal mapper violates the term-bank sharing preconditions.
pub fn post_cnf_encode_clause_terms(
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<(), Diagnostic> {
    for literal in clause.literals_mut().as_mut_slice() {
        let old_left = literal.left().clone();
        let old_right = literal.right().clone();
        let new_left = post_cnf_encode_formulas(bank, &old_left)?;
        let new_right = post_cnf_encode_formulas(bank, &old_right)?;
        literal.map_terms(bank, |term| {
            if term == &old_left {
                new_left.clone()
            } else {
                new_right.clone()
            }
        });
    }
    clause.recompute_lit_counts();
    clause.set_weight(clause.standard_weight());
    Ok(())
}

fn extract_formula_core(
    bank: &mut TermBank,
    form: &Term,
    vars: &mut Vec<Term>,
) -> Result<Term, Diagnostic> {
    let (qall_code, qex_code, and_code, or_code) = {
        let sig = bank.signature();
        (
            sig.qall_code(),
            sig.qex_code(),
            sig.and_code(),
            sig.or_code(),
        )
    };

    let mut current = form.clone();
    while current.f_code() == qall_code || current.f_code() == qex_code {
        assert_eq!(
            current.f_code(),
            qall_code,
            "TFormulaShiftQuantors expects only universal quantifiers"
        );
        assert_eq!(
            current.arity(),
            2,
            "quantified formula cell must have variable and body arguments"
        );
        vars.push(formula_argument(&current, 0));
        current = formula_argument(&current, 1);
    }

    if current.arity() == 2 && (current.f_code() == and_code || current.f_code() == or_code) {
        let stack_len = vars.len();
        let left = formula_argument(&current, 0);
        let right = formula_argument(&current, 1);
        let shifted_left = extract_formula_core(bank, &left, vars)?;
        let shifted_right = extract_formula_core(bank, &right, vars)?;
        if vars.len() != stack_len {
            return tformula_fcode_alloc(bank, current.f_code(), shifted_left, Some(shifted_right));
        }
        assert_eq!(
            shifted_left, left,
            "left formula changed without shifted vars"
        );
        assert_eq!(
            shifted_right, right,
            "right formula changed without shifted vars"
        );
    }

    Ok(current)
}

/// Returns whether a term-encoded formula has a first formula subterm.
///
/// This matches C `TFormulaHasSubForm1`: the root symbol must carry
/// `FPFOFOp`, and the cell arity must be at least one.
#[must_use]
pub fn tformula_has_subform1(bank: &TermBank, form: &Term) -> bool {
    bank.signature().query_prop(form.f_code(), FP_FOF_OP) && form.arity() >= 1
}

/// Returns whether a term-encoded formula has a second formula subterm.
///
/// This matches C `TFormulaHasSubForm2`: the root symbol must carry
/// `FPFOFOp`, and the cell arity must be at least two.
#[must_use]
pub fn tformula_has_subform2(bank: &TermBank, form: &Term) -> bool {
    bank.signature().query_prop(form.f_code(), FP_FOF_OP) && form.arity() >= 2
}

/// Returns whether a term-encoded formula has binary arity.
///
/// This matches C `TFormulaIsBinary`.
#[must_use]
pub fn tformula_is_binary(form: &Term) -> bool {
    form.arity() == 2
}

/// Returns whether a term-encoded formula has unary arity.
///
/// This matches C `TFormulaIsUnary`.
#[must_use]
pub fn tformula_is_unary(form: &Term) -> bool {
    form.arity() == 1
}

/// Returns whether a term-encoded formula is a first-order quantifier cell.
///
/// This matches C `TFormulaIsQuantifiedNL`, which excludes DB variables and
/// recognizes only `$qex` and `$qall`, not named lambda.
#[must_use]
pub fn tformula_is_quantified_nl(bank: &TermBank, form: &Term) -> bool {
    !form.is_db_var()
        && matches!(form.f_code(), code if code == bank.signature().qex_code()
            || code == bank.signature().qall_code())
}

/// Returns whether a term-encoded formula is a quantifier-like cell.
///
/// This matches C `TFormulaIsQuantified`, including the named-lambda f-code.
#[must_use]
pub fn tformula_is_quantified(bank: &TermBank, form: &Term) -> bool {
    !form.is_db_var()
        && matches!(form.f_code(), code if code == bank.signature().qex_code()
            || code == bank.signature().qall_code()
            || code == SIG_NAMED_LAMBDA_CODE)
}

/// Returns whether a term-encoded formula is an encoded equality literal.
///
/// This matches C `TFormulaIsLiteral`.
#[must_use]
pub fn tformula_is_literal(bank: &TermBank, form: &Term) -> bool {
    matches!(form.f_code(), code if code == bank.signature().eqn_code()
        || code == bank.signature().neqn_code())
        && form.arity() == 2
}

/// Returns whether a term-encoded formula is C's macro-level complex Boolean.
///
/// This mirrors the literal C `TFormulaIsComplexBool` macro. That macro passes
/// the term cell itself to `TypeIsBool`, so compatibility is the term f-code
/// check against `STBool`, not `form.type == $o`.
#[must_use]
pub fn tformula_is_complex_bool(bank: &TermBank, form: &Term) -> bool {
    !form.is_any_var()
        && bank.signature().is_logical_symbol(form.f_code())
        && form.f_code() == ST_BOOL
}

struct BindingRestore {
    variable: Term,
    old_binding: Option<Term>,
}

impl BindingRestore {
    fn install(variable: Term, new_binding: Term) -> Self {
        let old_binding = variable.binding();
        variable.set_binding(Some(new_binding));
        Self {
            variable,
            old_binding,
        }
    }
}

impl Drop for BindingRestore {
    fn drop(&mut self) {
        self.variable.set_binding(self.old_binding.clone());
    }
}

fn extract_formula_core2(
    bank: &mut TermBank,
    form: &Term,
    quantifiers: &mut Vec<(i64, Term)>,
) -> Result<Term, Diagnostic> {
    let (qall_code, qex_code, and_code, or_code) = {
        let sig = bank.signature();
        (
            sig.qall_code(),
            sig.qex_code(),
            sig.and_code(),
            sig.or_code(),
        )
    };

    let mut current = form.clone();
    while current.f_code() == qall_code || current.f_code() == qex_code {
        assert_eq!(
            current.arity(),
            2,
            "quantified formula cell must have variable and body arguments"
        );
        quantifiers.push((current.f_code(), formula_argument(&current, 0)));
        current = formula_argument(&current, 1);
    }

    if current.arity() == 2 && (current.f_code() == and_code || current.f_code() == or_code) {
        let stack_len = quantifiers.len();
        let left = formula_argument(&current, 0);
        let right = formula_argument(&current, 1);
        let shifted_left = extract_formula_core2(bank, &left, quantifiers)?;
        let shifted_right = extract_formula_core2(bank, &right, quantifiers)?;
        if quantifiers.len() != stack_len {
            return tformula_fcode_alloc(bank, current.f_code(), shifted_left, Some(shifted_right));
        }
        assert_eq!(
            shifted_left, left,
            "left formula changed without shifted quantifiers"
        );
        assert_eq!(
            shifted_right, right,
            "right formula changed without shifted quantifiers"
        );
    }

    Ok(current)
}

fn unroll_binary_formula(formula: &Term, f_code: i64, args: &mut Vec<Term>) {
    let mut tasks = vec![formula.clone()];
    while let Some(task) = tasks.pop() {
        if !task.is_db_var() && task.arity() == 2 && task.f_code() == f_code {
            tasks.push(formula_argument(&task, 1));
            tasks.push(formula_argument(&task, 0));
        } else {
            args.push(task);
        }
    }
}

fn fold_and_or(bank: &mut TermBank, mut args: Vec<Term>, f_code: i64) -> Result<Term, Diagnostic> {
    if args.len() == 1 {
        let Some(term) = args.pop() else {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "fold_and_or expected a single formula argument",
            ));
        };
        return Ok(term);
    }

    let Some(mut left) = args.pop() else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "fold_and_or expected a left formula argument",
        ));
    };
    let Some(right) = args.pop() else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "fold_and_or expected a right formula argument",
        ));
    };
    left = tformula_fcode_alloc(bank, f_code, left, Some(right))?;
    while let Some(right) = args.pop() {
        left = tformula_fcode_alloc(bank, f_code, left, Some(right))?;
    }
    Ok(left)
}

struct DedupedTerms {
    terms: Vec<Term>,
    removed_duplicate: bool,
}

fn dedup_sorted_terms(terms: Vec<Term>) -> DedupedTerms {
    let mut deduped = Vec::with_capacity(terms.len());
    let mut removed_duplicate = false;
    for term in terms {
        if deduped.last().is_some_and(|last| last == &term) {
            removed_duplicate = true;
        } else {
            deduped.push(term);
        }
    }
    DedupedTerms {
        terms: deduped,
        removed_duplicate,
    }
}

fn contains_decoded_complement(bank: &mut TermBank, terms: &[Term]) -> Result<bool, Diagnostic> {
    for term in terms {
        let negated = negate_decoded_formula(bank, term)?;
        let key = term_identity_id(&negated);
        if terms.binary_search_by_key(&key, term_identity_id).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn formula_argument(formula: &Term, index: usize) -> Term {
    formula
        .argument(index)
        .unwrap_or_else(|| panic!("formula argument {index} is uninitialized"))
}

/// Applies C `ClausePruneArgs`.
///
/// The pass removes arguments from applied free variables when the argument is
/// constant across all occurrences or repeated at another argument position.
///
/// # Errors
///
/// Returns a diagnostic if generated lambda bindings or rebuilt terms cannot be
/// inserted through the term bank.
///
/// # Panics
///
/// Panics if a candidate higher-order variable is untyped or if an applied
/// variable carries more explicit arguments than its type permits.
pub fn clause_prune_args(clause: &mut Clause, bank: &mut TermBank) -> Result<bool, Diagnostic> {
    if clause.is_empty() {
        return Ok(false);
    }

    let mut var_data = BTreeMap::new();
    for literal in clause.literals().as_slice() {
        collect_prune_arg_occurrences(literal.left(), &mut var_data);
        collect_prune_arg_occurrences(literal.right(), &mut var_data);
    }

    remove_constant_args(&mut var_data);
    remove_repeated_args(&mut var_data);

    let mut substitution = Substitution::new();
    let result = (|| {
        if !compute_prune_arg_substitution(&var_data, bank, &mut substitution)? {
            return Ok(false);
        }
        apply_prune_arg_substitution(clause, bank)?;
        Ok(true)
    })();
    substitution.delete();
    result
}

#[derive(Clone, Debug)]
struct PruneArgVarData {
    var: Term,
    occurrences: Vec<Vec<Option<Term>>>,
    removed_args: BTreeSet<usize>,
}

fn collect_prune_arg_occurrences(term: &Term, vars: &mut BTreeMap<usize, PruneArgVarData>) {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if let Some((var, args)) = prune_arg_candidate(&current) {
            let key = term_identity_id(&var);
            vars.entry(key)
                .or_insert_with(|| PruneArgVarData {
                    var,
                    occurrences: Vec::new(),
                    removed_args: BTreeSet::new(),
                })
                .occurrences
                .push(args);
        }

        for index in usize::from(current.is_phony_app())..current.arity() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            if !term_is_ground(&arg) {
                stack.push(arg);
            }
        }
    }
}

fn prune_arg_candidate(term: &Term) -> Option<(Term, Vec<Option<Term>>)> {
    let var = if term.is_applied_free_var() {
        term.argument(0)
            .unwrap_or_else(|| panic!("applied free variable head is uninitialized"))
    } else if term.is_free_var() && term.type_().is_some_and(|type_| type_.is_arrow()) {
        term.clone()
    } else {
        return None;
    };

    let var_type = var.type_().expect("higher-order variable must have a type");
    let max_args = type_get_max_arity(&var_type);
    let explicit_args = term.arity().saturating_sub(1);
    assert!(
        explicit_args <= max_args,
        "applied variable has more arguments than its type permits"
    );

    let mut args = vec![None; max_args];
    for index in 1..term.arity() {
        args[index - 1] = Some(
            term.argument(index)
                .unwrap_or_else(|| panic!("applied variable argument {index} is uninitialized")),
        );
    }
    Some((var, args))
}

fn remove_constant_args(vars: &mut BTreeMap<usize, PruneArgVarData>) {
    for data in vars.values_mut() {
        let Some(first_occurrence) = data.occurrences.first() else {
            continue;
        };
        let mut arg_idx = 0;
        while arg_idx < first_occurrence.len() {
            let Some(first_arg) = first_occurrence[arg_idx].as_ref() else {
                break;
            };

            let removable = term_is_db_closed(first_arg)
                && !data.removed_args.contains(&arg_idx)
                && data.occurrences[1..].iter().all(|occurrence| {
                    occurrence
                        .get(arg_idx)
                        .and_then(Option::as_ref)
                        .is_some_and(|arg| arg == first_arg)
                });
            if removable {
                data.removed_args.insert(arg_idx);
            }
            arg_idx += 1;
        }
    }
}

fn remove_repeated_args(vars: &mut BTreeMap<usize, PruneArgVarData>) {
    for data in vars.values_mut() {
        let Some(first_occurrence) = data.occurrences.first() else {
            continue;
        };
        let num_args = first_occurrence.len();
        let mut arg_i = 0;
        while arg_i < num_args {
            if first_occurrence[arg_i].is_none() {
                break;
            }

            let mut arg_j = arg_i + 1;
            while arg_j < num_args {
                if first_occurrence[arg_j].is_none() {
                    break;
                }
                let removable = !data.removed_args.contains(&arg_i)
                    && !data.removed_args.contains(&arg_j)
                    && data.occurrences.iter().all(|occurrence| {
                        let Some(left) = occurrence.get(arg_i).and_then(Option::as_ref) else {
                            return false;
                        };
                        occurrence
                            .get(arg_j)
                            .and_then(Option::as_ref)
                            .is_some_and(|right| right == left)
                    });
                if removable {
                    data.removed_args.insert(arg_i);
                    break;
                }
                arg_j += 1;
            }
            arg_i += 1;
        }
    }
}

fn compute_prune_arg_substitution(
    vars: &BTreeMap<usize, PruneArgVarData>,
    bank: &mut TermBank,
    substitution: &mut Substitution,
) -> Result<bool, Diagnostic> {
    let mut removed_any = false;
    for data in vars.values() {
        if data.removed_args.is_empty() {
            continue;
        }

        let var_type = data
            .var
            .type_()
            .expect("higher-order variable must have a type");
        assert!(
            var_type.is_arrow(),
            "argument pruning expects an arrow-typed variable"
        );
        let max_args = type_get_max_arity(&var_type);
        let ret_type = var_type.args()[var_type.arity() - 1].clone();
        let mut retained_db_vars = Vec::new();
        let mut retained_types = Vec::new();
        for arg_idx in 0..max_args {
            if data.removed_args.contains(&arg_idx) {
                continue;
            }
            let arg_type = var_type.args()[arg_idx].clone();
            retained_types.push(arg_type.clone());
            retained_db_vars
                .push(bank.request_db_var(&arg_type, usize_to_i64(max_args - arg_idx - 1)));
        }

        let fresh_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(arrow_type_flattened(&retained_types, &ret_type));
        let fresh_var = bank.vars().get_fresh_var(&fresh_type);
        let matrix = apply_terms(bank, &fresh_var, &retained_db_vars)?;
        let closed = close_with_type_prefix(bank, &var_type.args()[..max_args], &matrix)?;
        substitution.add_binding(&data.var, &closed);
        removed_any = true;
    }
    Ok(removed_any)
}

fn apply_prune_arg_substitution(
    clause: &mut Clause,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    for literal in clause.literals_mut().as_mut_slice() {
        let left = bank.insert_instantiated_ho(literal.left(), true)?;
        let left = beta_normalize_db(bank, &left)?;
        let right = bank.insert_instantiated_ho(literal.right(), true)?;
        let right = beta_normalize_db(bank, &right)?;
        literal.set_left_raw(left);
        literal.set_right_raw(right);
    }

    let _ = clause.literals_mut().remove_resolved(bank);
    let _ = clause.literals_mut().remove_duplicates(bank);
    clause.recompute_lit_counts();
    clause.set_weight(clause.standard_weight());
    clause_push_derivation(clause, DC_PRUNE_ARG, None, None);
    Ok(())
}

/// Recognizes an injectivity definition and creates the inverse-function clause.
///
/// This mirrors C `ClauseRecognizeInjectivity`, including the `DCInvRec`
/// derivation parent. Full proof-document output for this clause kind is still
/// intentionally absent in C.
///
/// # Errors
///
/// Returns a diagnostic if typed Skolem creation, term-bank insertion, or
/// equation allocation fails.
///
/// # Panics
///
/// Panics if a syntactically accepted candidate has uninitialized term
/// arguments or non-variable argument pairs where the C code asserts.
pub fn clause_recognize_injectivity(
    bank: &mut TermBank,
    clause: &Clause,
) -> Result<Option<Clause>, Diagnostic> {
    if clause.positive_literal_count() != 1 || clause.negative_literal_count() != 1 {
        return Ok(None);
    }

    let (pos_lit, neg_lit) = split_injectivity_literals(clause);
    if !pos_lit.is_equ_lit(bank)
        || !neg_lit.is_equ_lit(bank)
        || !pos_lit.left().is_free_var()
        || !pos_lit.right().is_free_var()
        || pos_lit.left() == pos_lit.right()
        || neg_lit.left().is_top_level_any_var()
        || neg_lit.right().is_top_level_any_var()
        || neg_lit.left().f_code() != neg_lit.right().f_code()
        || neg_lit.left().f_code() <= bank.signature().internal_symbols()
        || neg_lit.left().type_().is_none_or(|type_| type_.is_arrow())
        || bank
            .signature()
            .query_prop(neg_lit.left().f_code(), FP_IS_INJ_DEF_SKOLEM)
        || neg_lit.left().arity() == 0
        || neg_lit.left().arity() != neg_lit.right().arity()
    {
        return Ok(None);
    }

    let arity = neg_lit.left().arity();
    let var_tuple_weight = DEFAULT_FWEIGHT + usize_to_i64(arity) * DEFAULT_VWEIGHT;
    if term_standard_weight(neg_lit.left()) != term_standard_weight(neg_lit.right())
        || term_standard_weight(neg_lit.left()) != var_tuple_weight
    {
        return Ok(None);
    }

    let Some(index) = injectivity_variable_index(pos_lit, neg_lit) else {
        return Ok(None);
    };
    let Some(skolem_vars) = collect_injectivity_skolem_vars(neg_lit) else {
        return Ok(None);
    };

    build_injectivity_inverse_clause(bank, clause, neg_lit, index, skolem_vars).map(Some)
}

/// Checks whether an inverse-function definition is already represented modulo
/// variable renaming in `all_defs`.
///
/// # Errors
///
/// Returns a diagnostic if copying the generated definition into the current
/// term bank with disjoint variables fails.
///
/// # Panics
///
/// Panics if `inj_def` or a candidate definition is not a positive unit clause,
/// matching the C assertions.
pub fn clause_set_injectivity_is_defined(
    all_defs: &ClauseSet,
    inj_def: &Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    assert_eq!(inj_def.positive_literal_count(), 1);
    assert_eq!(inj_def.negative_literal_count(), 0);

    let inj_literal = &inj_def.literals().as_slice()[0];
    let lhs = bank.insert_disjoint(inj_literal.left())?;
    let rhs = bank.insert_disjoint(inj_literal.right())?;

    for candidate in all_defs.iter() {
        assert_eq!(candidate.positive_literal_count(), 1);
        assert_eq!(candidate.negative_literal_count(), 0);

        let cand_literal = &candidate.literals().as_slice()[0];
        let cand_lhs = cand_literal.left();
        if cand_lhs.arity() != lhs.arity() {
            continue;
        }

        let mut pairs = Vec::with_capacity(2 + 2 * lhs.arity());
        pairs.push(rhs.clone());
        pairs.push(cand_literal.right().clone());
        for index in 0..cand_lhs.arity() {
            pairs.push(required_arg(&lhs, index));
            pairs.push(required_arg(cand_lhs, index));
        }

        let mut subst = Substitution::new();
        let is_defined = unif_all_pairs(&mut pairs, &mut subst) && subst.is_renaming();
        subst.delete();
        if is_defined {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Replaces recognized injectivity definitions by inverse-function clauses.
///
/// The originals that produce a new definition are moved to `archive`; duplicate
/// recognized definitions keep their original clause in `set`, matching C
/// `ClauseSetReplaceInjectivityDefs`.
///
/// # Errors
///
/// Returns a diagnostic if recognition, duplicate checking, or generated term
/// construction fails.
///
/// # Panics
///
/// Panics under the same internal candidate-shape invariants as
/// [`clause_recognize_injectivity`] and [`clause_set_injectivity_is_defined`].
pub fn clause_set_replace_injectivity_defs(
    set: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let ids = set.iter().map(Clause::ident).collect::<Vec<_>>();
    let mut replacements = ClauseSet::new();
    let mut archived_ids = Vec::new();
    let mut count = 0;

    for id in ids {
        let Some(clause) = set.find_by_id(id) else {
            continue;
        };
        let Some(replacement) = clause_recognize_injectivity(bank, clause)? else {
            continue;
        };
        if replacement.query_prop(CP_IS_PURE_INJECTIVITY)
            && !clause_set_injectivity_is_defined(&replacements, &replacement, bank)?
        {
            archived_ids.push(id);
            replacements.insert(replacement);
            count += 1;
        }
    }

    for id in archived_ids {
        if let Some(clause) = set.extract_by_id(id) {
            archive.insert(clause);
        }
    }
    set.insert_set(&mut replacements);
    Ok(count)
}

/// Recognizes a C defined-choice axiom and records its choice symbol.
///
/// This mirrors the represented `ClauseRecognizeChoice` path for already
/// beta/eta-normal clauses of shape `~P X | P (choice P)`. Full eta reduction
/// remains tied to the broader lambda-normalization port.
///
/// # Errors
///
/// Returns diagnostics from beta normalization.
pub fn clause_recognize_choice(
    bank: &mut TermBank,
    clause: &mut Clause,
    choice_symbols: &BTreeMap<i64, Clause>,
) -> Result<Option<i64>, Diagnostic> {
    let Some(candidate) = clause_choice_candidate(bank, clause, choice_symbols)? else {
        return Ok(None);
    };

    let literals = clause.literals_mut().as_mut_slice();
    literals[candidate.negative_index].set_left_raw(candidate.negative_term);
    literals[candidate.positive_index].set_left_raw(candidate.positive_term);
    Ok(Some(candidate.choice_code))
}

/// Tests whether a clause is a represented C defined-choice axiom without
/// recording the choice symbol or mutating the clause.
///
/// This mirrors `ClauseRecognizeChoice(NULL, cl)`: duplicate choice-symbol
/// checks and normalized-literal replacement are skipped when no side map is
/// supplied.
///
/// # Errors
///
/// Returns diagnostics from beta normalization.
pub fn clause_recognizes_choice(bank: &mut TermBank, clause: &Clause) -> Result<bool, Diagnostic> {
    clause_choice_candidate(bank, clause, &BTreeMap::new()).map(|candidate| candidate.is_some())
}

/// Recognizes all represented choice axioms in `set`.
///
/// The C helper stores pointers to clauses that remain in the source set,
/// despite a stale comment saying they are moved to the archive. Rust stores
/// owned clause copies until proof-state clause handles are stable enough to
/// represent the pointer map directly.
///
/// # Errors
///
/// Returns diagnostics from [`clause_recognize_choice`].
pub fn clause_set_recognize_choice(
    bank: &mut TermBank,
    set: &mut ClauseSet,
    choice_symbols: &mut BTreeMap<i64, Clause>,
) -> Result<i64, Diagnostic> {
    let mut recognized = 0;
    for clause in set.iter_mut() {
        let Some(choice_code) = clause_recognize_choice(bank, clause, choice_symbols)? else {
            continue;
        };
        choice_symbols.insert(choice_code, clause.clone());
        recognized += 1;
    }
    Ok(recognized)
}

struct ChoiceCandidate {
    choice_code: i64,
    positive_index: usize,
    positive_term: Term,
    negative_index: usize,
    negative_term: Term,
}

fn clause_choice_candidate(
    bank: &mut TermBank,
    clause: &Clause,
    choice_symbols: &BTreeMap<i64, Clause>,
) -> Result<Option<ChoiceCandidate>, Diagnostic> {
    if clause.positive_literal_count() != 1 || clause.negative_literal_count() != 1 {
        return Ok(None);
    }

    let Some((positive_index, negative_index)) = choice_literal_indices(clause) else {
        return Ok(None);
    };
    let positive_literal = &clause.literals().as_slice()[positive_index];
    let negative_literal = &clause.literals().as_slice()[negative_index];
    if positive_literal.is_equ_lit(bank) || negative_literal.is_equ_lit(bank) {
        return Ok(None);
    }

    let negative_term = beta_normalize_db(bank, negative_literal.left())?;
    let positive_term = beta_normalize_db(bank, positive_literal.left())?;
    if !negative_term.is_applied_free_var()
        || !positive_term.is_applied_free_var()
        || negative_term.arity() != 2
        || positive_term.arity() != 2
    {
        return Ok(None);
    }

    let Some(negative_arg) = negative_term.argument(1) else {
        return Ok(None);
    };
    if !negative_arg.is_free_var() {
        return Ok(None);
    }
    let Some(predicate_var) = negative_term.argument(0) else {
        return Ok(None);
    };
    if positive_term.argument(0) != Some(predicate_var.clone()) {
        return Ok(None);
    }
    let Some(choice_application) = positive_term.argument(1) else {
        return Ok(None);
    };
    if choice_application.arity() != 1
        || choice_application.f_code() <= bank.signature().internal_symbols()
        || choice_application.argument(0) != Some(predicate_var)
        || choice_symbols.contains_key(&choice_application.f_code())
    {
        return Ok(None);
    }

    Ok(Some(ChoiceCandidate {
        choice_code: choice_application.f_code(),
        positive_index,
        positive_term,
        negative_index,
        negative_term,
    }))
}

fn choice_literal_indices(clause: &Clause) -> Option<(usize, usize)> {
    let mut positive = None;
    let mut negative = None;
    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_positive() {
            positive = Some(index);
        } else {
            negative = Some(index);
        }
    }
    Some((positive?, negative?))
}

#[must_use]
pub fn clause_canon_compare_ref(left: &Clause, right: &Clause, bank: &TermBank) -> i32 {
    left.cmp_by_struct_weight(right, bank)
}

fn split_injectivity_literals(clause: &Clause) -> (&Eqn, &Eqn) {
    let first = &clause.literals().as_slice()[0];
    let second = &clause.literals().as_slice()[1];
    if first.is_positive() {
        (first, second)
    } else {
        (second, first)
    }
}

fn injectivity_variable_index(pos_lit: &Eqn, neg_lit: &Eqn) -> Option<usize> {
    for index in 0..neg_lit.left().arity() {
        let left_arg = required_arg(neg_lit.left(), index);
        let right_arg = required_arg(neg_lit.right(), index);
        if (&left_arg == pos_lit.left() && &right_arg == pos_lit.right())
            || (&left_arg == pos_lit.right() && &right_arg == pos_lit.left())
        {
            return Some(index);
        }
    }
    None
}

fn collect_injectivity_skolem_vars(neg_lit: &Eqn) -> Option<Vec<Term>> {
    clear_injectivity_marks(neg_lit);
    let mut skolem_vars = Vec::new();
    let mut applicable = true;

    for index in 0..neg_lit.left().arity() {
        let left_var = required_arg(neg_lit.left(), index);
        let right_var = required_arg(neg_lit.right(), index);
        assert!(left_var.is_free_var());
        assert!(right_var.is_free_var());

        if left_var == right_var {
            if left_var.query_prop(TP_CHECK_FLAG) || right_var.query_prop(TP_CHECK_FLAG) {
                applicable = false;
                break;
            }
            if !left_var.query_prop(TP_OP_FLAG) {
                left_var.set_prop(TP_OP_FLAG);
                skolem_vars.push(left_var);
            }
        } else if left_var.is_any_prop_set(TP_CHECK_FLAG | TP_OP_FLAG)
            || right_var.is_any_prop_set(TP_CHECK_FLAG | TP_OP_FLAG)
        {
            applicable = false;
            break;
        } else {
            left_var.set_prop(TP_CHECK_FLAG);
            right_var.set_prop(TP_CHECK_FLAG);
        }
    }

    clear_injectivity_marks(neg_lit);
    applicable.then_some(skolem_vars)
}

fn clear_injectivity_marks(neg_lit: &Eqn) {
    let flags = TP_OP_FLAG | TP_CHECK_FLAG;
    term_del_prop(neg_lit.left(), DerefType::Never, flags);
    term_del_prop(neg_lit.right(), DerefType::Never, flags);
}

fn build_injectivity_inverse_clause(
    bank: &mut TermBank,
    source: &Clause,
    neg_lit: &Eqn,
    index: usize,
    skolem_vars: Vec<Term>,
) -> Result<Clause, Diagnostic> {
    let inverse_arg = neg_lit.left().clone();
    let inverse_var = required_arg(neg_lit.left(), index);
    let ret_type = inverse_var
        .type_()
        .expect("injectivity inverse variable has a type");
    let mut args = skolem_vars;
    args.push(inverse_arg);
    let arg_types = args
        .iter()
        .map(|arg| {
            arg.type_()
                .expect("injectivity inverse argument has a type")
        })
        .collect::<Vec<_>>();

    let inverse_code = bank
        .signature_mut()
        .get_new_typed_skolem(&arg_types, &ret_type)?;
    bank.signature_mut()
        .set_func_prop(inverse_code, FP_IS_INJ_DEF_SKOLEM);

    let inverse_term = Term::top_alloc(inverse_code, args.len());
    for (arg_index, arg) in args.into_iter().enumerate() {
        inverse_term.set_argument(arg_index, arg);
    }
    inverse_term.set_type(Some(ret_type));
    let inverse_term = bank.term_top_insert(inverse_term)?;
    let equation = Eqn::alloc(inverse_term, inverse_var, bank, true)?;
    let mut result = Clause::alloc(EqnList::from_vec(vec![equation]));
    result.set_proof_depth(source.proof_depth() + 1);
    result.set_proof_size(source.proof_size() + 1);
    result.set_tptp_type(source.query_tptp_type());
    result.set_prop(source.give_props(CP_IS_SOS));
    result.set_prop(CP_IS_PURE_INJECTIVITY);
    clause_push_derivation(&mut result, DC_INV_REC, Some(source), None);
    result.set_weight(result.standard_weight());
    Ok(result)
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

fn unif_all_pairs(pairs: &mut Vec<Term>, subst: &mut Substitution) -> bool {
    assert_eq!(pairs.len() % 2, 0);
    let pos = subst.len();
    let mut unifies = true;

    while unifies && !pairs.is_empty() {
        let left = pairs
            .pop()
            .expect("even-sized unification pair stack has a left term");
        let right = pairs
            .pop()
            .expect("even-sized unification pair stack has a right term");
        unifies = subst_mgu_complete(&left, &right, subst);
    }

    if !unifies {
        subst.backtrack_to_pos(pos);
    }
    unifies
}

fn cmp_i64_to_order(value: i64) -> Ordering {
    value.cmp(&0)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_archive, clause_archive_copy, clause_boolean_simplification,
        clause_canon_compare_ref, clause_eliminate_naked_boolean_variables,
        clause_flip_literal_sign_index, clause_is_orphaned_with, clause_normalize_equations,
        clause_prune_args, clause_recognize_injectivity, clause_recognizes_choice,
        clause_remove_ac_resolved, clause_remove_literal, clause_remove_literal_index,
        clause_remove_superfluous_literals, clause_resolve_flex_clause, clause_set_archive_copy,
        clause_set_canonize, clause_set_delete_orphans_with, clause_set_recognize_choice,
        clause_set_remove_superfluous_literals, clause_set_replace_injectivity_defs,
        clause_unit_simplify_test, close_with_db_var, pstack_clause_print_format_string,
        pstack_clause_print_lop_string, tcf_tstp_parse, tformula_add_quantor,
        tformula_app_encode_string, tformula_clause_closed_encode, tformula_clause_encode,
        tformula_closure, tformula_collect_clause, tformula_collect_free_vars,
        tformula_conjunctive_nf, tformula_conjunctive_nf3, tformula_copy, tformula_copy_def,
        tformula_create_def, tformula_decode_polarity, tformula_def_rename,
        tformula_distribute_disjunctions, tformula_encode_predicate_as_eqn, tformula_equal,
        tformula_estimate_clauses, tformula_expand_distinct, tformula_expand_literals,
        tformula_fcode_alloc, tformula_find_defs, tformula_find_max_var_code,
        tformula_gc_mark_cells, tformula_has_free_vars, tformula_has_subform1,
        tformula_has_subform2, tformula_is_binary, tformula_is_closed, tformula_is_complex_bool,
        tformula_is_literal, tformula_is_prop_const, tformula_is_prop_false, tformula_is_prop_true,
        tformula_is_quantified, tformula_is_quantified_nl, tformula_is_unary, tformula_is_untyped,
        tformula_lift_ite, tformula_lift_lets, tformula_lit_alloc, tformula_mark_polarity,
        tformula_mini_scope, tformula_mini_scope3, tformula_neg_alloc, tformula_negate,
        tformula_nnf, tformula_preload_types, tformula_prop_constant_alloc, tformula_quantor_alloc,
        tformula_shift_quantors, tformula_shift_quantors2, tformula_simplify,
        tformula_simplify_decoded, tformula_skolemize_outermost, tformula_stack_to_form,
        tformula_to_cnf, tformula_tptp_parse, tformula_tptp_string, tformula_tstp_parse,
        tformula_unroll_fool, tformula_var_is_free, tformula_var_is_free_cached,
        tformula_var_rename, TFormulaDefinitions, TFormulaTptpPrintOptions, TFORM_MANY_CLAUSES,
    };
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_PURE_INJECTIVITY, CP_IS_SOS, CP_LIMITED_RW,
        CP_TYPE_AXIOM, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        clause_push_derivation, ClauseDerivationRef, DerivationEntry, DerivationParentRef,
        FormulaDerivationRef, DC_CNF_ADD_ARG, DC_CNF_EVAL_GC, DC_CNF_QUOTE, DC_DIST_DISJUNCTIONS,
        DC_ELIMINATE_BVAR, DC_FLEX_RESOLVE, DC_FNNF, DC_FOOL_UNROLL, DC_INV_REC, DC_NORMALIZE,
        DC_ORDERED_FACTOR, DC_PARAMOD, DC_PRUNE_ARG, DC_REWRITE, DC_SPLIT_CONJUNCT, DC_VAR_RENAME,
    };
    use crate::clauses::eqn::{eqn_app_encode_string, Eqn};
    use crate::clauses::eqn_props::{EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::terms::lambda::apply_terms as lambda_apply_terms;
    use crate::terms::signature::{
        Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE, FP_IS_INJ_DEF_SKOLEM, SIG_DB_LAMBDA_CODE,
        SIG_ITE_CODE, SIG_LET_CODE, SIG_NAMED_LAMBDA_CODE,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{
        DerefType, Term, TP_CHECK_FLAG, TP_GARBAGE_FLAG, TP_NEG_POLARITY, TP_POS_POLARITY,
    };
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;
    use std::collections::BTreeMap;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn prepare_formula_fresh_vars(bank: &TermBank) {
        bank.vars().set_v_counts_to_used();
        bank.vars().set_fresh_count_to_used();
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn bool_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn predicate_var(bank: &mut TermBank, code: i64) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type]));
        bank.vars().var_assert_alloc(code, &predicate_type)
    }

    fn higher_order_var(bank: &mut TermBank, code: i64, arg_count: usize) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let mut args = Vec::with_capacity(arg_count + 1);
        for _ in 0..arg_count {
            args.push(type_.clone());
        }
        args.push(type_);
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(args));
        bank.vars().var_assert_alloc(code, &arrow)
    }

    fn applied_predicate_var(bank: &mut TermBank, code: i64, arg_name: &str) -> Term {
        let predicate = predicate_var(bank, code);
        let argument = typed_const(bank, arg_name);
        let applied = bank.term_apply_arg(&predicate, &argument);
        bank.term_top_insert(applied).unwrap()
    }

    fn apply_many(bank: &mut TermBank, head: &Term, args: &[Term]) -> Term {
        lambda_apply_terms(bank, head, args).unwrap()
    }

    fn choice_const(bank: &mut TermBank, name: &str) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), bool_type]));
        let choice_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![predicate_type, arg_type]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, choice_type)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn choice_axiom(bank: &mut TermBank, name: &str, p_code: i64, x_code: i64) -> (Clause, i64) {
        let predicate = predicate_var(bank, p_code);
        let witness = typed_var(bank, x_code);
        let choice = choice_const(bank, name);
        let choice_applied = apply_many(bank, &choice, std::slice::from_ref(&predicate));
        let negative_atom = apply_many(bank, &predicate, std::slice::from_ref(&witness));
        let positive_atom = apply_many(bank, &predicate, std::slice::from_ref(&choice_applied));
        let true_term = bank.true_term().clone();
        let clause = clause_from(vec![
            literal(bank, &negative_atom, &true_term, false),
            literal(bank, &positive_atom, &true_term, true),
        ]);
        (clause, choice.f_code())
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn distinct_formula(bank: &mut TermBank, args: &[Term]) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(bank.signature().distinct_code(), args.len());
        term.set_type(Some(type_));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        bank.term_top_insert(term).unwrap()
    }

    fn default_bool_arg_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let value_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![value_type.clone(), bool_type, value_type.clone()]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(value_type));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn bool_unary_with_code(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![bool_type.clone(), bool_type]));
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(unary_type));
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn bool_result_unary_with_code(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn ite_with_type(
        bank: &mut TermBank,
        condition: &Term,
        if_true: &Term,
        if_false: &Term,
        type_: &crate::terms::simpletypes::Type,
    ) -> Term {
        let term = Term::top_alloc(SIG_ITE_CODE, 3);
        term.set_type(Some(type_.clone()));
        term.set_argument(0, condition.clone());
        term.set_argument(1, if_true.clone());
        term.set_argument(2, if_false.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn bool_ite(bank: &mut TermBank, condition: &Term, if_true: &Term, if_false: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        ite_with_type(bank, condition, if_true, if_false, &bool_type)
    }

    fn typed_ite(bank: &mut TermBank, condition: &Term, if_true: &Term, if_false: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        ite_with_type(bank, condition, if_true, if_false, &type_)
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

    fn typed_binary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
                )
                .unwrap();
        }
        f_code
    }

    fn ac_code(bank: &mut TermBank) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
                )
                .unwrap();
        }
        bank.signature_mut()
            .set_func_prop(f_code, FP_ASSOCIATIVE | FP_COMMUTATIVE);
        f_code
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn clause_archive_moves_original_and_returns_quoted_flat_copy_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "archive_a");
        let b = typed_const(&mut bank, "archive_b");
        let mut original = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        original.set_ident(60);
        original.set_csscpa_source(4);
        original.set_info(Some(ClauseInfo::new(Some("source"), None, -1, -1)));
        original
            .ensure_derivation()
            .push(DerivationEntry::Operation(DC_CNF_EVAL_GC));

        let mut archive = ClauseSet::new();
        let quoted = clause_archive(&mut archive, original, &mut bank).unwrap();

        assert_eq!(archive.members(), 1);
        let archived = archive.find_by_id(60).unwrap();
        assert_eq!(archived.info().and_then(ClauseInfo::name), Some("source"));
        assert_eq!(
            archived.derivation().map(PStack::as_slice),
            Some(&[DerivationEntry::Operation(DC_CNF_EVAL_GC)][..])
        );
        assert!(quoted.info().is_none());
        assert_eq!(
            quoted.derivation().map(PStack::as_slice),
            Some(
                &[
                    DerivationEntry::Operation(DC_CNF_QUOTE),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(60, 4)),
                ][..]
            )
        );
    }

    #[test]
    fn clause_archive_copy_transfers_info_and_derivation_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "copy_a");
        let b = typed_const(&mut bank, "copy_b");
        let mut clause = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        clause.set_ident(61);
        clause.set_csscpa_source(5);
        clause.set_info(Some(ClauseInfo::new(Some("active"), None, -1, -1)));
        clause
            .ensure_derivation()
            .push(DerivationEntry::Operation(DC_CNF_EVAL_GC));

        let mut archive = ClauseSet::new();
        let archived_ref = clause_archive_copy(&mut archive, &mut clause, &mut bank).unwrap();

        assert_eq!(archived_ref, ClauseDerivationRef::new(61, 5));
        assert_eq!(archive.members(), 1);
        let archived = archive.find_by_id(61).unwrap();
        assert_eq!(archived.info().and_then(ClauseInfo::name), Some("active"));
        assert_eq!(
            archived.derivation().map(PStack::as_slice),
            Some(&[DerivationEntry::Operation(DC_CNF_EVAL_GC)][..])
        );
        assert!(clause.info().is_none());
        assert_eq!(
            clause.derivation().map(PStack::as_slice),
            Some(
                &[
                    DerivationEntry::Operation(DC_CNF_QUOTE),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(61, 5)),
                ][..]
            )
        );
    }

    #[test]
    fn clause_set_archive_copy_archives_each_member_and_requotes_originals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_archive_a");
        let b = typed_const(&mut bank, "set_archive_b");
        let mut first = clause_from(vec![literal(&mut bank, &a, &b, true)]);
        first.set_ident(62);
        first.set_info(Some(ClauseInfo::new(Some("first"), None, -1, -1)));
        let mut second = clause_from(vec![literal(&mut bank, &b, &a, false)]);
        second.set_ident(63);
        second.set_info(Some(ClauseInfo::new(Some("second"), None, -1, -1)));
        let mut active = ClauseSet::from_clauses([first, second]);
        let mut archive = ClauseSet::new();

        let archived = clause_set_archive_copy(&mut archive, &mut active, &mut bank).unwrap();

        assert_eq!(archived, 2);
        assert_eq!(archive.members(), 2);
        assert_eq!(active.members(), 2);
        assert_eq!(
            archive
                .find_by_id(62)
                .and_then(Clause::info)
                .and_then(ClauseInfo::name),
            Some("first")
        );
        assert_eq!(
            active
                .find_by_id(62)
                .and_then(Clause::derivation)
                .and_then(|derivation| derivation.as_slice().first()),
            Some(&DerivationEntry::Operation(DC_CNF_QUOTE))
        );
        assert!(active.find_by_id(63).and_then(Clause::info).is_none());
    }

    #[test]
    fn clause_is_orphaned_ignores_missing_empty_and_non_generating_derivations() {
        let parent = Clause::alloc(EqnList::new());
        let mut no_derivation = Clause::alloc(EqnList::new());
        assert!(!clause_is_orphaned_with(&no_derivation, |_| true));

        no_derivation.ensure_derivation();
        assert!(!clause_is_orphaned_with(&no_derivation, |_| true));

        let mut rewritten = Clause::alloc(EqnList::new());
        clause_push_derivation(&mut rewritten, DC_REWRITE, Some(&parent), None);
        assert!(!clause_is_orphaned_with(&rewritten, |_| true));
    }

    #[test]
    fn clause_is_orphaned_checks_direct_generating_parents() {
        let mut left_parent = Clause::alloc(EqnList::new());
        left_parent.set_ident(70);
        let mut right_parent = Clause::alloc(EqnList::new());
        right_parent.set_ident(71);
        let mut child = Clause::alloc(EqnList::new());
        clause_push_derivation(
            &mut child,
            DC_PARAMOD,
            Some(&left_parent),
            Some(&right_parent),
        );

        assert!(!clause_is_orphaned_with(&child, |_| false));
        assert!(clause_is_orphaned_with(&child, |parent| {
            parent == DerivationParentRef::Clause(ClauseDerivationRef::new(71, 0))
        }));
    }

    #[test]
    fn clause_is_orphaned_scans_following_cnf_add_arg_entries_only() {
        let mut generating_parent = Clause::alloc(EqnList::new());
        generating_parent.set_ident(80);
        let mut added_parent = Clause::alloc(EqnList::new());
        added_parent.set_ident(81);
        let mut hidden_parent = Clause::alloc(EqnList::new());
        hidden_parent.set_ident(82);
        let mut child = Clause::alloc(EqnList::new());
        clause_push_derivation(
            &mut child,
            DC_ORDERED_FACTOR,
            Some(&generating_parent),
            None,
        );
        clause_push_derivation(&mut child, DC_CNF_ADD_ARG, Some(&added_parent), None);
        clause_push_derivation(&mut child, DC_REWRITE, Some(&hidden_parent), None);
        clause_push_derivation(&mut child, DC_CNF_ADD_ARG, Some(&hidden_parent), None);

        assert!(clause_is_orphaned_with(&child, |parent| {
            parent == DerivationParentRef::Clause(ClauseDerivationRef::new(81, 0))
        }));
        assert!(!clause_is_orphaned_with(&child, |parent| {
            parent == DerivationParentRef::Clause(ClauseDerivationRef::new(82, 0))
        }));
    }

    #[test]
    fn clause_set_delete_orphans_marks_deletes_and_clears_survivors_like_c() {
        let mut dead_parent = Clause::alloc(EqnList::new());
        dead_parent.set_ident(90);
        let mut live_parent = Clause::alloc(EqnList::new());
        live_parent.set_ident(91);

        let mut orphan = Clause::alloc(EqnList::new());
        orphan.set_ident(100);
        clause_push_derivation(&mut orphan, DC_ORDERED_FACTOR, Some(&dead_parent), None);

        let mut survivor = Clause::alloc(EqnList::new());
        survivor.set_ident(101);
        survivor.set_prop(CP_DELETE_CLAUSE);
        clause_push_derivation(&mut survivor, DC_ORDERED_FACTOR, Some(&live_parent), None);

        let mut set = ClauseSet::from_clauses([orphan, survivor]);

        let deleted = clause_set_delete_orphans_with(&mut set, |parent| {
            parent == DerivationParentRef::Clause(ClauseDerivationRef::new(90, 0))
        });

        assert_eq!(deleted, 1);
        assert!(set.find_by_id(100).is_none());
        let survivor = set.find_by_id(101).unwrap();
        assert!(!survivor.query_prop(CP_DELETE_CLAUSE));
    }

    #[test]
    fn clause_set_delete_orphans_preserves_non_orphan_counting() {
        let mut parent = Clause::alloc(EqnList::new());
        parent.set_ident(110);
        let mut child = Clause::alloc(EqnList::new());
        child.set_ident(111);
        clause_push_derivation(&mut child, DC_ORDERED_FACTOR, Some(&parent), None);

        let mut set = ClauseSet::from_clauses([child]);

        assert_eq!(clause_set_delete_orphans_with(&mut set, |_| false), 0);
        assert_eq!(set.find_by_id(111).map(Clause::ident), Some(111));
    }

    #[test]
    fn pstack_clause_print_lop_string_preserves_stack_order_extra_and_newlines() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "stack_a");
        let second = typed_const(&mut bank, "stack_b");
        let third = typed_const(&mut bank, "stack_c");
        let unit = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let mixed = clause_from(vec![
            literal(&mut bank, &second, &third, true),
            literal(&mut bank, &third, &first, false),
        ]);
        let mut stack = PStack::new();
        stack.push(&unit);
        stack.push(&mixed);

        assert_eq!(
            pstack_clause_print_lop_string(&bank, &stack, Some(" # extra")),
            "stack_a=stack_b <- . # extra\nstack_b=stack_c <- stack_c=stack_a. # extra\n"
        );
        assert_eq!(
            pstack_clause_print_lop_string(&bank, &stack, None),
            "stack_a=stack_b <- .\nstack_b=stack_c <- stack_c=stack_a.\n"
        );
    }

    #[test]
    fn pstack_clause_print_format_string_dispatches_clause_output() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "format_stack_a");
        let second = typed_const(&mut bank, "format_stack_b");
        let third = typed_const(&mut bank, "format_stack_c");
        let unit = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let mixed = clause_from(vec![
            literal(&mut bank, &second, &third, true),
            literal(&mut bank, &third, &first, false),
        ]);
        let mut stack = PStack::new();
        stack.push(&unit);
        stack.push(&mixed);

        let input_clause_stack = pstack_clause_print_format_string(
            &bank,
            &stack,
            Some(" # stack-extra"),
            IoFormat::Tptp,
            ProblemType::FirstOrder,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(input_clause_stack.matches(" # stack-extra\n").count(), 2);
        assert_eq!(input_clause_stack.matches("input_clause(").count(), 2);
        assert!(input_clause_stack.contains("++equal(format_stack_a, format_stack_b)"));
        assert!(!input_clause_stack.contains("<-"));

        let wrapped_clause_stack = pstack_clause_print_format_string(
            &bank,
            &stack,
            None,
            IoFormat::Tstp,
            ProblemType::FirstOrder,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(
            wrapped_clause_stack.matches("cnf(").count()
                + wrapped_clause_stack.matches("tcf(").count(),
            2
        );
        assert!(wrapped_clause_stack.contains("format_stack_a"));
        assert!(!wrapped_clause_stack.contains("<-"));

        assert_eq!(
            pstack_clause_print_format_string(
                &bank,
                &stack,
                Some(" # extra"),
                IoFormat::Auto,
                ProblemType::FirstOrder,
            )
            .unwrap_or_else(|err| panic!("{err}")),
            pstack_clause_print_lop_string(&bank, &stack, Some(" # extra"))
        );
    }

    #[test]
    fn remove_literal_helpers_update_counts_and_cached_weight() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let positive = literal(&mut bank, &first, &second, true);
        let negative = literal(&mut bank, &second, &third, false);
        let mut clause = clause_from(vec![positive.clone(), negative.clone()]);
        let original_weight = clause.weight();

        let removed = clause_remove_literal(&mut clause, &positive).unwrap();

        assert_eq!(removed, positive);
        assert_eq!(clause.literal_number(), 1);
        assert_eq!(clause.positive_literal_count(), 0);
        assert_eq!(clause.negative_literal_count(), 1);
        assert_eq!(clause.weight(), original_weight - removed.standard_weight());
        assert!(clause_remove_literal_index(&mut clause, 10).is_none());
    }

    #[test]
    fn flip_literal_sign_updates_cached_polarity_counts() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let mut clause = clause_from(vec![literal(&mut bank, &first, &second, true)]);

        assert!(clause_flip_literal_sign_index(&mut clause, 0));

        assert_eq!(clause.positive_literal_count(), 0);
        assert_eq!(clause.negative_literal_count(), 1);
        assert!(clause.literals().as_slice()[0].is_negative());
        assert!(!clause_flip_literal_sign_index(&mut clause, 1));
    }

    #[test]
    fn remove_superfluous_literals_deletes_false_and_duplicate_literals() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let positive = literal(&mut bank, &first, &second, true);
        let duplicate = literal(&mut bank, &second, &first, true);
        let false_literal = literal(&mut bank, &first, &first, false);
        let mut clause = clause_from(vec![positive, duplicate, false_literal]);
        clause.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert_eq!(clause_remove_superfluous_literals(&mut clause, &bank), 2);

        assert_eq!(clause.literal_number(), 1);
        assert_eq!(clause.weight(), clause.standard_weight());
        assert!(!clause.query_prop(CP_INITIAL));
        assert!(!clause.query_prop(CP_LIMITED_RW));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn clause_set_remove_superfluous_literals_updates_cached_literal_count() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let positive = literal(&mut bank, &first, &second, true);
        let duplicate = literal(&mut bank, &second, &first, true);
        let false_literal = literal(&mut bank, &first, &first, false);
        let dirty = clause_from(vec![positive, duplicate, false_literal]);
        let clean = clause_from(vec![literal(&mut bank, &second, &first, true)]);
        let mut set = ClauseSet::from_clauses([dirty, clean]);

        assert_eq!(set.literals(), 4);
        assert_eq!(clause_set_remove_superfluous_literals(&mut set, &bank), 2);

        assert_eq!(set.literals(), 2);
        assert_eq!(
            set.iter().map(Clause::literal_number).collect::<Vec<_>>(),
            vec![1, 1]
        );
    }

    #[test]
    fn clause_set_canonize_cleans_clauses_and_sorts_by_structural_weight() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let heavy = clause_from(vec![
            literal(&mut bank, &first, &second, true),
            literal(&mut bank, &second, &third, true),
        ]);
        let light = clause_from(vec![
            literal(&mut bank, &third, &third, false),
            literal(&mut bank, &first, &second, true),
        ]);
        let heavy_id = heavy.ident();
        let light_id = light.ident();
        let mut set = ClauseSet::from_clauses([heavy, light]);

        clause_set_canonize(&mut set, &bank);

        assert_eq!(set.literals(), 3);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![light_id, heavy_id]
        );
        assert!(set
            .iter()
            .all(|clause| clause.weight() == clause.standard_weight()));
        assert!(set.iter().all(|clause| {
            clause.is_sorted_by(|left, right| left.struct_weight_lex_compare(right, &bank))
        }));
    }

    #[test]
    fn remove_ac_resolved_deletes_negative_ac_trivial_literals() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let f_code = ac_code(&mut bank);
        let left = typed_binary_with_code(&mut bank, f_code, &first, &second);
        let right = typed_binary_with_code(&mut bank, f_code, &second, &first);
        let mut clause = clause_from(vec![literal(&mut bank, &left, &right, false)]);
        clause.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert_eq!(clause_remove_ac_resolved(&mut clause, &bank), 1);

        assert!(clause.is_empty());
        assert_eq!(clause.weight(), 0);
        assert!(!clause.query_prop(CP_INITIAL));
        assert!(!clause.query_prop(CP_LIMITED_RW));
    }

    #[test]
    fn boolean_simplification_collapses_absorbing_or_to_tautology() {
        let mut bank = test_bank();
        let variable = bool_var(&bank, -2);
        let truth = bank.true_term().clone();
        let or_code = bank.signature().or_code();
        let disjunction = bool_binary_with_code(&mut bank, or_code, &variable, &truth);
        let mut clause = clause_from(vec![literal(&mut bank, &disjunction, &truth, true)]);

        assert!(clause_boolean_simplification(&mut clause, &mut bank).unwrap());

        assert!(clause.literals().find_true(&bank).is_some());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn boolean_simplification_removes_duplicate_and_argument() {
        let mut bank = test_bank();
        let variable = bool_var(&bank, -4);
        let truth = bank.true_term().clone();
        let and_code = bank.signature().and_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &variable, &variable);
        let mut clause = clause_from(vec![literal(&mut bank, &conjunction, &truth, true)]);

        assert!(!clause_boolean_simplification(&mut clause, &mut bank).unwrap());
        let literal = &clause.literals().as_slice()[0];

        assert_eq!(literal.left(), &variable);
        assert_eq!(literal.right(), &truth);
        assert!(!literal.is_equ_lit(&bank));
    }

    #[test]
    fn normalize_equations_lifts_encoded_equality_to_literal_level() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "norm_eq_a");
        let right = typed_const(&mut bank, "norm_eq_b");
        let truth = bank.true_term().clone();
        let eqn_code = bank.signature().eqn_code();
        let encoded = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut literal = literal(&mut bank, &encoded, &truth, true);
        literal.set_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = clause_from(vec![literal]);

        assert!(clause_normalize_equations(&mut clause, &bank));

        let normalized = &clause.literals().as_slice()[0];
        assert_eq!(normalized.left(), &left);
        assert_eq!(normalized.right(), &right);
        assert!(normalized.is_positive());
        assert!(normalized.is_equ_lit(&bank));
        assert!(!normalized.query_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE));
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn normalize_equations_strips_not_and_flips_literal_sign() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "norm_not_a");
        let right = typed_const(&mut bank, "norm_not_b");
        let truth = bank.true_term().clone();
        let eqn_code = bank.signature().eqn_code();
        let not_code = bank.signature().not_code();
        let encoded_eq = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let encoded_not = bool_result_unary_with_code(&mut bank, not_code, &encoded_eq);
        let mut clause = clause_from(vec![literal(&mut bank, &encoded_not, &truth, true)]);

        assert!(clause_normalize_equations(&mut clause, &bank));

        let normalized = &clause.literals().as_slice()[0];
        assert_eq!(normalized.left(), &left);
        assert_eq!(normalized.right(), &right);
        assert!(normalized.is_negative());
        assert!(normalized.is_equ_lit(&bank));
    }

    #[test]
    fn normalize_equations_swaps_true_left_before_lifting_encoded_equality() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "norm_swap_a");
        let right = typed_const(&mut bank, "norm_swap_b");
        let truth = bank.true_term().clone();
        let eqn_code = bank.signature().eqn_code();
        let encoded = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let placeholder = bool_var(&bank, -60);
        let mut raw = literal(&mut bank, &placeholder, &truth, true);
        raw.set_left_raw(truth);
        raw.set_right_raw(encoded);
        raw.set_prop(EP_IS_EQU_LITERAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = clause_from(vec![raw]);

        assert!(clause_normalize_equations(&mut clause, &bank));

        let normalized = &clause.literals().as_slice()[0];
        assert_eq!(normalized.left(), &left);
        assert_eq!(normalized.right(), &right);
        assert!(normalized.is_positive());
        assert!(normalized.is_equ_lit(&bank));
        assert!(!normalized.query_prop(EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn clause_prune_args_removes_constant_argument_across_occurrences() {
        let mut bank = test_bank();
        let function = higher_order_var(&mut bank, -100, 2);
        let constant = typed_const(&mut bank, "prune_const_a");
        let x = typed_var(&bank, -102);
        let y = typed_var(&bank, -104);
        let first = apply_many(&mut bank, &function, &[constant.clone(), x.clone()]);
        let second = apply_many(&mut bank, &function, &[constant, y.clone()]);
        let rhs_first = typed_const(&mut bank, "prune_const_rhs_1");
        let rhs_second = typed_const(&mut bank, "prune_const_rhs_2");
        let mut clause = clause_from(vec![
            literal(&mut bank, &first, &rhs_first, true),
            literal(&mut bank, &second, &rhs_second, true),
        ]);

        assert!(clause_prune_args(&mut clause, &mut bank).unwrap());

        let first_left = clause.literals().as_slice()[0].left();
        let second_left = clause.literals().as_slice()[1].left();
        assert!(first_left.is_applied_free_var());
        assert!(second_left.is_applied_free_var());
        assert_eq!(first_left.arity(), 2);
        assert_eq!(second_left.arity(), 2);
        assert_ne!(first_left.argument(0).as_ref(), Some(&function));
        assert_eq!(first_left.argument(1).as_ref(), Some(&x));
        assert_eq!(second_left.argument(1).as_ref(), Some(&y));
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_PRUNE_ARG)]
        );
    }

    #[test]
    fn clause_prune_args_removes_repeated_argument_position() {
        let mut bank = test_bank();
        let function = higher_order_var(&mut bank, -110, 2);
        let x = typed_var(&bank, -112);
        let y = typed_var(&bank, -114);
        let first = apply_many(&mut bank, &function, &[x.clone(), x.clone()]);
        let second = apply_many(&mut bank, &function, &[y.clone(), y.clone()]);
        let rhs_first = typed_const(&mut bank, "prune_repeat_rhs_1");
        let rhs_second = typed_const(&mut bank, "prune_repeat_rhs_2");
        let mut clause = clause_from(vec![
            literal(&mut bank, &first, &rhs_first, true),
            literal(&mut bank, &second, &rhs_second, true),
        ]);

        assert!(clause_prune_args(&mut clause, &mut bank).unwrap());

        let first_left = clause.literals().as_slice()[0].left();
        let second_left = clause.literals().as_slice()[1].left();
        assert!(first_left.is_applied_free_var());
        assert!(second_left.is_applied_free_var());
        assert_eq!(first_left.arity(), 2);
        assert_eq!(second_left.arity(), 2);
        assert_ne!(first_left.argument(0).as_ref(), Some(&function));
        assert_eq!(first_left.argument(1).as_ref(), Some(&x));
        assert_eq!(second_left.argument(1).as_ref(), Some(&y));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_PRUNE_ARG)]
        );
    }

    #[test]
    fn clause_prune_args_ignores_variables_without_removable_arguments() {
        let mut bank = test_bank();
        let function = higher_order_var(&mut bank, -120, 2);
        let x = typed_var(&bank, -122);
        let y = typed_var(&bank, -124);
        let first = apply_many(&mut bank, &function, &[x.clone(), y.clone()]);
        let second = apply_many(&mut bank, &function, &[y, x]);
        let rhs_first = typed_const(&mut bank, "prune_none_rhs_1");
        let rhs_second = typed_const(&mut bank, "prune_none_rhs_2");
        let mut clause = clause_from(vec![
            literal(&mut bank, &first, &rhs_first, true),
            literal(&mut bank, &second, &rhs_second, true),
        ]);

        assert!(!clause_prune_args(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literals().as_slice()[0].left(), &first);
        assert!(clause.derivation().is_none());
    }

    #[test]
    fn tformula_simplify_decoded_unary_or_neutral_returns_identity_lambda() {
        let mut bank = test_bank();
        let false_term = bank.false_term().clone();
        let or_code = bank.signature().or_code();
        let unary = bool_unary_with_code(&mut bank, or_code, &false_term);

        let simplified = tformula_simplify_decoded(&mut bank, &unary, true).unwrap();

        assert_eq!(simplified.f_code(), SIG_DB_LAMBDA_CODE);
        let binder = simplified.argument(0).unwrap();
        let body = simplified.argument(1).unwrap();
        assert!(binder.is_db_var());
        assert_eq!(body, binder);
        assert_eq!(simplified.type_(), unary.type_());
    }

    #[test]
    fn tformula_simplify_decoded_unary_and_absorbing_returns_constant_lambda() {
        let mut bank = test_bank();
        let false_term = bank.false_term().clone();
        let and_code = bank.signature().and_code();
        let unary = bool_unary_with_code(&mut bank, and_code, &false_term);

        let simplified = tformula_simplify_decoded(&mut bank, &unary, true).unwrap();

        assert_eq!(simplified.f_code(), SIG_DB_LAMBDA_CODE);
        let binder = simplified.argument(0).unwrap();
        let body = simplified.argument(1).unwrap();
        assert!(binder.is_db_var());
        assert_eq!(body, false_term);
        assert_eq!(simplified.type_(), unary.type_());
    }

    #[test]
    fn tformula_simplify_decoded_quantifier_closed_lambda_returns_matrix() {
        let mut bank = test_bank();
        let bool_type = bank.signature().type_bank().bool_type();
        let body = bank.true_term().clone();
        let lambda = close_with_db_var(&mut bank, &bool_type, &body).unwrap();
        let qex_code = bank.signature().qex_code();
        let formula = bool_result_unary_with_code(&mut bank, qex_code, &lambda);

        let simplified = tformula_simplify_decoded(&mut bank, &formula, true).unwrap();

        assert_eq!(simplified, body);
    }

    #[test]
    fn tformula_simplify_decoded_quantifier_open_lambda_keeps_formula() {
        let mut bank = test_bank();
        let bool_type = bank.signature().type_bank().bool_type();
        let open_body = bank.request_db_var(&bool_type, 1);
        let lambda = close_with_db_var(&mut bank, &bool_type, &open_body).unwrap();
        let qall_code = bank.signature().qall_code();
        let formula = bool_result_unary_with_code(&mut bank, qall_code, &lambda);

        let simplified = tformula_simplify_decoded(&mut bank, &formula, true).unwrap();

        assert_eq!(simplified, formula);
    }

    #[test]
    fn tformula_neg_alloc_wraps_non_negated_formula() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "neg_alloc_left");
        let right = typed_const(&mut bank, "neg_alloc_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &left, &right);

        let negated = tformula_neg_alloc(&mut bank, &atom).unwrap();

        assert_eq!(negated.f_code(), bank.signature().not_code());
        assert_eq!(negated.argument(0).as_ref(), Some(&atom));
        assert_eq!(
            negated.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
    }

    #[test]
    fn tformula_neg_alloc_flattens_one_root_negation() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "neg_alloc_flatten_left");
        let right = typed_const(&mut bank, "neg_alloc_flatten_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let negated = tformula_neg_alloc(&mut bank, &atom).unwrap();

        let flattened = tformula_neg_alloc(&mut bank, &negated).unwrap();

        assert_eq!(flattened, atom);
    }

    #[test]
    fn tformula_expand_literals_makes_disequality_negation_explicit() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "expand_neq_left");
        let right = typed_const(&mut bank, "expand_neq_right");
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let disequality = bool_binary_with_code(&mut bank, neqn_code, &left, &right);

        let expanded = tformula_expand_literals(&mut bank, &disequality).unwrap();

        assert_eq!(expanded.f_code(), bank.signature().not_code());
        let equality = expanded.argument(0).unwrap();
        assert_eq!(equality.f_code(), bank.signature().eqn_code());
        assert_eq!(equality.argument(0).as_ref(), Some(&left));
        assert_eq!(equality.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_expand_literals_turns_boolean_equality_into_equivalence() {
        let mut bank = test_bank();
        let and_code = bank.signature().and_code();
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();
        let left = bool_binary_with_code(&mut bank, and_code, &true_term, &false_term);
        let right = false_term;
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);

        let expanded = tformula_expand_literals(&mut bank, &equality).unwrap();

        assert_eq!(expanded.f_code(), bank.signature().equiv_code());
        assert_eq!(expanded.argument(0).as_ref(), Some(&left));
        assert_eq!(expanded.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_expand_literals_unwraps_internal_boolean_eq_true() {
        let mut bank = test_bank();
        let or_code = bank.signature().or_code();
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();
        let left = bool_binary_with_code(&mut bank, or_code, &true_term, &false_term);
        let right = true_term;
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);

        let expanded = tformula_expand_literals(&mut bank, &equality).unwrap();

        assert_eq!(expanded, left);
    }

    #[test]
    fn tformula_expand_literals_keeps_boolean_free_var_eq_true() {
        let mut bank = test_bank();
        let left = bool_var(&bank, -144);
        let right = bank.true_term().clone();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);

        let expanded = tformula_expand_literals(&mut bank, &equality).unwrap();

        assert_eq!(expanded, equality);
    }

    #[test]
    fn tformula_lift_ite_unrolls_formula_position_ite() {
        let mut bank = test_bank();
        let condition_left = typed_const(&mut bank, "lift_ite_formula_condition_left");
        let condition_right = typed_const(&mut bank, "lift_ite_formula_condition_right");
        let then_left = typed_const(&mut bank, "lift_ite_formula_then_left");
        let then_right = typed_const(&mut bank, "lift_ite_formula_then_right");
        let else_left = typed_const(&mut bank, "lift_ite_formula_else_left");
        let else_right = typed_const(&mut bank, "lift_ite_formula_else_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let condition =
            bool_binary_with_code(&mut bank, eqn_code, &condition_left, &condition_right);
        let then_atom = bool_binary_with_code(&mut bank, eqn_code, &then_left, &then_right);
        let else_atom = bool_binary_with_code(&mut bank, eqn_code, &else_left, &else_right);
        let ite = bool_ite(&mut bank, &condition, &then_atom, &else_atom);

        let lifted = tformula_lift_ite(&mut bank, &ite).unwrap();

        assert_eq!(lifted.f_code(), and_code);
        let true_branch = lifted.argument(0).unwrap();
        let false_branch = lifted.argument(1).unwrap();
        assert_eq!(true_branch.f_code(), or_code);
        assert_eq!(false_branch.f_code(), or_code);

        let negated_condition = true_branch.argument(0).unwrap();
        assert_eq!(negated_condition.f_code(), neqn_code);
        assert_eq!(negated_condition.argument(0), condition.argument(0));
        assert_eq!(negated_condition.argument(1), condition.argument(1));
        assert_eq!(true_branch.argument(1).as_ref(), Some(&then_atom));
        assert_eq!(false_branch.argument(0).as_ref(), Some(&condition));
        assert_eq!(false_branch.argument(1).as_ref(), Some(&else_atom));
    }

    #[test]
    fn tformula_lift_ite_unrolls_literal_left_side_term_ite() {
        let mut bank = test_bank();
        let cond_left = typed_const(&mut bank, "lift_ite_left_cond_l");
        let cond_right = typed_const(&mut bank, "lift_ite_left_cond_r");
        let then_value = typed_const(&mut bank, "lift_ite_left_then");
        let else_value = typed_const(&mut bank, "lift_ite_left_else");
        let target = typed_const(&mut bank, "lift_ite_left_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let condition = bool_binary_with_code(&mut bank, eqn_code, &cond_left, &cond_right);
        let ite = typed_ite(&mut bank, &condition, &then_value, &else_value);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &ite, &target);

        let lifted = tformula_lift_ite(&mut bank, &formula).unwrap();

        assert_eq!(lifted.f_code(), and_code);
        let true_branch = lifted.argument(0).unwrap();
        let false_branch = lifted.argument(1).unwrap();
        assert_eq!(true_branch.f_code(), or_code);
        assert_eq!(false_branch.f_code(), or_code);

        let negated_condition = true_branch.argument(0).unwrap();
        assert_eq!(negated_condition.f_code(), neqn_code);
        assert_eq!(negated_condition.argument(0), condition.argument(0));
        assert_eq!(negated_condition.argument(1), condition.argument(1));

        let true_case = true_branch.argument(1).unwrap();
        assert_eq!(true_case.f_code(), eqn_code);
        assert_eq!(true_case.argument(0).as_ref(), Some(&then_value));
        assert_eq!(true_case.argument(1).as_ref(), Some(&target));

        assert_eq!(false_branch.argument(0).as_ref(), Some(&condition));
        let false_case = false_branch.argument(1).unwrap();
        assert_eq!(false_case.f_code(), eqn_code);
        assert_eq!(false_case.argument(0).as_ref(), Some(&else_value));
        assert_eq!(false_case.argument(1).as_ref(), Some(&target));
    }

    #[test]
    fn tformula_lift_ite_prefers_literal_left_side_before_right_side() {
        let mut bank = test_bank();
        let left_cond_l = typed_const(&mut bank, "lift_ite_prefer_left_cond_l");
        let left_cond_r = typed_const(&mut bank, "lift_ite_prefer_left_cond_r");
        let right_cond_l = typed_const(&mut bank, "lift_ite_prefer_right_cond_l");
        let right_cond_r = typed_const(&mut bank, "lift_ite_prefer_right_cond_r");
        let left_then = typed_const(&mut bank, "lift_ite_prefer_left_then");
        let left_else = typed_const(&mut bank, "lift_ite_prefer_left_else");
        let right_then = typed_const(&mut bank, "lift_ite_prefer_right_then");
        let right_else = typed_const(&mut bank, "lift_ite_prefer_right_else");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let left_condition = bool_binary_with_code(&mut bank, eqn_code, &left_cond_l, &left_cond_r);
        let right_condition =
            bool_binary_with_code(&mut bank, eqn_code, &right_cond_l, &right_cond_r);
        let left_ite = typed_ite(&mut bank, &left_condition, &left_then, &left_else);
        let right_ite = typed_ite(&mut bank, &right_condition, &right_then, &right_else);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &left_ite, &right_ite);

        let lifted = tformula_lift_ite(&mut bank, &formula).unwrap();

        assert_eq!(lifted.f_code(), and_code);
        let true_branch = lifted.argument(0).unwrap();
        let false_branch = lifted.argument(1).unwrap();
        assert_eq!(true_branch.f_code(), or_code);
        assert_eq!(false_branch.f_code(), or_code);

        let negated_left_condition = true_branch.argument(0).unwrap();
        assert_eq!(negated_left_condition.f_code(), neqn_code);
        assert_eq!(
            negated_left_condition.argument(0),
            left_condition.argument(0)
        );
        assert_eq!(
            negated_left_condition.argument(1),
            left_condition.argument(1)
        );
        assert_eq!(false_branch.argument(0).as_ref(), Some(&left_condition));

        let true_case_after_right_unroll = true_branch.argument(1).unwrap();
        assert_eq!(true_case_after_right_unroll.f_code(), and_code);
        let true_case_first_branch = true_case_after_right_unroll.argument(0).unwrap();
        assert_eq!(true_case_first_branch.f_code(), or_code);
        let true_case_literal = true_case_first_branch.argument(1).unwrap();
        assert_eq!(true_case_literal.f_code(), eqn_code);
        assert_eq!(true_case_literal.argument(0).as_ref(), Some(&left_then));
    }

    #[test]
    fn tformula_lift_lets_replaces_local_symbol_and_emits_definition() {
        let mut bank = test_bank();
        let local_symbol = typed_const(&mut bank, "lift_let_local_symbol");
        let definition_value = typed_const(&mut bank, "lift_let_definition_value");
        let target = typed_const(&mut bank, "lift_let_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition =
            bool_binary_with_code(&mut bank, eqn_code, &local_symbol, &definition_value);
        let let_term = let_term(&mut bank, &[definition], &local_symbol);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &let_term, &target);

        let lifted = tformula_lift_lets(&mut bank, &formula).unwrap();

        assert_eq!(lifted.definitions.len(), 1);
        assert_eq!(lifted.formula.f_code(), eqn_code);
        assert_eq!(lifted.formula.argument(1).as_ref(), Some(&target));
        let fresh_symbol = lifted.formula.argument(0).unwrap();
        assert_ne!(fresh_symbol, local_symbol);
        assert_ne!(fresh_symbol.f_code(), local_symbol.f_code());

        let generated_definition = &lifted.definitions[0];
        assert_eq!(generated_definition.f_code(), eqn_code);
        assert_eq!(
            generated_definition.argument(0).as_ref(),
            Some(&fresh_symbol)
        );
        assert_eq!(
            generated_definition.argument(1).as_ref(),
            Some(&definition_value)
        );
    }

    #[test]
    fn tformula_unroll_fool_splits_boolean_term_argument_in_literal() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "fool_unroll_a");
        let b = typed_const(&mut bank, "fool_unroll_b");
        let c = typed_const(&mut bank, "fool_unroll_c");
        let d = typed_const(&mut bank, "fool_unroll_d");
        let target = typed_const(&mut bank, "fool_unroll_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let not_code = bank.signature().not_code();
        let bool_subformula = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let applied = default_bool_arg_binary(&mut bank, "fool_unroll_f", &a, &bool_subformula);
        assert!(applied.has_bool_subterm());
        let formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);

        let unrolled = tformula_unroll_fool(&mut bank, &formula).unwrap();

        assert_eq!(unrolled.f_code(), and_code);
        let true_branch = unrolled.argument(0).unwrap();
        let false_branch = unrolled.argument(1).unwrap();
        assert_eq!(true_branch.f_code(), or_code);
        assert_eq!(false_branch.f_code(), or_code);

        let negated_guard = true_branch.argument(0).unwrap();
        assert_eq!(negated_guard.f_code(), not_code);
        assert_eq!(negated_guard.argument(0).as_ref(), Some(&bool_subformula));
        let true_case = true_branch.argument(1).unwrap();
        assert_eq!(true_case.f_code(), eqn_code);
        let true_case_left = true_case.argument(0).unwrap();
        assert_eq!(true_case_left.argument(0).as_ref(), Some(&a));
        assert_eq!(true_case_left.argument(1).as_ref(), Some(bank.true_term()));
        assert_eq!(true_case.argument(1).as_ref(), Some(&target));

        assert_eq!(false_branch.argument(0).as_ref(), Some(&bool_subformula));
        let false_case = false_branch.argument(1).unwrap();
        assert_eq!(false_case.f_code(), eqn_code);
        let false_case_left = false_case.argument(0).unwrap();
        assert_eq!(false_case_left.argument(0).as_ref(), Some(&a));
        assert_eq!(
            false_case_left.argument(1).as_ref(),
            Some(bank.false_term())
        );
        assert_eq!(false_case.argument(1).as_ref(), Some(&target));
    }

    #[test]
    fn tformula_unroll_fool_ignores_boolean_variable_argument() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "fool_ignore_a");
        let target = typed_const(&mut bank, "fool_ignore_target");
        let bool_arg = bool_var(&bank, -145);
        let applied = default_bool_arg_binary(&mut bank, "fool_ignore_f", &a, &bool_arg);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);

        let unrolled = tformula_unroll_fool(&mut bank, &formula).unwrap();

        assert_eq!(unrolled, formula);
    }

    #[test]
    fn tformula_simplify_reduces_or_and_constants_and_duplicates() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "simplify_const_a");
        let b = typed_const(&mut bank, "simplify_const_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let true_prop = bool_binary_with_code(&mut bank, eqn_code, &truth, &truth);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let false_or_atom = bool_binary_with_code(&mut bank, or_code, &false_prop, &atom);
        let true_and_atom = bool_binary_with_code(&mut bank, and_code, &true_prop, &atom);
        let duplicate_or = bool_binary_with_code(&mut bank, or_code, &atom, &atom);

        assert_eq!(
            tformula_simplify(&mut bank, &false_or_atom, 1000).unwrap(),
            atom
        );
        assert_eq!(
            tformula_simplify(&mut bank, &true_and_atom, 1000).unwrap(),
            atom
        );
        assert_eq!(
            tformula_simplify(&mut bank, &duplicate_or, 1000).unwrap(),
            atom
        );
    }

    #[test]
    fn tformula_simplify_recurses_then_applies_root_loop() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "simplify_recurse_a");
        let b = typed_const(&mut bank, "simplify_recurse_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let true_prop = bool_binary_with_code(&mut bank, eqn_code, &truth, &truth);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let simplified_child = bool_binary_with_code(&mut bank, or_code, &false_prop, &atom);
        let formula = bool_binary_with_code(&mut bank, and_code, &simplified_child, &true_prop);

        let simplified = tformula_simplify(&mut bank, &formula, 1000).unwrap();

        assert_eq!(simplified, atom);
    }

    #[test]
    fn tformula_simplify_rewrites_equivalence_and_implication_rules() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "simplify_equiv_a");
        let b = typed_const(&mut bank, "simplify_equiv_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let true_prop = bool_binary_with_code(&mut bank, eqn_code, &truth, &truth);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let equiv_code = bank.signature().equiv_code();
        let impl_code = bank.signature().impl_code();
        let equiv_false = bool_binary_with_code(&mut bank, equiv_code, &atom, &false_prop);
        let impl_false = bool_binary_with_code(&mut bank, impl_code, &atom, &false_prop);
        let false_impl = bool_binary_with_code(&mut bank, impl_code, &false_prop, &atom);

        let simplified_equiv = tformula_simplify(&mut bank, &equiv_false, 1000).unwrap();
        let simplified_impl = tformula_simplify(&mut bank, &impl_false, 1000).unwrap();
        let simplified_false_impl = tformula_simplify(&mut bank, &false_impl, 1000).unwrap();

        assert_eq!(simplified_equiv.f_code(), neqn_code);
        assert_eq!(simplified_equiv.argument(0).as_ref(), Some(&a));
        assert_eq!(simplified_equiv.argument(1).as_ref(), Some(&b));
        assert_eq!(simplified_impl, simplified_equiv);
        assert_eq!(simplified_false_impl, true_prop);
    }

    #[test]
    fn tformula_simplify_rewrites_xor_bimpl_nand_and_nor() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "simplify_alt_a");
        let b = typed_const(&mut bank, "simplify_alt_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let true_prop = bool_binary_with_code(&mut bank, eqn_code, &truth, &truth);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let xor_code = bank.signature().xor_code();
        let bimpl_code = bank.signature().bimpl_code();
        let nand_code = bank.signature().nand_code();
        let nor_code = bank.signature().nor_code();
        let xor_same = bool_binary_with_code(&mut bank, xor_code, &atom, &atom);
        let bimpl_false = bool_binary_with_code(&mut bank, bimpl_code, &false_prop, &atom);
        let nand_true = bool_binary_with_code(&mut bank, nand_code, &true_prop, &true_prop);
        let nor_false = bool_binary_with_code(&mut bank, nor_code, &false_prop, &false_prop);

        assert_eq!(
            tformula_simplify(&mut bank, &xor_same, 1000).unwrap(),
            false_prop
        );
        let simplified_bimpl = tformula_simplify(&mut bank, &bimpl_false, 1000).unwrap();
        assert_eq!(simplified_bimpl.f_code(), neqn_code);
        assert_eq!(simplified_bimpl.argument(0).as_ref(), Some(&a));
        assert_eq!(simplified_bimpl.argument(1).as_ref(), Some(&b));
        assert_eq!(
            tformula_simplify(&mut bank, &nand_true, 1000).unwrap(),
            false_prop
        );
        assert_eq!(
            tformula_simplify(&mut bank, &nor_false, 1000).unwrap(),
            true_prop
        );
    }

    #[test]
    fn tformula_simplify_removes_redundant_small_quantifier() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -194);
        let a = typed_const(&mut bank, "simplify_quant_a");
        let b = typed_const(&mut bank, "simplify_quant_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let qall_code = bank.signature().qall_code();
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        let simplified = tformula_simplify(&mut bank, &quantified, 1000).unwrap();

        assert_eq!(simplified, body);
    }

    #[test]
    fn tformula_simplify_keeps_quantifier_when_variable_is_free() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -196);
        let a = typed_const(&mut bank, "simplify_quant_keep_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qex_code = bank.signature().qex_code();
        let quantified = bool_binary_with_code(&mut bank, qex_code, &x, &body);

        let simplified = tformula_simplify(&mut bank, &quantified, 1000).unwrap();

        assert_eq!(simplified, quantified);
    }

    #[test]
    fn tformula_estimate_clauses_counts_positive_and_negative_connectives() {
        let mut bank = test_bank();
        let left_a = typed_const(&mut bank, "estimate_a");
        let left_b = typed_const(&mut bank, "estimate_b");
        let left_c = typed_const(&mut bank, "estimate_c");
        let left_d = typed_const(&mut bank, "estimate_d");
        let right_a = typed_const(&mut bank, "estimate_e");
        let right_b = typed_const(&mut bank, "estimate_f");
        let right_c = typed_const(&mut bank, "estimate_g");
        let right_d = typed_const(&mut bank, "estimate_h");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_1 = bool_binary_with_code(&mut bank, eqn_code, &left_a, &left_b);
        let left_2 = bool_binary_with_code(&mut bank, eqn_code, &left_c, &left_d);
        let right_1 = bool_binary_with_code(&mut bank, eqn_code, &right_a, &right_b);
        let right_2 = bool_binary_with_code(&mut bank, eqn_code, &right_c, &right_d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let impl_code = bank.signature().impl_code();
        let equiv_code = bank.signature().equiv_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &left_1, &left_2);
        let disjunction = bool_binary_with_code(&mut bank, or_code, &right_1, &right_2);
        let implication = bool_binary_with_code(&mut bank, impl_code, &conjunction, &disjunction);
        let equivalence = bool_binary_with_code(&mut bank, equiv_code, &conjunction, &disjunction);

        assert_eq!(tformula_estimate_clauses(&bank, &conjunction, true), 2);
        assert_eq!(tformula_estimate_clauses(&bank, &conjunction, false), 1);
        assert_eq!(tformula_estimate_clauses(&bank, &disjunction, true), 1);
        assert_eq!(tformula_estimate_clauses(&bank, &disjunction, false), 2);
        assert_eq!(tformula_estimate_clauses(&bank, &implication, true), 1);
        assert_eq!(tformula_estimate_clauses(&bank, &implication, false), 4);
        assert_eq!(tformula_estimate_clauses(&bank, &equivalence, true), 5);
        assert_eq!(tformula_estimate_clauses(&bank, &equivalence, false), 4);
    }

    #[test]
    fn tformula_estimate_clauses_treats_marked_subform_as_definition_atom() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "estimate_marked_a");
        let b = typed_const(&mut bank, "estimate_marked_b");
        let c = typed_const(&mut bank, "estimate_marked_c");
        let d = typed_const(&mut bank, "estimate_marked_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &left, &right);
        conjunction.set_prop(TP_CHECK_FLAG);

        assert_eq!(tformula_estimate_clauses(&bank, &conjunction, true), 1);
        assert_eq!(tformula_estimate_clauses(&bank, &conjunction, false), 1);
    }

    #[test]
    fn tformula_estimate_clauses_passes_through_quantifier_body() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -34);
        let a = typed_const(&mut bank, "estimate_quant_a");
        let b = typed_const(&mut bank, "estimate_quant_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let right = bool_binary_with_code(&mut bank, eqn_code, &x, &b);
        let and_code = bank.signature().and_code();
        let qall_code = bank.signature().qall_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left, &right);
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        assert_eq!(tformula_estimate_clauses(&bank, &quantified, true), 2);
        assert_eq!(tformula_estimate_clauses(&bank, &quantified, false), 1);
    }

    #[test]
    fn tformula_estimate_clauses_returns_many_sentinel_above_c_limit() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "estimate_many_a");
        let b = typed_const(&mut bank, "estimate_many_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let mut left = atom.clone();
        let mut right = atom.clone();
        for _ in 1..33 {
            left = bool_binary_with_code(&mut bank, and_code, &left, &atom);
            right = bool_binary_with_code(&mut bank, and_code, &right, &atom);
        }
        let disjunction = bool_binary_with_code(&mut bank, or_code, &left, &right);

        assert_eq!(
            tformula_estimate_clauses(&bank, &disjunction, true),
            TFORM_MANY_CLAUSES
        );
    }

    #[test]
    fn tformula_def_rename_allocates_bool_definition_atom_and_generalizes_polarity() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -35);
        let left_const = typed_const(&mut bank, "def_rename_left");
        let right_const = typed_const(&mut bank, "def_rename_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom_with_var = bool_binary_with_code(&mut bank, eqn_code, &variable, &left_const);
        let ground_atom = bool_binary_with_code(&mut bank, eqn_code, &left_const, &right_const);
        let and_code = bank.signature().and_code();
        let formula = bool_binary_with_code(&mut bank, and_code, &atom_with_var, &ground_atom);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();

        let rename_atom =
            tformula_def_rename(&mut bank, &formula, 1, &mut defs, &mut renamed_forms).unwrap();

        assert!(formula.query_prop(TP_CHECK_FLAG));
        assert_eq!(renamed_forms, vec![formula.clone()]);
        let definition = defs.get(&formula.entry_no()).unwrap();
        assert_eq!(definition.polarity(), 1);
        assert_eq!(definition.rename_atom(), &rename_atom);
        assert_eq!(rename_atom.f_code(), bank.signature().eqn_code());
        assert_eq!(rename_atom.argument(1).as_ref(), Some(bank.true_term()));
        let def_predicate = rename_atom.argument(0).unwrap();
        assert_eq!(def_predicate.arity(), 1);
        assert_eq!(def_predicate.argument(0).as_ref(), Some(&variable));

        let repeated =
            tformula_def_rename(&mut bank, &formula, -1, &mut defs, &mut renamed_forms).unwrap();

        assert_eq!(repeated, rename_atom);
        assert_eq!(renamed_forms, vec![formula.clone()]);
        assert_eq!(defs.get(&formula.entry_no()).unwrap().polarity(), 0);
    }

    #[test]
    fn tformula_find_defs_renames_expensive_disjunct_depth_first() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "find_defs_first");
        let second = typed_const(&mut bank, "find_defs_second");
        let third = typed_const(&mut bank, "find_defs_third");
        let fourth = typed_const(&mut bank, "find_defs_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equivalence_code = bank.signature().equiv_code();
        let or_code = bank.signature().or_code();
        let expensive = bool_binary_with_code(&mut bank, equivalence_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();

        tformula_find_defs(&mut bank, &formula, 1, 1, &mut defs, &mut renamed_forms).unwrap();

        assert_eq!(renamed_forms, vec![expensive.clone()]);
        assert!(expensive.query_prop(TP_CHECK_FLAG));
        assert_eq!(defs.get(&expensive.entry_no()).unwrap().polarity(), 1);
    }

    #[test]
    fn tformula_find_defs_preserves_implication_consequent_polarity_artifact() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "find_defs_impl_first");
        let second = typed_const(&mut bank, "find_defs_impl_second");
        let third = typed_const(&mut bank, "find_defs_impl_third");
        let fourth = typed_const(&mut bank, "find_defs_impl_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let antecedent = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let left_consequent = bool_binary_with_code(&mut bank, eqn_code, &first, &third);
        let right_consequent = bool_binary_with_code(&mut bank, eqn_code, &second, &fourth);
        let and_code = bank.signature().and_code();
        let implication_code = bank.signature().impl_code();
        let consequent =
            bool_binary_with_code(&mut bank, and_code, &left_consequent, &right_consequent);
        let formula = bool_binary_with_code(&mut bank, implication_code, &antecedent, &consequent);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();

        tformula_find_defs(&mut bank, &formula, 1, 1, &mut defs, &mut renamed_forms).unwrap();

        assert!(renamed_forms.is_empty());
        assert!(defs.is_empty());
        assert!(!consequent.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn tformula_copy_def_replaces_unblocked_marked_subform_and_records_definition() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "copy_def_first");
        let second = typed_const(&mut bank, "copy_def_second");
        let third = typed_const(&mut bank, "copy_def_third");
        let fourth = typed_const(&mut bank, "copy_def_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let marked = bool_binary_with_code(&mut bank, or_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let formula = bool_binary_with_code(&mut bank, and_code, &marked, &tail);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();
        let rename_atom =
            tformula_def_rename(&mut bank, &marked, 1, &mut defs, &mut renamed_forms).unwrap();
        let archived_ref = FormulaDerivationRef::new(77);
        defs.get_mut(&marked.entry_no())
            .unwrap()
            .set_definition_metadata(77, marked.clone(), archived_ref);
        let mut defs_used = Vec::new();

        let copied = tformula_copy_def(&mut bank, &formula, 99, &defs, &mut defs_used).unwrap();

        assert_eq!(copied.f_code(), bank.signature().and_code());
        assert_eq!(copied.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(copied.argument(1).as_ref(), Some(&tail));
        assert_eq!(defs_used, vec![archived_ref]);

        let mut blocked_used = Vec::new();
        let blocked = tformula_copy_def(&mut bank, &formula, 77, &defs, &mut blocked_used).unwrap();
        assert_eq!(blocked.argument(0).as_ref(), Some(&marked));
        assert!(blocked_used.is_empty());
    }

    #[test]
    fn tformula_create_def_builds_quantified_equivalence_over_definition_vars() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -36);
        let left_const = typed_const(&mut bank, "create_def_quant_left");
        let right_const = typed_const(&mut bank, "create_def_quant_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let variable_atom = bool_binary_with_code(&mut bank, eqn_code, &variable, &left_const);
        let ground_atom = bool_binary_with_code(&mut bank, eqn_code, &left_const, &right_const);
        let and_code = bank.signature().and_code();
        let formula = bool_binary_with_code(&mut bank, and_code, &variable_atom, &ground_atom);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();
        let def_atom =
            tformula_def_rename(&mut bank, &formula, 0, &mut defs, &mut renamed_forms).unwrap();

        let definition = tformula_create_def(&mut bank, &def_atom, &formula, 0).unwrap();

        assert_eq!(definition.f_code(), bank.signature().qall_code());
        assert_eq!(definition.argument(0).as_ref(), Some(&variable));
        let body = definition.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().equiv_code());
        assert_eq!(body.argument(0).as_ref(), Some(&def_atom));
        assert_eq!(body.argument(1).as_ref(), Some(&formula));
    }

    #[test]
    fn tformula_create_def_uses_polarity_direction() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "create_def_dir_first");
        let second = typed_const(&mut bank, "create_def_dir_second");
        let third = typed_const(&mut bank, "create_def_dir_third");
        let fourth = typed_const(&mut bank, "create_def_dir_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let and_code = bank.signature().and_code();
        let formula = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();
        let def_atom =
            tformula_def_rename(&mut bank, &formula, 1, &mut defs, &mut renamed_forms).unwrap();

        let positive_definition = tformula_create_def(&mut bank, &def_atom, &formula, 1).unwrap();
        assert_eq!(positive_definition.f_code(), bank.signature().impl_code());
        assert_eq!(positive_definition.argument(0).as_ref(), Some(&def_atom));
        assert_eq!(positive_definition.argument(1).as_ref(), Some(&formula));

        let negative_definition = tformula_create_def(&mut bank, &def_atom, &formula, -1).unwrap();
        assert_eq!(negative_definition.f_code(), bank.signature().impl_code());
        assert_eq!(negative_definition.argument(0).as_ref(), Some(&formula));
        assert_eq!(negative_definition.argument(1).as_ref(), Some(&def_atom));
    }

    #[test]
    fn tformula_mark_polarity_marks_connective_children_like_c() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -37);
        let first = typed_const(&mut bank, "mark_pol_first");
        let second = typed_const(&mut bank, "mark_pol_second");
        let third = typed_const(&mut bank, "mark_pol_third");
        let fourth = typed_const(&mut bank, "mark_pol_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let second_atom = bool_binary_with_code(&mut bank, eqn_code, &second, &third);
        let third_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let fourth_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &fourth);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let not_code = bank.signature().not_code();
        let implication_code = bank.signature().impl_code();
        let existential_code = bank.signature().qex_code();
        let left_inner = bool_binary_with_code(&mut bank, or_code, &first_atom, &second_atom);
        let left = bool_result_unary_with_code(&mut bank, not_code, &left_inner);
        let right_body = bool_binary_with_code(&mut bank, and_code, &third_atom, &fourth_atom);
        let right = bool_binary_with_code(&mut bank, existential_code, &x, &right_body);
        let formula = bool_binary_with_code(&mut bank, implication_code, &left, &right);

        tformula_mark_polarity(&bank, &formula, 1);

        assert_eq!(tformula_decode_polarity(&formula), 1);
        assert_eq!(tformula_decode_polarity(&left), -1);
        assert_eq!(tformula_decode_polarity(&left_inner), 1);
        assert_eq!(tformula_decode_polarity(&right), 1);
        assert_eq!(tformula_decode_polarity(&right_body), 1);
        for literal in [first_atom, second_atom, third_atom, fourth_atom] {
            assert!(!literal.query_prop(TP_POS_POLARITY));
            assert!(!literal.query_prop(TP_NEG_POLARITY));
        }
    }

    #[test]
    fn tformula_decode_polarity_decodes_all_marker_states() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "decode_pol_first");
        let second = typed_const(&mut bank, "decode_pol_second");
        let third = typed_const(&mut bank, "decode_pol_third");
        let fourth = typed_const(&mut bank, "decode_pol_fourth");
        let fifth = typed_const(&mut bank, "decode_pol_fifth");
        let sixth = typed_const(&mut bank, "decode_pol_sixth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let second_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let third_atom = bool_binary_with_code(&mut bank, eqn_code, &fifth, &sixth);
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let implication_code = bank.signature().impl_code();
        let positive = bool_binary_with_code(&mut bank, or_code, &first_atom, &second_atom);
        let negative = bool_binary_with_code(&mut bank, and_code, &second_atom, &third_atom);
        let both = bool_binary_with_code(&mut bank, implication_code, &first_atom, &third_atom);

        positive.set_prop(TP_POS_POLARITY);
        negative.set_prop(TP_NEG_POLARITY);
        both.set_prop(TP_POS_POLARITY | TP_NEG_POLARITY);

        assert_eq!(tformula_decode_polarity(&positive), 1);
        assert_eq!(tformula_decode_polarity(&negative), -1);
        assert_eq!(tformula_decode_polarity(&both), 0);
    }

    #[test]
    fn tformula_nnf_pushes_negation_through_disjunction_to_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "nnf_demorgan_a");
        let b = typed_const(&mut bank, "nnf_demorgan_b");
        let c = typed_const(&mut bank, "nnf_demorgan_c");
        let d = typed_const(&mut bank, "nnf_demorgan_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let or_code = bank.signature().or_code();
        let not_code = bank.signature().not_code();
        let disjunction = bool_binary_with_code(&mut bank, or_code, &left, &right);
        let negated = bool_result_unary_with_code(&mut bank, not_code, &disjunction);

        let nnf = tformula_nnf(&mut bank, &negated, 1).unwrap();

        assert_eq!(nnf.f_code(), bank.signature().and_code());
        let left_neg = nnf.argument(0).unwrap();
        let right_neg = nnf.argument(1).unwrap();
        assert_eq!(left_neg.f_code(), neqn_code);
        assert_eq!(right_neg.f_code(), neqn_code);
        assert_eq!(left_neg.argument(0).as_ref(), Some(&a));
        assert_eq!(left_neg.argument(1).as_ref(), Some(&b));
        assert_eq!(right_neg.argument(0).as_ref(), Some(&c));
        assert_eq!(right_neg.argument(1).as_ref(), Some(&d));
    }

    #[test]
    fn tformula_nnf_pushes_negation_through_universal_quantifier() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -37);
        let a = typed_const(&mut bank, "nnf_quant_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qall_code = bank.signature().qall_code();
        let qex_code = bank.signature().qex_code();
        let not_code = bank.signature().not_code();
        let universal = bool_binary_with_code(&mut bank, qall_code, &x, &atom);
        let negated = bool_result_unary_with_code(&mut bank, not_code, &universal);

        let nnf = tformula_nnf(&mut bank, &negated, 1).unwrap();

        assert_eq!(nnf.f_code(), qex_code);
        assert_eq!(nnf.argument(0).as_ref(), Some(&x));
        let body = nnf.argument(1).unwrap();
        assert_eq!(body.f_code(), neqn_code);
        assert_eq!(body.argument(0).as_ref(), Some(&x));
        assert_eq!(body.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn tformula_nnf_expands_positive_equivalence_by_polarity() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "nnf_equiv_pos_a");
        let b = typed_const(&mut bank, "nnf_equiv_pos_b");
        let c = typed_const(&mut bank, "nnf_equiv_pos_c");
        let d = typed_const(&mut bank, "nnf_equiv_pos_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let equiv_code = bank.signature().equiv_code();
        let equiv = bool_binary_with_code(&mut bank, equiv_code, &left, &right);

        let nnf = tformula_nnf(&mut bank, &equiv, 1).unwrap();

        assert_eq!(nnf.f_code(), bank.signature().and_code());
        let first_direction = nnf.argument(0).unwrap();
        let second_direction = nnf.argument(1).unwrap();
        assert_eq!(first_direction.f_code(), bank.signature().or_code());
        assert_eq!(second_direction.f_code(), bank.signature().or_code());
        assert_eq!(first_direction.argument(0).unwrap().f_code(), neqn_code);
        assert_eq!(first_direction.argument(1).as_ref(), Some(&right));
        assert_eq!(second_direction.argument(0).unwrap().f_code(), neqn_code);
        assert_eq!(second_direction.argument(1).as_ref(), Some(&left));
    }

    #[test]
    fn tformula_nnf_expands_negative_equivalence_by_polarity() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "nnf_equiv_neg_a");
        let b = typed_const(&mut bank, "nnf_equiv_neg_b");
        let c = typed_const(&mut bank, "nnf_equiv_neg_c");
        let d = typed_const(&mut bank, "nnf_equiv_neg_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let equiv_code = bank.signature().equiv_code();
        let equiv = bool_binary_with_code(&mut bank, equiv_code, &left, &right);

        let nnf = tformula_nnf(&mut bank, &equiv, -1).unwrap();

        assert_eq!(nnf.f_code(), bank.signature().or_code());
        let positive_pair = nnf.argument(0).unwrap();
        let negative_pair = nnf.argument(1).unwrap();
        assert_eq!(positive_pair.f_code(), bank.signature().and_code());
        assert_eq!(positive_pair.argument(0).as_ref(), Some(&left));
        assert_eq!(positive_pair.argument(1).as_ref(), Some(&right));
        assert_eq!(negative_pair.f_code(), bank.signature().and_code());
        assert_eq!(negative_pair.argument(0).unwrap().f_code(), neqn_code);
        assert_eq!(negative_pair.argument(1).unwrap().f_code(), neqn_code);
    }

    #[test]
    fn tformula_nnf_encodes_applied_free_variable_as_truth_equality() {
        let mut bank = test_bank();
        let predicate = predicate_var(&mut bank, -146);
        let arg = typed_const(&mut bank, "nnf_pred_arg");
        let applied = apply_many(&mut bank, &predicate, std::slice::from_ref(&arg));

        let nnf = tformula_nnf(&mut bank, &applied, 1).unwrap();

        assert_eq!(nnf.f_code(), bank.signature().eqn_code());
        assert_eq!(nnf.argument(0).as_ref(), Some(&applied));
        assert_eq!(nnf.argument(1).as_ref(), Some(bank.true_term()));
    }

    #[test]
    fn tformula_mini_scope_moves_quantifier_to_branch_containing_variable() {
        let mut bank = test_bank();
        let var_x = typed_var(&bank, -136);
        let const_a = typed_const(&mut bank, "miniscope_move_a");
        let const_b = typed_const(&mut bank, "miniscope_move_b");
        let const_c = typed_const(&mut bank, "miniscope_move_c");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &const_a, &const_b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_c);
        let and_code = bank.signature().and_code();
        let qall_code = bank.signature().qall_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left, &right);
        let quantified = bool_binary_with_code(&mut bank, qall_code, &var_x, &body);

        let scoped = tformula_mini_scope(&mut bank, &quantified).unwrap();

        assert_eq!(scoped.f_code(), and_code);
        assert_eq!(scoped.argument(0).as_ref(), Some(&left));
        let scoped_right = scoped.argument(1).unwrap();
        assert_eq!(scoped_right.f_code(), qall_code);
        assert_eq!(scoped_right.argument(0).as_ref(), Some(&var_x));
        assert_eq!(scoped_right.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_mini_scope_splits_universal_over_conjunction() {
        let mut bank = test_bank();
        let var_x = typed_var(&bank, -138);
        let const_a = typed_const(&mut bank, "miniscope_split_all_a");
        let const_b = typed_const(&mut bank, "miniscope_split_all_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_a);
        let right = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_b);
        let and_code = bank.signature().and_code();
        let qall_code = bank.signature().qall_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left, &right);
        let quantified = bool_binary_with_code(&mut bank, qall_code, &var_x, &body);

        let scoped = tformula_mini_scope(&mut bank, &quantified).unwrap();

        assert_eq!(scoped.f_code(), and_code);
        let scoped_left = scoped.argument(0).unwrap();
        let scoped_right = scoped.argument(1).unwrap();
        assert_eq!(scoped_left.f_code(), qall_code);
        assert_eq!(scoped_left.argument(0).as_ref(), Some(&var_x));
        assert_eq!(scoped_left.argument(1).as_ref(), Some(&left));
        assert_eq!(scoped_right.f_code(), qall_code);
        assert_eq!(scoped_right.argument(0).as_ref(), Some(&var_x));
        assert_eq!(scoped_right.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_mini_scope_splits_existential_over_disjunction() {
        let mut bank = test_bank();
        let var_x = typed_var(&bank, -140);
        let const_a = typed_const(&mut bank, "miniscope_split_ex_a");
        let const_b = typed_const(&mut bank, "miniscope_split_ex_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_a);
        let right = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_b);
        let or_code = bank.signature().or_code();
        let qex_code = bank.signature().qex_code();
        let body = bool_binary_with_code(&mut bank, or_code, &left, &right);
        let quantified = bool_binary_with_code(&mut bank, qex_code, &var_x, &body);

        let scoped = tformula_mini_scope(&mut bank, &quantified).unwrap();

        assert_eq!(scoped.f_code(), or_code);
        let scoped_left = scoped.argument(0).unwrap();
        let scoped_right = scoped.argument(1).unwrap();
        assert_eq!(scoped_left.f_code(), qex_code);
        assert_eq!(scoped_left.argument(1).as_ref(), Some(&left));
        assert_eq!(scoped_right.f_code(), qex_code);
        assert_eq!(scoped_right.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_mini_scope_respects_nested_quantifier_shadowing() {
        let mut bank = test_bank();
        let var_x = typed_var(&bank, -148);
        let const_a = typed_const(&mut bank, "miniscope_shadow_a");
        let const_b = typed_const(&mut bank, "miniscope_shadow_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let shadowed_atom = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_a);
        let free_atom = bool_binary_with_code(&mut bank, eqn_code, &var_x, &const_b);
        let qall_code = bank.signature().qall_code();
        let and_code = bank.signature().and_code();
        let shadowing_quantifier =
            bool_binary_with_code(&mut bank, qall_code, &var_x, &shadowed_atom);
        let body = bool_binary_with_code(&mut bank, and_code, &shadowing_quantifier, &free_atom);
        let quantified = bool_binary_with_code(&mut bank, qall_code, &var_x, &body);

        let scoped = tformula_mini_scope(&mut bank, &quantified).unwrap();

        assert_eq!(scoped.f_code(), and_code);
        assert_eq!(scoped.argument(0).as_ref(), Some(&shadowing_quantifier));
        let scoped_right = scoped.argument(1).unwrap();
        assert_eq!(scoped_right.f_code(), qall_code);
        assert_eq!(scoped_right.argument(1).as_ref(), Some(&free_atom));
    }

    #[test]
    fn tformula_mini_scope3_leaves_too_large_candidate_unchanged() {
        let mut bank = test_bank();
        let universal_var = typed_var(&bank, -149);
        let existential_var = typed_var(&bank, -151);
        let left_const = typed_const(&mut bank, "miniscope3_large_a");
        let right_const = typed_const(&mut bank, "miniscope3_large_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let universal_atom =
            bool_binary_with_code(&mut bank, eqn_code, &universal_var, &left_const);
        let existential_atom =
            bool_binary_with_code(&mut bank, eqn_code, &existential_var, &right_const);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let or_code = bank.signature().or_code();
        let existential =
            bool_binary_with_code(&mut bank, qex_code, &existential_var, &existential_atom);
        let body = bool_binary_with_code(&mut bank, or_code, &universal_atom, &existential);
        let formula = bool_binary_with_code(&mut bank, qall_code, &universal_var, &body);

        let scoped = tformula_mini_scope3(&mut bank, &formula, 8).unwrap();

        assert_eq!(scoped, formula);
        assert!(formula.binding().is_none());
    }

    #[test]
    fn tformula_mini_scope3_replaces_small_universal_existential_candidate() {
        let mut bank = test_bank();
        let universal_var = typed_var(&bank, -153);
        let existential_var = typed_var(&bank, -155);
        let left_const = typed_const(&mut bank, "miniscope3_small_a");
        let right_const = typed_const(&mut bank, "miniscope3_small_b");
        let other_left = typed_const(&mut bank, "miniscope3_small_c");
        let other_right = typed_const(&mut bank, "miniscope3_small_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let universal_atom =
            bool_binary_with_code(&mut bank, eqn_code, &universal_var, &left_const);
        let existential_atom =
            bool_binary_with_code(&mut bank, eqn_code, &existential_var, &right_const);
        let other = bool_binary_with_code(&mut bank, eqn_code, &other_left, &other_right);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let or_code = bank.signature().or_code();
        let and_code = bank.signature().and_code();
        let existential =
            bool_binary_with_code(&mut bank, qex_code, &existential_var, &existential_atom);
        let candidate_body =
            bool_binary_with_code(&mut bank, or_code, &universal_atom, &existential);
        let candidate =
            bool_binary_with_code(&mut bank, qall_code, &universal_var, &candidate_body);
        let formula = bool_binary_with_code(&mut bank, and_code, &candidate, &other);

        let scoped = tformula_mini_scope3(&mut bank, &formula, 9).unwrap();

        assert_eq!(scoped.f_code(), and_code);
        assert_eq!(scoped.argument(1).as_ref(), Some(&other));
        let scoped_candidate = scoped.argument(0).unwrap();
        assert_eq!(scoped_candidate.f_code(), or_code);
        let scoped_left = scoped_candidate.argument(0).unwrap();
        assert_eq!(scoped_left.f_code(), qall_code);
        assert_eq!(scoped_left.argument(0).as_ref(), Some(&universal_var));
        assert_eq!(scoped_left.argument(1).as_ref(), Some(&universal_atom));
        assert_eq!(scoped_candidate.argument(1).as_ref(), Some(&existential));
        assert!(candidate.binding().is_none());
    }

    #[test]
    fn tformula_distribute_disjunctions_distributes_left_conjunction() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "dist_left_a");
        let b = typed_const(&mut bank, "dist_left_b");
        let c = typed_const(&mut bank, "dist_left_c");
        let d = typed_const(&mut bank, "dist_left_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let middle = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &left, &middle);
        let formula = bool_binary_with_code(&mut bank, or_code, &conjunction, &right);

        let distributed = tformula_distribute_disjunctions(&mut bank, &formula).unwrap();

        assert_eq!(distributed.f_code(), and_code);
        let left_or = distributed.argument(0).unwrap();
        let right_or = distributed.argument(1).unwrap();
        assert_eq!(left_or.f_code(), or_code);
        assert_eq!(right_or.f_code(), or_code);
        assert_eq!(left_or.argument(0).as_ref(), Some(&left));
        assert_eq!(left_or.argument(1).as_ref(), Some(&right));
        assert_eq!(right_or.argument(0).as_ref(), Some(&middle));
        assert_eq!(right_or.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn tformula_distribute_disjunctions_distributes_right_conjunction_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "dist_right_a");
        let b = typed_const(&mut bank, "dist_right_b");
        let c = typed_const(&mut bank, "dist_right_c");
        let d = typed_const(&mut bank, "dist_right_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let middle = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &middle, &right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &conjunction);

        let distributed = tformula_distribute_disjunctions(&mut bank, &formula).unwrap();

        assert_eq!(distributed.f_code(), and_code);
        let left_or = distributed.argument(0).unwrap();
        let right_or = distributed.argument(1).unwrap();
        assert_eq!(left_or.f_code(), or_code);
        assert_eq!(right_or.f_code(), or_code);
        assert_eq!(left_or.argument(0).as_ref(), Some(&middle));
        assert_eq!(left_or.argument(1).as_ref(), Some(&left));
        assert_eq!(right_or.argument(0).as_ref(), Some(&right));
        assert_eq!(right_or.argument(1).as_ref(), Some(&left));
    }

    #[test]
    fn tformula_distribute_disjunctions_rebuilds_quantifier_body() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -142);
        let a = typed_const(&mut bank, "dist_quant_a");
        let b = typed_const(&mut bank, "dist_quant_b");
        let c = typed_const(&mut bank, "dist_quant_c");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let middle = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let qall_code = bank.signature().qall_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &left, &middle);
        let body = bool_binary_with_code(&mut bank, or_code, &conjunction, &right);
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        let distributed = tformula_distribute_disjunctions(&mut bank, &quantified).unwrap();

        assert_eq!(distributed.f_code(), qall_code);
        assert_eq!(distributed.argument(0).as_ref(), Some(&x));
        let body = distributed.argument(1).unwrap();
        assert_eq!(body.f_code(), and_code);
        assert_eq!(body.argument(0).unwrap().f_code(), or_code);
        assert_eq!(body.argument(1).unwrap().f_code(), or_code);
    }

    #[test]
    fn tformula_conjunctive_nf_records_nnf_and_distribution_phases() {
        let mut bank = test_bank();
        let antecedent_left = typed_const(&mut bank, "cnf_phase_a");
        let antecedent_right = typed_const(&mut bank, "cnf_phase_b");
        let left_consequent_left = typed_const(&mut bank, "cnf_phase_c");
        let left_consequent_right = typed_const(&mut bank, "cnf_phase_d");
        let right_consequent_left = typed_const(&mut bank, "cnf_phase_e");
        let right_consequent_right = typed_const(&mut bank, "cnf_phase_f");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let antecedent =
            bool_binary_with_code(&mut bank, eqn_code, &antecedent_left, &antecedent_right);
        let consequent_left = bool_binary_with_code(
            &mut bank,
            eqn_code,
            &left_consequent_left,
            &left_consequent_right,
        );
        let consequent_right = bool_binary_with_code(
            &mut bank,
            eqn_code,
            &right_consequent_left,
            &right_consequent_right,
        );
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let implication_code = bank.signature().impl_code();
        let consequent =
            bool_binary_with_code(&mut bank, and_code, &consequent_left, &consequent_right);
        let formula = bool_binary_with_code(&mut bank, implication_code, &antecedent, &consequent);

        let result = tformula_conjunctive_nf(&mut bank, &formula).unwrap();

        assert_eq!(result.derivation_ops(), &[DC_FNNF, DC_DIST_DISJUNCTIONS]);
        let cnf = result.formula();
        assert_eq!(cnf.f_code(), and_code);
        let left_clause = cnf.argument(0).unwrap();
        let right_clause = cnf.argument(1).unwrap();
        assert_eq!(left_clause.f_code(), or_code);
        assert_eq!(right_clause.f_code(), or_code);
        assert_eq!(left_clause.argument(0).as_ref(), Some(&consequent_left));
        assert_eq!(right_clause.argument(0).as_ref(), Some(&consequent_right));
    }

    #[test]
    fn tformula_conjunctive_nf3_seeds_fresh_vars_before_var_rename() {
        let mut bank = test_bank();
        let y = typed_var(&bank, -2);
        let x = typed_var(&bank, -8);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &x, &y);
        let qall_code = bank.signature().qall_code();
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &atom);

        let result = tformula_conjunctive_nf3(&mut bank, &formula, 100, false).unwrap();

        assert_eq!(result.derivation_ops(), &[DC_VAR_RENAME]);
        let renamed = result.formula();
        assert_eq!(renamed.f_code(), qall_code);
        let fresh = renamed.argument(0).unwrap();
        assert_ne!(fresh, x);
        assert_ne!(fresh, y);
        assert!(fresh.f_code() < y.f_code());
        let body = renamed.argument(1).unwrap();
        assert_eq!(body.argument(0).as_ref(), Some(&fresh));
        assert_eq!(body.argument(1).as_ref(), Some(&y));
    }

    #[test]
    fn tformula_conjunctive_nf3_records_fool_unroll_and_followup_nnf() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "cnf_fool_a");
        let b = typed_const(&mut bank, "cnf_fool_b");
        let c = typed_const(&mut bank, "cnf_fool_c");
        let d = typed_const(&mut bank, "cnf_fool_d");
        let target = typed_const(&mut bank, "cnf_fool_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let bool_subformula = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let applied = default_bool_arg_binary(&mut bank, "cnf_fool_fun", &a, &bool_subformula);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);

        let result = tformula_conjunctive_nf3(&mut bank, &formula, 100, true).unwrap();

        assert_eq!(
            result.derivation_ops(),
            &[DC_FOOL_UNROLL, DC_FNNF, DC_DIST_DISJUNCTIONS]
        );
        assert_eq!(result.formula().f_code(), and_code);
    }

    #[test]
    fn tformula_conjunctive_nf3_does_not_record_fool_op_for_literal_expansion_only() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "cnf_expand_only_a");
        let b = typed_const(&mut bank, "cnf_expand_only_b");
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let disequality = bool_binary_with_code(&mut bank, neqn_code, &a, &b);

        let result = tformula_conjunctive_nf3(&mut bank, &disequality, 100, true).unwrap();

        assert_eq!(result.formula(), &disequality);
        assert_eq!(result.derivation_ops(), &[DC_FNNF]);
    }

    #[test]
    fn tformula_lit_alloc_ho_decodes_formula_literal() {
        let mut bank = test_bank();
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();
        let equiv_code = bank.signature().equiv_code();
        let encoded = bool_binary_with_code(&mut bank, equiv_code, &true_term, &false_term);
        let literal = literal(&mut bank, &encoded, &true_term, false);

        let formula = tformula_lit_alloc(&mut bank, &literal, ProblemType::HigherOrder).unwrap();

        assert_eq!(formula.f_code(), bank.signature().not_code());
        let decoded = formula.argument(0).unwrap();
        assert_eq!(decoded.f_code(), equiv_code);
        assert_ne!(decoded.argument(0).unwrap(), true_term);
        assert_ne!(decoded.argument(1).unwrap(), false_term);
    }

    #[test]
    fn tformula_tstp_parse_wrapper_uses_term_bank_formula_parser() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("parse_wrap_left = parse_wrap_right", false).unwrap();

        let formula = tformula_tstp_parse(&mut scanner, &mut bank).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(0).unwrap().f_code()),
            Some("parse_wrap_left")
        );
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(1).unwrap().f_code()),
            Some("parse_wrap_right")
        );
    }

    #[test]
    fn tformula_tptp_parse_wrapper_uses_term_bank_formula_parser() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("tptp_wrap_left|tptp_wrap_right", false).unwrap();

        let formula = tformula_tptp_parse(&mut scanner, &mut bank).unwrap();

        assert_eq!(formula.f_code(), bank.signature().or_code());
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(0).unwrap().argument(0).unwrap().f_code()),
            Some("tptp_wrap_left")
        );
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(1).unwrap().argument(0).unwrap().f_code()),
            Some("tptp_wrap_right")
        );
    }

    #[test]
    fn tcf_tstp_parse_folds_unquantified_clause_literals() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("(tcf_parse_p(a)|~tcf_parse_q(b))", false).unwrap();
        scanner.set_format(IoFormat::Tstp);

        let formula = tcf_tstp_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(formula.f_code(), bank.signature().or_code());
        let left = formula.argument(0).unwrap();
        let right = formula.argument(1).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        assert_eq!(right.f_code(), bank.signature().neqn_code());
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("tcf_parse_p")
        );
        assert_eq!(
            bank.signature()
                .find_name(right.argument(0).unwrap().f_code()),
            Some("tcf_parse_q")
        );
    }

    #[test]
    fn tcf_tstp_parse_accepts_universal_formula_prefix() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("![X]:(tcf_quant_p(X)|tcf_quant_q(X))", false).unwrap();
        scanner.set_format(IoFormat::Tstp);

        let formula = tcf_tstp_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(formula.f_code(), bank.signature().qall_code());
        let body = formula.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().or_code());
    }

    #[test]
    fn tcf_tstp_parse_nests_comma_separated_universal_variables() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("![X,Y]:(tcf_quant2_p(X)|tcf_quant2_q(Y))", false).unwrap();
        scanner.set_format(IoFormat::Tstp);

        let formula = tcf_tstp_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap();

        assert_eq!(formula.f_code(), bank.signature().qall_code());
        let inner = formula.argument(1).unwrap();
        assert_eq!(inner.f_code(), bank.signature().qall_code());
        let body = inner.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().or_code());
        assert_eq!(
            bank.signature()
                .find_name(body.argument(0).unwrap().argument(0).unwrap().f_code()),
            Some("tcf_quant2_p")
        );
        assert_eq!(
            bank.signature()
                .find_name(body.argument(1).unwrap().argument(0).unwrap().f_code()),
            Some("tcf_quant2_q")
        );
    }

    #[test]
    fn tcf_tstp_parse_rejects_parenthesized_non_clause_quantified_body() {
        let mut bank = test_bank();
        let mut scanner =
            Scanner::from_user_string("![X]:(tcf_bad_p(X)&tcf_bad_q(X))", false).unwrap();
        scanner.set_format(IoFormat::Tstp);

        let error = tcf_tstp_parse(&mut scanner, &mut bank, ProblemType::FirstOrder).unwrap_err();

        assert_eq!(error.code(), crate::basics::error::ErrorCode::SYNTAX_ERROR);
    }

    #[test]
    fn tformula_macro_wrappers_use_identity_and_term_bank_copying() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -294);
        let a = typed_const(&mut bank, "formula_macro_a");
        let b = typed_const(&mut bank, "formula_macro_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &x, &b);
        let unshared_same_shape = Term::top_alloc(eqn_code, 2);
        unshared_same_shape.set_type(formula.type_());
        unshared_same_shape.set_argument(0, x.clone());
        unshared_same_shape.set_argument(1, b.clone());

        assert!(tformula_equal(&formula, &formula));
        assert!(!tformula_equal(&formula, &unshared_same_shape));
        assert_eq!(tformula_find_max_var_code(&formula), -294);

        x.set_binding(Some(a.clone()));
        let copied = tformula_copy(&mut bank, &formula).unwrap();
        x.set_binding(None);

        assert!(!tformula_equal(&formula, &copied));
        assert_eq!(copied.f_code(), eqn_code);
        assert_eq!(copied.argument(0).as_ref(), Some(&a));
        assert_eq!(copied.argument(1).as_ref(), Some(&b));
        assert_eq!(tformula_find_max_var_code(&copied), 0);
    }

    #[test]
    fn tformula_gc_mark_cells_marks_reachable_formula_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "formula_gc_a");
        let b = typed_const(&mut bank, "formula_gc_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);

        assert!(!formula.query_prop(TP_GARBAGE_FLAG));
        assert!(!a.query_prop(TP_GARBAGE_FLAG));
        assert!(!b.query_prop(TP_GARBAGE_FLAG));

        tformula_gc_mark_cells(&bank, &formula);

        assert!(formula.query_prop(TP_GARBAGE_FLAG));
        assert!(a.query_prop(TP_GARBAGE_FLAG));
        assert!(b.query_prop(TP_GARBAGE_FLAG));
    }

    #[test]
    fn tformula_predicate_wrappers_match_header_macros() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -296);
        let a = typed_const(&mut bank, "formula_pred_a");
        let b = typed_const(&mut bank, "formula_pred_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let not_code = bank.signature().not_code();
        let negated = tformula_fcode_alloc(&mut bank, not_code, equality.clone(), None).unwrap();
        let and_code = bank.signature().and_code();
        let conjunction = tformula_fcode_alloc(
            &mut bank,
            and_code,
            equality.clone(),
            Some(equality.clone()),
        )
        .unwrap();
        let qex_code = bank.signature().qex_code();
        let existential = tformula_quantor_alloc(&mut bank, qex_code, &x, &equality).unwrap();
        let lambda =
            tformula_quantor_alloc(&mut bank, SIG_NAMED_LAMBDA_CODE, &x, &equality).unwrap();

        assert!(tformula_has_subform1(&bank, &negated));
        assert!(!tformula_has_subform2(&bank, &negated));
        assert!(tformula_has_subform2(&bank, &conjunction));
        assert!(tformula_is_unary(&negated));
        assert!(!tformula_is_binary(&negated));
        assert!(tformula_is_binary(&equality));
        assert!(tformula_is_literal(&bank, &equality));
        assert!(!tformula_is_literal(&bank, &negated));
        assert!(tformula_is_quantified(&bank, &existential));
        assert!(tformula_is_quantified_nl(&bank, &existential));
        assert!(tformula_is_quantified(&bank, &lambda));
        assert!(!tformula_is_quantified_nl(&bank, &lambda));
    }

    #[test]
    fn tformula_complex_bool_preserves_c_type_macro_artifact() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "formula_complex_bool_a");
        let b = typed_const(&mut bank, "formula_complex_bool_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();

        assert!(true_term.type_().is_some_and(|type_| type_.is_bool()));
        assert!(false_term.type_().is_some_and(|type_| type_.is_bool()));
        assert!(equality.type_().is_some_and(|type_| type_.is_bool()));

        assert!(tformula_is_complex_bool(&bank, &true_term));
        assert!(!tformula_is_complex_bool(&bank, &false_term));
        assert!(!tformula_is_complex_bool(&bank, &equality));
    }

    #[test]
    fn tformula_clause_encode_handles_empty_and_folds_literals() {
        let mut bank = test_bank();
        let empty = Clause::alloc(EqnList::new());

        let false_formula =
            tformula_clause_encode(&mut bank, &empty, ProblemType::FirstOrder).unwrap();

        assert_eq!(false_formula.f_code(), bank.signature().neqn_code());
        assert_eq!(false_formula.argument(0).as_ref(), Some(bank.true_term()));
        assert_eq!(false_formula.argument(1).as_ref(), Some(bank.true_term()));

        let a = typed_const(&mut bank, "clause_encode_a");
        let b = typed_const(&mut bank, "clause_encode_b");
        let c = typed_const(&mut bank, "clause_encode_c");
        let d = typed_const(&mut bank, "clause_encode_d");
        let clause = clause_from(vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &c, &d, false),
        ]);

        let encoded = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();

        assert_eq!(encoded.f_code(), bank.signature().or_code());
        let left = encoded.argument(0).unwrap();
        let right = encoded.argument(1).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        assert_eq!(left.argument(0).as_ref(), Some(&a));
        assert_eq!(left.argument(1).as_ref(), Some(&b));
        assert_eq!(right.f_code(), bank.signature().neqn_code());
        assert_eq!(right.argument(0).as_ref(), Some(&c));
        assert_eq!(right.argument(1).as_ref(), Some(&d));
    }

    #[test]
    fn tformula_free_var_helpers_respect_quantifier_binding() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -300);
        let y = typed_var(&bank, -302);
        let a = typed_const(&mut bank, "free_vars_a");
        let b = typed_const(&mut bank, "free_vars_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &y, &b);
        let and_code = bank.signature().and_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let qall_code = bank.signature().qall_code();
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        let free_vars = tformula_collect_free_vars(&bank, &quantified);

        assert_eq!(free_vars, vec![y.clone()]);
        assert_eq!(tformula_has_free_vars(&bank, &quantified), Some(y.clone()));
        assert!(!tformula_is_closed(&bank, &quantified));

        let closed = tformula_add_quantor(&mut bank, &quantified, true, &y).unwrap();

        assert!(tformula_is_closed(&bank, &closed));
    }

    #[test]
    fn tformula_var_is_free_matches_direct_quantifier_query() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -304);
        let y = typed_var(&bank, -306);
        let a = typed_const(&mut bank, "direct_free_a");
        let b = typed_const(&mut bank, "direct_free_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &y, &b);
        let and_code = bank.signature().and_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let qall_code = bank.signature().qall_code();
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        assert!(tformula_var_is_free(&bank, &body, &x));
        assert!(!tformula_var_is_free(&bank, &quantified, &x));
        assert!(tformula_var_is_free(&bank, &quantified, &y));
        assert!(!tformula_var_is_free(&bank, &left_atom, &a));
        assert_eq!(
            tformula_var_is_free_cached(&bank, &quantified, &y),
            tformula_var_is_free(&bank, &quantified, &y)
        );
    }

    #[test]
    fn tformula_var_is_free_treats_named_lambda_binder_as_child() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -308);
        let a = typed_const(&mut bank, "direct_free_lambda_a");
        let b = typed_const(&mut bank, "direct_free_lambda_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let lambda = tformula_quantor_alloc(&mut bank, SIG_NAMED_LAMBDA_CODE, &x, &body).unwrap();

        assert!(tformula_var_is_free(&bank, &lambda, &x));
        assert!(tformula_var_is_free_cached(&bank, &lambda, &x));
        assert!(tformula_collect_free_vars(&bank, &lambda).is_empty());
    }

    #[test]
    fn tformula_closure_wraps_free_variables_with_requested_quantifier() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -310);
        let y = typed_var(&bank, -312);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &x, &y);
        let expected_vars = tformula_collect_free_vars(&bank, &formula);
        let qex_code = bank.signature().qex_code();

        let closure = tformula_closure(&mut bank, &formula, false).unwrap();

        assert_eq!(expected_vars.len(), 2);
        assert!(tformula_is_closed(&bank, &closure));
        let mut current = closure.clone();
        for expected_var in expected_vars.iter().rev() {
            assert_eq!(current.f_code(), qex_code);
            assert_eq!(current.argument(0).as_ref(), Some(expected_var));
            current = current.argument(1).unwrap();
        }
        assert_eq!(current, formula);
        assert!(!tformula_is_closed(&bank, &current));
    }

    #[test]
    fn tformula_clause_closed_encode_adds_universal_closure() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -320);
        let a = typed_const(&mut bank, "closed_encode_a");
        let clause = clause_from(vec![literal(&mut bank, &x, &a, true)]);

        let unclosed = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let closed =
            tformula_clause_closed_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();

        assert!(!tformula_is_closed(&bank, &unclosed));
        assert!(tformula_is_closed(&bank, &closed));
        assert_eq!(closed.f_code(), bank.signature().qall_code());
        assert_eq!(closed.argument(0).as_ref(), Some(&x));
        let body = closed.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().eqn_code());
        assert_eq!(body.argument(0).as_ref(), Some(&x));
        assert_eq!(body.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn tformula_prop_constant_helpers_use_encoded_truth_literals() {
        let mut bank = test_bank();

        let true_formula = tformula_prop_constant_alloc(&mut bank, true).unwrap();
        let false_formula = tformula_prop_constant_alloc(&mut bank, false).unwrap();

        assert!(tformula_is_prop_true(&bank, &true_formula));
        assert!(tformula_is_prop_const(&bank, &true_formula, true));
        assert!(!tformula_is_prop_false(&bank, &true_formula));
        assert!(tformula_is_prop_false(&bank, &false_formula));
        assert!(tformula_is_prop_const(&bank, &false_formula, false));
        assert!(!tformula_is_prop_true(&bank, &false_formula));
    }

    #[test]
    fn tformula_encode_predicate_as_eqn_wraps_boolean_predicate_shapes() {
        let mut bank = test_bank();
        let bool_variable = bool_var(&bank, -330);

        let encoded_var =
            tformula_encode_predicate_as_eqn(&mut bank, bool_variable.clone()).unwrap();

        assert_eq!(encoded_var.f_code(), bank.signature().eqn_code());
        assert_eq!(encoded_var.argument(0).as_ref(), Some(&bool_variable));
        assert_eq!(encoded_var.argument(1).as_ref(), Some(bank.true_term()));

        let false_term = bank.false_term().clone();
        let encoded_false = tformula_encode_predicate_as_eqn(&mut bank, false_term).unwrap();
        assert!(tformula_is_prop_false(&bank, &encoded_false));

        let plain_term = typed_const(&mut bank, "predicate_encode_plain");
        let encoded_plain =
            tformula_encode_predicate_as_eqn(&mut bank, plain_term.clone()).unwrap();
        assert_eq!(encoded_plain, plain_term);
    }

    #[test]
    fn tformula_negate_toggles_literals_and_wraps_non_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "negate_a");
        let b = typed_const(&mut bank, "negate_b");
        let c = typed_const(&mut bank, "negate_c");
        let d = typed_const(&mut bank, "negate_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);

        let negated_literal = tformula_negate(&mut bank, &left).unwrap();

        assert_eq!(negated_literal.f_code(), bank.signature().neqn_code());
        assert_eq!(negated_literal.argument(0).as_ref(), Some(&a));
        assert_eq!(negated_literal.argument(1).as_ref(), Some(&b));

        let right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let conjunction = bool_binary_with_code(&mut bank, and_code, &left, &right);
        let negated_formula = tformula_negate(&mut bank, &conjunction).unwrap();

        assert_eq!(negated_formula.f_code(), bank.signature().not_code());
        assert_eq!(negated_formula.argument(0).as_ref(), Some(&conjunction));
    }

    #[test]
    fn tformula_quantor_alloc_and_untyped_query_match_term_shape() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -340);
        let a = typed_const(&mut bank, "quantor_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qall_code = bank.signature().qall_code();

        let quantified = tformula_quantor_alloc(&mut bank, qall_code, &x, &atom).unwrap();

        assert_eq!(quantified.f_code(), qall_code);
        assert_eq!(quantified.argument(0).as_ref(), Some(&x));
        assert_eq!(quantified.argument(1).as_ref(), Some(&atom));
        assert!(tformula_is_untyped(&quantified));

        let higher_order = higher_order_var(&mut bank, -342, 1);
        let typed_atom = bool_binary_with_code(&mut bank, eqn_code, &higher_order, &higher_order);
        assert!(!tformula_is_untyped(&typed_atom));
    }

    #[test]
    fn tformula_stack_to_form_pops_like_c() {
        let mut bank = test_bank();
        let first_left = typed_const(&mut bank, "stack_a");
        let first_right = typed_const(&mut bank, "stack_b");
        let second_left = typed_const(&mut bank, "stack_c");
        let second_right = typed_const(&mut bank, "stack_d");
        let third_left = typed_const(&mut bank, "stack_e");
        let third_right = typed_const(&mut bank, "stack_f");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first = bool_binary_with_code(&mut bank, eqn_code, &first_left, &first_right);
        let second = bool_binary_with_code(&mut bank, eqn_code, &second_left, &second_right);
        let third = bool_binary_with_code(&mut bank, eqn_code, &third_left, &third_right);
        let mut stack = vec![first.clone(), second.clone(), third.clone()];
        let or_code = bank.signature().or_code();

        let formula = tformula_stack_to_form(&mut bank, &mut stack, or_code).unwrap();

        assert!(stack.is_empty());
        assert_eq!(formula.f_code(), or_code);
        assert_eq!(formula.argument(0).as_ref(), Some(&first));
        let right = formula.argument(1).unwrap();
        assert_eq!(right.f_code(), or_code);
        assert_eq!(right.argument(0).as_ref(), Some(&second));
        assert_eq!(right.argument(1).as_ref(), Some(&third));

        let mut empty = Vec::new();
        let truth = tformula_stack_to_form(&mut bank, &mut empty, or_code).unwrap();
        assert_eq!(&truth, bank.true_term());
    }

    #[test]
    fn tformula_expand_distinct_builds_pairwise_disequality_chain() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "distinct_a");
        let b = typed_const(&mut bank, "distinct_b");
        let c = typed_const(&mut bank, "distinct_c");
        let distinct = distinct_formula(&mut bank, &[a.clone(), b.clone(), c.clone()]);

        let expanded = tformula_expand_distinct(&mut bank, &distinct).unwrap();

        assert_eq!(expanded.f_code(), bank.signature().and_code());
        let first_pair = expanded.argument(0).unwrap();
        assert_eq!(first_pair.f_code(), bank.signature().neqn_code());
        assert_eq!(first_pair.argument(0).as_ref(), Some(&a));
        assert_eq!(first_pair.argument(1).as_ref(), Some(&b));
        let tail = expanded.argument(1).unwrap();
        assert_eq!(tail.f_code(), bank.signature().and_code());
        let second_pair = tail.argument(0).unwrap();
        assert_eq!(second_pair.f_code(), bank.signature().neqn_code());
        assert_eq!(second_pair.argument(0).as_ref(), Some(&a));
        assert_eq!(second_pair.argument(1).as_ref(), Some(&c));
        let third_pair = tail.argument(1).unwrap();
        assert_eq!(third_pair.f_code(), bank.signature().neqn_code());
        assert_eq!(third_pair.argument(0).as_ref(), Some(&b));
        assert_eq!(third_pair.argument(1).as_ref(), Some(&c));

        let singleton = distinct_formula(&mut bank, &[a]);
        let expanded_singleton = tformula_expand_distinct(&mut bank, &singleton).unwrap();
        assert_eq!(&expanded_singleton, bank.true_term());
    }

    #[test]
    fn tformula_app_encode_renders_literals_and_left_or_chain_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "app_form_a");
        let b = typed_const(&mut bank, "app_form_b");
        let c = typed_const(&mut bank, "app_form_c");
        let f_code = typed_binary_code(&mut bank, "app_form_f");
        let f_ab = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let first = bool_binary_with_code(&mut bank, eqn_code, &f_ab, &a);
        let second = bool_binary_with_code(&mut bank, neqn_code, &a, &b);
        let third = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let or_code = bank.signature().or_code();
        let left = bool_binary_with_code(&mut bank, or_code, &first, &second);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &third);

        let rendered = tformula_app_encode_string(&mut bank, &formula).unwrap();

        let first_lit = literal(&mut bank, &f_ab, &a, true);
        let first_expected = eqn_app_encode_string(&mut bank, &first_lit, false).unwrap();
        let second_lit = literal(&mut bank, &a, &b, true);
        let second_expected = eqn_app_encode_string(&mut bank, &second_lit, true).unwrap();
        let third_lit = literal(&mut bank, &b, &c, true);
        let third_expected = eqn_app_encode_string(&mut bank, &third_lit, false).unwrap();
        assert_eq!(
            rendered,
            format!("({first_expected}|{second_expected}|{third_expected})")
        );
        assert_eq!(
            bank.term_string(&f_ab, true),
            "app_form_f(app_form_a,app_form_b)"
        );
    }

    #[test]
    fn tformula_app_encode_coalesces_repeated_quantifiers() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "app_quant_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qall_code = bank.signature().qall_code();
        let inner = tformula_quantor_alloc(&mut bank, qall_code, &y, &body).unwrap();
        let outer = tformula_quantor_alloc(&mut bank, qall_code, &x, &inner).unwrap();

        let rendered = tformula_app_encode_string(&mut bank, &outer).unwrap();

        let x_name = bank.term_string(&x, true);
        let y_name = bank.term_string(&y, true);
        assert_eq!(
            rendered,
            format!("![{x_name}:$i, {y_name}:$i]:{x_name}=app_quant_a")
        );
    }

    #[test]
    fn tformula_app_encode_renders_fool_formula_and_term_positions() {
        let mut bank = test_bank();
        let value_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    value_type.clone(),
                    bool_type.clone(),
                ]));
        let p_code = bank.signature_mut().insert_id("app_fool_p", 1, false);
        bank.signature_mut()
            .declare_final_type(p_code, predicate_type)
            .unwrap();
        let f_code = bank.signature_mut().insert_id("app_fool_f", 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type)
            .unwrap();

        let a = typed_const(&mut bank, "app_fool_a");
        let b = typed_const(&mut bank, "app_fool_b");
        let c = typed_const(&mut bank, "app_fool_c");
        let atom = bool_result_unary_with_code(&mut bank, p_code, &a);
        let bool_ite = bool_ite(&mut bank, &atom, &atom, &atom);
        let term_ite = typed_ite(&mut bank, &atom, &a, &b);
        let f = bank.create_const_term(f_code).unwrap();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let definition = bool_binary_with_code(&mut bank, eqn_code, &f, &atom);
        let let_formula = let_term(&mut bank, &[definition], &f);
        let true_term = bank.true_term().clone();

        let bool_ite_lit = literal(&mut bank, &bool_ite, &true_term, true);
        let bool_ite_formula =
            tformula_lit_alloc(&mut bank, &bool_ite_lit, ProblemType::FirstOrder).unwrap();
        let term_ite_lit = literal(&mut bank, &term_ite, &c, true);
        let term_ite_formula =
            tformula_lit_alloc(&mut bank, &term_ite_lit, ProblemType::FirstOrder).unwrap();
        let let_lit = literal(&mut bank, &let_formula, &true_term, true);
        let let_formula = tformula_lit_alloc(&mut bank, &let_lit, ProblemType::FirstOrder).unwrap();

        let bool_ite_rendered = tformula_app_encode_string(&mut bank, &bool_ite_formula).unwrap();
        let term_ite_rendered = tformula_app_encode_string(&mut bank, &term_ite_formula).unwrap();
        let let_rendered = tformula_app_encode_string(&mut bank, &let_formula).unwrap();

        assert!(bool_ite_rendered.starts_with("$ite("));
        assert!(!bool_ite_rendered.contains("=$true"));
        assert!(term_ite_rendered.contains("$ite("));
        assert!(term_ite_rendered.contains("=app_fool_c"));
        assert!(let_rendered.starts_with("$let("));
        assert!(let_rendered.contains(":="));
    }

    #[test]
    fn tformula_preload_types_creates_typed_application_symbols() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "preload_a");
        let b = typed_const(&mut bank, "preload_b");
        let f_code = typed_binary_code(&mut bank, "preload_f");
        let declared_f_type = bank
            .signature()
            .get_type(f_code)
            .expect("fixture declares function type")
            .clone();
        let f_ab = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &f_ab, &a);
        let before_types = bank.signature().type_bank().types_count();

        tformula_preload_types(&mut bank, &formula).unwrap();

        assert!(bank.signature().type_bank().types_count() > before_types);
        let individual = bank.signature().type_bank().default_type();
        let f_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(declared_f_type);
        let prefix_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                ]));
        let inner_app = format!(
            "app_{}_{}_{}",
            f_type.type_uid(),
            individual.type_uid(),
            prefix_type.type_uid()
        );
        let outer_app = format!(
            "app_{}_{}_{}",
            prefix_type.type_uid(),
            individual.type_uid(),
            individual.type_uid()
        );
        assert_ne!(bank.signature().find_f_code(&inner_app), 0);
        assert_ne!(bank.signature().find_f_code(&outer_app), 0);
    }

    #[test]
    fn tformula_tptp_print_renders_literals_and_left_or_chain_like_c() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "print_form_a");
        let b = typed_const(&mut bank, "print_form_b");
        let c = typed_const(&mut bank, "print_form_c");
        let f_code = typed_binary_code(&mut bank, "print_form_f");
        let f_ab = typed_binary_with_code(&mut bank, f_code, &a, &b);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let first = bool_binary_with_code(&mut bank, eqn_code, &f_ab, &a);
        let second = bool_binary_with_code(&mut bank, neqn_code, &a, &b);
        let third = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let or_code = bank.signature().or_code();
        let left = bool_binary_with_code(&mut bank, or_code, &first, &second);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &third);

        let rendered = tformula_tptp_string(
            &mut bank,
            &formula,
            true,
            TFormulaTptpPrintOptions::tstp(ProblemType::FirstOrder),
        )
        .unwrap();

        assert_eq!(
            rendered,
            "(print_form_f(print_form_a,print_form_b)=print_form_a|\
             print_form_a!=print_form_b|print_form_b=print_form_c)"
        );
    }

    #[test]
    fn tformula_tptp_print_coalesces_quantifiers_and_prints_ho_types() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "print_quant_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qall_code = bank.signature().qall_code();
        let inner = tformula_quantor_alloc(&mut bank, qall_code, &y, &body).unwrap();
        let outer = tformula_quantor_alloc(&mut bank, qall_code, &x, &inner).unwrap();

        let first_order = tformula_tptp_string(
            &mut bank,
            &outer,
            true,
            TFormulaTptpPrintOptions::tstp(ProblemType::FirstOrder),
        )
        .unwrap();
        let higher_order = tformula_tptp_string(
            &mut bank,
            &outer,
            true,
            TFormulaTptpPrintOptions::tstp(ProblemType::HigherOrder),
        )
        .unwrap();

        assert_eq!(first_order, "![X1, X2]:(X1=print_quant_a)");
        assert_eq!(higher_order, "![X1:$i, X2:$i]:(X1=print_quant_a)");
    }

    #[test]
    fn tformula_collect_clause_preserves_order_and_handles_truth_constants() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "collect_a");
        let b = typed_const(&mut bank, "collect_b");
        let c = typed_const(&mut bank, "collect_c");
        let d = typed_const(&mut bank, "collect_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let second = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let true_term = bank.true_term().clone();
        let false_term = bank.false_term().clone();
        let or_code = bank.signature().or_code();
        let left = bool_binary_with_code(&mut bank, or_code, &first, &false_term);
        let right = bool_binary_with_code(&mut bank, or_code, &true_term, &second);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right);

        let clause = tformula_collect_clause(&mut bank, &formula, None).unwrap();

        assert_eq!(clause.literal_number(), 3);
        assert_eq!(clause.weight(), clause.standard_weight());
        let literals = clause.literals().as_slice();
        assert_eq!(literals[0].left(), &a);
        assert_eq!(literals[0].right(), &b);
        assert!(literals[0].is_positive());
        assert!(literals[1].is_true(&bank));
        assert_eq!(literals[2].left(), &c);
        assert_eq!(literals[2].right(), &d);
        assert!(literals[2].is_positive());
    }

    #[test]
    fn tformula_collect_clause_normalizes_variables_with_fresh_bank() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -210);
        let a = typed_const(&mut bank, "collect_norm_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let clause = tformula_collect_clause(&mut bank, &formula, Some(&fresh_vars)).unwrap();

        assert_eq!(clause.literal_number(), 1);
        assert_eq!(clause.weight(), clause.standard_weight());
        let literal = &clause.literals().as_slice()[0];
        assert!(literal.left().is_free_var());
        assert_ne!(literal.left(), &x);
        assert_eq!(literal.right(), &a);
        assert!(literal.is_positive());
    }

    #[test]
    fn tformula_to_cnf_skips_universals_and_splits_conjuncts_like_c() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -220);
        let a = typed_const(&mut bank, "to_cnf_a");
        let b = typed_const(&mut bank, "to_cnf_b");
        let c = typed_const(&mut bank, "to_cnf_c");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let and_code = bank.signature().and_code();
        let body = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let qall_code = bank.signature().qall_code();
        let quantified = bool_binary_with_code(&mut bank, qall_code, &x, &body);
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let source = FormulaDerivationRef::new(77);
        let mut set = ClauseSet::new();

        let count = tformula_to_cnf(
            &mut bank,
            &quantified,
            CP_TYPE_NEG_CONJECTURE,
            &mut set,
            &fresh_vars,
            source,
            ProblemType::FirstOrder,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(set.members(), 2);
        let clauses = set.iter().collect::<Vec<_>>();
        for clause in &clauses {
            assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
            assert_eq!(
                clause.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                    DerivationEntry::FormulaParent(source),
                ]
            );
        }
        let first_literal = &clauses[0].literals().as_slice()[0];
        assert_eq!(first_literal.left(), &b);
        assert_eq!(first_literal.right(), &c);
        let second_literal = &clauses[1].literals().as_slice()[0];
        assert!(second_literal.left().is_free_var());
        assert_ne!(second_literal.left(), &x);
        assert_eq!(second_literal.right(), &a);
    }

    #[test]
    fn tformula_to_cnf_records_boolean_elimination_derivations() {
        let mut bank = test_bank();
        let x = bool_var(&bank, -230);
        let true_term = bank.true_term().clone();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let positive = bool_binary_with_code(&mut bank, eqn_code, &x, &true_term);
        let negative = bool_binary_with_code(&mut bank, neqn_code, &x, &true_term);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &positive, &negative);
        let fresh_vars = VarBank::new(bank.signature().type_bank());
        let source = FormulaDerivationRef::new(88);
        let mut set = ClauseSet::new();

        let count = tformula_to_cnf(
            &mut bank,
            &formula,
            CP_TYPE_NEG_CONJECTURE,
            &mut set,
            &fresh_vars,
            source,
            ProblemType::FirstOrder,
        )
        .unwrap();

        assert_eq!(count, 1);
        let clause = set.iter().next().unwrap();
        assert_eq!(clause.literal_number(), 1);
        assert!(clause.literals().find_true(&bank).is_some());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                DerivationEntry::FormulaParent(source),
                DerivationEntry::Operation(DC_NORMALIZE),
                DerivationEntry::Operation(DC_ELIMINATE_BVAR),
            ]
        );
    }

    #[test]
    fn tformula_var_rename_refreshes_nested_shadowed_quantifiers() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -150);
        let a = typed_const(&mut bank, "var_rename_shadow_a");
        let b = typed_const(&mut bank, "var_rename_shadow_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let outer_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let inner_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &b);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let or_code = bank.signature().or_code();
        let inner_quantifier = bool_binary_with_code(&mut bank, qex_code, &x, &inner_atom);
        let body = bool_binary_with_code(&mut bank, or_code, &outer_atom, &inner_quantifier);
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &body);
        prepare_formula_fresh_vars(&bank);

        let renamed = tformula_var_rename(&mut bank, &formula).unwrap();

        assert!(x.binding().is_none());
        assert_eq!(renamed.f_code(), qall_code);
        let outer_fresh = renamed.argument(0).unwrap();
        assert_ne!(outer_fresh, x);
        let renamed_body = renamed.argument(1).unwrap();
        assert_eq!(renamed_body.f_code(), or_code);
        let renamed_outer_atom = renamed_body.argument(0).unwrap();
        assert_eq!(renamed_outer_atom.f_code(), eqn_code);
        assert_eq!(renamed_outer_atom.argument(0).as_ref(), Some(&outer_fresh));
        assert_eq!(renamed_outer_atom.argument(1).as_ref(), Some(&a));

        let renamed_inner = renamed_body.argument(1).unwrap();
        assert_eq!(renamed_inner.f_code(), qex_code);
        let inner_fresh = renamed_inner.argument(0).unwrap();
        assert_ne!(inner_fresh, x);
        assert_ne!(inner_fresh, outer_fresh);
        let renamed_inner_atom = renamed_inner.argument(1).unwrap();
        assert_eq!(renamed_inner_atom.argument(0).as_ref(), Some(&inner_fresh));
        assert_eq!(renamed_inner_atom.argument(1).as_ref(), Some(&b));
    }

    #[test]
    fn tformula_var_rename_restores_existing_quantified_binding() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -160);
        let existing = typed_var(&bank, -162);
        let a = typed_const(&mut bank, "var_rename_restore_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let qall_code = bank.signature().qall_code();
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &atom);
        x.set_binding(Some(existing.clone()));
        prepare_formula_fresh_vars(&bank);

        let renamed = tformula_var_rename(&mut bank, &formula).unwrap();

        assert_eq!(x.binding(), Some(existing.clone()));
        let fresh = renamed.argument(0).unwrap();
        assert_ne!(fresh, x);
        assert_ne!(fresh, existing);
        let renamed_atom = renamed.argument(1).unwrap();
        assert_eq!(renamed_atom.argument(0).as_ref(), Some(&fresh));
        assert_eq!(renamed_atom.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn tformula_var_rename_recurses_through_ite_arguments() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -170);
        let a = typed_const(&mut bank, "var_rename_ite_a");
        let b = typed_const(&mut bank, "var_rename_ite_b");
        let c = typed_const(&mut bank, "var_rename_ite_c");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let condition = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let then_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &b);
        let else_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &c);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let then_quantifier = bool_binary_with_code(&mut bank, qex_code, &x, &then_atom);
        let ite = Term::top_alloc(SIG_ITE_CODE, 3);
        ite.set_type(Some(bank.signature().type_bank().bool_type()));
        ite.set_argument(0, condition);
        ite.set_argument(1, then_quantifier);
        ite.set_argument(2, else_atom);
        let ite = bank.term_top_insert(ite).unwrap();
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &ite);
        prepare_formula_fresh_vars(&bank);

        let renamed = tformula_var_rename(&mut bank, &formula).unwrap();

        assert!(x.binding().is_none());
        let outer_fresh = renamed.argument(0).unwrap();
        let renamed_ite = renamed.argument(1).unwrap();
        assert_eq!(renamed_ite.f_code(), SIG_ITE_CODE);
        let renamed_condition = renamed_ite.argument(0).unwrap();
        assert_eq!(renamed_condition.argument(0).as_ref(), Some(&outer_fresh));
        assert_eq!(renamed_condition.argument(1).as_ref(), Some(&a));
        let renamed_then = renamed_ite.argument(1).unwrap();
        let inner_fresh = renamed_then.argument(0).unwrap();
        assert_ne!(inner_fresh, outer_fresh);
        let renamed_then_atom = renamed_then.argument(1).unwrap();
        assert_eq!(renamed_then_atom.argument(0).as_ref(), Some(&inner_fresh));
        assert_eq!(renamed_then_atom.argument(1).as_ref(), Some(&b));
        let renamed_else = renamed_ite.argument(2).unwrap();
        assert_eq!(renamed_else.argument(0).as_ref(), Some(&outer_fresh));
        assert_eq!(renamed_else.argument(1).as_ref(), Some(&c));
    }

    #[test]
    fn tformula_skolemize_outermost_replaces_closed_existential_with_constant() {
        let mut bank = test_bank();
        let z = typed_var(&bank, -180);
        let a = typed_const(&mut bank, "skolem_const_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &z, &a);
        let qex_code = bank.signature().qex_code();
        let formula = bool_binary_with_code(&mut bank, qex_code, &z, &atom);

        let skolemized = tformula_skolemize_outermost(&mut bank, &formula).unwrap();

        assert!(z.binding().is_none());
        assert_eq!(skolemized.f_code(), eqn_code);
        let skolem = skolemized.argument(0).unwrap();
        assert_ne!(skolem, z);
        assert_eq!(skolem.arity(), 0);
        assert_eq!(skolem.type_(), z.type_());
        assert_eq!(skolemized.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn tformula_skolemize_outermost_uses_free_and_universal_dependencies() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -182);
        let y = typed_var(&bank, -184);
        let z = typed_var(&bank, -186);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &z, &y);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let existential = bool_binary_with_code(&mut bank, qex_code, &z, &atom);
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &existential);

        let skolemized = tformula_skolemize_outermost(&mut bank, &formula).unwrap();

        assert!(z.binding().is_none());
        assert_eq!(skolemized.f_code(), qall_code);
        assert_eq!(skolemized.argument(0).as_ref(), Some(&x));
        let body = skolemized.argument(1).unwrap();
        assert_eq!(body.f_code(), eqn_code);
        let skolem = body.argument(0).unwrap();
        assert_eq!(skolem.arity(), 2);
        assert_eq!(skolem.argument(0).as_ref(), Some(&y));
        assert_eq!(skolem.argument(1).as_ref(), Some(&x));
        assert_eq!(body.argument(1).as_ref(), Some(&y));
    }

    #[test]
    fn tformula_skolemize_outermost_excludes_bound_vars_from_initial_free_stack() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -188);
        let z = typed_var(&bank, -190);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &z, &x);
        let qex_code = bank.signature().qex_code();
        let qall_code = bank.signature().qall_code();
        let existential = bool_binary_with_code(&mut bank, qex_code, &z, &atom);
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &existential);

        let skolemized = tformula_skolemize_outermost(&mut bank, &formula).unwrap();

        let body = skolemized.argument(1).unwrap();
        let skolem = body.argument(0).unwrap();
        assert_eq!(skolem.arity(), 1);
        assert_eq!(skolem.argument(0).as_ref(), Some(&x));
        assert_eq!(body.argument(1).as_ref(), Some(&x));
    }

    #[test]
    fn tformula_skolemize_outermost_copies_non_logical_boolean_terms_with_binding() {
        let mut bank = test_bank();
        let z = typed_var(&bank, -192);
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = alloc_arrow_type(vec![arg_type, bool_type]);
        let p_code = bank.signature_mut().insert_id("skolem_bool_atom", 1, false);
        bank.signature_mut()
            .declare_final_type(p_code, predicate_type)
            .unwrap();
        let atom = bool_result_unary_with_code(&mut bank, p_code, &z);
        let qex_code = bank.signature().qex_code();
        let formula = bool_binary_with_code(&mut bank, qex_code, &z, &atom);

        let skolemized = tformula_skolemize_outermost(&mut bank, &formula).unwrap();

        assert!(z.binding().is_none());
        assert_eq!(skolemized.f_code(), p_code);
        let skolem = skolemized.argument(0).unwrap();
        assert_ne!(skolem, z);
        assert_eq!(skolem.arity(), 0);
        assert_eq!(skolem.type_(), z.type_());
    }

    #[test]
    fn tformula_shift_quantors_moves_universals_outward_in_c_stack_order() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -130);
        let y = typed_var(&bank, -132);
        let a = typed_const(&mut bank, "shift_quant_a");
        let b = typed_const(&mut bank, "shift_quant_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &y, &b);
        let qall_code = bank.signature().qall_code();
        let or_code = bank.signature().or_code();
        let inner_quant = bool_binary_with_code(&mut bank, qall_code, &y, &right_atom);
        let disjunction = bool_binary_with_code(&mut bank, or_code, &left_atom, &inner_quant);
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &disjunction);

        let shifted = tformula_shift_quantors(&mut bank, &formula).unwrap();

        assert_eq!(shifted.f_code(), qall_code);
        assert_eq!(shifted.argument(0).as_ref(), Some(&x));
        let second_quant = shifted.argument(1).unwrap();
        assert_eq!(second_quant.f_code(), qall_code);
        assert_eq!(second_quant.argument(0).as_ref(), Some(&y));
        let body = second_quant.argument(1).unwrap();
        assert_eq!(body.f_code(), or_code);
        assert_eq!(body.argument(0).as_ref(), Some(&left_atom));
        assert_eq!(body.argument(1).as_ref(), Some(&right_atom));
    }

    #[test]
    fn tformula_shift_quantors_only_descends_through_and_or_like_c() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -134);
        let a = typed_const(&mut bank, "shift_quant_guard_a");
        let b = typed_const(&mut bank, "shift_quant_guard_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &b);
        let qall_code = bank.signature().qall_code();
        let impl_code = bank.signature().impl_code();
        let quant = bool_binary_with_code(&mut bank, qall_code, &x, &right_atom);
        let implication = bool_binary_with_code(&mut bank, impl_code, &left_atom, &quant);

        let shifted = tformula_shift_quantors(&mut bank, &implication).unwrap();

        assert_eq!(shifted, implication);
    }

    #[test]
    fn tformula_shift_quantors2_preserves_mixed_quantifier_codes() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -136);
        let y = typed_var(&bank, -138);
        let z = typed_var(&bank, -140);
        let a = typed_const(&mut bank, "shift_quant_mixed_a");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &x, &a);
        let middle_atom = bool_binary_with_code(&mut bank, eqn_code, &y, &a);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &z, &a);
        let qall_code = bank.signature().qall_code();
        let qex_code = bank.signature().qex_code();
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let existential = bool_binary_with_code(&mut bank, qex_code, &y, &middle_atom);
        let universal = bool_binary_with_code(&mut bank, qall_code, &z, &right_atom);
        let left_conjunction = bool_binary_with_code(&mut bank, and_code, &left_atom, &existential);
        let body = bool_binary_with_code(&mut bank, or_code, &left_conjunction, &universal);
        let formula = bool_binary_with_code(&mut bank, qall_code, &x, &body);

        let shifted = tformula_shift_quantors2(&mut bank, &formula).unwrap();

        assert_eq!(shifted.f_code(), qall_code);
        assert_eq!(shifted.argument(0).as_ref(), Some(&x));
        let second_quant = shifted.argument(1).unwrap();
        assert_eq!(second_quant.f_code(), qex_code);
        assert_eq!(second_quant.argument(0).as_ref(), Some(&y));
        let third_quant = second_quant.argument(1).unwrap();
        assert_eq!(third_quant.f_code(), qall_code);
        assert_eq!(third_quant.argument(0).as_ref(), Some(&z));
        let shifted_body = third_quant.argument(1).unwrap();
        assert_eq!(shifted_body.f_code(), or_code);
        let shifted_left = shifted_body.argument(0).unwrap();
        assert_eq!(shifted_left.f_code(), and_code);
        assert_eq!(shifted_left.argument(0).as_ref(), Some(&left_atom));
        assert_eq!(shifted_left.argument(1).as_ref(), Some(&middle_atom));
        assert_eq!(shifted_body.argument(1).as_ref(), Some(&right_atom));
    }

    #[test]
    fn unit_simplify_test_matches_c_sign_and_subsumption_conditions() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &first, true)]);
        let negative_clause = clause_from(vec![literal(&mut bank, &second, &first, false)]);
        let positive_clause = clause_from(vec![literal(&mut bank, &second, &first, true)]);

        assert!(clause_unit_simplify_test(&negative_clause, &positive_unit));
        assert!(!clause_unit_simplify_test(&positive_clause, &positive_unit));
    }

    #[test]
    #[should_panic(expected = "positive unit simplifier must not be oriented")]
    fn unit_simplify_test_rejects_positive_oriented_simplifier() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let mut oriented_unit_lit = literal(&mut bank, &variable, &first, true);
        oriented_unit_lit.set_prop(EP_IS_ORIENTED);
        let oriented_unit = clause_from(vec![oriented_unit_lit]);
        let negative_clause = clause_from(vec![literal(&mut bank, &second, &first, false)]);

        let _ = clause_unit_simplify_test(&negative_clause, &oriented_unit);
    }

    #[test]
    fn eliminate_naked_boolean_positive_literal_substitutes_false() {
        let mut bank = test_bank();
        let variable = bool_var(&bank, -20);
        let other = bool_var(&bank, -21);
        let true_term = bank.true_term().clone();
        let naked = literal(&mut bank, &variable, &true_term, true);
        let dependent = literal(&mut bank, &other, &variable, true);
        let mut clause = clause_from(vec![naked, dependent]);

        assert!(!clause_eliminate_naked_boolean_variables(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 1);
        let remaining = &clause.literals().as_slice()[0];
        assert!(remaining.is_negative());
        assert_eq!(remaining.left(), &other);
        assert_eq!(remaining.right(), bank.true_term());
        assert!(variable.binding().is_none());
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn eliminate_naked_boolean_negative_literal_substitutes_true() {
        let mut bank = test_bank();
        let variable = bool_var(&bank, -22);
        let other = bool_var(&bank, -23);
        let true_term = bank.true_term().clone();
        let naked = literal(&mut bank, &variable, &true_term, false);
        let dependent = literal(&mut bank, &other, &variable, false);
        let mut clause = clause_from(vec![naked, dependent]);

        assert!(!clause_eliminate_naked_boolean_variables(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 1);
        let remaining = &clause.literals().as_slice()[0];
        assert!(remaining.is_negative());
        assert_eq!(remaining.left(), &other);
        assert_eq!(remaining.right(), bank.true_term());
        assert!(variable.binding().is_none());
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn eliminate_naked_boolean_opposite_polarities_create_true_literal() {
        let mut bank = test_bank();
        let variable = bool_var(&bank, -24);
        let true_term = bank.true_term().clone();
        let positive = literal(&mut bank, &variable, &true_term, true);
        let negative = literal(&mut bank, &variable, &true_term, false);
        let mut clause = clause_from(vec![positive, negative]);

        assert!(clause_eliminate_naked_boolean_variables(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 1);
        assert!(clause.literals().find_true(&bank).is_some());
        assert!(clause.is_trivial(&bank));
        assert!(variable.binding().is_none());
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn resolve_flex_clause_negative_applied_predicate_equality_derives_empty() {
        let mut bank = test_bank();
        let left = applied_predicate_var(&mut bank, -40, "a");
        let right = applied_predicate_var(&mut bank, -41, "b");
        let mut clause = clause_from(vec![literal(&mut bank, &left, &right, false)]);

        assert!(clause_resolve_flex_clause(&mut clause, &bank));

        assert!(clause.is_empty());
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_FLEX_RESOLVE)]
        );
    }

    #[test]
    fn resolve_flex_clause_negative_free_variable_equality_derives_empty() {
        let mut bank = test_bank();
        let left = typed_var(&bank, -42);
        let right = typed_var(&bank, -43);
        let mut clause = clause_from(vec![literal(&mut bank, &left, &right, false)]);

        assert!(clause_resolve_flex_clause(&mut clause, &bank));

        assert!(clause.is_empty());
        assert_eq!(clause.weight(), clause.standard_weight());
    }

    #[test]
    fn resolve_flex_clause_rejects_conflicting_predicate_literal_signs() {
        let mut bank = test_bank();
        let predicate = applied_predicate_var(&mut bank, -44, "a");
        let true_term = bank.true_term().clone();
        let positive = literal(&mut bank, &predicate, &true_term, true);
        let negative = literal(&mut bank, &predicate, &true_term, false);
        let mut clause = clause_from(vec![positive, negative]);
        let original = clause.clone();

        assert!(!clause_resolve_flex_clause(&mut clause, &bank));

        assert_eq!(clause.literal_number(), original.literal_number());
        assert_eq!(clause.weight(), original.weight());
        assert!(clause.derivation().is_none());
    }

    #[test]
    fn resolve_flex_clause_rejects_predicate_variable_also_seen_in_equality() {
        let mut bank = test_bank();
        let left = applied_predicate_var(&mut bank, -45, "a");
        let right = applied_predicate_var(&mut bank, -46, "b");
        let true_term = bank.true_term().clone();
        let equality = literal(&mut bank, &left, &right, false);
        let predicate = literal(&mut bank, &left, &true_term, true);
        let mut clause = clause_from(vec![equality, predicate]);
        let original = clause.clone();

        assert!(!clause_resolve_flex_clause(&mut clause, &bank));

        assert_eq!(clause.literal_number(), original.literal_number());
        assert_eq!(clause.weight(), original.weight());
        assert!(clause.derivation().is_none());
    }

    #[test]
    fn recognize_injectivity_builds_inverse_definition_clause() {
        let mut bank = test_bank();
        let f_code = typed_binary_code(&mut bank, "inj_f");
        let x = typed_var(&bank, -30);
        let y = typed_var(&bank, -31);
        let z = typed_var(&bank, -32);
        let left = typed_binary_with_code(&mut bank, f_code, &x, &z);
        let right = typed_binary_with_code(&mut bank, f_code, &y, &z);
        let mut source = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &x, &y, true),
        ]);
        source.set_tptp_type(CP_TYPE_AXIOM);
        source.set_prop(CP_IS_SOS);
        source.set_proof_depth(4);
        source.set_proof_size(7);

        let recognized = clause_recognize_injectivity(&mut bank, &source)
            .unwrap()
            .unwrap();

        assert_eq!(recognized.positive_literal_count(), 1);
        assert_eq!(recognized.negative_literal_count(), 0);
        assert!(recognized.query_prop(CP_IS_PURE_INJECTIVITY));
        assert!(recognized.query_prop(CP_IS_SOS));
        assert_eq!(recognized.query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(recognized.proof_depth(), 5);
        assert_eq!(recognized.proof_size(), 8);
        assert_eq!(
            recognized.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_INV_REC),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(source.ident(), 0)),
            ],
        );
        assert_eq!(recognized.weight(), recognized.standard_weight());

        let inverse_literal = &recognized.literals().as_slice()[0];
        let inverse = inverse_literal.left();
        assert!(bank
            .signature()
            .query_prop(inverse.f_code(), FP_IS_INJ_DEF_SKOLEM));
        assert_eq!(inverse.arity(), 2);
        assert_eq!(inverse.argument(0), Some(z));
        assert_eq!(inverse.argument(1), Some(left));
        assert_eq!(inverse_literal.right(), &x);
    }

    #[test]
    fn recognize_injectivity_rejects_repeated_variable_conflicts() {
        let mut bank = test_bank();
        let f_code = typed_binary_code(&mut bank, "bad_inj_f");
        let x = typed_var(&bank, -40);
        let y = typed_var(&bank, -41);
        let left = typed_binary_with_code(&mut bank, f_code, &x, &x);
        let right = typed_binary_with_code(&mut bank, f_code, &y, &x);
        let source = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &x, &y, true),
        ]);

        assert!(clause_recognize_injectivity(&mut bank, &source)
            .unwrap()
            .is_none());
        assert!(!x.query_prop(crate::terms::termtypes::TP_CHECK_FLAG));
        assert!(!x.query_prop(crate::terms::termtypes::TP_OP_FLAG));
        assert!(!y.query_prop(crate::terms::termtypes::TP_CHECK_FLAG));
        assert!(!y.query_prop(crate::terms::termtypes::TP_OP_FLAG));
    }

    #[test]
    fn replace_injectivity_defs_archives_first_definition_and_keeps_duplicate_original() {
        let mut bank = test_bank();
        let f_code = typed_binary_code(&mut bank, "replace_inj_f");
        let first_x = typed_var(&bank, -50);
        let first_y = typed_var(&bank, -51);
        let first_shared = typed_var(&bank, -52);
        let first_left = typed_binary_with_code(&mut bank, f_code, &first_x, &first_shared);
        let first_right = typed_binary_with_code(&mut bank, f_code, &first_y, &first_shared);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_left, &first_right, false),
            literal(&mut bank, &first_x, &first_y, true),
        ]);
        first.set_prop(CP_IS_SOS);
        let first_id = first.ident();

        let duplicate_x = typed_var(&bank, -60);
        let duplicate_y = typed_var(&bank, -61);
        let duplicate_shared = typed_var(&bank, -62);
        let duplicate_left =
            typed_binary_with_code(&mut bank, f_code, &duplicate_x, &duplicate_shared);
        let duplicate_right =
            typed_binary_with_code(&mut bank, f_code, &duplicate_y, &duplicate_shared);
        let duplicate = clause_from(vec![
            literal(&mut bank, &duplicate_left, &duplicate_right, false),
            literal(&mut bank, &duplicate_x, &duplicate_y, true),
        ]);
        let duplicate_id = duplicate.ident();

        let noise = clause_from(vec![literal(&mut bank, &first_x, &first_shared, true)]);
        let noise_id = noise.ident();
        let mut set = ClauseSet::from_clauses([first, duplicate, noise]);
        let mut archive = ClauseSet::new();

        assert_eq!(
            clause_set_replace_injectivity_defs(&mut set, &mut archive, &mut bank).unwrap(),
            1
        );

        assert_eq!(archive.len(), 1);
        assert_eq!(archive.iter().next().map(Clause::ident), Some(first_id));
        assert!(set.find_by_id(first_id).is_none());
        assert!(set.find_by_id(duplicate_id).is_some());
        assert!(set.find_by_id(noise_id).is_some());
        let generated = set
            .iter()
            .find(|clause| clause.query_prop(CP_IS_PURE_INJECTIVITY))
            .expect("replacement clause inserted");
        assert!(generated.query_prop(CP_IS_SOS));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn recognize_choice_axiom_records_choice_symbol_copy() {
        let mut bank = test_bank();
        let (choice_clause, choice_code) = choice_axiom(&mut bank, "choice_recognized", -70, -72);
        let mut set = ClauseSet::from_clauses([choice_clause]);
        let mut choice_symbols = BTreeMap::new();

        assert_eq!(
            clause_set_recognize_choice(&mut bank, &mut set, &mut choice_symbols).unwrap(),
            1
        );

        assert_eq!(choice_symbols.len(), 1);
        let stored = choice_symbols
            .get(&choice_code)
            .expect("choice operator should be recorded");
        assert_eq!(stored.literal_number(), 2);
        let live = set.iter().next().expect("source clause remains in set");
        assert_eq!(stored.ident(), live.ident());
        assert!(live.literals().as_slice()[0].left().is_applied_free_var());
        assert!(live.literals().as_slice()[1].left().is_applied_free_var());
    }

    #[test]
    fn recognizes_choice_without_map_does_not_mutate_or_reject_duplicates() {
        let mut bank = test_bank();
        let (first, choice_code) = choice_axiom(&mut bank, "choice_boolean", -71, -73);
        let (second, _) = choice_axiom(&mut bank, "choice_boolean", -75, -77);
        let first_left = first.literals().as_slice()[0].left().clone();
        let mut set = ClauseSet::from_clauses([first.clone(), second]);
        let mut choice_symbols = BTreeMap::new();

        assert!(clause_recognizes_choice(&mut bank, &first).unwrap());
        assert_eq!(first.literals().as_slice()[0].left(), &first_left);
        assert_eq!(
            clause_set_recognize_choice(&mut bank, &mut set, &mut choice_symbols).unwrap(),
            1
        );
        let duplicate = set
            .iter()
            .find(|clause| clause.ident() != choice_symbols[&choice_code].ident())
            .expect("duplicate choice clause remains in source set");
        assert!(clause_recognizes_choice(&mut bank, duplicate).unwrap());
    }

    #[test]
    fn recognize_choice_axiom_rejects_duplicate_choice_symbol() {
        let mut bank = test_bank();
        let (first, choice_code) = choice_axiom(&mut bank, "choice_duplicate", -80, -82);
        let (second, _) = choice_axiom(&mut bank, "choice_duplicate", -84, -86);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut set = ClauseSet::from_clauses([first, second]);
        let mut choice_symbols = BTreeMap::new();

        assert_eq!(
            clause_set_recognize_choice(&mut bank, &mut set, &mut choice_symbols).unwrap(),
            1
        );

        assert_eq!(choice_symbols.len(), 1);
        assert_eq!(
            choice_symbols.get(&choice_code).map(Clause::ident),
            Some(first_id)
        );
        assert!(set.find_by_id(first_id).is_some());
        assert!(set.find_by_id(second_id).is_some());
    }

    #[test]
    fn canon_compare_ref_uses_clause_structural_weight_order() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "a");
        let second = typed_const(&mut bank, "b");
        let third = typed_const(&mut bank, "c");
        let light = clause_from(vec![literal(&mut bank, &first, &second, true)]);
        let heavy = clause_from(vec![
            literal(&mut bank, &first, &second, true),
            literal(&mut bank, &second, &third, true),
        ]);

        assert!(clause_canon_compare_ref(&light, &heavy, &bank) < 0);
        assert_eq!(clause_canon_compare_ref(&light, &light, &bank), 0);
    }
}
