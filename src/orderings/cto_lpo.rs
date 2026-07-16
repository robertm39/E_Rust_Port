//! Standard first-order lexicographic path ordering from `cto_lpo`.

use crate::basics::error::Diagnostic;
use crate::basics::partial_orderings::CompareResult;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::lambda::{beta_normalize_db, lambda_eta_reduce_db, whnf_deref, whnf_step};
use crate::terms::signature::Signature;
use crate::terms::simpletypes::types_cmp;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_copy_keep_vars, term_is_subterm, term_struct_equal_deref, term_struct_equal_no_deref,
};
use crate::terms::termtypes::{term_deref, DerefType, Term};
use std::sync::atomic::{AtomicI64, Ordering};

pub const DEFAULT_LPO_RECURSION_DEPTH_LIMIT: i64 = 1_000;

static LPO_RECURSION_DEPTH_LIMIT: AtomicI64 = AtomicI64::new(DEFAULT_LPO_RECURSION_DEPTH_LIMIT);

/// Return the process-wide LPO recursion depth limit.
#[must_use]
pub fn lpo_recursion_depth_limit() -> i64 {
    LPO_RECURSION_DEPTH_LIMIT.load(Ordering::Relaxed)
}

/// Set the process-wide LPO recursion depth limit.
///
/// # Panics
///
/// Panics if `limit` is negative. The command-line layer rejects zero too, but
/// the C global can technically be set to zero by internal callers.
pub fn set_lpo_recursion_depth_limit(limit: i64) {
    assert!(limit >= 0, "LPO recursion depth limit must be non-negative");
    LPO_RECURSION_DEPTH_LIMIT.store(limit, Ordering::Relaxed);
}

/// Return whether `s` is strictly greater than `t` in standard LPO.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo_compare`].
#[must_use]
pub fn lpo_greater(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    lpo_greater_with_limit(
        ocb,
        signature,
        s,
        t,
        deref_s,
        deref_t,
        lpo_recursion_depth_limit(),
    )
}

/// Return whether `s` is strictly greater than `t` in standard LPO, using an
/// explicit recursion limit.
///
/// # Panics
///
/// The optimized C build compiles out this legacy path's
/// `problemType != PROBLEM_HO` assertion and traverses higher-order cells as
/// ordinary symbols and arguments. Panics if term argument slots are
/// uninitialized or if the OCB lacks precedence storage.
#[must_use]
pub fn lpo_greater_with_limit(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    limit: i64,
) -> bool {
    LpoContext::new(ocb, signature, limit).greater_inner(s, t, deref_s, deref_t, 0)
        == CompareResult::Greater
}

/// Compare two terms in standard LPO.
///
/// This mirrors C `LPOCompare`: first test `s >= t`; if that returns the
/// internal "not greater-or-equal" result, test the reverse direction.
///
/// # Panics
///
/// Panics if a higher-order problem reaches this path with higher-order
/// ordering syntax, if term argument slots are uninitialized, or if the OCB
/// lacks precedence storage.
#[must_use]
pub fn lpo_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    lpo_compare_with_limit(
        ocb,
        signature,
        s,
        t,
        deref_s,
        deref_t,
        lpo_recursion_depth_limit(),
    )
}

/// Compare two terms in standard LPO, using an explicit recursion limit.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo_compare`].
#[must_use]
pub fn lpo_compare_with_limit(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    limit: i64,
) -> CompareResult {
    let context = LpoContext::new(ocb, signature, limit);
    let result = context.greater_inner(s, t, deref_s, deref_t, 0);
    match result {
        CompareResult::Greater
        | CompareResult::Lesser
        | CompareResult::Equal
        | CompareResult::Uncomparable => return result,
        CompareResult::NotGreaterEqual => {}
        CompareResult::Unknown | CompareResult::NotLessEqual => {
            panic!("unexpected one-sided LPO result: {result:?}")
        }
    }

    match context.greater_inner(t, s, deref_t, deref_s, 0) {
        CompareResult::Greater => CompareResult::Lesser,
        CompareResult::Uncomparable | CompareResult::NotGreaterEqual => CompareResult::Uncomparable,
        result => panic!("unexpected reverse LPO result: {result:?}"),
    }
}

/// Return whether `s` is strictly greater than `t` through the standard LPO
/// copy wrapper.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo_compare`], or if dereferencing an
/// applied variable is requested before higher-order term-bank support is
/// ported.
#[must_use]
pub fn lpo_greater_copy(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    let s_copy = copy_term_for_lpo_copy(s, deref_s);
    let t_copy = copy_term_for_lpo_copy(t, deref_t);
    let result = lpo_greater(
        ocb,
        signature,
        &s_copy,
        &t_copy,
        DerefType::Never,
        DerefType::Never,
    );
    debug_assert_eq!(result, lpo_greater(ocb, signature, s, t, deref_s, deref_t));
    result
}

/// Compare `s` and `t` through the standard LPO copy wrapper.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo_compare`], or if dereferencing an
/// applied variable is requested before higher-order term-bank support is
/// ported.
#[must_use]
pub fn lpo_compare_copy(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let s_copy = copy_term_for_lpo_copy(s, deref_s);
    let t_copy = copy_term_for_lpo_copy(t, deref_t);
    let result = lpo_compare(
        ocb,
        signature,
        &s_copy,
        &t_copy,
        DerefType::Never,
        DerefType::Never,
    );
    debug_assert_eq!(result, lpo_compare(ocb, signature, s, t, deref_s, deref_t));
    result
}

/// Return whether `s` is strictly greater than `t` in LPO4.
///
/// # Panics
///
/// Panics if term argument slots are uninitialized, if the OCB lacks
/// precedence storage, or if `DEREF_ONCE` over an applied variable would
/// require the bank-owned higher-order instantiation path.
#[must_use]
pub fn lpo4_greater(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    lpo4_greater_with_limit(
        ocb,
        signature,
        s,
        t,
        deref_s,
        deref_t,
        lpo_recursion_depth_limit(),
    )
}

/// Return whether `s` is strictly greater than `t` in LPO4, using an explicit
/// recursion limit.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo4_greater`].
#[must_use]
pub fn lpo4_greater_with_limit(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    limit: i64,
) -> bool {
    Lpo4Context::new(ocb, signature, limit).greater_inner(s, t, deref_s, deref_t, 0)
}

/// Compare `s` and `t` in LPO4.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo4_greater`].
#[must_use]
pub fn lpo4_compare(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    lpo4_compare_with_limit(
        ocb,
        signature,
        s,
        t,
        deref_s,
        deref_t,
        lpo_recursion_depth_limit(),
    )
}

/// Compare `s` and `t` in LPO4, using an explicit recursion limit.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo4_compare`].
#[must_use]
pub fn lpo4_compare_with_limit(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
    limit: i64,
) -> CompareResult {
    assert_lpo4_deref_once_ready(s, deref_s);
    assert_lpo4_deref_once_ready(t, deref_t);
    if term_struct_equal_deref(s, t, deref_s, deref_t) {
        return CompareResult::Equal;
    }

    let context = Lpo4Context::new(ocb, signature, limit);
    if context.greater_inner(s, t, deref_s, deref_t, 0) {
        CompareResult::Greater
    } else if context.greater_inner(t, s, deref_t, deref_s, 0) {
        CompareResult::Lesser
    } else {
        CompareResult::Uncomparable
    }
}

/// Return whether `s` is strictly greater than `t` in LPO4 using the
/// owner-bank higher-order normalization path.
///
/// # Errors
///
/// Returns a diagnostic if instantiated insertion, weak-head normalization, or
/// beta/eta normalization fails.
///
/// # Panics
///
/// Panics under the same structural invariants as [`lpo4_greater`].
pub fn lpo4_greater_with_bank(
    ocb: &OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<bool, Diagnostic> {
    let (s, deref_s) = adjust_lpo4_ho_deref(bank, s, deref_s)?;
    let (t, deref_t) = adjust_lpo4_ho_deref(bank, t, deref_t)?;
    Lpo4BankContext::new(ocb, bank, lpo_recursion_depth_limit())
        .greater_inner(&s, &t, deref_s, deref_t, 0)
}

/// Compare `s` and `t` in LPO4 using the owner-bank higher-order
/// normalization path.
///
/// # Errors
///
/// Returns a diagnostic if instantiated insertion, weak-head normalization, or
/// beta/eta normalization fails.
///
/// # Panics
///
/// Panics under the same structural invariants as [`lpo4_compare`].
pub fn lpo4_compare_with_bank(
    ocb: &OrderControlBlock,
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> Result<CompareResult, Diagnostic> {
    let (s, deref_s) = adjust_lpo4_ho_deref(bank, s, deref_s)?;
    let (t, deref_t) = adjust_lpo4_ho_deref(bank, t, deref_t)?;
    if term_struct_equal_deref(&s, &t, deref_s, deref_t) {
        return Ok(CompareResult::Equal);
    }

    let mut context = Lpo4BankContext::new(ocb, bank, lpo_recursion_depth_limit());
    if context.greater_inner(&s, &t, deref_s, deref_t, 0)? {
        Ok(CompareResult::Greater)
    } else if context.greater_inner(&t, &s, deref_t, deref_s, 0)? {
        Ok(CompareResult::Lesser)
    } else {
        Ok(CompareResult::Uncomparable)
    }
}

/// Return whether `s` is strictly greater than `t` through the LPO4 copy
/// wrapper.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo4_greater`], or if dereferencing an
/// applied variable is requested before higher-order term-bank support is
/// ported.
#[must_use]
pub fn lpo4_greater_copy(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> bool {
    let s_copy = copy_term_for_lpo_copy(s, deref_s);
    let t_copy = copy_term_for_lpo_copy(t, deref_t);
    let result = Lpo4CopyContext::new(ocb, signature).greater_inner(&s_copy, &t_copy);
    debug_assert_eq!(result, lpo_greater(ocb, signature, s, t, deref_s, deref_t));
    result
}

/// Compare `s` and `t` through the LPO4 copy wrapper.
///
/// # Panics
///
/// Panics under the same invariants as [`lpo4_greater_copy`].
#[must_use]
pub fn lpo4_compare_copy(
    ocb: &OrderControlBlock,
    signature: &Signature,
    s: &Term,
    t: &Term,
    deref_s: DerefType,
    deref_t: DerefType,
) -> CompareResult {
    let s_copy = copy_term_for_lpo_copy(s, deref_s);
    let t_copy = copy_term_for_lpo_copy(t, deref_t);
    let context = Lpo4CopyContext::new(ocb, signature);
    let result = if term_struct_equal_no_deref(&s_copy, &t_copy) {
        CompareResult::Equal
    } else if context.greater_inner(&s_copy, &t_copy) {
        CompareResult::Greater
    } else if context.greater_inner(&t_copy, &s_copy) {
        CompareResult::Lesser
    } else {
        CompareResult::Uncomparable
    };
    debug_assert_eq!(result, lpo_compare(ocb, signature, s, t, deref_s, deref_t));
    result
}

fn copy_term_for_lpo_copy(term: &Term, deref: DerefType) -> Term {
    if deref == DerefType::Never {
        term.clone()
    } else {
        term_copy_keep_vars(term, deref)
    }
}

struct LpoContext<'a> {
    ocb: &'a OrderControlBlock,
    signature: &'a Signature,
    limit: i64,
}

impl<'a> LpoContext<'a> {
    const fn new(ocb: &'a OrderControlBlock, signature: &'a Signature, limit: i64) -> Self {
        Self {
            ocb,
            signature,
            limit,
        }
    }

    fn term_dominates_args(
        &self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> bool {
        t.argument_clones().into_iter().flatten().all(|arg| {
            self.greater_inner(s, &arg, deref_s, deref_t, depth) == CompareResult::Greater
        })
    }

    fn subterm_dominates_term(
        &self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> bool {
        s.argument_clones().into_iter().flatten().any(|arg| {
            matches!(
                self.greater_inner(&arg, t, deref_s, deref_t, depth),
                CompareResult::Greater | CompareResult::Equal
            )
        })
    }

    fn lex_greater(
        &self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> CompareResult {
        let mut result = CompareResult::Equal;
        for index in 0..s.arity().min(t.arity()) {
            result = self.greater_inner(
                &initialized_arg(s, index),
                &initialized_arg(t, index),
                deref_s,
                deref_t,
                depth,
            );
            if result != CompareResult::Equal {
                break;
            }
        }

        if result == CompareResult::Equal {
            match s.arity().cmp(&t.arity()) {
                std::cmp::Ordering::Greater => CompareResult::Greater,
                std::cmp::Ordering::Less => CompareResult::NotGreaterEqual,
                std::cmp::Ordering::Equal => CompareResult::Equal,
            }
        } else {
            result
        }
    }

    fn greater_inner(
        &self,
        s: &Term,
        t: &Term,
        mut deref_s: DerefType,
        mut deref_t: DerefType,
        depth: i64,
    ) -> CompareResult {
        let s = term_deref(s, &mut deref_s);
        let t = term_deref(t, &mut deref_t);
        if depth > self.limit {
            return CompareResult::Uncomparable;
        }

        let child_depth = depth + 1;
        if s.is_free_var() {
            if s == t {
                CompareResult::Equal
            } else if t.is_free_var() {
                CompareResult::Uncomparable
            } else {
                CompareResult::NotGreaterEqual
            }
        } else if t.is_free_var() {
            if term_is_subterm(&s, &t, deref_s) {
                CompareResult::Greater
            } else {
                CompareResult::Uncomparable
            }
        } else {
            self.non_variable_compare(&s, &t, deref_s, deref_t, child_depth)
        }
    }

    fn non_variable_compare(
        &self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        child_depth: i64,
    ) -> CompareResult {
        match self.ocb.fun_compare(self.signature, s.f_code(), t.f_code()) {
            CompareResult::Greater => {
                if self.term_dominates_args(s, t, deref_s, deref_t, child_depth) {
                    CompareResult::Greater
                } else {
                    CompareResult::NotGreaterEqual
                }
            }
            CompareResult::Equal => self.equal_head_compare(s, t, deref_s, deref_t, child_depth),
            CompareResult::Lesser | CompareResult::Uncomparable => {
                if self.subterm_dominates_term(s, t, deref_s, deref_t, child_depth) {
                    CompareResult::Greater
                } else {
                    CompareResult::NotGreaterEqual
                }
            }
            result => panic!("unexpected function-symbol comparison in LPO: {result:?}"),
        }
    }

    fn equal_head_compare(
        &self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        child_depth: i64,
    ) -> CompareResult {
        let mut result = match self.lex_greater(s, t, deref_s, deref_t, child_depth) {
            CompareResult::Greater
                if self.term_dominates_args(s, t, deref_s, deref_t, child_depth) =>
            {
                CompareResult::Greater
            }
            CompareResult::Equal => CompareResult::Equal,
            _ => CompareResult::NotGreaterEqual,
        };

        if result == CompareResult::NotGreaterEqual
            && s.arity() >= 2
            && self.subterm_dominates_term(s, t, deref_s, deref_t, child_depth)
        {
            result = CompareResult::Greater;
        }
        result
    }
}

struct Lpo4Context<'a> {
    ocb: &'a OrderControlBlock,
    signature: &'a Signature,
    limit: i64,
}

impl<'a> Lpo4Context<'a> {
    const fn new(ocb: &'a OrderControlBlock, signature: &'a Signature, limit: i64) -> Self {
        Self {
            ocb,
            signature,
            limit,
        }
    }

    fn alpha(
        &self,
        s: &Term,
        pos: usize,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> bool {
        let offset = lpo4_argument_offset(s);
        (pos + offset..s.arity()).any(|index| {
            let arg = initialized_arg(s, index);
            term_struct_equal_deref(&arg, t, deref_s, deref_t)
                || self.greater_inner(&arg, t, deref_s, deref_t, depth)
        })
    }

    fn majo(
        &self,
        s: &Term,
        t: &Term,
        pos: usize,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> bool {
        let start = pos + lpo4_argument_offset(t);
        (start..t.arity())
            .all(|index| self.greater_inner(s, &initialized_arg(t, index), deref_s, deref_t, depth))
    }

    fn lex_ma(
        &self,
        s: &Term,
        t: &Term,
        mut pos: usize,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> bool {
        assert_eq!(s.f_code(), t.f_code(), "LPO4 lex_ma requires equal heads");
        let s_offset = lpo4_argument_offset(s);
        let t_offset = lpo4_argument_offset(t);
        while pos + s_offset < s.arity() {
            if pos + t_offset >= t.arity() {
                return true;
            }
            let s_arg = initialized_arg(s, pos + s_offset);
            let t_arg = initialized_arg(t, pos + t_offset);
            if term_struct_equal_deref(&s_arg, &t_arg, deref_s, deref_t) {
                pos += 1;
                continue;
            }
            return if self.greater_inner(&s_arg, &t_arg, deref_s, deref_t, depth) {
                self.majo(s, t, pos + 1, deref_s, deref_t, depth)
            } else {
                self.alpha(s, pos + 1, t, deref_s, deref_t, depth)
            };
        }
        false
    }

    fn greater_inner(
        &self,
        s: &Term,
        t: &Term,
        mut deref_s: DerefType,
        mut deref_t: DerefType,
        depth: i64,
    ) -> bool {
        assert_lpo4_deref_once_ready(s, deref_s);
        assert_lpo4_deref_once_ready(t, deref_t);
        if depth > self.limit {
            return false;
        }
        let s = term_deref(s, &mut deref_s);
        let t = term_deref(t, &mut deref_t);
        let child_depth = depth + 1;

        if s.is_top_level_free_var() {
            false
        } else if t.is_top_level_free_var() {
            lpo4_subterm(&s, &t, deref_s)
        } else {
            match self.head_compare(&s, &t) {
                CompareResult::Greater => self.majo(&s, &t, 0, deref_s, deref_t, child_depth),
                CompareResult::Equal => self.lex_ma(&s, &t, 0, deref_s, deref_t, child_depth),
                CompareResult::Lesser | CompareResult::Uncomparable => {
                    self.alpha(&s, 0, &t, deref_s, deref_t, child_depth)
                }
                result => panic!("unexpected function-symbol comparison in LPO4: {result:?}"),
            }
        }
    }

    fn head_compare(&self, s: &Term, t: &Term) -> CompareResult {
        let s_class = lpo4_head_class(self.signature, s);
        let t_class = lpo4_head_class(self.signature, t);
        if s_class != t_class {
            return if s_class > t_class {
                CompareResult::Greater
            } else {
                CompareResult::Lesser
            };
        }

        let s_head = lpo4_head(s);
        let t_head = lpo4_head(t);
        if s_head.is_db_var() {
            return match s_head.f_code().cmp(&t_head.f_code()) {
                std::cmp::Ordering::Greater => CompareResult::Greater,
                std::cmp::Ordering::Less => CompareResult::Lesser,
                std::cmp::Ordering::Equal => CompareResult::Equal,
            };
        }

        if s.is_lambda() {
            let s_binder = initialized_arg(s, 0);
            let t_binder = initialized_arg(t, 0);
            if s_binder == t_binder {
                CompareResult::Equal
            } else {
                let s_type = s_binder.type_().expect("lambda binder must have a type");
                let t_type = t_binder.type_().expect("lambda binder must have a type");
                if types_cmp(&s_type, &t_type) > 0 {
                    CompareResult::Greater
                } else {
                    CompareResult::Lesser
                }
            }
        } else {
            assert!(s.f_code() > 0, "LPO4 symbol head requires positive f-code");
            assert!(t.f_code() > 0, "LPO4 symbol head requires positive f-code");
            self.ocb.fun_compare(self.signature, s.f_code(), t.f_code())
        }
    }
}

struct Lpo4BankContext<'a> {
    ocb: &'a OrderControlBlock,
    bank: &'a mut TermBank,
    limit: i64,
}

impl<'a> Lpo4BankContext<'a> {
    fn new(ocb: &'a OrderControlBlock, bank: &'a mut TermBank, limit: i64) -> Self {
        Self { ocb, bank, limit }
    }

    fn alpha(
        &mut self,
        s: &Term,
        pos: usize,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> Result<bool, Diagnostic> {
        let offset = lpo4_argument_offset(s);
        for index in pos + offset..s.arity() {
            let arg = initialized_arg(s, index);
            if term_struct_equal_deref(&arg, t, deref_s, deref_t)
                || self.greater_inner(&arg, t, deref_s, deref_t, depth)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn majo(
        &mut self,
        s: &Term,
        t: &Term,
        pos: usize,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> Result<bool, Diagnostic> {
        let start = pos + lpo4_argument_offset(t);
        for index in start..t.arity() {
            if !self.greater_inner(s, &initialized_arg(t, index), deref_s, deref_t, depth)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn lex_ma(
        &mut self,
        s: &Term,
        t: &Term,
        mut pos: usize,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> Result<bool, Diagnostic> {
        assert_eq!(s.f_code(), t.f_code(), "LPO4 lex_ma requires equal heads");
        let s_offset = lpo4_argument_offset(s);
        let t_offset = lpo4_argument_offset(t);
        while pos + s_offset < s.arity() {
            if pos + t_offset >= t.arity() {
                return Ok(true);
            }
            let s_arg = initialized_arg(s, pos + s_offset);
            let t_arg = initialized_arg(t, pos + t_offset);
            if term_struct_equal_deref(&s_arg, &t_arg, deref_s, deref_t) {
                pos += 1;
                continue;
            }
            return if self.greater_inner(&s_arg, &t_arg, deref_s, deref_t, depth)? {
                self.majo(s, t, pos + 1, deref_s, deref_t, depth)
            } else {
                self.alpha(s, pos + 1, t, deref_s, deref_t, depth)
            };
        }
        Ok(false)
    }

    fn greater_inner(
        &mut self,
        s: &Term,
        t: &Term,
        deref_s: DerefType,
        deref_t: DerefType,
        depth: i64,
    ) -> Result<bool, Diagnostic> {
        if depth > self.limit {
            return Ok(false);
        }

        let s = self.normalize_term(s, deref_s)?;
        let mut t = self.normalize_term(t, deref_t)?;
        let child_depth = depth + 1;

        if s.is_top_level_free_var() {
            Ok(false)
        } else if t.is_top_level_free_var() {
            if t.is_applied_free_var() && deref_t == DerefType::Always {
                t = self.bank.insert_instantiated_deref(&t, deref_t)?;
            }
            self.ho_subterm(&s, &t, deref_s)
        } else {
            match self.head_compare(&s, &t) {
                CompareResult::Greater => self.majo(&s, &t, 0, deref_s, deref_t, child_depth),
                CompareResult::Equal => self.lex_ma(&s, &t, 0, deref_s, deref_t, child_depth),
                CompareResult::Lesser | CompareResult::Uncomparable => {
                    self.alpha(&s, 0, &t, deref_s, deref_t, child_depth)
                }
                result => panic!("unexpected function-symbol comparison in LPO4: {result:?}"),
            }
        }
    }

    fn normalize_term(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        let normalized = match deref {
            DerefType::Never => term.clone(),
            DerefType::Always => whnf_deref(self.bank, term)?,
            DerefType::Once => {
                let mut current_deref = deref;
                let derefed = term_deref(term, &mut current_deref);
                whnf_step(self.bank, &derefed)?
            }
        };
        lambda_eta_reduce_db(self.bank, &normalized)
    }

    fn ho_subterm(
        &mut self,
        super_term: &Term,
        test: &Term,
        mut super_deref: DerefType,
    ) -> Result<bool, Diagnostic> {
        let derefed = term_deref(super_term, &mut super_deref);
        let beta = beta_normalize_db(self.bank, &derefed)?;
        let mut normalized = lambda_eta_reduce_db(self.bank, &beta)?;

        if normalized.is_applied_free_var() {
            assert!(
                super_deref == DerefType::Never || super_deref == DerefType::Always,
                "applied-variable LPO4 subterm checks expect no one-step dereference"
            );
            if super_deref == DerefType::Always {
                let inserted = self.bank.insert(&normalized, super_deref)?;
                let beta = beta_normalize_db(self.bank, &inserted)?;
                normalized = lambda_eta_reduce_db(self.bank, &beta)?;
            }
            return Ok(normalized == *test);
        }

        if normalized == *test {
            return Ok(true);
        }

        for index in usize::from(normalized.is_lambda())..normalized.arity() {
            if self.ho_subterm(&initialized_arg(&normalized, index), test, super_deref)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn head_compare(&self, s: &Term, t: &Term) -> CompareResult {
        let signature = self.bank.signature();
        let s_class = lpo4_head_class(signature, s);
        let t_class = lpo4_head_class(signature, t);
        if s_class != t_class {
            return if s_class > t_class {
                CompareResult::Greater
            } else {
                CompareResult::Lesser
            };
        }

        let s_head = lpo4_head(s);
        let t_head = lpo4_head(t);
        if s_head.is_db_var() {
            return match s_head.f_code().cmp(&t_head.f_code()) {
                std::cmp::Ordering::Greater => CompareResult::Greater,
                std::cmp::Ordering::Less => CompareResult::Lesser,
                std::cmp::Ordering::Equal => CompareResult::Equal,
            };
        }

        if s.is_lambda() {
            let s_binder = initialized_arg(s, 0);
            let t_binder = initialized_arg(t, 0);
            if s_binder == t_binder {
                CompareResult::Equal
            } else {
                let s_type = s_binder.type_().expect("lambda binder must have a type");
                let t_type = t_binder.type_().expect("lambda binder must have a type");
                if types_cmp(&s_type, &t_type) > 0 {
                    CompareResult::Greater
                } else {
                    CompareResult::Lesser
                }
            }
        } else {
            assert!(s.f_code() > 0, "LPO4 symbol head requires positive f-code");
            assert!(t.f_code() > 0, "LPO4 symbol head requires positive f-code");
            self.ocb.fun_compare(signature, s.f_code(), t.f_code())
        }
    }
}

struct Lpo4CopyContext<'a> {
    ocb: &'a OrderControlBlock,
    signature: &'a Signature,
}

impl<'a> Lpo4CopyContext<'a> {
    const fn new(ocb: &'a OrderControlBlock, signature: &'a Signature) -> Self {
        Self { ocb, signature }
    }

    fn alpha(&self, s: &Term, pos: usize, t: &Term) -> bool {
        (pos..s.arity()).any(|index| {
            let arg = initialized_arg(s, index);
            term_struct_equal_no_deref(&arg, t) || self.greater_inner(&arg, t)
        })
    }

    fn majo(&self, s: &Term, t: &Term, pos: usize) -> bool {
        (pos..t.arity()).all(|index| self.greater_inner(s, &initialized_arg(t, index)))
    }

    fn lex_ma(&self, s: &Term, t: &Term, mut pos: usize) -> bool {
        assert_eq!(
            s.f_code(),
            t.f_code(),
            "LPO4 copy lex_ma requires equal heads"
        );
        while pos < s.arity() {
            let s_arg = initialized_arg(s, pos);
            let t_arg = initialized_arg(t, pos);
            if term_struct_equal_no_deref(&s_arg, &t_arg) {
                pos += 1;
                continue;
            }
            return if self.greater_inner(&s_arg, &t_arg) {
                self.majo(s, t, pos + 1)
            } else {
                self.alpha(s, pos + 1, t)
            };
        }
        false
    }

    fn greater_inner(&self, s: &Term, t: &Term) -> bool {
        if s.is_free_var() {
            return false;
        }
        if t.is_free_var() {
            return term_is_subterm(s, t, DerefType::Never);
        }

        match self.ocb.fun_compare(self.signature, s.f_code(), t.f_code()) {
            CompareResult::Greater => self.majo(s, t, 0),
            CompareResult::Equal => self.lex_ma(s, t, 0),
            CompareResult::Lesser | CompareResult::Uncomparable => self.alpha(s, 0, t),
            result => panic!("unexpected function-symbol comparison in LPO4 copy: {result:?}"),
        }
    }
}

fn lpo4_argument_offset(term: &Term) -> usize {
    usize::from(term.is_lambda() || term.is_phony_app())
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Lpo4HeadClass {
    Symbol = 1,
    Quantifier = 2,
    Db = 3,
    Lambda = 4,
}

fn lpo4_head(term: &Term) -> Term {
    if term.is_phony_app() {
        initialized_arg(term, 0)
    } else {
        term.clone()
    }
}

fn lpo4_head_class(signature: &Signature, term: &Term) -> Lpo4HeadClass {
    let head = lpo4_head(term);
    if head.is_lambda() {
        Lpo4HeadClass::Lambda
    } else if head.is_db_var() {
        Lpo4HeadClass::Db
    } else if head.f_code() == signature.qex_code() || head.f_code() == signature.qall_code() {
        Lpo4HeadClass::Quantifier
    } else {
        Lpo4HeadClass::Symbol
    }
}

fn lpo4_subterm(super_term: &Term, test: &Term, mut deref: DerefType) -> bool {
    let super_term = term_deref(super_term, &mut deref);
    if super_term.is_applied_free_var() {
        return super_term == *test;
    }
    if super_term == *test {
        return true;
    }
    (usize::from(super_term.is_lambda())..super_term.arity())
        .any(|index| lpo4_subterm(&initialized_arg(&super_term, index), test, deref))
}

fn adjust_lpo4_ho_deref(
    bank: &mut TermBank,
    term: &Term,
    deref: DerefType,
) -> Result<(Term, DerefType), Diagnostic> {
    if deref == DerefType::Once && term.has_app_var() {
        Ok((bank.insert_instantiated(term)?, DerefType::Never))
    } else {
        Ok((term.clone(), deref))
    }
}

fn assert_lpo4_deref_once_ready(term: &Term, deref: DerefType) {
    assert!(
        deref != DerefType::Once || !term.has_app_var(),
        "LPO4 DEREF_ONCE over applied variables needs higher-order instantiation support"
    );
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{
        lpo4_compare, lpo4_compare_copy, lpo4_compare_with_bank, lpo4_greater, lpo4_greater_copy,
        lpo4_greater_with_bank, lpo_compare, lpo_compare_copy, lpo_compare_with_limit, lpo_greater,
        lpo_greater_copy, lpo_recursion_depth_limit, set_lpo_recursion_depth_limit,
        DEFAULT_LPO_RECURSION_DEPTH_LIMIT,
    };
    use crate::basics::partial_orderings::{CompareResult, HoOrderKind};
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::dbvars::mk_db;
    use crate::terms::functypes::FunCode;
    use crate::terms::lambda::{apply_terms, close_with_db_var};
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
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
        OrderControlBlock::alloc(TermOrdering::Lpo, true, signature, HoOrderKind::LfhoOrder)
    }

    fn lpo4_ocb(signature: &Signature) -> OrderControlBlock {
        OrderControlBlock::alloc(TermOrdering::Lpo4, true, signature, HoOrderKind::LfhoOrder)
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

    fn db_lambda(binder: &Term, body: &Term) -> Term {
        let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 2);
        lambda.set_argument(0, binder.clone());
        lambda.set_argument(1, body.clone());
        lambda
    }

    fn phony_app(head: &Term, args: &[Term]) -> Term {
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, args.len() + 1);
        term.set_argument(0, head.clone());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index + 1, arg.clone());
        }
        term
    }

    #[test]
    fn lpo_orders_by_precedence_and_subterm_cases() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(a, 20);
        ocb.set_fun_prec_weight(f, 10);
        let a_term = Term::const_cell_alloc(a);
        let f_a = app(f, std::slice::from_ref(&a_term));

        assert_eq!(
            lpo_compare(
                &ocb,
                &signature,
                &f_a,
                &a_term,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(lpo_greater(
            &ocb,
            &signature,
            &f_a,
            &a_term,
            DerefType::Never,
            DerefType::Never
        ));
    }

    #[test]
    fn lpo_equal_heads_use_lexicographic_argument_order() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let b = symbol(&mut signature, "b", 0);
        let c = symbol(&mut signature, "c", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(f, 40);
        ocb.set_fun_prec_weight(b, 30);
        ocb.set_fun_prec_weight(c, 20);
        let b_term = Term::const_cell_alloc(b);
        let c_term = Term::const_cell_alloc(c);
        let f_b = app(f, std::slice::from_ref(&b_term));
        let f_c = app(f, std::slice::from_ref(&c_term));

        assert_eq!(
            lpo_compare(
                &ocb,
                &signature,
                &f_b,
                &f_c,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn lpo_variable_cases_use_identity_subterms() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let ocb = ocb(&signature);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let f_x = app(f, std::slice::from_ref(&x));

        assert_eq!(
            lpo_compare(
                &ocb,
                &signature,
                &f_x,
                &x,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            lpo_compare(
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
            lpo_compare(&ocb, &signature, &x, &y, DerefType::Never, DerefType::Never),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn lpo_recursion_limit_can_block_subterm_proof() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = ocb(&signature);
        ocb.set_fun_prec_weight(a, 20);
        ocb.set_fun_prec_weight(f, 10);
        let a_term = Term::const_cell_alloc(a);
        let f_a = app(f, std::slice::from_ref(&a_term));

        assert_eq!(
            lpo_compare_with_limit(
                &ocb,
                &signature,
                &f_a,
                &a_term,
                DerefType::Never,
                DerefType::Never,
                0,
            ),
            CompareResult::Uncomparable
        );
    }

    #[test]
    fn lpo_recursion_limit_global_matches_c_default() {
        let _guard = global_state_lock();
        assert_eq!(DEFAULT_LPO_RECURSION_DEPTH_LIMIT, 1_000);
        let old = lpo_recursion_depth_limit();
        set_lpo_recursion_depth_limit(7);
        assert_eq!(lpo_recursion_depth_limit(), 7);
        set_lpo_recursion_depth_limit(old);
    }

    #[test]
    fn lpo4_orders_first_order_precedence_lex_and_subterm_cases() {
        let mut signature = signature();
        let f = symbol(&mut signature, "f", 1);
        let g = symbol(&mut signature, "g", 1);
        let b = symbol(&mut signature, "b", 0);
        let a = symbol(&mut signature, "a", 0);
        let mut ocb = lpo4_ocb(&signature);
        ocb.set_fun_prec_weight(f, 40);
        ocb.set_fun_prec_weight(g, 30);
        ocb.set_fun_prec_weight(b, 20);
        ocb.set_fun_prec_weight(a, 10);

        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let f_a = app(f, std::slice::from_ref(&a_term));
        let f_b = app(f, std::slice::from_ref(&b_term));
        let g_a = app(g, std::slice::from_ref(&a_term));

        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &f_b,
                &g_a,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &f_b,
                &f_a,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(lpo4_greater(
            &ocb,
            &signature,
            &f_a,
            &a_term,
            DerefType::Never,
            DerefType::Never
        ));
        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &a_term,
                &f_a,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Lesser
        );
    }

    #[test]
    fn lpo4_orders_visible_lfho_head_classes() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut signature = signature();
        let a = symbol(&mut signature, "lpo4_ho_a", 0);
        let i_type = signature.type_bank().i_type();
        let bool_type = signature.type_bank().bool_type();
        let predicate_type = signature
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), bool_type]));
        let db = mk_db(0, &i_type);
        let body = Term::const_cell_alloc(a);
        let lambda = db_lambda(&db, &body);
        let quantified = app(signature.qall_code(), std::slice::from_ref(&lambda));
        let symbol_term = Term::const_cell_alloc(a);
        let mut ocb = lpo4_ocb(&signature);
        ocb.set_fun_prec_weight(a, 10);

        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &lambda,
                &db,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &db,
                &symbol_term,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &quantified,
                &symbol_term,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert_eq!(lambda.argument(0).unwrap().type_(), Some(i_type));
        assert!(predicate_type.is_arrow());
    }

    #[test]
    fn lpo4_phony_app_uses_head_class_but_skips_hidden_head_in_arguments() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut signature = signature();
        let a = symbol(&mut signature, "lpo4_ho_arg_a", 0);
        let b = symbol(&mut signature, "lpo4_ho_arg_b", 0);
        let i_type = signature.type_bank().i_type();
        let db_head = mk_db(1, &i_type);
        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let app_a = phony_app(&db_head, std::slice::from_ref(&a_term));
        let app_b = phony_app(&db_head, std::slice::from_ref(&b_term));
        let mut ocb = lpo4_ocb(&signature);
        ocb.set_fun_prec_weight(b, 20);
        ocb.set_fun_prec_weight(a, 10);

        assert_eq!(
            lpo4_compare(
                &ocb,
                &signature,
                &app_b,
                &app_a,
                DerefType::Never,
                DerefType::Never
            ),
            CompareResult::Greater
        );
    }

    #[test]
    fn lpo4_with_bank_deref_always_weak_head_normalizes_before_comparison() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "lpo4_bank_beta_a");
        let b = typed_const(&mut bank, "lpo4_bank_beta_b");
        let lambda = close_with_db_var(&mut bank, &type_, &a).unwrap_or_else(|err| panic!("{err}"));
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&b))
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = lpo4_ocb(bank.signature());
        ocb.set_fun_prec_weight(b.f_code(), 30);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            lpo4_compare_with_bank(
                &ocb,
                &mut bank,
                &applied,
                &b,
                DerefType::Always,
                DerefType::Never
            )
            .unwrap_or_else(|err| panic!("{err}")),
            CompareResult::Lesser
        );
        assert!(!lpo4_greater_with_bank(
            &ocb,
            &mut bank,
            &applied,
            &b,
            DerefType::Always,
            DerefType::Never
        )
        .unwrap_or_else(|err| panic!("{err}")));
    }

    #[test]
    fn lpo4_with_bank_deref_once_instantiates_bound_applied_free_var() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let head_binding = typed_unary_const(&mut bank, "lpo4_bank_applied_binding");
        let head_type = head_binding.type_().expect("binding must have a type");
        let head = bank.vars().get_fresh_var(&head_type);
        let a = typed_const(&mut bank, "lpo4_bank_applied_arg");
        let applied = phony_app(&head, std::slice::from_ref(&a));
        applied.set_type(Some(type_));
        let mut subst = Substitution::new();
        subst.add_binding(&head, &head_binding);
        let mut ocb = lpo4_ocb(bank.signature());
        ocb.set_fun_prec_weight(head_binding.f_code(), 20);
        ocb.set_fun_prec_weight(a.f_code(), 10);

        assert_eq!(
            lpo4_compare_with_bank(
                &ocb,
                &mut bank,
                &applied,
                &a,
                DerefType::Once,
                DerefType::Never
            )
            .unwrap_or_else(|err| panic!("{err}")),
            CompareResult::Greater
        );

        subst.backtrack();
    }

    #[test]
    fn lpo_copy_wrappers_apply_deref_by_copying_terms() {
        let mut signature = signature();
        let b = symbol(&mut signature, "b", 0);
        let a = symbol(&mut signature, "a", 0);
        let mut standard = ocb(&signature);
        let mut lpo4 = lpo4_ocb(&signature);
        standard.set_fun_prec_weight(b, 20);
        standard.set_fun_prec_weight(a, 10);
        lpo4.set_fun_prec_weight(b, 20);
        lpo4.set_fun_prec_weight(a, 10);

        let a_term = Term::const_cell_alloc(a);
        let b_term = Term::const_cell_alloc(b);
        let x = Term::const_cell_alloc(-2);
        x.set_binding(Some(b_term));

        assert_eq!(
            lpo_compare_copy(
                &standard,
                &signature,
                &x,
                &a_term,
                DerefType::Once,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(lpo_greater_copy(
            &standard,
            &signature,
            &x,
            &a_term,
            DerefType::Once,
            DerefType::Never
        ));
        assert_eq!(
            lpo4_compare_copy(
                &lpo4,
                &signature,
                &x,
                &a_term,
                DerefType::Once,
                DerefType::Never
            ),
            CompareResult::Greater
        );
        assert!(lpo4_greater_copy(
            &lpo4,
            &signature,
            &x,
            &a_term,
            DerefType::Once,
            DerefType::Never
        ));
    }
}
