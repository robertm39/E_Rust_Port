use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;

pub const DEFAULT_MAX_MULT: f64 = 1.5;
const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrientWeightParam {
    unorientable_literal_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
}

impl OrientWeightParam {
    #[must_use]
    pub const fn new(
        fweight: i64,
        vweight: i64,
        unorientable_literal_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        app_var_mult: f64,
    ) -> Self {
        Self {
            unorientable_literal_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
        }
    }

    #[must_use]
    pub const fn unorientable_literal_multiplier(self) -> f64 {
        self.unorientable_literal_multiplier
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
pub const fn clause_orient_weight_init(
    fweight: i64,
    vweight: i64,
    unorientable_literal_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> OrientWeightParam {
    OrientWeightParam::new(
        fweight,
        vweight,
        unorientable_literal_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    )
}

#[must_use]
pub const fn orient_lmax_weight_init(
    fweight: i64,
    vweight: i64,
    unorientable_literal_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> OrientWeightParam {
    OrientWeightParam::new(
        fweight,
        vweight,
        unorientable_literal_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    )
}

#[must_use]
pub fn clause_orient_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    unorientable_literal_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<OrientWeightParam> {
    wfcb_alloc_with_bank(
        clause_orient_weight_wfcb_compute,
        clause_orient_weight_wfcb_compute_with_bank,
        prio_fun,
        orient_weight_exit,
        Some(clause_orient_weight_init(
            fweight,
            vweight,
            unorientable_literal_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn orient_lmax_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    unorientable_literal_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<OrientWeightParam> {
    wfcb_alloc_with_bank(
        orient_lmax_weight_wfcb_compute,
        orient_lmax_weight_wfcb_compute_with_bank,
        prio_fun,
        orient_weight_exit,
        Some(orient_lmax_weight_init(
            fweight,
            vweight,
            unorientable_literal_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

pub fn clause_orient_weight_parse(
    scanner: &mut Scanner,
) -> Result<Wfcb<OrientWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_orient_weight_param(scanner)?;
    Ok(clause_orient_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.unorientable_literal_multiplier(),
        param.max_literal_multiplier(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

pub fn orient_lmax_weight_parse(
    scanner: &mut Scanner,
) -> Result<Wfcb<OrientWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_orient_weight_param(scanner)?;
    Ok(orient_lmax_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.unorientable_literal_multiplier(),
        param.max_literal_multiplier(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

fn parse_orient_weight_param(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, OrientWeightParam), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let unorientable_literal_multiplier = parse_float(scanner)?;
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
        OrientWeightParam::new(
            fweight,
            vweight,
            unorientable_literal_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        ),
    ))
}

#[must_use]
pub fn clause_orient_weight_compute(
    param: &OrientWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    clause.orient_weight(
        bank,
        param.unorientable_literal_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        false,
    )
}

/// Computes C `ClauseOrientWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn clause_orient_weight_compute_with_ocb(
    param: &OrientWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    clause_orient_weight_compute(param, bank, clause)
}

/// Computes C `ClauseOrientWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn clause_orient_weight_compute_with_bank(
    param: &OrientWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(clause_orient_weight_compute(param, bank, clause))
}

#[must_use]
pub fn orient_lmax_weight_compute(param: &OrientWeightParam, clause: &Clause) -> f64 {
    let mut result = 0.0;
    for literal in clause.literals().as_slice() {
        let mut weight = literal.max_weight(param.vweight, param.fweight, param.app_var_mult);
        if literal.is_positive() {
            weight *= param.pos_multiplier;
        }
        if literal.is_maximal() {
            weight *= param.max_literal_multiplier;
        }
        if !literal.is_oriented() {
            weight *= param.unorientable_literal_multiplier;
        }
        result += weight;
    }
    result
}

/// Computes C `OrientLMaxWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn orient_lmax_weight_compute_with_ocb(
    param: &OrientWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    orient_lmax_weight_compute(param, clause)
}

/// Computes C `OrientLMaxWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn orient_lmax_weight_compute_with_bank(
    param: &OrientWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(orient_lmax_weight_compute(param, clause))
}

fn clause_orient_weight_wfcb_compute(
    data: Option<&mut OrientWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => clause_orient_weight_compute(data, bank, clause),
        None => panic!("Orientweight WFCB requires initialized weight parameters"),
    }
}

fn clause_orient_weight_wfcb_compute_with_bank(
    data: Option<&mut OrientWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    match data {
        Some(data) => clause_orient_weight_compute_with_bank(data, ocb, bank, clause),
        None => panic!("Orientweight WFCB requires initialized weight parameters"),
    }
}

fn orient_lmax_weight_wfcb_compute(
    data: Option<&mut OrientWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => orient_lmax_weight_compute(data, clause),
        None => panic!("OrientLMaxWeight WFCB requires initialized weight parameters"),
    }
}

fn orient_lmax_weight_wfcb_compute_with_bank(
    data: Option<&mut OrientWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    match data {
        Some(data) => orient_lmax_weight_compute_with_bank(data, ocb, bank, clause),
        None => panic!("OrientLMaxWeight WFCB requires initialized weight parameters"),
    }
}

fn orient_weight_exit(_data: OrientWeightParam) {}

#[cfg(test)]
mod tests {
    use super::{
        clause_orient_weight_compute, clause_orient_weight_compute_with_ocb,
        clause_orient_weight_init, clause_orient_weight_parse, orient_lmax_weight_compute,
        orient_lmax_weight_compute_with_ocb, orient_lmax_weight_init, orient_lmax_weight_parse,
        DEFAULT_MAX_MULT,
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
    fn clause_orient_weight_uses_stored_maximal_and_orientation_flags() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let param = clause_orient_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);

        assert_close(clause_orient_weight_compute(&param, &bank, &clause), 636.0);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 1);
        assert_close(param.unorientable_literal_multiplier(), 7.0);
        assert_close(param.max_literal_multiplier(), 5.0);
        assert_close(param.pos_multiplier(), 3.0);
        assert_close(param.app_var_mult(), 1.0);
        assert_close(DEFAULT_MAX_MULT, 1.5);
    }

    #[test]
    fn orient_lmax_weight_uses_max_term_weight_with_same_multipliers() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let param = orient_lmax_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);

        assert_close(orient_lmax_weight_compute(&param, &clause), 212.0);
    }

    #[test]
    fn clause_orient_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = clause_orient_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let expected = clause_orient_weight_compute(&param, &bank, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = clause_orient_weight_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn orient_lmax_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = orient_lmax_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let expected = orient_lmax_weight_compute(&param, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = orient_lmax_weight_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn orient_weight_parse_banked_callbacks_mark_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();

        let mut orient_target = parsed_unit_clause(&mut bank, "a", "f(a)", true);
        let mut orient_marked = orient_target.clone();
        let mut orient_manual_ocb = kbo_ocb(&bank);
        assert!(orient_marked.cond_mark_maximal_terms(&mut orient_manual_ocb, &bank));
        let orient_param = clause_orient_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let orient_expected = clause_orient_weight_compute(&orient_param, &bank, &orient_marked);
        let mut orient_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut orient =
            clause_orient_weight_parse(&mut orient_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut orient_ocb = kbo_ocb(&bank);

        let orient_actual = orient
            .compute_eval_with_bank(&mut orient_ocb, &mut bank, &mut orient_target)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(orient_actual, orient_expected);
        assert!(orient_target.query_prop(CP_IS_ORIENTED));
        assert!(orient_target.literals().as_slice()[0].is_maximal());
        assert_eq!(orient_scanner.current_token().literal(), "tail");

        let mut lmax_target = parsed_unit_clause(&mut bank, "b", "g(b)", true);
        let mut lmax_marked = lmax_target.clone();
        let mut lmax_manual_ocb = kbo_ocb(&bank);
        assert!(lmax_marked.cond_mark_maximal_terms(&mut lmax_manual_ocb, &bank));
        let lmax_param = orient_lmax_weight_init(2, 1, 7.0, 5.0, 3.0, 1.0);
        let lmax_expected = orient_lmax_weight_compute(&lmax_param, &lmax_marked);
        let mut lmax_scanner = Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut lmax =
            orient_lmax_weight_parse(&mut lmax_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut lmax_ocb = kbo_ocb(&bank);

        let lmax_actual = lmax
            .compute_eval_with_bank(&mut lmax_ocb, &mut bank, &mut lmax_target)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(lmax_actual, lmax_expected);
        assert!(lmax_target.query_prop(CP_IS_ORIENTED));
        assert!(lmax_target.literals().as_slice()[0].is_maximal());
        assert_eq!(lmax_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn orient_weight_parsers_wrap_existing_scoring_cores() {
        let mut bank = test_bank();
        let clause = marked_clause(&mut bank);
        let mut orient_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut lmax_scanner = Scanner::from_user_string("(ConstPrio,2,1,7.0,5.0,3.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut orient =
            clause_orient_weight_parse(&mut orient_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut lmax =
            orient_lmax_weight_parse(&mut lmax_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(orient.compute_eval(&bank, &clause), 636.0);
        assert_close(lmax.compute_eval(&bank, &clause), 212.0);
        assert_eq!(orient.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(lmax.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(orient_scanner.current_token().literal(), "tail");
        assert_eq!(lmax_scanner.current_token().literal(), "tail");
    }
}
