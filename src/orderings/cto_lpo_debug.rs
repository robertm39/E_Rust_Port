//! Debug first-order lexicographic path ordering from `cto_lpo_debug`.

use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termfunc::term_is_subterm;
use crate::terms::termtypes::{term_deref, DerefType, Term};

/// Compare two terms in the debug LPO implementation.
///
/// This mirrors C `D_LPOCompare`, an older symmetric LPO implementation kept
/// separate from the production `cto_lpo` path.
///
/// # Panics
///
/// Panics if the global problem type is higher-order, if term argument slots
/// are uninitialized, or if the OCB lacks precedence storage.
#[must_use]
pub fn d_lpo_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    assert_debug_lpo_first_order();
    compare_inner(ocb, signature, s, t, deref_s, deref_t)
}

/// Return whether `s` is strictly greater than `t` in debug LPO.
///
/// # Panics
///
/// Panics under the same invariants as [`d_lpo_compare`].
#[must_use]
pub fn d_lpo_greater(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    assert_debug_lpo_first_order();
    greater_inner(ocb, signature, s, t, deref_s, deref_t) == CompareResult::Greater
}

/// Compare two terms in debug LPO when at least one side is a free variable.
///
/// # Panics
///
/// Panics if the global problem type is higher-order.
#[must_use]
pub fn d_lpo_compare_vars(
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    assert_debug_lpo_first_order();
    compare_vars(s, t, deref_s, deref_t)
}

fn compare_inner(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);

    if s.is_free_var() || t.is_free_var() {
        return compare_vars(&s, &t, deref_s, deref_t);
    }

    let funs = ocb.fun_compare(signature, s.f_code(), t.f_code());
    match funs {
        CompareResult::Greater => {
            let result = fun_greater_compare(ocb, signature, &s, &t, deref_s, deref_t);
            if result != CompareResult::Uncomparable {
                assert_ne!(result, CompareResult::Equal);
                return result;
            }
        }
        CompareResult::Equal => {
            let result = fun_equal_compare(ocb, signature, &s, &t, deref_s, deref_t);
            if result != CompareResult::Uncomparable {
                return result;
            }
        }
        CompareResult::Lesser => {
            let result = fun_greater_compare(ocb, signature, &t, &s, deref_t, deref_s);
            if result != CompareResult::Uncomparable {
                return inverse_concrete(result);
            }
        }
        CompareResult::Uncomparable => {}
        result => panic!("unexpected function-symbol comparison in debug LPO: {result:?}"),
    }

    if funs != CompareResult::Greater
        && (funs != CompareResult::Equal || s.arity() >= 2)
        && check_arg_compare(ocb, signature, &s, &t, deref_s, deref_t) == CompareResult::Greater
    {
        return CompareResult::Greater;
    }
    if funs != CompareResult::Lesser
        && (funs != CompareResult::Equal || t.arity() >= 2)
        && check_arg_compare(ocb, signature, &t, &s, deref_t, deref_s) == CompareResult::Greater
    {
        return CompareResult::Lesser;
    }
    CompareResult::Uncomparable
}

fn fun_greater_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let mut greater = true;
    for index in 0..t.arity() {
        match compare_inner(
            ocb,
            signature,
            s,
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        ) {
            CompareResult::Greater => {}
            CompareResult::Lesser | CompareResult::Equal => return CompareResult::Lesser,
            CompareResult::Uncomparable => greater = false,
            result => panic!("unexpected recursive debug LPO comparison: {result:?}"),
        }
    }
    if greater {
        CompareResult::Greater
    } else {
        CompareResult::Uncomparable
    }
}

fn fun_equal_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    for index in 0..s.arity().max(t.arity()) {
        if t.arity() <= index {
            return CompareResult::Greater;
        }
        if s.arity() <= index {
            return CompareResult::Lesser;
        }

        match compare_inner(
            ocb,
            signature,
            &initialized_arg(s, index),
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        ) {
            CompareResult::Equal => {}
            CompareResult::Greater => {
                return fun_equal_tail_greater(ocb, signature, s, t, deref_s, deref_t, index + 1);
            }
            CompareResult::Lesser => {
                return fun_equal_tail_lesser(ocb, signature, s, t, deref_s, deref_t, index + 1);
            }
            CompareResult::Uncomparable => return CompareResult::Uncomparable,
            result => panic!("unexpected recursive debug LPO comparison: {result:?}"),
        }
    }
    CompareResult::Equal
}

fn fun_equal_tail_greater(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    start: usize,
) -> CompareResult {
    for index in start..t.arity() {
        match compare_inner(
            ocb,
            signature,
            s,
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        ) {
            CompareResult::Greater => {}
            CompareResult::Lesser => return CompareResult::Lesser,
            CompareResult::Equal | CompareResult::Uncomparable => {
                return CompareResult::Uncomparable;
            }
            result => panic!("unexpected recursive debug LPO comparison: {result:?}"),
        }
    }
    CompareResult::Greater
}

fn fun_equal_tail_lesser(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    start: usize,
) -> CompareResult {
    for index in start..s.arity() {
        match compare_inner(
            ocb,
            signature,
            &initialized_arg(s, index),
            t,
            deref_s,
            deref_t,
        ) {
            CompareResult::Lesser => {}
            CompareResult::Greater => return CompareResult::Greater,
            CompareResult::Equal | CompareResult::Uncomparable => {
                return CompareResult::Uncomparable;
            }
            result => panic!("unexpected recursive debug LPO comparison: {result:?}"),
        }
    }
    CompareResult::Lesser
}

fn check_arg_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    for index in 0..s.arity() {
        match compare_inner(
            ocb,
            signature,
            &initialized_arg(s, index),
            t,
            deref_s,
            deref_t,
        ) {
            CompareResult::Greater | CompareResult::Equal => return CompareResult::Greater,
            CompareResult::Lesser | CompareResult::Uncomparable => {}
            result => panic!("unexpected recursive debug LPO comparison: {result:?}"),
        }
    }
    CompareResult::Uncomparable
}

fn greater_inner(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);

    if s.is_free_var() || t.is_free_var() {
        return greater_vars(&s, &t, deref_s, deref_t);
    }

    let funs = ocb.fun_compare(signature, s.f_code(), t.f_code());
    match funs {
        CompareResult::Greater => {
            let mut result = CompareResult::Greater;
            for index in 0..t.arity() {
                result = greater_inner(
                    ocb,
                    signature,
                    &s,
                    &initialized_arg(&t, index),
                    deref_s,
                    deref_t,
                );
                if result != CompareResult::Greater {
                    break;
                }
            }
            if result == CompareResult::Greater {
                return result;
            }
        }
        CompareResult::Lesser => {
            for index in 0..s.arity() {
                match greater_inner(
                    ocb,
                    signature,
                    &initialized_arg(&s, index),
                    &t,
                    deref_s,
                    deref_t,
                ) {
                    CompareResult::Greater | CompareResult::Equal => return CompareResult::Greater,
                    CompareResult::Uncomparable => {}
                    result => panic!("unexpected one-sided debug LPO comparison: {result:?}"),
                }
            }
            return CompareResult::Uncomparable;
        }
        CompareResult::Equal => {
            let result = greater_equal_head(ocb, signature, &s, &t, deref_s, deref_t);
            if result != CompareResult::Uncomparable {
                return result;
            }
        }
        CompareResult::Uncomparable => {}
        result => panic!("unexpected function-symbol comparison in debug LPO: {result:?}"),
    }

    if funs != CompareResult::Greater && (funs != CompareResult::Equal || s.arity() >= 2) {
        return greater_check_arg(ocb, signature, &s, &t, deref_s, deref_t);
    }
    CompareResult::Uncomparable
}

fn greater_vars(
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);

    if s.is_free_var() {
        if s == t {
            CompareResult::Equal
        } else {
            CompareResult::Uncomparable
        }
    } else {
        assert!(
            t.is_free_var(),
            "one-sided debug LPO variable path needs a free variable"
        );
        if term_is_subterm(&s, &t, deref_s) {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        }
    }
}

fn greater_equal_head(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let mut result = CompareResult::Equal;
    for index in 0..s.arity().max(t.arity()) {
        if t.arity() <= index {
            return CompareResult::Greater;
        }
        if s.arity() <= index {
            return CompareResult::Uncomparable;
        }

        result = greater_inner(
            ocb,
            signature,
            &initialized_arg(s, index),
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        );
        match result {
            CompareResult::Equal => {}
            CompareResult::Greater => {
                return greater_equal_head_tail(ocb, signature, s, t, deref_s, deref_t, index + 1);
            }
            CompareResult::Uncomparable => break,
            unexpected => panic!("unexpected one-sided debug LPO comparison: {unexpected:?}"),
        }
    }
    result
}

fn greater_equal_head_tail(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    start: usize,
) -> CompareResult {
    for index in start..t.arity() {
        if greater_inner(
            ocb,
            signature,
            s,
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        ) != CompareResult::Greater
        {
            return CompareResult::Uncomparable;
        }
    }
    CompareResult::Greater
}

fn greater_check_arg(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    for index in 0..s.arity() {
        match greater_inner(
            ocb,
            signature,
            &initialized_arg(s, index),
            t,
            deref_s,
            deref_t,
        ) {
            CompareResult::Greater | CompareResult::Equal => return CompareResult::Greater,
            CompareResult::Uncomparable => {}
            result => panic!("unexpected one-sided debug LPO comparison: {result:?}"),
        }
    }
    CompareResult::Uncomparable
}

fn compare_vars(
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);

    if s.is_free_var() {
        if s == t {
            CompareResult::Equal
        } else if term_is_subterm(&t, &s, deref_t) {
            CompareResult::Lesser
        } else {
            CompareResult::Uncomparable
        }
    } else if term_is_subterm(&s, &t, deref_s) {
        CompareResult::Greater
    } else {
        CompareResult::Uncomparable
    }
}

fn inverse_concrete(result: CompareResult) -> CompareResult {
    result
        .inverse()
        .unwrap_or_else(|| panic!("debug LPO comparison result must be invertible"))
}

fn assert_debug_lpo_first_order() {
    assert_ne!(
        problem_type(),
        ProblemType::HigherOrder,
        "debug LPO is not used for higher-order problems"
    );
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{d_lpo_compare, d_lpo_compare_vars, d_lpo_greater};
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::cto_lpo::lpo_compare;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature
    }

    fn symbol(signature: &mut Signature, name: &str, arity: i32) -> FunCode {
        signature.insert_id(name, arity, false)
    }

    fn ocb(signature: &Signature) -> OrderControlBlock {
        OrderControlBlock::alloc(TermOrdering::Lpo, true, signature, HoOrderKind::LfhoOrder)
    }

    fn app(symbol: FunCode, args: &[Term]) -> Term {
        let term = Term::top_alloc(symbol, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    #[test]
    fn debug_lpo_matches_standard_lpo_for_core_first_order_cases() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let g = symbol(&mut signature, "g", 1);
        let b = symbol(&mut signature, "b", 0);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(f, 40);
        ocb.set_fun_prec_weight(g, 30);
        ocb.set_fun_prec_weight(b, 20);
        ocb.set_fun_prec_weight(a, 10);

        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let f_a = app(f, std::slice::from_ref(&a_term));
        let f_b = app(f, std::slice::from_ref(&b_term));
        let g_a = app(g, std::slice::from_ref(&a_term));

        for (left, right, expected) in [
            (&f_b, &g_a, CompareResult::Greater),
            (&g_a, &f_b, CompareResult::Lesser),
            (&f_b, &f_a, CompareResult::Greater),
            (&a_term, &f_a, CompareResult::Lesser),
        ] {
            assert_eq!(
                d_lpo_compare(
                    &ocb,
                    &signature,
                    left,
                    right,
                    DerefType::Never,
                    DerefType::Never,
                ),
                expected
            );
            assert_eq!(
                d_lpo_compare(
                    &ocb,
                    &signature,
                    left,
                    right,
                    DerefType::Never,
                    DerefType::Never,
                ),
                lpo_compare(
                    &ocb,
                    &signature,
                    left,
                    right,
                    DerefType::Never,
                    DerefType::Never,
                )
            );
        }
        assert!(d_lpo_greater(
            &ocb,
            &signature,
            &f_a,
            &a_term,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn debug_lpo_variable_comparison_uses_identity_subterms() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x = app(f, std::slice::from_ref(&x));

        assert_eq!(
            d_lpo_compare_vars(&x, &x, DerefType::Never, DerefType::Never),
            CompareResult::Equal
        );
        assert_eq!(
            d_lpo_compare_vars(&x, &f_x, DerefType::Never, DerefType::Never),
            CompareResult::Lesser
        );
        assert_eq!(
            d_lpo_compare_vars(&f_x, &x, DerefType::Never, DerefType::Never),
            CompareResult::Greater
        );
        assert_eq!(
            d_lpo_compare_vars(&x, &y, DerefType::Never, DerefType::Never),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn debug_lpo_equal_head_length_cases_follow_c_surface() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 2);
        let b = symbol(&mut signature, "b", 0);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(f, 30);
        ocb.set_fun_prec_weight(b, 20);
        ocb.set_fun_prec_weight(a, 10);

        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let unary = app(f, std::slice::from_ref(&a_term));
        let binary = app(f, &[a_term, b_term]);

        assert_eq!(
            d_lpo_compare(
                &ocb,
                &signature,
                &binary,
                &unary,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Greater
        );
        assert_eq!(
            d_lpo_compare(
                &ocb,
                &signature,
                &unary,
                &binary,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Lesser
        );
    }
}
