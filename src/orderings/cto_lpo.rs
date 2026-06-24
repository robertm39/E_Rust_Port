//! Standard first-order lexicographic path ordering from `cto_lpo`.

use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termfunc::term_is_subterm;
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
/// Panics if the global problem type is higher-order, if term argument slots
/// are uninitialized, or if the OCB lacks precedence storage.
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
    assert_first_order_lpo();
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
/// Panics if the global problem type is higher-order, if term argument slots
/// are uninitialized, or if the OCB lacks precedence storage.
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
    assert_first_order_lpo();
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

fn assert_first_order_lpo() {
    assert_ne!(
        problem_type(),
        ProblemType::HigherOrder,
        "standard LPO path is not used for higher-order problems"
    );
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

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{
        lpo_compare, lpo_compare_with_limit, lpo_greater, lpo_recursion_depth_limit,
        set_lpo_recursion_depth_limit, DEFAULT_LPO_RECURSION_DEPTH_LIMIT,
    };
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
        assert_eq!(DEFAULT_LPO_RECURSION_DEPTH_LIMIT, 1_000);
        let old = lpo_recursion_depth_limit();
        set_lpo_recursion_depth_limit(7);
        assert_eq!(lpo_recursion_depth_limit(), 7);
        set_lpo_recursion_depth_limit(old);
    }
}
