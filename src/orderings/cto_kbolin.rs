//! Linear-time first-order KBO6 implementation from `cto_kbolin`.

use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termtypes::{term_deref, DerefType, Term};

/// Compare two first-order terms with C `KBO6Compare`.
///
/// This ports the non-`ENABLE_LFHO` first-order `kbolincmp` path. The C
/// wrapper resets OCB balance fields before comparison and leaves the final
/// comparison balances in the OCB; this function preserves that entry-reset
/// behavior.
///
/// # Panics
///
/// Panics if the global problem type is higher-order and either dereferenced
/// term needs the LFHO KBO6 path, if same-symbol terms do not have
/// C-compatible matching arities, if term argument slots are uninitialized, or
/// if the OCB lacks KBO weight/precedence storage.
pub fn kbo6_compare(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    assert_kbo6_surface_supported(s, t, deref_s, deref_t);
    kbo6_reset(ocb);
    kbo_lin_cmp(ocb, signature, s, t, deref_s, deref_t)
}

/// Return whether `s` is strictly greater than `t` in first-order KBO6.
///
/// # Panics
///
/// Panics under the same invariants as [`kbo6_compare`].
pub fn kbo6_greater(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    kbo6_compare(ocb, signature, s, t, deref_s, deref_t) == CompareResult::Greater
}

fn kbo6_reset(ocb: &mut OrderControlBlock) {
    if ocb.ho_order_kind == crate::basics::partial_orderings::HoOrderKind::LambdaOrder {
        ocb.reset_ho_var_map();
    } else {
        let max_var = usize::try_from(ocb.max_var).unwrap_or(0);
        for index in 0..=max_var.min(ocb.vb.len().saturating_sub(1)) {
            ocb.vb[index] = 0;
        }
    }
    ocb.wb = 0;
    ocb.pos_bal = 0;
    ocb.neg_bal = 0;
    ocb.max_var = 0;
}

fn assert_kbo6_surface_supported(
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) {
    if problem_type() == ProblemType::HigherOrder {
        let s = term_deref(s, &mut deref_s);
        let t = term_deref(t, &mut deref_t);
        assert!(
            !s.has_higher_order_ordering_surface() && !t.has_higher_order_ordering_surface(),
            "LFHO KBO6 term ordering is not ported yet"
        );
    }
}

fn resize_vb(ocb: &mut OrderControlBlock, index: usize) {
    while ocb.vb_size <= index {
        ocb.vb_size *= 2;
    }
    ocb.vb.resize(ocb.vb_size, 0);
}

fn inc_vb(ocb: &mut OrderControlBlock, var: &Term) {
    let index = var_index(var);
    if index > usize::try_from(ocb.max_var).unwrap_or(0) {
        if index >= ocb.vb_size {
            resize_vb(ocb, index);
        }
        ocb.max_var = i64::try_from(index).unwrap_or_else(|_| panic!("variable index too large"));
        ocb.vb[index] = 1;
        ocb.pos_bal += 1;
        ocb.wb += ocb.var_weight;
    } else {
        let tmp_bal = ocb.vb[index];
        ocb.vb[index] += 1;
        ocb.pos_bal += i64::from(tmp_bal == 0);
        ocb.neg_bal -= i64::from(tmp_bal == -1);
        ocb.wb += ocb.var_weight;
    }
}

fn dec_vb(ocb: &mut OrderControlBlock, var: &Term) {
    let index = var_index(var);
    if index > usize::try_from(ocb.max_var).unwrap_or(0) {
        if index >= ocb.vb_size {
            resize_vb(ocb, index);
        }
        ocb.max_var = i64::try_from(index).unwrap_or_else(|_| panic!("variable index too large"));
        ocb.vb[index] = -1;
        ocb.neg_bal += 1;
        ocb.wb -= ocb.var_weight;
    } else {
        let tmp_bal = ocb.vb[index];
        ocb.vb[index] -= 1;
        ocb.neg_bal += i64::from(tmp_bal == 0);
        ocb.pos_bal -= i64::from(tmp_bal == 1);
        ocb.wb -= ocb.var_weight;
    }
}

fn var_index(var: &Term) -> usize {
    assert!(var.f_code() < 0, "KBO6 variable f-code must be negative");
    usize::try_from(-var.f_code()).unwrap_or_else(|_| panic!("variable index must fit usize"))
}

fn mfy_vwb_lhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb(ocb, term, deref, true);
}

fn mfy_vwb_rhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb(ocb, term, deref, false);
}

fn mfy_vwb(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType, lhs: bool) {
    let mut stack = vec![(term.clone(), deref)];
    while let Some((candidate, mut current_deref)) = stack.pop() {
        let current = term_deref(&candidate, &mut current_deref);
        if current.is_free_var() {
            if lhs {
                inc_vb(ocb, &current);
            } else {
                dec_vb(ocb, &current);
            }
        } else {
            if lhs {
                ocb.wb += ocb.fun_weight(current.f_code());
            } else {
                ocb.wb -= ocb.fun_weight(current.f_code());
            }
            for arg in current.argument_clones().into_iter().flatten() {
                stack.push((arg, current_deref));
            }
        }
    }
}

fn kbo_lin_cmp(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    mut deref_s: DerefType,
    mut deref_t: DerefType,
) -> CompareResult {
    let s = term_deref(s, &mut deref_s);
    let t = term_deref(t, &mut deref_t);
    let mut res = CompareResult::Equal;

    if s.f_code() == t.f_code() {
        assert_eq!(
            s.arity(),
            t.arity(),
            "same-symbol first-order KBO6 terms must have matching arity"
        );
        for index in 0..s.arity() {
            res = kbo_lin_cmp(
                ocb,
                signature,
                &initialized_arg(&s, index),
                &initialized_arg(&t, index),
                deref_s,
                deref_t,
            );
            if res != CompareResult::Equal {
                let next = index + 1;
                if next < s.arity() {
                    for rest in next..s.arity() {
                        mfy_vwb_lhs(ocb, &initialized_arg(&s, rest), deref_s);
                        mfy_vwb_rhs(ocb, &initialized_arg(&t, rest), deref_t);
                    }
                    res = balance_result_after_lex(ocb, res);
                }
                break;
            }
        }
    } else if s.is_free_var() {
        if t.is_free_var() {
            inc_vb(ocb, &s);
            dec_vb(ocb, &t);
            res = CompareResult::Uncomparable;
        } else {
            inc_vb(ocb, &s);
            mfy_vwb_rhs(ocb, &t, deref_t);
            res = if ocb.pos_bal == 0 {
                CompareResult::Lesser
            } else {
                CompareResult::Uncomparable
            };
        }
    } else if t.is_free_var() {
        dec_vb(ocb, &t);
        mfy_vwb_lhs(ocb, &s, deref_s);
        res = if ocb.neg_bal == 0 {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        };
    } else {
        mfy_vwb_lhs(ocb, &s, deref_s);
        mfy_vwb_rhs(ocb, &t, deref_t);
        res = balance_result_after_heads(ocb, signature, &s, &t);
    }
    res
}

fn balance_result_after_lex(ocb: &OrderControlBlock, res: CompareResult) -> CompareResult {
    if ocb.wb > 0 {
        greater_or_uncomparable(ocb)
    } else if ocb.wb < 0 {
        lesser_or_uncomparable(ocb)
    } else if res == CompareResult::Greater {
        greater_or_uncomparable(ocb)
    } else if res == CompareResult::Lesser {
        lesser_or_uncomparable(ocb)
    } else {
        res
    }
}

fn balance_result_after_heads(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
) -> CompareResult {
    if ocb.wb > 0 {
        return greater_or_uncomparable(ocb);
    }
    if ocb.wb < 0 {
        return lesser_or_uncomparable(ocb);
    }

    match ocb.fun_compare(signature, s.f_code(), t.f_code()) {
        CompareResult::Greater => greater_or_uncomparable(ocb),
        CompareResult::Lesser => lesser_or_uncomparable(ocb),
        CompareResult::Equal
        | CompareResult::Uncomparable
        | CompareResult::Unknown
        | CompareResult::NotGreaterEqual
        | CompareResult::NotLessEqual => CompareResult::Equal,
    }
}

fn greater_or_uncomparable(ocb: &OrderControlBlock) -> CompareResult {
    if ocb.neg_bal == 0 {
        CompareResult::Greater
    } else {
        CompareResult::Uncomparable
    }
}

fn lesser_or_uncomparable(ocb: &OrderControlBlock) -> CompareResult {
    if ocb.pos_bal == 0 {
        CompareResult::Lesser
    } else {
        CompareResult::Uncomparable
    }
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{kbo6_compare, kbo6_greater};
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::cto_kbo::kbo_compare;
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
        OrderControlBlock::alloc(TermOrdering::Kbo6, true, signature, HoOrderKind::LfhoOrder)
    }

    fn app(symbol: FunCode, args: &[Term]) -> Term {
        let term = Term::top_alloc(symbol, args.len());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn assert_matches_classic(
        ocb: &mut OrderControlBlock,
        signature: &Signature,
        left: &Term,
        right: &Term,
        expected: CompareResult,
    ) {
        assert_eq!(
            kbo6_compare(
                ocb,
                signature,
                left,
                right,
                DerefType::Never,
                DerefType::Never
            ),
            expected
        );
        assert_eq!(
            kbo_compare(
                ocb,
                signature,
                left,
                right,
                DerefType::Never,
                DerefType::Never
            ),
            expected
        );
    }

    #[test]
    fn kbo6_orders_weight_precedence_and_lexicographic_cases() {
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

        assert_matches_classic(
            &mut ocb,
            &signature,
            &b_term,
            &c_term,
            CompareResult::Greater,
        );
        assert_matches_classic(&mut ocb, &signature, &f_a_b, &f_a_c, CompareResult::Greater);
        assert!(kbo6_greater(
            &mut ocb,
            &signature,
            &f_a_b,
            &f_a_c,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn kbo6_variable_balance_matches_classic_kbo() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x = app(f, std::slice::from_ref(&x));
        let f_y = app(f, std::slice::from_ref(&y));
        let f_a = app(f, &[Term::const_cell_alloc(a)]);

        assert_matches_classic(&mut ocb, &signature, &f_x, &x, CompareResult::Greater);
        assert_matches_classic(&mut ocb, &signature, &x, &f_x, CompareResult::Lesser);
        assert_matches_classic(&mut ocb, &signature, &f_x, &y, CompareResult::Uncomparable);
        assert_matches_classic(
            &mut ocb,
            &signature,
            &f_x,
            &f_y,
            CompareResult::Uncomparable,
        );
        assert_matches_classic(&mut ocb, &signature, &f_a, &x, CompareResult::Uncomparable);
    }

    #[test]
    fn kbo6_resets_balance_state_on_entry_but_leaves_last_trace() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let mut ocb = ocb(&signature);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x = app(f, std::slice::from_ref(&x));

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &f_x,
                &x,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(ocb.max_var > 0);
        assert_ne!(ocb.wb, 0);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &x,
                &y,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Uncomparable
        );
        assert_eq!(ocb.max_var, 4);
        assert_eq!(ocb.wb, 0);
        assert_eq!(ocb.pos_bal, 1);
        assert_eq!(ocb.neg_bal, 1);
    }

    #[test]
    fn kbo6_grows_variable_balance_array_like_c() {
        let signature = signature();
        let mut ocb = ocb(&signature);
        let far = Term::const_cell_alloc(-130);
        let near = Term::const_cell_alloc(-2);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &far,
                &near,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Uncomparable
        );

        assert!(ocb.vb_size > 130);
        assert_eq!(ocb.vb[130], 1);
        assert_eq!(ocb.vb[2], -1);
    }

    #[test]
    fn kbo6_preserves_linear_unordered_head_equal_result() {
        let mut signature = signature();
        let a = symbol(&mut signature, "a", 0);
        let b = symbol(&mut signature, "b", 0);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            false,
            &signature,
            HoOrderKind::LfhoOrder,
        );

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &Term::const_cell_alloc(a),
                &Term::const_cell_alloc(b),
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Equal
        );
    }
}
