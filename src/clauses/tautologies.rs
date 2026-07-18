use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EP_GO_NATURAL, EP_IS_POSITIVE};
#[cfg(test)]
use crate::clauses::eqnlist::EqnList;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_standard_weight, term_struct_equal_no_deref};
use crate::terms::termtypes::{DerefType, Term};

pub const MAX_EQ_TAUTOLOGY_CHECK_LITNO: usize = 1000;

#[must_use]
fn ground_compare(left: &Term, right: &Term) -> CompareResult {
    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((left, right)) = stack.pop() {
        let left_weight = term_standard_weight(&left);
        let right_weight = term_standard_weight(&right);
        if left_weight < right_weight {
            return CompareResult::Lesser;
        }
        if left_weight > right_weight {
            return CompareResult::Greater;
        }
        if left.f_code() < right.f_code() {
            return CompareResult::Lesser;
        }
        if left.f_code() > right.f_code() {
            return CompareResult::Greater;
        }
        if left.arity() < right.arity() {
            return CompareResult::Lesser;
        }
        if left.arity() > right.arity() {
            return CompareResult::Greater;
        }
        assert!(
            problem_type() == ProblemType::HigherOrder || left.arity() == right.arity(),
            "first-order ground comparison expects equal arities after arity comparison"
        );
        for index in 0..left.arity() {
            let left_arg = left.argument(index).unwrap_or_else(|| {
                panic!("ground comparison requires initialized argument {index}")
            });
            let right_arg = right.argument(index).unwrap_or_else(|| {
                panic!("ground comparison requires initialized argument {index}")
            });
            stack.push((left_arg, right_arg));
        }
    }
    CompareResult::Equal
}

fn ground_orient_eqn(eqn: &mut Eqn) -> bool {
    let cmp = ground_compare(eqn.left(), eqn.right());
    assert_ne!(
        cmp,
        CompareResult::Uncomparable,
        "ground comparison is total"
    );
    if cmp == CompareResult::Greater {
        eqn.set_prop(EP_GO_NATURAL);
    } else {
        eqn.del_prop(EP_GO_NATURAL);
    }
    true
}

/// Checks whether a clause is tautological using E's ground-completion test.
///
/// The clause is copied into `work_bank` before normalization, matching
/// `ClauseIsTautology`.
///
/// # Errors
///
/// Returns a diagnostic if copying clause terms into `work_bank` or inserting
/// normalized terms fails.
pub fn clause_is_tautology(work_bank: &mut TermBank, clause: &Clause) -> Result<bool, Diagnostic> {
    clause_is_tautology_real(work_bank, clause, true)
}

/// Checks whether a clause is tautological using E's `ClauseIsTautologyReal` path.
///
/// Rust always copies terms into `work_bank` before normalization. C can consume
/// the caller-owned temporary clause when `copy_clause` is false because its
/// scratch bank shares canonical truth terms with the source bank. Rust term
/// banks own distinct canonical handles, so a bank-local work copy is required
/// to preserve C's pointer-equality test without mutating the caller.
///
/// # Errors
///
/// Returns a diagnostic if copying clause terms into `work_bank` or inserting
/// normalized terms fails.
pub fn clause_is_tautology_real(
    work_bank: &mut TermBank,
    clause: &Clause,
    _copy_clause: bool,
) -> Result<bool, Diagnostic> {
    if clause.literals().find_true(work_bank).is_some() {
        return Ok(true);
    }
    if clause.positive_literal_count() == 0 || clause.negative_literal_count() == 0 {
        return Ok(false);
    }
    if clause.negative_literal_count() > MAX_EQ_TAUTOLOGY_CHECK_LITNO {
        return Ok(clause.is_trivial(work_bank));
    }

    let mut work_copy = clause.copy_to_bank(work_bank)?;
    let mut rw_system = work_copy
        .literals_mut()
        .extract_by_props(EP_IS_POSITIVE, true)
        .into_vec();
    debug_assert!(!rw_system.is_empty());
    if clause.negative_literal_count() > 1 {
        ground_complete_neg_eqns(&mut rw_system, work_bank)?;
    } else if let Some(rule) = rw_system.first_mut() {
        ground_orient_eqn(rule);
    }

    for literal in work_copy.literals_mut().as_mut_slice() {
        debug_assert!(literal.is_positive());
        ground_normalize_eqn(literal, &rw_system, work_bank)?;
        if literal.left() == literal.right() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn term_compute_top_nf(term: &mut Term, eqns: &[Eqn], bank: &mut TermBank) -> bool {
    for rule in eqns {
        let (left_side, right_side) = if rule.query_prop(EP_GO_NATURAL) {
            (rule.left(), rule.right())
        } else {
            (rule.right(), rule.left())
        };
        if term_struct_equal_no_deref(left_side, term) {
            *term = bank.copy_term(right_side, DerefType::Never);
            return true;
        }
    }
    false
}

fn term_compute_ground_nf(term: &mut Term, eqns: &[Eqn], bank: &mut TermBank) -> bool {
    let mut result = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let mut arg = arg.unwrap_or_else(|| {
            panic!("ground normalization requires initialized argument {index}")
        });
        result |= term_compute_ground_nf(&mut arg, eqns, bank);
        term.set_argument(index, arg);
    }
    result | term_compute_top_nf(term, eqns, bank)
}

fn ground_normalize_eqn(
    eqn: &mut Eqn,
    eqns: &[Eqn],
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let mut result = false;

    let mut term = bank.copy_term(eqn.left(), DerefType::Never);
    if term_compute_ground_nf(&mut term, eqns, bank) {
        let shared = bank.insert(&term, DerefType::Never)?;
        eqn.set_left_raw(shared);
        result = eqn.query_prop(EP_GO_NATURAL);
    }

    let mut term = bank.copy_term(eqn.right(), DerefType::Never);
    if term_compute_ground_nf(&mut term, eqns, bank) {
        let shared = bank.insert(&term, DerefType::Never)?;
        eqn.set_right_raw(shared);
        result |= eqn.query_prop(EP_GO_NATURAL);
    }

    Ok(result)
}

fn ground_backward_contract(
    from: &mut Vec<Eqn>,
    eqns: &[Eqn],
    to: &mut Vec<Eqn>,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    let mut index = 0;
    while index < from.len() {
        if ground_normalize_eqn(&mut from[index], eqns, bank)? {
            let handle = from.remove(index);
            to.insert(0, handle);
        } else {
            index += 1;
        }
    }
    Ok(())
}

fn ground_complete_neg_eqns(list: &mut Vec<Eqn>, bank: &mut TermBank) -> Result<(), Diagnostic> {
    let mut unprocessed = std::mem::take(list);
    let mut processed = Vec::new();

    while !unprocessed.is_empty() {
        let mut handle = unprocessed.remove(0);
        ground_normalize_eqn(&mut handle, &processed, bank)?;
        if handle.left() == handle.right() {
            continue;
        }
        let cmp = ground_orient_eqn(&mut handle);
        debug_assert!(cmp);
        ground_backward_contract(
            &mut processed,
            std::slice::from_ref(&handle),
            &mut unprocessed,
            bank,
        )?;
        processed.insert(0, handle);
    }

    *list = processed;
    Ok(())
}

#[must_use]
#[cfg(test)]
fn negative_rewrite_system_for_tests(clause: &Clause, bank: &mut TermBank) -> EqnList {
    let mut copy = clause
        .copy_to_bank(bank)
        .expect("test clause copies into work bank");
    copy.literals_mut().extract_by_props(EP_IS_POSITIVE, true)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_is_tautology, clause_is_tautology_real, ground_compare,
        negative_rewrite_system_for_tests,
    };
    use crate::basics::partial_orderings::CompareResult;
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

    fn predicate_atom(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let argument_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            let final_type = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![argument_type, bool_type.clone()]));
            bank.signature_mut()
                .declare_final_type(f_code, final_type)
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    #[test]
    fn ground_compare_uses_c_stack_argument_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let f_code = bank.signature_mut().insert_id("f", 2, false);
        let type_ = bank.signature().type_bank().default_type();
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        let left = Term::top_alloc(f_code, 2);
        left.set_argument(0, c.clone());
        left.set_argument(1, a);
        let right = Term::top_alloc(f_code, 2);
        right.set_argument(0, b);
        right.set_argument(1, c);

        assert_eq!(ground_compare(&left, &right), CompareResult::Lesser);
    }

    #[test]
    fn extracted_negative_rewrite_system_preserves_c_reversed_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let first = literal(&mut bank, &a, &b, false);
        let second = literal(&mut bank, &b, &c, false);
        let positive = literal(&mut bank, &a, &c, true);
        let clause = clause(vec![positive, first, second]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        let rewrite_system = negative_rewrite_system_for_tests(&clause, &mut work_bank);
        let literals = rewrite_system.as_slice();

        assert_eq!(literals[0].left().f_code(), b.f_code());
        assert_eq!(literals[0].right().f_code(), c.f_code());
        assert_eq!(literals[1].left().f_code(), a.f_code());
        assert_eq!(literals[1].right().f_code(), b.f_code());
    }

    #[test]
    fn tautology_completion_chains_negative_equalities() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let clause = clause(vec![
            literal(&mut bank, &a, &c, true),
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &b, &c, false),
        ]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert!(clause_is_tautology(&mut work_bank, &clause).unwrap());
    }

    #[test]
    fn tautology_single_negative_rule_rewrites_inside_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let fb = typed_unary(&mut bank, "f", &b);
        let clause = clause(vec![
            literal(&mut bank, &fb, &fa, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert!(clause_is_tautology(&mut work_bank, &clause).unwrap());
    }

    #[test]
    fn tautology_real_no_copy_path_does_not_mutate_source() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let fb = typed_unary(&mut bank, "f", &b);
        let clause = clause(vec![
            literal(&mut bank, &fb, &fa, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let original_positive_left = clause.literals().as_slice()[0].left().clone();

        assert!(clause_is_tautology_real(&mut bank, &clause, false).unwrap());

        assert_eq!(
            clause.literals().as_slice()[0].left(),
            &original_positive_left
        );
    }

    #[test]
    fn tautology_real_no_copy_path_rehomes_predicate_truth_term() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "predicate_tautology_a");
        let p_a = predicate_atom(&mut bank, "predicate_tautology_p", &a);
        let true_term = bank.true_term().clone();
        let clause = clause(vec![
            literal(&mut bank, &p_a, &true_term, true),
            literal(&mut bank, &p_a, &true_term, false),
        ]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert!(clause_is_tautology_real(&mut work_bank, &clause, false).unwrap());
    }

    #[test]
    fn non_implied_positive_literal_is_not_tautological() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let clause = clause(vec![
            literal(&mut bank, &a, &c, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert!(!clause_is_tautology(&mut work_bank, &clause).unwrap());
    }

    #[test]
    fn tautology_check_copies_clause_before_normalizing() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let fb = typed_unary(&mut bank, "f", &b);
        let clause = clause(vec![
            literal(&mut bank, &fb, &fa, true),
            literal(&mut bank, &a, &b, false),
        ]);
        let original_positive_left = clause.literals().as_slice()[0].left().clone();
        let mut work_bank = TermBank::new(bank.signature().clone()).unwrap();

        assert!(clause_is_tautology(&mut work_bank, &clause).unwrap());

        assert_eq!(clause.literal_number(), 2);
        assert_eq!(
            clause.literals().as_slice()[0].left(),
            &original_positive_left
        );
    }
}
