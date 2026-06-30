use crate::basics::error::{Diagnostic, ErrorCode};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
use crate::terms::simpletypes::{arrow_type_flattened, type_drop_first_arg, Type};
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_is_db_closed, term_is_ground};
use crate::terms::termtypes::Term;
use std::collections::BTreeMap;

/// Applies arguments to `head`, preserving C `ApplyTerms` sharing through the term bank.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion or type inference fails.
///
/// # Panics
///
/// Panics if an application violates the typed higher-order application shape.
pub fn apply_terms(bank: &mut TermBank, head: &Term, args: &[Term]) -> Result<Term, Diagnostic> {
    if args.is_empty() {
        return Ok(head.clone());
    }

    let mut result_type = head.type_().expect("application head must have a type");
    for arg in args {
        let arg_type = arg.type_().expect("application argument must have a type");
        assert!(
            result_type.is_arrow(),
            "application head type must be an arrow"
        );
        assert_eq!(
            result_type.args()[0],
            arg_type,
            "application argument type mismatch"
        );
        result_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(type_drop_first_arg(&result_type));
    }

    let applied = if head.is_any_var() || head.is_lambda() {
        let applied = Term::top_alloc(SIG_PHONY_APP_CODE, args.len() + 1);
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
    applied.set_type(Some(result_type));
    bank.term_top_insert(applied)
}

/// Builds a DB lambda with one binder, matching C `CloseWithDBVar`.
///
/// # Errors
///
/// Returns a diagnostic if the body is untyped or if term-bank insertion fails.
///
/// # Panics
///
/// Panics if `body` is not a shared bank term.
pub fn close_with_db_var(
    bank: &mut TermBank,
    binder_type: &Type,
    body: &Term,
) -> Result<Term, Diagnostic> {
    assert!(body.is_shared(), "lambda body must be a shared bank term");
    let body_type = body.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "CloseWithDBVar requires a typed lambda body",
        )
    })?;
    let binder = bank.request_db_var(binder_type, 0);
    let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 2);
    lambda.set_argument(0, binder);
    lambda.set_argument(1, body.clone());
    let lambda_type =
        bank.signature_mut()
            .type_bank_mut()
            .insert_type_shared(arrow_type_flattened(
                std::slice::from_ref(binder_type),
                &body_type,
            ));
    lambda.set_type(Some(lambda_type));
    bank.term_top_insert(lambda)
}

/// Closes `matrix` with a DB-lambda prefix for the supplied argument types.
///
/// # Errors
///
/// Returns a diagnostic if any lambda construction fails.
pub fn close_with_type_prefix(
    bank: &mut TermBank,
    types: &[Type],
    matrix: &Term,
) -> Result<Term, Diagnostic> {
    let mut result = matrix.clone();
    for type_ in types.iter().rev() {
        result = close_with_db_var(bank, type_, &result)?;
    }
    Ok(result)
}

/// Peels a DB-lambda prefix, matching C `UnfoldLambda`.
///
/// Binders are appended in descent order. The last appended binder is the top
/// of C's `PStack`.
///
/// # Panics
///
/// Panics if a lambda cell has an uninitialized binder or matrix argument.
#[must_use]
pub fn unfold_lambda(lambda: &Term, var_stack: &mut Vec<Term>) -> Term {
    let mut current = lambda.clone();
    while current.is_lambda() {
        let binder = current.argument(0).expect("lambda binder is uninitialized");
        var_stack.push(binder);
        current = current.argument(1).expect("lambda matrix is uninitialized");
    }
    current
}

/// Abstracts the free-variable prefix over `matrix`, matching C `AbstractVars`.
///
/// Variables later in `var_prefix` are closer to the top of C's stack and
/// receive lower De Bruijn indexes.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding the abstracted matrix or lambda prefix
/// fails.
///
/// # Panics
///
/// Panics if `matrix` is not DB-closed, if a prefix entry is not a typed free
/// variable, or if a rebuilt lambda violates the DB-lambda shape.
pub fn abstract_vars(
    bank: &mut TermBank,
    matrix: &Term,
    var_prefix: &[Term],
) -> Result<Term, Diagnostic> {
    assert!(
        term_is_db_closed(matrix),
        "AbstractVars requires a DB-closed matrix"
    );
    let mut bindings = BTreeMap::new();
    let prefix_len = var_prefix.len();
    for (index, variable) in var_prefix.iter().enumerate() {
        assert!(
            variable.is_free_var(),
            "AbstractVars prefix entries must be free variables"
        );
        let type_ = variable
            .type_()
            .expect("AbstractVars prefix variables must be typed");
        let db_index = i64::try_from(prefix_len - index - 1)
            .expect("AbstractVars prefix length fits in FunCode");
        bindings.insert(variable.f_code(), (db_index, type_));
    }

    let mut result = replace_free_vars(bank, matrix, &bindings, 0)?;
    for variable in var_prefix.iter().rev() {
        let type_ = variable
            .type_()
            .expect("AbstractVars prefix variables must be typed");
        result = close_with_db_var(bank, &type_, &result)?;
    }
    assert!(
        term_is_db_closed(&result),
        "AbstractVars result must be DB-closed"
    );
    Ok(result)
}

/// Shifts loose DB variables by `shift_val`, matching C `ShiftDB`.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding changed top cells fails.
///
/// # Panics
///
/// Panics if a shifted DB index would become negative.
pub fn shift_db(bank: &mut TermBank, term: &Term, shift_val: FunCode) -> Result<Term, Diagnostic> {
    if shift_val == 0 {
        return Ok(term.clone());
    }
    do_shift_db(bank, term, shift_val, 0)
}

fn do_shift_db(
    bank: &mut TermBank,
    term: &Term,
    shift_val: FunCode,
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    if term.is_db_var() {
        if term.f_code() >= depth {
            let new_index = term
                .f_code()
                .checked_add(shift_val)
                .expect("DB variable shift fits in FunCode");
            assert!(
                new_index >= 0,
                "DB variable shift produced a negative index"
            );
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(bank.request_db_var(&type_, new_index));
        }
        return Ok(term.clone());
    }

    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_DB_LAMBDA_CODE,
            "DB shifting expects DB lambdas"
        );
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let shifted = do_shift_db(bank, &matrix, shift_val, depth + 1)?;
        if shifted == matrix {
            return Ok(term.clone());
        }
        let binder_type = binder.type_().expect("DB lambda binder must have a type");
        return close_with_db_var(bank, &binder_type, &shifted);
    }

    if term.arity() == 0 || !term.has_db_subterm() {
        return Ok(term.clone());
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let shifted = do_shift_db(bank, &arg, shift_val, depth)?;
        if shifted != arg {
            changed = true;
        }
        copy.set_argument(index, shifted);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

fn replace_free_vars(
    bank: &mut TermBank,
    term: &Term,
    bindings: &BTreeMap<FunCode, (FunCode, Type)>,
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    if term_is_ground(term) {
        return Ok(term.clone());
    }
    if term.is_free_var() {
        let Some((db_index, type_)) = bindings.get(&term.f_code()) else {
            return Ok(term.clone());
        };
        return Ok(bank.request_db_var(type_, db_index + depth));
    }
    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_DB_LAMBDA_CODE,
            "free-variable replacement expects DB lambdas"
        );
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let new_matrix = replace_free_vars(bank, &matrix, bindings, depth + 1)?;
        if new_matrix == matrix {
            return Ok(term.clone());
        }
        let binder_type = binder.type_().expect("DB lambda binder must have a type");
        return close_with_db_var(bank, &binder_type, &new_matrix);
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let replaced = replace_free_vars(bank, &arg, bindings, depth)?;
        if replaced != arg {
            changed = true;
        }
        copy.set_argument(index, replaced);
    }
    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

/// Computes one weak-head beta step for a DB-lambda application.
///
/// # Errors
///
/// Returns a diagnostic if rebuilding the reduced term fails.
///
/// # Panics
///
/// Panics if the reducible term violates the DB-lambda application invariant.
pub fn whnf_step(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if !term.is_phony_app() || !term.argument(0).is_some_and(|head| head.is_lambda()) {
        return Ok(term.clone());
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
            matrix.f_code(),
            SIG_DB_LAMBDA_CODE,
            "WHNF reduction expects DB lambdas"
        );
        let binder = matrix
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let target = term
            .argument(next_arg)
            .unwrap_or_else(|| panic!("application argument {next_arg} is uninitialized"));
        assert_eq!(
            target.type_(),
            binder.type_(),
            "application argument type must match lambda binder"
        );
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

    let mut new_matrix = replace_bound_vars(bank, &matrix, &bindings, 0)?;
    if num_remaining != 0 {
        let mut rest = Vec::with_capacity(num_remaining);
        for index in next_arg..term.arity() {
            rest.push(
                term.argument(index)
                    .unwrap_or_else(|| panic!("application argument {index} is uninitialized")),
            );
        }
        new_matrix = apply_terms(bank, &new_matrix, &rest)?;
    }
    Ok(new_matrix)
}

fn replace_bound_vars(
    bank: &mut TermBank,
    term: &Term,
    bindings: &[Option<Term>],
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    let total_bound = FunCode::try_from(bindings.len()).unwrap_or(FunCode::MAX);
    assert!(
        total_bound > 0,
        "bound-variable replacement requires bindings"
    );

    if term.is_db_var() {
        if term.f_code() < depth {
            return Ok(term.clone());
        }
        let loose_index = term.f_code() - depth;
        if loose_index < total_bound {
            let binding = bindings[usize::try_from(loose_index).expect("DB index fits usize")]
                .as_ref()
                .expect("WHNF binding slot is initialized");
            return shift_db(bank, binding, depth);
        }
        let type_ = term.type_().expect("DB variable must have a type");
        return Ok(bank.request_db_var(&type_, term.f_code() - total_bound));
    }

    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_DB_LAMBDA_CODE,
            "bound-variable replacement expects DB lambdas"
        );
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let new_matrix = replace_bound_vars(bank, &matrix, bindings, depth + 1)?;
        if new_matrix == matrix {
            return Ok(term.clone());
        }
        let binder_type = binder.type_().expect("DB lambda binder must have a type");
        return close_with_db_var(bank, &binder_type, &new_matrix);
    }

    if term.arity() == 0 || !term.has_db_subterm() {
        return Ok(term.clone());
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let replaced = replace_bound_vars(bank, &arg, bindings, depth)?;
        if replaced != arg {
            changed = true;
        }
        copy.set_argument(index, replaced);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

/// Beta-normalizes DB-lambda terms, matching the beta half of C `LambdaNormalizeDB`.
///
/// # Errors
///
/// Returns a diagnostic if a reduced term cannot be rebuilt through the term bank.
///
/// # Panics
///
/// Panics if the beta-normalizer leaves a beta-reducible application behind.
pub fn beta_normalize_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if !term.is_beta_reducible() {
        return Ok(term.clone());
    }

    let result = do_beta_normalize_db(bank, term)?;
    if result.f_code() == bank.signature().eqn_code()
        && result.arity() == 2
        && result.argument(1).as_ref() == Some(bank.true_term())
        && result.argument(0).as_ref() != Some(bank.true_term())
    {
        let Some(left) = result.argument(0) else {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "encoded equality left argument is uninitialized",
            ));
        };
        if left.f_code() > 0 && bank.signature().is_logical_symbol(left.f_code()) {
            return Ok(left);
        }
    }
    assert!(
        !result.is_beta_reducible(),
        "beta normalization must remove beta-reducible applications"
    );
    Ok(result)
}

fn do_beta_normalize_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if term.is_phony_app() && term.argument(0).is_some_and(|head| head.is_lambda()) {
        let mut result = whnf_step(bank, term)?;
        if result.is_beta_reducible() {
            result = do_beta_normalize_db(bank, &result)?;
        }
        return Ok(result);
    }

    if term.arity() == 0 || !term.is_beta_reducible() {
        return Ok(term.clone());
    }

    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_DB_LAMBDA_CODE,
            "beta normalization expects DB lambdas"
        );
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let reduced_matrix = do_beta_normalize_db(bank, &matrix)?;
        if reduced_matrix == matrix {
            return Ok(term.clone());
        }
        let binder_type = binder.type_().expect("DB lambda binder must have a type");
        return close_with_db_var(bank, &binder_type, &reduced_matrix);
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let reduced = do_beta_normalize_db(bank, &arg)?;
        if reduced != arg {
            changed = true;
        }
        copy.set_argument(index, reduced);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_terms, beta_normalize_db, close_with_type_prefix, shift_db, unfold_lambda};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    #[test]
    fn close_prefix_and_beta_normalize_apply_retained_db_argument() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let g = bank.vars().get_fresh_var(&unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &g, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, &[i_type.clone(), i_type.clone()], &matrix).unwrap();
        let a = typed_const(&mut bank, "lambda_a");
        let b = typed_const(&mut bank, "lambda_b");
        let applied = apply_terms(&mut bank, &lambda, &[a, b.clone()]).unwrap();

        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert!(normalized.is_applied_free_var());
        assert_eq!(normalized.argument(0).as_ref(), Some(&g));
        assert_eq!(normalized.argument(1).as_ref(), Some(&b));
    }

    #[test]
    fn unfold_lambda_peels_prefix_in_c_push_order() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let user_sort_code = bank
            .signature_mut()
            .type_bank_mut()
            .define_simple_sort("$unfold_lambda_user")
            .unwrap();
        let user_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(user_sort_code));
        let matrix = typed_const(&mut bank, "unfold_lambda_matrix");
        let lambda =
            close_with_type_prefix(&mut bank, &[i_type.clone(), user_type.clone()], &matrix)
                .unwrap();
        let mut vars = Vec::new();

        let body = unfold_lambda(&lambda, &mut vars);

        assert_eq!(body, matrix);
        assert_eq!(vars.len(), 2);
        assert!(vars[0].is_db_var());
        assert!(vars[1].is_db_var());
        assert_eq!(vars[0].type_(), Some(i_type));
        assert_eq!(vars[1].type_(), Some(user_type));

        let returned_matrix = unfold_lambda(&matrix, &mut vars);
        assert_eq!(returned_matrix, matrix);
        assert_eq!(vars.len(), 2);
    }

    #[test]
    fn shift_db_only_moves_loose_indices() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let loose = bank.request_db_var(&i_type, 1);
        let closed_body = bank.request_db_var(&i_type, 0);
        let closed =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &closed_body).unwrap();
        let pair_code = bank.signature_mut().insert_id("lambda_pair", 2, false);
        bank.signature_mut()
            .declare_final_type(
                pair_code,
                alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]),
            )
            .unwrap();
        let pair = Term::top_alloc(pair_code, 2);
        pair.set_type(Some(i_type.clone()));
        pair.set_argument(0, loose);
        pair.set_argument(1, closed);
        let pair = bank.insert(&pair, DerefType::Never).unwrap();

        let shifted = shift_db(&mut bank, &pair, 2).unwrap();

        assert_eq!(shifted.argument(0).unwrap().f_code(), 3);
        let shifted_lambda = shifted.argument(1).unwrap();
        assert_eq!(shifted_lambda.argument(1).unwrap().f_code(), 0);
    }
}
