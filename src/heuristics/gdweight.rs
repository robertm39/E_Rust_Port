use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_is_ground, term_weight_compute};
use crate::terms::termtypes::{Term, TP_IS_CONJECTURE_TERM};

pub const DEFAULT_POS_MULT: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GdWeightEvaluator {
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
    goal_multiplier: f64,
    goal_const: i64,
    goal_terms_initialized: bool,
}

impl GdWeightEvaluator {
    #[must_use]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        pos_multiplier: f64,
        goal_multiplier: f64,
        goal_const: i64,
        app_var_mult: f64,
    ) -> Self {
        Self {
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
            goal_multiplier,
            goal_const,
            goal_terms_initialized: false,
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

    #[must_use]
    pub const fn goal_multiplier(self) -> f64 {
        self.goal_multiplier
    }

    #[must_use]
    pub const fn goal_const(self) -> i64 {
        self.goal_const
    }

    #[must_use]
    pub const fn goal_terms_initialized(self) -> bool {
        self.goal_terms_initialized
    }

    pub fn compute(&mut self, axioms: &ClauseSet, bank: &TermBank, clause: &Clause) -> f64 {
        gd_clause_weight_compute(self, axioms, bank, clause)
    }
}

#[must_use]
pub const fn gd_clause_weight_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    goal_multiplier: f64,
    goal_const: i64,
    app_var_mult: f64,
) -> GdWeightEvaluator {
    GdWeightEvaluator::new(
        fweight,
        vweight,
        pos_multiplier,
        goal_multiplier,
        goal_const,
        app_var_mult,
    )
}

#[must_use]
pub fn gd_term_weight(
    term: &Term,
    vweight: i64,
    fweight: i64,
    goal_multiplier: f64,
    goal_const: i64,
) -> i64 {
    if term_is_ground(term) && term.query_prop(TP_IS_CONJECTURE_TERM) {
        if goal_multiplier == 0.0 {
            return goal_const;
        }
        let swapped_weight = term_weight_compute(term, fweight, vweight);
        return f64_to_i64(i64_to_f64(goal_const) + goal_multiplier * i64_to_f64(swapped_weight));
    }

    if term.is_free_var() || (term.is_applied_free_var() && term.is_pattern()) {
        return vweight;
    }

    let mut result = if term.is_phony_app() || term.is_db_lambda() {
        0
    } else {
        fweight
    };
    for arg in term
        .argument_clones()
        .into_iter()
        .enumerate()
        .skip(usize::from(term.is_db_lambda()))
        .filter_map(|(_index, arg)| arg)
    {
        result += gd_term_weight(&arg, vweight, fweight, goal_multiplier, goal_const);
    }
    result
}

#[must_use]
pub fn gd_literal_weight(eqn: &Eqn, bank: &TermBank, param: &GdWeightEvaluator) -> f64 {
    let mut result = 0.0;
    if eqn.is_equ_lit(bank) {
        result = i64_to_f64(gd_term_weight(
            eqn.right(),
            param.vweight,
            param.fweight,
            param.goal_multiplier,
            param.goal_const,
        ));
        result = apply_app_var_mult(result, eqn.right(), param.app_var_mult);
        result += i64_to_f64(param.fweight);
    }

    let left_weight = i64_to_f64(gd_term_weight(
        eqn.left(),
        param.vweight,
        param.fweight,
        param.goal_multiplier,
        param.goal_const,
    ));
    result += apply_app_var_mult(left_weight, eqn.left(), param.app_var_mult);

    if eqn.is_positive() {
        result *= param.pos_multiplier;
    }
    result
}

#[must_use]
pub fn gd_clause_weight(param: &GdWeightEvaluator, bank: &TermBank, clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| gd_literal_weight(literal, bank, param))
        .sum()
}

pub fn initialize_goal_terms(axioms: &ClauseSet) {
    for clause in axioms.iter() {
        if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
            clause.term_set_prop(TP_IS_CONJECTURE_TERM);
        }
    }
}

pub fn gd_clause_weight_compute(
    evaluator: &mut GdWeightEvaluator,
    axioms: &ClauseSet,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    if !evaluator.goal_terms_initialized {
        initialize_goal_terms(axioms);
        evaluator.goal_terms_initialized = true;
    }
    gd_clause_weight(evaluator, bank, clause)
}

fn apply_app_var_mult(weight: f64, term: &Term, app_var_mult: f64) -> f64 {
    if term.is_applied_free_var() {
        weight * app_var_mult
    } else {
        weight
    }
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_i64(value: f64) -> i64 {
    value as i64
}

#[cfg(test)]
mod tests {
    use super::{
        gd_clause_weight_compute, gd_clause_weight_init, gd_term_weight, DEFAULT_POS_MULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_IS_CONJECTURE_TERM};
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

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        let literal = Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    #[test]
    fn gd_weight_initializes_negated_conjecture_terms_once() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut goal_clause = unit_clause(&mut bank, &a, &b, false);
        goal_clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let axioms = ClauseSet::from_clauses([goal_clause]);
        let target = unit_clause(&mut bank, &a, &b, true);
        let mut evaluator = gd_clause_weight_init(2, 1, 3.0, 0.0, 5, 1.0);

        assert!(!a.query_prop(TP_IS_CONJECTURE_TERM));
        assert_close(
            gd_clause_weight_compute(&mut evaluator, &axioms, &bank, &target),
            36.0,
        );
        assert!(a.query_prop(TP_IS_CONJECTURE_TERM));
        assert!(b.query_prop(TP_IS_CONJECTURE_TERM));
        assert!(evaluator.goal_terms_initialized());
        assert_close(evaluator.compute(&axioms, &bank, &target), 36.0);
        assert_eq!(evaluator.fweight(), 2);
        assert_eq!(evaluator.vweight(), 1);
        assert_close(evaluator.pos_multiplier(), 3.0);
        assert_close(evaluator.goal_multiplier(), 0.0);
        assert_eq!(evaluator.goal_const(), 5);
        assert_close(evaluator.app_var_mult(), 1.0);
        assert_close(DEFAULT_POS_MULT, 1.0);
    }

    #[test]
    fn gd_term_weight_preserves_goal_weight_swap_and_truncation() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let fa = typed_unary(&mut bank, "f", &a);
        fa.set_prop(TP_IS_CONJECTURE_TERM);

        assert_eq!(gd_term_weight(&fa, 3, 10, 1.25, 2), 9);
    }
}
