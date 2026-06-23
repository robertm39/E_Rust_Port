use crate::terms::functypes::FunCode;
use crate::terms::signature::{Signature, FP_COMMUTATIVE, FP_IS_AC, SIG_DB_LAMBDA_CODE};
use crate::terms::termfunc::{term_standard_weight, var_print_string};
use crate::terms::termtypes::Term;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcTerm {
    f_code: FunCode,
    args: Vec<AcTerm>,
}

impl AcTerm {
    #[must_use]
    pub const fn f_code(&self) -> FunCode {
        self.f_code
    }

    #[must_use]
    pub fn args(&self) -> &[Self] {
        &self.args
    }
}

#[must_use]
pub const fn ac_term_alloc(f_code: FunCode) -> AcTerm {
    AcTerm {
        f_code,
        args: Vec::new(),
    }
}

#[must_use]
pub fn ac_term_compare(left: &AcTerm, right: &AcTerm) -> i32 {
    let mut result = if left.f_code == SIG_DB_LAMBDA_CODE || right.f_code == SIG_DB_LAMBDA_CODE {
        -1
    } else {
        cmp_fun_code(left.f_code, right.f_code)
    };

    if result == 0 && left.f_code > 0 {
        for index in 0.. {
            match (left.args.get(index), right.args.get(index)) {
                (None, None) => break,
                (None, Some(_)) => {
                    result = -1;
                    break;
                }
                (Some(_), None) => {
                    result = 1;
                    break;
                }
                (Some(left_arg), Some(right_arg)) => {
                    result = ac_term_compare(left_arg, right_arg);
                    if result != 0 {
                        break;
                    }
                }
            }
        }
    }

    result
}

#[must_use]
pub fn ac_term_normalize(sig: &Signature, term: &Term) -> AcTerm {
    let mut handle = ac_term_alloc(term.f_code());

    if !term.is_any_var() && !term.is_lambda() && term.arity() != 0 {
        if sig.query_prop(term.f_code(), FP_IS_AC) {
            ac_collect_args(&mut handle.args, sig, term.f_code(), term);
            handle.args.sort_by(|left, right| {
                ac_term_compare(left, right)
                    .cmp(&0)
                    .then_with(|| left.f_code.cmp(&right.f_code))
            });
        } else if sig.query_prop(term.f_code(), FP_COMMUTATIVE) && term.arity() == 2 {
            let mut args = term.argument_clones().into_iter().flatten();
            if let (Some(left), Some(right)) = (args.next(), args.next()) {
                let mut left = ac_term_normalize(sig, &left);
                let mut right = ac_term_normalize(sig, &right);
                if ac_term_compare(&left, &right) > 0 {
                    std::mem::swap(&mut left, &mut right);
                }
                handle.args.push(left);
                handle.args.push(right);
            }
        } else {
            handle.args.extend(
                term.argument_clones()
                    .into_iter()
                    .flatten()
                    .map(|arg| ac_term_normalize(sig, &arg)),
            );
        }
    }

    handle
}

#[must_use]
pub fn ac_term_print_string(term: &AcTerm, sig: &Signature) -> String {
    if term.f_code < 0 {
        return var_print_string(term.f_code);
    }

    let mut result = sig
        .find_name(term.f_code)
        .map_or_else(|| term.f_code.to_string(), ToOwned::to_owned);
    if let Some((first, rest)) = term.args.split_first() {
        result.push('(');
        result.push_str(&ac_term_print_string(first, sig));
        for arg in rest {
            result.push(',');
            result.push_str(&ac_term_print_string(arg, sig));
        }
        result.push(')');
    }
    result
}

#[must_use]
pub fn term_ac_equal(sig: &Signature, left: &Term, right: &Term) -> bool {
    if term_standard_weight(left) != term_standard_weight(right)
        || left.is_phony_app()
        || right.is_phony_app()
    {
        return false;
    }

    let left = ac_term_normalize(sig, left);
    let right = ac_term_normalize(sig, right);
    ac_term_compare(&left, &right) == 0
}

fn ac_collect_args(args: &mut Vec<AcTerm>, sig: &Signature, f_code: FunCode, term: &Term) {
    if term.f_code() == f_code {
        for arg in term.argument_clones().into_iter().flatten() {
            ac_collect_args(args, sig, f_code, &arg);
        }
    } else {
        args.push(ac_term_normalize(sig, term));
    }
}

fn cmp_fun_code(left: FunCode, right: FunCode) -> i32 {
    i32::from(left > right) - i32::from(left < right)
}

#[cfg(test)]
mod tests {
    use super::{
        ac_term_alloc, ac_term_compare, ac_term_normalize, ac_term_print_string, term_ac_equal,
    };
    use crate::terms::signature::{
        Signature, FP_COMMUTATIVE, FP_IS_AC, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE,
    };
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;

    fn signature_with_symbols() -> (Signature, i64, i64, i64) {
        let mut sig = Signature::new(TypeBank::new());
        let f = sig.insert_id("f", 2, false);
        let a = sig.insert_id("a", 0, false);
        let b = sig.insert_id("b", 0, false);
        (sig, f, a, b)
    }

    fn binary(f_code: i64, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(f_code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    #[test]
    fn allocation_and_compare_follow_c_shape() {
        let var = ac_term_alloc(-2);
        let constant = ac_term_alloc(3);

        assert_eq!(var.f_code(), -2);
        assert!(var.args().is_empty());
        assert_eq!(ac_term_compare(&var, &constant), -1);
        assert_eq!(ac_term_compare(&constant, &var), 1);
    }

    #[test]
    fn compare_preserves_db_lambda_quirk() {
        let lambda = ac_term_alloc(SIG_DB_LAMBDA_CODE);

        assert_eq!(ac_term_compare(&lambda, &lambda), -1);
    }

    #[test]
    fn commutative_normalization_sorts_binary_arguments() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_COMMUTATIVE);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let term = binary(f, &b, &a);

        let normalized = ac_term_normalize(&sig, &term);

        assert_eq!(normalized.args()[0].f_code(), a_code);
        assert_eq!(normalized.args()[1].f_code(), b_code);
        assert_eq!(ac_term_print_string(&normalized, &sig), "f(a,b)");
        assert!(term_ac_equal(&sig, &binary(f, &a, &b), &term));
    }

    #[test]
    fn ac_normalization_flattens_and_sorts_nested_terms() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let left_assoc = binary(f, &b, &binary(f, &a, &b));
        let right_assoc = binary(f, &binary(f, &b, &a), &b);

        let normalized = ac_term_normalize(&sig, &left_assoc);

        assert_eq!(normalized.args().len(), 3);
        assert_eq!(
            normalized
                .args()
                .iter()
                .map(super::AcTerm::f_code)
                .collect::<Vec<_>>(),
            vec![a_code, b_code, b_code]
        );
        assert_eq!(ac_term_print_string(&normalized, &sig), "f(a,b,b)");
        assert!(term_ac_equal(&sig, &left_assoc, &right_assoc));
    }

    #[test]
    fn ac_equality_rejects_weight_mismatches_and_phony_apps() {
        let (mut sig, f, a_code, b_code) = signature_with_symbols();
        sig.set_func_prop(f, FP_IS_AC);
        let a = Term::const_cell_alloc(a_code);
        let b = Term::const_cell_alloc(b_code);
        let flat = Term::top_alloc(f, 3);
        flat.set_argument(0, a.clone());
        flat.set_argument(1, b.clone());
        flat.set_argument(2, b.clone());
        let nested = binary(f, &a, &binary(f, &b, &b));
        let phony = Term::top_alloc(SIG_PHONY_APP_CODE, 1);
        phony.set_argument(0, a);

        assert!(!term_ac_equal(&sig, &flat, &nested));
        assert!(!term_ac_equal(&sig, &phony, &phony));
    }
}
