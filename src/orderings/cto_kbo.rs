//! Classic Knuth-Bendix ordering implementation from `cto_kbo`.

use crate::basics::partial_orderings::CompareResult;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termfunc::term_is_subterm;
use crate::terms::termtypes::{term_deref, DerefType, Term};
use crate::terms::varhash::VarHash;

/// Compare two terms with the classic KBO implementation.
///
/// This mirrors C `KBOCompare`, including the delayed variable-condition
/// checks and pointer-identity equality for variables. The optimized C build
/// compiles out `KBOCompare`'s first-order-problem assertion, so this path also
/// traverses higher-order term cells as ordinary symbols and arguments when a
/// user explicitly selects classic KBO for a higher-order problem.
///
/// # Panics
///
/// Panics if term argument slots are uninitialized or if the OCB lacks
/// precedence/weight storage required by KBO.
#[must_use]
pub fn kbo_compare(
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
        return kbo_compare_vars(&s, &t, deref_s, deref_t);
    }

    let s_weight = get_term_weight(ocb, &s, deref_s);
    let t_weight = get_term_weight(ocb, &t, deref_t);

    if s_weight > t_weight {
        return match kbo_var_compare(&s, &t, deref_s, deref_t) {
            CompareResult::Greater | CompareResult::Equal => CompareResult::Greater,
            CompareResult::Uncomparable | CompareResult::Lesser => CompareResult::Uncomparable,
            result => panic!("unexpected KBO variable comparison result: {result:?}"),
        };
    }

    if s_weight < t_weight {
        return match kbo_var_compare(&s, &t, deref_s, deref_t) {
            CompareResult::Lesser | CompareResult::Equal => CompareResult::Lesser,
            CompareResult::Uncomparable | CompareResult::Greater => CompareResult::Uncomparable,
            result => panic!("unexpected KBO variable comparison result: {result:?}"),
        };
    }

    match ocb.fun_compare(signature, s.f_code(), t.f_code()) {
        CompareResult::Uncomparable => CompareResult::Uncomparable,
        CompareResult::Greater => match kbo_var_compare(&s, &t, deref_s, deref_t) {
            CompareResult::Greater | CompareResult::Equal => CompareResult::Greater,
            CompareResult::Uncomparable | CompareResult::Lesser => CompareResult::Uncomparable,
            result => panic!("unexpected KBO variable comparison result: {result:?}"),
        },
        CompareResult::Lesser => match kbo_var_compare(&s, &t, deref_s, deref_t) {
            CompareResult::Lesser | CompareResult::Equal => CompareResult::Lesser,
            CompareResult::Uncomparable | CompareResult::Greater => CompareResult::Uncomparable,
            result => panic!("unexpected KBO variable comparison result: {result:?}"),
        },
        CompareResult::Equal => compare_equal_heads_lex(ocb, signature, &s, &t, deref_s, deref_t),
        result => panic!("unexpected function comparison result in KBO: {result:?}"),
    }
}

/// Return whether `s` is strictly greater than `t` in classic KBO.
///
/// # Panics
///
/// Panics under the same invariants as [`kbo_compare`].
#[must_use]
pub fn kbo_greater(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    kbo_greater_new(ocb, signature, s, t, deref_s, deref_t) == CompareResult::Greater
}

/// Compare variable occurrence distributions for two terms.
///
/// This mirrors C `KBOVarCompare`: positive balance means the left term has
/// more occurrences for at least one variable, negative balance means the
/// right term does.
///
/// # Panics
///
/// Panics if a counted variable has a non-negative f-code, matching the C hash
/// function assertion.
#[must_use]
pub fn kbo_var_compare(
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let mut s_gt = false;
    let mut t_gt = false;
    let mut hash = VarHash::new();

    hash.add_var_distrib(s, deref_s, 1);
    hash.add_var_distrib(t, deref_t, -1);

    for entry in hash.entries() {
        if entry.value() > 0 {
            s_gt = true;
        } else if entry.value() < 0 {
            t_gt = true;
        }
        if s_gt && t_gt {
            break;
        }
    }

    match (s_gt, t_gt) {
        (true, true) => CompareResult::Uncomparable,
        (true, false) => CompareResult::Greater,
        (false, true) => CompareResult::Lesser,
        (false, false) => CompareResult::Equal,
    }
}

/// Return whether the variable condition permits `s > t`.
///
/// # Panics
///
/// Panics under the same invariants as [`kbo_var_compare`].
#[must_use]
pub fn kbo_var_greater(s: &Term, t: &Term, deref_s: DerefType, deref_t: DerefType) -> bool {
    let mut hash = VarHash::new();
    hash.add_var_distrib(s, deref_s, 1);
    hash.add_var_distrib(t, deref_t, -1);
    hash.entries().iter().all(|entry| entry.value() >= 0)
}

fn get_weight(ocb: &OrderControlBlock, symbol: FunCode) -> i64 {
    assert_ne!(symbol, 0, "KBO symbols must be non-zero");
    if symbol < 0 {
        ocb.var_weight
    } else {
        ocb.fun_weight(symbol)
    }
}

fn get_term_weight(ocb: &OrderControlBlock, term: &Term, mut deref: DerefType) -> i64 {
    let term = term_deref(term, &mut deref);
    let mut weight = get_weight(ocb, term.f_code());
    if !term.is_free_var() {
        for arg in term.argument_clones().into_iter().flatten() {
            weight += get_term_weight(ocb, &arg, deref);
        }
    }
    weight
}

fn kbo_compare_vars(s: &Term, t: &Term, deref_s: DerefType, deref_t: DerefType) -> CompareResult {
    assert!(
        t.binding().is_none() || deref_t == DerefType::Never,
        "C KBOVarCompare expects already-dereferenced right bindings"
    );
    assert!(
        s.binding().is_none() || deref_s == DerefType::Never,
        "C KBOVarCompare expects already-dereferenced left bindings"
    );

    if t.is_free_var() {
        if s == t {
            CompareResult::Equal
        } else if term_is_subterm(s, t, deref_s) {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        }
    } else {
        assert!(
            s.is_free_var(),
            "one KBO comparison term must be a variable"
        );
        if term_is_subterm(t, s, deref_t) {
            CompareResult::Lesser
        } else {
            CompareResult::Uncomparable
        }
    }
}

fn kbo_greater_new(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);

    if s.is_free_var() {
        return if s == t {
            CompareResult::Equal
        } else {
            CompareResult::Uncomparable
        };
    }
    if t.is_free_var() {
        return if term_is_subterm(&s, &t, deref_s) {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        };
    }

    let s_weight = get_term_weight(ocb, &s, deref_s);
    let t_weight = get_term_weight(ocb, &t, deref_t);

    if s_weight > t_weight {
        return if kbo_var_greater(&s, &t, deref_s, deref_t) {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        };
    }
    if s_weight < t_weight {
        return CompareResult::Uncomparable;
    }

    match ocb.fun_compare(signature, s.f_code(), t.f_code()) {
        CompareResult::Greater => {
            if kbo_var_greater(&s, &t, deref_s, deref_t) {
                CompareResult::Greater
            } else {
                CompareResult::Uncomparable
            }
        }
        CompareResult::Equal => greater_equal_heads_lex(ocb, signature, &s, &t, deref_s, deref_t),
        CompareResult::Uncomparable | CompareResult::Lesser => CompareResult::Uncomparable,
        result => panic!("unexpected function comparison result in KBO greater: {result:?}"),
    }
}

fn compare_equal_heads_lex(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    for index in 0..s.arity().max(t.arity()) {
        if t.arity() <= index {
            return match kbo_var_compare(s, t, deref_s, deref_t) {
                CompareResult::Greater | CompareResult::Equal => CompareResult::Greater,
                CompareResult::Uncomparable | CompareResult::Lesser => CompareResult::Uncomparable,
                result => panic!("unexpected KBO variable comparison result: {result:?}"),
            };
        }
        if s.arity() <= index {
            return match kbo_var_compare(s, t, deref_s, deref_t) {
                CompareResult::Lesser | CompareResult::Equal => CompareResult::Lesser,
                CompareResult::Uncomparable | CompareResult::Greater => CompareResult::Uncomparable,
                result => panic!("unexpected KBO variable comparison result: {result:?}"),
            };
        }

        let res = kbo_compare(
            ocb,
            signature,
            &initialized_arg(s, index),
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        );
        match res {
            CompareResult::Greater => {
                return match kbo_var_compare(s, t, deref_s, deref_t) {
                    CompareResult::Greater | CompareResult::Equal => CompareResult::Greater,
                    CompareResult::Uncomparable | CompareResult::Lesser => {
                        CompareResult::Uncomparable
                    }
                    result => panic!("unexpected KBO variable comparison result: {result:?}"),
                };
            }
            CompareResult::Lesser => {
                return match kbo_var_compare(s, t, deref_s, deref_t) {
                    CompareResult::Lesser | CompareResult::Equal => CompareResult::Lesser,
                    CompareResult::Uncomparable | CompareResult::Greater => {
                        CompareResult::Uncomparable
                    }
                    result => panic!("unexpected KBO variable comparison result: {result:?}"),
                };
            }
            CompareResult::Uncomparable => return CompareResult::Uncomparable,
            CompareResult::Equal => {}
            result => panic!("unexpected recursive KBO comparison result: {result:?}"),
        }
    }
    CompareResult::Equal
}

fn greater_equal_heads_lex(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    for index in 0..s.arity().max(t.arity()) {
        if t.arity() <= index {
            return if kbo_var_greater(s, t, deref_s, deref_t) {
                CompareResult::Greater
            } else {
                CompareResult::Uncomparable
            };
        }
        if s.arity() <= index {
            return CompareResult::Uncomparable;
        }

        match kbo_greater_new(
            ocb,
            signature,
            &initialized_arg(s, index),
            &initialized_arg(t, index),
            deref_s,
            deref_t,
        ) {
            CompareResult::Greater => {
                return if kbo_var_greater(s, t, deref_s, deref_t) {
                    CompareResult::Greater
                } else {
                    CompareResult::Uncomparable
                };
            }
            CompareResult::Uncomparable => return CompareResult::Uncomparable,
            CompareResult::Equal => {}
            result => panic!("unexpected recursive KBO greater result: {result:?}"),
        }
    }
    CompareResult::Equal
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{get_term_weight, kbo_compare, kbo_greater, kbo_var_compare, kbo_var_greater};
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::heuristics::to_params::TermOrdering;
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
        OrderControlBlock::alloc(TermOrdering::Kbo, true, signature, HoOrderKind::LfhoOrder)
    }

    fn app(symbol: FunCode, args: &[Term]) -> Term {
        let term = Term::top_alloc(symbol, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    #[test]
    fn variable_distribution_comparison_matches_c_cases() {
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x_y = app(10, &[x.clone(), y.clone()]);
        let g_x = app(11, std::slice::from_ref(&x));
        let f_x_x = app(10, &[x.clone(), x.clone()]);
        let g_x_y = app(11, &[x.clone(), y]);

        assert_eq!(
            kbo_var_compare(&f_x_y, &g_x, DerefType::Never, DerefType::Never),
            CompareResult::Greater
        );
        assert!(kbo_var_greater(
            &f_x_y,
            &g_x,
            DerefType::Never,
            DerefType::Never
        ));
        assert_eq!(
            kbo_var_compare(&g_x, &f_x_y, DerefType::Never, DerefType::Never),
            CompareResult::Lesser
        );
        assert!(!kbo_var_greater(
            &g_x,
            &f_x_y,
            DerefType::Never,
            DerefType::Never
        ));
        assert_eq!(
            kbo_var_compare(&f_x_x, &g_x_y, DerefType::Never, DerefType::Never),
            CompareResult::Uncomparable
        );
        assert!(!kbo_var_greater(
            &f_x_x,
            &g_x_y,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn variable_special_cases_use_subterm_identity() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let ocb = ocb(&signature);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x = app(f, std::slice::from_ref(&x));
        let f_y = app(f, std::slice::from_ref(&y));

        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &f_x,
                &x,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(kbo_greater(
            &ocb,
            &signature,
            &f_x,
            &x,
            DerefType::Never,
            DerefType::Never
        ));
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &x,
                &f_x,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Lesser
        );
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &f_x,
                &y,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Uncomparable
        );
        assert_eq!(
            kbo_compare(&ocb, &signature, &x, &y, DerefType::Never, DerefType::Never),
            CompareResult::Uncomparable
        );
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &f_x,
                &f_y,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn deref_once_reaches_bound_terms_before_classic_comparison() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let a = symbol(&mut signature, "a", 0);
        let ocb = ocb(&signature);
        let a_term = Term::const_cell_alloc(a);
        let f_a = app(f, std::slice::from_ref(&a_term));
        let bound = Term::const_cell_alloc(-2);
        bound.set_binding(Some(f_a));

        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &bound,
                &a_term,
                DerefType::Once,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &a_term,
                &bound,
                DerefType::Never,
                DerefType::Once
            ),
            CompareResult::Lesser
        );
        assert!(kbo_greater(
            &ocb,
            &signature,
            &bound,
            &a_term,
            DerefType::Once,
            DerefType::Never
        ));
    }

    #[test]
    fn term_weight_uses_function_and_variable_weights() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 2);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.var_weight = 3;
        ocb.set_fun_weight(f, 5);
        ocb.set_fun_weight(a, 7);
        let x = Term::const_cell_alloc(-2);
        let term = app(f, &[Term::const_cell_alloc(a), x]);

        assert_eq!(get_term_weight(&ocb, &term, DerefType::Never), 15);
    }

    #[test]
    fn weight_precedence_and_lexicographic_paths_match_kbo() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 2);
        let a = symbol(&mut signature, "a", 0);
        let b = symbol(&mut signature, "b", 0);
        let c = symbol(&mut signature, "c", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(b, 30);
        ocb.set_fun_prec_weight(c, 20);
        ocb.set_fun_prec_weight(a, 10);

        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let c_term = Term::const_cell_alloc(c);
        let f_a_b = app(f, &[a_term.clone(), b_term.clone()]);
        let f_a_c = app(f, &[a_term, c_term.clone()]);

        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &b_term,
                &c_term,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &f_a_b,
                &f_a_c,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(kbo_greater(
            &ocb,
            &signature,
            &f_a_b,
            &f_a_c,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn equal_root_arity_branch_is_preserved() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_weight(a, 0);
        let long = app(f, &[Term::const_cell_alloc(a)]);
        let short = Term::const_cell_alloc(f);

        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &long,
                &short,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            kbo_compare(
                &ocb,
                &signature,
                &short,
                &long,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Lesser
        );
    }
}
