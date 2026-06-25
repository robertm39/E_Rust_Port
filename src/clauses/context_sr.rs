use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_SOS, CP_LIMITED_RW};
use crate::clauses::clausefunc::{clause_flip_literal_sign_index, clause_remove_literal_index};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EP_IS_POSITIVE;
use crate::clauses::subsumption::{
    clause_set_find_subsumed_clauses, clause_subsume_order_sort_lits, clause_subsumes_clause,
};
use crate::terms::termbanks::TermBank;

/// Performs C `ClauseContextualSimplifyReflect` over a plain clause set.
///
/// Derivation and proof-documentation side effects are left to the future
/// proof-control integration layer.
pub fn clause_contextual_simplify_reflect(
    set: &ClauseSet,
    clause: &mut Clause,
    bank: &TermBank,
) -> usize {
    let mut result = 0;
    let mut literal_stack = literal_stack(clause);
    clause.set_weight(clause.standard_weight());

    while let Some(literal) = literal_stack.pop() {
        let flipped = flipped_literal(&literal);
        if !flip_literal_sign(clause, &literal) {
            continue;
        }
        clause_subsume_order_sort_lits(clause, bank);

        if let Some(subsumer) = clause_set_subsumes_context_clause(set, clause, bank) {
            if subsumer.is_sos() {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            let removed = remove_literal(clause, &flipped);
            debug_assert!(
                removed.is_some(),
                "flipped literal must still be present after subsumption"
            );
            debug_assert_eq!(clause.weight(), clause.standard_weight());
            result += 1;
        } else {
            let restored = flip_literal_sign(clause, &flipped);
            debug_assert!(restored, "flipped literal must be restored if kept");
        }
    }

    result
}

/// Pushes clauses that C `ClauseSetFindContextSRClauses` would find.
///
/// A target clause may be pushed more than once when different flipped literals
/// make the query subsume it, matching the C stack behavior.
pub fn clause_set_find_context_sr_clauses<'set>(
    set: &'set ClauseSet,
    clause: &mut Clause,
    result: &mut PStack<&'set Clause>,
    bank: &TermBank,
) -> i64 {
    let old_len = result.len();
    let mut literal_stack = literal_stack(clause);
    assert_eq!(clause.weight(), clause.standard_weight());

    while let Some(literal) = literal_stack.pop() {
        let flipped = flipped_literal(&literal);
        if !flip_literal_sign(clause, &literal) {
            continue;
        }
        clause_subsume_order_sort_lits(clause, bank);
        clause_set_find_subsumed_clauses(set, clause, result, bank);
        let restored = flip_literal_sign(clause, &flipped);
        debug_assert!(restored, "flipped literal must be restored after lookup");
    }

    usize_to_i64(result.len() - old_len)
}

fn clause_set_subsumes_context_clause<'set>(
    set: &'set ClauseSet,
    clause: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    set.iter()
        .find(|candidate| clause_subsumes_clause(candidate, clause, bank))
}

fn literal_stack(clause: &Clause) -> Vec<Eqn> {
    clause.literals().as_slice().to_vec()
}

fn flipped_literal(literal: &Eqn) -> Eqn {
    let mut flipped = literal.clone();
    flipped.flip_prop(EP_IS_POSITIVE);
    flipped
}

fn flip_literal_sign(clause: &mut Clause, literal: &Eqn) -> bool {
    let Some(index) = clause
        .literals()
        .as_slice()
        .iter()
        .position(|candidate| same_literal_ignoring_position(candidate, literal))
    else {
        return false;
    };
    clause_flip_literal_sign_index(clause, index)
}

fn remove_literal(clause: &mut Clause, literal: &Eqn) -> Option<Eqn> {
    let index = clause
        .literals()
        .as_slice()
        .iter()
        .position(|candidate| same_literal_ignoring_position(candidate, literal))?;
    clause_remove_literal_index(clause, index)
}

fn same_literal_ignoring_position(left: &Eqn, right: &Eqn) -> bool {
    left.properties() == right.properties()
        && left.left() == right.left()
        && left.right() == right.right()
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{clause_contextual_simplify_reflect, clause_set_find_context_sr_clauses};
    use crate::basics::pstacks::PStack;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_IS_SOS, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::subsumption::clause_subsume_order_sort_lits;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn predicate_atom(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, bool_type.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        Eqn::alloc(atom.clone(), bank.true_term().clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn prepare(mut clause: Clause, bank: &TermBank, ident: i64) -> Clause {
        clause.set_ident(ident);
        clause.set_weight(clause.standard_weight());
        clause_subsume_order_sort_lits(&mut clause, bank);
        clause
    }

    #[test]
    fn contextual_simplify_reflect_removes_flipped_subsumed_literal() {
        let mut bank = test_bank();
        let p = predicate_atom(&mut bank, "p");
        let q = predicate_atom(&mut bank, "q");
        let mut subsumer = prepare(
            clause_from(vec![
                predicate_literal(&mut bank, &p, true),
                predicate_literal(&mut bank, &q, false),
            ]),
            &bank,
            10,
        );
        subsumer.set_prop(CP_IS_SOS);
        let set = ClauseSet::from_clauses([subsumer]);
        let mut target = clause_from(vec![
            predicate_literal(&mut bank, &p, true),
            predicate_literal(&mut bank, &q, true),
        ]);
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        let removed = clause_contextual_simplify_reflect(&set, &mut target, &bank);

        assert_eq!(removed, 1);
        assert_eq!(target.literal_number(), 1);
        assert!(target.literals().as_slice()[0].is_positive());
        assert_eq!(target.literals().as_slice()[0].left(), &p);
        assert!(target.query_prop(CP_IS_SOS));
        assert!(!target.is_any_prop_set(CP_INITIAL | CP_LIMITED_RW));
        assert_eq!(target.weight(), target.standard_weight());
    }

    #[test]
    fn contextual_simplify_reflect_restores_literal_when_no_subsumer_exists() {
        let mut bank = test_bank();
        let p = predicate_atom(&mut bank, "p_nohit");
        let q = predicate_atom(&mut bank, "q_nohit");
        let set = ClauseSet::default();
        let mut target = clause_from(vec![
            predicate_literal(&mut bank, &p, true),
            predicate_literal(&mut bank, &q, true),
        ]);
        let original_positive = target
            .literals()
            .as_slice()
            .iter()
            .filter(|literal| literal.is_positive())
            .count();

        let removed = clause_contextual_simplify_reflect(&set, &mut target, &bank);

        assert_eq!(removed, 0);
        assert_eq!(target.literal_number(), 2);
        assert_eq!(
            target
                .literals()
                .as_slice()
                .iter()
                .filter(|literal| literal.is_positive())
                .count(),
            original_positive
        );
        assert_eq!(target.weight(), target.standard_weight());
    }

    #[test]
    fn find_context_sr_clauses_preserves_c_duplicate_push_behavior() {
        let mut bank = test_bank();
        let p = predicate_atom(&mut bank, "p_find");
        let q = predicate_atom(&mut bank, "q_find");
        let mut candidate = prepare(
            clause_from(vec![
                predicate_literal(&mut bank, &p, true),
                predicate_literal(&mut bank, &q, true),
                predicate_literal(&mut bank, &p, false),
                predicate_literal(&mut bank, &q, false),
            ]),
            &bank,
            20,
        );
        candidate.set_weight(candidate.standard_weight());
        let candidate_id = candidate.ident();
        let set = ClauseSet::from_clauses([candidate]);
        let mut query = clause_from(vec![
            predicate_literal(&mut bank, &p, true),
            predicate_literal(&mut bank, &q, true),
        ]);
        query.set_weight(query.standard_weight());
        let mut result = PStack::new();

        let pushed = clause_set_find_context_sr_clauses(&set, &mut query, &mut result, &bank);

        assert_eq!(pushed, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result.as_slice()[0].ident(), candidate_id);
        assert_eq!(result.as_slice()[1].ident(), candidate_id);
        assert_eq!(query.literal_number(), 2);
        assert!(query.literals().as_slice().iter().all(Eqn::is_positive));
        assert_eq!(query.weight(), query.standard_weight());
    }
}
