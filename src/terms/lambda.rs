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

/// Flattens additional arguments onto an already-applied head, matching C `FlattenApps`.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion or type inference fails.
///
/// # Panics
///
/// Panics if `head` is a variable or lambda, if a copied head argument is
/// uninitialized, or if the rebuilt top cell violates term-bank insertion
/// invariants.
pub fn flatten_apps(
    bank: &mut TermBank,
    head: &Term,
    args: &[Term],
    result_type: &Type,
) -> Result<Term, Diagnostic> {
    assert!(
        !head.is_any_var(),
        "FlattenApps expects an already-applied top-cell head"
    );
    assert!(
        !head.is_lambda(),
        "FlattenApps cannot flatten a lambda top cell"
    );

    let flattened = Term::top_alloc(head.f_code(), head.arity() + args.len());
    for (index, arg) in head.argument_clones().into_iter().enumerate() {
        flattened.set_argument(
            index,
            arg.unwrap_or_else(|| panic!("head argument {index} is uninitialized")),
        );
    }
    for (index, arg) in args.iter().enumerate() {
        flattened.set_argument(head.arity() + index, arg.clone());
    }
    flattened.set_type(Some(result_type.clone()));
    bank.term_top_insert(flattened)
}

/// Drops trailing application arguments, matching C `drop_args`.
///
/// # Errors
///
/// Returns a diagnostic if the shortened application cannot be inserted into
/// the term bank.
///
/// # Panics
///
/// Panics if `args_to_drop` is larger than the term arity, if a phony
/// application would lose its head, if required term types are missing, or if a
/// copied argument is uninitialized.
pub fn drop_args(
    bank: &mut TermBank,
    term: &Term,
    args_to_drop: usize,
) -> Result<Term, Diagnostic> {
    assert!(
        args_to_drop <= term.arity(),
        "cannot drop more arguments than a term has"
    );
    assert!(
        !term.is_phony_app() || args_to_drop < term.arity(),
        "cannot drop the head of a phony application"
    );

    if args_to_drop == 0 {
        return Ok(term.clone());
    }

    if term.is_phony_app() && term.arity() == args_to_drop + 1 {
        return Ok(term
            .argument(0)
            .expect("phony application head is uninitialized"));
    }

    let kept_arity = term.arity() - args_to_drop;
    let dropped_types = (kept_arity..term.arity())
        .map(|index| {
            term.argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
                .type_()
                .unwrap_or_else(|| panic!("dropped argument {index} is untyped"))
        })
        .collect::<Vec<_>>();
    let term_type = term
        .type_()
        .expect("term to drop arguments from must have a type");
    let result_type = bank
        .signature_mut()
        .type_bank_mut()
        .insert_type_shared(arrow_type_flattened(&dropped_types, &term_type));
    let result = Term::top_alloc(term.f_code(), kept_arity);
    result.set_type(Some(result_type));
    for index in 0..kept_arity {
        result.set_argument(
            index,
            term.argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized")),
        );
    }
    bank.term_top_insert(result)
}

/// Finds the minimum loose De Bruijn index below `term`, matching C `find_min_db`.
///
/// Returns `None` for C's `DB_NOT_FOUND`.
///
/// # Panics
///
/// Panics if a lambda matrix or traversed argument is uninitialized.
#[must_use]
pub fn find_min_db(term: &Term, depth: FunCode) -> Option<FunCode> {
    if term.is_db_var() {
        return (term.f_code() >= depth).then_some(term.f_code() - depth);
    }
    if term.is_lambda() {
        let matrix = term.argument(1).expect("lambda matrix is uninitialized");
        return find_min_db(&matrix, depth + 1);
    }
    if !term.has_db_subterm() {
        return None;
    }

    let mut result = None;
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        if let Some(min_db) = find_min_db(&arg, depth) {
            result = Some(result.map_or(min_db, |current: FunCode| current.min(min_db)));
        }
    }
    result
}

/// Performs one top-level eta-reduction step, matching C `reduce_eta_top_level`.
///
/// # Errors
///
/// Returns a diagnostic if dropping arguments, shifting DB indexes, or closing
/// the retained lambda prefix fails.
///
/// # Panics
///
/// Panics if lambda/application cells are malformed or if required term types
/// are missing.
pub fn reduce_eta_top_level(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let mut bound_vars = Vec::new();
    let matrix = unfold_lambda(term, &mut bound_vars);
    let mut result = term.clone();

    if term.is_lambda()
        && matrix.arity() > 0
        && matrix
            .argument(matrix.arity() - 1)
            .is_some_and(|arg| arg.is_db_var() && arg.f_code() == 0)
    {
        let matrix_arity =
            i64::try_from(matrix.arity()).expect("term arity fits C-compatible long");
        let mut last_db = matrix_arity - 1;
        let bound_count =
            i64::try_from(bound_vars.len()).expect("lambda prefix length fits C-compatible long");
        let phony_limit = i64::from(matrix.is_phony_app());
        let left_limit = (matrix_arity - bound_count).max(phony_limit);

        while last_db >= left_limit {
            let expected_db = matrix_arity - 1 - last_db;
            let arg = matrix
                .argument(usize::try_from(last_db).expect("non-negative argument index"))
                .expect("term argument is uninitialized");
            if !(arg.is_db_var() && arg.f_code() == expected_db) {
                break;
            }
            last_db -= 1;
        }
        last_db += 1;

        assert!(last_db >= 0, "eta suffix scan must leave a valid start");
        assert!(
            last_db < matrix_arity,
            "eta suffix start must stay inside arity"
        );
        let last_db_index = usize::try_from(last_db).expect("non-negative argument index");

        let mut min_db = None;
        for index in 0..last_db_index {
            let arg = matrix
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            if let Some(arg_min_db) = find_min_db(&arg, 0) {
                min_db =
                    Some(min_db.map_or(arg_min_db, |current: FunCode| current.min(arg_min_db)));
            }
        }

        if min_db != Some(0) {
            let suffix_db = matrix
                .argument(last_db_index)
                .expect("eta suffix argument is uninitialized")
                .f_code();
            let suffix_drop =
                usize::try_from(suffix_db + 1).expect("eta suffix DB index is non-negative");
            let to_drop = min_db.map_or(suffix_drop, |min_db| {
                usize::try_from(min_db)
                    .expect("loose DB index is non-negative")
                    .min(suffix_drop)
            });
            let dropped = drop_args(bank, &matrix, to_drop)?;
            result = shift_db(
                bank,
                &dropped,
                -i64::try_from(to_drop).expect("argument count fits C-compatible long"),
            )?;

            for _ in 0..to_drop {
                bound_vars
                    .pop()
                    .expect("eta reduction drops an available binder");
            }
            while let Some(binder) = bound_vars.pop() {
                let binder_type = binder.type_().expect("lambda binder must have a type");
                result = close_with_db_var(bank, &binder_type, &result)?;
            }
        }
    }

    assert_eq!(
        result.type_(),
        term.type_(),
        "eta reduction preserves term type"
    );
    Ok(result)
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
    use super::{
        apply_terms, beta_normalize_db, close_with_type_prefix, flatten_apps, shift_db,
        unfold_lambda,
    };
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type};
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

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
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
    fn flatten_apps_appends_arguments_to_regular_head() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let full_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]);
        let partial_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let f_code = bank.signature_mut().insert_id("lambda_flatten_f", 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, full_type)
            .unwrap();
        let a = typed_const(&mut bank, "lambda_flatten_a");
        let b = typed_const(&mut bank, "lambda_flatten_b");
        let head = Term::top_alloc(f_code, 1);
        head.set_type(Some(partial_type));
        head.set_argument(0, a.clone());
        let head = bank.term_top_insert(head).unwrap();

        let flattened = flatten_apps(&mut bank, &head, std::slice::from_ref(&b), &i_type).unwrap();

        assert_eq!(flattened.f_code(), f_code);
        assert_eq!(flattened.arity(), 2);
        assert_eq!(flattened.argument(0).as_ref(), Some(&a));
        assert_eq!(flattened.argument(1).as_ref(), Some(&b));
        assert_eq!(flattened.type_(), Some(i_type));
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
    fn reduce_eta_top_level_drops_multi_binder_suffix() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let binary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "eta_reduce_f", binary_type);
        let db1 = bank.request_db_var(&i_type, 1);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &f, &[db1, db0]).unwrap();
        let lambda = close_with_type_prefix(&mut bank, &[i_type.clone(), i_type], &matrix).unwrap();

        let reduced = super::reduce_eta_top_level(&mut bank, &lambda).unwrap();

        assert_eq!(reduced, f);
    }

    #[test]
    fn reduce_eta_top_level_keeps_prefix_when_db_occurs_earlier() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let binary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]);
        let h = typed_const_with_type(&mut bank, "eta_keep_h", binary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &h, &[db0.clone(), db0]).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();

        let reduced = super::reduce_eta_top_level(&mut bank, &lambda).unwrap();

        assert_eq!(reduced, lambda);
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
