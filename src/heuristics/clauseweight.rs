use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;

pub const DEFAULT_POS_MULT: f64 = 1.0;
const APP_VAR_MULT_DEFAULT: f64 = 1.0;

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
pub fn clause_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<WeightParam> {
    wfcb_alloc(
        clause_weight_wfcb_compute,
        prio_fun,
        weight_param_exit,
        Some(clause_weight_init(
            fweight,
            vweight,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn lmax_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<WeightParam> {
    wfcb_alloc(
        lmax_weight_wfcb_compute,
        prio_fun,
        weight_param_exit,
        Some(lmax_weight_init(
            fweight,
            vweight,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn cmax_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<WeightParam> {
    wfcb_alloc(
        cmax_weight_wfcb_compute,
        prio_fun,
        weight_param_exit,
        Some(cmax_weight_init(
            fweight,
            vweight,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn uniq_weight_wfcb_init(prio_fun: ClausePrioFun) -> Wfcb<()> {
    wfcb_alloc(
        uniq_weight_wfcb_compute,
        prio_fun,
        trivial_weight_exit,
        None,
    )
}

#[must_use]
pub fn default_weight_wfcb_init(prio_fun: ClausePrioFun) -> Wfcb<()> {
    wfcb_alloc(
        default_weight_wfcb_compute,
        prio_fun,
        trivial_weight_exit,
        None,
    )
}

pub fn clause_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<WeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_weight_param(scanner)?;
    Ok(clause_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

pub fn lmax_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<WeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_weight_param(scanner)?;
    Ok(lmax_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

pub fn cmax_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<WeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_weight_param(scanner)?;
    Ok(cmax_weight_wfcb_init(
        prio_fun,
        param.fweight(),
        param.vweight(),
        param.pos_multiplier(),
        param.app_var_mult(),
    ))
}

pub fn uniq_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<()>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(uniq_weight_wfcb_init(prio_fun))
}

pub fn default_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<()>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(default_weight_wfcb_init(prio_fun))
}

fn parse_weight_param(scanner: &mut Scanner) -> Result<(ClausePrioFun, WeightParam), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
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
        WeightParam::new(fweight, vweight, pos_multiplier, app_var_mult),
    ))
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

fn clause_weight_wfcb_compute(
    data: Option<&mut WeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => clause_weight_compute(data, bank, clause),
        None => panic!("Clauseweight WFCB requires initialized weight parameters"),
    }
}

fn lmax_weight_wfcb_compute(
    data: Option<&mut WeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => lmax_weight_compute(data, clause),
        None => panic!("ClauseLMaxWeight WFCB requires initialized weight parameters"),
    }
}

fn cmax_weight_wfcb_compute(
    data: Option<&mut WeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => cmax_weight_compute(data, clause),
        None => panic!("ClauseCMaxWeight WFCB requires initialized weight parameters"),
    }
}

fn uniq_weight_wfcb_compute(_data: Option<&mut ()>, _bank: &TermBank, clause: &Clause) -> f64 {
    uniq_weight_compute(clause)
}

fn default_weight_wfcb_compute(_data: Option<&mut ()>, _bank: &TermBank, clause: &Clause) -> f64 {
    default_weight_compute(clause)
}

fn weight_param_exit(_data: WeightParam) {}

fn trivial_weight_exit(_data: ()) {}

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
        clause_weight_compute, clause_weight_init, clause_weight_parse, cmax_weight_compute,
        cmax_weight_init, cmax_weight_parse, default_weight_compute, default_weight_parse,
        lmax_weight_compute, lmax_weight_init, lmax_weight_parse, uniq_eqn_weight,
        uniq_term_weight, uniq_weight_compute, uniq_weight_parse, DEFAULT_POS_MULT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::inout::scanner::Scanner;
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
    fn clause_weight_parse_wraps_bank_aware_literal_weight() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,3.0) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut wfcb = clause_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 24.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn clause_weight_parse_accepts_optional_app_var_multiplier() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,3.0,2.5) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut wfcb = clause_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 24.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn lmax_and_cmax_weight_parsers_wrap_c_compatible_scoring() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let mut lmax_scanner = Scanner::from_user_string("(ConstPrio,2,1,3.0) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut cmax_scanner = Scanner::from_user_string("(ConstPrio,2,1,3.0) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut lmax = lmax_weight_parse(&mut lmax_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut cmax = cmax_weight_parse(&mut cmax_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(lmax.compute_eval(&bank, &clause), 0.0);
        assert_close(cmax.compute_eval(&bank, &clause), 8.0);
        assert_eq!(lmax.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(cmax.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(lmax_scanner.current_token().literal(), "tail");
        assert_eq!(cmax_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn uniq_and_default_weight_parsers_need_only_priority_function() {
        let mut bank = test_bank();
        let clause = mixed_clause(&mut bank);
        let mut uniq_scanner =
            Scanner::from_user_string("(ConstPrio) tail", false).unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut default_scanner = Scanner::from_user_string("(ConstPrio) tail", false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });
        let mut uniq = uniq_weight_parse(&mut uniq_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut default =
            default_weight_parse(&mut default_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(uniq.compute_eval(&bank, &clause), 36.0);
        assert_close(default.compute_eval(&bank, &clause), 8.0);
        assert_eq!(uniq.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(default.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(uniq_scanner.current_token().literal(), "tail");
        assert_eq!(default_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn uniq_term_weight_recurses_over_arguments_with_c_multipliers() {
        let mut bank = test_bank();
        let arg = typed_const(&mut bank, "a");
        let unary = typed_unary(&mut bank, "f", &arg);

        assert_close(uniq_term_weight(&unary), 7.0);
    }
}
