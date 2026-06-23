use crate::basics::pqueue::PQueue;
use crate::terms::subst::Substitution;
use crate::terms::termfunc::{term_standard_weight, term_struct_equal_deref};
use crate::terms::termtypes::{term_deref, DerefType, Term, DEFAULT_VWEIGHT, TP_PRED_POS};

pub const MATCH_FAILED: i32 = -1;

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

/// Checks whether `var` occurs in `term` after full variable dereferencing.
///
/// # Panics
///
/// Panics if a traversed term argument is uninitialized or if dereferencing an
/// applied variable reaches an unported higher-order term-bank path.
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
    let mut matcher_weight = term_standard_weight(matcher);
    let to_match_weight = term_standard_weight(to_match);
    if matcher_weight > to_match_weight
        || (to_match.query_prop(TP_PRED_POS) && matcher.is_free_var())
    {
        return false;
    }

    let backtrack = subst.len();
    let mut jobs = Vec::new();
    push_lifo_pair(&mut jobs, matcher.clone(), to_match.clone());

    let mut result = true;
    while let Some((matcher, to_match)) = pop_lifo_pair(&mut jobs) {
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
                push_lifo_pair(
                    &mut jobs,
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
/// argument is uninitialized, or if applied-variable dereferencing reaches an
/// unported higher-order term-bank path.
pub fn subst_compute_mgu(t1: &Term, t2: &Term, subst: &mut Substitution) -> bool {
    if (t1.query_prop(TP_PRED_POS) && t2.is_free_var())
        || (t2.query_prop(TP_PRED_POS) && t1.is_free_var())
    {
        return false;
    }

    let backtrack = subst.len();
    let mut jobs = PQueue::new();
    jobs.store(t1.clone());
    jobs.store(t2.clone());

    let mut result = true;
    while !jobs.is_empty() {
        let mut right_deref = DerefType::Always;
        let mut right = term_deref(
            &jobs
                .get_last()
                .expect("unification queue stores complete pairs"),
            &mut right_deref,
        );
        let mut left_deref = DerefType::Always;
        let mut left = term_deref(
            &jobs
                .get_last()
                .expect("unification queue stores complete pairs"),
            &mut left_deref,
        );

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
                    jobs.bury(right_arg);
                    jobs.bury(left_arg);
                } else {
                    jobs.store(left_arg);
                    jobs.store(right_arg);
                }
            }
        }
    }

    if !result {
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

/// First-order complete-MGU wrapper matching the non-LFHO C macro.
///
/// # Panics
///
/// Panics under the same conditions as [`subst_compute_mgu`].
pub fn subst_mgu_complete(t: &Term, s: &Term, subst: &mut Substitution) -> bool {
    subst_compute_mgu(t, s, subst)
}

/// Verifies that a matcher equals the target with one-step matcher
/// dereferencing and no target dereferencing.
///
/// # Panics
///
/// Panics if dereferencing reaches an unported applied-variable path or if a
/// traversed argument is uninitialized.
#[must_use]
pub fn verify_match(matcher: &Term, to_match: &Term) -> bool {
    term_struct_equal_deref(matcher, to_match, DerefType::Once, DerefType::Never)
}

fn push_lifo_pair(jobs: &mut Vec<Term>, left: Term, right: Term) {
    jobs.push(left);
    jobs.push(right);
}

fn pop_lifo_pair(jobs: &mut Vec<Term>) -> Option<(Term, Term)> {
    let right = jobs.pop()?;
    let left = jobs.pop().expect("match stack stores complete pairs");
    Some((left, right))
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{
        occur_check, subst_compute_match, subst_compute_mgu, subst_match_complete,
        subst_mgu_complete, unif_failed, verify_match, OracleUnifResult, UnifTermSide, UNIF_FAILED,
        UNIF_SUCC,
    };
    use crate::terms::simpletypes::Type;
    use crate::terms::subst::Substitution;
    use crate::terms::termtypes::{Term, TP_PRED_POS};
    use crate::terms::typebanks::TypeBank;

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
    fn mgu_rejects_predicate_position_variable_side() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let pred = typed_const(10, &type_);
        pred.set_prop(TP_PRED_POS);
        let mut subst = Substitution::new();

        assert!(!subst_compute_mgu(&pred, &x, &mut subst));
        assert!(subst.is_empty());
    }

    #[test]
    fn complete_wrappers_use_first_order_paths_for_now() {
        let type_ = TypeBank::new().i_type();
        let x = typed_var(-2, &type_);
        let a = typed_const(10, &type_);
        let mut subst = Substitution::new();

        assert!(subst_match_complete(&x, &a, &mut subst));
        subst.backtrack();
        assert!(subst_mgu_complete(&x, &a, &mut subst));
    }
}
