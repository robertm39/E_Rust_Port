use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::heuristics::varweights::clause_count_ext_symbols as varweight_clause_count_ext_symbols;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_depth;
use crate::terms::termtypes::Term;

#[must_use]
pub fn clause_count_ext_symbols(clause: &Clause, signature: &Signature, min_arity: i64) -> i64 {
    varweight_clause_count_ext_symbols(clause, signature, min_arity)
}

/// Adds free-variable occurrences by negated variable f-code.
///
/// # Panics
///
/// Panics if a traversed non-variable term has an uninitialized argument slot,
/// if a non-variable term has a non-positive f-code, or if a variable f-code
/// cannot be converted to the dynamic-array index shape used by the C helper.
pub fn term_add_var_distribution(term: &Term, dist_array: &mut PDIntArray) -> FunCode {
    let mut max_var = 0;
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            let var = positive_variable_code(current.f_code());
            let index = pd_index_from_positive(var);
            let count = dist_array.element_int(index) + 1;
            max_var = max_var.max(var);
            assert!(
                dist_array.assign(index, count),
                "variable distribution array must cover variable codes"
            );
        } else {
            assert!(
                current.f_code() > 0,
                "non-free terms in variable distribution require positive f-codes"
            );
            stack.extend(current.argument_clones().into_iter().map(|arg| {
                arg.unwrap_or_else(|| {
                    panic!("variable distribution requires initialized term arguments")
                })
            }));
        }
    }

    max_var
}

/// Adds variable occurrences for both literal sides.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn eqn_add_var_distribution(eqn: &Eqn, dist_array: &mut PDIntArray) -> FunCode {
    term_add_var_distribution(eqn.left(), dist_array)
        .max(term_add_var_distribution(eqn.right(), dist_array))
}

/// Adds variable occurrences for every literal in the list.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn eqn_list_add_var_distribution(list: &EqnList, dist_array: &mut PDIntArray) -> FunCode {
    list.as_slice()
        .iter()
        .map(|literal| eqn_add_var_distribution(literal, dist_array))
        .max()
        .unwrap_or(0)
}

/// Adds variable occurrences for every literal in the clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
pub fn clause_add_var_distribution(clause: &Clause, dist_array: &mut PDIntArray) -> FunCode {
    eqn_list_add_var_distribution(clause.literals(), dist_array)
}

/// Counts distinct variable f-codes in a clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
#[must_use]
pub fn clause_count_variable_set(clause: &Clause) -> i64 {
    let mut dist_array = PDIntArray::new_int(20, 20);
    let max_var = clause_add_var_distribution(clause, &mut dist_array);
    count_var_indices_with(&mut dist_array, max_var, |count| count != 0)
}

/// Counts variable f-codes that occur exactly once in a clause.
///
/// # Panics
///
/// Panics under the same conditions as [`term_add_var_distribution`].
#[must_use]
pub fn clause_count_singleton_set(clause: &Clause) -> i64 {
    let mut dist_array = PDIntArray::new_int(20, 20);
    let max_var = clause_add_var_distribution(clause, &mut dist_array);
    count_var_indices_with(&mut dist_array, max_var, |count| count == 1)
}

#[must_use]
pub fn clause_count_maximal_terms(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .map(Eqn::count_maximal_literals)
        .sum()
}

#[must_use]
pub fn clause_count_maximal_literals(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| literal.is_maximal())
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[must_use]
pub fn clause_count_unorientable_literals(clause: &Clause) -> i64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .filter(|literal| !literal.is_oriented())
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

/// Adds TPTP-style term-depth statistics to the provided accumulators.
///
/// Equational literals contribute both sides. Predicate literals contribute
/// only the arguments of the predicate atom, matching the C interpretation of
/// conventional TPTP literals.
///
/// # Panics
///
/// Panics if a predicate literal has an uninitialized atom argument slot.
pub fn clause_tptp_depth_info_add(
    bank: &TermBank,
    clause: &Clause,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    for literal in clause.literals().as_slice() {
        eqn_tptp_depth_info_add(bank, literal, depthmax, depthsum, count);
    }
    *depthmax
}

fn eqn_tptp_depth_info_add(
    bank: &TermBank,
    eqn: &Eqn,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    if eqn.is_equ_lit(bank) {
        term_depth_info_add(eqn.left(), depthmax, depthsum, count);
        term_depth_info_add(eqn.right(), depthmax, depthsum, count);
    } else {
        for index in 0..eqn.left().arity() {
            let arg = eqn.left().argument(index).unwrap_or_else(|| {
                panic!("TPTP depth collection requires initialized predicate arguments")
            });
            term_depth_info_add(&arg, depthmax, depthsum, count);
        }
    }
    *depthmax
}

fn term_depth_info_add(
    term: &Term,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    let depth = term_depth(term);
    *depthsum += depth;
    *count += 1;
    if depth > *depthmax {
        *depthmax = depth;
    }
    *depthmax
}

fn count_var_indices_with<F>(dist_array: &mut PDIntArray, max_var: FunCode, predicate: F) -> i64
where
    F: Fn(i64) -> bool,
{
    (1..=max_var)
        .filter(|var| predicate(dist_array.element_int(pd_index_from_positive(*var))))
        .count()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn positive_variable_code(f_code: FunCode) -> FunCode {
    assert!(f_code < 0, "variable f-code must be negative");
    f_code
        .checked_neg()
        .unwrap_or_else(|| panic!("variable f-code cannot be negated"))
}

fn pd_index_from_positive(value: FunCode) -> PDArrayIndex {
    PDArrayIndex::try_from(value)
        .unwrap_or_else(|_| panic!("positive variable code must fit the dynamic-array index type"))
}

#[cfg(test)]
mod tests {
    use super::{
        clause_add_var_distribution, clause_count_ext_symbols, clause_count_maximal_literals,
        clause_count_maximal_terms, clause_count_singleton_set, clause_count_unorientable_literals,
        clause_count_variable_set, clause_tptp_depth_info_add, eqn_add_var_distribution,
        eqn_list_add_var_distribution, term_add_var_distribution,
    };
    use crate::basics::pdarrays::PDIntArray;
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
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
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never)
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
        let shared = bank
            .insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"));
        shared.set_type(Some(bank.signature().type_bank().bool_type()));
        shared
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
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn variable_distribution_counts_by_negated_f_code() {
        let mut bank = term_bank();
        let x1 = typed_var(&bank, -2);
        let x2 = typed_var(&bank, -4);
        let x1_again = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x1);
        let gx = typed_binary(&mut bank, "g", &x1_again, &x2);
        let mut dist = PDIntArray::new_int(2, 2);

        assert_eq!(term_add_var_distribution(&gx, &mut dist), 4);
        assert_eq!(dist.element_int(2), 1);
        assert_eq!(dist.element_int(4), 1);

        let eqn = equation(&mut bank, &fx, &gx, false);
        assert_eq!(eqn_add_var_distribution(&eqn, &mut dist), 4);
        assert_eq!(dist.element_int(2), 3);
        assert_eq!(dist.element_int(4), 2);

        let list = EqnList::from_vec(vec![eqn.clone()]);
        assert_eq!(eqn_list_add_var_distribution(&list, &mut dist), 4);
        assert_eq!(dist.element_int(2), 5);
        assert_eq!(dist.element_int(4), 3);

        let clause = clause_from(vec![eqn]);
        assert_eq!(clause_add_var_distribution(&clause, &mut dist), 4);
        assert_eq!(dist.element_int(2), 7);
        assert_eq!(dist.element_int(4), 4);
    }

    #[test]
    fn variable_set_and_singleton_counts_use_variable_codes_not_identity() {
        let mut bank = term_bank();
        let x1 = typed_var(&bank, -2);
        let x1_same_code = typed_var(&bank, -2);
        let x2 = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "a");
        let left = typed_binary(&mut bank, "f", &x1, &x1_same_code);
        let right = typed_binary(&mut bank, "g", &x2, &a);
        let clause = clause_from(vec![equation(&mut bank, &left, &right, false)]);

        assert_eq!(clause_count_variable_set(&clause), 2);
        assert_eq!(clause_count_singleton_set(&clause), 1);
    }

    #[test]
    fn maximal_and_unorientable_counts_follow_literal_flags() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let mut oriented_max = equation(&mut bank, &a, &b, true);
        oriented_max.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let mut unoriented_max = equation(&mut bank, &b, &c, false);
        unoriented_max.set_prop(EP_IS_MAXIMAL);
        let ordinary = equation(&mut bank, &a, &c, false);
        let clause = clause_from(vec![oriented_max, unoriented_max, ordinary]);

        assert_eq!(clause_count_maximal_literals(&clause), 2);
        assert_eq!(clause_count_maximal_terms(&clause), 3);
        assert_eq!(clause_count_unorientable_literals(&clause), 2);
    }

    #[test]
    fn tptp_depth_info_counts_equation_sides_and_predicate_arguments() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let h = typed_binary(&mut bank, "h", &fa, &gb);
        let p = typed_predicate_binary(&mut bank, "p", &h, &a);
        let eqn = equation(&mut bank, &h, &gb, true);
        let pred = predicate_literal(&mut bank, &p);
        let clause = clause_from(vec![eqn, pred]);

        let mut depthmax = 0;
        let mut depthsum = 0;
        let mut count = 0;
        assert_eq!(
            clause_tptp_depth_info_add(&bank, &clause, &mut depthmax, &mut depthsum, &mut count,),
            3
        );
        assert_eq!(depthmax, 3);
        assert_eq!(depthsum, 3 + 2 + 3 + 1);
        assert_eq!(count, 4);
    }

    #[test]
    fn external_symbol_count_reuses_clause_feature_contract() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let gb = typed_unary(&mut bank, "g", &b);
        let clause = clause_from(vec![equation(&mut bank, &fa, &gb, true)]);

        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 0), 4);
        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 1), 2);
        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 2), 0);
    }
}
