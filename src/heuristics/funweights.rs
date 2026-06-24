use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pdarrays::{PDArrayIndex, PDIntArray};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::simpletypes::Type;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_parse_operator;
use std::collections::BTreeMap;

const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FunWeightInitKind {
    ExplicitSymbols,
    ConjectureSymbols,
    ConjectureSymbolTypes,
    ConjectureTypeBased,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunWeightParam {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    weight_stack: Vec<(String, i64)>,
    axioms: Option<ClauseSet>,
    init_kind: FunWeightInitKind,
    flimit: FunCode,
    fweights: Option<Vec<i64>>,
    type_freqs: Option<BTreeMap<i64, i64>>,
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
            cweight: fweight,
            pweight: fweight,
            conj_fweight: fweight,
            conj_cweight: fweight,
            conj_pweight: fweight,
            weight_stack,
            axioms: None,
            init_kind: FunWeightInitKind::ExplicitSymbols,
            flimit: 0,
            fweights: None,
            type_freqs: None,
            f_occur: with_occurrences.then(|| PDIntArray::new_int(8, 0)),
        }
    }

    #[must_use]
    #[allow(clippy::similar_names)]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors ConjectureSymbolWeightInit without OCB"
    )]
    pub fn with_conjecture_symbols(
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        cweight: i64,
        pweight: i64,
        conj_fweight: i64,
        conj_cweight: i64,
        conj_pweight: i64,
        axioms: &ClauseSet,
        app_var_mult: f64,
    ) -> Self {
        Self {
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
            vweight,
            fweight,
            cweight,
            pweight,
            conj_fweight,
            conj_cweight,
            conj_pweight,
            weight_stack: Vec::new(),
            axioms: Some(axioms.clone()),
            init_kind: FunWeightInitKind::ConjectureSymbols,
            flimit: 0,
            fweights: None,
            type_freqs: None,
            f_occur: None,
        }
    }

    #[must_use]
    #[allow(clippy::similar_names)]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors ConjectureSymbolWeightInit without OCB"
    )]
    pub fn with_conjecture_symbol_types(
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        fweight: i64,
        cweight: i64,
        pweight: i64,
        conj_fweight: i64,
        conj_cweight: i64,
        conj_pweight: i64,
        axioms: &ClauseSet,
        app_var_mult: f64,
    ) -> Self {
        Self {
            axioms: Some(axioms.clone()),
            init_kind: FunWeightInitKind::ConjectureSymbolTypes,
            ..Self::with_conjecture_symbols(
                max_term_multiplier,
                max_literal_multiplier,
                pos_multiplier,
                vweight,
                fweight,
                cweight,
                pweight,
                conj_fweight,
                conj_cweight,
                conj_pweight,
                axioms,
                app_var_mult,
            )
        }
    }

    #[must_use]
    pub fn with_conjecture_type_based(
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        vweight: i64,
        axioms: &ClauseSet,
        app_var_mult: f64,
    ) -> Self {
        Self {
            axioms: Some(axioms.clone()),
            init_kind: FunWeightInitKind::ConjectureTypeBased,
            ..Self::with_conjecture_symbols(
                max_term_multiplier,
                max_literal_multiplier,
                pos_multiplier,
                vweight,
                0,
                0,
                0,
                0,
                0,
                0,
                axioms,
                app_var_mult,
            )
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
    pub const fn cweight(&self) -> i64 {
        self.cweight
    }

    #[must_use]
    pub const fn pweight(&self) -> i64 {
        self.pweight
    }

    #[must_use]
    pub const fn conj_fweight(&self) -> i64 {
        self.conj_fweight
    }

    #[must_use]
    pub const fn conj_cweight(&self) -> i64 {
        self.conj_cweight
    }

    #[must_use]
    pub const fn conj_pweight(&self) -> i64 {
        self.conj_pweight
    }

    #[must_use]
    pub const fn axioms(&self) -> Option<&ClauseSet> {
        self.axioms.as_ref()
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

    #[must_use]
    pub fn type_freqs(&self) -> Option<&BTreeMap<i64, i64>> {
        self.type_freqs.as_ref()
    }

    fn ensure_fun_weights(&mut self, bank: &TermBank) {
        if self.fweights.is_some() {
            return;
        }

        match self.init_kind {
            FunWeightInitKind::ExplicitSymbols => self.init_explicit_fun_weights(bank.signature()),
            FunWeightInitKind::ConjectureSymbols => {
                self.init_conjecture_symbol_weights(bank.signature());
            }
            FunWeightInitKind::ConjectureSymbolTypes => {
                self.init_conjecture_symbol_type_weights(bank.signature());
            }
            FunWeightInitKind::ConjectureTypeBased => {
                self.init_conjecture_type_based_weights(bank.signature());
            }
        }
    }

    fn init_explicit_fun_weights(&mut self, signature: &Signature) {
        self.flimit = signature.f_count() + 1;
        let len = usize::try_from(self.flimit)
            .unwrap_or_else(|_| panic!("signature f-count must fit vector length"));
        let mut fweights = vec![0; len];
        for weight in fweights.iter_mut().skip(1) {
            *weight = self.fweight;
        }

        for (name, weight) in &self.weight_stack {
            let f_code = signature.find_f_code(name);
            if f_code != 0 && f_code < self.flimit {
                let index = usize::try_from(f_code)
                    .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
                fweights[index] = *weight;
            }
        }

        self.fweights = Some(fweights);
    }

    fn init_conjecture_symbol_type_weights(&mut self, signature: &Signature) {
        self.flimit = signature.f_count() + 1;
        let len = usize::try_from(self.flimit)
            .unwrap_or_else(|_| panic!("signature f-count must fit vector length"));
        let mut fweights = vec![0; len];
        let mut type_signature = signature.clone();
        let mut type_freqs = vec![0; type_freq_len(&type_signature)];
        self.add_neg_conjecture_type_distribution(&mut type_signature, &mut type_freqs);

        for f_code in 1..self.flimit {
            let index = usize::try_from(f_code)
                .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
            let type_uid = type_uid_for_f_code(&type_signature, f_code);
            fweights[index] = if type_freq_at(&type_freqs, type_uid) == 0 {
                typed_symbol_weight(signature, f_code, self.fweight, self.cweight, self.pweight)
            } else {
                typed_symbol_weight(
                    signature,
                    f_code,
                    self.conj_fweight,
                    self.conj_cweight,
                    self.conj_pweight,
                )
            };
        }

        self.type_freqs = Some(type_freq_map(&type_freqs, |freq| {
            if freq > 0 {
                self.vweight
            } else {
                2 * self.vweight
            }
        }));
        self.fweights = Some(fweights);
    }

    fn init_conjecture_type_based_weights(&mut self, signature: &Signature) {
        self.flimit = signature.f_count() + 1;
        let len = usize::try_from(self.flimit)
            .unwrap_or_else(|_| panic!("signature f-count must fit vector length"));
        let mut fweights = vec![0; len];
        let mut type_signature = signature.clone();
        let mut type_freqs = vec![0; type_freq_len(&type_signature)];

        let axioms = self
            .axioms
            .as_ref()
            .unwrap_or_else(|| panic!("ConjectureTypeBasedWeight requires proof-state axioms"));
        for clause in axioms.iter() {
            if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
                clause.add_type_distribution(&mut type_signature, &mut type_freqs);
                clause.add_symbol_distribution(&mut fweights);
            }
        }

        let mut max_occurrence = 0;
        for f_code in 1..self.flimit {
            let index = usize::try_from(f_code)
                .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
            let type_uid = type_uid_for_f_code(&type_signature, f_code);
            max_occurrence =
                max_occurrence.max(type_freq_at(&type_freqs, type_uid) + (2 * fweights[index]));
        }
        max_occurrence += 1;

        for f_code in 1..self.flimit {
            let index = usize::try_from(f_code)
                .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
            let type_uid = type_uid_for_f_code(&type_signature, f_code);
            let type_freq = type_freq_at(&type_freqs, type_uid);
            fweights[index] = if type_freq == 0 {
                5 * max_occurrence
            } else {
                max_occurrence - (type_freq + (2 * fweights[index]))
            };
        }

        self.type_freqs = Some(type_freq_map(&type_freqs, |freq| max_occurrence - freq));
        self.fweights = Some(fweights);
    }

    fn add_neg_conjecture_type_distribution(
        &self,
        signature: &mut Signature,
        type_freqs: &mut [i64],
    ) {
        let axioms = self
            .axioms
            .as_ref()
            .unwrap_or_else(|| panic!("ConjectureSymbolWeight requires proof-state axioms"));
        for clause in axioms.iter() {
            if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
                clause.add_type_distribution(signature, type_freqs);
            }
        }
    }

    fn init_conjecture_symbol_weights(&mut self, signature: &Signature) {
        self.flimit = signature.f_count() + 1;
        let len = usize::try_from(self.flimit)
            .unwrap_or_else(|_| panic!("signature f-count must fit vector length"));
        let mut fweights = vec![0; len];

        let axioms = self
            .axioms
            .as_ref()
            .unwrap_or_else(|| panic!("ConjectureSymbolWeight requires proof-state axioms"));
        for clause in axioms.iter() {
            if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
                clause.add_symbol_distribution(&mut fweights);
            }
        }

        for f_code in 1..self.flimit {
            let index = usize::try_from(f_code)
                .unwrap_or_else(|_| panic!("positive f-code must fit vector index"));
            fweights[index] = if fweights[index] == 0 {
                typed_symbol_weight(signature, f_code, self.fweight, self.cweight, self.pweight)
            } else {
                typed_symbol_weight(
                    signature,
                    f_code,
                    self.conj_fweight,
                    self.conj_cweight,
                    self.conj_pweight,
                )
            };
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
#[allow(clippy::similar_names)]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureSymbolWeightInit without prio/OCB"
)]
pub fn conjecture_symbol_weight_init(
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    axioms: &ClauseSet,
    app_var_mult: f64,
) -> FunWeightParam {
    FunWeightParam::with_conjecture_symbols(
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        cweight,
        pweight,
        conj_fweight,
        conj_cweight,
        conj_pweight,
        axioms,
        app_var_mult,
    )
}

#[must_use]
#[allow(clippy::similar_names)]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureSymbolWeightInit without prio/OCB"
)]
pub fn conjecture_symbol_type_weight_init(
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    axioms: &ClauseSet,
    app_var_mult: f64,
) -> FunWeightParam {
    FunWeightParam::with_conjecture_symbol_types(
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        cweight,
        pweight,
        conj_fweight,
        conj_cweight,
        conj_pweight,
        axioms,
        app_var_mult,
    )
}

#[must_use]
pub fn conjecture_type_based_weight_init(
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    axioms: &ClauseSet,
    app_var_mult: f64,
) -> FunWeightParam {
    FunWeightParam::with_conjecture_type_based(
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        axioms,
        app_var_mult,
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

#[must_use]
#[allow(clippy::similar_names)]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureSymbolWeightInit parameters without OCB"
)]
pub fn conjecture_symbol_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    app_var_mult: f64,
) -> Wfcb<FunWeightParam> {
    wfcb_alloc(
        generic_fun_weight_wfcb_compute,
        prio_fun,
        fun_weight_exit,
        Some(conjecture_symbol_weight_init(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            fweight,
            cweight,
            pweight,
            conj_fweight,
            conj_cweight,
            conj_pweight,
            axioms,
            app_var_mult,
        )),
    )
}

#[must_use]
#[allow(clippy::similar_names)]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureSymbolWeightInit parameters without OCB"
)]
pub fn conjecture_symbol_type_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    app_var_mult: f64,
) -> Wfcb<FunWeightParam> {
    wfcb_alloc(
        generic_fun_weight_wfcb_compute,
        prio_fun,
        fun_weight_exit,
        Some(conjecture_symbol_type_weight_init(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            fweight,
            cweight,
            pweight,
            conj_fweight,
            conj_cweight,
            conj_pweight,
            axioms,
            app_var_mult,
        )),
    )
}

#[must_use]
pub fn conjecture_type_based_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vweight: i64,
    app_var_mult: f64,
) -> Wfcb<FunWeightParam> {
    wfcb_alloc(
        generic_fun_weight_wfcb_compute,
        prio_fun,
        fun_weight_exit,
        Some(conjecture_type_based_weight_init(
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vweight,
            axioms,
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

#[allow(clippy::similar_names)]
pub fn conjecture_symbol_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let cweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_cweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_optional_app_var_mult(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_symbol_weight_wfcb_init(
        prio_fun,
        axioms,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        cweight,
        pweight,
        conj_fweight,
        conj_cweight,
        conj_pweight,
        app_var_mult,
    ))
}

#[allow(clippy::similar_names)]
pub fn conjecture_simplified_symbol_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_optional_app_var_mult(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_symbol_weight_wfcb_init(
        prio_fun,
        axioms,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        fweight,
        pweight,
        conj_fweight,
        conj_fweight,
        conj_pweight,
        app_var_mult,
    ))
}

pub fn conjecture_relative_symbol_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let cweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_optional_app_var_mult(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_symbol_weight_wfcb_init(
        prio_fun,
        axioms,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        cweight,
        pweight,
        f64_to_i64(conj_multiplier * i64_to_f64(fweight)),
        f64_to_i64(conj_multiplier * i64_to_f64(cweight)),
        f64_to_i64(conj_multiplier * i64_to_f64(pweight)),
        app_var_mult,
    ))
}

pub fn conjecture_relative_symbol_type_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conj_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let cweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_optional_app_var_mult(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_symbol_type_weight_wfcb_init(
        prio_fun,
        axioms,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        fweight,
        cweight,
        pweight,
        f64_to_i64(conj_multiplier * i64_to_f64(fweight)),
        f64_to_i64(conj_multiplier * i64_to_f64(cweight)),
        f64_to_i64(conj_multiplier * i64_to_f64(pweight)),
        app_var_mult,
    ))
}

pub fn conjecture_type_based_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<FunWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_optional_app_var_mult(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_type_based_weight_wfcb_init(
        prio_fun,
        axioms,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        vweight,
        app_var_mult,
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
        param.type_freqs.as_ref(),
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

fn parse_optional_app_var_mult(scanner: &mut Scanner) -> Result<f64, Diagnostic> {
    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }
    Ok(app_var_mult)
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

fn typed_symbol_weight(
    signature: &Signature,
    f_code: FunCode,
    fweight: i64,
    cweight: i64,
    pweight: i64,
) -> i64 {
    if signature.is_predicate(f_code) {
        pweight
    } else if signature.find_arity(f_code).unwrap_or(0) != 0 {
        fweight
    } else {
        cweight
    }
}

fn type_freq_len(signature: &Signature) -> usize {
    usize::try_from(signature.type_bank().types_count() + 1)
        .unwrap_or_else(|_| panic!("type count must fit vector length"))
}

fn type_uid_for_f_code(signature: &Signature, f_code: FunCode) -> i64 {
    signature.get_type(f_code).map_or(0, Type::type_uid)
}

fn type_freq_at(type_freqs: &[i64], type_uid: i64) -> i64 {
    let index = usize::try_from(type_uid)
        .unwrap_or_else(|_| panic!("type UID must fit frequency vector index"));
    type_freqs
        .get(index)
        .copied()
        .unwrap_or_else(|| panic!("type frequency vector must cover type UID {type_uid}"))
}

fn type_freq_map(type_freqs: &[i64], convert: impl Fn(i64) -> i64) -> BTreeMap<i64, i64> {
    type_freqs
        .iter()
        .enumerate()
        .map(|(type_uid, freq)| {
            (
                i64::try_from(type_uid).expect("type UID index fits i64"),
                convert(*freq),
            )
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn f64_to_i64(value: f64) -> i64 {
    value as i64
}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_relative_symbol_type_weight_parse, conjecture_relative_symbol_weight_parse,
        conjecture_simplified_symbol_weight_parse, conjecture_symbol_weight_parse,
        conjecture_type_based_weight_parse, fun_weight_compute, fun_weight_init, fun_weight_parse,
        sym_offset_weight_compute, sym_offset_weight_init, sym_offset_weight_parse,
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
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone()]));
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

    fn negated_conjecture_axioms(bank: &mut TermBank) -> ClauseSet {
        let a = typed_const(bank, "a");
        let f_of_a = typed_unary(bank, "f", &a);
        let literal = Eqn::alloc(f_of_a, a, bank, false).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        ClauseSet::from_clauses([clause])
    }

    fn symbol_type_uid(bank: &TermBank, name: &str) -> i64 {
        let f_code = bank.signature().find_f_code(name);
        bank.signature()
            .get_type(f_code)
            .expect("test symbol should have a declared type")
            .type_uid()
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
    fn conjecture_simplified_symbol_weight_parse_marks_negated_conjecture_symbols() {
        let mut bank = test_bank();
        let axioms = negated_conjecture_axioms(&mut bank);
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,10,99,1,88,1,1.0,1.0,1.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = conjecture_simplified_symbol_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 22.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
        let data = wfcb.data().expect("parser should install funweight data");
        assert_eq!(data.fweight(), 10);
        assert_eq!(data.cweight(), 10);
        assert_eq!(data.pweight(), 99);
        assert_eq!(data.conj_fweight(), 1);
        assert_eq!(data.conj_cweight(), 1);
        assert_eq!(data.conj_pweight(), 88);
        assert_eq!(data.axioms().map(ClauseSet::len), Some(1));
        assert_eq!(data.flimit(), bank.signature().f_count() + 1);
    }

    #[test]
    fn conjecture_general_symbol_weight_parse_keeps_constant_weight_distinct() {
        let mut bank = test_bank();
        let axioms = negated_conjecture_axioms(&mut bank);
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,10,3,99,1,2,88,1,1.0,1.0,1.0,2.5) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb =
            conjecture_symbol_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_close(wfcb.compute_eval(&bank, &clause), 16.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
        let data = wfcb.data().expect("parser should install funweight data");
        assert_close(data.app_var_mult(), 2.5);
        assert_eq!(data.cweight(), 3);
        assert_eq!(data.conj_cweight(), 2);
    }

    #[test]
    fn conjecture_relative_symbol_weight_parse_truncates_scaled_weights() {
        let mut bank = test_bank();
        let axioms = negated_conjecture_axioms(&mut bank);
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0.25,10,4,99,1,1.0,1.0,1.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = conjecture_relative_symbol_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        let data = wfcb.data().expect("parser should install funweight data");
        assert_eq!(data.conj_fweight(), 2);
        assert_eq!(data.conj_cweight(), 1);
        assert_eq!(data.conj_pweight(), 24);
        assert_close(wfcb.compute_eval(&bank, &clause), 17.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn conjecture_relative_symbol_type_weight_parse_marks_symbols_by_conjecture_type() {
        let mut bank = test_bank();
        let axioms = negated_conjecture_axioms(&mut bank);
        let clause = test_clause(&mut bank);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0.25,10,4,99,1,1.0,1.0,1.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = conjecture_relative_symbol_type_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 6.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
        let data = wfcb.data().expect("parser should install funweight data");
        assert_eq!(data.conj_fweight(), 2);
        assert_eq!(data.conj_cweight(), 1);
        assert_eq!(data.conj_pweight(), 24);
        assert_eq!(
            data.type_freqs()
                .and_then(|freqs| freqs.get(&symbol_type_uid(&bank, "a")).copied()),
            Some(1)
        );
        assert_eq!(
            data.type_freqs()
                .and_then(|freqs| freqs.get(&symbol_type_uid(&bank, "f")).copied()),
            Some(1)
        );
    }

    #[test]
    fn conjecture_type_based_weight_parse_scores_inverse_type_and_symbol_frequency() {
        let mut bank = test_bank();
        let axioms = negated_conjecture_axioms(&mut bank);
        let clause = test_clause(&mut bank);
        let mut scanner = Scanner::from_user_string("(ConstPrio,3,1.0,1.0,1.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb =
            conjecture_type_based_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_close(wfcb.compute_eval(&bank, &clause), 16.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
        let data = wfcb.data().expect("parser should install funweight data");
        assert_eq!(
            data.type_freqs()
                .and_then(|freqs| freqs.get(&symbol_type_uid(&bank, "a")).copied()),
            Some(5)
        );
        assert_eq!(
            data.type_freqs()
                .and_then(|freqs| freqs.get(&symbol_type_uid(&bank, "f")).copied()),
            Some(6)
        );
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
