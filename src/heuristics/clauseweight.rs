use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

pub const DEFAULT_POS_MULT: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightParam {
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
}

impl WeightParam {
    #[must_use]
    pub const fn new(fweight: i64, vweight: i64, pos_multiplier: f64, app_var_mult: f64) -> Self {
        Self {
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
        }
    }

    #[must_use]
    pub const fn pos_multiplier(self) -> f64 {
        self.pos_multiplier
    }

    #[must_use]
    pub const fn app_var_mult(self) -> f64 {
        self.app_var_mult
    }

    #[must_use]
    pub const fn vweight(self) -> i64 {
        self.vweight
    }

    #[must_use]
    pub const fn fweight(self) -> i64 {
        self.fweight
    }
}

#[must_use]
pub const fn clause_weight_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> WeightParam {
    WeightParam::new(fweight, vweight, pos_multiplier, app_var_mult)
}

#[must_use]
pub const fn lmax_weight_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> WeightParam {
    WeightParam::new(fweight, vweight, pos_multiplier, app_var_mult)
}

#[must_use]
pub const fn cmax_weight_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> WeightParam {
    WeightParam::new(fweight, vweight, pos_multiplier, app_var_mult)
}

#[must_use]
pub fn clause_weight_compute(param: &WeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    clause.literal_weight(
        bank,
        1.0,
        1.0,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        false,
    )
}

#[must_use]
pub fn lmax_weight_compute(param: &WeightParam, clause: &Clause) -> f64 {
    for literal in clause.literals().as_slice() {
        let mut tmp = literal.max_weight(param.vweight, param.fweight, param.app_var_mult);
        if literal.is_positive() {
            tmp *= param.pos_multiplier;
        }
        let _ = tmp;
    }
    0.0
}

#[must_use]
pub fn cmax_weight_compute(param: &WeightParam, clause: &Clause) -> f64 {
    let max_weight = clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| literal.max_weight(param.vweight, param.fweight, param.app_var_mult))
        .fold(0.0, f64::max);
    usize_to_f64(clause.positive_literal_count()) * max_weight * param.pos_multiplier
        + usize_to_f64(clause.negative_literal_count()) * max_weight
}

#[must_use]
/// # Panics
///
/// Panics if a non-variable term has an uninitialized argument slot, matching
/// the C helper's direct argument traversal precondition.
pub fn uniq_term_weight(term: &Term) -> f64 {
    if term.is_free_var() {
        return 3.0;
    }

    let mut weight = 5.0_f64.powi(usize_to_i32(term.arity()));
    for arg in term.argument_clones() {
        let arg = arg.expect("uniq term weight requires initialized term arguments");
        weight += 2.0 * uniq_term_weight(&arg);
    }
    weight
}

#[must_use]
pub fn uniq_eqn_weight(eqn: &Eqn) -> f64 {
    let multiplier = if eqn.is_positive() { 7.0 } else { 11.0 };
    multiplier * (uniq_term_weight(eqn.left()) + uniq_term_weight(eqn.right()))
}

#[must_use]
pub fn uniq_weight_compute(clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(uniq_eqn_weight)
        .sum()
}

#[must_use]
pub fn default_weight_compute(clause: &Clause) -> f64 {
    i64_to_f64(clause.standard_weight())
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_weight_compute, clause_weight_init, cmax_weight_compute, cmax_weight_init,
        default_weight_compute, lmax_weight_compute, lmax_weight_init, uniq_eqn_weight,
        uniq_term_weight, uniq_weight_compute, DEFAULT_POS_MULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

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
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
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
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn mixed_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let b = typed_const(bank, "b");
        let positive = Eqn::alloc(a.clone(), b.clone(), bank, true).unwrap();
        let negative = Eqn::alloc(a, b, bank, false).unwrap();
        Clause::alloc(EqnList::from_vec(vec![positive, negative]))
    }

    #[test]
    fn clause_weight_uses_literal_weight_with_c_default_multipliers() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let param = clause_weight_init(2, 1, 3.0, 1.0);

        assert_close(clause_weight_compute(&param, &bank, &clause), 24.0);
        assert_close(param.pos_multiplier(), 3.0);
        assert_close(param.app_var_mult(), 1.0);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 1);
        assert_close(DEFAULT_POS_MULT, 1.0);
    }

    #[test]
    fn lmax_weight_preserves_c_missing_accumulator_quirk() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let param = lmax_weight_init(2, 1, 3.0, 1.0);

        assert_close(lmax_weight_compute(&param, &clause), 0.0);
    }

    #[test]
    fn cmax_weight_multiplies_largest_term_weight_by_literal_counts() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let param = cmax_weight_init(2, 1, 3.0, 1.0);

        assert_close(cmax_weight_compute(&param, &clause), 8.0);
    }

    #[test]
    fn uniq_weight_uses_shape_and_literal_sign_only() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);

        assert_close(
            uniq_term_weight(clause.literals().as_slice()[0].left()),
            1.0,
        );
        assert_close(uniq_eqn_weight(&clause.literals().as_slice()[0]), 14.0);
        assert_close(uniq_eqn_weight(&clause.literals().as_slice()[1]), 22.0);
        assert_close(uniq_weight_compute(&clause), 36.0);
    }

    #[test]
    fn default_weight_returns_standard_clause_weight() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);

        assert_eq!(clause.standard_weight(), 8);
        assert_close(default_weight_compute(&clause), 8.0);
    }

    #[test]
    fn uniq_term_weight_recurses_over_arguments_with_c_multipliers() {
        let mut bank = test_bank();
        let arg = typed_const(&mut bank, "a");
        let unary = typed_unary(&mut bank, "f", &arg);

        assert_close(uniq_term_weight(&unary), 7.0);
    }
}
