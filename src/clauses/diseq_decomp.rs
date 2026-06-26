use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::{
    clause_cpos_first_lit, clause_cpos_next_lit, clause_cpos_split, CompactPos,
};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::terms::termbanks::TermBank;

/// Performs the disequality decomposition inference at a compact literal position.
///
/// # Errors
///
/// Returns any allocation/type diagnostic raised while copying the residual
/// literals or allocating the decomposed negative argument-pair literals.
///
/// # Panics
///
/// Panics if `litpos` does not select a literal top position, or if the
/// selected literal sides do not have equal top symbols and arities, matching
/// the C assertions in `ClauseDisEqDecomposition`.
pub fn clause_dis_eq_decomposition(
    bank: &mut TermBank,
    clause: &Clause,
    litpos: CompactPos,
) -> Result<Clause, Diagnostic> {
    let mut relative_pos = litpos;
    let (literal_index, inflit) = clause_cpos_split(clause, &mut relative_pos)
        .expect("compact position must select a literal");
    assert_eq!(
        relative_pos, 0,
        "disequality decomposition expects a top-level literal position"
    );
    assert_eq!(
        inflit.left().f_code(),
        inflit.right().f_code(),
        "disequality decomposition expects equal top symbols"
    );
    assert_eq!(
        inflit.left().arity(),
        inflit.right().arity(),
        "disequality decomposition expects equal top arity"
    );

    let mut literals = clause
        .literals()
        .copy_opt_except_index(Some(literal_index), bank)?;
    let mut new_literals = EqnList::new();
    for index in 0..inflit.left().arity() {
        let left = inflit
            .left()
            .argument(index)
            .expect("decomposed left term arguments must be initialized");
        let right = inflit
            .right()
            .argument(index)
            .expect("decomposed right term arguments must be initialized");
        new_literals.insert_first(Eqn::alloc(left, right, bank, false)?);
    }
    literals.append(new_literals);
    literals.remove_resolved(bank);
    literals.remove_duplicates(bank);
    Ok(Clause::alloc(literals))
}

/// Computes all C `ComputeDisEqDecompositions` results for one clause.
///
/// The wrapper scans literal compact positions in C order and inserts every
/// eligible decomposition into `store`.
///
/// # Errors
///
/// Returns diagnostics from [`clause_dis_eq_decomposition`].
pub fn compute_dis_eq_decompositions(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    diseq_decomposition: i64,
    diseq_decomp_maxarity: i64,
) -> Result<i64, Diagnostic> {
    let mut count = 0;
    let literal_count = i64::try_from(clause.literal_number()).unwrap_or(i64::MAX);
    if literal_count > diseq_decomposition {
        return Ok(count);
    }

    let mut litpos = 0;
    let mut current = clause_cpos_first_lit(clause, &mut litpos);
    while let Some((literal_index, literal)) = current {
        let arity = i64::try_from(literal.left().arity()).unwrap_or(i64::MAX);
        if literal.is_equ_lit(bank)
            && literal.is_negative()
            && literal.left().f_code() == literal.right().f_code()
            && arity <= diseq_decomp_maxarity
            && arity != 0
        {
            let new_clause = clause_dis_eq_decomposition(bank, clause, litpos)?;
            store.insert(new_clause);
            count += 1;
        }
        current = clause_cpos_next_lit(clause, literal_index, &mut litpos);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::{clause_dis_eq_decomposition, compute_dis_eq_decompositions};
    use crate::clauses::clause::Clause;
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
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn binary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        f_code
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    #[test]
    fn decomposition_replaces_selected_disequality_with_reversed_argument_disequalities() {
        let mut bank = test_bank();
        let a_term = typed_const(&mut bank, "a");
        let b_term = typed_const(&mut bank, "b");
        let c_term = typed_const(&mut bank, "c");
        let d_term = typed_const(&mut bank, "d");
        let f_code = binary_code(&mut bank, "f");
        let left = typed_binary_with_code(&mut bank, f_code, &a_term, &b_term);
        let right = typed_binary_with_code(&mut bank, f_code, &c_term, &d_term);
        let positive_rest = eqn(&mut bank, &a_term, &d_term, true);
        let diseq = eqn(&mut bank, &left, &right, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq, positive_rest.clone()]));
        let litpos = clause.literals().as_slice()[0].standard_weight();

        let decomposed = clause_dis_eq_decomposition(&mut bank, &clause, litpos).unwrap();

        assert_eq!(decomposed.literal_number(), 3);
        assert_eq!(decomposed.positive_literal_count(), 1);
        assert_eq!(decomposed.literals().as_slice()[0], positive_rest);
        assert_eq!(decomposed.literals().as_slice()[1].left(), &b_term);
        assert_eq!(decomposed.literals().as_slice()[1].right(), &d_term);
        assert!(decomposed.literals().as_slice()[1].is_negative());
        assert_eq!(decomposed.literals().as_slice()[2].left(), &a_term);
        assert_eq!(decomposed.literals().as_slice()[2].right(), &c_term);
        assert!(decomposed.literals().as_slice()[2].is_negative());
    }

    #[test]
    fn decomposition_removes_resolved_and_duplicate_new_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f = binary_code(&mut bank, "f");
        let left = typed_binary_with_code(&mut bank, f, &a, &a);
        let right = typed_binary_with_code(&mut bank, f, &a, &b);
        let diseq = eqn(&mut bank, &left, &right, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq]));

        let decomposed = clause_dis_eq_decomposition(&mut bank, &clause, 0).unwrap();

        assert_eq!(decomposed.literal_number(), 1);
        assert!(decomposed.is_negative());
        assert_eq!(decomposed.literals().as_slice()[0].left(), &a);
        assert_eq!(decomposed.literals().as_slice()[0].right(), &b);
    }

    #[test]
    #[should_panic(expected = "top-level literal position")]
    fn decomposition_rejects_non_top_literal_positions() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f = binary_code(&mut bank, "f");
        let left = typed_binary_with_code(&mut bank, f, &a, &a);
        let right = typed_binary_with_code(&mut bank, f, &b, &b);
        let diseq = eqn(&mut bank, &left, &right, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq]));

        let _ = clause_dis_eq_decomposition(&mut bank, &clause, 1);
    }

    #[test]
    fn compute_dis_eq_decompositions_inserts_eligible_decompositions_in_literal_order() {
        let mut bank = test_bank();
        let first_left_arg = typed_const(&mut bank, "all_a");
        let first_right_arg = typed_const(&mut bank, "all_b");
        let second_left_arg = typed_const(&mut bank, "all_c");
        let second_right_arg = typed_const(&mut bank, "all_d");
        let third_left_arg = typed_const(&mut bank, "all_e");
        let third_right_arg = typed_const(&mut bank, "all_f");
        let g_code = binary_code(&mut bank, "all_g");
        let h_code = binary_code(&mut bank, "all_h");
        let first_left =
            typed_binary_with_code(&mut bank, g_code, &first_left_arg, &first_right_arg);
        let first_right =
            typed_binary_with_code(&mut bank, g_code, &second_left_arg, &second_right_arg);
        let second_left =
            typed_binary_with_code(&mut bank, h_code, &first_left_arg, &third_left_arg);
        let second_right =
            typed_binary_with_code(&mut bank, h_code, &second_left_arg, &third_right_arg);
        let first_diseq = eqn(&mut bank, &first_left, &first_right, false);
        let second_diseq = eqn(&mut bank, &second_left, &second_right, false);
        let rest = eqn(&mut bank, &first_left_arg, &second_right_arg, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            first_diseq,
            rest.clone(),
            second_diseq,
        ]));
        let mut store = ClauseSet::new();

        let count = compute_dis_eq_decompositions(&mut bank, &clause, &mut store, 3, 2).unwrap();

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
        assert!(store
            .iter()
            .all(|clause| clause.literals().as_slice()[0] == rest));
        assert!(store.iter().any(|clause| {
            clause.literals().as_slice().iter().any(|literal| {
                literal.left() == &first_right_arg && literal.right() == &second_right_arg
            })
        }));
        assert!(store.iter().any(|clause| {
            clause.literals().as_slice().iter().any(|literal| {
                literal.left() == &third_left_arg && literal.right() == &third_right_arg
            })
        }));
    }

    #[test]
    fn compute_dis_eq_decompositions_honors_size_and_arity_gates() {
        let mut bank = test_bank();
        let left_first = typed_const(&mut bank, "gate_a");
        let left_second = typed_const(&mut bank, "gate_b");
        let right_first = typed_const(&mut bank, "gate_c");
        let right_second = typed_const(&mut bank, "gate_d");
        let function_code = binary_code(&mut bank, "gate_f");
        let left = typed_binary_with_code(&mut bank, function_code, &left_first, &left_second);
        let right = typed_binary_with_code(&mut bank, function_code, &right_first, &right_second);
        let diseq = eqn(&mut bank, &left, &right, false);
        let rest = eqn(&mut bank, &left_first, &right_second, true);
        let clause = Clause::alloc(EqnList::from_vec(vec![diseq, rest]));

        let mut size_blocked = ClauseSet::new();
        assert_eq!(
            compute_dis_eq_decompositions(&mut bank, &clause, &mut size_blocked, 1, 2).unwrap(),
            0
        );
        assert!(size_blocked.is_empty());

        let mut arity_blocked = ClauseSet::new();
        assert_eq!(
            compute_dis_eq_decompositions(&mut bank, &clause, &mut arity_blocked, 2, 1).unwrap(),
            0
        );
        assert!(arity_blocked.is_empty());
    }
}
