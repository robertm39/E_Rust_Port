use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termtypes::{Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT};

#[must_use]
pub fn detect_commutativity(clause: &Clause) -> FunCode {
    let Some(lit) = unit_positive_literal(clause) else {
        return 0;
    };
    let left = lit.left();
    let right = lit.right();

    if left.is_phony_app() || right.is_phony_app() || left.arity() != 2 || right.arity() != 2 {
        return 0;
    }
    if left.f_code() != right.f_code() {
        return 0;
    }

    let Some(left_0) = free_arg(left, 0) else {
        return 0;
    };
    let Some(left_1) = free_arg(left, 1) else {
        return 0;
    };
    let Some(right_0) = free_arg(right, 0) else {
        return 0;
    };
    let Some(right_1) = free_arg(right, 1) else {
        return 0;
    };

    if left_0 == left_1 || left_0 != right_1 || left_1 != right_0 {
        return 0;
    }

    left.f_code()
}

/// # Panics
///
/// Panics if a clause passes the C preconditions up to the point where the C
/// code asserts that the right side has arity two, but the right side does not
/// have that arity.
#[must_use]
pub fn detect_associativity(clause: &Clause) -> FunCode {
    let Some(lit) = unit_positive_literal(clause) else {
        return 0;
    };

    let left = lit.left();
    let right = lit.right();
    let expected_weight = 2 * DEFAULT_FWEIGHT + 3 * DEFAULT_VWEIGHT;
    if term_standard_weight(left) != expected_weight {
        return 0;
    }
    if left.is_applied_free_var()
        || left.is_lambda()
        || right.is_applied_free_var()
        || right.is_lambda()
        || left.f_code() != right.f_code()
        || left.arity() != 2
    {
        return 0;
    }
    assert_eq!(right.arity(), 2);

    let (lterm, rterm) = if left
        .argument(0)
        .is_some_and(|argument| argument.is_free_var())
    {
        (right, left)
    } else {
        (left, right)
    };

    let f = lterm.f_code();
    let Some(nested_left) = lterm.argument(0) else {
        return 0;
    };
    let Some(right_var) = lterm.argument(1) else {
        return 0;
    };

    if f != nested_left.f_code() || nested_left.arity() != 2 || !right_var.is_free_var() {
        return 0;
    }
    let Some(v1_term) = free_arg(&nested_left, 0) else {
        return 0;
    };
    let Some(v2_term) = free_arg(&nested_left, 1) else {
        return 0;
    };
    let v1 = v1_term.f_code();
    let v2 = v2_term.f_code();
    let v3 = right_var.f_code();
    if v1 == v2 || v1 == v3 || v2 == v3 {
        return 0;
    }

    let Some(rterm_left) = rterm.argument(0) else {
        return 0;
    };
    let Some(nested_right) = rterm.argument(1) else {
        return 0;
    };
    if f != nested_right.f_code()
        || v1 != rterm_left.f_code()
        || nested_right.arity() != 2
        || nested_right
            .argument(0)
            .is_none_or(|arg| v2 != arg.f_code())
        || nested_right
            .argument(1)
            .is_none_or(|arg| v3 != arg.f_code())
    {
        return 0;
    }

    f
}

pub fn clause_scan_ac(sig: &mut Signature, clause: &Clause) -> bool {
    let f = detect_commutativity(clause);
    if f != 0 {
        if !sig.query_prop(f, FP_COMMUTATIVE) {
            sig.set_func_prop(f, FP_COMMUTATIVE);
        }
        return true;
    }

    let f = detect_associativity(clause);
    if f != 0 && !sig.query_prop(f, FP_ASSOCIATIVE) {
        sig.set_func_prop(f, FP_ASSOCIATIVE);
    }
    false
}

pub fn clause_set_scan_ac(sig: &mut Signature, set: &ClauseSet) -> bool {
    let mut result = false;
    for clause in set.iter() {
        result |= clause_scan_ac(sig, clause);
    }
    result
}

fn unit_positive_literal(clause: &Clause) -> Option<&crate::clauses::eqn::Eqn> {
    if !clause.is_unit() {
        return None;
    }
    let lit = clause.literals().as_slice().first()?;
    lit.is_positive().then_some(lit)
}

fn free_arg(term: &Term, index: usize) -> Option<Term> {
    let arg = term.argument(index)?;
    arg.is_free_var().then_some(arg)
}

#[cfg(test)]
mod tests {
    use super::{clause_scan_ac, clause_set_scan_ac, detect_associativity, detect_commutativity};
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::{Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn bank_with_binary_symbol() -> (TermBank, i64) {
        let mut signature = Signature::new(TypeBank::new());
        let type_ = signature.type_bank().default_type();
        let f_code = signature.insert_id("f", 2, false);
        signature
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
            )
            .unwrap();
        (TermBank::new(signature).unwrap(), f_code)
    }

    fn variable(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn binary(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn unit_clause(lit: Eqn) -> Clause {
        Clause::alloc(EqnList::from_vec(vec![lit]))
    }

    #[test]
    fn detects_commutativity_from_positive_unit_equation() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let left = binary(&mut bank, f_code, &x, &y);
        let right = binary(&mut bank, f_code, &y, &x);
        let clause = unit_clause(literal(&mut bank, &left, &right, true));

        assert_eq!(detect_commutativity(&clause), f_code);
    }

    #[test]
    fn commutativity_rejects_non_unit_negative_or_non_swapped_shapes() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let left = binary(&mut bank, f_code, &x, &y);
        let same = binary(&mut bank, f_code, &x, &y);
        let swapped = binary(&mut bank, f_code, &y, &x);
        let positive = literal(&mut bank, &left, &swapped, true);
        let negative = literal(&mut bank, &left, &swapped, false);

        assert_eq!(detect_commutativity(&Clause::empty()), 0);
        assert_eq!(detect_commutativity(&unit_clause(negative)), 0);
        assert_eq!(
            detect_commutativity(&Clause::alloc(EqnList::from_vec(vec![
                positive.clone(),
                positive
            ]))),
            0
        );
        assert_eq!(
            detect_commutativity(&unit_clause(literal(&mut bank, &left, &same, true))),
            0
        );
    }

    #[test]
    fn detects_associativity_in_both_orientations() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let z = variable(&bank, -6);
        let xy = binary(&mut bank, f_code, &x, &y);
        let yz = binary(&mut bank, f_code, &y, &z);
        let left_assoc = binary(&mut bank, f_code, &xy, &z);
        let right_assoc = binary(&mut bank, f_code, &x, &yz);

        let forward = unit_clause(literal(&mut bank, &left_assoc, &right_assoc, true));
        let reverse = unit_clause(literal(&mut bank, &right_assoc, &left_assoc, true));

        assert_eq!(detect_associativity(&forward), f_code);
        assert_eq!(detect_associativity(&reverse), f_code);
    }

    #[test]
    fn associativity_preserves_left_weight_only_check() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let z = variable(&bank, -6);
        let extra = variable(&bank, -8);
        let xy = binary(&mut bank, f_code, &x, &y);
        let yz = binary(&mut bank, f_code, &y, &z);
        let left_assoc = binary(&mut bank, f_code, &xy, &z);
        let right_assoc = binary(&mut bank, f_code, &x, &yz);
        let heavy_left = binary(&mut bank, f_code, &left_assoc, &extra);

        assert_eq!(
            detect_associativity(&unit_clause(literal(
                &mut bank,
                &left_assoc,
                &heavy_left,
                true
            ))),
            0
        );
        assert_eq!(
            detect_associativity(&unit_clause(literal(
                &mut bank,
                &left_assoc,
                &right_assoc,
                true
            ))),
            f_code
        );
    }

    #[test]
    fn clause_scan_sets_props_but_returns_false_for_associativity_only() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let z = variable(&bank, -6);
        let xy = binary(&mut bank, f_code, &x, &y);
        let yz = binary(&mut bank, f_code, &y, &z);
        let left_assoc = binary(&mut bank, f_code, &xy, &z);
        let right_assoc = binary(&mut bank, f_code, &x, &yz);
        let assoc = unit_clause(literal(&mut bank, &left_assoc, &right_assoc, true));

        assert!(!clause_scan_ac(bank.signature_mut(), &assoc));
        assert!(bank.signature().query_prop(f_code, FP_ASSOCIATIVE));
        assert!(!bank.signature().query_prop(f_code, FP_COMMUTATIVE));
    }

    #[test]
    fn clause_scan_and_set_scan_report_commutativity() {
        let (mut bank, f_code) = bank_with_binary_symbol();
        let x = variable(&bank, -2);
        let y = variable(&bank, -4);
        let left = binary(&mut bank, f_code, &x, &y);
        let right = binary(&mut bank, f_code, &y, &x);
        let clause = unit_clause(literal(&mut bank, &left, &right, true));
        let set = ClauseSet::from_clauses([clause.clone()]);

        assert!(clause_scan_ac(bank.signature_mut(), &clause));
        assert!(bank.signature().query_prop(f_code, FP_COMMUTATIVE));
        assert!(clause_set_scan_ac(bank.signature_mut(), &set));
        assert!(bank.signature().query_prop(f_code, FP_COMMUTATIVE));
    }
}
