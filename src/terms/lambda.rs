use crate::basics::error::{Diagnostic, ErrorCode};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{
    SIG_DB_LAMBDA_CODE, SIG_FALSE_CODE, SIG_ITE_CODE, SIG_LET_CODE, SIG_NAMED_LAMBDA_CODE,
    SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
};
use crate::terms::simpletypes::{
    arrow_type_flattened, type_drop_first_arg, type_get_max_arity, type_is_predicate, Type,
};
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_is_db_closed, term_is_ground};
use crate::terms::termtypes::{term_deref, DerefType, Term, TP_PRED_POS};
use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

pub type TermNormalizer = fn(&mut TermBank, &Term) -> Result<Term, Diagnostic>;

fn eta_normalizer_cell() -> &'static RwLock<TermNormalizer> {
    static ETA_NORMALIZER: OnceLock<RwLock<TermNormalizer>> = OnceLock::new();
    ETA_NORMALIZER.get_or_init(|| RwLock::new(lambda_eta_reduce_db))
}

/// Registers the eta normalizer used by `lambda_normalize_db`, matching C `SetEtaNormalizer`.
///
/// # Panics
///
/// Panics if another caller poisoned the process-wide normalizer lock.
pub fn set_eta_normalizer(normalizer: TermNormalizer) {
    *eta_normalizer_cell()
        .write()
        .expect("eta normalizer lock is poisoned") = normalizer;
}

/// Returns the eta normalizer used by `lambda_normalize_db`, matching C `GetEtaNormalizer`.
///
/// # Panics
///
/// Panics if another caller poisoned the process-wide normalizer lock.
#[must_use]
pub fn get_eta_normalizer() -> TermNormalizer {
    *eta_normalizer_cell()
        .read()
        .expect("eta normalizer lock is poisoned")
}

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

fn flatten_and_make_shared(bank: &mut TermBank, term: Term) -> Result<Term, Diagnostic> {
    assert!(!term.is_shared(), "flattening expects an unshared top cell");
    if term.is_phony_app()
        && term
            .argument(0)
            .is_some_and(|head| !(head.is_any_var() || head.is_lambda()))
    {
        let head = term
            .argument(0)
            .expect("phony application head is uninitialized");
        let args = (1..term.arity())
            .map(|index| {
                term.argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
            })
            .collect::<Vec<_>>();
        let result_type = term
            .type_()
            .expect("flattened phony application must have a type");
        flatten_apps(bank, &head, &args, &result_type)
    } else {
        bank.term_top_insert(term)
    }
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

/// Performs one top-level eta-expansion step, matching C `LambdaEtaExpandDBTopLevel`.
///
/// # Errors
///
/// Returns a diagnostic if applying the generated DB arguments, shifting DB
/// indexes, or closing the lambda prefix fails.
///
/// # Panics
///
/// Panics if `term` is untyped or if generated intermediate cells violate
/// lambda/application invariants.
pub fn lambda_eta_expand_db_top_level(
    bank: &mut TermBank,
    term: &Term,
) -> Result<Term, Diagnostic> {
    let term_type = term.type_().expect("eta expansion requires a typed term");
    if !term_type.is_arrow() || term.is_lambda() {
        return Ok(term.clone());
    }

    let num_args = type_get_max_arity(&term_type);
    let mut db_args = Vec::with_capacity(num_args);
    for (index, arg_type) in term_type.args()[..num_args].iter().enumerate() {
        let db_index =
            i64::try_from(num_args - index - 1).expect("DB index fits C-compatible long");
        let fresh_db = bank.request_db_var(arg_type, db_index);
        if fresh_db
            .type_()
            .expect("fresh DB variable must have a type")
            .is_arrow()
        {
            db_args.push(lambda_eta_expand_db_top_level(bank, &fresh_db)?);
        } else {
            db_args.push(fresh_db);
        }
    }

    let shifted = shift_db(
        bank,
        term,
        i64::try_from(num_args).expect("argument count fits C-compatible long"),
    )?;
    let mut result = apply_terms(bank, &shifted, &db_args)?;
    while let Some(db_arg) = db_args.pop() {
        let arg_type = db_arg.type_().expect("eta DB argument must have a type");
        result = close_with_db_var(bank, &arg_type, &result)?;
    }
    Ok(result)
}

fn do_eta_expand_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let result = if term.is_lambda() {
        let matrix = term.argument(1).expect("lambda matrix is uninitialized");
        if matrix.has_eta_expandable_subterm() {
            let new_matrix = do_eta_expand_db(bank, &matrix)?;
            assert_ne!(new_matrix, matrix, "eta-expansion flag must imply a change");
            let copy = Term::top_copy(term);
            copy.set_argument(1, new_matrix);
            bank.term_top_insert(copy)?
        } else {
            term.clone()
        }
    } else if term.arity() == 0 || !term.has_eta_expandable_subterm() {
        term.clone()
    } else {
        let copy = Term::top_copy_without_args(term);
        let start = usize::from(term.is_phony_app());
        if term.is_phony_app() {
            let head = term
                .argument(0)
                .expect("phony application head is uninitialized");
            copy.set_argument(0, head);
        }

        let mut changed = false;
        for index in start..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let expanded = do_eta_expand_db(bank, &arg)?;
            if expanded != arg {
                changed = true;
            }
            copy.set_argument(index, expanded);
        }

        if changed {
            bank.term_top_insert(copy)?
        } else {
            term.clone()
        }
    };

    lambda_eta_expand_db_top_level(bank, &result)
}

/// Performs eta-expansion on DB terms, matching C `LambdaEtaExpandDB`.
///
/// # Errors
///
/// Returns a diagnostic if a rebuilt eta-expanded term cannot be inserted into
/// the term bank.
///
/// # Panics
///
/// Panics if lambda/application cells are malformed or required term types are
/// missing.
pub fn lambda_eta_expand_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    if term.has_eta_expandable_subterm() {
        do_eta_expand_db(bank, term)
    } else {
        Ok(term.clone())
    }
}

fn do_eta_reduce_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let mut result = if term.arity() == 0 || !term.has_lambda_subterm() {
        term.clone()
    } else if term.is_lambda() {
        let mut bound_vars = Vec::new();
        let matrix = unfold_lambda(term, &mut bound_vars);
        let reduced = do_eta_reduce_db(bank, &matrix)?;
        let rebuilt = if matrix == reduced {
            term.clone()
        } else {
            let mut rebuilt = reduced;
            while let Some(binder) = bound_vars.pop() {
                let binder_type = binder.type_().expect("lambda binder must have a type");
                rebuilt = close_with_db_var(bank, &binder_type, &rebuilt)?;
            }
            rebuilt
        };
        reduce_eta_top_level(bank, &rebuilt)?
    } else {
        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for index in 0..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let reduced = do_eta_reduce_db(bank, &arg)?;
            if reduced != arg {
                changed = true;
            }
            copy.set_argument(index, reduced);
        }
        if changed {
            flatten_and_make_shared(bank, copy)?
        } else {
            term.clone()
        }
    };

    let (qall_code, qex_code) = {
        let sig = bank.signature();
        (sig.qall_code(), sig.qex_code())
    };
    if (result.f_code() == qall_code || result.f_code() == qex_code)
        && result.arity() == 1
        && result.argument(0).is_some_and(|arg| !arg.is_lambda())
    {
        let copy = Term::top_copy_without_args(&result);
        let arg = result
            .argument(0)
            .expect("quantifier argument is uninitialized");
        copy.set_argument(0, lambda_eta_expand_db_top_level(bank, &arg)?);
        result = bank.term_top_insert(copy)?;
    }

    Ok(result)
}

/// Performs eta-reduction on DB terms, matching C `LambdaEtaReduceDB`.
///
/// # Errors
///
/// Returns a diagnostic if a rebuilt eta-reduced term cannot be inserted into
/// the term bank.
///
/// # Panics
///
/// Panics if lambda/application cells are malformed or required term types are
/// missing.
pub fn lambda_eta_reduce_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let result = if term.has_lambda_subterm() {
        do_eta_reduce_db(bank, term)?
    } else {
        term.clone()
    };
    assert_eq!(
        result.type_(),
        term.type_(),
        "eta reduction preserves term type"
    );
    Ok(result)
}

/// Performs beta normalization followed by the registered eta normalizer,
/// matching C `LambdaNormalizeDB`.
///
/// # Errors
///
/// Returns a diagnostic if beta normalization or the registered eta normalizer
/// fails.
///
/// # Panics
///
/// Panics if the registered eta-normalizer lock is poisoned or if the
/// normalizer encounters malformed lambda/application cells.
pub fn lambda_normalize_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let beta_normal = beta_normalize_db(bank, term)?;
    get_eta_normalizer()(bank, &beta_normal)
}

fn do_named_to_db(
    bank: &mut TermBank,
    term: &Term,
    bindings: &mut BTreeMap<FunCode, (FunCode, Type)>,
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_NAMED_LAMBDA_CODE,
            "NamedToDB expects named lambda input"
        );

        let mut current = term.clone();
        let mut vars = Vec::new();
        while current.is_lambda() {
            assert_eq!(
                current.f_code(),
                SIG_NAMED_LAMBDA_CODE,
                "NamedToDB expects named lambda prefixes"
            );
            let binder = current
                .argument(0)
                .expect("named lambda binder is uninitialized");
            assert!(
                binder.is_free_var(),
                "named lambda binder must be a free variable"
            );
            vars.push(binder);
            current = current
                .argument(1)
                .expect("named lambda body is uninitialized");
        }

        let mut saved = Vec::with_capacity(vars.len());
        let mut next_depth = depth;
        for var in &vars {
            let var_type = var.type_().expect("named lambda binder must have a type");
            saved.push((
                var.f_code(),
                bindings.insert(var.f_code(), (next_depth, var_type)),
            ));
            next_depth += 1;
        }

        let mut result = do_named_to_db(bank, &current, bindings, next_depth)?;

        for var in vars.into_iter().rev() {
            let (f_code, previous) = saved
                .pop()
                .expect("saved named-lambda binding stack is balanced");
            assert_eq!(f_code, var.f_code());
            if let Some(previous) = previous {
                bindings.insert(f_code, previous);
            } else {
                bindings.remove(&f_code);
            }

            let var_type = var.type_().expect("named lambda binder must have a type");
            result = close_with_db_var(bank, &var_type, &result)?;
        }

        return Ok(result);
    }

    if term.is_free_var() {
        if let Some((binding_depth, type_)) = bindings.get(&term.f_code()) {
            assert_eq!(
                term.type_(),
                Some(type_.clone()),
                "named lambda binding type must match variable type"
            );
            let db_index = depth - binding_depth - 1;
            return Ok(bank.request_db_var(type_, db_index));
        }
        return Ok(term.clone());
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let converted = do_named_to_db(bank, &arg, bindings, depth)?;
        if converted != arg {
            changed = true;
        }
        copy.set_argument(index, converted);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

/// Converts closed named-lambda terms to DB-lambda terms and beta-normalizes them.
///
/// This matches C `NamedToDB`.
///
/// # Errors
///
/// Returns a diagnostic if conversion or beta normalization rebuilds a term
/// that cannot be inserted into the term bank.
///
/// # Panics
///
/// Panics if a lambda cell is malformed, if a named-lambda binder is not a
/// typed free variable, or if a named binder is encountered outside its scope.
pub fn named_to_db(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let converted = if term.has_lambda_subterm() {
        do_named_to_db(bank, term, &mut BTreeMap::new(), 0)?
    } else {
        term.clone()
    };
    beta_normalize_db(bank, &converted)
}

fn do_post_cnf_encode(
    bank: &mut TermBank,
    term: &Term,
    bindings: &mut BTreeMap<FunCode, (FunCode, Type)>,
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    if term.is_db_var() {
        return Ok(term.clone());
    }

    if term.is_free_var() {
        if let Some((binding_depth, type_)) = bindings.get(&term.f_code()) {
            assert_eq!(
                term.type_(),
                Some(type_.clone()),
                "post-CNF variable binding type must match variable type"
            );
            let db_index = depth - binding_depth - 1;
            return Ok(bank.request_db_var(type_, db_index));
        }
        return Ok(term.clone());
    }

    if !term.has_bool_subterm() && term_is_ground(term) {
        return Ok(term.clone());
    }

    if term.is_lambda() {
        assert_eq!(
            term.f_code(),
            SIG_DB_LAMBDA_CODE,
            "post-CNF encoding expects DB lambdas"
        );
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        assert!(binder.is_db_var(), "DB lambda binder must be a DB variable");
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let encoded_matrix = do_post_cnf_encode(bank, &matrix, bindings, depth + 1)?;
        if encoded_matrix == matrix {
            return Ok(term.clone());
        }
        let binder_type = binder.type_().expect("DB lambda binder must have a type");
        return close_with_db_var(bank, &binder_type, &encoded_matrix);
    }

    let (qall_code, qex_code, eqn_code) = {
        let sig = bank.signature();
        (sig.qall_code(), sig.qex_code(), sig.eqn_code())
    };
    if (term.f_code() == qall_code || term.f_code() == qex_code) && term.arity() == 2 {
        return post_cnf_encode_quantifier_prefix(bank, term, term.f_code(), bindings, depth);
    }

    if term.f_code() == eqn_code
        && term.arity() == 2
        && term.argument(1).as_ref() == Some(bank.true_term())
    {
        let left = term
            .argument(0)
            .expect("encoded predicate left argument is uninitialized");
        return do_post_cnf_encode(bank, &left, bindings, depth);
    }

    let copy = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let encoded = do_post_cnf_encode(bank, &arg, bindings, depth)?;
        if encoded != arg {
            changed = true;
        }
        copy.set_argument(index, encoded);
    }

    if changed {
        bank.term_top_insert(copy)
    } else {
        Ok(term.clone())
    }
}

fn post_cnf_encode_quantifier_prefix(
    bank: &mut TermBank,
    term: &Term,
    quantifier_code: FunCode,
    bindings: &mut BTreeMap<FunCode, (FunCode, Type)>,
    depth: FunCode,
) -> Result<Term, Diagnostic> {
    let mut current = term.clone();
    let mut vars = Vec::new();
    let mut saved = Vec::new();
    let mut next_depth = depth;
    while current.f_code() == quantifier_code {
        assert_eq!(
            current.arity(),
            2,
            "post-CNF quantifier prefix must use binary variable form"
        );
        let var = current
            .argument(0)
            .expect("quantifier variable is uninitialized");
        assert!(
            var.is_free_var(),
            "quantifier binder must be a free variable"
        );
        let var_type = var.type_().expect("quantifier binder must have a type");
        saved.push((
            var.f_code(),
            bindings.insert(var.f_code(), (next_depth, var_type)),
        ));
        vars.push(var);
        next_depth += 1;
        current = current
            .argument(1)
            .expect("quantifier matrix is uninitialized");
    }

    let shift = FunCode::try_from(vars.len()).expect("quantifier prefix length fits FunCode");
    let shifted_matrix = shift_db(bank, &current, shift)?;
    let mut result = do_post_cnf_encode(bank, &shifted_matrix, bindings, next_depth)?;

    for var in vars.into_iter().rev() {
        let (f_code, previous) = saved
            .pop()
            .expect("saved quantifier binding stack is balanced");
        assert_eq!(f_code, var.f_code());
        if let Some(previous) = previous {
            bindings.insert(f_code, previous);
        } else {
            bindings.remove(&f_code);
        }

        let var_type = var.type_().expect("quantifier binder must have a type");
        let lambda = close_with_db_var(bank, &var_type, &result)?;
        let unary_formula = Term::top_alloc(quantifier_code, 1);
        unary_formula.set_type(Some(bank.signature().type_bank().bool_type()));
        unary_formula.set_argument(0, lambda);
        result = bank.term_top_insert(unary_formula)?;
    }

    Ok(result)
}

/// Encodes post-CNF variable quantifiers as DB-lambda quantifiers and normalizes.
///
/// This matches C `PostCNFEncodeFormulas`.
///
/// # Errors
///
/// Returns a diagnostic if conversion or lambda normalization rebuilds a term
/// that cannot be inserted into the term bank.
///
/// # Panics
///
/// Panics if quantifier, lambda, or encoded-predicate cells are malformed, or
/// if a quantified variable is not typed.
pub fn post_cnf_encode_formulas(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let encoded = do_post_cnf_encode(bank, term, &mut BTreeMap::new(), 0)?;
    lambda_normalize_db(bank, &encoded)
}

fn encode_predicate_as_eqn(bank: &mut TermBank, formula: Term) -> Result<Term, Diagnostic> {
    let f_code = formula.f_code();
    let is_encodable = (formula.is_any_var()
        || !bank.signature().is_logical_symbol(f_code)
        || f_code == bank.signature().answer_code()
        || matches!(
            f_code,
            SIG_TRUE_CODE | SIG_FALSE_CODE | SIG_ITE_CODE | SIG_LET_CODE
        )
        || formula.is_phony_app())
        && formula.type_().as_ref().is_some_and(Type::is_bool);
    if !is_encodable {
        return Ok(formula);
    }

    let positive = formula.is_any_var() || f_code != SIG_FALSE_CODE;
    let left = if f_code == SIG_FALSE_CODE && !formula.is_any_var() {
        bank.true_term().clone()
    } else {
        formula
    };
    let right = bank.true_term().clone();
    let eqn_code = bank.signature_mut().get_eqn_code(positive);
    let encoded = Term::top_alloc(eqn_code, 2);
    encoded.set_type(Some(bank.signature().type_bank().bool_type()));
    encoded.set_argument(0, left);
    encoded.set_argument(1, right);
    bank.term_top_insert(encoded)
}

fn tformula_quantor_alloc(
    bank: &mut TermBank,
    quantifier: FunCode,
    var: Term,
    body: Term,
) -> Result<Term, Diagnostic> {
    let formula = Term::top_alloc(quantifier, 2);
    formula.set_type(Some(bank.signature().type_bank().bool_type()));
    if bank.signature().is_predicate(quantifier) {
        formula.set_prop(TP_PRED_POS);
    }
    formula.set_argument(0, var);
    formula.set_argument(1, body);
    bank.term_top_insert(formula)
}

fn tformula_fcode_alloc(
    bank: &mut TermBank,
    f_code: FunCode,
    left: Term,
    right: Term,
) -> Result<Term, Diagnostic> {
    let formula = Term::top_alloc(f_code, 2);
    formula.set_type(Some(bank.signature().type_bank().bool_type()));
    if bank.signature().is_predicate(f_code) {
        formula.set_prop(TP_PRED_POS);
    }
    formula.set_argument(0, left);
    formula.set_argument(1, right);
    bank.term_top_insert(formula)
}

/// Decodes formula terms into the CNF-facing representation and encodes atoms.
///
/// This matches C `DecodeFormulasForCNF`.
///
/// # Errors
///
/// Returns a diagnostic if application, weak-head reduction, predicate
/// encoding, or term-bank insertion fails.
///
/// # Panics
///
/// Panics if lambda-encoded quantifier arguments or lambda cells are malformed.
pub fn decode_formulas_for_cnf(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let (qall_code, qex_code) = {
        let sig = bank.signature();
        (sig.qall_code(), sig.qex_code())
    };

    let result = if (term.f_code() == qall_code || term.f_code() == qex_code) && term.arity() == 1 {
        let quantifier_arg = term
            .argument(0)
            .expect("lambda-encoded quantifier argument is uninitialized");
        let quantifier_type = quantifier_arg
            .type_()
            .expect("lambda-encoded quantifier argument must have a type");
        assert!(
            quantifier_type.is_arrow(),
            "lambda-encoded quantifier argument must have an arrow type"
        );
        assert_eq!(
            quantifier_type.arity(),
            2,
            "lambda-encoded quantifier argument must be unary"
        );
        assert!(
            type_is_predicate(&quantifier_type),
            "lambda-encoded quantifier argument must be predicate-typed"
        );
        let fresh_var = bank.vars().get_fresh_var(&quantifier_type.args()[0]);
        let applied = bank.term_apply_arg(&quantifier_arg, &fresh_var);
        let applied = bank.term_top_insert(applied)?;
        let matrix = whnf_step(bank, &applied)?;
        let decoded_matrix = decode_formulas_for_cnf(bank, &matrix)?;
        tformula_quantor_alloc(bank, term.f_code(), fresh_var, decoded_matrix)?
    } else if term.is_any_var() || term.arity() == 0 {
        term.clone()
    } else if term.is_lambda() {
        let binder = term
            .argument(0)
            .unwrap_or_else(|| panic!("lambda binder is uninitialized"));
        let matrix = term
            .argument(1)
            .unwrap_or_else(|| panic!("lambda matrix is uninitialized"));
        let decoded_matrix = decode_formulas_for_cnf(bank, &matrix)?;
        if decoded_matrix == matrix {
            term.clone()
        } else {
            let binder_type = binder.type_().expect("lambda binder must have a type");
            close_with_db_var(bank, &binder_type, &decoded_matrix)?
        }
    } else {
        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let decoded = decode_formulas_for_cnf(bank, &arg)?;
            if decoded != arg {
                changed = true;
            }
            copy.set_argument(index, decoded);
        }
        if changed {
            bank.term_top_insert(copy)?
        } else {
            term.clone()
        }
    };

    encode_predicate_as_eqn(bank, result)
}

fn lambda_eq_to_forall(bank: &mut TermBank, term: &Term) -> Result<Option<Term>, Diagnostic> {
    if !term.has_eq_neq() {
        return Ok(None);
    }

    let (eqn_code, neqn_code) = {
        let sig = bank.signature();
        (sig.eqn_code(), sig.neqn_code())
    };
    if !matches!(term.f_code(), code if code == eqn_code || code == neqn_code) || term.arity() != 2
    {
        return Ok(Some(term.clone()));
    }

    let left = term
        .argument(0)
        .expect("lambda equality left argument is uninitialized");
    let right = term
        .argument(1)
        .expect("lambda equality right argument is uninitialized");
    if !(left.is_lambda() || right.is_lambda()) {
        return Ok(Some(term.clone()));
    }

    let mut left_vars = Vec::new();
    let mut right_vars = Vec::new();
    let _ = unfold_lambda(&left, &mut left_vars);
    let _ = unfold_lambda(&right, &mut right_vars);
    assert!(
        !left_vars.is_empty() || !right_vars.is_empty(),
        "lambda equality must expose at least one binder"
    );
    let longer_vars = if left_vars.len() > right_vars.len() {
        &left_vars
    } else {
        &right_vars
    };

    let mut fresh_vars = Vec::with_capacity(longer_vars.len());
    let mut encoded_vars = Vec::with_capacity(longer_vars.len());
    for db_var in longer_vars {
        let type_ = db_var.type_().expect("lambda binder must have a type");
        let fresh_var = bank.vars().get_fresh_var(&type_);
        encoded_vars.push(encode_predicate_as_eqn(bank, fresh_var.clone())?);
        fresh_vars.push(fresh_var);
    }

    let applied_left = apply_terms(bank, &left, &encoded_vars)?;
    let applied_right = apply_terms(bank, &right, &encoded_vars)?;
    let normalized_left = beta_normalize_db(bank, &applied_left)?;
    let normalized_right = beta_normalize_db(bank, &applied_right)?;
    let bool_type = bank.signature().type_bank().bool_type();
    let mut result = if normalized_left.type_().as_ref() == Some(&bool_type) {
        let f_code = if term.f_code() == eqn_code {
            bank.signature().equiv_code()
        } else {
            bank.signature().xor_code()
        };
        let left = encode_predicate_as_eqn(bank, normalized_left)?;
        let right = encode_predicate_as_eqn(bank, normalized_right)?;
        tformula_fcode_alloc(bank, f_code, left, right)?
    } else {
        tformula_fcode_alloc(bank, term.f_code(), normalized_left, normalized_right)?
    };

    let universal = matches!(
        result.f_code(),
        code if code == bank.signature().eqn_code() || code == bank.signature().equiv_code()
    );
    while let Some(var) = fresh_vars.pop() {
        let quantifier = if universal {
            bank.signature().qall_code()
        } else {
            bank.signature().qex_code()
        };
        result = tformula_quantor_alloc(bank, quantifier, var, result)?;
    }
    Ok(Some(result))
}

/// Turns equations between lambda terms into quantified non-lambda formulas.
///
/// This matches C `LambdaToForall` over a single term-encoded formula.
///
/// # Errors
///
/// Returns a diagnostic if mapping, application, beta normalization, predicate
/// encoding, or formula rebuilding fails.
///
/// # Panics
///
/// Panics if equality or lambda cells are malformed, or if mapper invariants are
/// violated.
pub fn lambda_to_forall(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    bank.vars().set_v_counts_to_used();
    bank.map_term(term, &mut lambda_eq_to_forall)
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

/// Dereferences and weak-head beta-normalizes until the head is known.
///
/// This mirrors C `WHNF_deref`, but takes the owning `TermBank` explicitly
/// because Rust term handles do not yet retain owner-bank metadata.
///
/// # Errors
///
/// Returns a diagnostic if weak-head reduction or lambda-prefix rebuilding
/// fails.
///
/// # Panics
///
/// Panics if lambda/application cells are malformed or if required term types
/// are missing.
pub fn whnf_deref(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let mut deref = DerefType::Always;
    let term = term_deref(term, &mut deref);

    if term.is_phony_app() && term.argument(0).is_some_and(|head| head.is_lambda()) {
        let reduced = whnf_step(bank, &term)?;
        return whnf_deref(bank, &reduced);
    }

    if term.is_lambda() {
        let mut dbvars = Vec::new();
        let matrix = unfold_lambda(&term, &mut dbvars);
        let new_matrix = whnf_deref(bank, &matrix)?;
        if matrix == new_matrix {
            return Ok(term);
        }

        let mut result = new_matrix;
        while let Some(dbvar) = dbvars.pop() {
            let binder_type = dbvar.type_().expect("lambda binder must have a type");
            result = close_with_db_var(bank, &binder_type, &result)?;
        }
        return Ok(result);
    }

    Ok(term)
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
        apply_terms, beta_normalize_db, close_with_type_prefix, decode_formulas_for_cnf,
        flatten_apps, lambda_eta_expand_db, lambda_eta_expand_db_top_level, lambda_eta_reduce_db,
        lambda_normalize_db, lambda_to_forall, named_to_db, post_cnf_encode_formulas, shift_db,
        unfold_lambda, whnf_deref,
    };
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_DB_LAMBDA_CODE, SIG_NAMED_LAMBDA_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

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

    fn close_with_named_var(bank: &mut TermBank, binder: &Term, body: &Term) -> Term {
        let lambda = Term::top_alloc(SIG_NAMED_LAMBDA_CODE, 2);
        lambda.set_argument(0, binder.clone());
        lambda.set_argument(1, body.clone());
        bank.term_top_insert(lambda).unwrap()
    }

    fn encoded_predicate(bank: &mut TermBank, predicate: &Term) -> Term {
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let encoded = Term::top_alloc(eqn_code, 2);
        encoded.set_type(Some(bank.signature().type_bank().bool_type()));
        encoded.set_argument(0, predicate.clone());
        encoded.set_argument(1, bank.true_term().clone());
        bank.term_top_insert(encoded).unwrap()
    }

    fn quantified_var_formula(
        bank: &mut TermBank,
        quantifier: FunCode,
        var: &Term,
        body: &Term,
    ) -> Term {
        let formula = Term::top_alloc(quantifier, 2);
        formula.set_type(Some(bank.signature().type_bank().bool_type()));
        formula.set_argument(0, var.clone());
        formula.set_argument(1, body.clone());
        bank.term_top_insert(formula).unwrap()
    }

    fn quantified_lambda_formula(bank: &mut TermBank, quantifier: FunCode, body: &Term) -> Term {
        let formula = Term::top_alloc(quantifier, 1);
        formula.set_type(Some(bank.signature().type_bank().bool_type()));
        formula.set_argument(0, body.clone());
        bank.term_top_insert(formula).unwrap()
    }

    fn formula_binary(bank: &mut TermBank, f_code: FunCode, left: &Term, right: &Term) -> Term {
        let formula = Term::top_alloc(f_code, 2);
        formula.set_type(Some(bank.signature().type_bank().bool_type()));
        formula.set_argument(0, left.clone());
        formula.set_argument(1, right.clone());
        bank.term_top_insert(formula).unwrap()
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
    fn lambda_eta_expand_db_top_level_wraps_arrow_term() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let f = typed_const_with_type(&mut bank, "eta_expand_f", unary_type.clone());

        let expanded = lambda_eta_expand_db_top_level(&mut bank, &f).unwrap();

        assert!(expanded.is_lambda());
        assert_eq!(expanded.type_(), Some(unary_type));
        let binder = expanded.argument(0).unwrap();
        assert!(binder.is_db_var());
        assert_eq!(binder.type_(), Some(i_type));
        let matrix = expanded.argument(1).unwrap();
        assert_eq!(matrix.f_code(), f.f_code());
        assert_eq!(matrix.arity(), 1);
        let arg = matrix.argument(0).unwrap();
        assert!(arg.is_db_var());
        assert_eq!(arg.f_code(), 0);
    }

    #[test]
    fn lambda_eta_expand_db_recurses_into_ordinary_arguments() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let f = typed_const_with_type(&mut bank, "eta_expand_arg_f", unary_type.clone());
        let wrapper_type = alloc_arrow_type(vec![unary_type.clone(), i_type.clone()]);
        let wrapper_code = bank
            .signature_mut()
            .insert_id("eta_expand_wrapper", 1, false);
        bank.signature_mut()
            .declare_final_type(wrapper_code, wrapper_type)
            .unwrap();
        let wrapper = Term::top_alloc(wrapper_code, 1);
        wrapper.set_type(Some(i_type));
        wrapper.set_argument(0, f);
        let wrapper = bank.term_top_insert(wrapper).unwrap();

        let expanded = lambda_eta_expand_db(&mut bank, &wrapper).unwrap();

        assert_eq!(expanded.f_code(), wrapper_code);
        assert_eq!(expanded.arity(), 1);
        let expanded_arg = expanded.argument(0).unwrap();
        assert!(expanded_arg.is_lambda());
        assert_eq!(expanded_arg.type_(), Some(unary_type));
    }

    #[test]
    fn lambda_eta_reduce_db_flattens_reduced_phony_head() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "eta_phony_f", unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let a = typed_const(&mut bank, "eta_phony_a");
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&a)).unwrap();

        let reduced = lambda_eta_reduce_db(&mut bank, &applied).unwrap();

        assert!(!reduced.is_phony_app());
        assert_eq!(reduced.f_code(), f.f_code());
        assert_eq!(reduced.arity(), 1);
        assert_eq!(reduced.argument(0).as_ref(), Some(&a));
    }

    #[test]
    fn lambda_normalize_db_runs_beta_then_eta() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "lambda_normalize_f", unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let inner_matrix = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let inner_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &inner_matrix)
                .unwrap();
        let outer_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &inner_lambda)
                .unwrap();
        let a = typed_const(&mut bank, "lambda_normalize_a");
        let applied = apply_terms(&mut bank, &outer_lambda, std::slice::from_ref(&a)).unwrap();

        let normalized = lambda_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(normalized, f);
    }

    #[test]
    fn whnf_deref_reduces_nested_lambda_application_to_known_head() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "whnf_deref_a");
        let db0 = bank.request_db_var(&i_type, 0);
        let inner_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &db0).unwrap();
        let outer_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &inner_lambda)
                .unwrap();
        let applied = apply_terms(&mut bank, &outer_lambda, std::slice::from_ref(&a)).unwrap();

        let reduced = whnf_deref(&mut bank, &applied).unwrap();

        assert!(reduced.is_lambda());
        assert_eq!(reduced.argument(1).unwrap().f_code(), 0);
    }

    #[test]
    fn whnf_deref_rebuilds_lambda_when_matrix_reduces() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let a = typed_const(&mut bank, "whnf_deref_matrix_a");
        let db0 = bank.request_db_var(&i_type, 0);
        let inner_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &db0).unwrap();
        let applied_inner =
            apply_terms(&mut bank, &inner_lambda, std::slice::from_ref(&a)).unwrap();
        let wrapped =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &applied_inner)
                .unwrap();

        let reduced = whnf_deref(&mut bank, &wrapped).unwrap();

        assert!(reduced.is_lambda());
        assert_eq!(reduced.argument(1).as_ref(), Some(&a));
    }

    #[test]
    fn named_to_db_converts_nested_named_binders_to_db_indexes() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let binary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "named_to_db_f", binary_type);
        let x = bank.vars().get_fresh_var(&i_type);
        let y = bank.vars().get_fresh_var(&i_type);
        let matrix = apply_terms(&mut bank, &f, &[x.clone(), y.clone()]).unwrap();
        let inner = close_with_named_var(&mut bank, &y, &matrix);
        let named = close_with_named_var(&mut bank, &x, &inner);

        let converted = named_to_db(&mut bank, &named).unwrap();

        assert_eq!(converted.f_code(), SIG_DB_LAMBDA_CODE);
        let inner = converted.argument(1).unwrap();
        assert_eq!(inner.f_code(), SIG_DB_LAMBDA_CODE);
        let body = inner.argument(1).unwrap();
        assert_eq!(body.f_code(), f.f_code());
        assert_eq!(body.argument(0).unwrap().f_code(), 1);
        assert_eq!(body.argument(1).unwrap().f_code(), 0);
    }

    #[test]
    fn named_to_db_beta_normalizes_applied_named_lambda() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let x = bank.vars().get_fresh_var(&i_type);
        let named = close_with_named_var(&mut bank, &x, &x);
        let a = typed_const(&mut bank, "named_to_db_a");
        let applied = apply_terms(&mut bank, &named, std::slice::from_ref(&a)).unwrap();

        let converted = named_to_db(&mut bank, &applied).unwrap();

        assert_eq!(converted, a);
    }

    #[test]
    fn post_cnf_encode_formulas_turns_variable_quantifiers_into_db_lambdas() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), bool_type]);
        let p = typed_const_with_type(&mut bank, "post_cnf_p", predicate_type);
        let x = bank.vars().get_fresh_var(&i_type);
        let y = bank.vars().get_fresh_var(&i_type);
        let atom = apply_terms(&mut bank, &p, &[x.clone(), y.clone()]).unwrap();
        let encoded_atom = encoded_predicate(&mut bank, &atom);
        let qall_code = bank.signature().qall_code();
        let inner = quantified_var_formula(&mut bank, qall_code, &y, &encoded_atom);
        let quantified = quantified_var_formula(&mut bank, qall_code, &x, &inner);

        let converted = post_cnf_encode_formulas(&mut bank, &quantified).unwrap();

        assert_eq!(converted.f_code(), qall_code);
        assert_eq!(converted.arity(), 1);
        let outer_lambda = converted.argument(0).unwrap();
        assert_eq!(outer_lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let inner_quantifier = outer_lambda.argument(1).unwrap();
        assert_eq!(inner_quantifier.f_code(), qall_code);
        assert_eq!(inner_quantifier.arity(), 1);
        let inner_lambda = inner_quantifier.argument(0).unwrap();
        assert_eq!(inner_lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let body = inner_lambda.argument(1).unwrap();
        assert_eq!(body.f_code(), p.f_code());
        assert_eq!(body.argument(0).unwrap().f_code(), 1);
        assert_eq!(body.argument(1).unwrap().f_code(), 0);
    }

    #[test]
    fn post_cnf_encode_formulas_shifts_existing_loose_db_indices() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone(), bool_type]);
        let p = typed_const_with_type(&mut bank, "post_cnf_shift_p", predicate_type);
        let x = bank.vars().get_fresh_var(&i_type);
        let loose_db = bank.request_db_var(&i_type, 0);
        let atom = apply_terms(&mut bank, &p, &[x.clone(), loose_db]).unwrap();
        let encoded_atom = encoded_predicate(&mut bank, &atom);
        let qex_code = bank.signature().qex_code();
        let quantified = quantified_var_formula(&mut bank, qex_code, &x, &encoded_atom);

        let converted = post_cnf_encode_formulas(&mut bank, &quantified).unwrap();

        assert_eq!(converted.f_code(), qex_code);
        assert_eq!(converted.arity(), 1);
        let lambda = converted.argument(0).unwrap();
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let body = lambda.argument(1).unwrap();
        assert_eq!(body.f_code(), p.f_code());
        assert_eq!(body.argument(0).unwrap().f_code(), 0);
        assert_eq!(body.argument(1).unwrap().f_code(), 1);
    }

    #[test]
    fn decode_formulas_for_cnf_turns_lambda_quantifier_into_var_quantifier() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = alloc_arrow_type(vec![i_type.clone(), bool_type]);
        let p = typed_const_with_type(&mut bank, "decode_cnf_p", predicate_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let atom = apply_terms(&mut bank, &p, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &atom).unwrap();
        let qall_code = bank.signature().qall_code();
        let quantified = quantified_lambda_formula(&mut bank, qall_code, &lambda);

        let decoded = decode_formulas_for_cnf(&mut bank, &quantified).unwrap();

        assert_eq!(decoded.f_code(), qall_code);
        assert_eq!(decoded.arity(), 2);
        let binder = decoded.argument(0).unwrap();
        assert!(binder.is_free_var());
        assert_eq!(binder.type_(), Some(i_type));
        let matrix = decoded.argument(1).unwrap();
        assert_eq!(matrix.f_code(), bank.signature().eqn_code());
        assert_eq!(matrix.argument(1).as_ref(), Some(bank.true_term()));
        let predicate = matrix.argument(0).unwrap();
        assert_eq!(predicate.f_code(), p.f_code());
        assert_eq!(predicate.argument(0).as_ref(), Some(&binder));
    }

    #[test]
    fn decode_formulas_for_cnf_encodes_false_as_negative_truth_equality() {
        let mut bank = test_bank();
        let false_term = bank.false_term().clone();

        let decoded = decode_formulas_for_cnf(&mut bank, &false_term).unwrap();

        assert_eq!(decoded.f_code(), bank.signature().neqn_code());
        assert_eq!(decoded.argument(0).as_ref(), Some(bank.true_term()));
        assert_eq!(decoded.argument(1).as_ref(), Some(bank.true_term()));
    }

    #[test]
    fn lambda_to_forall_turns_function_lambda_equality_into_universal_formula() {
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let unary_type = alloc_arrow_type(vec![i_type.clone(), i_type.clone()]);
        let f = typed_const_with_type(&mut bank, "lambda_forall_f", unary_type.clone());
        let g = typed_const_with_type(&mut bank, "lambda_forall_g", unary_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let left_body = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let left_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &left_body).unwrap();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = formula_binary(&mut bank, eqn_code, &left_lambda, &g);

        let converted = lambda_to_forall(&mut bank, &equality).unwrap();

        assert_eq!(converted.f_code(), bank.signature().qall_code());
        assert_eq!(converted.arity(), 2);
        let binder = converted.argument(0).unwrap();
        assert!(binder.is_free_var());
        assert_eq!(binder.type_(), Some(i_type));
        let body = converted.argument(1).unwrap();
        assert_eq!(body.f_code(), eqn_code);
        assert_eq!(body.argument(0).unwrap().f_code(), f.f_code());
        assert_eq!(
            body.argument(0).unwrap().argument(0).as_ref(),
            Some(&binder)
        );
        assert_eq!(body.argument(1).unwrap().f_code(), g.f_code());
        assert_eq!(
            body.argument(1).unwrap().argument(0).as_ref(),
            Some(&binder)
        );
    }

    #[test]
    fn lambda_to_forall_turns_boolean_lambda_equality_into_equivalence() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let pred_type = alloc_arrow_type(vec![i_type.clone(), bool_type]);
        let p = typed_const_with_type(&mut bank, "lambda_forall_p", pred_type.clone());
        let q = typed_const_with_type(&mut bank, "lambda_forall_q", pred_type);
        let db0 = bank.request_db_var(&i_type, 0);
        let left_body = apply_terms(&mut bank, &p, std::slice::from_ref(&db0)).unwrap();
        let left_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &left_body).unwrap();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = formula_binary(&mut bank, eqn_code, &left_lambda, &q);

        let converted = lambda_to_forall(&mut bank, &equality).unwrap();

        assert_eq!(converted.f_code(), bank.signature().qall_code());
        let body = converted.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().equiv_code());
        let left_atom = body.argument(0).unwrap();
        let right_atom = body.argument(1).unwrap();
        assert_eq!(left_atom.f_code(), eqn_code);
        assert_eq!(right_atom.f_code(), eqn_code);
        assert_eq!(left_atom.argument(0).unwrap().f_code(), p.f_code());
        assert_eq!(right_atom.argument(0).unwrap().f_code(), q.f_code());
        assert_eq!(left_atom.argument(1).as_ref(), Some(bank.true_term()));
        assert_eq!(right_atom.argument(1).as_ref(), Some(bank.true_term()));
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
