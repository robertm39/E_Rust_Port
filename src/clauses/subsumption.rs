use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

#[must_use]
pub fn eqn_topsubsumes_termpair(eqn: &Eqn, left: &Term, right: &Term) -> bool {
    let mut subst = Substitution::new();
    let result = (subst_match_complete(eqn.left(), left, &mut subst)
        && subst_match_complete(eqn.right(), right, &mut subst))
        || {
            subst.backtrack();
            subst_match_complete(eqn.left(), right, &mut subst)
                && subst_match_complete(eqn.right(), left, &mut subst)
        };
    subst.backtrack();
    result
}

/// Returns whether `eqn` subsumes the term pair, following the C descent
/// through equal top contexts with at most one differing argument pair.
///
/// # Panics
///
/// Panics if equal-headed non-phony terms have different arities, matching the
/// non-LFHO C assertion in `eqn_subsumes_termpair`.
#[must_use]
pub fn eqn_subsumes_termpair(eqn: &Eqn, left: &Term, right: &Term) -> bool {
    let mut current_left = left.clone();
    let mut current_right = right.clone();

    loop {
        if eqn_topsubsumes_termpair(eqn, &current_left, &current_right) {
            return true;
        }
        if current_left.is_phony_app()
            || current_right.is_phony_app()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            return false;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "non-LFHO subsumption descent expects equal arities"
        );

        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return false;
                }
                differing_pair = Some((next_left, next_right));
            }
        }
        let Some((next_left, next_right)) = differing_pair else {
            return true;
        };
        current_left = next_left;
        current_right = next_right;
    }
}

#[must_use]
pub fn literal_subsumes_clause(literal: &Eqn, clause: &Clause) -> bool {
    clause.literals().as_slice().iter().any(|candidate| {
        if literal.is_positive() {
            candidate.is_positive()
                && eqn_subsumes_termpair(literal, candidate.left(), candidate.right())
        } else {
            candidate.is_negative()
                && eqn_topsubsumes_termpair(literal, candidate.left(), candidate.right())
        }
    })
}

/// Returns whether a unit clause subsumes `clause`.
///
/// # Panics
///
/// Panics if `unit` does not contain exactly one literal, matching
/// `UnitClauseSubsumesClause`.
#[must_use]
pub fn unit_clause_subsumes_clause(unit: &Clause, clause: &Clause) -> bool {
    assert_eq!(
        unit.literal_number(),
        1,
        "unit subsumption requires one literal"
    );
    literal_subsumes_clause(&unit.literals().as_slice()[0], clause)
}

pub fn clause_subsume_order_sort_lits(clause: &mut Clause, bank: &TermBank) {
    clause.sort_literals_by(|left, right| {
        i64::from(left.subsume_inverse_refined_compare(right, bank))
    });
}

#[must_use]
pub fn clause_is_subsume_ordered(clause: &Clause, bank: &TermBank) -> bool {
    clause.is_sorted_by(|left, right| i64::from(left.subsume_inverse_compare(right, bank)))
}

/// Returns whether `subsumer` subsumes `sub_candidate`.
///
/// # Panics
///
/// Panics if either clause is not subsumption ordered or if either cached
/// clause weight does not match `ClauseStandardWeight`, matching the C
/// preconditions on `ClauseSubsumesClause`.
#[must_use]
pub fn clause_subsumes_clause(subsumer: &Clause, sub_candidate: &Clause, bank: &TermBank) -> bool {
    assert!(clause_is_subsume_ordered(subsumer, bank));
    assert!(clause_is_subsume_ordered(sub_candidate, bank));
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    assert_eq!(subsumer.weight(), subsumer.standard_weight());

    if subsumer.is_empty() {
        return true;
    }
    if subsumer.is_unit() {
        return unit_clause_subsumes_clause(subsumer, sub_candidate);
    }
    if subsumer.positive_literal_count() > sub_candidate.positive_literal_count()
        || subsumer.negative_literal_count() > sub_candidate.negative_literal_count()
        || subsumer.weight() > sub_candidate.weight()
    {
        return false;
    }
    if (sub_candidate.positive_literal_count() >= 3 || sub_candidate.negative_literal_count() >= 3)
        && !check_subsumption_possibility(subsumer, sub_candidate, bank)
    {
        return false;
    }

    let mut subst = Substitution::new();
    let mut picked = vec![false; sub_candidate.literal_number()];
    let result = eqn_list_rec_subsume(
        subsumer.literals().as_slice(),
        sub_candidate.literals().as_slice(),
        &mut subst,
        &mut picked,
        bank,
    );
    subst.backtrack();
    result
}

fn check_subsumption_possibility(
    subsumer: &Clause,
    sub_candidate: &Clause,
    bank: &TermBank,
) -> bool {
    subsumer
        .literals()
        .as_slice()
        .iter()
        .all(|literal| find_spec_literal(literal, sub_candidate.literals().as_slice(), bank))
}

fn find_spec_literal(literal: &Eqn, candidates: &[Eqn], bank: &TermBank) -> bool {
    let mut subst = Substitution::new();
    for candidate in candidates {
        let cmp = literal.subsume_q_order_compare(candidate, bank);
        if cmp > 0 {
            return false;
        }
        if cmp < 0 {
            continue;
        }
        if literal.standard_weight() > candidate.standard_weight() {
            return false;
        }
        if literal_matches_with_subst(literal, candidate, &mut subst) {
            subst.backtrack();
            return true;
        }
        subst.backtrack();
    }
    false
}

fn eqn_list_rec_subsume(
    subsum_list: &[Eqn],
    sub_cand_list: &[Eqn],
    subst: &mut Substitution,
    picked: &mut [bool],
    bank: &TermBank,
) -> bool {
    let Some((literal, remaining)) = subsum_list.split_first() else {
        return true;
    };

    for (index, candidate) in sub_cand_list.iter().enumerate() {
        if picked[index] {
            continue;
        }
        let cmp = candidate.subsume_q_order_compare(literal, bank);
        if cmp < 0 {
            return false;
        }
        if cmp > 0 {
            continue;
        }
        if candidate.standard_weight() < literal.standard_weight() {
            return false;
        }
        if literal.is_oriented() && !candidate.is_oriented() {
            continue;
        }

        picked[index] = true;
        let state = subst.len();
        if literal_matches_with_subst(literal, candidate, subst)
            && eqn_list_rec_subsume(remaining, sub_cand_list, subst, picked, bank)
        {
            return true;
        }
        subst.backtrack_to_pos(state);
        picked[index] = false;
    }
    false
}

fn literal_matches_with_subst(pattern: &Eqn, candidate: &Eqn, subst: &mut Substitution) -> bool {
    pattern.subsume(candidate, subst)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_is_subsume_ordered, clause_subsume_order_sort_lits, clause_subsumes_clause,
        eqn_subsumes_termpair, eqn_topsubsumes_termpair, literal_subsumes_clause,
        unit_clause_subsumes_clause,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn prepare(clause: &mut Clause, bank: &TermBank) {
        clause.set_weight(clause.standard_weight());
        clause_subsume_order_sort_lits(clause, bank);
    }

    #[test]
    fn positive_unit_subsumption_descends_through_single_differing_subterm_pair() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let unit_lit = literal(&mut bank, &variable, &constant, true);
        let candidate_lit = literal(&mut bank, &left, &right, true);
        let candidate_clause = clause_from(vec![candidate_lit]);

        assert!(eqn_subsumes_termpair(&unit_lit, &left, &right));
        assert!(literal_subsumes_clause(&unit_lit, &candidate_clause));
    }

    #[test]
    fn negative_unit_subsumption_checks_only_top_pair() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let unit_lit = literal(&mut bank, &variable, &constant, false);
        let nested_candidate = clause_from(vec![literal(&mut bank, &left, &right, false)]);
        let top_candidate = clause_from(vec![literal(&mut bank, &other, &constant, false)]);

        assert!(!eqn_topsubsumes_termpair(&unit_lit, &left, &right));
        assert!(!literal_subsumes_clause(&unit_lit, &nested_candidate));
        assert!(literal_subsumes_clause(&unit_lit, &top_candidate));
    }

    #[test]
    fn clause_subsumption_uses_shared_substitution_and_strict_literal_picking() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let witness = typed_const(&mut bank, "c");
        let different = typed_const(&mut bank, "d");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &other, true),
        ]);
        let mut matching = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &witness, &other, true),
        ]);
        let mut not_matching = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &different, &other, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut matching, &bank);
        prepare(&mut not_matching, &bank);

        assert!(clause_is_subsume_ordered(&subsumer, &bank));
        assert!(clause_subsumes_clause(&subsumer, &matching, &bank));
        assert!(!clause_subsumes_clause(&subsumer, &not_matching, &bank));
    }

    #[test]
    fn unit_clause_wrapper_requires_one_literal_and_uses_literal_subsumption() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let witness = typed_const(&mut bank, "b");
        let mut unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let mut candidate = clause_from(vec![literal(&mut bank, &witness, &constant, true)]);
        prepare(&mut unit, &bank);
        prepare(&mut candidate, &bank);

        assert!(unit_clause_subsumes_clause(&unit, &candidate));
        assert!(clause_subsumes_clause(&unit, &candidate, &bank));
    }
}
