use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::subsumption::{
    clause_is_subsume_ordered, clause_subsume_order_sort_lits, clause_subsumes_clause,
};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use std::sync::atomic::{AtomicI64, Ordering};

static CONDENSATION_ATTEMPTS: AtomicI64 = AtomicI64::new(0);
static CONDENSATION_SUCCESSES: AtomicI64 = AtomicI64::new(0);

pub type CondenseFun = fn(&mut Clause, &mut TermBank) -> Result<bool, Diagnostic>;

#[must_use]
pub fn condensation_attempts() -> i64 {
    CONDENSATION_ATTEMPTS.load(Ordering::SeqCst)
}

#[must_use]
pub fn condensation_successes() -> i64 {
    CONDENSATION_SUCCESSES.load(Ordering::SeqCst)
}

fn try_condensation(
    clause: &mut Clause,
    first_index: usize,
    second_index: usize,
    _swap: bool,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let Some(first) = clause.literals().as_slice().get(first_index).cloned() else {
        return Ok(false);
    };
    let Some(mut second) = clause.literals().as_slice().get(second_index).cloned() else {
        return Ok(false);
    };

    let mut subst = Substitution::new();
    if !first.literal_unify_one_way(&mut second, &mut subst, false) {
        return Ok(false);
    }

    let mut new_literals = match clause
        .literals()
        .copy_except_index(Some(second_index), bank)
    {
        Ok(literals) => literals,
        Err(error) => {
            subst.backtrack();
            return Err(error);
        }
    };
    subst.backtrack();

    new_literals.remove_duplicates(bank);
    new_literals.remove_resolved(bank);

    let mut candidate = Clause::alloc(new_literals);
    candidate.set_weight(candidate.standard_weight());
    clause_subsume_order_sort_lits(&mut candidate, bank);

    if clause_subsumes_clause(&candidate, clause, bank) {
        clause.replace_literals(candidate.into_literals());
        clause.set_weight(clause.standard_weight());
        return Ok(true);
    }
    Ok(false)
}

/// Tries to condense a clause once.
///
/// # Panics
///
/// Panics if `clause` is not subsumption-ordered or if the clause weight is
/// not its standard weight, matching the C preconditions reached through
/// `CondenseOnce` and `ClauseSubsumesClause`.
pub fn condense_once(clause: &mut Clause, bank: &mut TermBank) -> Result<bool, Diagnostic> {
    assert!(clause_is_subsume_ordered(clause, bank));

    let literal_count = clause.literal_number();
    for first_index in 0..literal_count {
        for second_index in first_index + 1..literal_count {
            if try_condensation(clause, first_index, second_index, false, bank)? {
                return Ok(true);
            }
            let needs_swapped_retry = {
                let literals = clause.literals().as_slice();
                !literals[first_index].is_oriented() || !literals[second_index].is_oriented()
            };
            if needs_swapped_retry
                && try_condensation(clause, first_index, second_index, true, bank)?
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn condense(clause: &mut Clause, bank: &mut TermBank) -> Result<bool, Diagnostic> {
    CONDENSATION_ATTEMPTS.fetch_add(1, Ordering::SeqCst);

    let mut result = false;
    if clause.positive_literal_count() > 1 || clause.negative_literal_count() > 1 {
        clause.set_weight(clause.standard_weight());
        clause_subsume_order_sort_lits(clause, bank);
        while condense_once(clause, bank)? {
            result = true;
        }
        if result {
            CONDENSATION_SUCCESSES.fetch_add(1, Ordering::SeqCst);
        }
    }
    Ok(result)
}

#[cfg(test)]
fn reset_condensation_counters() {
    CONDENSATION_ATTEMPTS.store(0, Ordering::SeqCst);
    CONDENSATION_SUCCESSES.store(0, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::{
        condensation_attempts, condensation_successes, condense, condense_once,
        reset_condensation_counters,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::subsumption::{clause_is_subsume_ordered, clause_subsume_order_sort_lits};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;
    use std::sync::{Mutex, MutexGuard};

    static COUNTER_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_counter_tests() -> MutexGuard<'static, ()> {
        COUNTER_TEST_LOCK.lock().unwrap()
    }

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
    fn condense_once_replaces_clause_with_subsuming_factor_and_simplifies_result() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let positive_pattern = literal(&mut bank, &variable, &a, true);
        let positive_instance = literal(&mut bank, &b, &a, true);
        let resolved_after_substitution = literal(&mut bank, &variable, &b, false);
        let expected_literal = literal(&mut bank, &b, &a, true);
        let mut clause = clause_from(vec![
            positive_pattern,
            positive_instance,
            resolved_after_substitution,
        ]);
        prepare(&mut clause, &bank);

        assert!(condense_once(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 1);
        assert_eq!(clause.positive_literal_count(), 1);
        assert_eq!(clause.negative_literal_count(), 0);
        assert!(clause.literals().as_slice()[0].literal_equal(&expected_literal));
        assert_eq!(clause.weight(), clause.standard_weight());
        assert!(clause_is_subsume_ordered(&clause, &bank));
    }

    #[test]
    fn condense_repeats_until_no_further_factor_is_found_and_updates_counters() {
        let _guard = lock_counter_tests();
        reset_condensation_counters();
        let mut bank = test_bank();
        let first_var = typed_var(&bank, -10);
        let second_var = typed_var(&bank, -12);
        let first_const = typed_const(&mut bank, "a");
        let second_const = typed_const(&mut bank, "b");
        let third_const = typed_const(&mut bank, "c");
        let fourth_const = typed_const(&mut bank, "d");
        let mut clause = clause_from(vec![
            literal(&mut bank, &first_var, &first_const, true),
            literal(&mut bank, &second_const, &first_const, true),
            literal(&mut bank, &second_var, &third_const, false),
            literal(&mut bank, &fourth_const, &third_const, false),
        ]);

        assert!(condense(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 2);
        assert_eq!(clause.positive_literal_count(), 1);
        assert_eq!(clause.negative_literal_count(), 1);
        assert_eq!(condensation_attempts(), 1);
        assert_eq!(condensation_successes(), 1);
        assert_eq!(clause.weight(), clause.standard_weight());
    }

    #[test]
    fn condense_counts_attempts_even_when_clause_is_too_small() {
        let _guard = lock_counter_tests();
        reset_condensation_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "a");
        let mut clause = clause_from(vec![literal(&mut bank, &x, &a, true)]);

        assert!(!condense(&mut clause, &mut bank).unwrap());

        assert_eq!(condensation_attempts(), 1);
        assert_eq!(condensation_successes(), 0);
    }

    #[test]
    fn condense_once_preserves_c_ignored_swap_argument() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut clause = clause_from(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &a, &b, true),
        ]);
        prepare(&mut clause, &bank);

        assert!(!condense_once(&mut clause, &mut bank).unwrap());

        assert_eq!(clause.literal_number(), 2);
        assert_eq!(clause.weight(), clause.standard_weight());
    }
}
