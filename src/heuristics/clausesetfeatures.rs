use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::heuristics::clausefeatures::{
    clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
    clause_count_unorientable_literals, clause_count_variable_set, clause_tptp_depth_info_add,
};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClauseSetArityInformation {
    pub max_fun_arity: i32,
    pub avg_fun_arity: i32,
    pub sum_fun_arity: i32,
    pub max_pred_arity: i32,
    pub avg_pred_arity: i32,
    pub sum_pred_arity: i32,
    pub non_const_funs: i32,
    pub non_const_preds: i32,
    pub fun_const_count: i64,
}

#[must_use]
pub fn clause_set_count_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_goal)
}

#[must_use]
pub fn clause_set_count_axioms(set: &ClauseSet) -> i64 {
    set.members() - clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_unit(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_unit)
}

#[must_use]
pub fn clause_set_count_unit_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_unit() && clause.is_goal())
}

#[must_use]
pub fn clause_set_count_unit_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_unit(set) - clause_set_count_unit_goals(set)
}

#[must_use]
pub fn clause_set_is_unit_set(set: &ClauseSet) -> bool {
    set.members() == clause_set_count_unit(set)
}

#[must_use]
pub fn clause_set_axioms_are_unit(set: &ClauseSet) -> bool {
    clause_set_count_unit_axioms(set) == clause_set_count_axioms(set)
}

#[must_use]
pub fn clause_set_goals_are_unit(set: &ClauseSet) -> bool {
    clause_set_count_unit_goals(set) == clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_horn(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_horn)
}

#[must_use]
pub fn clause_set_count_horn_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_horn() && clause.is_goal())
}

#[must_use]
pub fn clause_set_count_horn_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_horn(set) - clause_set_count_horn_goals(set)
}

#[must_use]
pub fn clause_set_is_horn_set(set: &ClauseSet) -> bool {
    set.members() == clause_set_count_horn(set)
}

#[must_use]
pub fn clause_set_axioms_are_horn(set: &ClauseSet) -> bool {
    clause_set_count_horn_axioms(set) == clause_set_count_axioms(set)
}

#[must_use]
pub fn clause_set_goals_are_horn(set: &ClauseSet) -> bool {
    clause_set_count_horn_goals(set) == clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_equational(bank: &TermBank, set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_equational(bank))
}

#[must_use]
pub fn clause_set_is_equational_set(bank: &TermBank, set: &ClauseSet) -> bool {
    set.members() == clause_set_count_equational(bank, set)
}

#[must_use]
pub fn clause_set_is_equational(bank: &TermBank, set: &ClauseSet) -> bool {
    clause_set_count_equational(bank, set) >= 1
}

#[must_use]
pub fn clause_set_count_pure_equational(bank: &TermBank, set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_pure_equational(bank))
}

#[must_use]
pub fn clause_set_is_pure_equational_set(bank: &TermBank, set: &ClauseSet) -> bool {
    set.members() == clause_set_count_pure_equational(bank, set)
}

#[must_use]
pub fn clause_set_count_pos_units(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_demodulator)
}

#[must_use]
pub fn clause_set_count_ground_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_goal() && clause.is_ground())
}

#[must_use]
pub fn clause_set_goals_are_ground(set: &ClauseSet) -> bool {
    clause_set_count_goals(set) == clause_set_count_ground_goals(set)
}

#[must_use]
pub fn clause_set_count_ground(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_ground)
}

#[must_use]
pub fn clause_set_is_ground(set: &ClauseSet) -> bool {
    clause_set_count_ground(set) == set.members()
}

#[must_use]
pub fn clause_set_count_ground_positive_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_positive() && clause.is_ground())
}

#[must_use]
pub fn clause_set_count_positive_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_positive)
}

#[must_use]
pub fn clause_set_count_ground_unit_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_demodulator() && clause.is_ground())
}

#[must_use]
pub fn clause_set_count_non_ground_unit_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_unit_axioms(set) - clause_set_count_ground_unit_axioms(set)
}

#[must_use]
pub fn clause_set_count_range_restricted(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_range_restricted)
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn clause_set_non_ground_axiom_part(set: &ClauseSet) -> f64 {
    let unit_axioms = clause_set_count_unit_axioms(set);
    if unit_axioms == 0 {
        0.0
    } else {
        (unit_axioms - clause_set_count_ground_unit_axioms(set)) as f64 / unit_axioms as f64
    }
}

/// Collects the arity statistics used by the C strategy feature extractor.
///
/// # Panics
///
/// Panics if a positive f-code in `signature` has no arity entry, or if the
/// signature f-code count cannot be represented as a Rust vector size.
#[must_use]
pub fn clause_set_collect_arity_information(
    set: &ClauseSet,
    signature: &Signature,
) -> ClauseSetArityInformation {
    let mut max_fun_arity = 0;
    let mut sum_fun_arity = 0;
    let mut fun_count = 0;
    let mut fun_const_count = 0;
    let mut non_const_preds = 0;
    let mut max_pred_arity = 0;
    let mut sum_pred_arity = 0;
    let mut pred_count = 0;
    let mut dist_array = vec![0; fcode_index(signature.f_count() + 1)];

    set.add_symbol_distribution(&mut dist_array);

    for symbol in 1..=signature.f_count() {
        let index = fcode_index(symbol);
        if signature.is_special(symbol) || dist_array[index] == 0 {
            continue;
        }
        let arity = signature
            .find_arity(symbol)
            .unwrap_or_else(|| panic!("signature arity must exist for positive f-code"));
        if signature.is_predicate(symbol) {
            max_pred_arity = max_pred_arity.max(arity);
            sum_pred_arity += arity;
            pred_count += 1;
            if arity != 0 {
                non_const_preds += 1;
            }
        } else if arity != 0 {
            max_fun_arity = max_fun_arity.max(arity);
            sum_fun_arity += arity;
            fun_count += 1;
        } else {
            fun_const_count += 1;
        }
    }

    ClauseSetArityInformation {
        max_fun_arity,
        avg_fun_arity: if fun_count == 0 {
            0
        } else {
            sum_fun_arity / fun_count
        },
        sum_fun_arity,
        max_pred_arity,
        avg_pred_arity: if pred_count == 0 {
            0
        } else {
            sum_pred_arity / pred_count
        },
        sum_pred_arity,
        non_const_funs: fun_count,
        non_const_preds,
        fun_const_count,
    }
}

#[must_use]
pub fn clause_set_count_maximal_terms(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_maximal_terms).sum()
}

#[must_use]
pub fn clause_set_count_maximal_literals(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_maximal_literals).sum()
}

/// Counts distinct variable f-codes per clause and sums the clause counts.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_count_variable_set`].
#[must_use]
pub fn clause_set_count_variables(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_variable_set).sum()
}

/// Counts singleton variable f-codes per clause and sums the clause counts.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_count_singleton_set`].
#[must_use]
pub fn clause_set_count_singletons(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_singleton_set).sum()
}

/// Adds TPTP-style depth statistics for all clauses in `set`.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_tptp_depth_info_add`].
pub fn clause_set_tptp_depth_info_add(
    bank: &TermBank,
    set: &ClauseSet,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    for clause in set.iter() {
        clause_tptp_depth_info_add(bank, clause, depthmax, depthsum, count);
    }
    *depthmax
}

#[must_use]
pub fn clause_set_count_unorientable_literals(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_unorientable_literals).sum()
}

#[must_use]
pub fn clause_set_count_eqn_literals(set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| usize_to_i64(clause.prop_lit_number(EP_IS_EQU_LITERAL)))
        .sum()
}

#[must_use]
pub fn clause_set_max_standard_weight(set: &ClauseSet) -> i64 {
    set.find_max_standard_weight()
        .map_or(-1, Clause::standard_weight)
}

#[must_use]
pub fn clause_set_term_cells(bank: &TermBank, set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| {
            clause_weight_to_i64(clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 1, 1.0, false))
        })
        .sum()
}

#[must_use]
pub fn clause_set_max_literal_number(set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| usize_to_i64(clause.literal_number()))
        .max()
        .unwrap_or(0)
}

fn count_clauses<F>(set: &ClauseSet, predicate: F) -> i64
where
    F: Fn(&Clause) -> bool,
{
    usize_to_i64(set.iter().filter(|clause| predicate(clause)).count())
}

fn fcode_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("f-code must fit feature-array index"))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_possible_truncation)]
fn clause_weight_to_i64(weight: f64) -> i64 {
    weight as i64
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_axioms_are_horn, clause_set_axioms_are_unit,
        clause_set_collect_arity_information, clause_set_count_axioms,
        clause_set_count_eqn_literals, clause_set_count_equational, clause_set_count_goals,
        clause_set_count_ground, clause_set_count_ground_goals,
        clause_set_count_ground_positive_axioms, clause_set_count_ground_unit_axioms,
        clause_set_count_horn, clause_set_count_horn_axioms, clause_set_count_horn_goals,
        clause_set_count_maximal_literals, clause_set_count_maximal_terms,
        clause_set_count_non_ground_unit_axioms, clause_set_count_pos_units,
        clause_set_count_positive_axioms, clause_set_count_pure_equational,
        clause_set_count_range_restricted, clause_set_count_singletons, clause_set_count_unit,
        clause_set_count_unit_axioms, clause_set_count_unit_goals,
        clause_set_count_unorientable_literals, clause_set_count_variables,
        clause_set_goals_are_ground, clause_set_goals_are_horn, clause_set_goals_are_unit,
        clause_set_is_equational, clause_set_is_equational_set, clause_set_is_ground,
        clause_set_is_horn_set, clause_set_is_pure_equational_set, clause_set_is_unit_set,
        clause_set_max_literal_number, clause_set_max_standard_weight,
        clause_set_non_ground_axiom_part, clause_set_term_cells, clause_set_tptp_depth_info_add,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::clausefeatures::{
        clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
        clause_count_unorientable_literals, clause_count_variable_set, clause_tptp_depth_info_add,
    };
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        TermBank::new(signature).unwrap_or_else(|err| panic!("{err}"))
    }

    fn individual(bank: &TermBank) -> Type {
        bank.signature().type_bank().default_type()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_predicate_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_, bool_type.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bool_type));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_var(bank: &TermBank, f_code: FunCode) -> Term {
        bank.vars().var_assert_alloc(f_code, &individual(bank))
    }

    fn equation(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term) -> Eqn {
        let mut literal = Eqn::create_true_lit(bank).unwrap_or_else(|err| panic!("{err}"));
        literal.set_left_raw(atom.clone());
        literal
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn polarity_unit_horn_ground_and_range_counts_match_clause_macros() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let positive_ground_unit = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let negative_var_unit = clause_from(vec![equation(&mut bank, &fx, &a, false)]);
        let positive_two_literal = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &c, true),
        ]);
        let mixed_range_restricted = clause_from(vec![
            equation(&mut bank, &fx, &a, true),
            equation(&mut bank, &x, &b, false),
        ]);
        let set = ClauseSet::from_clauses([
            positive_ground_unit,
            negative_var_unit,
            positive_two_literal,
            mixed_range_restricted,
        ]);

        assert_eq!(clause_set_count_goals(&set), 1);
        assert_eq!(clause_set_count_axioms(&set), 3);
        assert_eq!(clause_set_count_unit(&set), 2);
        assert_eq!(clause_set_count_unit_goals(&set), 1);
        assert_eq!(clause_set_count_unit_axioms(&set), 1);
        assert!(!clause_set_is_unit_set(&set));
        assert!(!clause_set_axioms_are_unit(&set));
        assert!(clause_set_goals_are_unit(&set));
        assert_eq!(clause_set_count_horn(&set), 3);
        assert_eq!(clause_set_count_horn_goals(&set), 1);
        assert_eq!(clause_set_count_horn_axioms(&set), 2);
        assert!(!clause_set_is_horn_set(&set));
        assert!(!clause_set_axioms_are_horn(&set));
        assert!(clause_set_goals_are_horn(&set));
        assert_eq!(clause_set_count_ground(&set), 2);
        assert_eq!(clause_set_count_ground_goals(&set), 0);
        assert!(!clause_set_goals_are_ground(&set));
        assert!(!clause_set_is_ground(&set));
        assert_eq!(clause_set_count_positive_axioms(&set), 2);
        assert_eq!(clause_set_count_ground_positive_axioms(&set), 2);
        assert_eq!(clause_set_count_pos_units(&set), 1);
        assert_eq!(clause_set_count_ground_unit_axioms(&set), 1);
        assert_eq!(clause_set_count_non_ground_unit_axioms(&set), 0);
        assert!(clause_set_non_ground_axiom_part(&set).abs() < f64::EPSILON);
        assert_eq!(clause_set_count_range_restricted(&set), 3);
    }

    #[test]
    fn equational_counts_distinguish_clause_predicates_from_literal_property_bits() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let p = typed_predicate_binary(&mut bank, "p", &a, &b);
        let equation_clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let predicate_clause = clause_from(vec![predicate_literal(&mut bank, &p)]);
        let mixed_clause = clause_from(vec![
            equation(&mut bank, &b, &a, false),
            predicate_literal(&mut bank, &p),
        ]);
        let set = ClauseSet::from_clauses([equation_clause, predicate_clause, mixed_clause]);

        assert_eq!(clause_set_count_equational(&bank, &set), 2);
        assert!(clause_set_is_equational(&bank, &set));
        assert!(!clause_set_is_equational_set(&bank, &set));
        assert_eq!(clause_set_count_pure_equational(&bank, &set), 1);
        assert!(!clause_set_is_pure_equational_set(&bank, &set));
        assert_eq!(clause_set_count_eqn_literals(&set), 2);
    }

    #[test]
    fn arity_information_uses_seen_non_special_symbols() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let hab = typed_binary(&mut bank, "h", &a, &b);
        let p = typed_predicate_binary(&mut bank, "p", &hab, &fa);
        let set = ClauseSet::from_clauses([clause_from(vec![
            equation(&mut bank, &hab, &fa, true),
            predicate_literal(&mut bank, &p),
        ])]);

        let info = clause_set_collect_arity_information(&set, bank.signature());

        assert_eq!(info.fun_const_count, 2);
        assert_eq!(info.non_const_funs, 2);
        assert_eq!(info.max_fun_arity, 2);
        assert_eq!(info.sum_fun_arity, 3);
        assert_eq!(info.avg_fun_arity, 1);
        assert_eq!(info.non_const_preds, 1);
        assert_eq!(info.max_pred_arity, 2);
        assert_eq!(info.sum_pred_arity, 2);
        assert_eq!(info.avg_pred_arity, 2);
    }

    #[test]
    fn aggregate_literal_variable_depth_and_size_features_sum_clause_helpers() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let mut first_lit = equation(&mut bank, &fx, &a, true);
        first_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let mut second_lit = equation(&mut bank, &b, &c, false);
        second_lit.set_prop(EP_IS_MAXIMAL);
        let first = clause_from(vec![first_lit, second_lit]);
        let second = clause_from(vec![equation(&mut bank, &x, &b, false)]);
        let set = ClauseSet::from_clauses([first, second]);
        let expected_max_terms: i64 = set.iter().map(clause_count_maximal_terms).sum();
        let expected_max_literals: i64 = set.iter().map(clause_count_maximal_literals).sum();
        let expected_unorientable: i64 = set.iter().map(clause_count_unorientable_literals).sum();
        let expected_variables: i64 = set.iter().map(clause_count_variable_set).sum();
        let expected_singletons: i64 = set.iter().map(clause_count_singleton_set).sum();
        let expected_term_cells: i64 = set
            .iter()
            .map(|clause| {
                super::clause_weight_to_i64(
                    clause.literal_weight(&bank, 1.0, 1.0, 1.0, 1, 1, 1.0, false),
                )
            })
            .sum();
        let mut expected_depthmax = 0;
        let mut expected_depthsum = 0;
        let mut expected_count = 0;
        for clause in set.iter() {
            clause_tptp_depth_info_add(
                &bank,
                clause,
                &mut expected_depthmax,
                &mut expected_depthsum,
                &mut expected_count,
            );
        }

        assert_eq!(clause_set_count_maximal_terms(&set), expected_max_terms);
        assert_eq!(
            clause_set_count_maximal_literals(&set),
            expected_max_literals
        );
        assert_eq!(
            clause_set_count_unorientable_literals(&set),
            expected_unorientable
        );
        assert_eq!(clause_set_count_variables(&set), expected_variables);
        assert_eq!(clause_set_count_singletons(&set), expected_singletons);
        assert_eq!(clause_set_term_cells(&bank, &set), expected_term_cells);
        assert_eq!(
            clause_set_max_standard_weight(&set),
            set.find_max_standard_weight()
                .map_or(-1, Clause::standard_weight)
        );
        assert_eq!(clause_set_max_literal_number(&set), 2);

        let mut depthmax = 0;
        let mut depthsum = 0;
        let mut count = 0;
        assert_eq!(
            clause_set_tptp_depth_info_add(&bank, &set, &mut depthmax, &mut depthsum, &mut count,),
            expected_depthmax
        );
        assert_eq!(depthsum, expected_depthsum);
        assert_eq!(count, expected_count);
    }
}
