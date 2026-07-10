//! Higher-order pattern unification from `TERMS/cte_pattern_match_mgu`.

use crate::basics::error::Diagnostic;
use crate::basics::pqueue::PQueue;
use crate::terms::functypes::FunCode;
use crate::terms::lambda::{
    apply_terms, close_with_db_var, close_with_type_prefix, fresh_var_with_args,
    lambda_eta_expand_db, lambda_eta_reduce_db, lambda_normalize_db, shift_db, unfold_lambda,
    whnf_deref,
};
use crate::terms::match_mgu::{occur_check, OracleUnifResult};
use crate::terms::simpletypes::{get_ret_type, type_get_max_arity, Type};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_array_no_duplicates, term_is_db_closed, term_is_ground, term_standard_weight,
};
use crate::terms::termtypes::{term_deref, DerefType, Term, DEFAULT_VWEIGHT};
use std::collections::BTreeMap;

type DbVarMap = BTreeMap<FunCode, Term>;

/// C `SubstComputeMatchPattern`.
///
/// # Errors
///
/// Returns diagnostics from eta expansion, lambda normalization, DB-variable
/// remapping, lambda closure, or term-bank insertion.
///
/// # Panics
///
/// Panics on malformed lambda/application cells or missing type metadata,
/// matching the C implementation's internal assertions.
pub fn subst_compute_match_pattern(
    bank: &mut TermBank,
    matcher: &Term,
    to_match: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    if matcher.type_() != to_match.type_() || !matcher.is_pattern() || !to_match.is_pattern() {
        return Ok(OracleUnifResult::NotUnifiable);
    }

    let backtrack = subst.len();
    let result = subst_compute_match_pattern_inner(bank, matcher, to_match, subst);
    if !matches!(result, Ok(OracleUnifResult::Unifiable)) {
        subst.backtrack_to_pos(backtrack);
    }
    result
}

fn subst_compute_match_pattern_inner(
    bank: &mut TermBank,
    matcher: &Term,
    to_match: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    let matcher = lambda_eta_expand_db(bank, matcher)?;
    let to_match = lambda_eta_expand_db(bank, to_match)?;
    let mut jobs = Vec::new();
    push_match_pair(&mut jobs, matcher.clone(), to_match.clone());
    let mut result = OracleUnifResult::Unifiable;
    let mut matcher_weight = term_standard_weight(&matcher);
    let to_match_weight = term_standard_weight(&to_match);

    while let Some((mut matcher, mut to_match)) = pop_match_pair(&mut jobs) {
        if result != OracleUnifResult::Unifiable {
            break;
        }
        (matcher, to_match) = prune_lambda_prefix(bank, matcher, to_match)?;

        if term_is_ground(&matcher) && term_is_ground(&to_match) {
            let normalized_matcher = lambda_normalize_db(bank, &matcher)?;
            let normalized_target = lambda_normalize_db(bank, &to_match)?;
            if normalized_matcher != normalized_target {
                result = OracleUnifResult::NotUnifiable;
                break;
            }
        }
        if matcher_weight > to_match_weight {
            result = OracleUnifResult::NotUnifiable;
            break;
        }

        if matcher.is_top_level_free_var() {
            let Some(normalized_matcher) = normalize_pattern_app_var(bank, &matcher)? else {
                result = OracleUnifResult::NotInFragment;
                break;
            };
            let matcher_head = free_var_head(&normalized_matcher);
            if matcher_head.binding().is_some() {
                let mut deref = DerefType::Once;
                let dereferenced = term_deref(&normalized_matcher, &mut deref);
                let normalized_binding = lambda_normalize_db(bank, &dereferenced)?;
                let normalized_target = lambda_normalize_db(bank, &to_match)?;
                if normalized_binding != normalized_target {
                    result = OracleUnifResult::NotUnifiable;
                }
            } else {
                result = match_pattern_var(bank, subst, &normalized_matcher, &to_match)?;
            }

            matcher_weight += term_standard_weight(&to_match) - DEFAULT_VWEIGHT;
            if matcher_weight > to_match_weight {
                result = OracleUnifResult::NotUnifiable;
            }
        } else if matcher.is_top_level_db_var() {
            result = match_pattern_db_pair(&mut jobs, &matcher, &to_match);
        } else if matcher.f_code() == to_match.f_code() {
            if bank.signature().is_polymorphic(matcher.f_code())
                && matcher.arity() != 0
                && required_arg(&matcher, 0).type_() != required_arg(&to_match, 0).type_()
            {
                result = OracleUnifResult::NotUnifiable;
                continue;
            }
            assert_eq!(matcher.arity(), to_match.arity());
            for index in (0..matcher.arity()).rev() {
                push_match_pair(
                    &mut jobs,
                    required_arg(&matcher, index),
                    required_arg(&to_match, index),
                );
            }
        } else {
            result = OracleUnifResult::NotUnifiable;
        }
    }

    Ok(result)
}

fn match_pattern_db_pair(
    jobs: &mut Vec<Term>,
    matcher: &Term,
    to_match: &Term,
) -> OracleUnifResult {
    if matcher.is_db_var() {
        return if to_match.is_db_var() && matcher.f_code() == to_match.f_code() {
            OracleUnifResult::Unifiable
        } else {
            OracleUnifResult::NotUnifiable
        };
    }

    if !to_match.is_applied_db_var()
        || required_arg(matcher, 0) != required_arg(to_match, 0)
        || matcher.arity() != to_match.arity()
    {
        return OracleUnifResult::NotUnifiable;
    }
    for index in (1..matcher.arity()).rev() {
        push_match_pair(
            jobs,
            required_arg(matcher, index),
            required_arg(to_match, index),
        );
    }
    OracleUnifResult::Unifiable
}

fn match_pattern_var(
    bank: &mut TermBank,
    subst: &mut Substitution,
    matcher: &Term,
    to_match: &Term,
) -> Result<OracleUnifResult, Diagnostic> {
    assert!(matcher.is_top_level_free_var());
    if matcher.is_free_var() {
        return if term_is_db_closed(to_match) {
            subst.add_binding(matcher, to_match);
            Ok(OracleUnifResult::Unifiable)
        } else {
            Ok(OracleUnifResult::NotUnifiable)
        };
    }

    let db_map = db_var_map(bank, matcher);
    let mut result = OracleUnifResult::Unifiable;
    let Some(remapped) = remap_pattern_variables(bank, &db_map, to_match, 0, &mut result)? else {
        return Ok(result);
    };
    let binding = close_with_type_prefix(bank, &visible_arg_types(matcher), &remapped)?;
    subst.add_binding(&free_var_head(matcher), &binding);
    Ok(result)
}

fn remap_pattern_variables(
    bank: &mut TermBank,
    db_map: &DbVarMap,
    term: &Term,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Result<Option<Term>, Diagnostic> {
    if !term.has_db_subterm() {
        return Ok(Some(term.clone()));
    }

    let term = if term.is_applied_free_var() {
        let Some(normalized) = normalize_pattern_app_var(bank, term)? else {
            *result = OracleUnifResult::NotInFragment;
            return Ok(None);
        };
        normalized
    } else {
        term.clone()
    };

    if term.is_lambda() {
        let mut binders = Vec::new();
        let matrix = unfold_lambda(&term, &mut binders);
        let next_depth =
            depth + FunCode::try_from(binders.len()).expect("lambda prefix length fits in FunCode");
        let Some(mut remapped) =
            remap_pattern_variables(bank, db_map, &matrix, next_depth, result)?
        else {
            return Ok(None);
        };
        if remapped == matrix {
            return Ok(Some(term));
        }
        while let Some(binder) = binders.pop() {
            let binder_type = binder.type_().expect("lambda binder must have a type");
            remapped = close_with_db_var(bank, &binder_type, &remapped)?;
        }
        return Ok(Some(remapped));
    }

    if term.is_db_var() {
        if term.f_code() < depth {
            return Ok(Some(term));
        }
        let Some(replacement) = db_map.get(&(term.f_code() - depth)) else {
            *result = OracleUnifResult::NotUnifiable;
            return Ok(None);
        };
        let replacement_type = replacement
            .type_()
            .expect("DB replacement variable must have a type");
        return Ok(Some(
            bank.request_db_var(&replacement_type, replacement.f_code() + depth),
        ));
    }

    let copy = Term::top_copy_without_args(&term);
    let mut changed = false;
    for index in 0..term.arity() {
        let old_arg = required_arg(&term, index);
        let Some(new_arg) = remap_pattern_variables(bank, db_map, &old_arg, depth, result)? else {
            return Ok(None);
        };
        changed |= old_arg != new_arg;
        copy.set_argument(index, new_arg);
    }
    if changed {
        bank.term_top_insert(copy).map(Some)
    } else {
        Ok(Some(term))
    }
}

fn push_match_pair(jobs: &mut Vec<Term>, matcher: Term, to_match: Term) {
    jobs.push(matcher);
    jobs.push(to_match);
}

fn pop_match_pair(jobs: &mut Vec<Term>) -> Option<(Term, Term)> {
    let to_match = jobs.pop()?;
    let matcher = jobs
        .pop()
        .expect("pattern-match stack stores complete pairs");
    Some((matcher, to_match))
}

/// C `SubstComputeMguPattern`.
///
/// # Errors
///
/// Returns diagnostics from weak-head normalization, eta reduction, DB shifting,
/// fresh-variable application, lambda closure, or term-bank insertion.
///
/// # Panics
///
/// Panics on malformed lambda/application cells or untyped variables, matching
/// the C implementation's internal assertions.
pub fn subst_compute_mgu_pattern(
    bank: &mut TermBank,
    t1: &Term,
    t2: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    if t1.type_() != t2.type_() {
        return Ok(OracleUnifResult::NotUnifiable);
    }

    let backtrack = subst.len();
    let mut result = OracleUnifResult::Unifiable;
    let mut jobs = PQueue::new();
    jobs.store(t1.clone());
    jobs.store(t2.clone());
    bank.vars().set_v_counts_to_used();

    while !jobs.is_empty() && result == OracleUnifResult::Unifiable {
        let mut right = whnf_deref(bank, &jobs.get_last())?;
        let mut left = whnf_deref(bank, &jobs.get_last())?;
        (left, right) = prune_lambda_prefix(bank, left, right)?;

        if left == right {
            continue;
        }
        if term_is_ground(&left) && term_is_ground(&right) {
            result = OracleUnifResult::NotUnifiable;
            break;
        }

        assert_eq!(left.type_(), right.type_());

        if right.is_top_level_free_var() {
            std::mem::swap(&mut left, &mut right);
        }

        result = if left.is_top_level_free_var() {
            process_flex_pair(bank, subst, &left, &right)?
        } else if left.is_phony_app() {
            process_phony_pair(bank, &mut jobs, &left, &right)
        } else if left.is_db_var() {
            process_left_db_pair(&left, &right)
        } else if right.is_db_var() {
            assert!(
                !left.is_phony_app() && !left.is_db_var(),
                "right DB variable branch expects rigid left side"
            );
            OracleUnifResult::NotUnifiable
        } else if left.f_code() == right.f_code() {
            process_same_rigid_symbol(bank, &mut jobs, &left, &right)
        } else {
            OracleUnifResult::NotUnifiable
        };
    }

    if result != OracleUnifResult::Unifiable {
        subst.backtrack_to_pos(backtrack);
    }
    Ok(result)
}

pub(crate) fn prune_lambda_prefix(
    bank: &mut TermBank,
    mut left: Term,
    mut right: Term,
) -> Result<(Term, Term), Diagnostic> {
    while left.is_lambda() && right.is_lambda() {
        let left_binder = required_arg(&left, 0);
        let right_binder = required_arg(&right, 0);
        assert_eq!(left_binder.type_(), right_binder.type_());
        left = required_arg(&left, 1);
        right = required_arg(&right, 1);
    }

    if left.is_lambda() {
        eta_expand_on_the_fly(bank, &left, &right)
    } else if right.is_lambda() {
        let (right, left) = eta_expand_on_the_fly(bank, &right, &left)?;
        Ok((left, right))
    } else {
        Ok((left, right))
    }
}

fn process_flex_pair(
    bank: &mut TermBank,
    subst: &mut Substitution,
    left: &Term,
    right: &Term,
) -> Result<OracleUnifResult, Diagnostic> {
    if right.is_top_level_free_var() {
        if free_var_head(left) == free_var_head(right) {
            flex_flex_same(bank, left, right, subst)
        } else {
            flex_flex_diff(bank, left, right, subst)
        }
    } else {
        flex_rigid(bank, left, right, subst)
    }
}

fn process_phony_pair(
    bank: &TermBank,
    jobs: &mut PQueue<Term>,
    left: &Term,
    right: &Term,
) -> OracleUnifResult {
    if !right.is_phony_app() {
        return OracleUnifResult::NotUnifiable;
    }
    let left_head = required_arg(left, 0);
    let right_head = required_arg(right, 0);
    assert!(left_head.is_db_var());
    assert!(right_head.is_db_var());
    if left_head != right_head {
        return OracleUnifResult::NotUnifiable;
    }
    assert_eq!(left.arity(), right.arity());
    schedule_jobs(bank, jobs, left, right, 1, left.arity().saturating_sub(1));
    OracleUnifResult::Unifiable
}

fn process_left_db_pair(left: &Term, right: &Term) -> OracleUnifResult {
    if right.is_db_var() && left.f_code() == right.f_code() {
        OracleUnifResult::Unifiable
    } else {
        OracleUnifResult::NotUnifiable
    }
}

fn process_same_rigid_symbol(
    bank: &TermBank,
    jobs: &mut PQueue<Term>,
    left: &Term,
    right: &Term,
) -> OracleUnifResult {
    assert_eq!(left.arity(), right.arity());
    if bank.signature().is_polymorphic(left.f_code())
        && left.arity() != 0
        && required_arg(left, 0).type_() != required_arg(right, 0).type_()
    {
        return OracleUnifResult::NotUnifiable;
    }
    schedule_jobs(bank, jobs, left, right, 0, left.arity());
    OracleUnifResult::Unifiable
}

fn flex_rigid(
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    let Some(s) = normalize_pattern_app_var(bank, s)? else {
        return Ok(OracleUnifResult::NotInFragment);
    };

    if s.is_free_var() && t.is_pattern() && term_is_db_closed(t) {
        if occur_check(t, &s) {
            Ok(OracleUnifResult::NotUnifiable)
        } else {
            subst.add_binding(&s, t);
            Ok(OracleUnifResult::Unifiable)
        }
    } else {
        let s_var = free_var_head(&s);
        let db_map = db_var_map(bank, &s);
        let mut result = OracleUnifResult::Unifiable;
        let s_binding_matrix = solve_flex_rigid(bank, &s_var, &db_map, t, subst, 0, &mut result)?;
        if result == OracleUnifResult::Unifiable {
            let s_binding_matrix =
                s_binding_matrix.expect("successful flex-rigid returns a binding matrix");
            let s_prefix = visible_arg_types(&s);
            let binding = close_with_type_prefix(bank, &s_prefix, &s_binding_matrix)?;
            subst.add_binding(&s_var, &binding);
        }
        Ok(result)
    }
}

fn solve_flex_rigid(
    bank: &mut TermBank,
    s_var: &Term,
    db_map: &DbVarMap,
    term: &Term,
    subst: &mut Substitution,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Result<Option<Term>, Diagnostic> {
    let term = whnf_deref(bank, term)?;
    if term.is_free_var() {
        assert!(term.binding().is_none());
        if term == *s_var {
            *result = OracleUnifResult::NotUnifiable;
            Ok(None)
        } else {
            Ok(Some(term))
        }
    } else if term.is_db_var() {
        Ok(solve_db_var(bank, db_map, &term, depth, result))
    } else if term.is_lambda() {
        solve_flex_rigid_lambda(bank, s_var, db_map, &term, subst, depth, result)
    } else if term.is_applied_free_var() {
        solve_flex_rigid_app_var(bank, s_var, db_map, &term, subst, depth, result)
    } else {
        solve_flex_rigid_rigid(bank, s_var, db_map, &term, subst, depth, result)
    }
}

fn solve_db_var(
    bank: &mut TermBank,
    db_map: &DbVarMap,
    term: &Term,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Option<Term> {
    if term.f_code() < depth {
        return Some(term.clone());
    }
    let Some(replacement) = db_map.get(&(term.f_code() - depth)) else {
        *result = OracleUnifResult::NotUnifiable;
        return None;
    };
    let replacement_type = replacement
        .type_()
        .expect("DB replacement variable must have a type");
    Some(bank.request_db_var(&replacement_type, replacement.f_code() + depth))
}

fn solve_flex_rigid_lambda(
    bank: &mut TermBank,
    s_var: &Term,
    db_map: &DbVarMap,
    term: &Term,
    subst: &mut Substitution,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Result<Option<Term>, Diagnostic> {
    let mut dbvars = Vec::new();
    let matrix = unfold_lambda(term, &mut dbvars);
    let next_depth =
        depth + FunCode::try_from(dbvars.len()).expect("lambda prefix length fits in FunCode");
    let new_matrix = solve_flex_rigid(bank, s_var, db_map, &matrix, subst, next_depth, result)?;
    if *result != OracleUnifResult::Unifiable {
        return Ok(None);
    }
    let new_matrix = new_matrix.expect("successful lambda solve returns a matrix");
    if matrix == new_matrix {
        return Ok(Some(term.clone()));
    }

    let mut rebuilt = new_matrix;
    while let Some(dbvar) = dbvars.pop() {
        let dbvar_type = dbvar.type_().expect("lambda binder must have a type");
        rebuilt = close_with_db_var(bank, &dbvar_type, &rebuilt)?;
    }
    Ok(Some(rebuilt))
}

fn solve_flex_rigid_app_var(
    bank: &mut TermBank,
    s_var: &Term,
    db_map: &DbVarMap,
    term: &Term,
    subst: &mut Substitution,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Result<Option<Term>, Diagnostic> {
    let Some(term) = normalize_pattern_app_var(bank, term)? else {
        *result = OracleUnifResult::NotInFragment;
        return Ok(None);
    };
    if free_var_head(&term) == *s_var {
        *result = OracleUnifResult::NotUnifiable;
        return Ok(None);
    }

    let num_args = num_actual_args(&term);
    let mut t_dbs = Vec::new();
    let mut s_dbs = Vec::new();
    for index in 1..term.arity() {
        let arg = required_arg(&term, index);
        assert!(arg.is_db_var());
        let arg_type = arg.type_().expect("DB variable must have a type");
        if arg.f_code() < depth {
            t_dbs.push(bank.request_db_var(&arg_type, reverse_visible_index(num_args, index)));
            s_dbs.push(bank.request_db_var(&arg_type, arg.f_code()));
        } else if let Some(db_val) = db_map.get(&(arg.f_code() - depth)) {
            let db_val_type = db_val.type_().expect("mapped DB variable must have a type");
            t_dbs.push(bank.request_db_var(&arg_type, reverse_visible_index(num_args, index)));
            s_dbs.push(bank.request_db_var(&db_val_type, db_val.f_code() + depth));
        }
    }

    let t_var = free_var_head(&term);
    let return_type = term.type_().expect("applied variable must have a type");
    let t_binding_matrix = fresh_var_with_args(bank, &t_dbs, &return_type)?;
    let t_prefix = visible_arg_types(&term);
    let t_binding = close_with_type_prefix(bank, &t_prefix, &t_binding_matrix)?;
    subst.add_binding(&t_var, &t_binding);
    Ok(Some(apply_terms(
        bank,
        &free_var_head(&t_binding_matrix),
        &s_dbs,
    )?))
}

fn solve_flex_rigid_rigid(
    bank: &mut TermBank,
    s_var: &Term,
    db_map: &DbVarMap,
    term: &Term,
    subst: &mut Substitution,
    depth: FunCode,
    result: &mut OracleUnifResult,
) -> Result<Option<Term>, Diagnostic> {
    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for index in 0..copy.arity() {
        if *result != OracleUnifResult::Unifiable {
            break;
        }
        let old_arg = required_arg(term, index);
        let Some(new_arg) = solve_flex_rigid(bank, s_var, db_map, &old_arg, subst, depth, result)?
        else {
            continue;
        };
        changed |= old_arg != new_arg;
        copy.set_argument(index, new_arg);
    }

    if *result != OracleUnifResult::Unifiable {
        return Ok(None);
    }
    if changed {
        bank.term_top_insert(copy).map(Some)
    } else {
        Ok(Some(term.clone()))
    }
}

fn flex_flex_diff(
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    let Some(s) = normalize_pattern_app_var(bank, s)? else {
        return Ok(OracleUnifResult::NotInFragment);
    };
    let Some(t) = normalize_pattern_app_var(bank, t)? else {
        return Ok(OracleUnifResult::NotInFragment);
    };

    let db_map = db_var_map(bank, &s);
    let mut t_dbs = Vec::new();
    let mut s_dbs = Vec::new();
    let num_args = num_actual_args(&t);
    for index in 1..t.arity() {
        let arg = required_arg(&t, index);
        assert!(arg.is_db_var());
        if let Some(db_val) = db_map.get(&arg.f_code()) {
            let arg_type = arg.type_().expect("DB argument must have a type");
            t_dbs.push(bank.request_db_var(&arg_type, reverse_visible_index(num_args, index)));
            s_dbs.push(db_val.clone());
        }
    }

    let t_var = free_var_head(&t);
    let t_type = t.type_().expect("applied variable must have a type");
    let t_binding_matrix = fresh_var_with_args(bank, &t_dbs, &t_type)?;
    let t_prefix = visible_arg_types(&t);
    let t_binding = close_with_type_prefix(bank, &t_prefix, &t_binding_matrix)?;
    subst.add_binding(&t_var, &t_binding);

    let s_var = free_var_head(&s);
    let s_binding_matrix = apply_terms(bank, &free_var_head(&t_binding_matrix), &s_dbs)?;
    let s_prefix = visible_arg_types(&s);
    let s_binding = close_with_type_prefix(bank, &s_prefix, &s_binding_matrix)?;
    subst.add_binding(&s_var, &s_binding);

    Ok(OracleUnifResult::Unifiable)
}

fn flex_flex_same(
    bank: &mut TermBank,
    s: &Term,
    t: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    assert!(s.is_top_level_free_var());
    assert!(t.is_top_level_free_var());
    if s.is_free_var() {
        assert!(t.is_free_var());
        assert_eq!(s, t);
        return Ok(OracleUnifResult::Unifiable);
    }

    assert!(t.is_applied_free_var());
    let Some(s) = normalize_pattern_app_var(bank, s)? else {
        return Ok(OracleUnifResult::NotInFragment);
    };
    let Some(t) = normalize_pattern_app_var(bank, t)? else {
        return Ok(OracleUnifResult::NotInFragment);
    };
    let var = free_var_head(&s);
    assert_eq!(var, free_var_head(&t));
    let var_type = var.type_().expect("pattern variable must have a type");
    assert!(var_type.is_arrow());
    let max_args = type_get_max_arity(&var_type);
    assert_eq!(s.arity(), t.arity());

    let mut db_args = Vec::new();
    for index in 1..s.arity() {
        let s_arg = required_arg(&s, index);
        if s_arg == required_arg(&t, index) {
            let arg_type = s_arg.type_().expect("DB argument must have a type");
            let db_index = FunCode::try_from(max_args).expect("arity fits in FunCode")
                - FunCode::try_from(index).expect("argument index fits in FunCode")
                - 1;
            db_args.push(bank.request_db_var(&arg_type, db_index));
        }
    }

    let return_type = get_ret_type(&var_type);
    let matrix = fresh_var_with_args(bank, &db_args, &return_type)?;
    let binding = close_with_type_prefix(bank, &var_type.args()[..max_args], &matrix)?;
    subst.add_binding(&var, &binding);
    Ok(OracleUnifResult::Unifiable)
}

fn db_var_map(bank: &mut TermBank, term: &Term) -> DbVarMap {
    assert!(term.is_top_level_free_var());
    let mut result = BTreeMap::new();
    let num_args = num_actual_args(term);
    for index in 1..term.arity() {
        let arg = required_arg(term, index);
        assert!(arg.is_db_var());
        let arg_type = arg.type_().expect("DB argument must have a type");
        result.insert(
            arg.f_code(),
            bank.request_db_var(&arg_type, reverse_visible_index(num_args, index)),
        );
    }
    result
}

fn normalize_pattern_app_var(bank: &mut TermBank, term: &Term) -> Result<Option<Term>, Diagnostic> {
    if term.is_free_var() {
        return Ok(Some(term.clone()));
    }
    assert!(term.is_applied_free_var(), "expected applied free variable");

    let reduced = lambda_eta_reduce_db(bank, term)?;
    if reduced.is_free_var() {
        return Ok(Some(reduced));
    }
    if !reduced.is_applied_free_var() {
        return Ok(None);
    }

    let mut args = Vec::with_capacity(reduced.arity());
    for index in 0..reduced.arity() {
        let arg = required_arg(&reduced, index);
        if index != 0 && !arg.is_db_var() {
            return Ok(None);
        }
        args.push(arg);
    }

    if term_array_no_duplicates(&args) {
        Ok(Some(reduced))
    } else {
        Ok(None)
    }
}

fn eta_expand_on_the_fly(
    bank: &mut TermBank,
    lambda: &Term,
    non_lambda: &Term,
) -> Result<(Term, Term), Diagnostic> {
    assert!(lambda.is_lambda());
    assert!(!non_lambda.is_lambda());

    let mut dbvars = Vec::new();
    let lambda_body = unfold_lambda(lambda, &mut dbvars);
    let prefix_len = dbvars.len();
    for (index, dbvar) in dbvars.iter_mut().enumerate() {
        let dbvar_type = dbvar.type_().expect("lambda binder must have a type");
        let db_index = FunCode::try_from(prefix_len - index - 1)
            .expect("lambda prefix length fits in FunCode");
        *dbvar = bank.request_db_var(&dbvar_type, db_index);
    }

    let shift = FunCode::try_from(prefix_len).expect("lambda prefix length fits in FunCode");
    let shifted = shift_db(bank, non_lambda, shift)?;
    let expanded = apply_terms(bank, &shifted, &dbvars)?;
    Ok((lambda_body, expanded))
}

fn schedule_jobs(
    _bank: &TermBank,
    jobs: &mut PQueue<Term>,
    left: &Term,
    right: &Term,
    start: usize,
    size: usize,
) {
    for index in start..start + size {
        let left_arg = required_arg(left, index);
        let right_arg = required_arg(right, index);
        if is_rigid(&left_arg) && is_rigid(&right_arg) {
            jobs.store(left_arg);
            jobs.store(right_arg);
        } else {
            jobs.bury(left_arg);
            jobs.bury(right_arg);
        }
    }
}

fn is_rigid(term: &Term) -> bool {
    term.f_code() > 0 || !term.is_top_level_free_var()
}

fn visible_arg_types(term: &Term) -> Vec<Type> {
    (1..term.arity())
        .map(|index| {
            required_arg(term, index)
                .type_()
                .expect("visible argument must have a type")
        })
        .collect()
}

fn num_actual_args(term: &Term) -> usize {
    if term.is_applied_free_var() {
        term.arity() - 1
    } else {
        term.arity()
    }
}

fn reverse_visible_index(num_args: usize, index: usize) -> FunCode {
    FunCode::try_from(num_args).expect("argument count fits in FunCode")
        - FunCode::try_from(index).expect("argument index fits in FunCode")
}

fn free_var_head(term: &Term) -> Term {
    assert!(term.is_top_level_free_var());
    if term.is_applied_free_var() {
        required_arg(term, 0)
    } else {
        term.clone()
    }
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("pattern MGU term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::{subst_compute_match_pattern, subst_compute_mgu_pattern};
    use crate::terms::lambda::apply_terms;
    use crate::terms::match_mgu::OracleUnifResult;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = arg.type_().expect("argument has a type");
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, unary_type)
            .unwrap();
        let head = bank.create_const_term(f_code).unwrap();
        apply_terms(bank, &head, std::slice::from_ref(arg)).unwrap()
    }

    fn shared_unary_type(bank: &mut TermBank, type_: &Type) -> Type {
        bank.signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]))
    }

    #[test]
    fn pattern_mgu_binds_applied_variable_to_closed_rigid_term() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let flex_type = shared_unary_type(&mut bank, &type_);
        let flex = bank.vars().get_fresh_var(&flex_type);
        let db0 = bank.request_db_var(&type_, 0);
        let applied = apply_terms(&mut bank, &flex, std::slice::from_ref(&db0)).unwrap();
        let rigid = typed_const(&mut bank, "pattern_mgu_a", &type_);
        let mut subst = Substitution::new();

        let result = subst_compute_mgu_pattern(&mut bank, &applied, &rigid, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::Unifiable);
        let binding = flex.binding().expect("pattern MGU binds flex head");
        assert!(binding.is_lambda());
        assert_eq!(subst.bindings(), std::slice::from_ref(&flex));
    }

    #[test]
    fn pattern_mgu_rejects_non_pattern_applied_variable_argument() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let flex_type = shared_unary_type(&mut bank, &type_);
        let flex = bank.vars().get_fresh_var(&flex_type);
        let rigid = typed_const(&mut bank, "pattern_mgu_np_a", &type_);
        let applied = apply_terms(&mut bank, &flex, std::slice::from_ref(&rigid)).unwrap();
        let mut subst = Substitution::new();

        let result = subst_compute_mgu_pattern(&mut bank, &applied, &rigid, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotInFragment);
        assert!(subst.is_empty());
        assert!(flex.binding().is_none());
    }

    #[test]
    fn pattern_mgu_backtracks_on_occurs_check_failure() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let variable = bank.vars().get_fresh_var(&type_);
        let containing = typed_unary(&mut bank, "pattern_mgu_occurs_f", &variable);
        let mut subst = Substitution::new();

        let result =
            subst_compute_mgu_pattern(&mut bank, &variable, &containing, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotUnifiable);
        assert!(subst.is_empty());
        assert!(variable.binding().is_none());
    }

    #[test]
    fn pattern_mgu_flex_flex_different_heads_binds_both_variables() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let flex_type = shared_unary_type(&mut bank, &type_);
        let left_head = bank.vars().get_fresh_var(&flex_type);
        let right_head = bank.vars().get_fresh_var(&flex_type);
        let db0 = bank.request_db_var(&type_, 0);
        let left = apply_terms(&mut bank, &left_head, std::slice::from_ref(&db0)).unwrap();
        let right = apply_terms(&mut bank, &right_head, std::slice::from_ref(&db0)).unwrap();
        let mut subst = Substitution::new();

        let result = subst_compute_mgu_pattern(&mut bank, &left, &right, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::Unifiable);
        assert_eq!(subst.len(), 2);
        assert!(left_head
            .binding()
            .is_some_and(|binding| binding.is_lambda()));
        assert!(right_head
            .binding()
            .is_some_and(|binding| binding.is_lambda()));
    }

    #[test]
    fn pattern_match_binds_only_the_directed_applied_variable() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let flex_type = shared_unary_type(&mut bank, &type_);
        let flex = bank.vars().get_fresh_var(&flex_type);
        let db0 = bank.request_db_var(&type_, 0);
        let matcher = apply_terms(&mut bank, &flex, std::slice::from_ref(&db0)).unwrap();
        let target = typed_unary(&mut bank, "pattern_match_target", &db0);
        let mut subst = Substitution::new();

        let result = subst_compute_match_pattern(&mut bank, &matcher, &target, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::Unifiable);
        assert_eq!(subst.bindings(), std::slice::from_ref(&flex));
        assert!(flex.binding().is_some_and(|binding| binding.is_lambda()));
    }

    #[test]
    fn pattern_match_backtracks_when_target_uses_unmapped_db_variable() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let flex_type = shared_unary_type(&mut bank, &type_);
        let flex = bank.vars().get_fresh_var(&flex_type);
        let db0 = bank.request_db_var(&type_, 0);
        let db1 = bank.request_db_var(&type_, 1);
        let matcher = apply_terms(&mut bank, &flex, std::slice::from_ref(&db0)).unwrap();
        let target = typed_unary(&mut bank, "pattern_match_unmapped", &db1);
        let mut subst = Substitution::new();

        let result = subst_compute_match_pattern(&mut bank, &matcher, &target, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotUnifiable);
        assert!(subst.is_empty());
        assert!(flex.binding().is_none());
    }
}
