use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::{
    PredefinedArithmeticSymbol, Signature, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_RATIONAL,
};
use crate::terms::simpletypes::{
    alloc_arrow_type, type_is_predicate, Type, TypeConsCode, ST_BOOL, ST_INTEGER, ST_RATIONAL,
    ST_REAL,
};
use crate::terms::termtypes::Term;
use crate::terms::typebanks::TypeBank;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeInferOptions {
    pub problem_type: ProblemType,
    pub app_encode: bool,
}

impl Default for TypeInferOptions {
    fn default() -> Self {
        Self {
            problem_type: problem_type(),
            app_encode: false,
        }
    }
}

#[must_use]
pub fn term_determine_type(term: &Term, type_: &Type, bank: &mut TypeBank) -> Option<Type> {
    debug_assert!(type_.is_arrow());
    let term_arity = term.arg_num();
    let type_args = type_.args();
    let consumed_args = type_.arity().checked_sub(1)?;

    match consumed_args.cmp(&term_arity) {
        Ordering::Equal => Some(type_args[term_arity].clone()),
        Ordering::Less => None,
        Ordering::Greater => {
            Some(bank.insert_type_shared(alloc_arrow_type(type_args[term_arity..].to_vec())))
        }
    }
}

#[must_use]
pub fn infer_return_sort(sig: &Signature, f_code: FunCode) -> Type {
    if sig.query_prop(f_code, FP_IS_INTEGER) && sig.distinct_props().intersects(FP_IS_INTEGER) {
        sig.type_bank().integer_type()
    } else if sig.query_prop(f_code, FP_IS_RATIONAL)
        && sig.distinct_props().intersects(FP_IS_RATIONAL)
    {
        sig.type_bank().rational_type()
    } else if sig.query_prop(f_code, FP_IS_FLOAT) && sig.distinct_props().intersects(FP_IS_FLOAT) {
        sig.type_bank().real_type()
    } else {
        sig.type_bank().default_type()
    }
}

#[must_use]
pub fn type_check_consistent(sig: &Signature, term: &Term) -> bool {
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            continue;
        }

        if let Some(symbol) = sig.predefined_arithmetic_symbol(current.f_code()) {
            if !predefined_arithmetic_occurrence_is_consistent(sig, &current, symbol) {
                return false;
            }
            stack.extend(current.argument_clones().into_iter().flatten());
            continue;
        }

        if sig.is_polymorphic(current.f_code()) {
            continue;
        }

        let Some(type_) = sig.get_type(current.f_code()) else {
            return false;
        };

        if type_.is_arrow() {
            let expected_arity = type_.arity().saturating_sub(1);
            if current.arity() != expected_arity
                || !term_has_type(&current, &type_.args()[type_.arity() - 1])
            {
                return false;
            }
        } else if current.arity() != 0 || !term_has_type(&current, type_) {
            return false;
        }

        for (index, expected_arg_type) in type_.args().iter().take(current.arity()).enumerate() {
            let Some(arg) = current.argument(index) else {
                return false;
            };
            if !term_has_type(&arg, expected_arg_type) {
                return false;
            }
            stack.push(arg);
        }
    }

    true
}

pub fn type_infer_sort(sig: &mut Signature, term: &Term) -> Result<(), Diagnostic> {
    type_infer_sort_with_options(sig, term, TypeInferOptions::default())
}

pub fn type_infer_sort_with_options(
    sig: &mut Signature,
    term: &Term,
    options: TypeInferOptions,
) -> Result<(), Diagnostic> {
    if term.is_free_var() {
        if term.type_().is_none() {
            term.set_type(Some(sig.type_bank().default_type()));
        }
        return Ok(());
    }

    let type_ = special_type_for_term(sig, term, options)?;
    if let Some(type_) = type_ {
        apply_known_type(sig, term, &type_, options)
    } else if term.is_lambda() {
        Ok(())
    } else {
        infer_and_declare_symbol_type(sig, term)
    }
}

/// Declares a non-variable term as a predicate occurrence and updates its sort.
///
/// # Panics
///
/// Panics if called for a free variable, matching the C assertion that
/// predicate declarations are for function-symbol terms.
pub fn type_declare_is_predicate(sig: &mut Signature, term: &Term) -> Result<(), Diagnostic> {
    assert!(
        !term.is_free_var(),
        "free variables cannot be declared as predicates"
    );
    if term.arity() != 0 && sig.get_type(term.f_code()).is_some_and(Type::is_bool) {
        let bool_type = sig.type_bank().bool_type();
        let mut args = Vec::with_capacity(term.arity() + 1);
        for index in 0..term.arity() {
            args.push(required_type(&required_arg(term, index)?)?);
        }
        args.push(bool_type);
        let predicate_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(args));
        sig.declare_type(term.f_code(), predicate_type)?;
    }
    sig.declare_is_predicate(term.f_code())?;
    term.set_type(Some(sig.type_bank().bool_type()));
    Ok(())
}

pub fn type_declare_is_not_predicate(
    sig: &mut Signature,
    term: &Term,
    options: TypeInferOptions,
) -> Result<(), Diagnostic> {
    if !term.is_any_var() && term.f_code() > sig.internal_symbols() {
        type_infer_sort_with_options(sig, term, options)?;
        sig.declare_is_function(term.f_code())?;
    }
    Ok(())
}

fn special_type_for_term(
    sig: &mut Signature,
    term: &Term,
    options: TypeInferOptions,
) -> Result<Option<Type>, Diagnostic> {
    if term.is_phony_app() {
        return Ok(required_arg(term, 0)?.type_());
    }

    if term.is_lambda() {
        assert!(
            term.arity() == 2,
            "lambda terms must carry variable and body arguments"
        );
        let variable = required_arg(term, 0)?;
        let body = required_arg(term, 1)?;
        let variable_type = required_type(&variable)?;
        let body_type = required_type(&body)?;
        let type_ = sig.type_bank_mut().insert_type_shared(
            crate::terms::simpletypes::arrow_type_flattened(
                std::slice::from_ref(&variable_type),
                &body_type,
            ),
        );
        term.set_type(Some(type_));
        return Ok(None);
    }

    if let Some(symbol) = sig.predefined_arithmetic_symbol(term.f_code()) {
        return predefined_arithmetic_occurrence_type(sig, term, symbol, options).map(Some);
    }

    if term.f_code() == sig.eqn_code() || term.f_code() == sig.neqn_code() {
        if term.arity() == 0 {
            return Err(type_error("Equality must have at least one argument"));
        }
        let arg_type = required_type(&required_arg(term, 0)?)?;
        let bool_type = sig.type_bank().bool_type();
        let type_ = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                arg_type.clone(),
                arg_type,
                bool_type,
            ]));
        return Ok(Some(type_));
    }

    if term.f_code() == sig.qex_code() || term.f_code() == sig.qall_code() {
        if term.arity() == 0 {
            return Err(type_error("Quantifiers must have at least one argument"));
        }
        let first_arg = required_arg(term, 0)?;
        if first_arg.is_free_var() {
            let arg_type = required_type(&first_arg)?;
            let bool_type = sig.type_bank().bool_type();
            let type_ = sig
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    arg_type,
                    bool_type.clone(),
                    bool_type,
                ]));
            return Ok(Some(type_));
        }

        let arg_type = required_type(&first_arg)?;
        if !arg_type.is_arrow() || !type_is_predicate(&arg_type) {
            return Err(type_error("Wrong encoding of quantifier arguments"));
        }
        let bool_type = sig.type_bank().bool_type();
        let type_ = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type]));
        return Ok(Some(type_));
    }

    Ok(sig.get_type(term.f_code()).cloned())
}

fn apply_known_type(
    sig: &mut Signature,
    term: &Term,
    type_: &Type,
    options: TypeInferOptions,
) -> Result<(), Diagnostic> {
    if type_.is_arrow() {
        if options.problem_type == ProblemType::FirstOrder
            && !options.app_encode
            && term.arity() != type_.arity() - 1
        {
            return Err(type_error("Type error"));
        }

        if term.is_phony_app() {
            for index in 1..term.arity() {
                let expected_index = index - 1;
                let Some(expected_type) = type_.args().get(expected_index) else {
                    return Err(type_error("Type error"));
                };
                let arg = required_arg(term, index)?;
                if !term_has_type(&arg, expected_type) {
                    return Err(type_error("Type error"));
                }
            }
        } else if sig.is_fixed_type(term.f_code())
            || sig.predefined_arithmetic_symbol(term.f_code()).is_some()
        {
            for index in 0..term.arity() {
                let Some(expected_type) = type_.args().get(index) else {
                    return Err(type_error("Type error"));
                };
                let arg = required_arg(term, index)?;
                if !term_has_type(&arg, expected_type) {
                    return Err(type_error("Type error"));
                }
            }
        }

        let Some(term_type) = term_determine_type(term, type_, sig.type_bank_mut()) else {
            return Err(type_error("Type error"));
        };
        term.set_type(Some(term_type));
    } else if term.arity() != 0 {
        return Err(type_error("Type error"));
    } else {
        term.set_type(Some(type_.clone()));
    }
    Ok(())
}

fn infer_and_declare_symbol_type(sig: &mut Signature, term: &Term) -> Result<(), Diagnostic> {
    let sort = infer_return_sort(sig, term.f_code());
    let type_ = if term.arity() == 0 {
        sort.clone()
    } else {
        let mut args = Vec::with_capacity(term.arity() + 1);
        for index in 0..term.arity() {
            args.push(required_type(&required_arg(term, index)?)?);
        }
        args.push(sort.clone());
        sig.type_bank_mut()
            .insert_type_shared(alloc_arrow_type(args))
    };

    if !sig.is_polymorphic(term.f_code()) {
        sig.declare_type(term.f_code(), type_)?;
    }
    term.set_type(Some(sort));
    Ok(())
}

fn term_has_type(term: &Term, expected: &Type) -> bool {
    term.type_().is_some_and(|type_| &type_ == expected)
}

fn predefined_arithmetic_occurrence_type(
    sig: &mut Signature,
    term: &Term,
    symbol: PredefinedArithmeticSymbol,
    options: TypeInferOptions,
) -> Result<Type, Diagnostic> {
    if options.problem_type == ProblemType::HigherOrder {
        return Err(arithmetic_type_error(format!(
            "{} is not supported in THF terms",
            symbol.name()
        )));
    }
    if term.arity() != usize::try_from(symbol.arity()).unwrap_or(0) {
        return Err(arithmetic_type_error(format!(
            "{} expects {} argument(s), got {}",
            symbol.name(),
            symbol.arity(),
            term.arity()
        )));
    }

    let mut argument_types = Vec::with_capacity(term.arity());
    let mut argument_sorts = Vec::with_capacity(term.arity());
    for index in 0..term.arity() {
        let argument_type = required_type(&required_arg(term, index)?)?;
        argument_sorts.push(argument_type.f_code());
        argument_types.push(argument_type);
    }
    let result_sort = symbol.result_sort(&argument_sorts).ok_or_else(|| {
        arithmetic_type_error(format!(
            "{} has incompatible arithmetic argument sorts",
            symbol.name()
        ))
    })?;
    let result_type = predefined_sort_type(sig, result_sort).ok_or_else(|| {
        arithmetic_type_error(format!(
            "{} produced an unsupported result sort",
            symbol.name()
        ))
    })?;
    argument_types.push(result_type);
    Ok(sig
        .type_bank_mut()
        .insert_type_shared(alloc_arrow_type(argument_types)))
}

fn predefined_arithmetic_occurrence_is_consistent(
    sig: &Signature,
    term: &Term,
    symbol: PredefinedArithmeticSymbol,
) -> bool {
    if term.arity() != usize::try_from(symbol.arity()).unwrap_or(0) {
        return false;
    }
    let Some(argument_sorts) = term
        .argument_clones()
        .into_iter()
        .map(|argument| {
            argument
                .and_then(|argument| argument.type_())
                .map(|type_| type_.f_code())
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    let Some(result_sort) = symbol.result_sort(&argument_sorts) else {
        return false;
    };
    term.type_()
        .is_some_and(|type_| type_.f_code() == result_sort)
        && predefined_sort_type(sig, result_sort).is_some()
}

fn predefined_sort_type(sig: &Signature, sort: TypeConsCode) -> Option<Type> {
    match sort {
        ST_BOOL => Some(sig.type_bank().bool_type()),
        ST_INTEGER => Some(sig.type_bank().integer_type()),
        ST_RATIONAL => Some(sig.type_bank().rational_type()),
        ST_REAL => Some(sig.type_bank().real_type()),
        _ => None,
    }
}

fn required_arg(term: &Term, index: usize) -> Result<Term, Diagnostic> {
    term.argument(index).ok_or_else(|| type_error("Type error"))
}

fn required_type(term: &Term) -> Result<Type, Diagnostic> {
    term.type_().ok_or_else(|| type_error("Type error"))
}

fn type_error(message: &'static str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

fn arithmetic_type_error(message: String) -> Diagnostic {
    Diagnostic::new(ErrorCode::TYPE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        infer_return_sort, term_determine_type, type_check_consistent,
        type_declare_is_not_predicate, type_declare_is_predicate, type_infer_sort_with_options,
        TypeInferOptions,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::terms::signature::{Signature, FP_IS_INTEGER};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn signature_with_internal_codes() -> Signature {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        sig
    }

    fn first_order_options() -> TypeInferOptions {
        TypeInferOptions {
            problem_type: ProblemType::FirstOrder,
            app_encode: false,
        }
    }

    fn arithmetic_term(sig: &mut Signature, name: &str, argument_types: &[Type]) -> Term {
        let arity = i32::try_from(argument_types.len()).unwrap();
        let f_code = sig.insert_id_for_problem(name, arity, false, ProblemType::FirstOrder);
        let term = Term::top_alloc(f_code, argument_types.len());
        for (index, type_) in argument_types.iter().enumerate() {
            let argument = Term::const_cell_alloc(-2 - 2 * i64::try_from(index).unwrap());
            argument.set_type(Some(type_.clone()));
            term.set_argument(index, argument);
        }
        term
    }

    #[test]
    fn term_determine_type_returns_result_or_residual_arrow() {
        let mut bank = TypeBank::new();
        let function_type = bank.insert_type_shared(alloc_arrow_type(vec![
            bank.i_type(),
            bank.integer_type(),
            bank.bool_type(),
        ]));

        let full = Term::top_alloc(20, 2);
        assert_eq!(
            term_determine_type(&full, &function_type, &mut bank),
            Some(bank.bool_type())
        );

        let partial = Term::top_alloc(20, 1);
        let residual = term_determine_type(&partial, &function_type, &mut bank).unwrap();
        assert!(residual.is_arrow());
        assert_eq!(residual.args()[0], bank.integer_type());
        assert_eq!(residual.args()[1], bank.bool_type());

        let over_applied = Term::top_alloc(20, 3);
        assert!(term_determine_type(&over_applied, &function_type, &mut bank).is_none());
    }

    #[test]
    fn infer_return_sort_uses_distinct_numeric_properties() {
        let mut sig = signature_with_internal_codes();
        let number = sig.insert_id("number", 0, false);
        sig.set_func_prop(number, FP_IS_INTEGER);

        assert_eq!(
            infer_return_sort(&sig, number),
            sig.type_bank().integer_type()
        );

        let ordinary = sig.insert_id("ordinary", 0, false);
        assert_eq!(
            infer_return_sort(&sig, ordinary),
            sig.type_bank().default_type()
        );
    }

    #[test]
    fn infer_sort_assigns_variables_and_declares_untyped_functions() {
        let mut sig = signature_with_internal_codes();
        let variable = Term::const_cell_alloc(-2);
        type_infer_sort_with_options(&mut sig, &variable, first_order_options()).unwrap();
        assert_eq!(variable.type_(), Some(sig.type_bank().default_type()));

        let f_code = sig.insert_id("f", 1, false);
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, variable);

        type_infer_sort_with_options(&mut sig, &root, first_order_options()).unwrap();

        assert_eq!(root.type_(), Some(sig.type_bank().default_type()));
        let declared = sig.get_type(f_code).unwrap();
        assert!(declared.is_arrow());
        assert_eq!(declared.args()[0], sig.type_bank().default_type());
        assert_eq!(declared.args()[1], sig.type_bank().default_type());
    }

    #[test]
    fn fixed_type_argument_mismatch_reports_type_error() {
        let mut sig = signature_with_internal_codes();
        let f_code = sig.insert_id("f", 1, false);
        let integer_type = sig.type_bank().integer_type();
        let bool_type = sig.type_bank().bool_type();
        let f_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![integer_type, bool_type]));
        sig.declare_final_type(f_code, f_type).unwrap();

        let arg = Term::const_cell_alloc(-2);
        arg.set_type(Some(sig.type_bank().default_type()));
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, arg);

        let error =
            type_infer_sort_with_options(&mut sig, &root, first_order_options()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
    }

    #[test]
    fn equality_and_quantifier_types_are_inferred_from_arguments() {
        let mut sig = signature_with_internal_codes();
        let x = Term::const_cell_alloc(-2);
        x.set_type(Some(sig.type_bank().integer_type()));
        let y = Term::const_cell_alloc(-4);
        y.set_type(Some(sig.type_bank().integer_type()));
        let eq = Term::top_alloc(sig.eqn_code(), 2);
        eq.set_argument(0, x.clone());
        eq.set_argument(1, y);

        type_infer_sort_with_options(&mut sig, &eq, first_order_options()).unwrap();
        assert_eq!(eq.type_(), Some(sig.type_bank().bool_type()));

        let qex = Term::top_alloc(sig.qex_code(), 2);
        qex.set_argument(0, x);
        qex.set_argument(1, eq.clone());

        type_infer_sort_with_options(&mut sig, &qex, first_order_options()).unwrap();
        assert_eq!(qex.type_(), Some(sig.type_bank().bool_type()));
    }

    #[test]
    fn predicate_and_non_predicate_declarations_update_term_and_signature() {
        let mut sig = signature_with_internal_codes();
        let pred = sig.insert_id("p", 1, false);
        let default_type = sig.type_bank().default_type();
        let pred_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![default_type.clone(), default_type]));
        sig.declare_type(pred, pred_type).unwrap();
        let atom = Term::top_alloc(pred, 1);
        let arg = Term::const_cell_alloc(-2);
        arg.set_type(Some(sig.type_bank().default_type()));
        atom.set_argument(0, arg);

        type_declare_is_predicate(&mut sig, &atom).unwrap();

        assert_eq!(atom.type_(), Some(sig.type_bank().bool_type()));
        assert!(sig.is_fixed_type(pred));
        assert_eq!(
            sig.get_type(pred).unwrap().args()[1],
            sig.type_bank().bool_type()
        );

        let fun = sig.insert_id("g", 0, false);
        let constant = Term::const_cell_alloc(fun);
        type_declare_is_not_predicate(&mut sig, &constant, first_order_options()).unwrap();
        assert!(sig.is_fixed_type(fun));
        assert_eq!(constant.type_(), Some(sig.type_bank().default_type()));
    }

    #[test]
    fn consistency_checks_follow_declared_symbol_types() {
        let mut sig = signature_with_internal_codes();
        let f_code = sig.insert_id("f", 1, false);
        let default_type = sig.type_bank().default_type();
        let bool_type = sig.type_bank().bool_type();
        let f_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![default_type, bool_type]));
        sig.declare_final_type(f_code, f_type).unwrap();

        let arg = Term::const_cell_alloc(-2);
        arg.set_type(Some(sig.type_bank().default_type()));
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, arg.clone());
        root.set_type(Some(sig.type_bank().bool_type()));
        assert!(type_check_consistent(&sig, &root));

        arg.set_type(Some(sig.type_bank().integer_type()));
        assert!(!type_check_consistent(&sig, &root));
    }

    #[test]
    fn every_predefined_arithmetic_symbol_instantiates_for_numeric_sorts() {
        let mut sig = signature_with_internal_codes();
        let numeric_sorts = [
            sig.type_bank().integer_type(),
            sig.type_bank().rational_type(),
            sig.type_bank().real_type(),
        ];

        for sort in &numeric_sorts {
            for name in ["$less", "$lesseq", "$greater", "$greatereq"] {
                let term = arithmetic_term(&mut sig, name, &[sort.clone(), sort.clone()]);
                type_infer_sort_with_options(&mut sig, &term, first_order_options()).unwrap();
                assert_eq!(term.type_(), Some(sig.type_bank().bool_type()), "{name}");
                assert!(type_check_consistent(&sig, &term), "{name}");
            }
            for name in ["$is_int", "$is_rat"] {
                let term = arithmetic_term(&mut sig, name, std::slice::from_ref(sort));
                type_infer_sort_with_options(&mut sig, &term, first_order_options()).unwrap();
                assert_eq!(term.type_(), Some(sig.type_bank().bool_type()), "{name}");
                assert!(type_check_consistent(&sig, &term), "{name}");
            }
            for name in [
                "$uminus",
                "$floor",
                "$ceiling",
                "$truncate",
                "$round",
                "$abs",
            ] {
                let term = arithmetic_term(&mut sig, name, std::slice::from_ref(sort));
                type_infer_sort_with_options(&mut sig, &term, first_order_options()).unwrap();
                assert_eq!(term.type_(), Some(sort.clone()), "{name}");
                assert!(type_check_consistent(&sig, &term), "{name}");
            }
            for name in [
                "$sum",
                "$difference",
                "$product",
                "$quotient_e",
                "$quotient_t",
                "$quotient_f",
                "$remainder_e",
                "$remainder_t",
                "$remainder_f",
            ] {
                let term = arithmetic_term(&mut sig, name, &[sort.clone(), sort.clone()]);
                type_infer_sort_with_options(&mut sig, &term, first_order_options()).unwrap();
                assert_eq!(term.type_(), Some(sort.clone()), "{name}");
                assert!(type_check_consistent(&sig, &term), "{name}");
            }
        }
    }

    #[test]
    fn quotient_and_coercions_use_their_exact_result_sorts() {
        let mut sig = signature_with_internal_codes();
        let integer = sig.type_bank().integer_type();
        let rational = sig.type_bank().rational_type();
        let real = sig.type_bank().real_type();

        for (source, result) in [
            (integer.clone(), rational.clone()),
            (rational.clone(), rational.clone()),
            (real.clone(), real.clone()),
        ] {
            let quotient =
                arithmetic_term(&mut sig, "$quotient", &[source.clone(), source.clone()]);
            type_infer_sort_with_options(&mut sig, &quotient, first_order_options()).unwrap();
            assert_eq!(quotient.type_(), Some(result));
        }

        for source in [integer, rational, real] {
            for (name, target) in [
                ("$to_int", sig.type_bank().integer_type()),
                ("$to_rat", sig.type_bank().rational_type()),
                ("$to_real", sig.type_bank().real_type()),
            ] {
                let coercion = arithmetic_term(&mut sig, name, std::slice::from_ref(&source));
                type_infer_sort_with_options(&mut sig, &coercion, first_order_options()).unwrap();
                assert_eq!(coercion.type_(), Some(target), "{name}");
            }
        }
    }

    #[test]
    fn arithmetic_mismatches_partial_applications_and_thf_are_type_errors() {
        let mut sig = signature_with_internal_codes();
        let integer = sig.type_bank().integer_type();
        let real = sig.type_bank().real_type();
        let individual = sig.type_bank().i_type();

        let cases = [
            arithmetic_term(&mut sig, "$sum", &[integer.clone(), real]),
            arithmetic_term(&mut sig, "$sum", &[integer.clone(), individual]),
            arithmetic_term(&mut sig, "$sum", std::slice::from_ref(&integer)),
            arithmetic_term(&mut sig, "$to_int", &[]),
        ];
        for term in cases {
            let error =
                type_infer_sort_with_options(&mut sig, &term, first_order_options()).unwrap_err();
            assert_eq!(error.code(), ErrorCode::TYPE_ERROR);
        }

        let thf_sum = arithmetic_term(&mut sig, "$sum", &[integer.clone(), integer.clone()]);
        let error = type_infer_sort_with_options(
            &mut sig,
            &thf_sum,
            TypeInferOptions {
                problem_type: ProblemType::HigherOrder,
                app_encode: false,
            },
        )
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::TYPE_ERROR);

        let valid = arithmetic_term(&mut sig, "$sum", &[integer.clone(), integer]);
        type_infer_sort_with_options(&mut sig, &valid, first_order_options()).unwrap();
        valid.set_type(Some(sig.type_bank().real_type()));
        assert!(!type_check_consistent(&sig, &valid));
    }
}
