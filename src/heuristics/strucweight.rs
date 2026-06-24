use crate::terms::termfunc::term_weight_compute;
use crate::terms::termtypes::Term;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrucDistanceParams {
    var_mismatch: f64,
    sym_mismatch: f64,
    inst_factor: f64,
    gen_factor: f64,
}

impl StrucDistanceParams {
    #[must_use]
    pub const fn new(
        var_mismatch: f64,
        sym_mismatch: f64,
        inst_factor: f64,
        gen_factor: f64,
    ) -> Self {
        Self {
            var_mismatch,
            sym_mismatch,
            inst_factor,
            gen_factor,
        }
    }

    #[must_use]
    pub const fn var_mismatch(self) -> f64 {
        self.var_mismatch
    }

    #[must_use]
    pub const fn sym_mismatch(self) -> f64 {
        self.sym_mismatch
    }

    #[must_use]
    pub const fn inst_factor(self) -> f64 {
        self.inst_factor
    }

    #[must_use]
    pub const fn gen_factor(self) -> f64 {
        self.gen_factor
    }
}

#[must_use]
pub const fn struc_distance_init(
    var_mismatch: f64,
    sym_mismatch: f64,
    inst_factor: f64,
    gen_factor: f64,
) -> StrucDistanceParams {
    StrucDistanceParams::new(var_mismatch, sym_mismatch, inst_factor, gen_factor)
}

/// Computes C `strc_terms_distance` for already-normalized terms.
///
/// # Panics
///
/// Panics if the C fall-through recursive case needs an argument that is not
/// initialized on either term. This matches the C helper's unchecked argument
/// access when top symbols have the same arity or the same f-code.
#[must_use]
pub fn struc_terms_distance(left: &Term, right: &Term, param: &StrucDistanceParams) -> f64 {
    if left.is_free_var() {
        if right.is_free_var() {
            return if left.f_code() == right.f_code() {
                0.0
            } else {
                (param.inst_factor + param.gen_factor).min(param.var_mismatch)
            };
        }
        return param.inst_factor * term_c_weight(right);
    }

    if right.is_free_var() {
        return param.gen_factor * term_c_weight(left);
    }

    if left.f_code() != right.f_code() && left.arity() != right.arity() {
        return param.gen_factor * term_c_weight(left) + param.inst_factor * term_c_weight(right);
    }

    let mut arg_distance = 0.0;
    for index in 0..left.arity() {
        let left_arg = left
            .argument(index)
            .unwrap_or_else(|| panic!("left term argument {index} is uninitialized"));
        let right_arg = right
            .argument(index)
            .unwrap_or_else(|| panic!("right term argument {index} is uninitialized"));
        arg_distance += struc_terms_distance(&left_arg, &right_arg, param);
    }

    let geninst = param.gen_factor * term_c_weight(left) + param.inst_factor * term_c_weight(right);
    let factor = if left.f_code() == right.f_code() {
        1.0
    } else {
        param.sym_mismatch
    };
    (factor * arg_distance).min(geninst)
}

/// Scores `term` against normalized conjecture terms using structural distance.
///
/// # Panics
///
/// Panics under the same conditions as [`struc_terms_distance`].
#[must_use]
pub fn struc_term_weight(
    term: &Term,
    conjecture_terms: &[Term],
    param: &StrucDistanceParams,
) -> f64 {
    let mut minimum = f64::MAX;
    for conjecture in conjecture_terms {
        minimum = minimum.min(struc_terms_distance(term, conjecture, param));
    }
    minimum
}

#[allow(clippy::cast_precision_loss)]
fn term_c_weight(term: &Term) -> f64 {
    term_weight_compute(term, 1, 1) as f64
}

#[cfg(test)]
mod tests {
    use super::{struc_distance_init, struc_term_weight, struc_terms_distance};
    use crate::terms::functypes::FunCode;
    use crate::terms::termtypes::Term;

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn params() -> super::StrucDistanceParams {
        struc_distance_init(5.0, 10.0, 2.0, 3.0)
    }

    fn unary(code: FunCode, arg: &Term) -> Term {
        let term = Term::top_alloc(code, 1);
        term.set_argument(0, arg.clone());
        term
    }

    fn binary(code: FunCode, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    #[test]
    fn init_preserves_parameters() {
        let param = params();

        assert_f64_bits_eq(param.var_mismatch(), 5.0);
        assert_f64_bits_eq(param.sym_mismatch(), 10.0);
        assert_f64_bits_eq(param.inst_factor(), 2.0);
        assert_f64_bits_eq(param.gen_factor(), 3.0);
    }

    #[test]
    fn variable_cases_match_c_formula() {
        let x = Term::const_cell_alloc(-2);
        let x_again = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let fa = unary(10, &Term::const_cell_alloc(1));

        assert_f64_bits_eq(struc_terms_distance(&x, &x_again, &params()), 0.0);
        assert_f64_bits_eq(struc_terms_distance(&x, &y, &params()), 5.0);
        assert_f64_bits_eq(struc_terms_distance(&x, &fa, &params()), 4.0);
        assert_f64_bits_eq(struc_terms_distance(&fa, &x, &params()), 6.0);
    }

    #[test]
    fn symbol_and_arity_fallback_matches_c_condition() {
        let fa = unary(10, &Term::const_cell_alloc(1));
        let gab = binary(11, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_terms_distance(&fa, &gab, &params()), 12.0);
    }

    #[test]
    fn different_same_arity_symbols_can_have_zero_distance() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let fa = unary(10, &a);
        let gb = unary(11, &b);

        assert_f64_bits_eq(struc_terms_distance(&a, &b, &params()), 0.0);
        assert_f64_bits_eq(struc_terms_distance(&fa, &gb, &params()), 0.0);
    }

    #[test]
    fn same_symbol_extra_right_arguments_are_ignored_by_left_arity_loop() {
        let left = unary(10, &Term::const_cell_alloc(1));
        let right = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_terms_distance(&left, &right, &params()), 0.0);
    }

    #[test]
    #[should_panic(expected = "right term argument 1 is uninitialized")]
    fn same_symbol_missing_right_argument_panics_like_unchecked_c_access() {
        let left = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let right = unary(10, &Term::const_cell_alloc(1));

        let _ = struc_terms_distance(&left, &right, &params());
    }

    #[test]
    fn term_weight_returns_minimum_or_dbl_max() {
        let term = unary(10, &Term::const_cell_alloc(1));
        let exact = unary(10, &Term::const_cell_alloc(1));
        let fallback = binary(11, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_term_weight(&term, &[fallback], &params()), 12.0);
        assert_f64_bits_eq(
            struc_term_weight(&term, &[Term::const_cell_alloc(-2), exact], &params()),
            0.0,
        );
        assert_f64_bits_eq(struc_term_weight(&term, &[], &params()), f64::MAX);
    }
}
