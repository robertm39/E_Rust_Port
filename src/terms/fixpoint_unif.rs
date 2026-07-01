//! Port of `TERMS/cte_fixpoint_unif`.

use crate::basics::error::Diagnostic;
use crate::terms::lambda::{lambda_eta_reduce_db, whnf_deref};
use crate::terms::match_mgu::OracleUnifResult;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

/// C `SubstComputeFixpointMgu`.
///
/// Computes the fixpoint-oracle unifier for the currently represented higher-
/// order fragment. On success, it adds the same single binding the C helper
/// would add.
///
/// # Errors
///
/// Returns a diagnostic if weak-head dereferencing or eta-reduction needs to
/// rebuild a term and term-bank insertion fails.
///
/// # Panics
///
/// Panics if a term has malformed lambda/application children or missing
/// typing metadata, matching the C internal preconditions.
pub fn subst_compute_fixpoint_mgu(
    bank: &mut TermBank,
    t1: &Term,
    t2: &Term,
    subst: &mut Substitution,
) -> Result<OracleUnifResult, Diagnostic> {
    let mut left = whnf_eta_reduce(bank, t1)?;
    let mut right = whnf_eta_reduce(bank, t2)?;

    if left.is_free_var() && right.is_free_var() {
        if left != right {
            subst.add_binding(&left, &right);
        }
        return Ok(OracleUnifResult::Unifiable);
    }

    if !left.is_free_var() && !right.is_free_var() {
        return Ok(OracleUnifResult::NotInFragment);
    }

    if !left.is_free_var() {
        std::mem::swap(&mut left, &mut right);
    }

    assert!(left.is_free_var(), "fixpoint MGU expects one free variable");
    assert!(
        !right.is_free_var(),
        "fixpoint MGU variable-variable case is handled earlier"
    );

    let has_prefix = right.is_lambda();
    let result = rigid_path_check(bank, &left, &right, has_prefix, false, 0)?;
    if result == OracleUnifResult::Unifiable {
        subst.add_binding(&left, &right);
    }
    Ok(result)
}

fn whnf_eta_reduce(bank: &mut TermBank, term: &Term) -> Result<Term, Diagnostic> {
    let weak_head = whnf_deref(bank, term)?;
    lambda_eta_reduce_db(bank, &weak_head)
}

fn rigid_path_check(
    bank: &mut TermBank,
    var: &Term,
    term: &Term,
    has_prefix: bool,
    under_var: bool,
    depth: i64,
) -> Result<OracleUnifResult, Diagnostic> {
    assert!(var.is_free_var(), "rigid-path check variable must be free");
    let mut term = whnf_deref(bank, term)?;

    if term.is_applied_free_var() {
        let head = required_arg(&term, 0);
        if *var == head {
            return Ok(if under_var || has_prefix {
                OracleUnifResult::NotInFragment
            } else {
                OracleUnifResult::NotUnifiable
            });
        }
        return rigid_path_check_args(bank, var, &term, 1, has_prefix, true, depth, term.arity());
    }

    if term.is_free_var() {
        if *var == term {
            let type_ = term.type_().expect("fixpoint variable must have a type");
            return Ok(if under_var || type_.is_arrow() {
                OracleUnifResult::NotInFragment
            } else {
                OracleUnifResult::NotUnifiable
            });
        }
        return Ok(OracleUnifResult::Unifiable);
    }

    if term.is_lambda() {
        while term.is_lambda() {
            term = required_arg(&term, 1);
        }
        return rigid_path_check(bank, var, &term, has_prefix, under_var, depth + 1);
    }

    if term.is_db_var() {
        return Ok(if term.f_code() >= depth {
            if under_var {
                OracleUnifResult::NotInFragment
            } else {
                OracleUnifResult::NotUnifiable
            }
        } else {
            OracleUnifResult::Unifiable
        });
    }

    rigid_path_check_args(
        bank,
        var,
        &term,
        0,
        has_prefix,
        under_var,
        depth,
        term.arity(),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible recursive helper keeps the rigid-path state explicit"
)]
fn rigid_path_check_args(
    bank: &mut TermBank,
    var: &Term,
    term: &Term,
    start: usize,
    has_prefix: bool,
    under_var: bool,
    depth: i64,
    length: usize,
) -> Result<OracleUnifResult, Diagnostic> {
    let mut result = OracleUnifResult::Unifiable;
    for index in start..length {
        result = rigid_path_check(
            bank,
            var,
            &required_arg(term, index),
            has_prefix,
            under_var,
            depth,
        )?;
        if result != OracleUnifResult::Unifiable {
            break;
        }
    }
    Ok(result)
}

fn required_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("fixpoint term argument {index} is uninitialized"))
}

#[cfg(test)]
mod tests {
    use super::subst_compute_fixpoint_mgu;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
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

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn fresh_var(bank: &TermBank, type_: &Type) -> Term {
        bank.vars().get_fresh_var(type_)
    }

    #[test]
    fn fixpoint_mgu_binds_distinct_free_variables() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let left = fresh_var(&bank, &type_);
        let right = fresh_var(&bank, &type_);
        let mut subst = Substitution::new();

        let result = subst_compute_fixpoint_mgu(&mut bank, &left, &right, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::Unifiable);
        assert_eq!(subst.bindings(), std::slice::from_ref(&left));
        assert_eq!(left.binding(), Some(right));
    }

    #[test]
    fn fixpoint_mgu_reports_non_variable_pair_outside_fragment() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "fixpoint_a");
        let right = typed_const(&mut bank, "fixpoint_b");
        let mut subst = Substitution::new();

        let result = subst_compute_fixpoint_mgu(&mut bank, &left, &right, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotInFragment);
        assert!(subst.is_empty());
    }

    #[test]
    fn fixpoint_mgu_binds_variable_to_rigid_term_without_occurrence() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let variable = fresh_var(&bank, &type_);
        let other = fresh_var(&bank, &type_);
        let rigid = typed_unary(&mut bank, "fixpoint_f", &other);
        let mut subst = Substitution::new();

        let result = subst_compute_fixpoint_mgu(&mut bank, &variable, &rigid, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::Unifiable);
        assert_eq!(variable.binding(), Some(rigid));
        assert_eq!(subst.len(), 1);
    }

    #[test]
    fn fixpoint_mgu_rejects_direct_rigid_occurrence_as_not_unifiable() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let variable = fresh_var(&bank, &type_);
        let rigid = typed_unary(&mut bank, "fixpoint_occurs_f", &variable);
        let mut subst = Substitution::new();

        let result = subst_compute_fixpoint_mgu(&mut bank, &variable, &rigid, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotUnifiable);
        assert!(variable.binding().is_none());
        assert!(subst.is_empty());
    }

    #[test]
    fn fixpoint_mgu_reports_occurrence_under_applied_variable_outside_fragment() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let variable = fresh_var(&bank, &type_);
        let head = fresh_var(&bank, &unary_type);
        let applied = apply_terms(&mut bank, &head, std::slice::from_ref(&variable)).unwrap();
        let mut subst = Substitution::new();

        let result =
            subst_compute_fixpoint_mgu(&mut bank, &variable, &applied, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotInFragment);
        assert!(variable.binding().is_none());
        assert!(subst.is_empty());
    }

    #[test]
    fn fixpoint_mgu_treats_lambda_prefix_occurrence_as_outside_fragment() {
        let mut bank = test_bank();
        let type_ = bank.signature().type_bank().default_type();
        let arrow_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
        let variable = fresh_var(&bank, &arrow_type);
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&type_), &variable).unwrap();
        let mut subst = Substitution::new();

        let result = subst_compute_fixpoint_mgu(&mut bank, &variable, &lambda, &mut subst).unwrap();

        assert_eq!(result, OracleUnifResult::NotInFragment);
        assert!(variable.binding().is_none());
        assert!(subst.is_empty());
    }
}
