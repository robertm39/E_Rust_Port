use crate::basics::error::{Diagnostic, ErrorCode};
use std::cell::Cell;
use std::rc::Rc;

pub type TypeConsCode = i64;
pub type TypeUniqueId = i64;

pub const ARROW_TYPE_CONS: TypeConsCode = 0;
pub const ST_BOOL: TypeConsCode = 1;
pub const ST_INDIVIDUALS: TypeConsCode = 2;
pub const ST_KIND: TypeConsCode = 3;
pub const ST_INTEGER: TypeConsCode = 4;
pub const ST_RATIONAL: TypeConsCode = 5;
pub const ST_REAL: TypeConsCode = 6;
pub const ST_PREDEFINED: TypeConsCode = ST_REAL;
pub const INVALID_TYPE_UID: TypeUniqueId = -1;

#[derive(Clone, Debug)]
pub struct Type(Rc<TypeCell>);

#[derive(Debug)]
struct TypeCell {
    f_code: TypeConsCode,
    args: Vec<Type>,
    type_uid: Cell<TypeUniqueId>,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Type {}

impl Type {
    #[must_use]
    pub fn f_code(&self) -> TypeConsCode {
        self.0.f_code
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.0.args.len()
    }

    #[must_use]
    pub fn args(&self) -> &[Type] {
        &self.0.args
    }

    #[must_use]
    pub fn type_uid(&self) -> TypeUniqueId {
        self.0.type_uid.get()
    }

    pub fn set_type_uid(&self, type_uid: TypeUniqueId) {
        self.0.type_uid.set(type_uid);
    }

    #[must_use]
    pub fn is_arrow(&self) -> bool {
        self.f_code() == ARROW_TYPE_CONS
    }

    #[must_use]
    pub fn is_kind(&self) -> bool {
        self.f_code() == ST_KIND
    }

    #[must_use]
    pub fn is_bool(&self) -> bool {
        self.f_code() == ST_BOOL
    }

    #[must_use]
    pub fn is_individual(&self) -> bool {
        self.f_code() == ST_INDIVIDUALS
    }
}

#[must_use]
pub fn sort_is_user_defined(sort: TypeConsCode) -> bool {
    sort > ST_PREDEFINED
}

#[must_use]
pub fn sort_is_interpreted(sort: TypeConsCode) -> bool {
    (ST_INTEGER..=ST_PREDEFINED).contains(&sort)
}

#[must_use]
pub fn alloc_simple_sort(code: TypeConsCode) -> Type {
    type_alloc(code, Vec::new())
}

/// Allocates an arrow type, returning the sole argument unchanged for arity 1.
///
/// # Panics
///
/// Panics when called with no arguments, matching the C `AllocArrowType`
/// assertion that arrow arity is positive.
#[must_use]
pub fn alloc_arrow_type(args: Vec<Type>) -> Type {
    assert!(!args.is_empty(), "arrow type arity must be positive");
    if args.len() == 1 {
        args[0].clone()
    } else {
        type_alloc(ARROW_TYPE_CONS, args)
    }
}

#[must_use]
pub fn alloc_arrow_type_copy_args(args: &[Type]) -> Type {
    alloc_arrow_type(args.to_vec())
}

pub(crate) fn type_alloc(f_code: TypeConsCode, args: Vec<Type>) -> Type {
    Type(Rc::new(TypeCell {
        f_code,
        args,
        type_uid: Cell::new(INVALID_TYPE_UID),
    }))
}

#[must_use]
pub fn type_is_predicate(type_: &Type) -> bool {
    type_.is_bool() || (type_.is_arrow() && type_.args().last().is_some_and(Type::is_bool))
}

#[must_use]
pub fn type_is_type_constructor(type_: &Type) -> bool {
    type_.is_kind() || (type_.is_arrow() && type_.args().first().is_some_and(Type::is_kind))
}

#[must_use]
pub fn get_ret_type(type_: &Type) -> Type {
    if type_.is_arrow() {
        type_.args()[type_.arity() - 1].clone()
    } else {
        type_.clone()
    }
}

#[must_use]
pub fn type_get_order(type_: &Type) -> usize {
    if !type_.is_arrow() {
        return 0;
    }
    debug_assert!(!get_ret_type(type_).is_arrow());
    type_.args().iter().map(type_get_order).max().unwrap_or(0) + 1
}

#[must_use]
pub fn var_order(type_: &Type) -> usize {
    type_get_order(type_) + usize::from(type_.is_arrow())
}

#[must_use]
pub fn is_flattened(type_: &Type) -> bool {
    for arg in type_.args().iter().take(type_.arity().saturating_sub(1)) {
        if !is_flattened(arg) {
            return false;
        }
    }
    type_.arity() == 0 || !type_.args()[type_.arity() - 1].is_arrow()
}

#[must_use]
pub fn arguments_flattened(type_: &Type) -> bool {
    type_
        .args()
        .iter()
        .take(type_.arity().saturating_sub(1))
        .all(is_flattened)
}

#[must_use]
pub fn type_is_untyped(type_: &Type) -> bool {
    if !type_.is_arrow() {
        return type_.is_bool() || type_.is_individual();
    }
    type_.args().iter().all(type_is_untyped)
}

#[must_use]
pub fn type_copy(type_: &Type) -> Type {
    type_alloc(type_.f_code(), type_.args().to_vec())
}

#[must_use]
pub fn types_cmp(left: &Type, right: &Type) -> i32 {
    let f_code_cmp = cmp_i64(left.f_code(), right.f_code());
    if f_code_cmp != 0 {
        return f_code_cmp;
    }

    let arity_cmp = cmp_usize(left.arity(), right.arity());
    if arity_cmp != 0 {
        return arity_cmp;
    }

    for (left_arg, right_arg) in left.args().iter().zip(right.args()) {
        let arg_cmp = pointer_cmp(left_arg, right_arg);
        if arg_cmp != 0 {
            return arg_cmp;
        }
    }
    0
}

#[must_use]
pub fn type_identity_cmp(left: &Type, right: &Type) -> i32 {
    pointer_cmp(left, right)
}

#[must_use]
pub fn flatten_type(type_: &Type) -> Type {
    debug_assert!(arguments_flattened(type_));
    if !type_.is_arrow() {
        return type_.clone();
    }
    let Some(last) = type_.args().last() else {
        return type_.clone();
    };
    if !last.is_arrow() {
        return type_.clone();
    }

    let mut args = Vec::with_capacity(type_.arity() - 1 + last.arity());
    args.extend(type_.args()[..type_.arity() - 1].iter().cloned());
    args.extend(last.args().iter().cloned());
    alloc_arrow_type(args)
}

pub fn type_app_encoded_name(type_: &Type) -> Result<String, Diagnostic> {
    if sort_is_user_defined(type_.f_code()) || type_.is_arrow() {
        let type_uid = type_.type_uid();
        if type_uid == INVALID_TYPE_UID {
            Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Type UID is not initialized",
            ))
        } else {
            Ok(format!("type_{type_uid}"))
        }
    } else {
        get_builtin_name(type_)
            .map(str::to_owned)
            .ok_or_else(|| Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Type is not built-in"))
    }
}

#[must_use]
pub fn type_get_max_arity(type_: &Type) -> usize {
    if type_.is_arrow() {
        type_.arity() - 1
    } else {
        0
    }
}

#[must_use]
pub fn type_has_bool(type_: &Type) -> bool {
    type_.is_bool() || type_.args().iter().any(type_has_bool)
}

#[must_use]
pub fn arrow_type_flattened(args: &[Type], ret: &Type) -> Type {
    if args.is_empty() {
        return ret.clone();
    }

    let mut args_ret = Vec::with_capacity(args.len() + 1);
    args_ret.extend(args.iter().cloned());
    args_ret.push(ret.clone());
    flatten_type(&alloc_arrow_type(args_ret))
}

/// Drops the first argument from an arrow type.
///
/// # Panics
///
/// Panics when `type_` is not an arrow type, or when the arrow shape has fewer
/// than two entries. The C helper encodes the same preconditions as assertions.
#[must_use]
pub fn type_drop_first_arg(type_: &Type) -> Type {
    assert!(type_.is_arrow(), "expected arrow type");
    if type_.arity() == 2 {
        type_.args()[1].clone()
    } else {
        assert!(type_.arity() >= 3, "arrow type arity must be at least 2");
        alloc_arrow_type(type_.args()[1..].to_vec())
    }
}

#[must_use]
pub fn is_choice_type(type_: &Type) -> bool {
    if !type_.is_arrow() || type_.arity() < 2 {
        return false;
    }

    let predicate = &type_.args()[0];
    if !(predicate.is_arrow() && predicate.arity() == 2 && type_is_predicate(predicate)) {
        return false;
    }

    let a_type = &predicate.args()[0];
    if a_type.is_arrow() && a_type.arity() == type_.arity() - 1 {
        a_type
            .args()
            .iter()
            .zip(&type_.args()[1..])
            .all(|(left, right)| left == right)
    } else {
        !a_type.is_arrow() && type_.arity() == 2 && a_type == &type_.args()[1]
    }
}

fn get_builtin_name(type_: &Type) -> Option<&'static str> {
    if sort_is_user_defined(type_.f_code()) || type_.is_arrow() {
        return None;
    }
    match type_.f_code() {
        ST_BOOL => Some("$o"),
        ST_INDIVIDUALS => Some("$i"),
        ST_KIND => Some("$tType"),
        ST_INTEGER => Some("$int"),
        ST_RATIONAL => Some("$rat"),
        ST_REAL => Some("$real"),
        _ => None,
    }
}

fn cmp_i64(left: i64, right: i64) -> i32 {
    i32::from(left > right) - i32::from(left < right)
}

fn cmp_usize(left: usize, right: usize) -> i32 {
    i32::from(left > right) - i32::from(left < right)
}

fn pointer_cmp(left: &Type, right: &Type) -> i32 {
    let left_ptr = Rc::as_ptr(&left.0).cast::<()>() as usize;
    let right_ptr = Rc::as_ptr(&right.0).cast::<()>() as usize;
    cmp_usize(left_ptr, right_ptr)
}

#[cfg(test)]
mod tests {
    use super::{
        alloc_arrow_type, alloc_arrow_type_copy_args, alloc_simple_sort, arguments_flattened,
        arrow_type_flattened, flatten_type, get_ret_type, is_choice_type, is_flattened,
        sort_is_interpreted, sort_is_user_defined, type_app_encoded_name, type_copy,
        type_get_max_arity, type_get_order, type_has_bool, type_is_predicate,
        type_is_type_constructor, type_is_untyped, types_cmp, var_order, ARROW_TYPE_CONS,
        INVALID_TYPE_UID, ST_BOOL, ST_INDIVIDUALS, ST_INTEGER, ST_KIND, ST_PREDEFINED, ST_RATIONAL,
        ST_REAL,
    };
    use crate::basics::error::ErrorCode;

    fn bool_sort() -> super::Type {
        alloc_simple_sort(ST_BOOL)
    }

    fn individual_sort() -> super::Type {
        alloc_simple_sort(ST_INDIVIDUALS)
    }

    #[test]
    fn constants_and_basic_sort_predicates_match_c_header() {
        assert_eq!(ARROW_TYPE_CONS, 0);
        assert_eq!(ST_BOOL, 1);
        assert_eq!(ST_INDIVIDUALS, 2);
        assert_eq!(ST_KIND, 3);
        assert_eq!(ST_INTEGER, 4);
        assert_eq!(ST_RATIONAL, 5);
        assert_eq!(ST_REAL, 6);
        assert_eq!(ST_PREDEFINED, ST_REAL);
        assert!(sort_is_user_defined(ST_PREDEFINED + 1));
        assert!(!sort_is_user_defined(ST_REAL));
        assert!(sort_is_interpreted(ST_INTEGER));
        assert!(sort_is_interpreted(ST_REAL));
        assert!(!sort_is_interpreted(ST_INDIVIDUALS));
    }

    #[test]
    fn type_allocation_and_queries_preserve_c_shapes() {
        let kind = alloc_simple_sort(ST_KIND);
        let bool = bool_sort();
        let individual = individual_sort();
        let constructor = alloc_arrow_type(vec![kind.clone(), individual.clone()]);
        let predicate = alloc_arrow_type(vec![individual.clone(), bool.clone()]);

        assert!(kind.is_kind());
        assert!(bool.is_bool());
        assert!(individual.is_individual());
        assert!(predicate.is_arrow());
        assert!(type_is_predicate(&bool));
        assert!(type_is_predicate(&predicate));
        assert!(type_is_type_constructor(&kind));
        assert!(type_is_type_constructor(&constructor));
        assert_eq!(get_ret_type(&predicate), bool);
        assert_eq!(type_get_max_arity(&predicate), 1);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn type_handles_and_optional_slots_are_one_pointer_wide() {
        assert_eq!(std::mem::size_of::<super::Type>(), 8);
        assert_eq!(std::mem::size_of::<Option<super::Type>>(), 8);
    }

    #[test]
    fn single_argument_arrow_returns_the_argument_identity() {
        let individual = individual_sort();
        let arrow = alloc_arrow_type(vec![individual.clone()]);
        assert_eq!(arrow, individual);

        let copied = alloc_arrow_type_copy_args(std::slice::from_ref(&individual));
        assert_eq!(copied, individual);
    }

    #[test]
    fn type_copy_is_shallow_and_resets_uid() {
        let bool = bool_sort();
        let individual = individual_sort();
        let arrow = alloc_arrow_type(vec![individual.clone(), bool.clone()]);
        arrow.set_type_uid(99);

        let copied = type_copy(&arrow);

        assert_ne!(copied, arrow);
        assert_eq!(copied.f_code(), arrow.f_code());
        assert_eq!(copied.arity(), arrow.arity());
        assert_eq!(copied.args()[0], individual);
        assert_eq!(copied.args()[1], bool);
        assert_eq!(copied.type_uid(), INVALID_TYPE_UID);
    }

    #[test]
    fn flattening_combines_arrow_return_types() {
        let bool = bool_sort();
        let individual = individual_sort();
        let return_arrow = alloc_arrow_type(vec![individual.clone(), bool.clone()]);
        let nested = alloc_arrow_type(vec![individual.clone(), return_arrow.clone()]);

        assert!(!is_flattened(&nested));
        assert!(arguments_flattened(&nested));
        let flattened = flatten_type(&nested);
        assert_ne!(flattened, nested);
        assert_eq!(flattened.arity(), 3);
        assert_eq!(flattened.args()[0], individual);
        assert_eq!(flattened.args()[1], return_arrow.args()[0]);
        assert_eq!(flattened.args()[2], bool);
        assert!(is_flattened(&flattened));

        assert_eq!(flatten_type(&flattened), flattened);
        let flattened_from_builder =
            arrow_type_flattened(std::slice::from_ref(&return_arrow.args()[0]), &return_arrow);
        assert_ne!(flattened_from_builder, flattened);
        assert_eq!(flattened_from_builder.arity(), flattened.arity());
        assert_eq!(flattened_from_builder.args()[0], flattened.args()[0]);
        assert_eq!(flattened_from_builder.args()[1], flattened.args()[1]);
        assert_eq!(flattened_from_builder.args()[2], flattened.args()[2]);
    }

    #[test]
    fn type_order_and_untyped_helpers_recurse_through_arrows() {
        let bool = bool_sort();
        let individual = individual_sort();
        let int = alloc_simple_sort(ST_INTEGER);
        let predicate = alloc_arrow_type(vec![individual.clone(), bool.clone()]);
        let higher = alloc_arrow_type(vec![predicate.clone(), bool.clone()]);

        assert_eq!(type_get_order(&predicate), 1);
        assert_eq!(type_get_order(&higher), 2);
        assert_eq!(var_order(&higher), 3);
        assert!(type_has_bool(&higher));
        assert!(type_is_untyped(&predicate));
        assert!(!type_is_untyped(&alloc_arrow_type(vec![int, bool])));
    }

    #[test]
    fn type_drop_first_arg_matches_c_return_shapes() {
        let bool = bool_sort();
        let individual = individual_sort();
        let binary = alloc_arrow_type(vec![individual.clone(), bool.clone()]);
        assert_eq!(super::type_drop_first_arg(&binary), bool);

        let ternary = alloc_arrow_type(vec![individual.clone(), individual.clone(), bool.clone()]);
        let dropped = super::type_drop_first_arg(&ternary);
        assert!(dropped.is_arrow());
        assert_eq!(dropped.arity(), 2);
        assert_eq!(dropped.args()[0], individual);
        assert_eq!(dropped.args()[1], bool);
    }

    #[test]
    fn encoded_names_use_builtins_or_initialized_uid() {
        let bool = bool_sort();
        assert_eq!(type_app_encoded_name(&bool).unwrap(), "$o");

        let user = alloc_simple_sort(ST_PREDEFINED + 1);
        let error = type_app_encoded_name(&user).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        user.set_type_uid(42);
        assert_eq!(type_app_encoded_name(&user).unwrap(), "type_42");

        let arrow = alloc_arrow_type(vec![individual_sort(), bool]);
        assert!(type_app_encoded_name(&arrow).is_err());
        arrow.set_type_uid(7);
        assert_eq!(type_app_encoded_name(&arrow).unwrap(), "type_7");
    }

    #[test]
    fn types_cmp_uses_f_code_arity_then_argument_pointer_identity() {
        let bool = bool_sort();
        let individual = individual_sort();
        assert!(types_cmp(&bool, &individual) < 0);
        assert_eq!(types_cmp(&bool, &bool), 0);

        let left_arg = individual_sort();
        let right_arg = individual_sort();
        let arg_order = super::type_identity_cmp(&left_arg, &right_arg);
        let left = alloc_arrow_type(vec![left_arg, bool.clone()]);
        let right = alloc_arrow_type(vec![right_arg, bool]);
        assert_eq!(types_cmp(&left, &right), arg_order);
    }

    #[test]
    fn type_identity_cmp_uses_pointer_identity() {
        let left = individual_sort();
        let right = individual_sort();
        let left_address = std::rc::Rc::as_ptr(&left.0) as usize;
        let right_address = std::rc::Rc::as_ptr(&right.0) as usize;
        let address_order =
            i32::from(left_address > right_address) - i32::from(left_address < right_address);

        assert_eq!(super::type_identity_cmp(&left, &left), 0);
        assert_eq!(super::type_identity_cmp(&left, &right), address_order);
    }

    #[test]
    fn choice_type_detection_uses_pointer_identity() {
        let bool = bool_sort();
        let a = individual_sort();
        let pred = alloc_arrow_type(vec![a.clone(), bool]);
        let choice = alloc_arrow_type(vec![pred.clone(), a.clone()]);
        assert!(is_choice_type(&choice));

        let structurally_equal_but_distinct = individual_sort();
        let not_choice = alloc_arrow_type(vec![pred, structurally_equal_but_distinct]);
        assert!(!is_choice_type(&not_choice));
    }
}
