//! Linear-time first-order KBO6 implementation from `cto_kbolin`.

use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::lambda::{beta_normalize_db, lambda_eta_reduce_db};
use crate::terms::signature::{Signature, SIG_TRUE_CODE};
use crate::terms::simpletypes::Type;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_is_ground;
use crate::terms::termtypes::{
    term_deref, term_identity_id, DerefType, Term, TP_IS_DB_VAR, TP_PRED_POS,
};
use std::cmp::Ordering;

/// Compare two first-order terms with C `KBO6Compare`.
///
/// This ports the non-`ENABLE_LFHO` first-order `kbolincmp` path plus the
/// direct LFHO `kbolincmp_ho` path used by `LFHO_ORDER`. Since Rust term cells
/// do not yet retain owner-bank metadata, `DerefType::Always` uses a local
/// weak-head dereference rebuilt only for comparison instead of C's cache-backed
/// `WHNF_deref`. The C wrapper resets OCB balance fields before comparison and
/// leaves the final comparison balances in the OCB; this function preserves that
/// entry-reset behavior.
///
/// # Panics
///
/// Panics if term argument slots are uninitialized, or if the OCB lacks KBO
/// weight/precedence storage.
pub fn kbo6_compare(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    kbo6_reset(ocb);
    if problem_type() == ProblemType::HigherOrder {
        return match ocb.ho_order_kind {
            HoOrderKind::LfhoOrder => kbo_lin_cmp_lfho(ocb, signature, s, t, deref_s, deref_t),
            HoOrderKind::LambdaOrder => {
                kbo_lin_cmp_lambda_no_bank(ocb, signature, s, t, deref_s, deref_t)
            }
        };
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

/// Compare two terms with the KBO6 Lambda-order owner-bank normalization path.
///
/// This ports the C `kbolincmp_lambda` wrapper for callers that can provide the
/// term bank used for instantiated insertion, beta-normalization, and
/// eta-reduction. Other KBO6 branches use the same comparison logic as
/// [`kbo6_compare`].
///
/// # Errors
///
/// Returns a diagnostic if instantiated insertion or lambda normalization fails.
///
/// # Panics
///
/// Panics under the same structural invariants as [`kbo6_compare`].
pub fn kbo6_compare_with_bank(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<CompareResult, Diagnostic> {
    kbo6_reset(ocb);
    if problem_type() == ProblemType::HigherOrder {
        return match ocb.ho_order_kind {
            HoOrderKind::LfhoOrder => Ok(kbo_lin_cmp_lfho(
                ocb,
                bank.signature(),
                s,
                t,
                deref_s,
                deref_t,
            )),
            HoOrderKind::LambdaOrder => {
                let s = lambda_order_prepare(bank, s, deref_s)?;
                let t = lambda_order_prepare(bank, t, deref_t)?;
                Ok(kbo_lin_cmp_lambda_no_bank(
                    ocb,
                    bank.signature(),
                    &s,
                    &t,
                    DerefType::Never,
                    DerefType::Never,
                ))
            }
        };
    }
    Ok(kbo_lin_cmp(ocb, bank.signature(), s, t, deref_s, deref_t))
}

/// Return whether `s` is strictly greater than `t` using bank-backed KBO6.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed Lambda-order preparation fails.
///
/// # Panics
///
/// Panics under the same structural invariants as [`kbo6_compare_with_bank`].
pub fn kbo6_greater_with_bank(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<bool, Diagnostic> {
    Ok(kbo6_compare_with_bank(ocb, bank, s, t, deref_s, deref_t)? == CompareResult::Greater)
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
    var_index_from_code(var.f_code())
}

fn var_index_from_code(f_code: i64) -> usize {
    assert!(f_code < 0, "KBO6 variable f-code must be negative");
    usize::try_from(-f_code).unwrap_or_else(|_| panic!("variable index must fit usize"))
}

fn inc_vb_code(ocb: &mut OrderControlBlock, f_code: i64) {
    let index = var_index_from_code(f_code);
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

fn dec_vb_code(ocb: &mut OrderControlBlock, f_code: i64) {
    let index = var_index_from_code(f_code);
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

fn mfy_vwb_lhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb(ocb, term, deref, true);
}

fn mfy_vwb_rhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb(ocb, term, deref, false);
}

fn mfy_vwb_lfho_lhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb_lfho(ocb, term, deref, true);
}

fn mfy_vwb_lfho_rhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb_lfho(ocb, term, deref, false);
}

#[allow(
    unsafe_code,
    reason = "measured private first-order traversal over stable Rc term allocations"
)]
fn mfy_vwb(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType, lhs: bool) {
    ocb.kbo_borrowed_balance_stack.clear();
    ocb.kbo_borrowed_balance_stack
        .push((term.borrowed_cell(), deref));
    while let Some((candidate, mut current_deref)) = ocb.kbo_borrowed_balance_stack.pop() {
        // SAFETY: `term` remains borrowed for the complete traversal and owns
        // every structural descendant of the initial cursor. First-order KBO
        // comparison does not replace argument or binding slots, release a
        // reachable root, or invoke user code. Every followed binding remains
        // owned by its variable cell. The single-threaded `Rc` graph prevents
        // concurrent structural mutation, and all pointers preserve
        // `Rc::as_ptr` provenance, alignment, and initialization. Entry clears
        // stale cursors left by a caught invariant panic before any cursor is
        // dereferenced.
        unsafe {
            let current = candidate.deref_first_order(&mut current_deref);
            let f_code = current.f_code();
            if f_code < 0 {
                if lhs {
                    inc_vb_code(ocb, f_code);
                } else {
                    dec_vb_code(ocb, f_code);
                }
            } else {
                if lhs {
                    ocb.wb += ocb.fun_weight(f_code);
                } else {
                    ocb.wb -= ocb.fun_weight(f_code);
                }
                current.push_first_order_arguments_reversed(
                    &mut ocb.kbo_borrowed_balance_stack,
                    current_deref,
                );
            }
        }
    }
    debug_assert!(ocb.kbo_borrowed_balance_stack.is_empty());
}

fn mfy_vwb_lfho(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType, lhs: bool) {
    debug_assert!(
        ocb.kbo_balance_stack.is_empty(),
        "KBO balance traversal scratch must be empty on entry"
    );
    ocb.kbo_balance_stack.push((term.clone(), deref));
    while let Some((candidate, current_deref)) = ocb.kbo_balance_stack.pop() {
        let (current, current_deref, limit) = lfho_deref_for_kbo(&candidate, current_deref);
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
            let arguments = current.arguments();
            for (index, arg) in arguments.iter().enumerate() {
                let arg = arg
                    .as_ref()
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                ocb.kbo_balance_stack
                    .push((arg.clone(), convert_lfho_deref(index, limit, current_deref)));
            }
        }
    }
    debug_assert!(ocb.kbo_balance_stack.is_empty());
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

fn kbo_lin_cmp_lfho(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let (s, deref_s, limit_s) = lfho_deref_for_kbo(s, deref_s);
    let (t, deref_t, limit_t) = lfho_deref_for_kbo(t, deref_t);
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
                kbo_lin_cmp_lfho(
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

            if res == CompareResult::Equal {
                assert_eq!(
                    s.arity(),
                    t.arity(),
                    "equal LFHO KBO6 recursive result requires matching arity"
                );
                index += 1;
                done = index == s.arity();
            } else {
                if s.arity() == t.arity() {
                    index += 1;
                }
                res = balance_lfho_rest_after_difference(
                    ocb,
                    LfhoRest::new(&s, limit_s, deref_s),
                    LfhoRest::new(&t, limit_t, deref_t),
                    index,
                    res,
                );
                done = true;
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
            mfy_vwb_lfho_rhs(ocb, &t, deref_t);
            res = if ocb.pos_bal == 0 {
                CompareResult::Lesser
            } else {
                CompareResult::Uncomparable
            };
        }
    } else if t.is_free_var() {
        dec_vb(ocb, &t);
        mfy_vwb_lfho_lhs(ocb, &s, deref_s);
        res = if ocb.neg_bal == 0 {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        };
    } else {
        mfy_vwb_lfho_lhs(ocb, &s, deref_s);
        mfy_vwb_lfho_rhs(ocb, &t, deref_t);
        res = match ocb.wb.cmp(&0) {
            Ordering::Greater => greater_or_uncomparable(ocb),
            Ordering::Less => lesser_or_uncomparable(ocb),
            Ordering::Equal => match lfho_head_compare(ocb, signature, &s, &t) {
                CompareResult::Greater => greater_or_uncomparable(ocb),
                CompareResult::Lesser => lesser_or_uncomparable(ocb),
                _ => CompareResult::Uncomparable,
            },
        };
    }
    res
}

/// Return whether Lambda-order KBO6 can compare `term` without the C
/// owner-bank normalization path.
///
/// This is intentionally conservative: it accepts terms whose exposed
/// dereferenced shape has no lambda surface after the local weak-head reduction
/// used by the no-bank comparator. The full C branch first inserts
/// instantiated terms into the owner bank, beta-normalizes, and eta-reduces
/// them; this predicate marks cases where that owner-bank work is unnecessary
/// for the current Rust subset. DB variables and variable-headed phony
/// applications are accepted because the Lambda-order driver handles those
/// shapes directly.
#[must_use]
#[cfg(test)]
pub(crate) fn kbo6_lambda_order_can_skip_bank_normalization(term: &Term, deref: DerefType) -> bool {
    let mut stack = vec![(term.clone(), deref)];
    while let Some((candidate, current_deref)) = stack.pop() {
        let (current, current_deref) = lambda_deref_for_kbo(&candidate, current_deref);
        if current.is_lambda() {
            return false;
        }
        for arg in current.argument_clones().into_iter().flatten() {
            stack.push((arg, current_deref));
        }
    }
    true
}

fn lambda_order_prepare(
    bank: &mut TermBank,
    term: &Term,
    deref: DerefType,
) -> Result<Term, Diagnostic> {
    let instantiated = bank.insert_instantiated_deref(term, deref)?;
    let beta_normal = beta_normalize_db(bank, &instantiated)?;
    lambda_eta_reduce_db(bank, &beta_normal)
}

fn kbo_lin_cmp_lambda_no_bank(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let (s, deref_s) = lambda_deref_for_kbo(s, deref_s);
    let (t, deref_t) = lambda_deref_for_kbo(t, deref_t);

    if s.f_code() == SIG_TRUE_CODE {
        if t.f_code() == SIG_TRUE_CODE {
            CompareResult::Equal
        } else {
            CompareResult::Lesser
        }
    } else if t.f_code() == SIG_TRUE_CODE {
        CompareResult::Greater
    } else {
        kbo_lin_cmp_lambda_driver_deref(ocb, signature, &s, &t, deref_s, deref_t)
    }
}

fn lambda_deref_for_kbo(term: &Term, deref: DerefType) -> (Term, DerefType) {
    if deref == DerefType::Always {
        return (whnf_deref_for_kbo(term), DerefType::Never);
    }

    let mut current_deref = deref;
    let term = term_deref(term, &mut current_deref);
    if term.is_phony_app() && term.argument(0).is_some_and(|head| head.is_lambda()) {
        (whnf_deref_for_kbo(&term), DerefType::Never)
    } else {
        (term, current_deref)
    }
}

fn mfy_vwb_lambda_lhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb_lambda(ocb, term, deref, true);
}

fn mfy_vwb_lambda_rhs(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType) {
    mfy_vwb_lambda(ocb, term, deref, false);
}

fn mfy_vwb_lambda(ocb: &mut OrderControlBlock, term: &Term, deref: DerefType, lhs: bool) {
    debug_assert!(
        ocb.kbo_balance_stack.is_empty(),
        "KBO balance traversal scratch must be empty on entry"
    );
    ocb.kbo_balance_stack.push((term.clone(), deref));
    while let Some((candidate, current_deref)) = ocb.kbo_balance_stack.pop() {
        let (current, current_deref) = lambda_deref_for_kbo(&candidate, current_deref);
        if is_fluid_lambda(&current) {
            if lhs {
                ocb.inc_ho_var_balance(&current);
            } else {
                ocb.dec_ho_var_balance(&current);
            }
        } else {
            let weight = lambda_order_term_weight(ocb, &current);
            if lhs {
                ocb.wb += weight;
            } else {
                ocb.wb -= weight;
            }

            for index in usize::from(current.is_lambda())..current.arity() {
                ocb.kbo_balance_stack
                    .push((initialized_arg(&current, index), current_deref));
            }
        }
    }
    debug_assert!(ocb.kbo_balance_stack.is_empty());
}

fn lambda_order_term_weight(ocb: &OrderControlBlock, term: &Term) -> i64 {
    if term.is_lambda() {
        ocb.lam_weight
    } else if term.is_db_var() {
        ocb.db_weight
    } else if term.is_phony_app() {
        0
    } else {
        ocb.fun_weight(term.f_code())
    }
}

fn kbo_lin_cmp_lambda_driver(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let (s, deref_s) = lambda_deref_for_kbo(s, deref_s);
    let (t, deref_t) = lambda_deref_for_kbo(t, deref_t);
    kbo_lin_cmp_lambda_driver_deref(ocb, signature, &s, &t, deref_s, deref_t)
}

fn kbo_lin_cmp_lambda_driver_deref(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    if is_fluid_lambda(s) {
        if is_fluid_lambda(t) {
            ocb.inc_ho_var_balance(s);
            ocb.dec_ho_var_balance(t);
            if term_identity_id(s) == term_identity_id(t) {
                CompareResult::Equal
            } else {
                CompareResult::Uncomparable
            }
        } else {
            ocb.inc_ho_var_balance(s);
            mfy_vwb_lambda_rhs(ocb, t, deref_t);
            if ocb.pos_bal == 0 {
                CompareResult::Lesser
            } else {
                CompareResult::Uncomparable
            }
        }
    } else if is_fluid_lambda(t) {
        ocb.dec_ho_var_balance(t);
        mfy_vwb_lambda_lhs(ocb, s, deref_s);
        if ocb.neg_bal == 0 {
            CompareResult::Greater
        } else {
            CompareResult::Uncomparable
        }
    } else if heads_same_lambda(s, t) {
        compare_lambda_same_heads(ocb, signature, s, t, deref_s, deref_t)
    } else {
        mfy_vwb_lambda_lhs(ocb, s, deref_s);
        mfy_vwb_lambda_rhs(ocb, t, deref_t);
        balance_lambda_result_after_heads(ocb, signature, s, t)
    }
}

fn is_fluid_lambda(term: &Term) -> bool {
    term.is_top_level_free_var() || (term.is_lambda() && !term_is_ground(term))
}

fn compare_lambda_same_heads(
    ocb: &mut OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let mut res = CompareResult::Equal;
    let mut index = 0;
    let mut done = s.arity() == t.arity() && s.arity() == 0;

    while !done {
        res = if s.arity() == t.arity() {
            kbo_lin_cmp_lambda_driver(
                ocb,
                signature,
                &initialized_arg(s, index),
                &initialized_arg(t, index),
                deref_s,
                deref_t,
            )
        } else {
            cmp_arities(s, t)
        };

        if res == CompareResult::Equal {
            assert_eq!(
                s.arity(),
                t.arity(),
                "equal Lambda-order KBO6 recursive result requires matching arity"
            );
            index += 1;
            done = index == s.arity();
        } else {
            if s.arity() == t.arity() {
                index += 1;
            }
            res = balance_lambda_rest_after_difference(ocb, s, t, index, res, deref_s, deref_t);
            done = true;
        }
    }

    res
}

fn balance_lambda_rest_after_difference(
    ocb: &mut OrderControlBlock,
    s: &Term,
    t: &Term,
    index: usize,
    res: CompareResult,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let left_start = lambda_rest_start(s, index);
    let right_start = lambda_rest_start(t, index);
    if left_start < s.arity() || right_start < t.arity() {
        for rest in left_start..s.arity() {
            mfy_vwb_lambda_lhs(ocb, &initialized_arg(s, rest), deref_s);
        }
        for rest in right_start..t.arity() {
            mfy_vwb_lambda_rhs(ocb, &initialized_arg(t, rest), deref_t);
        }
        balance_result_after_lex(ocb, res)
    } else {
        res
    }
}

fn lambda_rest_start(term: &Term, index: usize) -> usize {
    if index == 0 && (term.is_phony_app() || term.is_lambda()) {
        1
    } else {
        index
    }
}

fn balance_lambda_result_after_heads(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
) -> CompareResult {
    match ocb.wb.cmp(&0) {
        Ordering::Greater => greater_or_uncomparable(ocb),
        Ordering::Less => lesser_or_uncomparable(ocb),
        Ordering::Equal => match cmp_heads_lambda(ocb, signature, s, t) {
            CompareResult::Greater => greater_or_uncomparable(ocb),
            CompareResult::Lesser => lesser_or_uncomparable(ocb),
            CompareResult::Equal
            | CompareResult::Uncomparable
            | CompareResult::Unknown
            | CompareResult::NotGreaterEqual
            | CompareResult::NotLessEqual => CompareResult::Uncomparable,
        },
    }
}

fn heads_same_lambda(s: &Term, t: &Term) -> bool {
    if !s.is_phony_app() && !t.is_phony_app() {
        s.f_code() == t.f_code()
    } else {
        s.is_phony_app()
            && t.is_phony_app()
            && term_identity_id(&initialized_arg(s, 0)) == term_identity_id(&initialized_arg(t, 0))
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum LambdaHeadClass {
    Symbol,
    DbOrPhony,
    Lambda,
}

fn classify_head_lambda(term: &Term) -> LambdaHeadClass {
    if term.is_lambda() {
        LambdaHeadClass::Lambda
    } else if term.is_db_var() || term.is_phony_app() {
        LambdaHeadClass::DbOrPhony
    } else {
        LambdaHeadClass::Symbol
    }
}

fn cmp_heads_lambda(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
) -> CompareResult {
    let s_class = classify_head_lambda(s);
    let t_class = classify_head_lambda(t);
    match s_class.cmp(&t_class) {
        Ordering::Greater => CompareResult::Greater,
        Ordering::Less => CompareResult::Lesser,
        Ordering::Equal => {
            if !s.is_top_level_any_var() && !t.is_top_level_any_var() && !s.is_lambda() {
                ocb.fun_compare(signature, s.f_code(), t.f_code())
            } else if s.is_top_level_any_var() && t.is_top_level_any_var() {
                match lambda_top_level_var_code(s).cmp(&lambda_top_level_var_code(t)) {
                    Ordering::Greater => CompareResult::Greater,
                    Ordering::Less => CompareResult::Lesser,
                    Ordering::Equal => CompareResult::Equal,
                }
            } else {
                CompareResult::Uncomparable
            }
        }
    }
}

fn lambda_top_level_var_code(term: &Term) -> i64 {
    if term.is_applied_any_var() {
        initialized_arg(term, 0).f_code()
    } else {
        term.f_code()
    }
}

#[derive(Clone, Copy)]
struct LfhoRest<'term> {
    term: &'term Term,
    limit: usize,
    deref: DerefType,
}

impl<'term> LfhoRest<'term> {
    const fn new(term: &'term Term, limit: usize, deref: DerefType) -> Self {
        Self { term, limit, deref }
    }
}

fn balance_lfho_rest_after_difference(
    ocb: &mut OrderControlBlock,
    left: LfhoRest<'_>,
    right: LfhoRest<'_>,
    index: usize,
    res: CompareResult,
) -> CompareResult {
    if index < left.term.arity() || index < right.term.arity() {
        for rest in index..left.term.arity() {
            mfy_vwb_lfho_lhs(
                ocb,
                &initialized_arg(left.term, rest),
                convert_lfho_deref(rest, left.limit, left.deref),
            );
        }
        for rest in index..right.term.arity() {
            mfy_vwb_lfho_rhs(
                ocb,
                &initialized_arg(right.term, rest),
                convert_lfho_deref(rest, right.limit, right.deref),
            );
        }
        balance_result_after_lex(ocb, res)
    } else {
        res
    }
}

fn lfho_head_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
) -> CompareResult {
    if s.is_top_level_any_var() || t.is_top_level_any_var() {
        CompareResult::Uncomparable
    } else {
        ocb.fun_compare(signature, s.f_code(), t.f_code())
    }
}

fn lfho_deref_for_kbo(term: &Term, deref: DerefType) -> (Term, DerefType, usize) {
    let limit = lfho_deref_limit(term, deref);
    if deref == DerefType::Always {
        return (whnf_deref_for_kbo(term), deref, limit);
    }
    if deref == DerefType::Once
        && term.is_applied_free_var()
        && term
            .argument(0)
            .is_some_and(|head| head.binding().is_some())
    {
        return (expand_lfho_applied_free_var_once(term), deref, limit);
    }
    let mut current_deref = deref;
    let term = term_deref(term, &mut current_deref);
    (term, current_deref, limit)
}

fn whnf_deref_for_kbo(term: &Term) -> Term {
    let mut deref = DerefType::Always;
    let term = term_deref(term, &mut deref);

    if term.is_phony_app() && term.argument(0).is_some_and(|head| head.is_lambda()) {
        let reduced = whnf_step_for_kbo(&term);
        return whnf_deref_for_kbo(&reduced);
    }

    if term.is_lambda() {
        assert_eq!(
            term.arity(),
            2,
            "WHNF dereference expects a binary DB-lambda cell"
        );
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let new_matrix = whnf_deref_for_kbo(&matrix);
        if new_matrix == matrix {
            term
        } else {
            rebuild_db_lambda_for_kbo(&term, new_matrix)
        }
    } else {
        term
    }
}

fn whnf_step_for_kbo(term: &Term) -> Term {
    if !term.is_phony_app() || !term.argument(0).is_some_and(|head| head.is_lambda()) {
        return term.clone();
    }

    let mut num_remaining = term.arity() - 1;
    assert!(num_remaining > 0, "phony application must have arguments");
    let mut next_arg = 1;
    let mut matrix = term
        .argument(0)
        .unwrap_or_else(|| panic!("phony application head is uninitialized"));
    let mut consumed = Vec::new();

    while matrix.is_lambda() && num_remaining != 0 {
        assert_eq!(
            matrix.arity(),
            2,
            "WHNF reduction expects a binary DB-lambda cell"
        );
        let binder = matrix
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let target = term
            .argument(next_arg)
            .unwrap_or_else(|| panic!("application argument {next_arg} is uninitialized"));
        consumed.push(target);
        next_arg += 1;
        num_remaining -= 1;
        matrix = matrix
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
    }

    let total_bound = consumed.len();
    assert!(
        total_bound > 0,
        "WHNF step requires at least one consumed lambda"
    );
    let mut bindings = vec![None; total_bound];
    for (index, target) in consumed.into_iter().enumerate() {
        let db_index = total_bound - index - 1;
        bindings[db_index] = Some(target);
    }

    let mut new_matrix = replace_bound_vars_for_kbo(&matrix, &bindings, 0);
    if num_remaining != 0 {
        let mut rest = Vec::with_capacity(num_remaining);
        for index in next_arg..term.arity() {
            rest.push(
                term.argument(index)
                    .unwrap_or_else(|| panic!("application argument {index} is uninitialized")),
            );
        }
        new_matrix = apply_terms_for_kbo(&new_matrix, &rest, term.type_());
    }
    new_matrix
}

fn replace_bound_vars_for_kbo(term: &Term, bindings: &[Option<Term>], depth: i64) -> Term {
    let total_bound = i64::try_from(bindings.len()).unwrap_or(i64::MAX);
    assert!(
        total_bound > 0,
        "bound-variable replacement requires bindings"
    );

    if term.is_db_var() {
        if term.f_code() < depth {
            return term.clone();
        }
        let loose_index = term.f_code() - depth;
        if loose_index < total_bound {
            let binding = bindings[usize::try_from(loose_index).expect("DB index fits usize")]
                .as_ref()
                .expect("WHNF binding slot is initialized");
            return shift_db_for_kbo(binding, depth, 0);
        }
        return copy_db_var_for_kbo(term, term.f_code() - total_bound);
    }

    if term.is_lambda() {
        assert_eq!(
            term.arity(),
            2,
            "bound-variable replacement expects a binary DB-lambda cell"
        );
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let new_matrix = replace_bound_vars_for_kbo(&matrix, bindings, depth + 1);
        if new_matrix == matrix {
            term.clone()
        } else {
            rebuild_db_lambda_for_kbo(term, new_matrix)
        }
    } else if term.arity() == 0 || !contains_db_subterm_for_kbo(term) {
        term.clone()
    } else {
        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let replaced = replace_bound_vars_for_kbo(&arg, bindings, depth);
            if replaced != arg {
                changed = true;
            }
            copy.set_argument(index, replaced);
        }
        if changed {
            copy
        } else {
            term.clone()
        }
    }
}

fn shift_db_for_kbo(term: &Term, shift_val: i64, depth: i64) -> Term {
    if shift_val == 0 {
        return term.clone();
    }

    if term.is_db_var() {
        if term.f_code() >= depth {
            let shifted = term
                .f_code()
                .checked_add(shift_val)
                .expect("DB variable shift fits in FunCode");
            assert!(shifted >= 0, "DB variable shift produced a negative index");
            return copy_db_var_for_kbo(term, shifted);
        }
        return term.clone();
    }

    if term.is_lambda() {
        assert_eq!(
            term.arity(),
            2,
            "DB shifting expects a binary DB-lambda cell"
        );
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let shifted_matrix = shift_db_for_kbo(&matrix, shift_val, depth + 1);
        if shifted_matrix == matrix {
            term.clone()
        } else {
            rebuild_db_lambda_for_kbo(term, shifted_matrix)
        }
    } else if term.arity() == 0 || !contains_db_subterm_for_kbo(term) {
        term.clone()
    } else {
        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shifted = shift_db_for_kbo(&arg, shift_val, depth);
            if shifted != arg {
                changed = true;
            }
            copy.set_argument(index, shifted);
        }
        if changed {
            copy
        } else {
            term.clone()
        }
    }
}

fn apply_terms_for_kbo(head: &Term, args: &[Term], result_type: Option<Type>) -> Term {
    if args.is_empty() {
        return head.clone();
    }

    let applied = if head.is_any_var() || head.is_lambda() {
        let applied = Term::top_alloc(crate::terms::signature::SIG_PHONY_APP_CODE, args.len() + 1);
        applied.set_argument(0, head.clone());
        for (index, arg) in args.iter().enumerate() {
            applied.set_argument(index + 1, arg.clone());
        }
        applied
    } else {
        let applied = Term::top_alloc(head.f_code(), head.arity() + args.len());
        for (index, arg) in head.argument_clones().into_iter().enumerate() {
            applied.set_argument(
                index,
                arg.unwrap_or_else(|| panic!("head argument {index} is uninitialized")),
            );
        }
        for (index, arg) in args.iter().enumerate() {
            applied.set_argument(head.arity() + index, arg.clone());
        }
        applied
    };
    applied.set_type(result_type);
    applied
}

fn rebuild_db_lambda_for_kbo(lambda: &Term, matrix: Term) -> Term {
    assert!(
        lambda.is_lambda(),
        "lambda rebuild expects a lambda top cell"
    );
    let binder = lambda
        .argument(0)
        .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
    assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
    let rebuilt = Term::top_copy_without_args(lambda);
    rebuilt.set_argument(0, binder);
    rebuilt.set_argument(1, matrix);
    rebuilt
}

fn copy_db_var_for_kbo(source: &Term, f_code: i64) -> Term {
    assert!(source.is_db_var(), "DB copy expects a DB variable");
    let copy = Term::const_cell_alloc(f_code);
    copy.set_prop(TP_IS_DB_VAR);
    copy.set_type(source.type_());
    copy
}

fn contains_db_subterm_for_kbo(term: &Term) -> bool {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_db_var() {
            return true;
        }
        stack.extend(current.argument_clones().into_iter().flatten());
    }
    false
}

fn expand_lfho_applied_free_var_once(term: &Term) -> Term {
    assert!(term.is_applied_free_var(), "expected applied free variable");
    assert!(
        term.arity() > 1,
        "applied free variable must have arguments"
    );
    let head = term.argument(0).expect("applied free variable has a head");
    let binding = head.binding().expect("applied free variable head is bound");

    if binding.is_any_var() || binding.is_lambda() {
        let expanded = Term::top_alloc(term.f_code(), term.arity());
        expanded.set_properties(term.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        expanded.set_argument(0, binding);
        for index in 1..term.arity() {
            expanded.set_argument(index, initialized_arg(term, index));
        }
        expanded
    } else {
        let expanded = Term::top_alloc(binding.f_code(), binding.arity() + term.arity() - 1);
        expanded.set_properties(binding.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        for index in 0..binding.arity() {
            expanded.set_argument(index, initialized_arg(&binding, index));
        }
        for index in 1..term.arity() {
            expanded.set_argument(binding.arity() + index - 1, initialized_arg(term, index));
        }
        expanded
    }
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
    use super::{
        kbo6_compare, kbo6_compare_with_bank, kbo6_greater, kbo6_greater_with_bank,
        kbo6_lambda_order_can_skip_bank_normalization, mfy_vwb_lhs, mfy_vwb_rhs,
    };
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::cto_kbo::kbo_compare;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::lambda::{apply_terms, close_with_db_var};
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

    fn lambda_ocb(signature: &Signature) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            signature,
            HoOrderKind::LambdaOrder,
        )
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
    fn kbo6_reuses_balance_traversal_scratch() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 2);
        let a = Term::const_cell_alloc(symbol(&mut signature, "a", 0));
        let x = Term::const_cell_alloc(-2);
        let nested = app(
            f,
            &[app(f, &[x.clone(), a.clone()]), app(f, &[a.clone(), x])],
        );
        let mut ocb = ocb(&signature);

        mfy_vwb_lhs(&mut ocb, &nested, DerefType::Never);
        assert!(ocb.kbo_borrowed_balance_stack.is_empty());
        let capacity = ocb.kbo_borrowed_balance_stack.capacity();
        assert!(capacity > 0);

        mfy_vwb_rhs(&mut ocb, &nested, DerefType::Never);
        assert!(ocb.kbo_borrowed_balance_stack.is_empty());
        assert_eq!(ocb.kbo_borrowed_balance_stack.capacity(), capacity);
        assert_eq!(ocb.wb, 0);
        assert_eq!(ocb.pos_bal, 0);
        assert_eq!(ocb.neg_bal, 0);
    }

    #[test]
    fn kbo6_borrowed_balance_discards_stale_cursors_after_unwind() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 2);
        let a = Term::const_cell_alloc(symbol(&mut signature, "a", 0));
        let invalid_var = Term::const_cell_alloc(i64::MIN);
        let broken = app(f, &[invalid_var, a.clone()]);
        let valid = app(f, std::slice::from_ref(&a));
        let mut ocb = ocb(&signature);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            mfy_vwb_lhs(&mut ocb, &broken, DerefType::Never);
        }));
        assert!(result.is_err());
        assert!(!ocb.kbo_borrowed_balance_stack.is_empty());

        mfy_vwb_lhs(&mut ocb, &valid, DerefType::Never);
        assert!(ocb.kbo_borrowed_balance_stack.is_empty());
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
    fn kbo6_higher_order_lfho_uses_higher_order_unordered_head_result() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
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
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn kbo6_higher_order_lambda_order_handles_first_order_terms() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut signature = signature();
        let a = symbol(&mut signature, "a", 0);
        let b = symbol(&mut signature, "b", 0);
        let mut ocb = lambda_ocb(&signature);
        ocb.set_fun_prec_weight(b, 30);
        ocb.set_fun_prec_weight(a, 10);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &Term::const_cell_alloc(b),
                &Term::const_cell_alloc(a),
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn kbo6_higher_order_lambda_order_tracks_fluid_variables_by_identity() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let signature = signature();
        let mut ocb = lambda_ocb(&signature);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-2);

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &x,
                &x,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Equal
        );
        assert!(ocb.ho_vb.values().all(|balance| *balance == 0));

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                &signature,
                &x,
                &y,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Uncomparable
        );
        assert_eq!(ocb.ho_vb.len(), 2);
    }

    #[test]
    fn kbo6_higher_order_lambda_order_handles_db_variable_applications() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let result_type = bank.signature().type_bank().default_type();
        let head_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                result_type.clone(),
                result_type.clone(),
            ]));
        let head = bank.request_db_var(&head_type, 0);
        let a = typed_const(&mut bank, "kbo6_lambda_db_app_a");
        let b = typed_const(&mut bank, "kbo6_lambda_db_app_b");
        let short = app(SIG_PHONY_APP_CODE, &[head.clone(), a.clone()]);
        short.set_type(Some(result_type.clone()));
        let long = app(SIG_PHONY_APP_CODE, &[head, a, b]);
        long.set_type(Some(result_type));
        let mut ocb = lambda_ocb(bank.signature());

        assert!(kbo6_lambda_order_can_skip_bank_normalization(
            &long,
            DerefType::Never
        ));
        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &long,
                &short,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn kbo6_higher_order_lambda_order_non_bank_api_normalizes_lambda_applications() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let arg = typed_const(&mut bank, "kbo6_lambda_pending_arg");
        let applied = app(SIG_PHONY_APP_CODE, &[lambda, arg.clone()]);
        let mut ocb = lambda_ocb(bank.signature());

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &applied,
                &arg,
                DerefType::Never,
                DerefType::Never,
            ),
            CompareResult::Equal
        );
    }

    #[test]
    fn kbo6_higher_order_lambda_order_bank_api_normalizes_lambda_applications() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let arg = typed_const(&mut bank, "kbo6_lambda_bank_arg");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&arg))
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = lambda_ocb(bank.signature());

        assert_eq!(
            kbo6_compare_with_bank(
                &mut ocb,
                &mut bank,
                &applied,
                &arg,
                DerefType::Never,
                DerefType::Never,
            )
            .unwrap_or_else(|err| panic!("{err}")),
            CompareResult::Equal
        );
        assert!(!kbo6_greater_with_bank(
            &mut ocb,
            &mut bank,
            &applied,
            &arg,
            DerefType::Never,
            DerefType::Never,
        )
        .unwrap_or_else(|err| panic!("{err}")));
    }

    #[test]
    fn kbo6_lambda_order_support_check_rejects_dereferenced_lambda_surface() {
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let arg = typed_const(&mut bank, "kbo6_lambda_supported_arg");
        let applied = app(SIG_PHONY_APP_CODE, &[lambda.clone(), arg]);
        let variable = Term::const_cell_alloc(-4);
        variable.set_binding(Some(lambda));

        assert!(!kbo6_lambda_order_can_skip_bank_normalization(
            &variable,
            DerefType::Always
        ));
        variable.set_binding(Some(applied));
        assert!(kbo6_lambda_order_can_skip_bank_normalization(
            &variable,
            DerefType::Always
        ));
        assert!(kbo6_lambda_order_can_skip_bank_normalization(
            &variable,
            DerefType::Never
        ));
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
    fn kbo6_lfho_deref_always_weak_head_reduces_lambda_application() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let binder_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "kbo6_lfho_deref_a");
        let db0 = bank.request_db_var(&binder_type, 0);
        let lambda =
            close_with_db_var(&mut bank, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
        let applied = app(SIG_PHONY_APP_CODE, &[lambda, a.clone()]);
        applied.set_type(Some(binder_type));
        let mut ocb = ocb(bank.signature());

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &applied,
                &a,
                DerefType::Always,
                DerefType::Never,
            ),
            CompareResult::Equal
        );
    }

    #[test]
    fn kbo6_lfho_deref_once_expands_bound_applied_free_var() {
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

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &applied,
                &a,
                DerefType::Once,
                DerefType::Never,
            ),
            CompareResult::Greater
        );

        subst.backtrack();
    }

    #[test]
    fn kbo6_lfho_deref_once_skips_expanded_binding_prefix_arguments() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let function = symbol(bank.signature_mut(), "kbo6_lfho_applied_prefix_f", 0);
        let prefix_var = bank.vars().get_fresh_var(&type_);
        let suffix_var = bank.vars().get_fresh_var(&type_);
        let prefix_binding = typed_const(&mut bank, "kbo6_lfho_applied_prefix_b");
        let suffix_binding = typed_const(&mut bank, "kbo6_lfho_applied_prefix_c");
        let head_binding = app(function, std::slice::from_ref(&prefix_var));
        head_binding.set_type(Some(type_.clone()));
        let head_type = head_binding.type_().expect("binding must have a type");
        let head = bank.vars().get_fresh_var(&head_type);
        let applied = app(SIG_PHONY_APP_CODE, &[head.clone(), suffix_var.clone()]);
        applied.set_type(Some(type_));
        let expected = app(function, &[prefix_var.clone(), suffix_binding.clone()]);
        let mut subst = Substitution::new();
        subst.add_binding(&head, &head_binding);
        subst.add_binding(&prefix_var, &prefix_binding);
        subst.add_binding(&suffix_var, &suffix_binding);
        let mut ocb = ocb(bank.signature());

        assert_eq!(
            kbo6_compare(
                &mut ocb,
                bank.signature(),
                &applied,
                &expected,
                DerefType::Once,
                DerefType::Never,
            ),
            CompareResult::Equal
        );

        subst.backtrack();
    }
}
