use std::collections::VecDeque;

#[cfg(feature = "measure-unification")]
use std::sync::atomic::{AtomicI64, Ordering};

use crate::basics::error::Diagnostic;
use crate::basics::pqueue::PQueue;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::lambda::{lambda_eta_reduce_db, whnf_deref};
use crate::terms::pattern_match_mgu::{
    prune_lambda_prefix, subst_compute_match_pattern, subst_compute_mgu_pattern,
};
use crate::terms::signature::{SIG_ITE_CODE, SIG_LET_CODE, SIG_PHONY_APP_CODE};
use crate::terms::simpletypes::{type_drop_first_arg, Type};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_is_db_closed, term_standard_weight, term_struct_equal_deref, term_struct_prefix_equal,
};
use crate::terms::termtypes::{
    term_deref, term_deref_owned, term_is_prefix, DerefType, Term, DEFAULT_VWEIGHT, TP_PRED_POS,
};

#[cfg(feature = "measure-unification")]
static UNIFICATION_ATTEMPTS: AtomicI64 = AtomicI64::new(0);
#[cfg(feature = "measure-unification")]
static UNIFICATION_SUCCESSES: AtomicI64 = AtomicI64::new(0);

pub const MATCH_FAILED: i32 = -1;

const INLINE_MATCH_JOB_PAIRS: usize = 4;

struct MatchJobStack {
    inline: [Option<(Term, Term)>; INLINE_MATCH_JOB_PAIRS],
    inline_len: usize,
    overflow: Vec<(Term, Term)>,
}

impl MatchJobStack {
    fn new(left: Term, right: Term) -> Self {
        let mut stack = Self {
            inline: std::array::from_fn(|_| None),
            inline_len: 0,
            overflow: Vec::new(),
        };
        stack.push(left, right);
        stack
    }

    fn push(&mut self, left: Term, right: Term) {
        if self.overflow.is_empty() && self.inline_len < INLINE_MATCH_JOB_PAIRS {
            self.inline[self.inline_len] = Some((left, right));
            self.inline_len += 1;
        } else {
            self.overflow.push((left, right));
        }
    }

    fn pop(&mut self) -> Option<(Term, Term)> {
        if let Some(pair) = self.overflow.pop() {
            return Some(pair);
        }
        self.inline_len = self.inline_len.checked_sub(1)?;
        self.inline[self.inline_len].take()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum UnifTermSide {
    NoTerm = 0,
    LeftTerm = 1,
    RightTerm = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum OracleUnifResult {
    Unifiable = 0,
    NotUnifiable = 1,
    NotInFragment = 2,
}

pub type UnificationResult = bool;

pub const UNIF_FAILED: UnificationResult = false;
pub const UNIF_SUCC: UnificationResult = true;

#[must_use]
pub const fn unif_failed(result: UnificationResult) -> bool {
    !result
}

#[cfg(feature = "measure-unification")]
#[must_use]
pub fn unification_attempts() -> i64 {
    UNIFICATION_ATTEMPTS.load(Ordering::Relaxed)
}

#[cfg(feature = "measure-unification")]
#[must_use]
pub fn unification_successes() -> i64 {
    UNIFICATION_SUCCESSES.load(Ordering::Relaxed)
}

#[cfg(feature = "measure-unification")]
fn record_unification_attempt() {
    UNIFICATION_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
}

#[cfg(feature = "measure-unification")]
fn record_unification_success() {
    UNIFICATION_SUCCESSES.fetch_add(1, Ordering::Relaxed);
}

/// Checks whether `var` occurs in `term` after full variable dereferencing.
///
/// # Panics
///
/// Panics if a traversed term argument is uninitialized.
#[must_use]
pub fn occur_check(term: &Term, var: &Term) -> bool {
    let mut deref = DerefType::Always;
    let term = term_deref(term, &mut deref);
    if &term == var {
        return true;
    }

    for index in 0..term.arity() {
        let arg = required_arg(&term, index);
        if occur_check(&arg, var) {
            return true;
        }
    }
    false
}

/// Computes a first-order match from `matcher` onto `to_match`.
///
/// On success, new matcher-variable bindings are appended to `subst`. On
/// failure, the substitution is backtracked to its entry state.
///
/// # Panics
///
/// Panics if matching reaches untyped variables/terms or uninitialized
/// arguments. The C routine asserts these as internal preconditions.
pub fn subst_compute_match(matcher: &Term, to_match: &Term, subst: &mut Substitution) -> bool {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::MguTimer);
    let mut matcher_weight = term_standard_weight(matcher);
    let to_match_weight = term_standard_weight(to_match);
    if matcher_weight > to_match_weight
        || (to_match.query_prop(TP_PRED_POS) && matcher.is_free_var())
    {
        return false;
    }

    let backtrack = subst.len();
    let mut jobs = MatchJobStack::new(matcher.clone(), to_match.clone());

    let mut result = true;
    while let Some((matcher, to_match)) = jobs.pop() {
        if matcher.is_free_var() {
            let matcher_type = matcher.type_().expect("matcher variable must have a type");
            let to_match_type = to_match.type_().expect("matched term must have a type");
            if matcher_type != to_match_type {
                result = false;
                break;
            }

            if let Some(binding) = matcher.binding() {
                if binding != to_match {
                    result = false;
                    break;
                }
            } else {
                subst.add_binding(&matcher, &to_match);
            }

            matcher_weight += term_standard_weight(&to_match) - DEFAULT_VWEIGHT;
            if matcher_weight > to_match_weight {
                result = false;
                break;
            }
        } else if matcher.f_code() != to_match.f_code() {
            result = false;
            break;
        } else {
            assert_eq!(
                matcher.arity(),
                to_match.arity(),
                "matched terms with identical f-code have identical arity"
            );
            for index in (0..matcher.arity()).rev() {
                jobs.push(
                    required_arg(&matcher, index),
                    required_arg(&to_match, index),
                );
            }
        }
    }

    if !result {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

/// Computes a first-order most-general unifier.
///
/// The input variables are expected to be variable-disjoint, matching the C
/// routine's documented precondition. On failure, `subst` is backtracked to its
/// entry state.
///
/// # Panics
///
/// Panics if non-variable terms with the same function code have inconsistent
/// arity/type metadata, if variable bindings would be untyped, if a traversed
/// argument is uninitialized.
pub fn subst_compute_mgu(t1: &Term, t2: &Term, subst: &mut Substitution) -> bool {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::MguTimer);
    #[cfg(feature = "measure-unification")]
    record_unification_attempt();

    if (t1.query_prop(TP_PRED_POS) && t2.is_free_var())
        || (t2.query_prop(TP_PRED_POS) && t1.is_free_var())
    {
        return false;
    }

    let backtrack = subst.len();
    let mut jobs = VecDeque::new();
    jobs.push_back((t1.clone(), t2.clone()));

    let mut result = true;
    while let Some((left, right)) = jobs.pop_back() {
        let mut right_deref = DerefType::Always;
        let mut right = term_deref_owned(right, &mut right_deref);
        let mut left_deref = DerefType::Always;
        let mut left = term_deref_owned(left, &mut left_deref);

        if right.is_free_var() {
            std::mem::swap(&mut left, &mut right);
        }

        if left.is_free_var() {
            if left != right {
                let left_type = left.type_().expect("left variable must have a type");
                let right_type = right.type_().expect("right term must have a type");
                if left_type != right_type || occur_check(&right, &left) {
                    result = false;
                    break;
                }
                subst.add_binding(&left, &right);
            }
        } else if left.f_code() != right.f_code() {
            result = false;
            break;
        } else {
            assert_eq!(
                left.arity(),
                right.arity(),
                "unified terms with identical f-code have identical arity"
            );
            assert_eq!(
                left.type_(),
                right.type_(),
                "unified non-variable terms have identical types"
            );
            for index in (0..left.arity()).rev() {
                let left_arg = required_arg(&left, index);
                let right_arg = required_arg(&right, index);
                if left_arg.is_free_var() || right_arg.is_free_var() {
                    jobs.push_front((left_arg, right_arg));
                } else {
                    jobs.push_back((left_arg, right_arg));
                }
            }
        }
    }

    if result {
        #[cfg(feature = "measure-unification")]
        record_unification_success();
    } else {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

/// First-order complete-match wrapper matching the non-LFHO C macro.
///
/// # Panics
///
/// Panics under the same conditions as [`subst_compute_match`].
pub fn subst_match_complete(pattern: &Term, target: &Term, subst: &mut Substitution) -> bool {
    subst_compute_match(pattern, target, subst)
}

/// C `SubstMatchComplete`, with the owning term bank passed explicitly for the
/// LFHO branch that C obtains through `TermGetBank`.
///
/// # Errors
///
/// Returns diagnostics from eta reduction, prefix binding construction, or
/// the higher-order pattern-matching fallback.
///
/// # Panics
///
/// Panics on malformed higher-order terms or missing type metadata, matching
/// the C implementation's internal assertions.
pub fn subst_match_complete_with_bank(
    bank: &mut TermBank,
    pattern: &Term,
    target: &Term,
    subst: &mut Substitution,
) -> Result<bool, Diagnostic> {
    if problem_type() != ProblemType::HigherOrder {
        return Ok(subst_compute_match(pattern, target, subst));
    }

    let backtrack = subst.len();
    let reduced_pattern = lambda_eta_reduce_db(bank, pattern)?;
    let reduced_target = lambda_eta_reduce_db(bank, target)?;
    let mut result = subst_compute_match_ho(bank, &reduced_pattern, &reduced_target, subst)?;

    if result != 0 && pattern.is_non_fo_pattern() && target.is_non_fo_pattern() {
        subst.backtrack_to_pos(backtrack);
        result = if subst_compute_match_pattern(bank, pattern, target, subst)?
            == OracleUnifResult::Unifiable
        {
            0
        } else {
            MATCH_FAILED
        };
        if result != 0 {
            subst.backtrack_to_pos(backtrack);
        }
    }

    Ok(result == 0)
}

/// C `SubstComputeMatchHO`.
///
/// Returns `MATCH_FAILED` on failure and the number of unmatched target
/// arguments on success. The current C algorithm returns zero for complete
/// successful matches, but retaining the integer result preserves its API.
///
/// # Errors
///
/// Returns diagnostics from lambda-prefix preparation or application-variable
/// binding construction.
///
/// # Panics
///
/// Panics on malformed higher-order applications, missing type metadata, or
/// an internal negative result other than `MATCH_FAILED`, matching C's
/// assertion-based contract.
pub fn subst_compute_match_ho(
    bank: &mut TermBank,
    matcher: &Term,
    to_match: &Term,
    subst: &mut Substitution,
) -> Result<i32, Diagnostic> {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::MguTimer);
    let backtrack = subst.len();
    match subst_compute_match_ho_inner(bank, matcher, to_match, subst) {
        Ok(MATCH_FAILED) => {
            subst.backtrack_to_pos(backtrack);
            Ok(MATCH_FAILED)
        }
        Ok(result) => {
            let remaining = usize::try_from(result)
                .expect("successful higher-order match has a nonnegative remainder");
            debug_assert!(term_struct_prefix_equal(
                matcher,
                to_match,
                DerefType::Once,
                DerefType::Never,
                remaining,
            ));
            Ok(result)
        }
        Err(error) => {
            subst.backtrack_to_pos(backtrack);
            Err(error)
        }
    }
}

fn subst_compute_match_ho_inner(
    bank: &mut TermBank,
    matcher: &Term,
    to_match: &Term,
    subst: &mut Substitution,
) -> Result<i32, Diagnostic> {
    let mut matcher_weight = term_standard_weight(matcher);
    let to_match_weight = term_standard_weight(to_match);
    if matcher_weight > to_match_weight || matcher.type_() != to_match.type_() {
        return Ok(MATCH_FAILED);
    }

    let mut jobs = MatchJobStack::new(matcher.clone(), to_match.clone());
    let mut result = 0;

    while let Some((mut matcher, mut to_match)) = jobs.pop() {
        (matcher, to_match) = prune_lambda_prefix(bank, matcher, to_match)?;
        let start_index;

        if matcher.is_top_level_free_var() {
            let variable = if matcher.is_applied_free_var() {
                required_arg(&matcher, 0)
            } else {
                matcher.clone()
            };

            if let Some(binding) = variable.binding() {
                if binding.is_lambda() || !term_is_prefix(Some(&binding), &to_match) {
                    result = MATCH_FAILED;
                    break;
                }
                start_index = binding.arg_num();
                matcher_weight += term_standard_weight(&binding) - DEFAULT_VWEIGHT;
                if matcher_weight > to_match_weight {
                    result = MATCH_FAILED;
                    break;
                }
                assert_eq!(
                    start_index + matcher.arg_num(),
                    to_match.arg_num(),
                    "bound HO matcher prefix must consume the target"
                );
            } else {
                let Some(args_eaten) = partially_match_var(bank, &variable, &to_match, false)
                else {
                    result = MATCH_FAILED;
                    break;
                };
                subst.bind_app_var(
                    &variable,
                    &to_match,
                    args_eaten,
                    bank,
                    ProblemType::HigherOrder,
                )?;
                start_index = args_eaten;
                let binding = variable
                    .binding()
                    .expect("successful application-variable binding is installed");
                matcher_weight += term_standard_weight(&binding) - DEFAULT_VWEIGHT;
                if matcher_weight > to_match_weight {
                    result = MATCH_FAILED;
                    break;
                }
                assert_eq!(
                    args_eaten + matcher.arg_num(),
                    to_match.arg_num(),
                    "HO matcher must consume all target arguments"
                );
            }
        } else {
            if matcher.is_db_var() != to_match.is_db_var()
                || matcher.is_applied_db_var() != to_match.is_applied_db_var()
                || matcher.arity() != to_match.arity()
            {
                result = MATCH_FAILED;
                break;
            }
            if matcher.f_code() != to_match.f_code()
                || (!matcher.is_top_level_db_var()
                    && bank.signature().is_polymorphic(matcher.f_code())
                    && matcher.arity() != 0
                    && required_arg(&matcher, 0).type_() != required_arg(&to_match, 0).type_())
            {
                result = MATCH_FAILED;
                break;
            }
            assert_eq!(matcher.arg_num(), to_match.arg_num());
            start_index = 0;
        }

        let matcher_offset = usize::from(matcher.is_applied_free_var());
        let target_offset = start_index + usize::from(to_match.is_applied_free_var());
        for index in 0..matcher.arity().saturating_sub(matcher_offset) {
            jobs.push(
                required_arg(&matcher, index + matcher_offset),
                required_arg(&to_match, index + target_offset),
            );
        }
    }

    Ok(result)
}

/// First-order complete-MGU wrapper matching the non-LFHO C macro.
///
/// # Panics
///
/// Panics under the same conditions as [`subst_compute_mgu`].
pub fn subst_mgu_complete(t: &Term, s: &Term, subst: &mut Substitution) -> bool {
    subst_compute_mgu(t, s, subst)
}

/// C `SubstMguComplete`, with the owning term bank passed explicitly for the
/// LFHO branch that C obtains through `TermGetBank`.
///
/// # Errors
///
/// Returns diagnostics from weak-head normalization, eta reduction, prefix
/// binding construction, or the pattern-unification fallback.
///
/// # Panics
///
/// Panics on malformed higher-order term cells or missing type metadata,
/// matching the C implementation's internal assertions.
pub fn subst_mgu_complete_with_bank(
    bank: &mut TermBank,
    t: &Term,
    s: &Term,
    subst: &mut Substitution,
) -> Result<bool, Diagnostic> {
    if problem_type() != ProblemType::HigherOrder {
        return Ok(subst_compute_mgu(t, s, subst));
    }

    let backtrack = subst.len();
    let result = (|| {
        let reduced_t = lambda_eta_reduce_db(bank, t)?;
        let reduced_s = lambda_eta_reduce_db(bank, s)?;
        let mut result = subst_compute_mgu_ho(bank, &reduced_t, &reduced_s, subst)?;

        if !result && t.is_non_fo_pattern() && s.is_non_fo_pattern() {
            subst.backtrack_to_pos(backtrack);
            result = subst_compute_mgu_pattern(bank, t, s, subst)? == OracleUnifResult::Unifiable;
        }
        Ok(result)
    })();

    if !matches!(result, Ok(true)) {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

/// C `SubstComputeMguHO`.
fn subst_compute_mgu_ho(
    bank: &mut TermBank,
    t1: &Term,
    t2: &Term,
    subst: &mut Substitution,
) -> Result<UnificationResult, Diagnostic> {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::MguTimer);
    #[cfg(feature = "measure-unification")]
    record_unification_attempt();

    if t1.type_() != t2.type_() {
        return Ok(UNIF_FAILED);
    }

    let backtrack = subst.len();
    let mut jobs = PQueue::new();
    jobs.store(t1.clone());
    jobs.store(t2.clone());

    let mut result = UNIF_SUCC;
    while !jobs.is_empty() {
        let mut right = whnf_deref(bank, &jobs.get_last())?;
        let mut left = whnf_deref(bank, &jobs.get_last())?;

        if left.is_free_var() && term_is_db_closed(&right) && !occur_check(&right, &left) {
            subst.add_binding(&left, &right);
            continue;
        }
        if right.is_free_var() && term_is_db_closed(&left) && !occur_check(&left, &right) {
            subst.add_binding(&right, &left);
            continue;
        }

        (left, right) = prune_lambda_prefix(bank, left, right)?;

        if reorientation_needed(&left, &right) {
            std::mem::swap(&mut left, &mut right);
        }

        let args_eaten = if left.is_top_level_free_var() {
            let var = if left.is_applied_free_var() {
                required_arg(&left, 0)
            } else {
                left.clone()
            };
            assert!(
                !right.is_top_level_free_var() || left.arity() <= right.arity(),
                "HO MGU reorientation orders top-level free-variable arities"
            );

            let Some(mut args_eaten) = partially_match_var(bank, &var, &right, true) else {
                result = UNIF_FAILED;
                break;
            };

            let subst_pos =
                subst.bind_app_var(&var, &right, args_eaten, bank, ProblemType::HigherOrder)?;
            if var.binding().as_ref() == Some(&var) {
                subst.backtrack_to_pos(subst_pos);
                args_eaten = 0;
            }
            args_eaten
        } else {
            if left.is_db_var() != right.is_db_var()
                || left.is_applied_db_var() != right.is_applied_db_var()
                || left.arity() != right.arity()
            {
                result = UNIF_FAILED;
                break;
            }

            if left.f_code() != right.f_code()
                || (!left.is_top_level_db_var()
                    && bank.signature().is_polymorphic(left.f_code())
                    && left.arity() != 0
                    && required_arg(&left, 0).type_() != required_arg(&right, 0).type_())
            {
                result = UNIF_FAILED;
                break;
            }
            0
        };

        schedule_ho_mgu_jobs(&mut jobs, &left, &right, args_eaten);
    }

    if result == UNIF_SUCC {
        #[cfg(feature = "measure-unification")]
        record_unification_success();
        debug_assert!(term_struct_prefix_equal(
            t1,
            t2,
            DerefType::Always,
            DerefType::Always,
            0,
        ));
    } else {
        subst.backtrack_to_pos(backtrack);
    }

    Ok(result)
}

fn reorientation_needed(left: &Term, right: &Term) -> bool {
    (right.is_top_level_free_var() && !left.is_top_level_free_var())
        || (left.is_top_level_free_var()
            && right.is_top_level_free_var()
            && left.arity() > right.arity())
}

fn partially_match_var(
    bank: &mut TermBank,
    var_matcher: &Term,
    to_match: &Term,
    perform_occur_check: bool,
) -> Option<usize> {
    assert!(
        var_matcher.is_free_var() && var_matcher.binding().is_none(),
        "partial variable matching expects an unbound free variable"
    );
    assert!(
        problem_type() == ProblemType::HigherOrder
            || !var_matcher.type_().is_some_and(|type_| type_.is_arrow()),
        "first-order variable matching does not consume arrow-typed variables"
    );
    assert!(
        !to_match.is_lambda(),
        "partial variable matching expects a non-lambda target"
    );

    let term_head_type = head_type(bank, to_match)?;
    if to_match.is_top_level_db_var() {
        return None;
    }

    let matcher_type = var_matcher
        .type_()
        .expect("partial variable matcher has a type");
    let target_type = to_match
        .type_()
        .expect("partial variable target has a type");
    let args_to_eat = if matcher_type == target_type {
        to_match.arg_num()
    } else if term_head_type.is_arrow()
        && matcher_type.is_arrow()
        && matcher_type.arity() <= term_head_type.arity()
    {
        let start = term_head_type.arity() - matcher_type.arity();
        for index in start..term_head_type.arity() {
            if matcher_type.args()[index - start] != term_head_type.args()[index] {
                return None;
            }
        }
        assert!(
            start != 0 || matcher_type == term_head_type,
            "zero consumed arguments imply shared head and matcher types"
        );
        start
    } else {
        return None;
    };

    if args_to_eat > to_match.arg_num() {
        return None;
    }

    let checked_args = args_to_eat + usize::from(to_match.is_applied_any_var());
    for index in 0..checked_args {
        let arg = required_arg(to_match, index);
        if !term_is_db_closed(&arg) || (perform_occur_check && occur_check(&arg, var_matcher)) {
            return None;
        }
    }

    Some(args_to_eat)
}

fn head_type(bank: &mut TermBank, term: &Term) -> Option<Type> {
    let f_code = term.f_code();
    if f_code == SIG_ITE_CODE || f_code == SIG_LET_CODE {
        return term.type_();
    }

    if f_code == bank.signature().qex_code() || f_code == bank.signature().qall_code() {
        return Some(bank.signature().type_bank().bool_type());
    }

    if term.is_applied_any_var() {
        return required_arg(term, 0).type_();
    }
    if term.is_any_var() || term.is_lambda() {
        assert!(
            !term.is_any_var() || term.arity() == 0,
            "unapplied variables have no visible arguments"
        );
        return term.type_();
    }
    if f_code == SIG_PHONY_APP_CODE {
        let head = required_arg(term, 0);
        let head_type = head_type(bank, &head)?;
        assert!(head_type.is_arrow(), "phony-app head type must be an arrow");
        return Some(
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(type_drop_first_arg(&head_type)),
        );
    }

    bank.signature().get_type(f_code).cloned()
}

fn schedule_ho_mgu_jobs(jobs: &mut PQueue<Term>, left: &Term, right: &Term, args_eaten: usize) {
    let left_offset = usize::from(left.is_applied_free_var());
    let right_offset = args_eaten + usize::from(right.is_applied_free_var());
    for index in 0..left.arity().saturating_sub(left_offset) {
        let left_arg = required_arg(left, index + left_offset);
        let right_arg = required_arg(right, index + right_offset);
        if left_arg.is_top_level_free_var() || right_arg.is_top_level_free_var() {
            jobs.bury(right_arg);
            jobs.bury(left_arg);
        } else {
            jobs.store(left_arg);
            jobs.store(right_arg);
        }
    }
}

/// Returns whether a term contains syntax that needs the higher-order CSU path.
///
/// The first-order MGU routines can still be used for ordinary first-order
/// subterms in a higher-order problem. Lambda terms, DB variables, and phony
/// applications require the higher-order CSU path.
#[must_use]
pub fn term_has_higher_order_unification_surface(term: &Term) -> bool {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_db_var() || current.is_lambda() || current.is_phony_app() {
            return true;
        }
        stack.extend(current.argument_clones().into_iter().flatten());
    }
    false
}

/// Verifies that a matcher equals the target with one-step matcher
/// dereferencing and no target dereferencing.
///
/// # Panics
///
/// Panics if a traversed argument is uninitialized.
#[must_use]
pub fn verify_match(matcher: &Term, to_match: &Term) -> bool {
    term_struct_equal_deref(matcher, to_match, DerefType::Once, DerefType::Never)
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{
        occur_check, subst_compute_match, subst_compute_match_ho, subst_compute_mgu,
        subst_match_complete, subst_match_complete_with_bank, subst_mgu_complete,
        subst_mgu_complete_with_bank, unif_failed, verify_match, OracleUnifResult, UnifTermSide,
        MATCH_FAILED, UNIF_FAILED, UNIF_SUCC,
    };
    #[cfg(feature = "measure-unification")]
    use super::{unification_attempts, unification_successes};
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::terms::lambda::apply_terms;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{Term, TP_PRED_POS};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    fn typed_var(code: i64, type_: &Type) -> Term {
        let var = Term::const_cell_alloc(code);
        var.set_type(Some(type_.clone()));
        var
    }

    fn typed_const(code: i64, type_: &Type) -> Term {
        let term = Term::const_cell_alloc(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn typed_term(code: i64, args: &[Term], type_: &Type) -> Term {
        let term = Term::top_alloc(code, args.len());
        term.set_type(Some(type_.clone()));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        term
    }

    fn applied_free_var(head: &Term, args: &[Term], type_: &Type) -> Term {
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, args.len() + 1);
        term.set_type(Some(type_.clone()));
        term.set_argument(0, head.clone());
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index + 1, arg.clone());
        }
        term
    }

    #[test]
    fn public_result_shapes_match_c_discriminants() {
        assert_eq!(UnifTermSide::NoTerm as i32, 0);
        assert_eq!(UnifTermSide::LeftTerm as i32, 1);
        assert_eq!(UnifTermSide::RightTerm as i32, 2);
        assert_eq!(OracleUnifResult::Unifiable as i32, 0);
        assert_eq!(OracleUnifResult::NotUnifiable as i32, 1);
        assert_eq!(OracleUnifResult::NotInFragment as i32, 2);
        assert!(unif_failed(UNIF_FAILED));
        assert!(!unif_failed(UNIF_SUCC));
    }

    #[test]
    fn occur_check_follows_existing_bindings() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let y = typed_var(-4, &type_);
        assert!(!occur_check(&y, &x));
        y.set_binding(Some(x.clone()));
        assert!(occur_check(&y, &x));
    }

    #[test]
    fn occur_check_expands_bound_applied_free_variable_heads() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let head = typed_var(-4, &type_);
        let suffix = typed_const(10, &type_);
        let prefix = typed_term(20, std::slice::from_ref(&x), &type_);
        head.set_binding(Some(prefix));
        let applied = applied_free_var(&head, &[suffix], &type_);

        assert!(occur_check(&applied, &x));
    }

    #[test]
    fn matching_binds_variables_and_verifies_dereferenced_result() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let a = typed_const(10, &type_);
        let pattern = typed_term(20, std::slice::from_ref(&x), &type_);
        let target = typed_term(20, std::slice::from_ref(&a), &type_);
        let mut subst = Substitution::new();

        assert!(subst_compute_match(&pattern, &target, &mut subst));
        assert_eq!(subst.len(), 1);
        assert_eq!(x.binding(), Some(a));
        assert!(verify_match(&pattern, &target));
    }

    #[test]
    fn matching_preserves_lifo_order_after_inline_stack_overflow() {
        let type_ = TypeBank::new().i_type();
        let variables: Vec<_> = (0_i64..40)
            .map(|index| typed_var(-2 - 2 * index, &type_))
            .collect();
        let constants: Vec<_> = (0_i64..40)
            .map(|index| typed_const(100 + index, &type_))
            .collect();
        let pattern = typed_term(20, &variables, &type_);
        let target = typed_term(20, &constants, &type_);
        let mut subst = Substitution::new();

        assert!(subst_compute_match(&pattern, &target, &mut subst));
        assert_eq!(subst.len(), variables.len());
        for (variable, constant) in variables.iter().zip(&constants) {
            assert_eq!(variable.binding().as_ref(), Some(constant));
        }
        assert_eq!(subst.backtrack(), variables.len());
        assert!(variables
            .iter()
            .all(|variable| variable.binding().is_none()));
    }

    #[test]
    fn matching_repeated_variables_uses_binding_identity_like_c() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let a_left = typed_const(10, &type_);
        let a_right = typed_const(10, &type_);
        let pattern = typed_term(20, &[x.clone(), x.clone()], &type_);
        let target = typed_term(20, &[a_left, a_right], &type_);
        let mut subst = Substitution::new();

        assert!(!subst_compute_match(&pattern, &target, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());
    }

    #[test]
    fn matching_backtracks_on_type_or_weight_failure() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let bool_type = types.bool_type();
        let x = typed_var(-2, &i_type);
        let a = typed_const(10, &i_type);
        let wrong_type = typed_const(11, &bool_type);
        let pattern = typed_term(20, &[x.clone(), x.clone()], &i_type);
        let target = typed_term(20, &[a, wrong_type], &i_type);
        let mut subst = Substitution::new();

        assert!(!subst_compute_match(&pattern, &target, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());
    }

    #[test]
    fn matching_rejects_predicate_position_free_variable_target() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let pred = typed_const(10, &type_);
        pred.set_prop(TP_PRED_POS);
        let mut subst = Substitution::new();

        assert!(!subst_compute_match(&x, &pred, &mut subst));
        assert!(subst.is_empty());
    }

    #[test]
    fn mgu_binds_disjoint_variables_and_delays_variable_jobs() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let y = typed_var(-4, &type_);
        let a = typed_const(10, &type_);
        let b = typed_const(11, &type_);
        let left = typed_term(20, &[x.clone(), a.clone()], &type_);
        let right = typed_term(20, &[b.clone(), y.clone()], &type_);
        let mut subst = Substitution::new();

        assert!(subst_compute_mgu(&left, &right, &mut subst));
        assert_eq!(subst.len(), 2);
        assert_eq!(y.binding(), Some(a));
        assert_eq!(x.binding(), Some(b));
    }

    #[test]
    fn mgu_backtracks_on_occurs_check_failure() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let containing_x = typed_term(20, std::slice::from_ref(&x), &type_);
        let mut subst = Substitution::new();

        assert!(!subst_compute_mgu(&x, &containing_x, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());
    }

    #[test]
    fn mgu_backtracks_an_earlier_delayed_binding_when_a_later_job_fails() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let y = typed_var(-4, &type_);
        let a = typed_const(10, &type_);
        let containing_x = typed_term(20, std::slice::from_ref(&x), &type_);
        let left = typed_term(30, &[x.clone(), a], &type_);
        let right = typed_term(30, &[containing_x, y.clone()], &type_);
        let mut subst = Substitution::new();

        assert!(!subst_compute_mgu(&left, &right, &mut subst));
        assert!(subst.is_empty());
        assert!(x.binding().is_none());
        assert!(y.binding().is_none());
    }

    #[test]
    fn mgu_expands_bound_applied_free_variable_heads() {
        let type_ = TypeBank::new().i_type();
        let head = typed_var(-2, &type_);
        let prefix_arg = typed_const(10, &type_);
        let suffix_arg = typed_const(11, &type_);
        let prefix = typed_term(20, std::slice::from_ref(&prefix_arg), &type_);
        let target = typed_term(20, &[prefix_arg, suffix_arg.clone()], &type_);
        let applied = applied_free_var(&head, &[suffix_arg], &type_);
        let mut subst = Substitution::new();

        head.set_binding(Some(prefix));

        assert!(subst_compute_mgu(&applied, &target, &mut subst));
        assert!(subst.is_empty());
    }

    #[test]
    fn mgu_rejects_predicate_position_variable_side() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let pred = typed_const(10, &type_);
        pred.set_prop(TP_PRED_POS);
        let mut subst = Substitution::new();

        assert!(!subst_compute_mgu(&pred, &x, &mut subst));
        assert!(subst.is_empty());
    }

    #[cfg(feature = "measure-unification")]
    #[test]
    fn mgu_measurement_counters_follow_c_attempt_success_points() {
        let attempts_before = unification_attempts();
        let successes_before = unification_successes();
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let a = typed_const(10, &type_);
        let containing_x = typed_term(20, std::slice::from_ref(&x), &type_);
        let mut subst = Substitution::new();

        assert!(subst_compute_mgu(&x, &a, &mut subst));
        subst.backtrack();
        assert!(!subst_compute_mgu(&x, &containing_x, &mut subst));

        assert!(unification_attempts() >= attempts_before + 2);
        assert!(unification_successes() > successes_before);
    }

    #[test]
    fn unbanked_complete_wrappers_use_first_order_paths() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let a = typed_const(10, &type_);
        let mut subst = Substitution::new();

        assert!(subst_match_complete(&x, &a, &mut subst));
        subst.backtrack();
        assert!(subst_mgu_complete(&x, &a, &mut subst));
    }

    #[test]
    fn banked_complete_mgu_binds_applied_variable_to_rigid_prefix() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        let mut bank = TermBank::new(signature).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex = bank.vars().get_fresh_var(&unary);
        let prefix_code = bank.signature_mut().insert_id("mgu_ho_prefix", 0, false);
        bank.signature_mut()
            .declare_final_type(prefix_code, individual.clone())
            .unwrap();
        let prefix = bank.create_const_term(prefix_code).unwrap();
        let suffix_code = bank.signature_mut().insert_id("mgu_ho_suffix", 0, false);
        bank.signature_mut()
            .declare_final_type(suffix_code, individual.clone())
            .unwrap();
        let suffix = bank.create_const_term(suffix_code).unwrap();
        let binary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual,
            ]));
        let rigid_code = bank.signature_mut().insert_id("mgu_ho_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, binary)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let applied = apply_terms(&mut bank, &flex, std::slice::from_ref(&suffix)).unwrap();
        let target = apply_terms(&mut bank, &rigid, &[prefix.clone(), suffix.clone()]).unwrap();
        let mut subst = Substitution::new();

        assert!(!subst_mgu_complete(&applied, &target, &mut subst));
        assert!(subst_mgu_complete_with_bank(&mut bank, &applied, &target, &mut subst).unwrap());
        assert!(flex
            .binding()
            .is_some_and(|binding| binding.f_code() == rigid_code && binding.arity() == 1));
        subst.backtrack();
        assert!(flex.binding().is_none());

        let mismatch_code = bank.signature_mut().insert_id("mgu_ho_mismatch", 0, false);
        let mismatch_type = bank.signature().type_bank().default_type();
        bank.signature_mut()
            .declare_final_type(mismatch_code, mismatch_type)
            .unwrap();
        let mismatch = bank.create_const_term(mismatch_code).unwrap();
        let mismatch_target = apply_terms(&mut bank, &rigid, &[prefix, mismatch]).unwrap();
        let retained = bank
            .vars()
            .get_fresh_var(&bank.signature().type_bank().default_type());
        subst.add_binding(&retained, &suffix);

        assert!(
            !subst_mgu_complete_with_bank(&mut bank, &applied, &mismatch_target, &mut subst)
                .unwrap()
        );
        assert_eq!(subst.len(), 1);
        assert_eq!(retained.binding(), Some(suffix));
        assert!(flex.binding().is_none());
        subst.backtrack();
    }

    #[test]
    fn complete_match_with_bank_dispatches_to_higher_order_application_matching() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        let mut bank = TermBank::new(signature).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex = bank.vars().get_fresh_var(&unary);
        let db0 = bank.request_db_var(&individual, 0);
        let matcher = apply_terms(&mut bank, &flex, std::slice::from_ref(&db0)).unwrap();
        let rigid_code = bank.signature_mut().insert_id("match_ho_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, unary)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let target = apply_terms(&mut bank, &rigid, std::slice::from_ref(&db0)).unwrap();
        let mut subst = Substitution::new();

        assert!(!subst_compute_match(&matcher, &target, &mut subst));
        assert_eq!(
            subst_compute_match_ho(&mut bank, &matcher, &target, &mut subst).unwrap(),
            0
        );
        assert_eq!(flex.binding(), Some(rigid.clone()));
        subst.backtrack();

        assert!(subst_match_complete_with_bank(&mut bank, &matcher, &target, &mut subst).unwrap());
        assert_eq!(flex.binding(), Some(rigid));
    }

    #[test]
    fn complete_match_with_bank_falls_back_to_pattern_matching() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        let mut bank = TermBank::new(signature).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex = bank.vars().get_fresh_var(&unary);
        let db0 = bank.request_db_var(&individual, 0);
        let matcher_body = apply_terms(&mut bank, &flex, std::slice::from_ref(&db0)).unwrap();
        let matcher = crate::terms::lambda::close_with_type_prefix(
            &mut bank,
            std::slice::from_ref(&individual),
            &matcher_body,
        )
        .unwrap();
        let target = crate::terms::lambda::close_with_type_prefix(
            &mut bank,
            std::slice::from_ref(&individual),
            &db0,
        )
        .unwrap();
        let mut subst = Substitution::new();

        assert!(matcher.is_non_fo_pattern());
        assert!(target.is_non_fo_pattern());
        assert!(subst_match_complete_with_bank(&mut bank, &matcher, &target, &mut subst).unwrap());
        assert!(flex.binding().is_some_and(|binding| binding.is_lambda()));
    }

    #[test]
    fn higher_order_match_backtracks_failed_prefix_binding() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        let mut bank = TermBank::new(signature).unwrap();
        let individual = bank.signature().type_bank().default_type();
        let variable = bank.vars().get_fresh_var(&individual);
        let binary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual.clone(),
            ]));
        let h_code = bank
            .signature_mut()
            .insert_id("match_ho_backtrack_h", 0, false);
        bank.signature_mut()
            .declare_final_type(h_code, binary)
            .unwrap();
        let h = bank.create_const_term(h_code).unwrap();
        let mut constant = |name: &str| {
            let code = bank.signature_mut().insert_id(name, 0, false);
            bank.signature_mut()
                .declare_final_type(code, individual.clone())
                .unwrap();
            bank.create_const_term(code).unwrap()
        };
        let a = constant("match_ho_backtrack_a");
        let b = constant("match_ho_backtrack_b");
        let c = constant("match_ho_backtrack_c");
        let matcher = apply_terms(&mut bank, &h, &[variable.clone(), a]).unwrap();
        let target = apply_terms(&mut bank, &h, &[b, c]).unwrap();
        let mut subst = Substitution::new();

        assert_eq!(
            subst_compute_match_ho(&mut bank, &matcher, &target, &mut subst).unwrap(),
            MATCH_FAILED
        );
        assert!(subst.is_empty());
        assert!(variable.binding().is_none());
    }

    #[test]
    fn verify_match_expands_bound_applied_free_variable_once() {
        let type_ = TypeBank::new().i_type();
        let head = typed_var(-2, &type_);
        let prefix_arg = typed_const(10, &type_);
        let suffix_arg = typed_const(11, &type_);
        let prefix = typed_term(20, std::slice::from_ref(&prefix_arg), &type_);
        let target = typed_term(20, &[prefix_arg, suffix_arg.clone()], &type_);
        let applied = applied_free_var(&head, &[suffix_arg], &type_);

        head.set_binding(Some(prefix));

        assert!(verify_match(&applied, &target));
    }
}
