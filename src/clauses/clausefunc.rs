use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_D_INDEXED, CP_IS_S_INDEXED, CP_LIMITED_RW};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::terms::termbanks::TermBank;
use std::cmp::Ordering;

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

#[must_use]
pub fn clause_canon_compare_ref(left: &Clause, right: &Clause, bank: &TermBank) -> i32 {
    left.cmp_by_struct_weight(right, bank)
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
        clause_canon_compare_ref, clause_flip_literal_sign_index, clause_remove_ac_resolved,
        clause_remove_literal, clause_remove_literal_index, clause_remove_superfluous_literals,
        clause_set_canonize, clause_set_remove_superfluous_literals, clause_unit_simplify_test,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_ORIENTED;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
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

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
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
