use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_is_ground, term_weight};
use crate::terms::termtypes::{Term, TP_IS_CONJECTURE_TERM};

pub const DEFAULT_POS_MULT: f64 = 1.0;
const APP_VAR_MULT_DEFAULT: f64 = 1.0;

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

#[derive(Clone, Debug, PartialEq)]
pub struct GdWeightWfcbData {
    evaluator: GdWeightEvaluator,
    axioms: ClauseSet,
}

impl GdWeightWfcbData {
    #[must_use]
    pub fn new(evaluator: GdWeightEvaluator, axioms: ClauseSet) -> Self {
        Self { evaluator, axioms }
    }

    #[must_use]
    pub const fn evaluator(&self) -> &GdWeightEvaluator {
        &self.evaluator
    }

    #[must_use]
    pub const fn axioms(&self) -> &ClauseSet {
        &self.axioms
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
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors GDClauseWeightInit parameters without OCB"
)]
pub fn gd_clause_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    goal_multiplier: f64,
    goal_const: i64,
    app_var_mult: f64,
) -> Wfcb<GdWeightWfcbData> {
    wfcb_alloc(
        gd_clause_weight_wfcb_compute,
        prio_fun,
        gd_clause_weight_exit,
        Some(GdWeightWfcbData::new(
            gd_clause_weight_init(
                fweight,
                vweight,
                pos_multiplier,
                goal_multiplier,
                goal_const,
                app_var_mult,
            ),
            axioms.clone(),
        )),
    )
}

pub fn gd_clause_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<GdWeightWfcbData>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let goal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let goal_const = parse_int(scanner)?;

    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(gd_clause_weight_wfcb_init(
        prio_fun,
        axioms,
        fweight,
        vweight,
        pos_multiplier,
        goal_multiplier,
        goal_const,
        app_var_mult,
    ))
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
        let swapped_weight = term_weight(term, fweight, vweight);
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

fn gd_clause_weight_wfcb_compute(
    data: Option<&mut GdWeightWfcbData>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    let data = data.unwrap_or_else(|| panic!("GDWeight WFCB requires initialized parameters"));
    data.evaluator.compute(&data.axioms, bank, clause)
}

fn gd_clause_weight_exit(_data: GdWeightWfcbData) {}

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
        gd_clause_weight_compute, gd_clause_weight_init, gd_clause_weight_parse, gd_term_weight,
        DEFAULT_POS_MULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::inout::scanner::Scanner;
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
    fn gd_weight_parse_initializes_goal_terms_from_axiom_context() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut goal_clause = unit_clause(&mut bank, &a, &b, false);
        goal_clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let axioms = ClauseSet::from_clauses([goal_clause]);
        let target = unit_clause(&mut bank, &a, &b, true);
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,3.0,0.0,5) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb =
            gd_clause_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| panic!("{err}"));

        assert!(!a.query_prop(TP_IS_CONJECTURE_TERM));
        assert_close(wfcb.compute_eval(&bank, &target), 36.0);
        assert!(a.query_prop(TP_IS_CONJECTURE_TERM));
        assert!(b.query_prop(TP_IS_CONJECTURE_TERM));
        assert_eq!(wfcb.compute_priority(&bank, &target), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
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
