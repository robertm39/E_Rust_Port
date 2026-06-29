//! Linear-time first-order KBO6 implementation from `cto_kbolin`.

use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termtypes::{term_deref, DerefType, Term};

/// Compare two first-order terms with C `KBO6Compare`.
///
/// This ports the non-`ENABLE_LFHO` first-order `kbolincmp` path plus the
/// no-WHNF subset of C `kbolincmp_ho` for higher-order surfaces with ordinary
/// dereferencing under `LFHO_ORDER`. The C wrapper resets OCB balance fields
/// before comparison and leaves the final comparison balances in the OCB; this
/// function preserves that entry-reset behavior.
///
/// # Panics
///
/// Panics if a higher-order surface needs Lambda-order normalization, WHNF
/// dereferencing, applied-variable dereference expansion, if term argument
/// slots are uninitialized, or if the OCB lacks KBO weight/precedence storage.
pub fn kbo6_compare(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    kbo6_reset(ocb);
    if needs_lfho_kbo6_surface(s, t, deref_s, deref_t) {
        assert!(
            ocb.ho_order_kind == HoOrderKind::LfhoOrder,
            "Lambda-order KBO6 term ordering is not ported yet"
        );
        assert!(
            deref_s != DerefType::Always && deref_t != DerefType::Always,
            "LFHO KBO6 WHNF dereferencing is not ported yet"
        );
        return kbo_lin_cmp_lfho_no_whnf(ocb, signature, s, t, deref_s, deref_t);
    }
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
    if ocb.ho_order_kind == HoOrderKind::LambdaOrder {
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

fn needs_lfho_kbo6_surface(s: &Term, t: &Term, deref_s: DerefType, deref_t: DerefType) -> bool {
    if problem_type() == ProblemType::HigherOrder {
        term_needs_lfho_kbo6_surface(s, deref_s) || term_needs_lfho_kbo6_surface(t, deref_t)
    } else {
        false
    }
}

fn term_needs_lfho_kbo6_surface(term: &Term, deref: DerefType) -> bool {
    if term.has_higher_order_ordering_surface() {
        return true;
    }
    if deref == DerefType::Never || !term.is_free_var() {
        return false;
    }
    let Some(binding) = term.binding() else {
        return false;
    };
    binding_has_lfho_kbo6_surface(&binding, deref)
}

fn binding_has_lfho_kbo6_surface(binding: &Term, deref: DerefType) -> bool {
    if binding.has_higher_order_ordering_surface() {
        return true;
    }
    if deref != DerefType::Always {
        return false;
    }
    let mut current = binding.clone();
    while current.is_free_var() {
        let Some(next) = current.binding() else {
            return false;
        };
        if next.has_higher_order_ordering_surface() {
            return true;
        }
        current = next;
    }
    false
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
        assert!(
            !lfho_bound_applied_free_var_needs_expansion(&candidate, current_deref),
            "LFHO KBO6 applied-variable dereferencing is not ported yet"
        );
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

fn kbo_lin_cmp_lfho_no_whnf(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let (s, deref_s, limit_s) = lfho_deref_no_whnf(s, deref_s);
    let (t, deref_t, limit_t) = lfho_deref_no_whnf(t, deref_t);
    let mut res = CompareResult::Equal;
    if s.f_code() == t.f_code() {
        let mut done = if s.arity() == t.arity() {
            s.arity() == 0
        } else {
            false
        };
        let mut index = 0;
        while !done {
            res = if s.arity() == t.arity() {
                kbo_lin_cmp_lfho_no_whnf(
                    ocb,
                    signature,
                    &initialized_arg(&s, index),
                    &initialized_arg(&t, index),
                    convert_lfho_deref(index, limit_s, deref_s),
                    convert_lfho_deref(index, limit_t, deref_t),
                )
            } else {
                cmp_arities(&s, &t)
            };

            if res != CompareResult::Equal {
                if s.arity() == t.arity() {
                    index += 1;
                }
                if index < s.arity() || index < t.arity() {
                    for rest in index..s.arity() {
                        mfy_vwb_lhs(
                            ocb,
                            &initialized_arg(&s, rest),
                            convert_lfho_deref(rest, limit_s, deref_s),
                        );
                    }
                    for rest in index..t.arity() {
                        mfy_vwb_rhs(
                            ocb,
                            &initialized_arg(&t, rest),
                            convert_lfho_deref(rest, limit_t, deref_t),
                        );
                    }
                    res = balance_result_after_lex(ocb, res);
                }
                done = true;
            } else {
                assert_eq!(
                    s.arity(),
                    t.arity(),
                    "equal LFHO KBO6 recursive result requires matching arity"
                );
                index += 1;
                done = index == s.arity();
            }
        }
    } else if s.is_free_var() {
        if t.is_free_var() {
            inc_vb(ocb, &s);
            dec_vb(ocb, &t);
            res = if s == t {
                CompareResult::Equal
            } else {
                CompareResult::Uncomparable
            };
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
        if ocb.wb > 0 {
            res = greater_or_uncomparable(ocb);
        } else if ocb.wb < 0 {
            res = lesser_or_uncomparable(ocb);
        } else {
            let head_cmp = if s.is_top_level_any_var() || t.is_top_level_any_var() {
                CompareResult::Uncomparable
            } else {
                ocb.fun_compare(signature, s.f_code(), t.f_code())
            };
            res = match head_cmp {
                CompareResult::Greater => greater_or_uncomparable(ocb),
                CompareResult::Lesser => lesser_or_uncomparable(ocb),
                _ => CompareResult::Uncomparable,
            };
        }
    }
    res
}

fn lfho_deref_no_whnf(term: &Term, deref: DerefType) -> (Term, DerefType, usize) {
    assert!(
        !lfho_bound_applied_free_var_needs_expansion(term, deref),
        "LFHO KBO6 applied-variable dereferencing is not ported yet"
    );
    let limit = lfho_deref_limit(term, deref);
    let mut current_deref = deref;
    let term = term_deref(term, &mut current_deref);
    (term, current_deref, limit)
}

fn lfho_deref_limit(term: &Term, deref: DerefType) -> usize {
    if deref != DerefType::Once || !term.is_applied_free_var() {
        return 0;
    }
    let Some(head) = term.argument(0) else {
        return 0;
    };
    let Some(binding) = head.binding() else {
        return 0;
    };
    if binding.is_lambda() {
        1
    } else {
        binding.arity() + usize::from(binding.is_free_var())
    }
}

fn convert_lfho_deref(index: usize, limit: usize, deref: DerefType) -> DerefType {
    if index < limit && deref == DerefType::Once {
        DerefType::Never
    } else {
        deref
    }
}

fn lfho_bound_applied_free_var_needs_expansion(term: &Term, deref: DerefType) -> bool {
    deref == DerefType::Once
        && term.is_applied_free_var()
        && term
            .argument(0)
            .is_some_and(|head| head.binding().is_some())
}

fn cmp_arities(left: &Term, right: &Term) -> CompareResult {
    assert_ne!(
        left.arity(),
        right.arity(),
        "arity comparison needs a difference"
    );
    if left.arity() > right.arity() {
        CompareResult::Greater
    } else {
        CompareResult::Lesser
    }
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
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::cto_kbo::kbo_compare;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::lambda::close_with_db_var;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

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

    fn test_bank() -> TermBank {
        TermBank::new(signature()).unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = symbol(bank.signature_mut(), name, 0);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap_or_else(|err| panic!("{err}"));
        }
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let symbol_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_]));
        let f_code = symbol(bank.signature_mut(), name, 0);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, symbol_type)
                .unwrap_or_else(|err| panic!("{err}"));
        }
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
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

    #[test]
    fn kbo6_lfho_no_whnf_orders_closed_lambdas_by_body() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "kbo6_lfho_lambda_a");
        let b = typed_const(&mut bank, "kbo6_lfho_lambda_b");
        let lambda_a =
            close_with_db_var(&mut bank, &binder_type, &a).unwrap_or_else(|err| panic!("{err}"));
        let lambda_b =
            close_with_db_var(&mut bank, &binder_type, &b).unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = ocb(bank.signature());
        ocb.set_fun_prec_weight(b.f_code(), 30);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &lambda_b,
                &lambda_a,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn kbo6_lfho_no_whnf_uses_length_lexicographic_phony_applications() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let head = bank.request_db_var(&type_, 0);
        let a = typed_const(&mut bank, "kbo6_lfho_phony_a");
        let b = typed_const(&mut bank, "kbo6_lfho_phony_b");
        let short = app(SIG_PHONY_APP_CODE, &[head.clone(), a.clone()]);
        let long = app(SIG_PHONY_APP_CODE, &[head, a, b]);
        let mut ocb = ocb(bank.signature());

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &long,
                &short,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn kbo6_lfho_deref_once_follows_bound_lambda_surface() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "kbo6_lfho_bound_lambda_a");
        let b = typed_const(&mut bank, "kbo6_lfho_bound_lambda_b");
        let lambda_a =
            close_with_db_var(&mut bank, &binder_type, &a).unwrap_or_else(|err| panic!("{err}"));
        let lambda_b =
            close_with_db_var(&mut bank, &binder_type, &b).unwrap_or_else(|err| panic!("{err}"));
        let lambda_type = lambda_b.type_().expect("lambda must have a type");
        let x = bank.vars().get_fresh_var(&lambda_type);
        let mut subst = Substitution::new();
        subst.add_binding(&x, &lambda_b);
        let mut ocb = ocb(bank.signature());
        ocb.set_fun_prec_weight(b.f_code(), 30);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &x,
                &lambda_a,
                DerefType::Once,
                DerefType::Never
            ),
            CompareResult::Greater
        );

        subst.backtrack();
    }

    #[test]
    fn kbo6_lfho_deref_once_propagates_through_phony_arguments() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let head = bank.request_db_var(&type_, 0);
        let a = typed_const(&mut bank, "kbo6_lfho_arg_a");
        let b = typed_const(&mut bank, "kbo6_lfho_arg_b");
        let x = bank.vars().get_fresh_var(&type_);
        let mut subst = Substitution::new();
        subst.add_binding(&x, &b);
        let left = app(SIG_PHONY_APP_CODE, &[head.clone(), x.clone()]);
        let right = app(SIG_PHONY_APP_CODE, &[head, a.clone()]);
        let mut ocb = ocb(bank.signature());
        ocb.set_fun_prec_weight(b.f_code(), 30);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &left,
                &right,
                DerefType::Once,
                DerefType::Never
            ),
            CompareResult::Greater
        );

        subst.backtrack();
    }

    #[test]
    #[should_panic(expected = "LFHO KBO6 WHNF dereferencing is not ported yet")]
    fn kbo6_lfho_deref_always_stays_diagnostic() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "kbo6_lfho_deref_a");
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &a).unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = ocb(bank.signature());

        kbo6_compare(
            &mut ocb,
            bank.signature(),
            &lambda,
            &a,
            DerefType::Always,
            DerefType::Never,
        );
    }

    #[test]
    #[should_panic(expected = "LFHO KBO6 applied-variable dereferencing is not ported yet")]
    fn kbo6_lfho_deref_once_bound_applied_free_var_stays_diagnostic() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let head_binding = typed_unary_const(&mut bank, "kbo6_lfho_applied_binding");
        let head_type = head_binding.type_().expect("binding must have a type");
        let head = bank.vars().get_fresh_var(&head_type);
        let a = typed_const(&mut bank, "kbo6_lfho_applied_arg");
        let applied = app(SIG_PHONY_APP_CODE, &[head.clone(), a.clone()]);
        applied.set_type(Some(type_));
        let mut subst = Substitution::new();
        subst.add_binding(&head, &head_binding);
        let mut ocb = ocb(bank.signature());

        kbo6_compare(
            &mut ocb,
            bank.signature(),
            &applied,
            &a,
            DerefType::Once,
            DerefType::Never,
        );
    }
}
