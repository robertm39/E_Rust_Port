use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::terms::dbvars::DbVarBank;
use crate::terms::functypes::{func_symb_parse, FunCode, FuncSymbType};
use crate::terms::signature::{
    Signature, FP_INTERPRETED, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
    SIG_CONS_CODE, SIG_ITE_CODE, SIG_LET_CODE, SIG_NIL_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
};
use crate::terms::simpletypes::{type_drop_first_arg, types_cmp, var_order};
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{
    term_del_prop_opt, term_deref, term_identity_id, DerefType, Term, DEFAULT_FWEIGHT,
    DEFAULT_VWEIGHT, TP_IS_GROUND, TP_OP_FLAG, TP_PRED_POS, TP_TOP_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use crate::terms::typecheck::type_infer_sort;
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

impl VarNormStyle {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            -1 => Some(Self::None),
            0 => Some(Self::Univar),
            1 => Some(Self::Alpha),
            _ => None,
        }
    }
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
    if sig.supports_lists() && scanner.test_tok(TokenType::OPEN_SQUARE) {
        return term_parse_cons_list(scanner, sig, vars);
    }

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

fn term_parse_cons_list(
    scanner: &mut Scanner,
    sig: &mut Signature,
    vars: &VarBank,
) -> Result<Term, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_SQUARE)?;
    let mut elements = Vec::new();
    if !scanner.test_tok(TokenType::CLOSE_SQUARE) {
        elements.push(term_parse(scanner, sig, vars)?);
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            let element = term_parse(scanner, sig, vars)?;
            term_del_prop_opt(&element, TP_TOP_POS);
            elements.push(element);
        }
    }
    scanner.accept_tok(TokenType::CLOSE_SQUARE)?;

    let default_type = sig.type_bank().default_type();
    let mut list = Term::const_cell_alloc(SIG_NIL_CODE);
    list.set_type(Some(default_type.clone()));
    for element in elements.into_iter().rev() {
        let cons = Term::top_alloc(SIG_CONS_CODE, 2);
        cons.set_type(Some(default_type.clone()));
        cons.set_argument(0, element);
        cons.set_argument(1, list);
        list = cons;
    }
    Ok(list)
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

pub(crate) fn reject_distinct_argument_list(
    sig: &Signature,
    id_type: FuncSymbType,
) -> Result<(), Diagnostic> {
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

pub(crate) fn reject_term_bank_distinct_argument_list(
    sig: &Signature,
    id_type: FuncSymbType,
) -> Result<(), Diagnostic> {
    if id_type == FuncSymbType::IdentInt && sig.distinct_props().intersects(FP_IS_INTEGER) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Number cannot have argument list (consider --free-numbers)",
        ));
    }
    if id_type == FuncSymbType::IdentFloat && sig.distinct_props().intersects(FP_IS_FLOAT) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Floating point number cannot have argument list (consider --free-numbers)",
        ));
    }
    if id_type == FuncSymbType::IdentRational && sig.distinct_props().intersects(FP_IS_RATIONAL) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Rational number cannot have argument list (consider --free-numbers)",
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

/// Copies a term, allocating free variables from `vars`.
///
/// Non-variable cells are always copied into unshared cells. Free variables are
/// looked up or created in `vars`; DB variables are reused unless `dbvars` is
/// supplied.
///
/// # Panics
///
/// Panics if a copied free or DB variable has no type, or if variable-bank type
/// invariants are violated.
#[must_use]
pub fn term_copy(
    source: &Term,
    vars: &VarBank,
    mut dbvars: Option<&mut DbVarBank>,
    deref: DerefType,
) -> Term {
    let (source, current_deref, limit) = lfho_deref_no_whnf(source, deref);
    if source.is_free_var() {
        let type_ = source.type_().expect("copied free variables have types");
        return vars.var_assert_alloc(source.f_code(), &type_);
    }
    if source.is_db_var() {
        return if let Some(dbvars) = dbvars.as_deref_mut() {
            let type_ = source.type_().expect("copied DB variables have types");
            dbvars.request_db_var(&type_, source.f_code())
        } else {
            source
        };
    }

    let copy = Term::top_copy_without_args(&source);
    for (index, arg) in source.argument_clones().into_iter().enumerate() {
        let arg = arg.expect("term copy requires initialized args");
        let copied = term_copy(
            &arg,
            vars,
            dbvars.as_deref_mut(),
            convert_lfho_deref(index, limit, current_deref),
        );
        copy.set_argument(index, copied);
    }
    copy
}

/// Copies a term while preserving variable handles.
///
/// # Panics
///
/// Panics if a traversed argument slot is uninitialized.
#[must_use]
pub fn term_copy_keep_vars(source: &Term, deref: DerefType) -> Term {
    let (source, current_deref, limit) = lfho_deref_no_whnf(source, deref);
    if source.is_any_var() {
        return source;
    }

    let copy = Term::top_copy_without_args(&source);
    for (index, arg) in source.argument_clones().into_iter().enumerate() {
        let arg = arg.expect("term copy requires initialized args");
        let copied = term_copy_keep_vars(&arg, convert_lfho_deref(index, limit, current_deref));
        copy.set_argument(index, copied);
    }
    copy
}

/// Checks whether a term cell repeats on a single branch.
///
/// A shared DAG subterm that appears in different branches is not an
/// inconsistency; only a repeat before the recursion unwinds from that branch
/// is reported.
#[must_use]
pub fn term_check_consistency(term: &Term, deref: DerefType) -> Option<Term> {
    let mut branch = BTreeSet::new();
    term_check_consistency_rek(term, deref, &mut branch)
}

fn term_check_consistency_rek(
    term: &Term,
    deref: DerefType,
    branch: &mut BTreeSet<usize>,
) -> Option<Term> {
    let (term, current_deref, limit) = lfho_deref_no_whnf(term, deref);
    let key = term_identity_id(&term);
    if !branch.insert(key) {
        return Some(term);
    }

    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let Some(arg) = arg else {
            continue;
        };
        if let Some(repeated) = term_check_consistency_rek(
            &arg,
            convert_lfho_deref(index, limit, current_deref),
            branch,
        ) {
            return Some(repeated);
        }
    }

    let removed = branch.remove(&key);
    debug_assert!(removed, "branch entry should be present while unwinding");
    None
}

/// Copies a term using a precomputed free-variable renaming map.
///
/// # Panics
///
/// Panics if a free variable is missing from `renaming`, or if a traversed
/// argument slot is uninitialized.
#[must_use]
pub fn term_copy_rename_vars(renaming: &BTreeMap<FunCode, Term>, term: &Term) -> Term {
    if term.is_free_var() {
        return renaming
            .get(&term.f_code())
            .cloned()
            .expect("free variable must have an alpha-renaming entry");
    }
    if term.is_db_var() {
        return term.clone();
    }

    let copy = Term::top_copy_without_args(term);
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.expect("term copy requires initialized args");
        copy.set_argument(index, term_copy_rename_vars(renaming, &arg));
    }
    copy
}

/// Copies a term with free variables alpha-normalized by first occurrence.
///
/// # Panics
///
/// Panics if a free variable has no type, if variable-bank type invariants are
/// violated, or if a traversed argument slot is uninitialized.
#[must_use]
pub fn term_copy_normalize_vars_alpha(vars: &VarBank, term: &Term) -> Term {
    let renaming = create_var_renaming_de_bruijn(vars, term);
    term_copy_rename_vars(&renaming, term)
}

/// Copies a term with all free variables unified to `X0`.
///
/// The C helper allocates the unified variable with the variable bank's default
/// type, independent of the source variable types.
///
/// # Panics
///
/// Panics if variable-bank type invariants are violated, or if a traversed
/// argument slot is uninitialized.
#[must_use]
pub fn term_copy_unify_vars(vars: &VarBank, term: &Term) -> Term {
    if term.is_free_var() {
        return vars.var_assert_alloc(-2, &vars.default_type());
    }
    if term.is_db_var() {
        return term.clone();
    }

    let copy = Term::top_copy_without_args(term);
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.expect("term copy requires initialized args");
        copy.set_argument(index, term_copy_unify_vars(vars, &arg));
    }
    copy
}

/// Copies a term using the requested C variable-normalization style.
///
/// # Panics
///
/// Panics under the same conditions as the selected copy helper.
#[must_use]
pub fn term_copy_normalize_vars(vars: &VarBank, term: &Term, var_norm: VarNormStyle) -> Term {
    match var_norm {
        VarNormStyle::Univar => term_copy_unify_vars(vars, term),
        VarNormStyle::Alpha => term_copy_normalize_vars_alpha(vars, term),
        VarNormStyle::None => term_copy(term, vars, None, DerefType::Never),
    }
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

fn lfho_deref_no_whnf(term: &Term, deref: DerefType) -> (Term, DerefType, usize) {
    let limit = lfho_deref_limit(term, deref);
    if deref == DerefType::Once
        && term.is_applied_free_var()
        && term
            .argument(0)
            .is_some_and(|head| head.binding().is_some())
    {
        return (expand_lfho_applied_free_var_once(term), deref, limit);
    }

    let mut current_deref = deref;
    let term = term_deref(term, &mut current_deref);
    (term, current_deref, limit)
}

fn expand_lfho_applied_free_var_once(term: &Term) -> Term {
    assert!(term.is_applied_free_var(), "expected applied free variable");
    assert!(
        term.arity() > 1,
        "applied free variable must have arguments"
    );
    let head = term.argument(0).expect("applied free variable has a head");
    let binding = head.binding().expect("applied variable head is bound");

    if binding.is_any_var() || binding.is_lambda() {
        let expanded = Term::top_alloc(term.f_code(), term.arity());
        expanded.set_properties(term.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        expanded.set_argument(0, binding);
        for index in 1..term.arity() {
            expanded.set_argument(index, initialized_arg(term, index));
        }
        expanded
    } else {
        let expanded = Term::top_alloc(binding.f_code(), binding.arity() + term.arity() - 1);
        expanded.set_properties(binding.give_props(TP_PRED_POS));
        expanded.set_type(term.type_());
        for index in 0..binding.arity() {
            expanded.set_argument(index, initialized_arg(&binding, index));
        }
        for index in 1..term.arity() {
            expanded.set_argument(binding.arity() + index - 1, initialized_arg(term, index));
        }
        expanded
    }
}

fn lfho_deref_limit(term: &Term, deref: DerefType) -> usize {
    if deref != DerefType::Once || !term.is_applied_free_var() {
        return 0;
    }
    let Some(head) = term.argument(0) else {
        return 0;
    };
    let Some(binding) = head.binding() else {
        return 0;
    };
    if binding.is_lambda() {
        1
    } else {
        binding.arity() + usize::from(binding.is_free_var())
    }
}

fn convert_lfho_deref(index: usize, limit: usize, deref: DerefType) -> DerefType {
    if deref == DerefType::Once && index < limit {
        DerefType::Never
    } else {
        deref
    }
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[must_use]
pub fn term_struct_equal_deref(
    left: &Term,
    right: &Term,
    mut left_deref: DerefType,
    mut right_deref: DerefType,
) -> bool {
    let (left, left_current_deref, left_limit) = lfho_deref_no_whnf(left, left_deref);
    let (right, right_current_deref, right_limit) = lfho_deref_no_whnf(right, right_deref);
    left_deref = left_current_deref;
    right_deref = right_current_deref;

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
        .all(|(index, (left, right))| {
            left.zip(right).is_some_and(|(left, right)| {
                term_struct_equal_deref(
                    &left,
                    &right,
                    convert_lfho_deref(index, left_limit, left_deref),
                    convert_lfho_deref(index, right_limit, right_deref),
                )
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

    let (left, left_current_deref, left_limit) = lfho_deref_no_whnf(left, left_deref);
    let (mut right, right_current_deref, right_limit) = lfho_deref_no_whnf(right, right_deref);
    left_deref = left_current_deref;
    right_deref = right_current_deref;
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
        .enumerate()
        .all(|(index, (left, right))| {
            left.zip(right).is_some_and(|(left, right)| {
                term_struct_equal_deref(
                    &left,
                    &right,
                    convert_lfho_deref(index, left_limit, left_deref),
                    convert_lfho_deref(index, right_limit, right_deref),
                )
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
    let (super_term, current_deref, limit) = lfho_deref_no_whnf(super_term, deref);
    deref = current_deref;
    if &super_term == test {
        return true;
    }
    super_term
        .argument_clones()
        .into_iter()
        .enumerate()
        .any(|(index, arg)| {
            arg.is_some_and(|arg| {
                term_is_subterm(&arg, test, convert_lfho_deref(index, limit, deref))
            })
        })
}

#[must_use]
pub fn term_is_subterm_deref(
    super_term: &Term,
    test: &Term,
    mut super_deref: DerefType,
    test_deref: DerefType,
) -> bool {
    let (super_term, current_deref, _limit) = lfho_deref_no_whnf(super_term, super_deref);
    super_deref = current_deref;
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
pub fn term_is_ground(term: &Term) -> bool {
    if term.is_shared() {
        term.query_prop(TP_IS_GROUND)
    } else {
        term_is_ground_compute(term)
    }
}

#[must_use]
pub fn term_find_max_var_code(term: &Term) -> FunCode {
    if term.is_free_var() {
        return term.f_code();
    }
    if term_is_ground(term) {
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
            stack.extend(
                current
                    .argument_clones()
                    .into_iter()
                    .flatten()
                    .filter(|arg| !term_is_ground(arg)),
            );
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

/// Adds occurrences of function symbols with `f_code < limit` to `dist_array`.
///
/// # Panics
///
/// Panics if `dist_array` cannot address a counted f-code, or if a traversed
/// non-variable term has a non-positive f-code.
pub fn term_add_symbol_distribution_limited(term: &Term, dist_array: &mut [i64], limit: usize) {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.is_any_var() {
            let index = positive_symbol_index(current.f_code());
            if index < limit {
                assert!(
                    index < dist_array.len(),
                    "distribution array must cover all counted f-codes"
                );
                dist_array[index] += 1;
            }
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
}

/// Adds occurrences of each visited term's head type to `type_array`.
///
/// Phony applications count the application head type but skip the hidden head
/// cell while traversing children, matching `TermAddTypeDistribution`.
///
/// # Panics
///
/// Panics if a visited term has an unshared type with a negative UID, or if
/// `type_array` cannot address a counted type UID.
pub fn term_add_type_distribution(term: &Term, sig: &mut Signature, type_array: &mut [i64]) {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if let Some(type_) = term_head_type(sig, &current) {
            let index = usize::try_from(type_.type_uid())
                .expect("counted term type must have a non-negative UID");
            assert!(
                index < type_array.len(),
                "type distribution array must cover all counted type UIDs"
            );
            type_array[index] += 1;
        }

        let first_visible_arg = usize::from(current.is_phony_app());
        stack.extend(
            current
                .argument_clones()
                .into_iter()
                .skip(first_visible_arg)
                .flatten(),
        );
    }
}

/// Adds symbol occurrences and records newly seen non-phony function symbols.
///
/// # Panics
///
/// Panics if `dist_array` cannot address a traversed non-variable term f-code,
/// or if such a term has a non-positive f-code.
pub fn term_add_symbol_dist_exist(term: &Term, dist_array: &mut [i64], exists: &mut Vec<FunCode>) {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.is_any_var() {
            let index = positive_symbol_index(current.f_code());
            assert!(
                index < dist_array.len(),
                "distribution array must cover all traversed f-codes"
            );
            if dist_array[index] == 0 && !current.is_phony_app() && !current.is_lambda() {
                exists.push(current.f_code());
            }
            if !(current.is_phony_app() || current.is_lambda()) {
                dist_array[index] += 1;
            }
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
}

fn term_head_type(sig: &mut Signature, term: &Term) -> Option<crate::terms::simpletypes::Type> {
    if term.f_code() == SIG_ITE_CODE {
        debug_assert_eq!(term.arity(), 3);
        term.type_()
    } else if term.f_code() == SIG_LET_CODE {
        term.type_()
    } else if term.f_code() == sig.qex_code() || term.f_code() == sig.qall_code() {
        Some(sig.type_bank().bool_type())
    } else if term.is_applied_any_var() {
        debug_assert_eq!(term.f_code(), SIG_PHONY_APP_CODE);
        term.argument(0).and_then(|head| head.type_())
    } else if term.is_any_var() || term.is_lambda() {
        debug_assert!(!term.is_any_var() || term.arity() == 0);
        term.type_()
    } else if term.f_code() == SIG_PHONY_APP_CODE {
        let head = term.argument(0)?;
        let head_type = term_head_type(sig, &head)?;
        debug_assert!(head_type.is_arrow());
        debug_assert!(head_type.arity() >= 2);
        Some(
            sig.type_bank_mut()
                .insert_type_shared(type_drop_first_arg(&head_type)),
        )
    } else {
        sig.get_type(term.f_code()).cloned()
    }
}

/// Adds symbol frequencies and maximum depths, with out-of-limit symbols in slot 0.
///
/// # Panics
///
/// Panics if `limit` is zero, either array lacks overflow slot 0, either array
/// cannot address a counted in-limit f-code, or a traversed non-variable term
/// has a non-positive f-code.
pub fn term_add_symbol_features_limited(
    term: &Term,
    depth: i64,
    freq_array: &mut [i64],
    depth_array: &mut [i64],
    limit: usize,
) {
    assert!(limit != 0, "feature arrays need slot 0 for overflow");
    assert!(
        !freq_array.is_empty() && !depth_array.is_empty(),
        "feature arrays need slot 0 for overflow"
    );
    if term.is_any_var() {
        return;
    }

    let index = positive_symbol_index(term.f_code());
    if index < limit && !term.is_phony_app() {
        assert!(
            index < freq_array.len() && index < depth_array.len(),
            "feature arrays must cover all counted in-limit f-codes"
        );
        freq_array[index] += 1;
        depth_array[index] = depth_array[index].max(depth);
    } else {
        if !term.is_phony_app() {
            freq_array[0] += 1;
        }
        depth_array[0] = depth_array[0].max(depth);
    }
    for arg in term.argument_clones().into_iter().flatten() {
        term_add_symbol_features_limited(&arg, depth + 1, freq_array, depth_array, limit);
    }
}

/// Adds four-slot symbol features and records first-touched frequency slots.
///
/// The `offset` convention follows C: `0` for positive literals and `2` for
/// negative literals. Each symbol uses slots `4*f_code + offset` for frequency
/// and `4*f_code + offset + 1` for maximum depth.
///
/// # Panics
///
/// Panics if `feature_array` cannot address a touched feature slot, if feature
/// index arithmetic overflows, or if a traversed non-variable, non-phony term
/// has a non-positive f-code.
pub fn term_add_symbol_features(
    term: &Term,
    mod_stack: &mut Vec<usize>,
    depth: i64,
    feature_array: &mut [i64],
    offset: usize,
) {
    if term.is_any_var() {
        return;
    }
    if !term.is_phony_app() {
        let freq_index = symbol_feature_index(term.f_code(), offset);
        let depth_index = freq_index
            .checked_add(1)
            .expect("symbol feature depth index fits in usize");
        assert!(
            depth_index < feature_array.len(),
            "feature array must cover touched symbol slots"
        );
        if feature_array[freq_index] == 0 {
            mod_stack.push(freq_index);
        }
        feature_array[freq_index] += 1;
        feature_array[depth_index] = feature_array[depth_index].max(depth);
    }
    for arg in term.argument_clones().into_iter().flatten() {
        term_add_symbol_features(&arg, mod_stack, depth + 1, feature_array, offset);
    }
}

/// Assigns post-order occurrence ranks to first occurrences of function symbols.
///
/// # Panics
///
/// Panics if `rank_array` cannot address a traversed non-variable term f-code,
/// or if such a term has a non-positive f-code.
pub fn term_compute_function_ranks(term: &Term, rank_array: &mut [i64], count: &mut i64) {
    if term.is_any_var() {
        return;
    }
    for arg in term.argument_clones().into_iter().flatten() {
        term_compute_function_ranks(&arg, rank_array, count);
    }
    if !term.is_phony_app() {
        let index = positive_symbol_index(term.f_code());
        assert!(
            index < rank_array.len(),
            "rank array must cover all traversed f-codes"
        );
        if rank_array[index] == 0 {
            rank_array[index] = *count;
            *count += 1;
        }
    }
}

pub fn term_collect_ground_terms(
    term: &Term,
    result: &mut BTreeMap<usize, Term>,
    all_subterms: bool,
) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.is_free_var() {
            let is_ground = term_is_ground(&current);
            if is_ground
                && !current.is_const()
                && !current.query_prop(TP_PRED_POS)
                && result
                    .insert(term_identity_id(&current), current.clone())
                    .is_none()
            {
                count += 1;
            }
            if !is_ground || all_subterms {
                stack.extend(current.argument_clones().into_iter().flatten());
            }
        }
    }
    count
}

/// Adds newly encountered non-phony function symbols to `res_stack`.
///
/// The dynamic occurrence array is addressed by f-code and grows like C's
/// `PDArray`.
///
/// # Panics
///
/// Panics if a traversed non-variable, non-phony term has a non-positive f-code
/// or one that cannot be represented as a `PDArrayIndex`.
pub fn term_add_fun_occ(
    term: &Term,
    f_occur: &mut PDIntArray,
    res_stack: &mut Vec<FunCode>,
) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.is_any_var() {
            if !current.is_phony_app() {
                let index = positive_symbol_pd_index(current.f_code());
                if f_occur.element_int(index) == 0 {
                    count += 1;
                    res_stack.push(current.f_code());
                    assert!(
                        f_occur.assign(index, 1),
                        "function-occurrence array must cover positive f-codes"
                    );
                }
            }
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

/// Finds the first `$ite` child subterm and writes its position to `pos`.
///
/// The root itself is not considered a match, matching C `TermFindIteSubterm`.
///
/// # Panics
///
/// Panics if a traversed argument slot is uninitialized.
pub fn term_find_ite_subterm(term: &Term, pos: &mut TermPos) -> bool {
    pos.clear();
    let mut path = Vec::new();
    if term_find_ite_subterm_inner(term, &mut path) {
        for (superterm, index) in path {
            pos.push_component(superterm, index);
        }
        true
    } else {
        false
    }
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

/// App-encodes a typed term into binary typed-application symbols.
///
/// # Panics
///
/// Panics if `orig` or an applied argument is missing its inferred type, if a
/// traversed argument slot is uninitialized, or if the inherited prefix
/// invariants from `term_create_prefix` are violated.
pub fn term_app_encode(orig: &Term, sig: &mut Signature) -> Result<Term, Diagnostic> {
    if orig.arity() == 0 {
        return Ok(term_copy_keep_vars(orig, DerefType::Never));
    }

    let arg_num = orig.arg_num();
    assert!(arg_num > 0, "app-encoding requires a logical argument");
    let orig_prefix = term_create_prefix(orig, arg_num - 1);
    let applied_to = orig
        .argument(orig.arity() - 1)
        .expect("app-encoding requires initialized args");

    assert!(
        orig_prefix.is_free_var() || orig_prefix.type_().is_none(),
        "non-variable prefixes are inferred during app-encoding"
    );
    type_infer_sort(sig, &orig_prefix)?;
    let prefix_type = orig_prefix
        .type_()
        .expect("prefix type is inferred during app-encoding");
    let applied_type = applied_to
        .type_()
        .expect("applied argument type is known before app-encoding");
    let ret_type = orig
        .type_()
        .expect("app-encoded term has a known return type");

    let app_code = sig.get_typed_app(&prefix_type, &applied_type, &ret_type);
    let encoded = Term::top_alloc(app_code, 2);
    encoded.set_argument(0, term_app_encode(&orig_prefix, sig)?);
    encoded.set_argument(1, term_app_encode(&applied_to, sig)?);
    Ok(encoded)
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

fn positive_symbol_index(f_code: FunCode) -> usize {
    assert!(f_code > 0, "function symbol f-code must be positive");
    usize::try_from(f_code).expect("positive f-code fits in usize")
}

fn positive_symbol_pd_index(f_code: FunCode) -> PDArrayIndex {
    assert!(f_code > 0, "function symbol f-code must be positive");
    PDArrayIndex::try_from(f_code).expect("positive f-code fits in PDArrayIndex")
}

fn symbol_feature_index(f_code: FunCode, offset: usize) -> usize {
    positive_symbol_index(f_code)
        .checked_mul(4)
        .and_then(|index| index.checked_add(offset))
        .expect("symbol feature index fits in usize")
}

fn term_find_ite_subterm_inner(term: &Term, path: &mut Vec<(Term, usize)>) -> bool {
    if term.is_lambda() {
        return false;
    }
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .expect("ITE-subterm search requires initialized args");
        path.push((term.clone(), index));
        if arg.f_code() == SIG_ITE_CODE || term_find_ite_subterm_inner(&arg, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn create_var_renaming_de_bruijn(vars: &VarBank, term: &Term) -> BTreeMap<FunCode, Term> {
    let mut renaming = BTreeMap::new();
    let mut fresh_var_code = -2;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                renaming.entry(current.f_code())
            {
                let type_ = current.type_().expect("alpha-renamed variables have types");
                entry.insert(vars.var_assert_alloc(fresh_var_code, &type_));
                fresh_var_code -= 2;
            }
        } else {
            for arg in current.argument_clones().into_iter().rev().flatten() {
                stack.push(arg);
            }
        }
    }
    renaming
}

#[cfg(test)]
mod tests {
    use super::{
        term_add_fun_occ, term_add_symbol_dist_exist, term_add_symbol_distribution_limited,
        term_add_symbol_features, term_add_symbol_features_limited, term_add_type_distribution,
        term_app_encode, term_apply_arg, term_array_no_duplicates, term_check_consistency,
        term_collect_fcodes, term_collect_ground_terms, term_collect_variables,
        term_compute_function_ranks, term_compute_order, term_copy, term_copy_keep_vars,
        term_copy_normalize_vars, term_copy_normalize_vars_alpha, term_copy_rename_vars,
        term_copy_unify_vars, term_create_prefix, term_dag_weight, term_depth,
        term_find_ite_subterm, term_find_max_var_code, term_has_f_code, term_has_unbound_variables,
        term_is_db_closed, term_is_def_term, term_is_flat, term_is_ground, term_is_ground_compute,
        term_is_subterm, term_is_subterm_deref, term_is_untyped, term_lex_compare, term_linearize,
        term_non_linear_weight, term_parse, term_parse_arg_list, term_parse_operator,
        term_s_expr_string, term_sig_insert, term_simple_string, term_standard_weight,
        term_struct_equal, term_struct_equal_deref, term_struct_equal_no_deref,
        term_struct_prefix_equal, term_struct_weight_compare, term_sym_type_weight,
        term_weight_compute, var_print_string, VarNormStyle,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::basics::error::ErrorCode;
    use crate::basics::pdarrays::{PDIntArray, GROW_EXPONENTIAL};
    use crate::inout::scanner::Scanner;
    use crate::terms::dbvars::{mk_db, DbVarBank};
    use crate::terms::functypes::FuncSymbType;
    use crate::terms::signature::{
        Signature, FP_INTERPRETED, FP_IS_INTEGER, FP_IS_OBJECT, FP_TYPED_APPLICATION,
        SIG_CONS_CODE, SIG_DB_LAMBDA_CODE, SIG_ITE_CODE, SIG_NAMED_LAMBDA_CODE, SIG_NIL_CODE,
        SIG_PHONY_APP_CODE,
    };
    use crate::terms::simpletypes::{alloc_arrow_type, type_drop_first_arg, Type};
    use crate::terms::termpos::TermPos;
    use crate::terms::termtypes::{
        term_identity_id, DerefType, Term, TP_HAS_DB_SUBTERM, TP_IS_DB_VAR, TP_IS_GROUND,
        TP_IS_SHARED, TP_OP_FLAG, TP_PRED_POS,
    };
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;
    use crate::terms::typecheck::type_infer_sort;
    use std::collections::{BTreeMap, BTreeSet};

    fn typed_var(code: i64, type_: &crate::terms::simpletypes::Type) -> Term {
        let term = Term::const_cell_alloc(code);
        term.set_type(Some(type_.clone()));
        term
    }

    fn type_uid_index(type_: &Type) -> usize {
        usize::try_from(type_.type_uid()).unwrap()
    }

    fn parse_unshared(source: &str) -> (Signature, VarBank, Term) {
        let mut sig = Signature::new(TypeBank::new());
        let vars = VarBank::new(sig.type_bank());
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = term_parse(&mut scanner, &mut sig, &vars).unwrap();
        (sig, vars, term)
    }

    fn parse_unshared_with_lists(source: &str) -> (Signature, VarBank, Term) {
        let mut sig = Signature::new_with_list_support(TypeBank::new(), true);
        let vars = VarBank::new(sig.type_bank());
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = term_parse(&mut scanner, &mut sig, &vars).unwrap();
        (sig, vars, term)
    }

    struct AppliedPrefixFixture {
        app: Term,
        y: Term,
        b: Term,
        c: Term,
        expected: Term,
        fully_derefed: Term,
        prefix_expected: Term,
        prefix_fully_derefed: Term,
    }

    fn applied_prefix_fixture() -> AppliedPrefixFixture {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let f_code = 200;

        let b = Term::const_cell_alloc(201);
        b.set_type(Some(i_type.clone()));
        let c = Term::const_cell_alloc(202);
        c.set_type(Some(i_type.clone()));

        let y = typed_var(-4, &i_type);
        y.set_binding(Some(b.clone()));
        let z = typed_var(-6, &i_type);
        z.set_binding(Some(c.clone()));

        let head_binding = Term::top_alloc(f_code, 1);
        head_binding.set_type(Some(i_type.clone()));
        head_binding.set_argument(0, y.clone());
        let head = typed_var(-2, &i_type);
        head.set_binding(Some(head_binding));

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type.clone()));
        app.set_argument(0, head);
        app.set_argument(1, z);

        let expected = Term::top_alloc(f_code, 2);
        expected.set_type(Some(i_type.clone()));
        expected.set_argument(0, y.clone());
        expected.set_argument(1, c.clone());

        let fully_derefed = Term::top_alloc(f_code, 2);
        fully_derefed.set_type(Some(i_type.clone()));
        fully_derefed.set_argument(0, b.clone());
        fully_derefed.set_argument(1, c.clone());

        let prefix_expected = Term::top_alloc(f_code, 1);
        prefix_expected.set_type(Some(i_type.clone()));
        prefix_expected.set_argument(0, y.clone());

        let prefix_fully_derefed = Term::top_alloc(f_code, 1);
        prefix_fully_derefed.set_type(Some(i_type));
        prefix_fully_derefed.set_argument(0, b.clone());

        AppliedPrefixFixture {
            app,
            y,
            b,
            c,
            expected,
            fully_derefed,
            prefix_expected,
            prefix_fully_derefed,
        }
    }

    #[test]
    fn term_copy_allocates_free_variables_from_target_bank() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let source_var = typed_var(-2, &i_type);
        let root = Term::top_alloc(10, 2);
        root.set_type(Some(i_type.clone()));
        root.set_prop(TP_IS_SHARED | TP_PRED_POS);
        root.set_argument(0, source_var.clone());
        root.set_argument(1, source_var.clone());

        let target_vars = VarBank::new(&types);
        let copy = term_copy(&root, &target_vars, None, DerefType::Never);

        assert_ne!(copy, root);
        assert_eq!(copy.f_code(), 10);
        assert_eq!(copy.type_(), Some(i_type.clone()));
        assert!(copy.query_prop(TP_PRED_POS));
        assert!(!copy.query_prop(TP_IS_SHARED));

        let first = copy.argument(0).unwrap();
        let second = copy.argument(1).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, source_var);
        assert_eq!(first.f_code(), -2);
        assert_eq!(first.type_(), Some(i_type));
        assert_eq!(target_vars.f_code_find(-2), Some(first));
    }

    #[test]
    fn term_copy_handles_db_vars_and_dereferencing_like_c() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let db = mk_db(3, &i_type);
        let root = Term::top_alloc(10, 1);
        root.set_argument(0, db.clone());
        let target_vars = VarBank::new(&types);

        let copy_without_db_bank = term_copy(&root, &target_vars, None, DerefType::Never);
        assert_ne!(copy_without_db_bank, root);
        assert_eq!(copy_without_db_bank.argument(0), Some(db.clone()));

        let mut db_bank = DbVarBank::new();
        let copy_with_db_bank =
            term_copy(&root, &target_vars, Some(&mut db_bank), DerefType::Never);
        let copied_db = copy_with_db_bank.argument(0).unwrap();
        assert_ne!(copied_db, db);
        assert!(copied_db.is_db_var());
        assert_eq!(copied_db.f_code(), 3);
        assert_eq!(db_bank.len(), 1);

        let repeated = term_copy(&root, &target_vars, Some(&mut db_bank), DerefType::Never);
        assert_eq!(repeated.argument(0), Some(copied_db));

        let bound_var = typed_var(-4, &i_type);
        let bound = Term::const_cell_alloc(99);
        bound.set_type(Some(i_type));
        bound_var.set_binding(Some(bound.clone()));
        assert_eq!(term_copy_keep_vars(&bound_var, DerefType::Never), bound_var);
        let derefed = term_copy_keep_vars(&bound_var, DerefType::Once);
        assert_ne!(derefed, bound);
        assert_eq!(derefed.f_code(), 99);
        assert!(derefed.is_const());
    }

    #[test]
    fn term_copy_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture();
        let target_vars = VarBank::new(&TypeBank::new());

        let copied = term_copy(&fixture.app, &target_vars, None, DerefType::Once);

        assert_eq!(copied.f_code(), fixture.expected.f_code());
        assert_eq!(copied.arity(), 2);
        let prefix = copied.argument(0).unwrap();
        assert!(prefix.is_free_var());
        assert_eq!(prefix.f_code(), fixture.y.f_code());
        assert_ne!(prefix, fixture.y);
        assert!(prefix.binding().is_none());
        assert_eq!(target_vars.f_code_find(fixture.y.f_code()), Some(prefix));

        let suffix = copied.argument(1).unwrap();
        assert!(suffix.is_const());
        assert_eq!(suffix.f_code(), fixture.c.f_code());
        assert_ne!(suffix, fixture.c);
        assert_ne!(copied.argument(0), Some(fixture.b));
    }

    #[test]
    fn term_copy_keep_vars_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture();

        let copied = term_copy_keep_vars(&fixture.app, DerefType::Once);

        assert_eq!(copied.f_code(), fixture.expected.f_code());
        assert_eq!(copied.arity(), 2);
        assert_eq!(copied.argument(0), Some(fixture.y.clone()));
        let suffix = copied.argument(1).unwrap();
        assert!(suffix.is_const());
        assert_eq!(suffix.f_code(), fixture.c.f_code());
        assert_ne!(suffix, fixture.c);
        assert_ne!(copied.argument(0), Some(fixture.b));
    }

    #[test]
    fn term_check_consistency_allows_shared_dag_subterms() {
        let shared = Term::const_cell_alloc(301);
        let root = Term::top_alloc(300, 2);
        root.set_argument(0, shared.clone());
        root.set_argument(1, shared);

        assert_eq!(term_check_consistency(&root, DerefType::Never), None);
    }

    #[test]
    fn term_check_consistency_reports_branch_cycles() {
        let root = Term::top_alloc(310, 1);
        root.set_argument(0, root.clone());

        assert_eq!(term_check_consistency(&root, DerefType::Never), Some(root));
    }

    #[test]
    fn term_check_consistency_handles_applied_deref_prefixes() {
        let fixture = applied_prefix_fixture();

        assert_eq!(term_check_consistency(&fixture.app, DerefType::Once), None);
    }

    #[test]
    fn term_copy_normalize_vars_alpha_uses_left_to_right_first_occurrence_order() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let bool_type = types.bool_type();
        let y = typed_var(-4, &bool_type);
        let x = typed_var(-2, &i_type);
        let db = mk_db(0, &i_type);
        let root = Term::top_alloc(10, 4);
        root.set_argument(0, y.clone());
        root.set_argument(1, x.clone());
        root.set_argument(2, y);
        root.set_argument(3, db.clone());

        let target_vars = VarBank::new(&types);
        let copy = term_copy_normalize_vars_alpha(&target_vars, &root);

        let first = copy.argument(0).unwrap();
        let second = copy.argument(1).unwrap();
        let repeated = copy.argument(2).unwrap();
        assert_eq!(first.f_code(), -2);
        assert_eq!(first.type_(), Some(bool_type));
        assert_eq!(second.f_code(), -4);
        assert_eq!(second.type_(), Some(i_type));
        assert_eq!(repeated, first);
        assert_eq!(copy.argument(3), Some(db));
    }

    #[test]
    fn term_copy_unify_vars_and_dispatcher_follow_c_var_norm_styles() {
        let types = TypeBank::new();
        let i_type = types.i_type();
        let bool_type = types.bool_type();
        let bool_var = typed_var(-6, &bool_type);
        let i_var = typed_var(-8, &i_type);
        let root = Term::top_alloc(10, 2);
        root.set_argument(0, bool_var.clone());
        root.set_argument(1, i_var);

        let univar_bank = VarBank::new(&types);
        let univar = term_copy_unify_vars(&univar_bank, &root);
        let first = univar.argument(0).unwrap();
        let second = univar.argument(1).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.f_code(), -2);
        assert_eq!(first.type_(), Some(i_type.clone()));

        let none_bank = VarBank::new(&types);
        let copied = term_copy_normalize_vars(&none_bank, &bool_var, VarNormStyle::None);
        assert_eq!(copied.f_code(), -6);
        assert_eq!(copied.type_(), Some(bool_type.clone()));

        let alpha_bank = VarBank::new(&types);
        let alpha = term_copy_normalize_vars(&alpha_bank, &bool_var, VarNormStyle::Alpha);
        assert_eq!(alpha.f_code(), -2);
        assert_eq!(alpha.type_(), Some(bool_type));

        let mut renaming = BTreeMap::new();
        let replacement = typed_var(-20, &i_type);
        renaming.insert(-6, replacement.clone());
        assert_eq!(term_copy_rename_vars(&renaming, &bool_var), replacement);
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
    fn term_parse_reads_list_literals_when_signature_supports_lists() {
        let (sig, vars, term) = parse_unshared_with_lists("[a,X,[b]]");

        assert_eq!(term.f_code(), SIG_CONS_CODE);
        assert_eq!(term.arity(), 2);
        assert_eq!(sig.find_name(term.argument(0).unwrap().f_code()), Some("a"));
        let tail = term.argument(1).unwrap();
        assert_eq!(tail.f_code(), SIG_CONS_CODE);
        assert_eq!(tail.argument(0).unwrap(), vars.ext_name_find("X").unwrap());
        let nested = tail.argument(1).unwrap().argument(0).unwrap();
        assert_eq!(nested.f_code(), SIG_CONS_CODE);
        assert_eq!(
            term_simple_string(&term, &sig),
            "$cons(a,$cons(X1,$cons($cons(b,$nil),$nil)))"
        );
    }

    #[test]
    fn term_parse_reads_empty_list_literal() {
        let (sig, _vars, term) = parse_unshared_with_lists("[]");

        assert_eq!(term.f_code(), SIG_NIL_CODE);
        assert_eq!(term.arity(), 0);
        assert_eq!(term_simple_string(&term, &sig), "$nil");
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
        assert_eq!(VarNormStyle::from_c_value(-1), Some(VarNormStyle::None));
        assert_eq!(VarNormStyle::from_c_value(0), Some(VarNormStyle::Univar));
        assert_eq!(VarNormStyle::from_c_value(1), Some(VarNormStyle::Alpha));
        assert_eq!(VarNormStyle::from_c_value(2), None);
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
    fn structural_deref_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture();

        assert!(term_struct_equal_deref(
            &fixture.app,
            &fixture.expected,
            DerefType::Once,
            DerefType::Never
        ));
        assert!(!term_struct_equal_deref(
            &fixture.app,
            &fixture.fully_derefed,
            DerefType::Once,
            DerefType::Never
        ));
    }

    #[test]
    fn structural_prefix_equal_preserves_applied_binding_prefix() {
        let fixture = applied_prefix_fixture();

        assert!(term_struct_prefix_equal(
            &fixture.prefix_expected,
            &fixture.app,
            DerefType::Never,
            DerefType::Once,
            1
        ));
        assert!(!term_struct_prefix_equal(
            &fixture.prefix_fully_derefed,
            &fixture.app,
            DerefType::Never,
            DerefType::Once,
            1
        ));
    }

    #[test]
    fn subterm_check_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture();

        assert!(term_is_subterm(&fixture.app, &fixture.y, DerefType::Once));
        assert!(term_is_subterm(&fixture.app, &fixture.c, DerefType::Once));
        assert!(!term_is_subterm(&fixture.app, &fixture.b, DerefType::Once));
    }

    #[test]
    fn subterm_deref_keeps_c_same_deref_descendant_behavior() {
        let fixture = applied_prefix_fixture();

        assert!(term_is_subterm_deref(
            &fixture.app,
            &fixture.b,
            DerefType::Once,
            DerefType::Never
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
    fn symbol_distribution_features_and_ranks_follow_c_traversals() {
        let root = Term::top_alloc(10, 2);
        let nested = Term::top_alloc(20, 1);
        let leaf = Term::const_cell_alloc(30);
        let var = Term::const_cell_alloc(-2);
        nested.set_argument(0, leaf.clone());
        root.set_argument(0, nested);
        root.set_argument(1, var.clone());

        let mut limited = vec![0; 25];
        term_add_symbol_distribution_limited(&root, &mut limited, 25);
        assert_eq!(limited[10], 1);
        assert_eq!(limited[20], 1);
        assert_eq!(limited[0], 0);

        let mut dist = vec![0; 40];
        let mut exists = Vec::new();
        term_add_symbol_dist_exist(&root, &mut dist, &mut exists);
        assert_eq!(exists, vec![10, 20, 30]);
        assert_eq!(dist[10], 1);
        assert_eq!(dist[20], 1);
        assert_eq!(dist[30], 1);

        let mut freq = vec![0; 25];
        let mut depth = vec![0; 25];
        term_add_symbol_features_limited(&root, 0, &mut freq, &mut depth, 25);
        assert_eq!(freq[10], 1);
        assert_eq!(depth[10], 0);
        assert_eq!(freq[20], 1);
        assert_eq!(depth[20], 1);
        assert_eq!(freq[0], 1);
        assert_eq!(depth[0], 2);

        let mut ranks = vec![0; 40];
        let mut count = 1;
        term_compute_function_ranks(&root, &mut ranks, &mut count);
        assert_eq!(ranks[30], 1);
        assert_eq!(ranks[20], 2);
        assert_eq!(ranks[10], 3);
        assert_eq!(count, 4);

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, var);
        app.set_argument(1, leaf);
        let mut dist = vec![0; 40];
        let mut exists = Vec::new();
        term_add_symbol_dist_exist(&app, &mut dist, &mut exists);
        assert_eq!(dist[usize::try_from(SIG_PHONY_APP_CODE).unwrap()], 0);
        assert_eq!(exists, vec![30]);
    }

    #[test]
    fn type_distribution_counts_head_types_and_skips_phony_heads() {
        let mut sig = Signature::new(TypeBank::new());
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let unary_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let f_code = sig.insert_id("f", 1, true);
        sig.declare_final_type(f_code, unary_type.clone()).unwrap();

        let root = Term::top_alloc(f_code, 1);
        root.set_type(Some(individual.clone()));
        root.set_argument(0, typed_var(-2, &individual));
        let mut type_dist = vec![0; usize::try_from(sig.type_bank().types_count() + 8).unwrap()];
        term_add_type_distribution(&root, &mut sig, &mut type_dist);
        assert_eq!(type_dist[type_uid_index(&unary_type)], 1);
        assert_eq!(type_dist[type_uid_index(&individual)], 1);

        let lambda_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                bool_type,
            ]));
        let lambda = Term::top_alloc(SIG_NAMED_LAMBDA_CODE, 0);
        lambda.set_type(Some(lambda_type.clone()));
        let argument = Term::top_alloc(f_code, 1);
        argument.set_type(Some(individual.clone()));
        argument.set_argument(0, typed_var(-4, &individual));
        let phony_app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        phony_app.set_argument(0, lambda);
        phony_app.set_argument(1, argument);

        let before_drop_insert = sig.type_bank().types_count();
        let mut type_dist = vec![0; usize::try_from(before_drop_insert + 8).unwrap()];
        term_add_type_distribution(&phony_app, &mut sig, &mut type_dist);
        let dropped_type = sig
            .type_bank_mut()
            .insert_type_shared(type_drop_first_arg(&lambda_type));

        assert!(dropped_type.type_uid() > before_drop_insert);
        assert_eq!(type_dist[type_uid_index(&dropped_type)], 1);
        assert_eq!(type_dist[type_uid_index(&lambda_type)], 0);
        assert_eq!(type_dist[type_uid_index(&unary_type)], 1);
        assert_eq!(type_dist[type_uid_index(&individual)], 1);
    }

    #[test]
    fn symbol_feature_and_fun_occ_helpers_preserve_c_stack_shapes() {
        let root = Term::top_alloc(10, 2);
        let nested = Term::top_alloc(20, 1);
        let leaf = Term::const_cell_alloc(30);
        nested.set_argument(0, leaf.clone());
        root.set_argument(0, nested);
        root.set_argument(1, leaf);

        let mut feature_array = vec![0; 128];
        let mut mod_stack = Vec::new();
        term_add_symbol_features(&root, &mut mod_stack, 0, &mut feature_array, 2);
        assert_eq!(mod_stack, vec![42, 82, 122]);
        assert_eq!(feature_array[42], 1);
        assert_eq!(feature_array[43], 0);
        assert_eq!(feature_array[82], 1);
        assert_eq!(feature_array[83], 1);
        assert_eq!(feature_array[122], 2);
        assert_eq!(feature_array[123], 2);

        term_add_symbol_features(&root, &mut mod_stack, 0, &mut feature_array, 2);
        assert_eq!(mod_stack, vec![42, 82, 122]);
        assert_eq!(feature_array[42], 2);
        assert_eq!(feature_array[122], 4);

        let mut f_occur = PDIntArray::new_int(1, GROW_EXPONENTIAL);
        let mut occ_stack = Vec::new();
        assert_eq!(term_add_fun_occ(&root, &mut f_occur, &mut occ_stack), 3);
        assert_eq!(occ_stack, vec![10, 30, 20]);
        assert_eq!(f_occur.element_int(10), 1);
        assert_eq!(f_occur.element_int(20), 1);
        assert_eq!(f_occur.element_int(30), 1);

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, Term::const_cell_alloc(-2));
        app.set_argument(1, Term::const_cell_alloc(30));
        let mut f_occur = PDIntArray::new_int(1, GROW_EXPONENTIAL);
        let mut occ_stack = Vec::new();
        assert_eq!(term_add_fun_occ(&app, &mut f_occur, &mut occ_stack), 1);
        assert_eq!(occ_stack, vec![30]);
        assert_eq!(
            f_occur.element_int(isize::try_from(SIG_PHONY_APP_CODE).unwrap()),
            0
        );
    }

    #[test]
    fn ground_collection_uses_cached_ground_and_predicate_filter() {
        let root = Term::top_alloc(10, 2);
        let nested = Term::top_alloc(20, 1);
        let leaf = Term::const_cell_alloc(30);
        nested.set_argument(0, leaf.clone());
        root.set_argument(0, nested.clone());
        root.set_argument(1, Term::const_cell_alloc(31));

        let mut maximal = BTreeMap::new();
        assert_eq!(term_collect_ground_terms(&root, &mut maximal, false), 1);
        assert!(maximal.contains_key(&term_identity_id(&root)));
        assert!(!maximal.contains_key(&term_identity_id(&nested)));

        let mut all = BTreeMap::new();
        assert_eq!(term_collect_ground_terms(&root, &mut all, true), 2);
        assert!(all.contains_key(&term_identity_id(&root)));
        assert!(all.contains_key(&term_identity_id(&nested)));
        assert!(!all.contains_key(&term_identity_id(&leaf)));

        nested.set_prop(TP_PRED_POS);
        let mut filtered = BTreeMap::new();
        assert_eq!(term_collect_ground_terms(&root, &mut filtered, true), 1);
        assert!(filtered.contains_key(&term_identity_id(&root)));
        assert!(!filtered.contains_key(&term_identity_id(&nested)));

        let cached = Term::top_alloc(40, 1);
        let hidden_var = Term::const_cell_alloc(-4);
        cached.set_argument(0, hidden_var);
        cached.set_prop(TP_IS_SHARED | TP_IS_GROUND);
        assert!(!term_is_ground_compute(&cached));
        assert!(term_is_ground(&cached));
        assert_eq!(term_find_max_var_code(&cached), 0);

        let mut vars = BTreeMap::new();
        assert_eq!(term_collect_variables(&cached, &mut vars), 1);

        let parent = Term::top_alloc(50, 1);
        parent.set_argument(0, cached.clone());
        let mut vars = BTreeMap::new();
        assert_eq!(term_collect_variables(&parent, &mut vars), 0);

        let mut cached_terms = BTreeMap::new();
        assert_eq!(
            term_collect_ground_terms(&cached, &mut cached_terms, false),
            1
        );
        assert!(cached_terms.contains_key(&term_identity_id(&cached)));
    }

    #[test]
    fn ite_subterm_search_records_first_child_position_and_skips_lambdas() {
        let ite = Term::top_alloc(SIG_ITE_CODE, 3);
        ite.set_argument(0, Term::const_cell_alloc(1));
        ite.set_argument(1, Term::const_cell_alloc(2));
        ite.set_argument(2, Term::const_cell_alloc(3));

        let root = Term::top_alloc(10, 2);
        root.set_argument(0, Term::const_cell_alloc(4));
        root.set_argument(1, ite.clone());
        let mut pos = TermPos::new();
        assert!(term_find_ite_subterm(&root, &mut pos));
        assert_eq!(pos.print_string(), "1");
        assert_eq!(pos.get_subterm(&root), ite);

        let deep_ite = Term::top_alloc(SIG_ITE_CODE, 3);
        deep_ite.set_argument(0, Term::const_cell_alloc(5));
        deep_ite.set_argument(1, Term::const_cell_alloc(6));
        deep_ite.set_argument(2, Term::const_cell_alloc(7));
        let wrapper = Term::top_alloc(25, 1);
        wrapper.set_argument(0, deep_ite.clone());
        let later_ite = Term::top_alloc(SIG_ITE_CODE, 3);
        let root = Term::top_alloc(30, 2);
        root.set_argument(0, wrapper);
        root.set_argument(1, later_ite);
        assert!(term_find_ite_subterm(&root, &mut pos));
        assert_eq!(pos.print_string(), "0.0\n");
        assert_eq!(pos.get_subterm(&root), deep_ite);

        let hidden_ite = Term::top_alloc(SIG_ITE_CODE, 3);
        let lambda = Term::top_alloc(SIG_DB_LAMBDA_CODE, 1);
        lambda.set_argument(0, hidden_ite);
        let visible_ite = Term::top_alloc(SIG_ITE_CODE, 3);
        let root = Term::top_alloc(40, 2);
        root.set_argument(0, lambda);
        root.set_argument(1, visible_ite.clone());
        assert!(term_find_ite_subterm(&root, &mut pos));
        assert_eq!(pos.print_string(), "1");
        assert_eq!(pos.get_subterm(&root), visible_ite);

        pos.push_component(root.clone(), 0);
        assert!(!term_find_ite_subterm(
            &Term::top_alloc(SIG_ITE_CODE, 0),
            &mut pos
        ));
        assert!(pos.is_top_pos());
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
    fn term_app_encode_builds_binary_typed_application_tree() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let f_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                i_type.clone(),
                i_type.clone(),
                i_type.clone(),
            ]));
        let f_code = sig.insert_id("f", 2, false);
        sig.declare_final_type(f_code, f_type).unwrap();
        let a_code = sig.insert_id("a", 0, false);
        sig.declare_final_type(a_code, i_type.clone()).unwrap();
        let b_code = sig.insert_id("b", 0, false);
        sig.declare_final_type(b_code, i_type.clone()).unwrap();

        let a = Term::const_cell_alloc(a_code);
        a.set_type(Some(i_type.clone()));
        let b = Term::const_cell_alloc(b_code);
        b.set_type(Some(i_type.clone()));
        let root = Term::top_alloc(f_code, 2);
        root.set_argument(0, a.clone());
        root.set_argument(1, b.clone());
        type_infer_sort(&mut sig, &root).unwrap();

        let encoded = term_app_encode(&root, &mut sig).unwrap();

        assert_eq!(encoded.arity(), 2);
        assert!(sig.query_prop(encoded.f_code(), FP_TYPED_APPLICATION));
        let left = encoded.argument(0).unwrap();
        let right = encoded.argument(1).unwrap();
        assert!(sig.query_prop(left.f_code(), FP_TYPED_APPLICATION));
        assert_eq!(left.argument(0).unwrap().f_code(), f_code);
        assert_eq!(left.argument(1).unwrap().f_code(), a_code);
        assert_eq!(right.f_code(), b_code);

        let encoded_const = term_app_encode(&a, &mut sig).unwrap();
        assert_ne!(encoded_const, a);
        assert_eq!(encoded_const.f_code(), a_code);
        assert_eq!(encoded_const.type_(), Some(i_type));
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
