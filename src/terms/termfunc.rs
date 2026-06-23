use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::terms::functypes::{func_symb_parse, FunCode, FuncSymbType};
use crate::terms::signature::{
    Signature, FP_INTERPRETED, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
    SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
};
use crate::terms::simpletypes::{type_drop_first_arg, types_cmp, var_order};
use crate::terms::termtypes::{
    term_del_prop_opt, term_deref, term_identity_id, DerefType, Term, DEFAULT_FWEIGHT,
    DEFAULT_VWEIGHT, TP_OP_FLAG, TP_PRED_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum VarNormStyle {
    None = -1,
    Univar = 0,
    Alpha = 1,
}

/// Formats a free-variable code using E's `Xn`/`Yn` convention.
///
/// # Panics
///
/// Panics if `var` is not a negative free-variable code.
#[must_use]
pub fn var_print_string(var: FunCode) -> String {
    assert!(var < 0, "variable f-code must be negative");
    let prefix = if var % 2 == 0 { 'X' } else { 'Y' };
    format!("{prefix}{}", -((var - 1) / 2))
}

pub fn term_parse_operator(
    scanner: &mut Scanner,
    id: &mut DynamicString,
) -> Result<FuncSymbType, Diagnostic> {
    if scanner.test_id("$distinct") {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{} $distinct is only allowed as the sole predicate symbol of an atomic formula",
                token_pos_rep(scanner.current_token())
            ),
        ));
    }

    let mut result = func_symb_parse(scanner, id)?;
    if matches!(
        id.view_bytes().first(),
        Some(first) if first.is_ascii_uppercase() || *first == b'_'
    ) && scanner.test_tok(TokenType::OPEN_BRACKET)
    {
        result = FuncSymbType::IdentFreeFun;
    }
    Ok(result)
}

#[must_use]
pub fn term_sig_insert(
    sig: &mut Signature,
    name: &str,
    arity: i32,
    special_id: bool,
    ident_type: FuncSymbType,
) -> FunCode {
    let result = sig.insert_id(name, arity, special_id);
    if result == 0 {
        return result;
    }

    match ident_type {
        FuncSymbType::IdentInt => sig.set_func_prop(result, FP_IS_INTEGER),
        FuncSymbType::IdentFloat => sig.set_func_prop(result, FP_IS_FLOAT),
        FuncSymbType::IdentRational => sig.set_func_prop(result, FP_IS_RATIONAL),
        FuncSymbType::IdentObject => sig.set_func_prop(result, FP_IS_OBJECT),
        FuncSymbType::IdentInterpreted => sig.set_func_prop(result, FP_INTERPRETED),
        FuncSymbType::None | FuncSymbType::IdentVar | FuncSymbType::IdentFreeFun => {}
    }
    result
}

pub fn term_parse(
    scanner: &mut Scanner,
    sig: &mut Signature,
    vars: &VarBank,
) -> Result<Term, Diagnostic> {
    let mut id = DynamicString::new();
    let id_type = term_parse_operator(scanner, &mut id)?;
    let name = id.view().into_owned();
    if id_type == FuncSymbType::IdentVar {
        if scanner.test_tok(TokenType::COLON) {
            scanner.accept_tok(TokenType::COLON)?;
            let type_ = sig
                .type_bank_mut()
                .parse_type_from_current_problem(scanner)?;
            return Ok(vars.ext_name_assert_alloc_sort(&name, &type_));
        }
        return Ok(vars.ext_name_assert_alloc(&name));
    }

    let handle = if scanner.test_tok(TokenType::OPEN_BRACKET) {
        reject_distinct_argument_list(sig, id_type)?;
        term_parse_arg_list(scanner, sig, vars)?
    } else {
        Term::default_cell_alloc()
    };
    let arity = c_arity(handle.arity())?;
    let f_code = term_sig_insert(sig, &name, arity, false, id_type);
    if f_code == 0 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!("{name} used with incompatible arity {arity}"),
        ));
    }
    handle.set_f_code(f_code);
    Ok(handle)
}

pub fn term_parse_arg_list(
    scanner: &mut Scanner,
    sig: &mut Signature,
    vars: &VarBank,
) -> Result<Term, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    if scanner.test_tok(TokenType::CLOSE_BRACKET) {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        return Ok(Term::default_cell_alloc());
    }

    let mut args = vec![term_parse(scanner, sig, vars)?];
    while scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        args.push(term_parse(scanner, sig, vars)?);
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    let result = Term::default_cell_arity_alloc(args.len());
    for (index, arg) in args.into_iter().enumerate() {
        result.set_argument(index, arg);
    }
    Ok(result)
}

fn reject_distinct_argument_list(sig: &Signature, id_type: FuncSymbType) -> Result<(), Diagnostic> {
    if id_type == FuncSymbType::IdentInt && sig.distinct_props().intersects(FP_IS_INTEGER) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Number cannot have argument list (consider --free-numbers)",
        ));
    }
    if id_type == FuncSymbType::IdentObject && sig.distinct_props().intersects(FP_IS_OBJECT) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Object cannot have argument list (consider --free-objects)",
        ));
    }
    Ok(())
}

fn c_arity(arity: usize) -> Result<i32, Diagnostic> {
    i32::try_from(arity).map_err(|_| {
        Diagnostic::new(
            ErrorCode::RESOURCE_OUT,
            "Term arity is too large for C-compatible signatures",
        )
    })
}

/// Writes a first-order term without assigning special semantics to symbols.
///
/// # Panics
///
/// Panics if a non-constant term has an uninitialized argument, matching the
/// C precondition that all argument slots are valid term pointers.
pub fn term_write_simple(
    output: &mut impl fmt::Write,
    term: &Term,
    sig: &Signature,
) -> fmt::Result {
    if term.is_free_var() {
        return write!(output, "{}", var_print_string(term.f_code()));
    }

    write!(
        output,
        "{}",
        sig.find_name(term.f_code()).unwrap_or("<unknown>")
    )?;
    if !term.is_const() {
        write!(output, "(")?;
        let first = term
            .argument(0)
            .unwrap_or_else(|| panic!("term argument 0 is uninitialized"));
        term_write_simple(output, &first, sig)?;
        for index in 1..term.arity() {
            write!(output, ",")?;
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            term_write_simple(output, &arg, sig)?;
        }
        write!(output, ")")?;
    }
    Ok(())
}

#[must_use]
pub fn term_simple_string(term: &Term, sig: &Signature) -> String {
    let mut output = String::new();
    let _ = term_write_simple(&mut output, term, sig);
    output
}

/// Writes an uninstantiated term as an s-expression.
///
/// # Panics
///
/// Panics if a non-constant term has an uninitialized argument, matching the
/// C precondition that all argument slots are valid term pointers.
pub fn term_write_s_expr(
    output: &mut impl fmt::Write,
    term: &Term,
    sig: &Signature,
) -> fmt::Result {
    if term.arity() != 0 {
        write!(output, "(")?;
    }

    if term.is_db_var() {
        write!(output, "db({})", term.f_code())?;
    } else if term.is_free_var() {
        write!(output, "{}", var_print_string(term.f_code()))?;
    } else {
        write!(
            output,
            "{}",
            sig.find_name(term.f_code()).unwrap_or("<unknown>")
        )?;
    }

    for index in 0..term.arity() {
        write!(output, "   ")?;
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        term_write_s_expr(output, &arg, sig)?;
    }
    if term.arity() != 0 {
        write!(output, ")")?;
    }
    Ok(())
}

#[must_use]
pub fn term_s_expr_string(term: &Term, sig: &Signature) -> String {
    let mut output = String::new();
    let _ = term_write_s_expr(&mut output, term, sig);
    output
}

#[must_use]
pub fn term_is_flat(term: &Term) -> bool {
    if term.is_const() || term.is_free_var() {
        return true;
    }
    term.argument_clones()
        .into_iter()
        .flatten()
        .all(|arg| arg.is_const() || arg.is_free_var())
}

#[must_use]
pub fn term_struct_equal(left: &Term, right: &Term) -> bool {
    term_struct_equal_deref(left, right, DerefType::Always, DerefType::Always)
}

#[must_use]
pub fn term_struct_equal_no_deref(left: &Term, right: &Term) -> bool {
    if left == right {
        return true;
    }
    if left.is_db_var() != right.is_db_var()
        || left.f_code() != right.f_code()
        || !term_type_eq(left, right)
        || left.arity() != right.arity()
    {
        return false;
    }

    left.argument_clones()
        .into_iter()
        .zip(right.argument_clones())
        .all(|(left, right)| {
            left.zip(right)
                .is_some_and(|(left, right)| term_struct_equal_no_deref(&left, &right))
        })
}

#[must_use]
pub fn term_struct_equal_deref(
    left: &Term,
    right: &Term,
    mut left_deref: DerefType,
    mut right_deref: DerefType,
) -> bool {
    let left = term_deref(left, &mut left_deref);
    let right = term_deref(right, &mut right_deref);

    if !term_type_eq(&left, &right) {
        return false;
    }
    if left_deref == DerefType::Never && right_deref == DerefType::Never {
        return left == right;
    }
    if left == right && left_deref == right_deref {
        return true;
    }
    if left.is_db_var() != right.is_db_var()
        || left.f_code() != right.f_code()
        || left.arity() != right.arity()
    {
        return false;
    }

    let start = usize::from(left.is_lambda());
    left.argument_clones()
        .into_iter()
        .zip(right.argument_clones())
        .enumerate()
        .skip(start)
        .all(|(_index, (left, right))| {
            left.zip(right).is_some_and(|(left, right)| {
                term_struct_equal_deref(&left, &right, left_deref, right_deref)
            })
        })
}

#[must_use]
pub fn term_struct_prefix_equal(
    left: &Term,
    right: &Term,
    mut left_deref: DerefType,
    mut right_deref: DerefType,
    remaining: usize,
) -> bool {
    if remaining == 0 {
        return term_struct_equal_deref(left, right, left_deref, right_deref);
    }

    let left = term_deref(left, &mut left_deref);
    let mut right = term_deref(right, &mut right_deref);
    if right.is_applied_any_var() && right.arity().saturating_sub(remaining) == 1 {
        let Some(head) = right.argument(0) else {
            return false;
        };
        right = head;
    }

    if left.f_code() != right.f_code() || (!right.is_any_var() && right.arity() < remaining) {
        return false;
    }
    if !(left.is_any_var() && right.is_any_var()) && left.arity() != right.arity() - remaining {
        return false;
    }

    left.argument_clones()
        .into_iter()
        .zip(right.argument_clones())
        .all(|(left, right)| {
            left.zip(right).is_some_and(|(left, right)| {
                term_struct_equal_deref(&left, &right, left_deref, right_deref)
            })
        })
}

#[must_use]
pub fn term_struct_weight_compare(left: &Term, right: &Term) -> i64 {
    if left.f_code() == SIG_TRUE_CODE {
        return if right.f_code() == SIG_TRUE_CODE {
            0
        } else {
            -1
        };
    }
    if right.f_code() == SIG_TRUE_CODE {
        return 1;
    }

    let weight_cmp = term_standard_weight(left) - term_standard_weight(right);
    if weight_cmp != 0 {
        return weight_cmp;
    }

    if left.is_free_var() {
        return compare_term_types(left, right);
    }

    let db_cmp = cmp_bool(!left.is_db_var(), !right.is_db_var());
    if db_cmp != 0 {
        return db_cmp;
    }
    if left.is_db_var() {
        return compare_term_types(left, right);
    }

    let arity_cmp = cmp_usize(left.arity(), right.arity());
    if arity_cmp != 0 {
        return arity_cmp;
    }

    left.argument_clones()
        .into_iter()
        .zip(right.argument_clones())
        .find_map(|(left, right)| {
            left.zip(right).and_then(|(left, right)| {
                let cmp = term_struct_weight_compare(&left, &right);
                (cmp != 0).then_some(cmp)
            })
        })
        .unwrap_or(0)
}

#[must_use]
pub fn term_lex_compare(left: &Term, right: &Term) -> i64 {
    let f_code_cmp = left.f_code() - right.f_code();
    if f_code_cmp != 0 {
        return f_code_cmp;
    }
    let arity_cmp = cmp_usize(left.arity(), right.arity());
    if arity_cmp != 0 {
        return arity_cmp;
    }
    left.argument_clones()
        .into_iter()
        .zip(right.argument_clones())
        .find_map(|(left, right)| {
            left.zip(right).and_then(|(left, right)| {
                let cmp = term_lex_compare(&left, &right);
                (cmp != 0).then_some(cmp)
            })
        })
        .unwrap_or(0)
}

#[must_use]
pub fn term_is_subterm(super_term: &Term, test: &Term, mut deref: DerefType) -> bool {
    let super_term = term_deref(super_term, &mut deref);
    if &super_term == test {
        return true;
    }
    super_term
        .argument_clones()
        .into_iter()
        .flatten()
        .any(|arg| term_is_subterm(&arg, test, deref))
}

#[must_use]
pub fn term_is_subterm_deref(
    super_term: &Term,
    test: &Term,
    mut super_deref: DerefType,
    test_deref: DerefType,
) -> bool {
    let super_term = term_deref(super_term, &mut super_deref);
    if term_struct_equal_deref(&super_term, test, super_deref, test_deref) {
        return true;
    }
    super_term
        .argument_clones()
        .into_iter()
        .flatten()
        .any(|arg| term_is_subterm_deref(&arg, test, super_deref, test_deref))
}

#[must_use]
pub fn term_weight_compute(term: &Term, vweight: i64, fweight: i64) -> i64 {
    if term.is_free_var() {
        return vweight;
    }

    let mut result = if term.is_phony_app() || term.is_db_lambda() {
        0
    } else {
        fweight
    };
    for arg in term
        .argument_clones()
        .into_iter()
        .enumerate()
        .skip(usize::from(term.is_db_lambda()))
        .filter_map(|(_index, arg)| arg)
    {
        result += term_weight_compute(&arg, vweight, fweight);
    }
    result
}

#[must_use]
pub fn term_standard_weight(term: &Term) -> i64 {
    term_weight_compute(term, DEFAULT_VWEIGHT, DEFAULT_FWEIGHT)
}

#[must_use]
pub fn term_fsum_weight(
    term: &Term,
    vweight: i64,
    flimit: FunCode,
    fweights: &[i64],
    default_fweight: i64,
    typefreqs: Option<&BTreeMap<i64, i64>>,
) -> i64 {
    if term.is_free_var() {
        return vweight;
    }

    let mut result = 0;
    if term.f_code() < flimit {
        if !term.is_phony_app() {
            result += usize::try_from(term.f_code())
                .ok()
                .and_then(|index| fweights.get(index))
                .copied()
                .unwrap_or(default_fweight);
        } else if let (Some(freqs), Some(type_)) =
            (typefreqs, term.argument(0).and_then(|head| head.type_()))
        {
            result += freqs.get(&type_.type_uid()).copied().unwrap_or(0);
        }
    } else if !term.is_phony_app() {
        result += default_fweight;
    }

    for arg in term.argument_clones().into_iter().flatten() {
        result += term_fsum_weight(&arg, vweight, flimit, fweights, default_fweight, typefreqs);
    }
    result
}

#[must_use]
pub fn term_non_linear_weight(
    term: &Term,
    first_var_weight: i64,
    repeat_var_weight: i64,
    fweight: i64,
) -> i64 {
    term_del_prop_opt(term, TP_OP_FLAG);
    let mut result = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if current.query_prop(TP_OP_FLAG) {
                result += repeat_var_weight;
            } else {
                current.set_prop(TP_OP_FLAG);
                result += first_var_weight;
            }
        } else {
            result += if current.is_phony_app() { 0 } else { fweight };
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    result
}

#[must_use]
pub fn term_sym_type_weight(
    term: &Term,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
) -> i64 {
    let mut result = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            result += vweight;
        } else {
            if current.query_prop(TP_PRED_POS) {
                result += pweight;
            } else if current.arity() == 0 {
                result += cweight;
            } else if !current.is_phony_app() {
                result += fweight;
            }
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    result
}

#[must_use]
pub fn term_depth(term: &Term) -> i64 {
    term.argument_clones()
        .into_iter()
        .flatten()
        .map(|arg| term_depth(&arg))
        .max()
        .unwrap_or(0)
        + 1
}

#[must_use]
pub fn term_is_def_term(term: &Term, min_arity: usize) -> bool {
    if term.is_any_var() || term.is_phony_app() || term.is_lambda() || term.arity() < min_arity {
        return false;
    }
    let expected_weight =
        DEFAULT_FWEIGHT + i64::try_from(term.arity()).unwrap_or(i64::MAX) * DEFAULT_VWEIGHT;
    if term_standard_weight(term) != expected_weight {
        return false;
    }

    for arg in term.argument_clones().into_iter().flatten() {
        arg.del_prop(TP_OP_FLAG);
    }
    for arg in term.argument_clones().into_iter().flatten() {
        if !arg.is_free_var() || arg.query_prop(TP_OP_FLAG) {
            return false;
        }
        arg.set_prop(TP_OP_FLAG);
    }
    true
}

#[must_use]
pub fn term_has_f_code(term: &Term, f_code: FunCode) -> bool {
    if term.is_db_var() {
        return false;
    }
    term.f_code() == f_code
        || term
            .argument_clones()
            .into_iter()
            .flatten()
            .any(|arg| term_has_f_code(&arg, f_code))
}

#[must_use]
pub fn term_has_unbound_variables(term: &Term) -> bool {
    if term.is_free_var() {
        return term.binding().is_none();
    }
    term.argument_clones()
        .into_iter()
        .flatten()
        .any(|arg| term_has_unbound_variables(&arg))
}

#[must_use]
pub fn term_is_ground_compute(term: &Term) -> bool {
    !term.is_free_var()
        && term
            .argument_clones()
            .into_iter()
            .flatten()
            .all(|arg| term_is_ground_compute(&arg))
}

#[must_use]
pub fn term_find_max_var_code(term: &Term) -> FunCode {
    if term.is_free_var() {
        return term.f_code();
    }
    if term_is_ground_compute(term) {
        return 0;
    }
    term.argument_clones()
        .into_iter()
        .flatten()
        .map(|arg| term_find_max_var_code(&arg))
        .min()
        .unwrap_or(0)
}

pub fn term_collect_prop_variables(
    term: &Term,
    vars: &mut BTreeMap<usize, Term>,
    prop: crate::terms::termtypes::TermProperties,
) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if current.query_prop(prop)
                && vars.insert(term_identity_id(&current), current).is_none()
            {
                count += 1;
            }
        } else {
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

pub fn term_collect_variables(term: &Term, vars: &mut BTreeMap<usize, Term>) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if vars.insert(term_identity_id(&current), current).is_none() {
                count += 1;
            }
        } else {
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

pub fn term_collect_fcodes(term: &Term, fcodes: &mut BTreeSet<FunCode>) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.f_code() > 0 && fcodes.insert(current.f_code()) {
            count += 1;
        }
        stack.extend(current.argument_clones().into_iter().flatten());
    }
    count
}

#[must_use]
pub fn term_array_no_duplicates(args: &[Term]) -> bool {
    if args.len() <= 1 {
        return true;
    }
    args.windows(2).all(|window| window[0] != window[1])
}

pub fn term_linearize(stack: &mut Vec<Term>, term: &Term) -> i64 {
    stack.push(term.clone());
    let mut result = 1;
    for arg in term.argument_clones().into_iter().flatten() {
        result += term_linearize(stack, &arg);
    }
    result
}

#[must_use]
pub fn term_is_untyped(term: &Term) -> bool {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        let Some(type_) = current.type_() else {
            return false;
        };
        if !(type_.is_individual() || type_.is_bool()) {
            return false;
        }
        stack.extend(current.argument_clones().into_iter().flatten());
    }
    true
}

/// Creates a term prefix with the first `arg_num` logical arguments.
///
/// # Panics
///
/// Panics if `arg_num` is larger than the term arity or would return an
/// unchanged phony application with all physical arguments.
#[must_use]
pub fn term_create_prefix(orig: &Term, arg_num: usize) -> Term {
    assert!(orig.arity() >= arg_num, "prefix arity exceeds term arity");
    assert!(
        !orig.is_phony_app() || orig.arity() != arg_num,
        "phony app prefix cannot include the hidden head as a full prefix"
    );

    if arg_num == orig.arg_num() {
        return orig.clone();
    }
    if orig.is_phony_app() && arg_num == 0 {
        return orig
            .argument(0)
            .expect("phony application must have a hidden head");
    }

    assert!(arg_num < orig.arg_num(), "prefix must be shorter");
    let prefix_len = arg_num + usize::from(orig.is_phony_app());
    let prefix = Term::top_alloc(orig.f_code(), prefix_len);
    for (index, arg) in orig
        .argument_clones()
        .into_iter()
        .take(prefix_len)
        .flatten()
        .enumerate()
    {
        prefix.set_argument(index, arg);
    }
    prefix
}

#[must_use]
pub fn term_dag_weight(
    term: &Term,
    fweight: i64,
    vweight: i64,
    dup_weight: i64,
    new_term: bool,
) -> i64 {
    if new_term {
        term_del_prop_opt(term, TP_OP_FLAG);
    }

    let mut result = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.query_prop(TP_OP_FLAG) {
            result += dup_weight;
        } else {
            current.set_prop(TP_OP_FLAG);
            if current.is_free_var() {
                result += vweight;
            } else {
                result += fweight;
                stack.extend(current.argument_clones().into_iter().flatten());
            }
        }
    }
    result
}

#[must_use]
pub fn term_is_db_closed(term: &Term) -> bool {
    !term.has_db_subterm() || do_is_db_closed(term, 0)
}

/// Applies one argument to an unshared term.
///
/// # Panics
///
/// Panics unless the function term has an arrow type whose first argument is
/// pointer-identical to the argument type.
#[must_use]
pub fn term_apply_arg(type_bank: &mut TypeBank, source: &Term, arg: &Term) -> Term {
    let source_type = source.type_().expect("source term must have a type");
    let arg_type = arg.type_().expect("argument term must have a type");
    assert!(source_type.is_arrow(), "source type must be an arrow");
    assert_eq!(source_type.args()[0], arg_type);

    let applied = if !source.is_any_var() && !source.is_lambda() {
        let applied = Term::top_alloc(source.f_code(), source.arity() + 1);
        for (index, arg) in source.argument_clones().into_iter().flatten().enumerate() {
            applied.set_argument(index, arg);
        }
        applied.set_argument(source.arity(), arg.clone());
        applied
    } else {
        let applied = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        applied.set_argument(0, source.clone());
        applied.set_argument(1, arg.clone());
        applied
    };
    applied.set_type(Some(
        type_bank.insert_type_shared(type_drop_first_arg(&source_type)),
    ));
    applied
}

#[must_use]
pub fn term_compute_order(_sig: &Signature, term: &Term) -> usize {
    let mut order = term.type_().map_or(0, |type_| var_order(&type_));
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if let Some(type_) = current.type_() {
            order = order.max(var_order(&type_));
        }
        for arg in current
            .argument_clones()
            .into_iter()
            .enumerate()
            .skip(usize::from(current.is_lambda()))
            .filter_map(|(_index, arg)| arg)
        {
            stack.push(arg);
        }
    }
    order
}

fn do_is_db_closed(term: &Term, depth: FunCode) -> bool {
    if !term.has_db_subterm() {
        return true;
    }
    if term.is_db_var() {
        return term.f_code() < depth;
    }
    if term.is_lambda() {
        return term
            .argument(1)
            .is_some_and(|body| do_is_db_closed(&body, depth + 1));
    }
    term.argument_clones()
        .into_iter()
        .flatten()
        .all(|arg| do_is_db_closed(&arg, depth))
}

fn term_type_eq(left: &Term, right: &Term) -> bool {
    left.type_() == right.type_()
}

fn compare_term_types(left: &Term, right: &Term) -> i64 {
    match (left.type_(), right.type_()) {
        (Some(left), Some(right)) => i64::from(types_cmp(&left, &right)),
        (None, Some(_)) => -1,
        (Some(_), None) => 1,
        (None, None) => 0,
    }
}

fn cmp_bool(left: bool, right: bool) -> i64 {
    cmp_order(left.cmp(&right))
}

fn cmp_usize(left: usize, right: usize) -> i64 {
    cmp_order(left.cmp(&right))
}

fn cmp_order(ordering: Ordering) -> i64 {
    match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        term_apply_arg, term_array_no_duplicates, term_collect_fcodes, term_collect_variables,
        term_compute_order, term_create_prefix, term_dag_weight, term_depth,
        term_find_max_var_code, term_has_f_code, term_has_unbound_variables, term_is_db_closed,
        term_is_def_term, term_is_flat, term_is_ground_compute, term_is_subterm,
        term_is_subterm_deref, term_is_untyped, term_lex_compare, term_linearize,
        term_non_linear_weight, term_parse, term_parse_arg_list, term_parse_operator,
        term_s_expr_string, term_sig_insert, term_simple_string, term_standard_weight,
        term_struct_equal, term_struct_equal_deref, term_struct_equal_no_deref,
        term_struct_prefix_equal, term_struct_weight_compare, term_sym_type_weight,
        term_weight_compute, var_print_string, VarNormStyle,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::Scanner;
    use crate::terms::functypes::FuncSymbType;
    use crate::terms::signature::{
        Signature, FP_INTERPRETED, FP_IS_INTEGER, FP_IS_OBJECT, SIG_PHONY_APP_CODE,
    };
    use crate::terms::simpletypes::{alloc_arrow_type, type_drop_first_arg};
    use crate::terms::termtypes::{
        DerefType, Term, TP_HAS_DB_SUBTERM, TP_IS_DB_VAR, TP_OP_FLAG, TP_PRED_POS,
    };
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;
    use std::collections::{BTreeMap, BTreeSet};

    fn typed_var(code: i64, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::const_cell_alloc(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn parse_unshared(source: &str) -> (Signature, VarBank, Term) {
        let mut sig = Signature::new(TypeBank::new());
        let vars = VarBank::new(sig.type_bank());
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = term_parse(&mut scanner, &mut sig, &vars).unwrap();
        (sig, vars, term)
    }

    #[test]
    fn term_parse_operator_preserves_c_identifier_classification() {
        let mut scanner = Scanner::from_user_string("F(a)", false).unwrap();
        let mut id = DynamicString::new();
        assert_eq!(
            term_parse_operator(&mut scanner, &mut id).unwrap(),
            FuncSymbType::IdentFreeFun
        );
        assert_eq!(id.view(), "F");

        let mut scanner = Scanner::from_user_string("X", false).unwrap();
        let mut id = DynamicString::new();
        assert_eq!(
            term_parse_operator(&mut scanner, &mut id).unwrap(),
            FuncSymbType::IdentVar
        );

        let mut scanner = Scanner::from_user_string("$distinct", false).unwrap();
        let error = term_parse_operator(&mut scanner, &mut DynamicString::new()).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
    }

    #[test]
    fn term_sig_insert_sets_identifier_properties() {
        let mut sig = Signature::new(TypeBank::new());
        let integer = term_sig_insert(&mut sig, "12", 0, false, FuncSymbType::IdentInt);
        let object = term_sig_insert(&mut sig, "\"obj\"", 0, false, FuncSymbType::IdentObject);
        let interpreted = term_sig_insert(
            &mut sig,
            "$trueish",
            0,
            false,
            FuncSymbType::IdentInterpreted,
        );

        assert!(sig.query_prop(integer, FP_IS_INTEGER));
        assert!(sig.query_prop(object, FP_IS_OBJECT));
        assert!(sig.query_prop(interpreted, FP_INTERPRETED));
    }

    #[test]
    fn term_parse_builds_unshared_recursive_terms_and_bank_variables() {
        let (sig, vars, term) = parse_unshared("f(a,X,g(Y))");

        assert_eq!(sig.find_name(term.f_code()), Some("f"));
        assert_eq!(term.arity(), 3);
        assert!(!term.is_shared());
        assert_eq!(sig.find_name(term.argument(0).unwrap().f_code()), Some("a"));
        assert!(!term.argument(0).unwrap().is_shared());
        assert_eq!(term.argument(1).unwrap().f_code(), -2);
        assert_eq!(term.argument(2).unwrap().arity(), 1);
        assert_eq!(term.argument(2).unwrap().argument(0).unwrap().f_code(), -4);
        assert_eq!(vars.ext_name_find("X").unwrap().f_code(), -2);
        assert_eq!(vars.ext_name_find("Y").unwrap().f_code(), -4);
    }

    #[test]
    fn term_parse_treats_uppercase_application_as_function_symbol() {
        let (sig, _vars, term) = parse_unshared("F(a)");

        assert_eq!(sig.find_name(term.f_code()), Some("F"));
        assert_eq!(term.arity(), 1);
        assert!(term.f_code() > 0);
    }

    #[test]
    fn term_parse_arg_list_accepts_empty_and_nested_lists() {
        let mut sig = Signature::new(TypeBank::new());
        let vars = VarBank::new(sig.type_bank());
        let mut empty = Scanner::from_user_string("()", false).unwrap();
        assert_eq!(
            term_parse_arg_list(&mut empty, &mut sig, &vars)
                .unwrap()
                .arity(),
            0
        );

        let mut nested = Scanner::from_user_string("(a,f(X))", false).unwrap();
        let args = term_parse_arg_list(&mut nested, &mut sig, &vars).unwrap();
        assert_eq!(args.arity(), 2);
        assert_eq!(sig.find_name(args.argument(0).unwrap().f_code()), Some("a"));
        assert_eq!(args.argument(1).unwrap().arity(), 1);
    }

    #[test]
    fn term_parse_rejects_distinct_number_and_object_argument_lists() {
        let mut sig = Signature::new(TypeBank::new());
        let vars = VarBank::new(sig.type_bank());
        let mut number = Scanner::from_user_string("12(a)", false).unwrap();
        let error = term_parse(&mut number, &mut sig, &vars).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));

        let mut object = Scanner::from_user_string("\"obj\"(a)", false).unwrap();
        let error = term_parse(&mut object, &mut sig, &vars).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Object cannot have argument list"));
    }

    #[test]
    fn term_print_simple_serializes_unshared_tree_shape() {
        let (sig, _vars, term) = parse_unshared("f(a,X,g(Y))");

        assert_eq!(term_simple_string(&term, &sig), "f(a,X1,g(X2))");

        let (empty_sig, _vars, empty) = parse_unshared("f()");
        assert_eq!(term_simple_string(&empty, &empty_sig), "f");
    }

    #[test]
    fn term_print_simple_round_trips_through_unshared_parser() {
        let (sig, _vars, term) = parse_unshared("f(a,X,g(Y))");
        let printed = term_simple_string(&term, &sig);
        let (round_sig, _round_vars, round_term) = parse_unshared(&printed);

        assert_eq!(printed, "f(a,X1,g(X2))");
        assert_eq!(term_simple_string(&round_term, &round_sig), printed);
    }

    #[test]
    fn term_print_s_expr_matches_c_spacing_and_db_var_shape() {
        let (sig, _vars, term) = parse_unshared("f(a,X,g(Y))");

        assert_eq!(term_s_expr_string(&term, &sig), "(f   a   X1   (g   X2))");

        let (sig, _vars, variable) = parse_unshared("X");
        assert_eq!(term_s_expr_string(&variable, &sig), "X1");

        let db = Term::const_cell_alloc(0);
        db.set_prop(TP_IS_DB_VAR);
        assert_eq!(term_s_expr_string(&db, &sig), "db(0)");
    }

    #[test]
    fn variable_printing_and_norm_style_match_c_values() {
        assert_eq!(VarNormStyle::None as i32, -1);
        assert_eq!(VarNormStyle::Univar as i32, 0);
        assert_eq!(VarNormStyle::Alpha as i32, 1);
        assert_eq!(var_print_string(-2), "X1");
        assert_eq!(var_print_string(-1), "Y1");
        assert_eq!(var_print_string(-4), "X2");
    }

    #[test]
    fn flatness_weight_depth_and_symbol_tests_follow_term_shape() {
        let root = Term::top_alloc(10, 2);
        let x = Term::const_cell_alloc(-2);
        let nested = Term::top_alloc(11, 1);
        let y = Term::const_cell_alloc(-4);
        nested.set_argument(0, y.clone());
        root.set_argument(0, x.clone());
        root.set_argument(1, nested.clone());

        assert!(!term_is_flat(&root));
        assert!(term_is_flat(&nested));
        assert_eq!(term_weight_compute(&root, 1, 2), 6);
        assert_eq!(term_standard_weight(&root), 6);
        assert_eq!(term_depth(&root), 3);
        assert!(term_has_f_code(&root, 11));
        assert!(!term_has_f_code(&root, 99));
        assert!(term_has_unbound_variables(&root));
        y.set_binding(Some(Term::const_cell_alloc(99)));
        x.set_binding(Some(Term::const_cell_alloc(98)));
        assert!(!term_has_unbound_variables(&root));
        assert!(!term_is_ground_compute(&root));
        assert_eq!(term_find_max_var_code(&root), -4);
    }

    #[test]
    fn structural_comparisons_use_identity_deref_and_prefix_rules() {
        let bank = TypeBank::new();
        let i_type = bank.i_type();
        let left = Term::top_alloc(20, 1);
        let right = Term::top_alloc(20, 1);
        let x = typed_var(-2, &i_type);
        let x_copy = typed_var(-2, &i_type);
        left.set_type(Some(i_type.clone()));
        right.set_type(Some(i_type));
        left.set_argument(0, x.clone());
        right.set_argument(0, x_copy);

        assert!(term_struct_equal_no_deref(&left, &right));
        assert!(!term_struct_equal_deref(
            &left,
            &right,
            DerefType::Never,
            DerefType::Never
        ));
        assert!(term_struct_equal(&left, &right));

        let bound = Term::const_cell_alloc(30);
        bound.set_type(left.type_());
        x.set_binding(Some(bound.clone()));
        assert!(term_is_subterm(&left, &bound, DerefType::Always));
        assert!(term_is_subterm_deref(
            &left,
            &bound,
            DerefType::Always,
            DerefType::Never
        ));

        let longer = Term::top_alloc(20, 2);
        longer.set_type(left.type_());
        longer.set_argument(0, left.argument(0).unwrap());
        longer.set_argument(1, Term::const_cell_alloc(31));
        assert!(term_struct_prefix_equal(
            &left,
            &longer,
            DerefType::Never,
            DerefType::Never,
            1
        ));
    }

    #[test]
    fn comparisons_and_definition_checks_match_c_ordering_shapes() {
        let bank = TypeBank::new();
        let i_type = bank.i_type();
        let true_term = Term::const_cell_alloc(crate::terms::signature::SIG_TRUE_CODE);
        true_term.set_type(Some(bank.bool_type()));
        let f = Term::top_alloc(20, 2);
        let x = typed_var(-2, &i_type);
        let y = typed_var(-4, &i_type);
        f.set_argument(0, x.clone());
        f.set_argument(1, y.clone());
        f.set_type(Some(i_type.clone()));

        assert!(term_struct_weight_compare(&true_term, &f) < 0);
        assert_eq!(term_lex_compare(&x, &y), 2);
        assert!(term_is_def_term(&f, 2));
        assert!(!term_is_def_term(&Term::top_alloc(21, 0), 1));
        assert!(x.query_prop(TP_OP_FLAG));
        assert!(y.query_prop(TP_OP_FLAG));
    }

    #[test]
    fn nonlinear_symbol_type_and_dag_weights_preserve_property_side_effects() {
        let root = Term::top_alloc(30, 2);
        let x = Term::const_cell_alloc(-2);
        root.set_argument(0, x.clone());
        root.set_argument(1, x.clone());
        assert_eq!(term_non_linear_weight(&root, 5, 1, 2), 8);
        assert!(x.query_prop(TP_OP_FLAG));

        root.set_prop(TP_PRED_POS);
        assert_eq!(term_sym_type_weight(&root, 3, 5, 7, 11), 17);
        assert_eq!(term_dag_weight(&root, 2, 3, 13, true), 18);
    }

    #[test]
    fn collection_linearization_and_duplicate_helpers_match_c_shapes() {
        let root = Term::top_alloc(10, 2);
        let x = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        root.set_argument(0, x.clone());
        root.set_argument(1, y.clone());

        let mut vars = BTreeMap::new();
        assert_eq!(term_collect_variables(&root, &mut vars), 2);
        assert_eq!(vars.len(), 2);

        let mut fcodes = BTreeSet::new();
        assert_eq!(term_collect_fcodes(&root, &mut fcodes), 1);
        assert!(fcodes.contains(&10));

        let mut linearized = Vec::new();
        assert_eq!(term_linearize(&mut linearized, &root), 3);
        assert_eq!(linearized, vec![root.clone(), x.clone(), y.clone()]);

        assert!(term_array_no_duplicates(&[x.clone(), y.clone(), x.clone()]));
        assert!(!term_array_no_duplicates(&[x.clone(), x]));
    }

    #[test]
    fn prefix_application_untyped_and_order_helpers_match_type_shapes() {
        let mut bank = TypeBank::new();
        let i_type = bank.i_type();
        let bool_type = bank.bool_type();
        let arrow = bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), bool_type]));
        let f = Term::top_alloc(50, 1);
        f.set_type(Some(arrow.clone()));
        let x = typed_var(-2, &i_type);
        f.set_argument(0, x.clone());

        let prefix = term_create_prefix(&f, 0);
        assert_eq!(prefix.f_code(), 50);
        assert_eq!(prefix.arity(), 0);

        let arrow_var = typed_var(-8, &arrow);
        let variable_app = term_apply_arg(&mut bank, &arrow_var, &typed_var(-4, &i_type));
        assert_eq!(variable_app.f_code(), SIG_PHONY_APP_CODE);

        let applied = term_apply_arg(&mut bank, &f, &typed_var(-6, &i_type));
        assert_eq!(applied.arity(), 2);
        assert_eq!(applied.type_(), Some(type_drop_first_arg(&arrow)));

        let sig = Signature::new(TypeBank::new());
        applied.set_type(Some(bank.bool_type()));
        assert!(term_is_untyped(&applied));
        assert_eq!(term_compute_order(&sig, &applied), 0);
    }

    #[test]
    fn db_closed_checks_use_recorded_db_subterm_flags() {
        let lambda = Term::top_alloc(crate::terms::signature::SIG_DB_LAMBDA_CODE, 2);
        let binder = Term::const_cell_alloc(0);
        binder.set_prop(TP_IS_DB_VAR | TP_HAS_DB_SUBTERM);
        let body = Term::const_cell_alloc(0);
        body.set_prop(TP_IS_DB_VAR | TP_HAS_DB_SUBTERM);
        lambda.set_argument(0, binder);
        lambda.set_argument(1, body);
        lambda.set_prop(TP_HAS_DB_SUBTERM);
        assert!(term_is_db_closed(&lambda));

        let leaking = Term::const_cell_alloc(1);
        leaking.set_prop(TP_IS_DB_VAR | TP_HAS_DB_SUBTERM);
        assert!(!term_is_db_closed(&leaking));
    }
}
