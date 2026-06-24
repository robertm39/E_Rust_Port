use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_parse_operator;

const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct FunWeightParam {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
    weight_stack: Vec<(String, i64)>,
    flimit: FunCode,
    fweights: Option<Vec<i64>>,
    f_occur: Option<PDIntArray>,
}

impl FunWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors FunWeightInit parameters without OCB"
    )]
    pub fn new(
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        weight_stack: Vec<(String, i64)>,
        app_var_mult: f64,
        with_occurrences: bool,
    ) -> Self {
        Self {
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
            weight_stack,
            flimit: 0,
            fweights: None,
            f_occur: with_occurrences.then(|| PDIntArray::new_int(8, 0)),
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
    pub fn weight_stack(&self) -> &[(String, i64)] {
        &self.weight_stack
    }

    #[must_use]
    pub const fn flimit(&self) -> FunCode {
        self.flimit
    }

    #[must_use]
    pub fn fweights(&self) -> Option<&[i64]> {
        self.fweights.as_deref()
    }

    fn ensure_fun_weights(&mut self, bank: &TermBank) {
        if self.fweights.is_some() {
            return;
        }

        self.flimit = bank.signature().f_count() + 1;
        let len = usize::try_from(self.flimit)
            .unwrap_or_else(|_| panic!("signature f-count must fit vector length"));
        let mut fweights = vec![0; len];
        for weight in fweights.iter_mut().skip(1) {
            *weight = self.fweight;
        }

        for (name, weight) in &self.weight_stack {
            let f_code = bank.signature().find_f_code(name);
            if f_code != 0 && f_code < self.flimit {
                let index = usize::try_from(f_code)
                    .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
                fweights[index] = *weight;
            }
        }

        self.fweights = Some(fweights);
    }

    fn weight_for_f_code(&self, f_code: FunCode) -> i64 {
        if f_code < self.flimit {
            let index = usize::try_from(f_code)
                .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
            self.fweights
                .as_ref()
                .and_then(|weights| weights.get(index))
                .copied()
                .unwrap_or(self.fweight)
        } else {
            self.fweight
        }
    }
}

#[must_use]
pub fn fun_weight_init(
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    fweights: Vec<(String, i64)>,
    app_var_mult: f64,
) -> FunWeightParam {
    FunWeightParam::new(
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        fweights,
        app_var_mult,
        false,
    )
}

#[must_use]
pub fn sym_offset_weight_init(
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    fweights: Vec<(String, i64)>,
    app_var_mult: f64,
) -> FunWeightParam {
    FunWeightParam::new(
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        fweights,
        app_var_mult,
        true,
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors FunWeightInit parameters without OCB"
)]
pub fn fun_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    fweights: Vec<(String, i64)>,
    app_var_mult: f64,
) -> Wfcb<FunWeightParam> {
    wfcb_alloc(
        generic_fun_weight_wfcb_compute,
        prio_fun,
        fun_weight_exit,
        Some(fun_weight_init(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            fweight,
            fweights,
            app_var_mult,
        )),
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors SymOffsetWeightInit parameters without OCB"
)]
pub fn sym_offset_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    fweights: Vec<(String, i64)>,
    app_var_mult: f64,
) -> Wfcb<FunWeightParam> {
    wfcb_alloc(
        sym_offset_weight_wfcb_compute,
        prio_fun,
        fun_weight_exit,
        Some(sym_offset_weight_init(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            fweight,
            fweights,
            app_var_mult,
        )),
    )
}

pub fn fun_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_fun_weight_param(scanner, false)?;
    Ok(fun_weight_wfcb_init(
        prio_fun,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.weight_stack,
        param.app_var_mult,
    ))
}

pub fn sym_offset_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    let (prio_fun, param) = parse_fun_weight_param(scanner, true)?;
    Ok(sym_offset_weight_wfcb_init(
        prio_fun,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.weight_stack,
        param.app_var_mult,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if the lazy function-weight vector cannot be initialized, matching
/// the C WFCB invariant that compute is only called with initialized data.
pub fn generic_fun_weight_compute(
    param: &mut FunWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_fun_weights(bank);
    clause.fun_weight(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.flimit,
        param
            .fweights
            .as_deref()
            .unwrap_or_else(|| panic!("FunWeight vector must be initialized")),
        param.fweight,
        param.app_var_mult,
        None,
    )
}

pub fn fun_weight_compute(param: &mut FunWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    generic_fun_weight_compute(param, bank, clause)
}

#[must_use]
/// # Panics
///
/// Panics if the parameter cell was not initialized for symbol-offset scoring,
/// or if occurrence-array index conversion fails for a positive f-code.
pub fn sym_offset_weight_compute(
    param: &mut FunWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_fun_weights(bank);
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

    let mut symbols = Vec::new();
    {
        let f_occur = param
            .f_occur
            .as_mut()
            .unwrap_or_else(|| panic!("SymOffsetWeight requires an occurrence array"));
        clause.add_fun_occs(f_occur, &mut symbols);
    }

    while let Some(f_code) = symbols.pop() {
        result += i64_to_f64(param.weight_for_f_code(f_code));
        let f_occur = param
            .f_occur
            .as_mut()
            .unwrap_or_else(|| panic!("SymOffsetWeight requires an occurrence array"));
        assert!(
            f_occur.assign(f_code_to_pd_index(f_code), 0),
            "function-occurrence array must cover positive f-codes"
        );
    }

    result
}

fn generic_fun_weight_wfcb_compute(
    data: Option<&mut FunWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    generic_fun_weight_compute(
        data.unwrap_or_else(|| panic!("FunWeight WFCB requires initialized parameters")),
        bank,
        clause,
    )
}

fn sym_offset_weight_wfcb_compute(
    data: Option<&mut FunWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    sym_offset_weight_compute(
        data.unwrap_or_else(|| panic!("SymOffsetWeight WFCB requires initialized parameters")),
        bank,
        clause,
    )
}

fn fun_weight_exit(_data: FunWeightParam) {}

fn parse_fun_weight_param(
    scanner: &mut Scanner,
    signed_weights: bool,
) -> Result<(ClausePrioFun, FunWeightParam), Diagnostic> {
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

    let mut weights = Vec::new();
    while scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        weights.push(parse_op_weight(scanner, signed_weights)?);
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok((
        prio_fun,
        FunWeightParam::new(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            fweight,
            weights,
            APP_VAR_MULT_DEFAULT,
            signed_weights,
        ),
    ))
}

fn parse_op_weight(
    scanner: &mut Scanner,
    signed_weight: bool,
) -> Result<(String, i64), Diagnostic> {
    let mut op = DynamicString::new();
    term_parse_operator(scanner, &mut op)?;
    scanner.accept_tok(TokenType::COLON)?;
    let weight = if signed_weight {
        parse_int(scanner)?
    } else {
        let token = scanner.current_token().clone();
        scanner.accept_tok(TokenType::POS_INT)?;
        i64::try_from(token.numval()).map_err(|_| {
            Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{} unsigned function weight does not fit long",
                    token_pos_rep(&token)
                ),
            )
        })?
    };
    Ok((op.view().into_owned(), weight))
}

fn f_code_to_pd_index(f_code: FunCode) -> PDArrayIndex {
    PDArrayIndex::try_from(f_code)
        .unwrap_or_else(|_| panic!("positive f-code must fit dynamic-array index"))
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        fun_weight_compute, fun_weight_init, fun_weight_parse, sym_offset_weight_compute,
        sym_offset_weight_init, sym_offset_weight_parse,
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
        let arrow = alloc_arrow_type(vec![type_.clone(), type_.clone()]);
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn test_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let b = typed_const(bank, "b");
        let f_of_a = typed_unary(bank, "f", &a);
        let g_of_b = typed_unary(bank, "g", &b);
        let literal = Eqn::alloc(f_of_a, g_of_b, bank, true).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    #[test]
    fn fun_weight_uses_lazy_named_symbol_weights() {
        let mut bank = test_bank();
        let clause = test_clause(&mut bank);
        let mut param = fun_weight_init(
            1.0,
            1.0,
            1.0,
            1,
            2,
            vec![
                ("f".to_owned(), 10),
                ("g".to_owned(), 20),
                ("missing".to_owned(), 99),
            ],
            1.0,
        );

        assert!(param.fweights().is_none());
        assert_close(fun_weight_compute(&mut param, &bank, &clause), 34.0);
        assert_eq!(param.flimit(), bank.signature().f_count() + 1);
        assert_eq!(param.weight_stack().len(), 3);
    }

    #[test]
    fn fun_weight_parse_wraps_generic_scoring() {
        let mut bank = test_bank();
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,f:10,g:20) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = fun_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 34.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn fun_weight_rejects_signed_symbol_weights_like_c_posint_parser() {
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,f:-1)", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let Err(error) = fun_weight_parse(&mut scanner) else {
            panic!("signed FunWeight symbol weight should fail");
        };

        assert!(error.to_string().contains("expected"));
    }

    #[test]
    fn sym_offset_weight_adds_one_offset_per_distinct_symbol() {
        let mut bank = test_bank();
        let clause = test_clause(&mut bank);
        let mut param = sym_offset_weight_init(
            1.0,
            1.0,
            1.0,
            1,
            2,
            vec![("f".to_owned(), 5), ("a".to_owned(), -1)],
            1.0,
        );

        assert_close(sym_offset_weight_compute(&mut param, &bank, &clause), 18.0);
        assert_close(sym_offset_weight_compute(&mut param, &bank, &clause), 18.0);
    }

    #[test]
    fn sym_offset_weight_parse_accepts_signed_offsets() {
        let mut bank = test_bank();
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,f:5,a:-1) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = sym_offset_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 18.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn fun_weight_preserves_trailing_comma_as_operator_weight_quirk() {
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,2.5)", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let Err(error) = fun_weight_parse(&mut scanner) else {
            panic!("bare trailing app-var multiplier should fail");
        };

        assert!(error.to_string().contains("Colon"));
    }
}
