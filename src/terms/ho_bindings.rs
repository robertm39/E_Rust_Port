use crate::basics::error::Diagnostic;
use crate::terms::ho_csu::Limits;
use crate::terms::lambda::{close_with_db_var, close_with_type_prefix, fresh_var_with_args};
use crate::terms::simpletypes::{get_ret_type, Type};
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

fn type_prefix(type_: &Type) -> &[Type] {
    if type_.is_arrow() {
        &type_.args()[..type_.arity() - 1]
    } else {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_elim, build_trivial_ident, elimination_count, identification_count, imitation_count,
        inc_elimination, inc_identification, inc_imitation, inc_projection, projection_count,
        ELIM_MASK, IDENT_MASK, IMIT_MASK, PROJ_MASK,
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
