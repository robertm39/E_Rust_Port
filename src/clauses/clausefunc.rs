use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::clauses::clause::{clause_print_lop_format_string, Clause};
use crate::clauses::clause_props::{
    CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_D_INDEXED, CP_IS_PURE_INJECTIVITY, CP_IS_SOS,
    CP_IS_S_INDEXED, CP_LIMITED_RW,
};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_push_derivation, op_has_cnf_arg1, op_has_cnf_arg2, op_is_generating,
    ClauseDerivationRef, DerivationEntry, DerivationParentRef, DC_CNF_ADD_ARG, DC_CNF_QUOTE,
    DC_FLEX_RESOLVE, DC_NORMALIZE,
};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE,
};
use crate::clauses::eqnlist::EqnList;
use crate::terms::match_mgu::subst_mgu_complete;
use crate::terms::signature::{FP_IS_INJ_DEF_SKOLEM, SIG_DB_LAMBDA_CODE};
use crate::terms::simpletypes::{arrow_type_flattened, type_is_predicate, Type};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_is_db_closed, term_standard_weight};
use crate::terms::termtypes::{
    term_del_prop, term_identity_id, DerefType, Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT,
    TP_CHECK_FLAG, TP_OP_FLAG, TP_PRED_POS,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;

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
        let _ = clause_remove_superfluous_literals(clause, bank);
        clause.recompute_lit_counts();
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

fn tformula_fcode_alloc(
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
    term.set_type(Some(bank.signature().type_bank().bool_type()));
    if bank.signature().is_predicate(op) {
        term.set_prop(TP_PRED_POS);
    }
    term.set_argument(0, arg1);
    if let Some(arg2) = arg2 {
        term.set_argument(1, arg2);
    }
    bank.term_top_insert(term)
}

fn close_with_db_var(
    bank: &mut TermBank,
    binder_type: &Type,
    body: &Term,
) -> Result<Term, Diagnostic> {
    assert!(body.is_shared(), "lambda body must be a shared bank term");
    let body_type = body.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "CloseWithDBVar requires a typed lambda body",
        )
    })?;
    let binder = bank.request_db_var(binder_type, 0);
    let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 2);
    lambda.set_argument(0, binder);
    lambda.set_argument(1, body.clone());
    let lambda_type =
        bank.signature_mut()
            .type_bank_mut()
            .insert_type_shared(arrow_type_flattened(
                std::slice::from_ref(binder_type),
                &body_type,
            ));
    lambda.set_type(Some(lambda_type));
    bank.term_top_insert(lambda)
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

/// Recognizes an injectivity definition and creates the inverse-function clause.
///
/// This is the plain clause-building part of C `ClauseRecognizeInjectivity`;
/// proof-documentation and derivation-stack updates remain tied to the later
/// derivation port.
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
        clause_recognize_injectivity, clause_remove_ac_resolved, clause_remove_literal,
        clause_remove_literal_index, clause_remove_superfluous_literals,
        clause_resolve_flex_clause, clause_set_archive_copy, clause_set_canonize,
        clause_set_delete_orphans_with, clause_set_remove_superfluous_literals,
        clause_set_replace_injectivity_defs, clause_unit_simplify_test, close_with_db_var,
        pstack_clause_print_lop_string, tformula_simplify_decoded,
    };
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_PURE_INJECTIVITY, CP_IS_SOS, CP_LIMITED_RW,
        CP_TYPE_AXIOM,
    };
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        clause_push_derivation, ClauseDerivationRef, DerivationEntry, DerivationParentRef,
        DC_CNF_ADD_ARG, DC_CNF_EVAL_GC, DC_CNF_QUOTE, DC_FLEX_RESOLVE, DC_NORMALIZE,
        DC_ORDERED_FACTOR, DC_PARAMOD, DC_REWRITE,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{
        Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE, FP_IS_INJ_DEF_SKOLEM, SIG_DB_LAMBDA_CODE,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
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

    fn applied_predicate_var(bank: &mut TermBank, code: i64, arg_name: &str) -> Term {
        let predicate = predicate_var(bank, code);
        let argument = typed_const(bank, arg_name);
        let applied = bank.term_apply_arg(&predicate, &argument);
        bank.term_top_insert(applied).unwrap()
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
