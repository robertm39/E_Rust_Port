use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;

pub const DEFAULT_MAX_MULT: f64 = 1.5;
const APP_VAR_MULT_DEFAULT: f64 = 1.0;

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
pub fn clause_refined_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<RefinedWeightParam> {
    wfcb_alloc(
        clause_refined_weight_wfcb_compute,
        prio_fun,
        refined_weight_exit,
        Some(clause_refined_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn clause_refined_weight2_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<RefinedWeightParam> {
    wfcb_alloc(
        clause_refined_weight2_wfcb_compute,
        prio_fun,
        refined_weight_exit,
        Some(clause_refined_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

pub fn clause_refined_weight_parse(
    scanner: &mut Scanner,
) -> Result<Wfcb<RefinedWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_refined_weight_param(scanner)?;
    Ok(clause_refined_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.max_term_multiplier(),
        param.max_literal_multiplier(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

pub fn clause_refined_weight2_parse(
    scanner: &mut Scanner,
) -> Result<Wfcb<RefinedWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_refined_weight_param(scanner)?;
    Ok(clause_refined_weight2_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.max_term_multiplier(),
        param.max_literal_multiplier(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

fn parse_refined_weight_param(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, RefinedWeightParam), Diagnostic> {
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

    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok((
        prio_fun,
        clause_refined_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        ),
    ))
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

/// Computes C `ClauseRefinedWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// The existing WFCB compute callback cannot mutate clauses yet, so this
/// explicit entry point is used by callers that already own a mutable clause.
#[must_use]
pub fn clause_refined_weight_compute_with_ocb(
    param: &RefinedWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    clause_refined_weight_compute(param, bank, clause)
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

/// Computes C `ClauseRefinedWeight2Compute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// The existing WFCB compute callback cannot mutate clauses yet, so this
/// explicit entry point is used by callers that already own a mutable clause.
#[must_use]
pub fn clause_refined_weight2_compute_with_ocb(
    param: &RefinedWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    clause_refined_weight2_compute(param, bank, clause)
}

fn clause_refined_weight_wfcb_compute(
    data: Option<&mut RefinedWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => clause_refined_weight_compute(data, bank, clause),
        None => panic!("Refinedweight WFCB requires initialized weight parameters"),
    }
}

fn clause_refined_weight2_wfcb_compute(
    data: Option<&mut RefinedWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => clause_refined_weight2_compute(data, bank, clause),
        None => panic!("Refinedweight2 WFCB requires initialized weight parameters"),
    }
}

fn refined_weight_exit(_data: RefinedWeightParam) {}

#[cfg(test)]
mod tests {
    use super::{
        clause_refined_weight2_compute, clause_refined_weight2_compute_with_ocb,
        clause_refined_weight2_parse, clause_refined_weight_compute,
        clause_refined_weight_compute_with_ocb, clause_refined_weight_init,
        clause_refined_weight_parse, DEFAULT_MAX_MULT,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
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

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn parsed_unit_clause(bank: &mut TermBank, left: &str, right: &str, positive: bool) -> Clause {
        let left = parse_in_bank(bank, left);
        let right = parse_in_bank(bank, right);
        Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            left, right, bank, positive,
        )
        .unwrap()]))
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

    #[test]
    fn refined_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = clause_refined_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let expected = clause_refined_weight_compute(&param, &bank, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = clause_refined_weight_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn refined_weight2_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = clause_refined_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let expected = clause_refined_weight2_compute(&param, &bank, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = clause_refined_weight2_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn refined_weight_parsers_wrap_existing_scoring_cores() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let mut standard_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut encoding_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0,2.5) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut refined = clause_refined_weight_parse(&mut standard_scanner)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut refined2 = clause_refined_weight2_parse(&mut encoding_scanner)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(refined.compute_eval(&bank, &clause), 468.0);
        assert_close(refined2.compute_eval(&bank, &clause), 436.0);
        assert_eq!(refined.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(refined2.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(standard_scanner.current_token().literal(), "tail");
        assert_eq!(encoding_scanner.current_token().literal(), "tail");
    }
}
