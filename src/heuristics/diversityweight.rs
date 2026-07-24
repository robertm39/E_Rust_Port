use std::collections::{BTreeMap, BTreeSet};

use crate::basics::error::Diagnostic;
use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, Term, TP_OP_FLAG};

pub const DEFAULT_MAX_MULT: f64 = 1.5;
const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
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
    scratch: DiversityWeightScratch,
}

#[derive(Clone, Debug, PartialEq)]
struct DiversityWeightScratch {
    // C allocates the variable set per evaluation. The WFCB owns the Rust
    // equivalent so ordinary clauses can retain its capacity.
    variable_ids: Vec<usize>,
}

const MAX_RETAINED_DIVERSITY_SCRATCH: usize = 1_024;

impl DiversityWeightScratch {
    fn new() -> Self {
        Self {
            variable_ids: Vec::new(),
        }
    }

    fn count_clause_diversity(&mut self, clause: &Clause) -> (i64, i64) {
        debug_assert!(self.variable_ids.is_empty());

        let mut subterms = PStack::new();
        for literal in clause.literals().as_slice() {
            for root in [literal.left(), literal.right()] {
                collect_diversity_subterms(root, &mut subterms, &mut self.variable_ids);
            }
        }

        let mut function_codes = BTreeSet::new();
        for term in subterms.as_slice() {
            term.del_prop(TP_OP_FLAG);
            if !term.is_any_var() {
                function_codes.insert(term.f_code());
            }
        }
        let function_count = i64::try_from(function_codes.len()).unwrap_or(i64::MAX);

        self.variable_ids.sort_unstable();
        self.variable_ids.dedup();
        let variable_count = i64::try_from(self.variable_ids.len()).unwrap_or(i64::MAX);

        self.variable_ids.clear();
        if self.variable_ids.capacity() > MAX_RETAINED_DIVERSITY_SCRATCH {
            self.variable_ids = Vec::new();
        }
        (function_count, variable_count)
    }
}

fn collect_diversity_subterms(
    term: &Term,
    subterms: &mut PStack<Term>,
    variable_ids: &mut Vec<usize>,
) {
    assert!(
        term.is_shared(),
        "diversity collection expects shared terms"
    );
    if term.is_free_var() {
        // ClauseReturnFCodes may encounter a variable whose operation flag was
        // left set by another C-shaped traversal. Variables do not contribute
        // function codes, so count them independently without touching that
        // flag.
        variable_ids.push(term_identity_id(term));
        return;
    }
    if term.query_prop(TP_OP_FLAG) {
        return;
    }

    term.set_prop(TP_OP_FLAG);
    subterms.push(term.clone());
    let arguments = term.arguments();
    for argument in arguments.iter().flatten() {
        collect_diversity_subterms(argument, subterms, variable_ids);
    }
}

impl DiversityWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors DiversityWeightInit parameters without OCB"
    )]
    pub fn new(
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
            scratch: DiversityWeightScratch::new(),
        }
    }

    #[must_use]
    pub const fn max_term_multiplier(&self) -> f64 {
        self.max_term_multiplier
    }

    #[must_use]
    pub const fn max_literal_multiplier(&self) -> f64 {
        self.max_literal_multiplier
    }

    #[must_use]
    pub const fn pos_multiplier(&self) -> f64 {
        self.pos_multiplier
    }

    #[must_use]
    pub const fn app_var_mult(&self) -> f64 {
        self.app_var_mult
    }

    #[must_use]
    pub const fn vweight(&self) -> i64 {
        self.vweight
    }

    #[must_use]
    pub const fn fweight(&self) -> i64 {
        self.fweight
    }

    #[must_use]
    pub const fn fdiff1weight(&self) -> f64 {
        self.fdiff1weight
    }

    #[must_use]
    pub const fn fdiff2weight(&self) -> f64 {
        self.fdiff2weight
    }

    #[must_use]
    pub const fn vdiff1weight(&self) -> f64 {
        self.vdiff1weight
    }

    #[must_use]
    pub const fn vdiff2weight(&self) -> f64 {
        self.vdiff2weight
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors DiversityWeightInit parameters without OCB"
)]
pub fn diversity_weight_init(
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
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors DiversityWeightInit parameters without OCB"
)]
pub fn diversity_weight_wfcb_init(
    prio_fun: ClausePrioFun,
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
) -> Wfcb<DiversityWeightParam> {
    wfcb_alloc_with_bank(
        diversity_weight_wfcb_compute,
        diversity_weight_wfcb_compute_with_bank,
        prio_fun,
        diversity_weight_exit,
        Some(diversity_weight_init(
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
        )),
    )
}

pub fn diversity_weight_parse(
    scanner: &mut Scanner,
) -> Result<Wfcb<DiversityWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fdiff1weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fdiff2weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vdiff1weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vdiff2weight = parse_float(scanner)?;

    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(diversity_weight_wfcb_init(
        prio_fun,
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
    ))
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
    let f_diversity = clause.return_fcodes(&mut fcodes);

    let mut vars = BTreeMap::new();
    let v_diversity = clause.collect_variables(&mut vars);

    let f_diversity = i64_to_f64(f_diversity);
    let v_diversity = i64_to_f64(v_diversity);

    result += f_diversity * (param.fdiff2weight * f_diversity + param.fdiff1weight);
    result += v_diversity * (param.vdiff2weight * v_diversity + param.vdiff1weight);

    result
}

fn diversity_weight_compute_reusing_scratch(
    param: &mut DiversityWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    let diversity = param.scratch.count_clause_diversity(clause);
    diversity_weight_compute_from_counts(param, bank, clause, diversity)
}

fn diversity_weight_compute_from_counts(
    param: &DiversityWeightParam,
    bank: &TermBank,
    clause: &Clause,
    (f_diversity, v_diversity): (i64, i64),
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

    let f_diversity = i64_to_f64(f_diversity);
    let v_diversity = i64_to_f64(v_diversity);

    result += f_diversity * (param.fdiff2weight * f_diversity + param.fdiff1weight);
    result += v_diversity * (param.vdiff2weight * v_diversity + param.vdiff1weight);

    result
}

/// Computes C `DiversityWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn diversity_weight_compute_with_ocb(
    param: &DiversityWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    diversity_weight_compute(param, bank, clause)
}

/// Computes C `DiversityWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn diversity_weight_compute_with_bank(
    param: &DiversityWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(diversity_weight_compute(param, bank, clause))
}

fn diversity_weight_wfcb_compute(
    data: Option<&mut DiversityWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => diversity_weight_compute_reusing_scratch(data, bank, clause),
        None => panic!("Diversityweight WFCB requires initialized weight parameters"),
    }
}

fn diversity_weight_wfcb_compute_with_bank(
    data: Option<&mut DiversityWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    match data {
        Some(data) => {
            clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
            Ok(diversity_weight_compute_reusing_scratch(data, bank, clause))
        }
        None => panic!("Diversityweight WFCB requires initialized weight parameters"),
    }
}

fn diversity_weight_exit(_data: DiversityWeightParam) {}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        diversity_weight_compute, diversity_weight_compute_with_ocb, diversity_weight_init,
        diversity_weight_parse, DiversityWeightScratch, DEFAULT_MAX_MULT,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_MAXIMAL;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_OP_FLAG};
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

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn parsed_unit_clause(bank: &mut TermBank, left: &str, right: &str, positive: bool) -> Clause {
        let left = parse_in_bank(bank, left);
        let right = parse_in_bank(bank, right);
        unit_clause(bank, &left, &right, positive)
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
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

    #[test]
    fn diversity_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = diversity_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 1.0);
        let expected = diversity_weight_compute(&param, &bank, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = diversity_weight_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn diversity_weight_parse_banked_callback_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = diversity_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 1.0);
        let expected = diversity_weight_compute(&param, &bank, &manually_marked);
        let mut scanner = Scanner::from_user_string(
            "(ConstPrio,2,1,3.0,5.0,7.0,11.0,13.0,17.0,19.0) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = diversity_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = kbo_ocb(&bank);

        let actual = wfcb
            .compute_eval_with_bank(&mut ocb, &mut bank, &mut target)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn diversity_weight_parse_wraps_diversity_penalties() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "a");
        let clause = unit_clause(&mut bank, &x, &a, true);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,2,3,1.0,1.0,1.0,10.0,1.0,20.0,2.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = diversity_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 40.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn diversity_scratch_counts_stale_flagged_variables_repeatedly() {
        let mut bank = test_bank();
        let clause = parsed_unit_clause(&mut bank, "f(X)", "g(X)", true);
        let mut scratch = DiversityWeightScratch::new();
        let mut expected_fcodes = Vec::new();
        let expected_function_count = clause.return_fcodes(&mut expected_fcodes);
        let mut expected_variables = BTreeMap::new();
        let expected_variable_count = clause.collect_variables(&mut expected_variables);
        for variable in expected_variables.values() {
            variable.set_prop(TP_OP_FLAG);
        }

        assert_eq!(
            scratch.count_clause_diversity(&clause),
            (expected_function_count, expected_variable_count)
        );
        assert_eq!((expected_function_count, expected_variable_count), (2, 1));
        let retained_variable_capacity = scratch.variable_ids.capacity();
        assert!(scratch.variable_ids.is_empty());

        assert_eq!(scratch.count_clause_diversity(&clause), (2, 1));
        assert_eq!(scratch.variable_ids.capacity(), retained_variable_capacity);
        assert!(scratch.variable_ids.is_empty());
    }
}
