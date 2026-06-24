use std::collections::BTreeMap;

use crate::clauses::clause::Clause;
use crate::terms::termbanks::TermBank;

pub const DEFAULT_MAX_MULT: f64 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiversityWeightParam {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
    fdiff1weight: f64,
    fdiff2weight: f64,
    vdiff1weight: f64,
    vdiff2weight: f64,
}

impl DiversityWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors DiversityWeightInit parameters without OCB"
    )]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        fdiff1weight: f64,
        fdiff2weight: f64,
        vdiff1weight: f64,
        vdiff2weight: f64,
        app_var_mult: f64,
    ) -> Self {
        Self {
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
            fdiff1weight,
            fdiff2weight,
            vdiff1weight,
            vdiff2weight,
        }
    }

    #[must_use]
    pub const fn max_term_multiplier(self) -> f64 {
        self.max_term_multiplier
    }

    #[must_use]
    pub const fn max_literal_multiplier(self) -> f64 {
        self.max_literal_multiplier
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

    #[must_use]
    pub const fn fdiff1weight(self) -> f64 {
        self.fdiff1weight
    }

    #[must_use]
    pub const fn fdiff2weight(self) -> f64 {
        self.fdiff2weight
    }

    #[must_use]
    pub const fn vdiff1weight(self) -> f64 {
        self.vdiff1weight
    }

    #[must_use]
    pub const fn vdiff2weight(self) -> f64 {
        self.vdiff2weight
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors DiversityWeightInit parameters without OCB"
)]
pub const fn diversity_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    fdiff1weight: f64,
    fdiff2weight: f64,
    vdiff1weight: f64,
    vdiff2weight: f64,
    app_var_mult: f64,
) -> DiversityWeightParam {
    DiversityWeightParam::new(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        fdiff1weight,
        fdiff2weight,
        vdiff1weight,
        vdiff2weight,
        app_var_mult,
    )
}

#[must_use]
pub fn diversity_weight_compute(
    param: &DiversityWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    let mut result = clause.literal_weight(
        bank,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        false,
    );

    let mut fcodes = Vec::new();
    let f_diversity = i64_to_f64(clause.return_fcodes(&mut fcodes));

    let mut vars = BTreeMap::new();
    let v_diversity = i64_to_f64(clause.collect_variables(&mut vars));

    result += f_diversity * (param.fdiff2weight * f_diversity + param.fdiff1weight);
    result += v_diversity * (param.vdiff2weight * v_diversity + param.vdiff1weight);

    result
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{diversity_weight_compute, diversity_weight_init, DEFAULT_MAX_MULT};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    #[test]
    fn diversity_weight_adds_function_and_variable_diversity_penalties() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "a");
        let clause = unit_clause(&mut bank, &x, &a, true);
        let param = diversity_weight_init(2, 3, 1.0, 1.0, 1.0, 10.0, 1.0, 20.0, 2.0, 1.0);

        assert_close(diversity_weight_compute(&param, &bank, &clause), 40.0);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 3);
        assert_close(param.max_term_multiplier(), 1.0);
        assert_close(param.max_literal_multiplier(), 1.0);
        assert_close(param.pos_multiplier(), 1.0);
        assert_close(param.fdiff1weight(), 10.0);
        assert_close(param.fdiff2weight(), 1.0);
        assert_close(param.vdiff1weight(), 20.0);
        assert_close(param.vdiff2weight(), 2.0);
        assert_close(param.app_var_mult(), 1.0);
        assert_close(DEFAULT_MAX_MULT, 1.5);
    }

    #[test]
    fn diversity_weight_uses_stored_maximal_literal_flags() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut literal = Eqn::alloc(a, b, &mut bank, true).unwrap();
        literal.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        let param = diversity_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 1.0);

        assert_close(diversity_weight_compute(&param, &bank, &clause), 564.0);
    }
}
