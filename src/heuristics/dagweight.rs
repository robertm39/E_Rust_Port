use crate::clauses::clause::Clause;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_dag_weight;
use crate::terms::termtypes::TP_OP_FLAG;

pub const DEFAULT_DAG_DUP_WEIGHT: i64 = 1;

#[expect(
    clippy::struct_excessive_bools,
    reason = "C-compatible DAGWeight parameters are a set of boolean reset modes"
)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DagWeightParam {
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    dup_weight: i64,
    pos_use_dag: bool,
    pos_term_reset: bool,
    pos_eqn_reset: bool,
    neg_use_dag: bool,
    neg_term_reset: bool,
    neg_eqn_reset: bool,
    pos_neg_reset: bool,
}

impl DagWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        clippy::fn_params_excessive_bools,
        reason = "C-compatible constructor mirrors DAGWeightInit"
    )]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        pos_multiplier: f64,
        dup_weight: i64,
        pos_use_dag: bool,
        pos_term_reset: bool,
        pos_eqn_reset: bool,
        neg_use_dag: bool,
        neg_term_reset: bool,
        neg_eqn_reset: bool,
        pos_neg_reset: bool,
    ) -> Self {
        Self {
            pos_multiplier,
            vweight,
            fweight,
            dup_weight,
            pos_use_dag,
            pos_term_reset,
            pos_eqn_reset,
            neg_use_dag,
            neg_term_reset,
            neg_eqn_reset,
            pos_neg_reset,
        }
    }

    #[must_use]
    pub const fn pos_multiplier(self) -> f64 {
        self.pos_multiplier
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
    pub const fn dup_weight(self) -> i64 {
        self.dup_weight
    }

    #[must_use]
    pub const fn pos_use_dag(self) -> bool {
        self.pos_use_dag
    }

    #[must_use]
    pub const fn pos_term_reset(self) -> bool {
        self.pos_term_reset
    }

    #[must_use]
    pub const fn pos_eqn_reset(self) -> bool {
        self.pos_eqn_reset
    }

    #[must_use]
    pub const fn neg_use_dag(self) -> bool {
        self.neg_use_dag
    }

    #[must_use]
    pub const fn neg_term_reset(self) -> bool {
        self.neg_term_reset
    }

    #[must_use]
    pub const fn neg_eqn_reset(self) -> bool {
        self.neg_eqn_reset
    }

    #[must_use]
    pub const fn pos_neg_reset(self) -> bool {
        self.pos_neg_reset
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    reason = "C-compatible helper mirrors DAGWeightInit"
)]
pub const fn dag_weight_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    dup_weight: i64,
    pos_use_dag: bool,
    pos_term_reset: bool,
    pos_eqn_reset: bool,
    neg_use_dag: bool,
    neg_term_reset: bool,
    neg_eqn_reset: bool,
    pos_neg_reset: bool,
) -> DagWeightParam {
    DagWeightParam::new(
        fweight,
        vweight,
        pos_multiplier,
        dup_weight,
        pos_use_dag,
        pos_term_reset,
        pos_eqn_reset,
        neg_use_dag,
        neg_term_reset,
        neg_eqn_reset,
        pos_neg_reset,
    )
}

#[must_use]
pub fn dag_weight_compute(param: &DagWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    clause.literals().term_del_prop(TP_OP_FLAG);

    let mut result = 0.0;
    for literal in clause.literals().as_slice() {
        if literal.is_positive() {
            if param.pos_use_dag {
                result += param.pos_multiplier
                    * literal.dag_weight(
                        1.0,
                        1.0,
                        param.vweight,
                        param.fweight,
                        param.dup_weight,
                        param.pos_eqn_reset,
                        param.pos_term_reset,
                    );
            } else {
                result += literal.literal_weight(
                    bank,
                    1.0,
                    1.0,
                    param.pos_multiplier,
                    param.vweight,
                    param.fweight,
                    1.0,
                    false,
                );
            }
        }
    }

    if param.pos_neg_reset {
        clause
            .literals()
            .signed_term_del_prop(TP_OP_FLAG, false, true);
    }

    for literal in clause.literals().as_slice() {
        if literal.is_negative() {
            if param.neg_use_dag {
                result += literal.dag_weight(
                    1.0,
                    1.0,
                    param.vweight,
                    param.fweight,
                    param.dup_weight,
                    param.neg_eqn_reset,
                    param.neg_term_reset,
                );
            } else {
                result += literal.literal_weight(
                    bank,
                    1.0,
                    1.0,
                    1.0,
                    param.vweight,
                    param.fweight,
                    1.0,
                    false,
                );
            }
        }
    }
    result
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RDagWeightParam {
    vweight: i64,
    fweight: i64,
    nvweight: i64,
    nfweight: i64,
    dup_weight: i64,
    uniqmax_term_multiplier: f64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    pneq_multiplier: f64,
    nneq_multiplier: f64,
}

impl RDagWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        clippy::similar_names,
        reason = "C-compatible constructor covers all RDAGWeight variants"
    )]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        nfweight: i64,
        nvweight: i64,
        dup_weight: i64,
        uniqmax_term_multiplier: f64,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        pneq_multiplier: f64,
        nneq_multiplier: f64,
    ) -> Self {
        Self {
            vweight,
            fweight,
            nvweight,
            nfweight,
            dup_weight,
            uniqmax_term_multiplier,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            pneq_multiplier,
            nneq_multiplier,
        }
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
    pub const fn nvweight(self) -> i64 {
        self.nvweight
    }

    #[must_use]
    pub const fn nfweight(self) -> i64 {
        self.nfweight
    }

    #[must_use]
    pub const fn dup_weight(self) -> i64 {
        self.dup_weight
    }

    #[must_use]
    pub const fn uniqmax_term_multiplier(self) -> f64 {
        self.uniqmax_term_multiplier
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
    pub const fn pneq_multiplier(self) -> f64 {
        self.pneq_multiplier
    }

    #[must_use]
    pub const fn nneq_multiplier(self) -> f64 {
        self.nneq_multiplier
    }
}

#[must_use]
pub const fn rdag_weight_init(
    fweight: i64,
    vweight: i64,
    dup_weight: i64,
    uniqmax_term_multiplier: f64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> RDagWeightParam {
    RDagWeightParam::new(
        fweight,
        vweight,
        fweight,
        vweight,
        dup_weight,
        uniqmax_term_multiplier,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        1.0,
        1.0,
    )
}

#[must_use]
pub fn rdag_weight_compute(param: &RDagWeightParam, clause: &Clause) -> f64 {
    clause.literals().term_del_prop(TP_OP_FLAG);
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            let pos_multiplier = if literal.is_positive() {
                param.pos_multiplier
            } else {
                1.0
            };
            pos_multiplier
                * literal.dag_weight(
                    param.uniqmax_term_multiplier,
                    param.max_term_multiplier,
                    param.vweight,
                    param.fweight,
                    param.dup_weight,
                    true,
                    true,
                )
        })
        .sum()
}

#[must_use]
pub const fn rdag_weight2_init(
    fweight: i64,
    vweight: i64,
    dup_weight: i64,
    max_term_multiplier: f64,
    pos_multiplier: f64,
) -> RDagWeightParam {
    RDagWeightParam::new(
        fweight,
        vweight,
        fweight,
        vweight,
        dup_weight,
        1.0,
        max_term_multiplier,
        1.0,
        pos_multiplier,
        1.0,
        1.0,
    )
}

#[must_use]
pub fn rdag_weight2_compute(param: &RDagWeightParam, clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            let pos_multiplier = if literal.is_positive() {
                param.pos_multiplier
            } else {
                1.0
            };
            pos_multiplier
                * literal.dag_weight2(
                    param.max_term_multiplier,
                    param.vweight,
                    param.fweight,
                    param.dup_weight,
                )
        })
        .sum()
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    clippy::similar_names,
    reason = "C-compatible helper mirrors RDAGWeight3Init without the OCB pointer"
)]
pub const fn rdag_weight3_init(
    fweight: i64,
    vweight: i64,
    nfweight: i64,
    nvweight: i64,
    dup_weight: i64,
    max_term_multiplier: f64,
    pos_multiplier: f64,
    pneq_multiplier: f64,
    nneq_multiplier: f64,
) -> RDagWeightParam {
    RDagWeightParam::new(
        fweight,
        vweight,
        nfweight,
        nvweight,
        dup_weight,
        1.0,
        max_term_multiplier,
        1.0,
        pos_multiplier,
        pneq_multiplier,
        nneq_multiplier,
    )
}

#[must_use]
pub fn rdag_weight3_compute(param: &RDagWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    let mut result = 0.0;
    for literal in clause.literals().as_slice() {
        let lit_weight = if literal.is_positive() {
            let mut left_weight = i64_to_f64(term_dag_weight(
                literal.left(),
                param.fweight,
                param.vweight,
                param.dup_weight,
                true,
            ));
            let mut right_weight = i64_to_f64(term_dag_weight(
                literal.right(),
                param.fweight,
                param.vweight,
                param.dup_weight,
                true,
            ));
            if left_weight >= right_weight {
                left_weight *= param.max_term_multiplier;
            } else {
                right_weight *= param.max_term_multiplier;
            }
            let mut lit_weight = (left_weight + right_weight) * param.pos_multiplier;
            if literal.is_equ_lit(bank) {
                lit_weight *= param.pneq_multiplier;
            }
            lit_weight
        } else {
            literal.term_del_prop(TP_OP_FLAG);
            let left_weight = i64_to_f64(term_dag_weight(
                literal.left(),
                param.nfweight,
                param.nvweight,
                param.dup_weight,
                false,
            ));
            let right_weight = i64_to_f64(term_dag_weight(
                literal.right(),
                param.nfweight,
                param.nvweight,
                param.dup_weight,
                false,
            ));
            let mut lit_weight = left_weight + right_weight;
            if literal.is_equ_lit(bank) {
                lit_weight *= param.nneq_multiplier;
            }
            lit_weight
        };
        result += lit_weight;
    }
    result
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        dag_weight_compute, dag_weight_init, rdag_weight2_compute, rdag_weight2_init,
        rdag_weight3_compute, rdag_weight3_init, rdag_weight_compute, rdag_weight_init,
        DEFAULT_DAG_DUP_WEIGHT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_ORIENTED;
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

    fn typed_pred_const(bank: &mut TermBank, name: &str) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, bool_type.clone())
            .unwrap();
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(bool_type));
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

    fn shared_positive_negative_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let fa = typed_unary(bank, "f", &a);
        let positive = Eqn::alloc(fa.clone(), fa.clone(), bank, true).unwrap();
        let negative = Eqn::alloc(fa, a, bank, false).unwrap();
        Clause::alloc(EqnList::from_vec(vec![positive, negative]))
    }

    #[test]
    fn dag_weight_compute_preserves_positive_negative_reset_boundary() {
        let mut bank = test_bank();
        let clause = shared_positive_negative_clause(&mut bank);
        let no_reset = dag_weight_init(
            2,
            1,
            3.0,
            DEFAULT_DAG_DUP_WEIGHT,
            true,
            false,
            false,
            true,
            false,
            false,
            false,
        );
        let reset = dag_weight_init(
            2,
            1,
            3.0,
            DEFAULT_DAG_DUP_WEIGHT,
            true,
            false,
            false,
            true,
            false,
            false,
            true,
        );

        assert_close(dag_weight_compute(&no_reset, &bank, &clause), 17.0);
        assert_close(dag_weight_compute(&reset, &bank, &clause), 20.0);
        assert_close(no_reset.pos_multiplier(), 3.0);
        assert_eq!(no_reset.fweight(), 2);
        assert_eq!(no_reset.vweight(), 1);
        assert_eq!(no_reset.dup_weight(), DEFAULT_DAG_DUP_WEIGHT);
        assert!(no_reset.pos_use_dag());
        assert!(no_reset.neg_use_dag());
        assert!(!no_reset.pos_neg_reset());
    }

    #[test]
    fn dag_weight_compute_uses_literal_weight_when_dag_disabled() {
        let mut bank = test_bank();
        let clause = shared_positive_negative_clause(&mut bank);
        let param = dag_weight_init(
            2,
            1,
            3.0,
            DEFAULT_DAG_DUP_WEIGHT,
            false,
            true,
            true,
            false,
            true,
            true,
            true,
        );

        assert_close(dag_weight_compute(&param, &bank, &clause), 38.0);
        assert!(param.pos_term_reset());
        assert!(param.pos_eqn_reset());
        assert!(param.neg_term_reset());
        assert!(param.neg_eqn_reset());
    }

    #[test]
    fn refined_dag_weight_compute_uses_marked_orientation_flags() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let fa = typed_unary(&mut bank, "f", &a);
        let ga = typed_unary(&mut bank, "g", &a);
        let mut positive = Eqn::alloc(fa, ga, &mut bank, true).unwrap();
        positive.set_prop(EP_IS_ORIENTED);
        let clause = Clause::alloc(EqnList::from_vec(vec![positive]));
        let param = rdag_weight_init(10, 3, 1, 5.0, 2.0, 7.0, 4.0);

        assert_close(rdag_weight_compute(&param, &clause), 880.0);
        assert_eq!(param.fweight(), 10);
        assert_eq!(param.vweight(), 3);
        assert_eq!(param.dup_weight(), 1);
        assert_close(param.uniqmax_term_multiplier(), 5.0);
        assert_close(param.max_term_multiplier(), 2.0);
        assert_close(param.max_literal_multiplier(), 7.0);
        assert_close(param.pos_multiplier(), 4.0);
    }

    #[test]
    fn refined_dag_weight2_boosts_larger_side_without_orientation() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let fa = typed_unary(&mut bank, "f", &a);
        let ga = typed_unary(&mut bank, "g", &a);
        let positive = Eqn::alloc(fa, ga, &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![positive]));
        let param = rdag_weight2_init(10, 3, 1, 4.0, 2.0);

        assert_close(rdag_weight2_compute(&param, &clause), 200.0);
    }

    #[test]
    fn refined_dag_weight3_preserves_equational_multiplier_condition() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let positive_eq = Eqn::alloc(a.clone(), b.clone(), &mut bank, true).unwrap();
        let negative_eq = Eqn::alloc(a, b, &mut bank, false).unwrap();
        let pred = typed_pred_const(&mut bank, "p");
        let pred_lit = Eqn::alloc(pred, bank.true_term().clone(), &mut bank, true).unwrap();
        let clause = Clause::alloc(EqnList::from_vec(vec![positive_eq, pred_lit, negative_eq]));
        let param = rdag_weight3_init(2, 1, 13, 17, 1, 3.0, 5.0, 7.0, 11.0);

        assert_close(rdag_weight3_compute(&param, &bank, &clause), 606.0);
        assert_eq!(param.nfweight(), 13);
        assert_eq!(param.nvweight(), 17);
        assert_close(param.pneq_multiplier(), 7.0);
        assert_close(param.nneq_multiplier(), 11.0);
    }
}
