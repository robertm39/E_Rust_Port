use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausecpos::{clause_cpos_split, CompactPos};
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

#[cfg(test)]
mod tests {
    use super::clause_dis_eq_decomposition;
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
}
