use crate::basics::error::Diagnostic;
use crate::terms::ho_csu::Limits;
use crate::terms::lambda::{
    apply_terms, close_with_db_var, close_with_type_prefix, fresh_var_with_args, whnf_step,
};
use crate::terms::signature::SIG_PHONY_APP_CODE;
use crate::terms::simpletypes::{arrow_type_flattened, get_ret_type, type_get_max_arity, Type};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, Term};

pub const IMIT_MASK: Limits = 63;
pub const PROJ_MASK: Limits = IMIT_MASK << 6;
pub const IDENT_MASK: Limits = PROJ_MASK << 6;
pub const ELIM_MASK: Limits = IDENT_MASK << 6;

/// Mirrors C `GET_IMIT(c)`.
#[must_use]
pub const fn imitation_count(limits: Limits) -> Limits {
    limits & IMIT_MASK
}

/// Mirrors C `GET_PROJ(c)`.
#[must_use]
pub const fn projection_count(limits: Limits) -> Limits {
    (limits & PROJ_MASK) >> 6
}

/// Mirrors C `GET_IDENT(c)`.
#[must_use]
pub const fn identification_count(limits: Limits) -> Limits {
    (limits & IDENT_MASK) >> 12
}

/// Mirrors C `GET_ELIM(c)`.
#[must_use]
pub const fn elimination_count(limits: Limits) -> Limits {
    (limits & ELIM_MASK) >> 18
}

/// Mirrors C `INC_IMIT(c)`.
#[must_use]
pub const fn inc_imitation(limits: Limits) -> Limits {
    (imitation_count(limits) + 1) | (!IMIT_MASK & limits)
}

/// Mirrors C `INC_PROJ(c)`.
#[must_use]
pub const fn inc_projection(limits: Limits) -> Limits {
    ((projection_count(limits) + 1) << 6) | (!PROJ_MASK & limits)
}

/// Mirrors C `INC_IDENT(c)`.
#[must_use]
pub const fn inc_identification(limits: Limits) -> Limits {
    ((identification_count(limits) + 1) << 12) | (!IDENT_MASK & limits)
}

/// Mirrors C `INC_ELIM(c)`.
#[must_use]
pub const fn inc_elimination(limits: Limits) -> Limits {
    ((elimination_count(limits) + 1) << 18) | (!ELIM_MASK & limits)
}

/// Result of C `build_ident` or `build_trivial_ident`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentificationBinding {
    pub left_target: Term,
    pub right_target: Term,
}

/// Builds the C `build_imitation` binding for a rigid-symbol `rhs`.
///
/// Returns `None` for the C `NULL` cases: variable-headed RHS, phony
/// application RHS, or a rigid symbol without a monomorphic signature type.
///
/// # Errors
///
/// Returns diagnostics from fresh-variable application, term-bank insertion, or
/// lambda construction.
///
/// # Panics
///
/// Panics if `rhs` is a phony application headed by a lambda, if `flex` is not
/// a top-level free variable, or if the flex head is untyped.
pub fn build_imitation(
    bank: &mut TermBank,
    flex: &Term,
    rhs: &Term,
) -> Result<Option<Term>, Diagnostic> {
    if rhs.is_phony_app() || rhs.is_free_var() || rhs.is_db_var() {
        assert!(
            !rhs.is_phony_app() || !rhs.argument(0).is_some_and(|head| head.is_lambda()),
            "imitation phony-app rhs must not have a lambda head"
        );
        return Ok(None);
    }

    let Some(rigid_type) = bank.signature().get_type(rhs.f_code()).cloned() else {
        return Ok(None);
    };
    let var_type = top_level_free_head(flex)
        .type_()
        .expect("imitation flex head has a type");
    let db_vars = db_vars_for_type_prefix(bank, &var_type);

    let matrix = if rigid_type.is_arrow() {
        let matrix = Term::top_alloc(rhs.f_code(), rigid_type.arity() - 1);
        for index in 0..rigid_type.arity() - 1 {
            matrix.set_argument(
                index,
                fresh_var_with_args(bank, &db_vars, &rigid_type.args()[index])?,
            );
        }
        matrix.set_type(Some(get_ret_type(&rigid_type)));
        bank.term_top_insert(matrix)?
    } else {
        let matrix = Term::const_cell_alloc(rhs.f_code());
        matrix.set_type(Some(rigid_type));
        bank.term_top_insert(matrix)?
    };

    Ok(Some(close_with_type_prefix(
        bank,
        type_prefix(&var_type),
        &matrix,
    )?))
}

/// Builds the C `build_projection` binding for visible argument `idx`.
///
/// `idx` is zero-based over the applied free variable's visible arguments,
/// excluding the head at argument slot 0. Returns `None` for the C `NULL`
/// cases where a quick rigid-head check proves the projection cannot solve the
/// current equation.
///
/// # Errors
///
/// Returns diagnostics from weak-head normalization, fresh-variable
/// application, term-bank insertion, or lambda construction.
///
/// # Panics
///
/// Panics if `flex` is not an applied free variable, if `rhs` is a lambda, if
/// `idx` is out of range, if required terms are untyped, or if the selected
/// argument has an incompatible return type.
pub fn build_projection(
    bank: &mut TermBank,
    flex: &Term,
    rhs: &Term,
    idx: usize,
) -> Result<Option<Term>, Diagnostic> {
    assert!(
        flex.is_applied_free_var(),
        "projection expects an applied free variable"
    );
    assert!(!rhs.is_lambda(), "projection rhs must not be a lambda");
    assert!(
        idx < flex.arity() - 1,
        "projection argument index is out of range"
    );

    let var_type = top_level_free_head(flex)
        .type_()
        .expect("projection flex head has a type");
    let raw_arg = flex
        .argument(idx + 1)
        .unwrap_or_else(|| panic!("flex argument {} is uninitialized", idx + 1));
    let arg = if rhs.is_top_level_free_var() {
        raw_arg
    } else {
        whnf_step(bank, &raw_arg)?
    };
    let arg_type = arg.type_().expect("projection argument has a type");
    assert_eq!(
        get_ret_type(&var_type),
        get_ret_type(&arg_type),
        "projection requires matching return types"
    );

    if projection_fails_fast(&arg, rhs) {
        return Ok(None);
    }

    let matrix = if arg_type.is_arrow() {
        let db_vars = db_vars_for_type_prefix(bank, &var_type);
        assert!(
            idx < db_vars.len(),
            "projection argument index exceeds flex type prefix"
        );
        let matrix = Term::top_alloc(SIG_PHONY_APP_CODE, arg_type.arity());
        matrix.set_argument(0, db_vars[idx].clone());
        for index in 1..arg_type.arity() {
            matrix.set_argument(
                index,
                fresh_var_with_args(bank, &db_vars, &arg_type.args()[index - 1])?,
            );
        }
        bank.term_top_insert(matrix)?
    } else {
        let db_index = (type_get_max_arity(&var_type) - idx - 1)
            .try_into()
            .expect("type arity fits in a FunCode");
        bank.request_db_var(&arg_type, db_index)
    };

    Ok(Some(close_with_type_prefix(
        bank,
        type_prefix(&var_type),
        &matrix,
    )?))
}

/// Builds the C `build_elim` binding that drops visible argument `idx`.
///
/// `idx` is zero-based over the applied free variable's visible arguments,
/// excluding the head at argument slot 0.
///
/// # Errors
///
/// Returns diagnostics from fresh-variable application or lambda construction.
///
/// # Panics
///
/// Panics if `flex` is not an applied free variable, if `idx` is out of range,
/// or if any visible argument or the applied term is untyped.
pub fn build_elim(bank: &mut TermBank, flex: &Term, idx: usize) -> Result<Term, Diagnostic> {
    assert!(
        flex.is_applied_free_var(),
        "elimination expects an applied free variable"
    );
    let visible_arity = flex.arity() - 1;
    assert!(
        idx < visible_arity,
        "elimination argument index is out of range"
    );

    let mut db_vars = Vec::with_capacity(visible_arity.saturating_sub(1));
    for index in 1..flex.arity() {
        if index - 1 != idx {
            let arg = flex
                .argument(index)
                .unwrap_or_else(|| panic!("flex argument {index} is uninitialized"));
            let arg_type = arg
                .type_()
                .expect("elimination expects typed flex arguments");
            let db_index = (flex.arity() - index - 1)
                .try_into()
                .expect("flex arity fits in a FunCode");
            db_vars.push(bank.request_db_var(&arg_type, db_index));
        }
    }

    let flex_type = flex.type_().expect("elimination expects a typed flex term");
    let mut result = fresh_var_with_args(bank, &db_vars, &flex_type)?;
    for index in (1..flex.arity()).rev() {
        let arg = flex
            .argument(index)
            .unwrap_or_else(|| panic!("flex argument {index} is uninitialized"));
        let arg_type = arg
            .type_()
            .expect("elimination expects typed flex arguments");
        result = close_with_db_var(bank, &arg_type, &result)?;
    }
    Ok(result)
}

/// Builds the C `build_ident` pair of bindings for two top-level free variables.
///
/// Returns `None` when `right` is not a top-level free variable, matching C's
/// success flag. The caller must pass a top-level free variable as `left`.
///
/// # Errors
///
/// Returns diagnostics from fresh-variable application, term-bank insertion, or
/// lambda construction.
///
/// # Panics
///
/// Panics if `left` is not a top-level free variable, if either variable head is
/// untyped, or if the return types differ.
pub fn build_ident(
    bank: &mut TermBank,
    left: &Term,
    right: &Term,
) -> Result<Option<IdentificationBinding>, Diagnostic> {
    assert!(
        left.is_top_level_free_var(),
        "identification expects a left top-level free variable"
    );
    if !right.is_top_level_free_var() {
        return Ok(None);
    }

    let left_type = top_level_free_head(left)
        .type_()
        .expect("left top-level free variable head has a type");
    let right_type = top_level_free_head(right)
        .type_()
        .expect("right top-level free variable head has a type");
    let return_type = get_ret_type(&left_type);
    assert_eq!(
        return_type,
        get_ret_type(&right_type),
        "identification requires matching return types"
    );

    let left_prefix = type_prefix(&left_type).to_vec();
    let right_prefix = type_prefix(&right_type).to_vec();
    let mut matrix_arg_types = Vec::with_capacity(left_prefix.len() + right_prefix.len());
    matrix_arg_types.extend(left_prefix.iter().cloned());
    matrix_arg_types.extend(right_prefix.iter().cloned());
    let matrix_type = bank
        .signature_mut()
        .type_bank_mut()
        .insert_type_shared(arrow_type_flattened(&matrix_arg_types, &return_type));
    let matrix = bank.vars().get_fresh_var(&matrix_type);
    let matrix = bank.insert(&matrix, DerefType::Never)?;

    let left_db_vars = db_vars_for_prefix(bank, &left_prefix);
    let right_db_vars = db_vars_for_prefix(bank, &right_prefix);

    let mut to_apply_left = Vec::with_capacity(left_prefix.len() + right_prefix.len());
    to_apply_left.extend(left_db_vars.iter().cloned());
    for type_ in &right_prefix {
        to_apply_left.push(fresh_var_with_args(bank, &left_db_vars, type_)?);
    }

    let mut to_apply_right = Vec::with_capacity(left_prefix.len() + right_prefix.len());
    for type_ in &left_prefix {
        to_apply_right.push(fresh_var_with_args(bank, &right_db_vars, type_)?);
    }
    to_apply_right.extend(right_db_vars.iter().cloned());

    let left_matrix = apply_terms(bank, &matrix, &to_apply_left)?;
    let right_matrix = apply_terms(bank, &matrix, &to_apply_right)?;

    Ok(Some(IdentificationBinding {
        left_target: close_with_type_prefix(bank, &left_prefix, &left_matrix)?,
        right_target: close_with_type_prefix(bank, &right_prefix, &right_matrix)?,
    }))
}

/// Builds the C `build_trivial_ident` fallback binding.
///
/// Returns `None` when `right` is not a top-level free variable, matching C's
/// success flag. The caller must pass a top-level free variable as `left`.
///
/// # Errors
///
/// Returns diagnostics from term-bank insertion or lambda construction.
///
/// # Panics
///
/// Panics if `left` is not a top-level free variable, if either variable head is
/// missing a type, or if the two return types differ.
pub fn build_trivial_ident(
    bank: &mut TermBank,
    left: &Term,
    right: &Term,
) -> Result<Option<IdentificationBinding>, Diagnostic> {
    assert!(
        left.is_top_level_free_var(),
        "trivial identification expects a left top-level free variable"
    );
    if !right.is_top_level_free_var() {
        return Ok(None);
    }

    let left_type = top_level_free_head(left)
        .type_()
        .expect("left top-level free variable head has a type");
    let right_type = top_level_free_head(right)
        .type_()
        .expect("right top-level free variable head has a type");
    let return_type = get_ret_type(&left_type);
    assert_eq!(
        return_type,
        get_ret_type(&right_type),
        "trivial identification requires matching return types"
    );

    let matrix = bank.vars().get_fresh_var(&return_type);
    let matrix = bank.insert(&matrix, DerefType::Never)?;
    let left_target = close_with_type_prefix(bank, type_prefix(&left_type), &matrix)?;
    let right_target = close_with_type_prefix(bank, type_prefix(&right_type), &matrix)?;

    Ok(Some(IdentificationBinding {
        left_target,
        right_target,
    }))
}

fn top_level_free_head(term: &Term) -> Term {
    if term.is_applied_free_var() {
        term.argument(0)
            .expect("applied free variable has an initialized head")
    } else {
        assert!(term.is_free_var(), "expected a top-level free variable");
        term.clone()
    }
}

fn top_level_db_head(term: &Term) -> Term {
    if term.is_applied_db_var() {
        term.argument(0)
            .expect("applied DB variable has an initialized head")
    } else {
        assert!(term.is_db_var(), "expected a top-level DB variable");
        term.clone()
    }
}

fn projection_fails_fast(arg: &Term, rhs: &Term) -> bool {
    if arg.is_top_level_free_var() || rhs.is_top_level_free_var() {
        return false;
    }
    if arg.is_top_level_db_var() && rhs.is_top_level_db_var() {
        return top_level_db_head(arg) != top_level_db_head(rhs);
    }
    if !arg.is_top_level_db_var()
        && !rhs.is_top_level_db_var()
        && !arg.is_lambda()
        && !rhs.is_lambda()
    {
        return arg.f_code() != rhs.f_code();
    }
    !arg.is_lambda() && !rhs.is_lambda() && (arg.is_top_level_db_var() != rhs.is_top_level_db_var())
}

fn type_prefix(type_: &Type) -> &[Type] {
    if type_.is_arrow() {
        &type_.args()[..type_.arity() - 1]
    } else {
        &[]
    }
}

fn db_vars_for_type_prefix(bank: &mut TermBank, type_: &Type) -> Vec<Term> {
    db_vars_for_prefix(bank, type_prefix(type_))
}

fn db_vars_for_prefix(bank: &mut TermBank, prefix: &[Type]) -> Vec<Term> {
    prefix
        .iter()
        .enumerate()
        .map(|(index, type_)| {
            let db_index = (prefix.len() - index - 1)
                .try_into()
                .expect("type prefix length fits in a FunCode");
            bank.request_db_var(type_, db_index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        build_elim, build_ident, build_imitation, build_projection, build_trivial_ident,
        elimination_count, identification_count, imitation_count, inc_elimination,
        inc_identification, inc_imitation, inc_projection, projection_count, ELIM_MASK, IDENT_MASK,
        IMIT_MASK, PROJ_MASK,
    };
    use crate::terms::lambda::{apply_terms, beta_normalize_db};
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn typed_const(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(code, type_.clone())
            .unwrap();
        bank.create_const_term(code).unwrap()
    }

    #[test]
    fn limit_masks_match_c_layout() {
        assert_eq!(IMIT_MASK, 63);
        assert_eq!(PROJ_MASK, 63 << 6);
        assert_eq!(IDENT_MASK, 63 << 12);
        assert_eq!(ELIM_MASK, 63 << 18);
    }

    #[test]
    fn limit_accessors_read_c_bit_fields() {
        let limits = 5 | (6 << 6) | (7 << 12) | (8 << 18);
        assert_eq!(imitation_count(limits), 5);
        assert_eq!(projection_count(limits), 6);
        assert_eq!(identification_count(limits), 7);
        assert_eq!(elimination_count(limits), 8);
    }

    #[test]
    fn limit_incrementers_preserve_other_fields() {
        let limits = 5 | (6 << 6) | (7 << 12) | (8 << 18);
        assert_eq!(imitation_count(inc_imitation(limits)), 6);
        assert_eq!(projection_count(inc_imitation(limits)), 6);

        assert_eq!(projection_count(inc_projection(limits)), 7);
        assert_eq!(identification_count(inc_projection(limits)), 7);

        assert_eq!(identification_count(inc_identification(limits)), 8);
        assert_eq!(elimination_count(inc_identification(limits)), 8);

        assert_eq!(elimination_count(inc_elimination(limits)), 9);
        assert_eq!(imitation_count(inc_elimination(limits)), 5);
    }

    #[test]
    fn limit_incrementers_do_not_mask_overflow_like_c_macros() {
        let carried = inc_imitation(IMIT_MASK);
        assert_eq!(imitation_count(carried), 0);
        assert_eq!(projection_count(carried), 1);
    }

    #[test]
    fn trivial_ident_builds_shared_closed_targets_for_free_variables() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let unary_pred = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let binary_pred =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                    bool_type,
                ]));
        let left = bank.vars().var_assert_alloc(-100, &unary_pred);
        let right = bank.vars().var_assert_alloc(-102, &binary_pred);

        let binding = build_trivial_ident(&mut bank, &left, &right)
            .unwrap()
            .expect("top-level free variables should identify");

        assert!(binding.left_target.is_db_lambda());
        assert!(binding.right_target.is_db_lambda());
        assert_eq!(binding.left_target.type_(), Some(unary_pred));
        assert_eq!(binding.right_target.type_(), Some(binary_pred));
        assert_ne!(binding.left_target, binding.right_target);
    }

    #[test]
    fn trivial_ident_returns_none_when_right_side_is_not_top_level_free() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let left = bank.vars().var_assert_alloc(-100, &individual);
        let constant = Term::const_cell_alloc(10);
        constant.set_type(Some(individual));
        let constant = bank.term_top_insert(constant).unwrap();

        assert!(build_trivial_ident(&mut bank, &left, &constant)
            .unwrap()
            .is_none());
    }

    #[test]
    fn trivial_ident_accepts_applied_free_variable_heads() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let left_head = bank.vars().var_assert_alloc(-100, &unary);
        let right_head = bank.vars().var_assert_alloc(-102, &unary);
        let arg_code = bank.signature_mut().insert_id("triv_ident_arg", 0, false);
        bank.signature_mut()
            .declare_final_type(arg_code, individual.clone())
            .unwrap();
        let arg = bank
            .term_top_insert(Term::const_cell_alloc(arg_code))
            .unwrap();
        let left = apply_terms(&mut bank, &left_head, std::slice::from_ref(&arg)).unwrap();
        let right = apply_terms(&mut bank, &right_head, std::slice::from_ref(&arg)).unwrap();

        let binding = build_trivial_ident(&mut bank, &left, &right)
            .unwrap()
            .expect("applied free variables should identify");

        assert_eq!(binding.left_target.type_(), Some(unary.clone()));
        assert_eq!(binding.right_target.type_(), Some(unary));
        assert_eq!(left.f_code(), SIG_PHONY_APP_CODE);
        assert_eq!(right.f_code(), SIG_PHONY_APP_CODE);
    }

    #[test]
    fn ident_returns_none_when_right_side_is_not_top_level_free() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let left = bank.vars().var_assert_alloc(-100, &individual);
        let constant = typed_const(&mut bank, "ident_const_rhs", &individual);

        assert!(build_ident(&mut bank, &left, &constant).unwrap().is_none());
    }

    #[test]
    fn ident_builds_asymmetric_matrix_applications_in_c_order() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let left_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let right_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                bool_type.clone(),
            ]));
        let left_head = bank.vars().var_assert_alloc(-100, &left_type);
        let right_head = bank.vars().var_assert_alloc(-102, &right_type);
        let a = typed_const(&mut bank, "ident_a", &individual);
        let b = typed_const(&mut bank, "ident_b", &individual);
        let c = typed_const(&mut bank, "ident_c", &individual);
        let left = apply_terms(&mut bank, &left_head, std::slice::from_ref(&a)).unwrap();
        let right = apply_terms(&mut bank, &right_head, &[b.clone(), c.clone()]).unwrap();

        let binding = build_ident(&mut bank, &left, &right)
            .unwrap()
            .expect("top-level free variables should identify");
        let left_applied =
            apply_terms(&mut bank, &binding.left_target, std::slice::from_ref(&a)).unwrap();
        let right_applied =
            apply_terms(&mut bank, &binding.right_target, &[b.clone(), c.clone()]).unwrap();
        let left_normalized = beta_normalize_db(&mut bank, &left_applied).unwrap();
        let right_normalized = beta_normalize_db(&mut bank, &right_applied).unwrap();

        assert_eq!(binding.left_target.type_(), Some(left_type));
        assert_eq!(binding.right_target.type_(), Some(right_type));
        assert!(left_normalized.is_applied_free_var());
        assert!(right_normalized.is_applied_free_var());
        assert_eq!(left_normalized.argument(0), right_normalized.argument(0));
        assert_eq!(left_normalized.type_(), Some(bool_type.clone()));
        assert_eq!(right_normalized.type_(), Some(bool_type));
        assert_eq!(left_normalized.arity(), 4);
        assert_eq!(right_normalized.arity(), 4);
        assert_eq!(left_normalized.argument(1), Some(a.clone()));

        let left_synth_1 = left_normalized
            .argument(2)
            .expect("left matrix has first synthesized right argument");
        let left_synth_2 = left_normalized
            .argument(3)
            .expect("left matrix has second synthesized right argument");
        assert!(left_synth_1.is_applied_free_var());
        assert!(left_synth_2.is_applied_free_var());
        assert_eq!(left_synth_1.argument(1), Some(a.clone()));
        assert_eq!(left_synth_2.argument(1), Some(a));

        let right_synth = right_normalized
            .argument(1)
            .expect("right matrix has synthesized left argument");
        assert!(right_synth.is_applied_free_var());
        assert_eq!(right_synth.argument(1), Some(b.clone()));
        assert_eq!(right_synth.argument(2), Some(c.clone()));
        assert_eq!(right_normalized.argument(2), Some(b));
        assert_eq!(right_normalized.argument(3), Some(c));
    }

    #[test]
    fn imitation_binding_returns_none_for_variable_rhs() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let flex_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &flex_type);
        let arg = typed_const(&mut bank, "imit_var_arg", &individual);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&arg)).unwrap();
        let rhs = bank.vars().var_assert_alloc(-102, &individual);

        assert!(build_imitation(&mut bank, &flex, &rhs).unwrap().is_none());
    }

    #[test]
    fn imitation_binding_closes_constant_rhs_under_flex_prefix() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let flex_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &flex_type);
        let a = typed_const(&mut bank, "imit_const_a", &individual);
        let c = typed_const(&mut bank, "imit_const_c", &individual);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&a)).unwrap();

        let binding = build_imitation(&mut bank, &flex, &c)
            .unwrap()
            .expect("rigid constant should imitate");
        let applied = apply_terms(&mut bank, &binding, std::slice::from_ref(&a)).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(flex_type));
        assert_eq!(normalized, c);
    }

    #[test]
    fn imitation_binding_synthesizes_args_for_rigid_function() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &predicate);
        let a = typed_const(&mut bank, "imit_func_a", &individual);
        let rigid = typed_const(&mut bank, "imit_func_f", &predicate);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&a)).unwrap();

        let binding = build_imitation(&mut bank, &flex, &rigid)
            .unwrap()
            .expect("rigid function should imitate");
        let applied = apply_terms(&mut bank, &binding, std::slice::from_ref(&a)).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(predicate));
        assert_eq!(normalized.f_code(), rigid.f_code());
        assert_eq!(normalized.type_(), Some(bool_type));
        assert_eq!(normalized.arity(), 1);
        let synthesized_arg = normalized
            .argument(0)
            .expect("imitated rigid application has a synthesized argument");
        assert!(synthesized_arg.is_applied_free_var());
        assert_eq!(synthesized_arg.type_(), Some(individual));
        assert_eq!(synthesized_arg.argument(1), Some(a));
    }

    #[test]
    fn projection_binding_returns_selected_non_function_argument() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &unary);
        let a = typed_const(&mut bank, "proj_a", &individual);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&a)).unwrap();

        let binding = build_projection(&mut bank, &flex, &a, 0)
            .unwrap()
            .expect("matching rigid head should project");
        let applied = apply_terms(&mut bank, &binding, std::slice::from_ref(&a)).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(unary));
        assert_eq!(normalized, a);
    }

    #[test]
    fn projection_binding_rejects_mismatched_rigid_heads() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &unary);
        let a = typed_const(&mut bank, "proj_reject_a", &individual);
        let b = typed_const(&mut bank, "proj_reject_b", &individual);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&a)).unwrap();

        assert!(build_projection(&mut bank, &flex, &b, 0).unwrap().is_none());
    }

    #[test]
    fn projection_binding_builds_fresh_args_for_function_argument() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                bool_type.clone(),
            ]));
        let flex_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![predicate.clone(), bool_type.clone()]));
        let flex_head = bank.vars().var_assert_alloc(-100, &flex_type);
        let p = bank.vars().var_assert_alloc(-102, &predicate);
        let flex = apply_terms(&mut bank, &flex_head, std::slice::from_ref(&p)).unwrap();

        let binding = build_projection(&mut bank, &flex, &p, 0)
            .unwrap()
            .expect("matching functional argument should project");
        let applied = apply_terms(&mut bank, &binding, std::slice::from_ref(&p)).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(flex_type));
        assert!(normalized.is_applied_free_var());
        assert_eq!(normalized.argument(0), Some(p.clone()));
        assert_eq!(normalized.type_(), Some(bool_type));
        assert_eq!(normalized.arity(), 2);
        let synthesized_arg = normalized
            .argument(1)
            .expect("projected predicate application has an argument");
        assert!(synthesized_arg.is_applied_free_var());
        assert_eq!(synthesized_arg.type_(), Some(individual));
        assert_eq!(synthesized_arg.argument(1), Some(p));
    }

    #[test]
    fn elim_binding_drops_first_visible_argument() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let binary_pred =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                    bool_type.clone(),
                ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &binary_pred);
        let a = typed_const(&mut bank, "elim_a", &individual);
        let b = typed_const(&mut bank, "elim_b", &individual);
        let flex = apply_terms(&mut bank, &flex_head, &[a.clone(), b.clone()]).unwrap();

        let binding = build_elim(&mut bank, &flex, 0).unwrap();
        let applied = apply_terms(&mut bank, &binding, &[a, b.clone()]).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(binary_pred));
        assert!(normalized.is_applied_free_var());
        assert_eq!(normalized.type_(), Some(bool_type));
        assert_eq!(normalized.arity(), 2);
        assert_eq!(normalized.argument(1), Some(b));
    }

    #[test]
    fn elim_binding_drops_second_visible_argument() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let individual = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let binary_pred =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                    bool_type.clone(),
                ]));
        let flex_head = bank.vars().var_assert_alloc(-100, &binary_pred);
        let a = typed_const(&mut bank, "elim_left_a", &individual);
        let b = typed_const(&mut bank, "elim_left_b", &individual);
        let flex = apply_terms(&mut bank, &flex_head, &[a.clone(), b.clone()]).unwrap();

        let binding = build_elim(&mut bank, &flex, 1).unwrap();
        let applied = apply_terms(&mut bank, &binding, &[a.clone(), b]).unwrap();
        let normalized = beta_normalize_db(&mut bank, &applied).unwrap();

        assert_eq!(binding.type_(), Some(binary_pred));
        assert!(normalized.is_applied_free_var());
        assert_eq!(normalized.type_(), Some(bool_type));
        assert_eq!(normalized.arity(), 2);
        assert_eq!(normalized.argument(1), Some(a));
    }
}
