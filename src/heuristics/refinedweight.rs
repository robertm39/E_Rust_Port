use crate::clauses::clause::Clause;
use crate::terms::termbanks::TermBank;

pub const DEFAULT_MAX_MULT: f64 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefinedWeightParam {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
}

impl RefinedWeightParam {
    #[must_use]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        app_var_mult: f64,
    ) -> Self {
        Self {
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
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
}

#[must_use]
pub const fn clause_refined_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> RefinedWeightParam {
    RefinedWeightParam::new(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    )
}

#[must_use]
pub fn clause_refined_weight_compute(
    param: &RefinedWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    clause.literal_weight(
        bank,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        false,
    )
}

#[must_use]
pub fn clause_refined_weight2_compute(
    param: &RefinedWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    clause.literal_weight(
        bank,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        clause_refined_weight2_compute, clause_refined_weight_compute, clause_refined_weight_init,
        DEFAULT_MAX_MULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
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

    fn marked_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let b = typed_const(bank, "b");
        let mut positive = Eqn::alloc(a.clone(), b.clone(), bank, true).unwrap();
        positive.set_prop(EP_IS_MAXIMAL);
        let mut negative = Eqn::alloc(a, b, bank, false).unwrap();
        negative.set_prop(EP_IS_ORIENTED);
        Clause::alloc(EqnList::from_vec(vec![positive, negative]))
    }

    #[test]
    fn refined_weight_uses_stored_maximal_and_orientation_flags() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let param = clause_refined_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);

        assert_close(clause_refined_weight_compute(&param, &bank, &clause), 468.0);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 1);
        assert_close(param.max_term_multiplier(), 7.0);
        assert_close(param.max_literal_multiplier(), 5.0);
        assert_close(param.pos_multiplier(), 3.0);
        assert_close(param.app_var_mult(), 1.0);
        assert_close(DEFAULT_MAX_MULT, 1.5);
    }

    #[test]
    fn refined_weight2_counts_equality_encoding() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let param = clause_refined_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);

        assert_close(
            clause_refined_weight2_compute(&param, &bank, &clause),
            436.0,
        );
    }
}
