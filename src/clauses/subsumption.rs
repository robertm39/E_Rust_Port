use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_SOS, CP_LIMITED_RW};
use crate::clauses::clausefunc::clause_remove_literal_index;
use crate::clauses::clausesets::ClauseSet;
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

/// Returns a unit clause from `set` that subsumes one literal of `clause`.
///
/// This plain-set path mirrors `UnitClauseSetSubsumesClause` without using the
/// C demodulator index.
#[must_use]
pub fn unit_clause_set_subsumes_clause<'set>(
    set: &'set ClauseSet,
    clause: &Clause,
) -> Option<&'set Clause> {
    clause.literals().as_slice().iter().find_map(|literal| {
        if literal.is_positive() {
            find_positive_unit_simplifier(set, literal.left(), literal.right())
        } else {
            find_negative_top_unit_simplifier(set, literal.left(), literal.right())
        }
    })
}

/// Returns the first clause at or after `start_index` subsumed by a unit clause.
///
/// # Panics
///
/// Panics if `subsumer` is not unit, matching `ClauseSetFindUnitSubsumedClause`.
#[must_use]
pub fn clause_set_find_unit_subsumed_clause<'set>(
    set: &'set ClauseSet,
    start_index: usize,
    subsumer: &Clause,
) -> Option<&'set Clause> {
    assert_eq!(
        subsumer.literal_number(),
        1,
        "unit subsumption requires one literal"
    );
    set.iter()
        .skip(start_index)
        .find(|candidate| unit_clause_subsumes_clause(subsumer, candidate))
}

/// Removes negative literals simplified by positive unit clauses in `set`.
///
/// This plain-set path preserves the C mutation semantics but uses linear
/// search until demodulator indexes are available.
#[must_use]
pub fn clause_positive_simplify_reflect(set: &ClauseSet, clause: &mut Clause) -> bool {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier_sos = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_positive() {
                None
            } else {
                find_positive_unit_simplifier(set, literal.left(), literal.right())
                    .map(Clause::is_sos)
            }
        };

        if let Some(simplifier_sos) = simplifier_sos {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
        } else {
            index += 1;
        }
    }
    clause.is_empty()
}

/// Removes positive literals simplified by negative unit clauses in `set`.
///
/// This plain-set path preserves the C mutation semantics but uses linear
/// search until demodulator indexes are available.
#[must_use]
pub fn clause_negative_simplify_reflect(set: &ClauseSet, clause: &mut Clause) -> bool {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier_sos = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_negative() {
                None
            } else {
                find_negative_top_unit_simplifier(set, literal.left(), literal.right())
                    .map(Clause::is_sos)
            }
        };

        if let Some(simplifier_sos) = simplifier_sos {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
        } else {
            index += 1;
        }
    }
    clause.is_empty()
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

/// Returns the first clause in `set` that subsumes `sub_candidate`.
///
/// This is the plain clause-set path used by `ClauseSetSubsumesClause` when no
/// feature-vector index is attached.
///
/// # Panics
///
/// Panics if `sub_candidate` is unit, if the candidate or any checked set
/// clause is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight, matching the C fallback preconditions.
#[must_use]
pub fn clause_set_subsumes_clause<'set>(
    set: &'set ClauseSet,
    sub_candidate: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    assert!(
        sub_candidate.literal_number() > 1,
        "plain ClauseSetSubsumesClause expects a non-unit candidate"
    );
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    set.iter()
        .find(|candidate| clause_subsumes_clause(candidate, sub_candidate, bank))
}

/// Returns the first clause at or after `start_index` subsumed by `subsumer`.
///
/// The index models C's linked-list `set_position` until stable clause handles
/// replace the plain owner.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_subsumed_clause<'set>(
    set: &'set ClauseSet,
    start_index: usize,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    set.iter()
        .skip(start_index)
        .find(|candidate| clause_subsumes_clause(subsumer, candidate, bank))
}

/// Pushes every clause in `set` subsumed by `subsumer` onto `result`.
///
/// Returns the number of newly pushed clauses, matching
/// `ClauseSetFindSubsumedClauses`' stack-pointer delta.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
pub fn clause_set_find_subsumed_clauses<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    result: &mut PStack<&'set Clause>,
    bank: &TermBank,
) -> i64 {
    let old_len = result.len();
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    for candidate in set.iter() {
        if clause_subsumes_clause(subsumer, candidate, bank) {
            result.push(candidate);
        }
    }
    usize_to_i64(result.len() - old_len)
}

/// Returns the first clause in `set` subsumed by `subsumer`.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_first_subsumed_clause<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    clause_set_find_subsumed_clause(set, 0, subsumer, bank)
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

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn find_positive_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    set.iter().find(|candidate| {
        candidate
            .literals()
            .as_slice()
            .first()
            .is_some_and(|literal| {
                candidate.is_unit()
                    && literal.is_positive()
                    && eqn_subsumes_termpair(literal, left, right)
            })
    })
}

fn find_negative_top_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    set.iter().find(|candidate| {
        candidate
            .literals()
            .as_slice()
            .first()
            .is_some_and(|literal| {
                candidate.is_unit()
                    && literal.is_negative()
                    && eqn_topsubsumes_termpair(literal, left, right)
            })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        clause_is_subsume_ordered, clause_negative_simplify_reflect,
        clause_positive_simplify_reflect, clause_set_find_first_subsumed_clause,
        clause_set_find_subsumed_clause, clause_set_find_subsumed_clauses,
        clause_set_find_unit_subsumed_clause, clause_set_subsumes_clause,
        clause_subsume_order_sort_lits, clause_subsumes_clause, eqn_subsumes_termpair,
        eqn_topsubsumes_termpair, literal_subsumes_clause, unit_clause_set_subsumes_clause,
        unit_clause_subsumes_clause,
    };
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_IS_SOS, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
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

    #[test]
    fn clause_set_subsumes_clause_returns_first_plain_subsumer() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let expected_right = typed_const(&mut bank, "c");
        let mismatch_left = typed_const(&mut bank, "d");
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &mismatch_left, &expected_right, true),
        ]);
        let mut matching = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut candidate = clause_from(vec![
            literal(&mut bank, &other, &constant, true),
            literal(&mut bank, &other, &expected_right, true),
        ]);
        prepare(&mut non_matching, &bank);
        prepare(&mut matching, &bank);
        prepare(&mut candidate, &bank);
        let matching_id = matching.ident();
        let set = ClauseSet::from_clauses([non_matching, matching]);

        assert_eq!(
            clause_set_subsumes_clause(&set, &candidate, &bank).map(Clause::ident),
            Some(matching_id)
        );
    }

    #[test]
    fn clause_set_find_subsumed_clause_honors_start_index() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut non_matching, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([non_matching, first, second]);

        assert_eq!(
            clause_set_find_subsumed_clause(&set, 0, &subsumer, &bank).map(Clause::ident),
            Some(first_id)
        );
        assert_eq!(
            clause_set_find_subsumed_clause(&set, 2, &subsumer, &bank).map(Clause::ident),
            Some(second_id)
        );
        assert!(clause_set_find_subsumed_clause(&set, 3, &subsumer, &bank).is_none());
        assert_eq!(
            clause_set_find_first_subsumed_clause(&set, &subsumer, &bank).map(Clause::ident),
            Some(first_id)
        );
    }

    #[test]
    fn clause_set_find_subsumed_clauses_pushes_all_and_returns_new_count() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut non_matching, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let non_matching_id = non_matching.ident();
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([non_matching, first, second]);
        let mut result = PStack::new();
        result.push(set.iter().next().unwrap());

        assert_eq!(
            clause_set_find_subsumed_clauses(&set, &subsumer, &mut result, &bank),
            2
        );

        assert_eq!(
            result
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![non_matching_id, first_id, second_id]
        );
    }

    #[test]
    fn unit_clause_set_subsumes_clause_returns_first_matching_unit() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let positive_id = positive_unit.ident();
        let negative_unit = clause_from(vec![literal(&mut bank, &variable, &constant, false)]);
        let negative_id = negative_unit.ident();
        let set = ClauseSet::from_clauses([positive_unit, negative_unit]);
        let positive_candidate = clause_from(vec![literal(&mut bank, &left, &right, true)]);
        let negative_candidate = clause_from(vec![literal(&mut bank, &other, &constant, false)]);

        assert_eq!(
            unit_clause_set_subsumes_clause(&set, &positive_candidate).map(Clause::ident),
            Some(positive_id)
        );
        assert_eq!(
            unit_clause_set_subsumes_clause(&set, &negative_candidate).map(Clause::ident),
            Some(negative_id)
        );
    }

    #[test]
    fn clause_set_find_unit_subsumed_clause_honors_start_index() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let first_witness = typed_const(&mut bank, "b");
        let second_witness = typed_const(&mut bank, "c");
        let mut unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let mut first = clause_from(vec![literal(&mut bank, &first_witness, &constant, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &second_witness, &constant, true)]);
        prepare(&mut unit, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([first, second]);

        assert_eq!(
            clause_set_find_unit_subsumed_clause(&set, 0, &unit).map(Clause::ident),
            Some(first_id)
        );
        assert_eq!(
            clause_set_find_unit_subsumed_clause(&set, 1, &unit).map(Clause::ident),
            Some(second_id)
        );
        assert!(clause_set_find_unit_subsumed_clause(&set, 2, &unit).is_none());
    }

    #[test]
    fn positive_simplify_reflect_removes_negative_literals_and_propagates_sos() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let unrelated = typed_const(&mut bank, "c");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let mut positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        positive_unit.set_prop(CP_IS_SOS);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &unrelated, &constant, true),
        ]);
        target.set_weight(target.standard_weight());
        let original_weight = target.weight();
        let removed_weight = target.literals().as_slice()[1].standard_weight();
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(!clause_positive_simplify_reflect(&set, &mut target));

        assert_eq!(target.positive_literal_count(), 1);
        assert_eq!(target.negative_literal_count(), 0);
        assert_eq!(target.weight(), original_weight - removed_weight);
        assert!(target.query_prop(CP_IS_SOS));
        assert!(!target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
    }

    #[test]
    fn negative_simplify_reflect_uses_top_level_negative_units_only() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let negative_unit = clause_from(vec![literal(&mut bank, &variable, &constant, false)]);
        let set = ClauseSet::from_clauses([negative_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, true),
            literal(&mut bank, &other, &constant, true),
        ]);
        target.set_weight(target.standard_weight());
        let top_literal_weight = target.literals().as_slice()[1].standard_weight();
        let original_weight = target.weight();
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(!clause_negative_simplify_reflect(&set, &mut target));

        assert_eq!(target.positive_literal_count(), 1);
        assert_eq!(target.negative_literal_count(), 0);
        assert_eq!(target.weight(), original_weight - top_literal_weight);
        assert_eq!(target.literals().as_slice()[0].left(), &left);
        assert!(!target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
    }

    #[test]
    fn simplify_reflect_reports_empty_clause_after_last_literal_removed() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let witness = typed_const(&mut bank, "b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &witness, &constant, false)]);
        target.set_weight(target.standard_weight());

        assert!(clause_positive_simplify_reflect(&set, &mut target));

        assert!(target.is_empty());
        assert_eq!(target.weight(), 0);
    }
}
