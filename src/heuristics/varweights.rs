use std::collections::BTreeSet;

use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS};
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_depth, term_weight_compute};
use crate::terms::termtypes::Term;

const APP_VAR_MULT_DEFAULT: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
struct RefinedWeightParsePrefix {
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VarWeightParam {
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    conjecture_multiplier: f64,
    hypothesis_multiplier: f64,
    sig_size_multiplier: f64,
    proof_size_multiplier: f64,
    proof_depth_multiplier: f64,
    term_weight_multiplier: f64,
    term_depth_multiplier: f64,
    weight_multiplier: f64,
    app_var_mult: f64,
    vlweight: i64,
    vweight: i64,
    fweight: i64,
    nvweight: i64,
    nfweight: i64,
    cweight: i64,
    pweight: i64,
    stagger_limit: i64,
}

impl VarWeightParam {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            max_term_multiplier: 0.0,
            max_literal_multiplier: 0.0,
            pos_multiplier: 0.0,
            conjecture_multiplier: 0.0,
            hypothesis_multiplier: 0.0,
            sig_size_multiplier: 0.0,
            proof_size_multiplier: 0.0,
            proof_depth_multiplier: 0.0,
            term_weight_multiplier: 0.0,
            term_depth_multiplier: 0.0,
            weight_multiplier: 0.0,
            app_var_mult: 0.0,
            vlweight: 0,
            vweight: 0,
            fweight: 0,
            nvweight: 0,
            nfweight: 0,
            cweight: 0,
            pweight: 0,
            stagger_limit: 0,
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
    pub const fn conjecture_multiplier(self) -> f64 {
        self.conjecture_multiplier
    }

    #[must_use]
    pub const fn hypothesis_multiplier(self) -> f64 {
        self.hypothesis_multiplier
    }

    #[must_use]
    pub const fn sig_size_multiplier(self) -> f64 {
        self.sig_size_multiplier
    }

    #[must_use]
    pub const fn proof_size_multiplier(self) -> f64 {
        self.proof_size_multiplier
    }

    #[must_use]
    pub const fn proof_depth_multiplier(self) -> f64 {
        self.proof_depth_multiplier
    }

    #[must_use]
    pub const fn term_weight_multiplier(self) -> f64 {
        self.term_weight_multiplier
    }

    #[must_use]
    pub const fn term_depth_multiplier(self) -> f64 {
        self.term_depth_multiplier
    }

    #[must_use]
    pub const fn weight_multiplier(self) -> f64 {
        self.weight_multiplier
    }

    #[must_use]
    pub const fn app_var_mult(self) -> f64 {
        self.app_var_mult
    }

    #[must_use]
    pub const fn vlweight(self) -> i64 {
        self.vlweight
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
    pub const fn cweight(self) -> i64 {
        self.cweight
    }

    #[must_use]
    pub const fn pweight(self) -> i64 {
        self.pweight
    }

    #[must_use]
    pub const fn stagger_limit(self) -> i64 {
        self.stagger_limit
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors TPTPTypeWeightInit parameters without prio/OCB"
)]
pub const fn tptp_type_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    conjecture_multiplier: f64,
    hypothesis_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.conjecture_multiplier = conjecture_multiplier;
    param.hypothesis_multiplier = hypothesis_multiplier;
    param
}

#[must_use]
pub fn tptp_type_weight_compute(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    let mut result = base_clause_weight(param, bank, clause);
    match clause.query_tptp_type() {
        CP_TYPE_HYPOTHESIS => result *= param.hypothesis_multiplier,
        CP_TYPE_CONJECTURE => result *= param.conjecture_multiplier,
        _ => {}
    }
    result
}

/// Computes C `TPTPTypeWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn tptp_type_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    tptp_type_weight_compute(param, bank, clause)
}

/// Computes C `TPTPTypeWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn tptp_type_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(tptp_type_weight_compute(param, bank, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors TPTPTypeWeightInit parameters without OCB"
)]
pub fn tptp_type_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    conjecture_multiplier: f64,
    hypothesis_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        tptp_type_weight_wfcb_compute,
        tptp_type_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(tptp_type_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            conjecture_multiplier,
            hypothesis_multiplier,
            app_var_mult,
        )),
    )
}

pub fn tptp_type_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    let prefix = parse_refined_weight_prefix(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let conjecture_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let hypothesis_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(tptp_type_weight_wfcb_init(
        prefix.prio_fun,
        prefix.fweight,
        prefix.vweight,
        prefix.max_term_multiplier,
        prefix.max_literal_multiplier,
        prefix.pos_multiplier,
        conjecture_multiplier,
        hypothesis_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub const fn sig_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    sig_size_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.sig_size_multiplier = sig_size_multiplier;
    param
}

#[must_use]
pub fn sig_weight_compute(
    param: &VarWeightParam,
    bank: &TermBank,
    signature: &Signature,
    clause: &Clause,
) -> f64 {
    let mut result = base_clause_weight(param, bank, clause);
    let sig_size = clause_count_ext_symbols(clause, signature, 0);
    let external_symbols = signature.external_symbols().max(1);
    let modify = i64_to_f64(sig_size) * param.sig_size_multiplier / i64_to_f64(external_symbols);
    result *= 1.0 + modify;
    result
}

/// Computes C `SigWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn sig_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    sig_weight_compute(param, bank, bank.signature(), clause)
}

/// Computes C `SigWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn sig_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(sig_weight_compute(param, bank, bank.signature(), clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors SigWeightInit parameters without OCB"
)]
pub fn sig_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    sig_size_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        sig_weight_wfcb_compute,
        sig_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(sig_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            sig_size_multiplier,
            app_var_mult,
        )),
    )
}

pub fn sig_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    let prefix = parse_refined_weight_prefix(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let sig_size_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(sig_weight_wfcb_init(
        prefix.prio_fun,
        prefix.fweight,
        prefix.vweight,
        prefix.max_term_multiplier,
        prefix.max_literal_multiplier,
        prefix.pos_multiplier,
        sig_size_multiplier,
        app_var_mult,
    ))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ProofWeightInit parameters without prio/OCB"
)]
pub const fn proof_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    proof_size_multiplier: f64,
    proof_depth_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.proof_size_multiplier = proof_size_multiplier;
    param.proof_depth_multiplier = proof_depth_multiplier;
    param
}

#[must_use]
pub fn proof_weight_compute(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    let mut result = base_clause_weight(param, bank, clause);
    result *= 1.0
        + param.proof_depth_multiplier * (1.0 / i64_to_f64(clause.proof_depth().saturating_add(1)));
    result *= 1.0
        + param.proof_size_multiplier * (1.0 / i64_to_f64(clause.proof_size().saturating_add(1)));
    result
}

/// Computes C `ProofWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn proof_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    proof_weight_compute(param, bank, clause)
}

/// Computes C `ProofWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn proof_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(proof_weight_compute(param, bank, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ProofWeightInit parameters without OCB"
)]
pub fn proof_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    proof_size_multiplier: f64,
    proof_depth_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        proof_weight_wfcb_compute,
        proof_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(proof_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            proof_size_multiplier,
            proof_depth_multiplier,
            app_var_mult,
        )),
    )
}

pub fn proof_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    let prefix = parse_refined_weight_prefix(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let proof_size_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let proof_depth_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(proof_weight_wfcb_init(
        prefix.prio_fun,
        prefix.fweight,
        prefix.vweight,
        prefix.max_term_multiplier,
        prefix.max_literal_multiplier,
        prefix.pos_multiplier,
        proof_size_multiplier,
        proof_depth_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub const fn depth_weight_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    term_weight_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.term_weight_multiplier = term_weight_multiplier;
    param
}

#[must_use]
pub fn depth_weight_compute(param: &VarWeightParam, clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            let mut left = i64_to_f64(term_depth(literal.left()))
                + param.term_weight_multiplier
                    * i64_to_f64(term_weight_compute(
                        literal.left(),
                        param.vweight,
                        param.fweight,
                    ));
            left *=
                param.max_term_multiplier * applied_var_factor(literal.left(), param.app_var_mult);

            let mut right = i64_to_f64(term_depth(literal.right()))
                + param.term_weight_multiplier
                    * i64_to_f64(term_weight_compute(
                        literal.right(),
                        param.vweight,
                        param.fweight,
                    ))
                    * applied_var_factor(literal.right(), param.app_var_mult);
            if !literal.is_oriented() {
                right *= param.max_term_multiplier;
            }

            let mut weight = left + right;
            if literal.is_positive() {
                weight *= param.pos_multiplier;
            }
            if literal.is_maximal() {
                weight *= param.max_literal_multiplier;
            }
            weight
        })
        .sum()
}

/// Computes C `DepthWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn depth_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    depth_weight_compute(param, clause)
}

/// Computes C `DepthWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn depth_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(depth_weight_compute(param, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors DepthWeightInit parameters without OCB"
)]
pub fn depth_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    term_weight_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        depth_weight_wfcb_compute,
        depth_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(depth_weight_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            term_weight_multiplier,
            app_var_mult,
        )),
    )
}

pub fn depth_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    let prefix = parse_refined_weight_prefix(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let term_weight_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(depth_weight_wfcb_init(
        prefix.prio_fun,
        prefix.fweight,
        prefix.vweight,
        prefix.max_term_multiplier,
        prefix.max_literal_multiplier,
        prefix.pos_multiplier,
        term_weight_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub const fn weight_less_depth_init(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    term_depth_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.term_depth_multiplier = term_depth_multiplier;
    param
}

#[must_use]
pub fn weight_less_depth_compute(param: &VarWeightParam, clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            let mut left = i64_to_f64(term_weight_compute(
                literal.left(),
                param.vweight,
                param.fweight,
            )) - param.term_depth_multiplier
                * i64_to_f64(term_depth(literal.left()));
            left *=
                param.max_term_multiplier * applied_var_factor(literal.left(), param.app_var_mult);

            let mut right = i64_to_f64(term_weight_compute(
                literal.right(),
                param.vweight,
                param.fweight,
            )) - param.term_depth_multiplier
                * i64_to_f64(term_depth(literal.right()));
            if !literal.is_oriented() {
                right *= param.max_term_multiplier;
            }

            let mut weight = left + right * applied_var_factor(literal.right(), param.app_var_mult);
            if literal.is_positive() {
                weight *= param.pos_multiplier;
            }
            if literal.is_maximal() {
                weight *= param.max_literal_multiplier;
            }
            weight
        })
        .sum()
}

/// Computes C `WeightLessDepthCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn weight_less_depth_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    weight_less_depth_compute(param, clause)
}

/// Computes C `WeightLessDepthCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn weight_less_depth_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(weight_less_depth_compute(param, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors WeightLessDepthInit parameters without OCB"
)]
pub fn weight_less_depth_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    term_depth_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        weight_less_depth_wfcb_compute,
        weight_less_depth_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(weight_less_depth_init(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            term_depth_multiplier,
            app_var_mult,
        )),
    )
}

pub fn weight_less_depth_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    let prefix = parse_refined_weight_prefix(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let term_depth_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(weight_less_depth_wfcb_init(
        prefix.prio_fun,
        prefix.fweight,
        prefix.vweight,
        prefix.max_term_multiplier,
        prefix.max_literal_multiplier,
        prefix.pos_multiplier,
        term_depth_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub const fn nl_weight_init(
    fweight: i64,
    linear_var_weight: i64,
    repeat_var_weight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        repeat_var_weight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.vlweight = linear_var_weight;
    param
}

#[must_use]
pub fn nl_weight_compute(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    clause.non_linear_weight(
        bank,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vlweight,
        param.vweight,
        param.fweight,
        param.app_var_mult,
        false,
    )
}

/// Computes C `NLWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn nl_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    nl_weight_compute(param, bank, clause)
}

/// Computes C `NLWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn nl_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(nl_weight_compute(param, bank, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors NLWeightInit parameters without OCB"
)]
pub fn nl_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    linear_var_weight: i64,
    repeat_var_weight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        nl_weight_wfcb_compute,
        nl_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(nl_weight_init(
            fweight,
            linear_var_weight,
            repeat_var_weight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

pub fn nl_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let linear_var_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let repeat_var_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(nl_weight_wfcb_init(
        prio_fun,
        fweight,
        linear_var_weight,
        repeat_var_weight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    ))
}

#[must_use]
#[allow(
    clippy::similar_names,
    reason = "C-compatible PNRefinedWeight names keep positive and negative weights comparable"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors PNRefinedWeightInit parameters without prio/OCB"
)]
pub const fn pn_refined_weight_init(
    fweight: i64,
    vweight: i64,
    negative_fweight: i64,
    negative_vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.nfweight = negative_fweight;
    param.nvweight = negative_vweight;
    param
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    clippy::similar_names,
    reason = "C-compatible helper mirrors PNRefinedWeightInit parameters without OCB"
)]
pub fn pn_refined_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    negative_fweight: i64,
    negative_vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        pn_refined_weight_wfcb_compute,
        pn_refined_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(pn_refined_weight_init(
            fweight,
            vweight,
            negative_fweight,
            negative_vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

#[expect(
    clippy::similar_names,
    reason = "C-compatible parser keeps positive and negative weight names comparable"
)]
pub fn pn_refined_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let nfweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let nvweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(pn_refined_weight_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        nfweight,
        nvweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub fn pn_refined_weight_compute(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| {
            if literal.is_positive() {
                literal.literal_weight(
                    bank,
                    param.max_term_multiplier,
                    param.max_literal_multiplier,
                    param.pos_multiplier,
                    param.vweight,
                    param.fweight,
                    param.app_var_mult,
                    false,
                )
            } else {
                literal.literal_weight(
                    bank,
                    param.max_term_multiplier,
                    param.max_literal_multiplier,
                    param.pos_multiplier,
                    param.nvweight,
                    param.nfweight,
                    param.app_var_mult,
                    false,
                )
            }
        })
        .sum()
}

/// Computes C `PNRefinedWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn pn_refined_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    pn_refined_weight_compute(param, bank, clause)
}

/// Computes C `PNRefinedWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn pn_refined_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(pn_refined_weight_compute(param, bank, clause))
}

fn pn_refined_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    pn_refined_weight_compute(var_weight_data(data, "PNRefinedweight"), bank, clause)
}

fn pn_refined_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    pn_refined_weight_compute_with_bank(var_weight_data(data, "PNRefinedweight"), ocb, bank, clause)
}

fn tptp_type_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    tptp_type_weight_compute(var_weight_data(data, "TPTPTypeweight"), bank, clause)
}

fn tptp_type_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    tptp_type_weight_compute_with_bank(var_weight_data(data, "TPTPTypeweight"), ocb, bank, clause)
}

fn sig_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    sig_weight_compute(
        var_weight_data(data, "Sigweight"),
        bank,
        bank.signature(),
        clause,
    )
}

fn sig_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    sig_weight_compute_with_bank(var_weight_data(data, "Sigweight"), ocb, bank, clause)
}

fn proof_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    proof_weight_compute(var_weight_data(data, "Proofweight"), bank, clause)
}

fn proof_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    proof_weight_compute_with_bank(var_weight_data(data, "Proofweight"), ocb, bank, clause)
}

fn depth_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    depth_weight_compute(var_weight_data(data, "Depthweight"), clause)
}

fn depth_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    depth_weight_compute_with_bank(var_weight_data(data, "Depthweight"), ocb, bank, clause)
}

fn weight_less_depth_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    weight_less_depth_compute(var_weight_data(data, "WLessDWeight"), clause)
}

fn weight_less_depth_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    weight_less_depth_compute_with_bank(var_weight_data(data, "WLessDWeight"), ocb, bank, clause)
}

fn nl_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    nl_weight_compute(var_weight_data(data, "NLweight"), bank, clause)
}

fn nl_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    nl_weight_compute_with_bank(var_weight_data(data, "NLweight"), ocb, bank, clause)
}

fn sym_type_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    sym_type_weight_compute(var_weight_data(data, "SymbolTypeweight"), clause)
}

fn sym_type_weight_wfcb_compute_with_bank(
    data: Option<&mut VarWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    sym_type_weight_compute_with_bank(var_weight_data(data, "SymbolTypeweight"), ocb, bank, clause)
}

fn clause_weight_age_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    clause_weight_age_compute(var_weight_data(data, "ClauseWeightAge"), bank, clause)
}

fn var_weight_data<'a>(data: Option<&'a mut VarWeightParam>, name: &str) -> &'a mut VarWeightParam {
    data.unwrap_or_else(|| panic!("{name} WFCB requires initialized weight parameters"))
}

fn var_weight_exit(_data: VarWeightParam) {}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors SymTypeWeightInit parameters without prio/OCB"
)]
pub const fn sym_type_weight_init(
    fweight: i64,
    vweight: i64,
    cweight: i64,
    pweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = base_refined_param(
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    );
    param.cweight = cweight;
    param.pweight = pweight;
    param
}

#[must_use]
pub fn sym_type_weight_compute(param: &VarWeightParam, clause: &Clause) -> f64 {
    clause.sym_type_weight(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        param.cweight,
        param.pweight,
        param.app_var_mult,
    )
}

/// Computes C `SymTypeWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn sym_type_weight_compute_with_ocb(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    sym_type_weight_compute(param, clause)
}

/// Computes C `SymTypeWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn sym_type_weight_compute_with_bank(
    param: &VarWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(sym_type_weight_compute(param, clause))
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors SymTypeWeightInit parameters without OCB"
)]
pub fn sym_type_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    cweight: i64,
    pweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc_with_bank(
        sym_type_weight_wfcb_compute,
        sym_type_weight_wfcb_compute_with_bank,
        prio_fun,
        var_weight_exit,
        Some(sym_type_weight_init(
            fweight,
            vweight,
            cweight,
            pweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            app_var_mult,
        )),
    )
}

pub fn sym_type_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let cweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(sym_type_weight_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        cweight,
        pweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub const fn clause_weight_age_init(
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    weight_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = VarWeightParam::empty();
    param.fweight = fweight;
    param.vweight = vweight;
    param.pos_multiplier = pos_multiplier;
    param.weight_multiplier = weight_multiplier;
    param.app_var_mult = app_var_mult;
    param
}

#[must_use]
pub fn clause_weight_age_compute(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
    param.weight_multiplier
        * clause.literal_weight(
            bank,
            1.0,
            1.0,
            param.pos_multiplier,
            param.vweight,
            param.fweight,
            param.app_var_mult,
            false,
        )
        + i64_to_f64(clause.create_date())
}

#[must_use]
pub fn clause_weight_age_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    pos_multiplier: f64,
    weight_multiplier: f64,
    app_var_mult: f64,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc(
        clause_weight_age_wfcb_compute,
        prio_fun,
        var_weight_exit,
        Some(clause_weight_age_init(
            fweight,
            vweight,
            pos_multiplier,
            weight_multiplier,
            app_var_mult,
        )),
    )
}

pub fn clause_weight_age_parse(scanner: &mut Scanner) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let weight_multiplier = parse_float(scanner)?;
    let app_var_mult = parse_app_var_mult_and_close(scanner)?;

    Ok(clause_weight_age_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        pos_multiplier,
        weight_multiplier,
        app_var_mult,
    ))
}

#[must_use]
pub fn staggered_weight_init(stagger_factor: f64, axioms: &ClauseSet) -> VarWeightParam {
    let clause_max_size = axioms
        .find_max_standard_weight()
        .map_or(-1, Clause::standard_weight);
    let stagger_limit = f64_to_i64((stagger_factor * i64_to_f64(clause_max_size)).max(1.0));
    let mut param = VarWeightParam::empty();
    param.stagger_limit = stagger_limit;
    param
}

#[must_use]
pub fn staggered_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    stagger_factor: f64,
    axioms: &ClauseSet,
) -> Wfcb<VarWeightParam> {
    wfcb_alloc(
        staggered_weight_wfcb_compute,
        prio_fun,
        var_weight_exit,
        Some(staggered_weight_init(stagger_factor, axioms)),
    )
}

pub fn staggered_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<VarWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let stagger_factor = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(staggered_weight_wfcb_init(prio_fun, stagger_factor, axioms))
}

#[must_use]
pub fn staggered_weight_compute(param: &VarWeightParam, clause: &Clause) -> f64 {
    i64_to_f64(clause.standard_weight() / param.stagger_limit)
}

fn staggered_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    staggered_weight_compute(
        data.unwrap_or_else(|| panic!("StaggeredWeight WFCB requires initialized parameters")),
        clause,
    )
}

#[must_use]
pub fn clause_count_ext_symbols(clause: &Clause, signature: &Signature, min_arity: i64) -> i64 {
    let mut fcodes = BTreeSet::new();
    let _ = clause.collect_fcodes(&mut fcodes);
    usize_to_i64(
        fcodes
            .into_iter()
            .filter(|&f_code| is_counted_external_symbol(signature, f_code, min_arity))
            .count(),
    )
}

fn parse_refined_weight_prefix(
    scanner: &mut Scanner,
) -> Result<RefinedWeightParsePrefix, Diagnostic> {
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

    Ok(RefinedWeightParsePrefix {
        prio_fun,
        fweight,
        vweight,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    })
}

fn parse_app_var_mult_and_close(scanner: &mut Scanner) -> Result<f64, Diagnostic> {
    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    Ok(app_var_mult)
}

const fn base_refined_param(
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    app_var_mult: f64,
) -> VarWeightParam {
    let mut param = VarWeightParam::empty();
    param.fweight = fweight;
    param.vweight = vweight;
    param.max_term_multiplier = max_term_multiplier;
    param.max_literal_multiplier = max_literal_multiplier;
    param.pos_multiplier = pos_multiplier;
    param.app_var_mult = app_var_mult;
    param
}

fn base_clause_weight(param: &VarWeightParam, bank: &TermBank, clause: &Clause) -> f64 {
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

fn is_counted_external_symbol(signature: &Signature, f_code: FunCode, min_arity: i64) -> bool {
    f_code > signature.internal_symbols()
        && f_code <= signature.f_count()
        && signature
            .find_arity(f_code)
            .is_some_and(|arity| i64::from(arity) >= min_arity)
}

fn applied_var_factor(term: &Term, app_var_mult: f64) -> f64 {
    if term.is_applied_free_var() {
        app_var_mult
    } else {
        1.0
    }
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

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_count_ext_symbols, clause_weight_age_compute, clause_weight_age_init,
        clause_weight_age_parse, depth_weight_compute, depth_weight_compute_with_ocb,
        depth_weight_init, depth_weight_parse, nl_weight_compute, nl_weight_compute_with_ocb,
        nl_weight_init, nl_weight_parse, pn_refined_weight_compute,
        pn_refined_weight_compute_with_ocb, pn_refined_weight_init, pn_refined_weight_parse,
        proof_weight_compute, proof_weight_compute_with_ocb, proof_weight_init, proof_weight_parse,
        sig_weight_compute, sig_weight_compute_with_ocb, sig_weight_init, sig_weight_parse,
        staggered_weight_compute, staggered_weight_init, staggered_weight_parse,
        sym_type_weight_compute, sym_type_weight_compute_with_ocb, sym_type_weight_init,
        sym_type_weight_parse, tptp_type_weight_compute, tptp_type_weight_compute_with_ocb,
        tptp_type_weight_init, tptp_type_weight_parse, weight_less_depth_compute,
        weight_less_depth_compute_with_ocb, weight_less_depth_init, weight_less_depth_parse,
        VarWeightParam,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_IS_ORIENTED, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::to_params::TermOrdering;
    use crate::heuristics::wfcb::Wfcb;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE};
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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn applied_free_var(bank: &TermBank, variable: &Term, arg: &Term) -> Term {
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, variable.clone());
        term.set_argument(1, arg.clone());
        term
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        Clause::alloc(EqnList::from_vec(vec![literal(
            bank, left, right, positive,
        )]))
    }

    fn ordering_clause(bank: &mut TermBank, suffix: &str) -> Clause {
        let a_name = format!("a_{suffix}");
        let f_name = format!("f_{suffix}");
        let a = typed_const(bank, &a_name);
        let f_of_a = typed_unary(bank, &f_name, &a);
        unit_clause(bank, &a, &f_of_a, true)
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn assert_ocb_helper_matches_manual(
        bank: &TermBank,
        mut target: Clause,
        expected_compute: impl FnOnce(&Clause) -> f64,
        actual_compute: impl FnOnce(&mut OrderControlBlock, &mut Clause) -> f64,
    ) {
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, bank));
        let expected = expected_compute(&manually_marked);
        let mut ocb = kbo_ocb(bank);

        let actual = actual_compute(&mut ocb, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    fn assert_banked_wfcb_matches_manual(
        bank: &mut TermBank,
        mut target: Clause,
        wfcb: &mut Wfcb<VarWeightParam>,
        expected_compute: impl FnOnce(&Clause, &TermBank) -> f64,
    ) {
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(bank);
        assert!(manually_marked
            .cond_mark_maximal_terms_with_bank(&mut manual_ocb, bank)
            .unwrap_or_else(|err| panic!("{err}")));
        let expected = expected_compute(&manually_marked, bank);
        let mut ocb = kbo_ocb(bank);

        let actual = wfcb
            .compute_eval_with_bank(&mut ocb, bank, &mut target)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(actual, expected);
        assert_eq!(wfcb.compute_priority(bank, &target), PRIO_NORMAL);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn tptp_type_weight_scales_only_exact_hypothesis_and_conjecture_roles() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let param = tptp_type_weight_init(2, 1, 1.0, 1.0, 1.0, 7.0, 5.0, 1.0);

        let mut hypothesis = unit_clause(&mut bank, &a, &b, true);
        hypothesis.set_tptp_type(CP_TYPE_HYPOTHESIS);
        assert_close(tptp_type_weight_compute(&param, &bank, &hypothesis), 30.0);

        let mut conjecture = unit_clause(&mut bank, &a, &b, true);
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        assert_close(tptp_type_weight_compute(&param, &bank, &conjecture), 42.0);

        let mut negated = unit_clause(&mut bank, &a, &b, true);
        negated.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        assert_close(tptp_type_weight_compute(&param, &bank, &negated), 6.0);
        assert_close(param.conjecture_multiplier(), 7.0);
        assert_close(param.hypothesis_multiplier(), 5.0);
    }

    #[test]
    fn sig_weight_counts_distinct_external_symbols_and_normalizes_by_signature() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let clause = unit_clause(&mut bank, &a, &b, true);
        let param = sig_weight_init(2, 1, 1.0, 1.0, 1.0, 3.0, 1.0);

        assert_eq!(clause_count_ext_symbols(&clause, bank.signature(), 0), 2);
        assert_close(
            sig_weight_compute(&param, &bank, bank.signature(), &clause),
            24.0,
        );
        assert_close(param.sig_size_multiplier(), 3.0);
    }

    #[test]
    fn proof_weight_multiplies_depth_factor_before_size_factor() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut clause = unit_clause(&mut bank, &a, &b, true);
        clause.set_proof_depth(2);
        clause.set_proof_size(3);
        let param = proof_weight_init(2, 1, 1.0, 1.0, 1.0, 8.0, 6.0, 1.0);

        assert_close(proof_weight_compute(&param, &bank, &clause), 54.0);
        assert_close(param.proof_size_multiplier(), 8.0);
        assert_close(param.proof_depth_multiplier(), 6.0);
    }

    #[test]
    fn type_signature_and_proof_weight_parsers_wrap_existing_cores() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut type_clause = unit_clause(&mut bank, &a, &b, true);
        type_clause.set_tptp_type(CP_TYPE_CONJECTURE);
        let sig_clause = unit_clause(&mut bank, &a, &b, true);
        let mut proof_clause = unit_clause(&mut bank, &a, &b, true);
        proof_clause.set_proof_depth(2);
        proof_clause.set_proof_size(3);
        let mut type_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,7.0,5.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut sig_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,3.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut proof_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,1.0,1.0,1.0,8.0,6.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut type_wfcb =
            tptp_type_weight_parse(&mut type_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut sig_wfcb = sig_weight_parse(&mut sig_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut proof_wfcb =
            proof_weight_parse(&mut proof_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(type_wfcb.compute_eval(&bank, &type_clause), 42.0);
        assert_close(sig_wfcb.compute_eval(&bank, &sig_clause), 24.0);
        assert_close(proof_wfcb.compute_eval(&bank, &proof_clause), 54.0);
        assert_eq!(type_wfcb.compute_priority(&bank, &type_clause), PRIO_NORMAL);
        assert_eq!(sig_wfcb.compute_priority(&bank, &sig_clause), PRIO_NORMAL);
        assert_eq!(
            proof_wfcb.compute_priority(&bank, &proof_clause),
            PRIO_NORMAL
        );
        assert_eq!(type_scanner.current_token().literal(), "tail");
        assert_eq!(sig_scanner.current_token().literal(), "tail");
        assert_eq!(proof_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn type_signature_and_proof_weight_parsers_use_banked_wfcb_callbacks() {
        let mut bank = test_bank();

        let type_param = tptp_type_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 1.0);
        let mut type_clause = ordering_clause(&mut bank, "type_banked");
        type_clause.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut type_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,5.0,7.0,11.0,13.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut type_wfcb =
            tptp_type_weight_parse(&mut type_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(
            &mut bank,
            type_clause,
            &mut type_wfcb,
            |clause, bank| tptp_type_weight_compute(&type_param, bank, clause),
        );
        assert_eq!(type_scanner.current_token().literal(), "tail");

        let sig_param = sig_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 1.0);
        let sig_clause = ordering_clause(&mut bank, "sig_banked");
        let mut sig_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,5.0,7.0,11.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut sig_wfcb = sig_weight_parse(&mut sig_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(&mut bank, sig_clause, &mut sig_wfcb, |clause, bank| {
            sig_weight_compute(&sig_param, bank, bank.signature(), clause)
        });
        assert_eq!(sig_scanner.current_token().literal(), "tail");

        let proof_param = proof_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 1.0);
        let mut proof_clause = ordering_clause(&mut bank, "proof_banked");
        proof_clause.set_proof_depth(2);
        proof_clause.set_proof_size(3);
        let mut proof_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,5.0,7.0,11.0,13.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut proof_wfcb =
            proof_weight_parse(&mut proof_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(
            &mut bank,
            proof_clause,
            &mut proof_wfcb,
            |clause, bank| proof_weight_compute(&proof_param, bank, clause),
        );
        assert_eq!(proof_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn type_signature_and_proof_weight_compute_with_ocb_mark_like_c() {
        let mut bank = test_bank();

        let type_param = tptp_type_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 1.0);
        let mut type_clause = ordering_clause(&mut bank, "type");
        type_clause.set_tptp_type(CP_TYPE_CONJECTURE);
        assert_ocb_helper_matches_manual(
            &bank,
            type_clause,
            |clause| tptp_type_weight_compute(&type_param, &bank, clause),
            |ocb, clause| tptp_type_weight_compute_with_ocb(&type_param, ocb, &bank, clause),
        );

        let sig_param = sig_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 1.0);
        let sig_clause = ordering_clause(&mut bank, "sig");
        assert_ocb_helper_matches_manual(
            &bank,
            sig_clause,
            |clause| sig_weight_compute(&sig_param, &bank, bank.signature(), clause),
            |ocb, clause| sig_weight_compute_with_ocb(&sig_param, ocb, &bank, clause),
        );

        let proof_param = proof_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 13.0, 1.0);
        let mut proof_clause = ordering_clause(&mut bank, "proof");
        proof_clause.set_proof_depth(2);
        proof_clause.set_proof_size(3);
        assert_ocb_helper_matches_manual(
            &bank,
            proof_clause,
            |clause| proof_weight_compute(&proof_param, &bank, clause),
            |ocb, clause| proof_weight_compute_with_ocb(&proof_param, ocb, &bank, clause),
        );
    }

    #[test]
    fn depth_and_weight_less_depth_follow_c_literal_formulas() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let clause = unit_clause(&mut bank, &a, &b, true);
        let depth_param = depth_weight_init(2, 1, 3.0, 1.0, 7.0, 11.0, 1.0);
        let less_depth_param = weight_less_depth_init(2, 1, 3.0, 1.0, 7.0, 0.5, 1.0);

        assert_close(depth_weight_compute(&depth_param, &clause), 966.0);
        assert_close(weight_less_depth_compute(&less_depth_param, &clause), 63.0);
        assert_close(depth_param.term_weight_multiplier(), 11.0);
        assert_close(less_depth_param.term_depth_multiplier(), 0.5);
    }

    #[test]
    fn depth_variants_preserve_c_applied_variable_asymmetry() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let app = applied_free_var(&bank, &x, &a);
        let clause = unit_clause(&mut bank, &b, &app, true);
        let depth_param = depth_weight_init(2, 1, 3.0, 1.0, 1.0, 11.0, 5.0);
        let less_depth_param = weight_less_depth_init(2, 1, 3.0, 1.0, 1.0, 0.5, 5.0);

        assert_close(depth_weight_compute(&depth_param, &clause), 570.0);
        assert_close(weight_less_depth_compute(&less_depth_param, &clause), 34.5);
        assert_close(depth_param.app_var_mult(), 5.0);
    }

    #[test]
    fn structural_varweight_compute_with_ocb_mark_like_c() {
        let mut bank = test_bank();

        let depth_param = depth_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 1.0);
        let depth_clause = ordering_clause(&mut bank, "depth");
        assert_ocb_helper_matches_manual(
            &bank,
            depth_clause,
            |clause| depth_weight_compute(&depth_param, clause),
            |ocb, clause| depth_weight_compute_with_ocb(&depth_param, ocb, &bank, clause),
        );

        let less_depth_param = weight_less_depth_init(2, 1, 3.0, 5.0, 7.0, 0.5, 1.0);
        let less_depth_clause = ordering_clause(&mut bank, "less_depth");
        assert_ocb_helper_matches_manual(
            &bank,
            less_depth_clause,
            |clause| weight_less_depth_compute(&less_depth_param, clause),
            |ocb, clause| weight_less_depth_compute_with_ocb(&less_depth_param, ocb, &bank, clause),
        );

        let nl_param = nl_weight_init(2, 7, 1, 3.0, 5.0, 7.0, 1.0);
        let nl_clause = ordering_clause(&mut bank, "nl");
        assert_ocb_helper_matches_manual(
            &bank,
            nl_clause,
            |clause| nl_weight_compute(&nl_param, &bank, clause),
            |ocb, clause| nl_weight_compute_with_ocb(&nl_param, ocb, &bank, clause),
        );

        let sym_param = sym_type_weight_init(2, 1, 3, 11, 3.0, 5.0, 7.0, 1.0);
        let sym_clause = ordering_clause(&mut bank, "sym");
        assert_ocb_helper_matches_manual(
            &bank,
            sym_clause,
            |clause| sym_type_weight_compute(&sym_param, clause),
            |ocb, clause| sym_type_weight_compute_with_ocb(&sym_param, ocb, &bank, clause),
        );
    }

    #[test]
    fn nl_and_symbol_type_weights_delegate_to_clause_helpers() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let clause = unit_clause(&mut bank, &a, &b, true);
        let nl_param = nl_weight_init(2, 7, 1, 1.0, 1.0, 1.0, 1.0);
        let sym_param = sym_type_weight_init(2, 1, 3, 11, 1.0, 1.0, 1.0, 1.0);

        assert_close(nl_weight_compute(&nl_param, &bank, &clause), 6.0);
        assert_close(sym_type_weight_compute(&sym_param, &clause), 6.0);
        assert_eq!(nl_param.vlweight(), 7);
        assert_eq!(sym_param.cweight(), 3);
        assert_eq!(sym_param.pweight(), 11);
    }

    #[test]
    fn structural_varweight_parsers_wrap_existing_cores() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let clause = unit_clause(&mut bank, &a, &b, true);
        let mut depth_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,1.0,7.0,11.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut less_depth_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,1.0,7.0,0.5) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut nl_scanner = Scanner::from_user_string("(ConstPrio,2,7,1,1.0,1.0,1.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut sym_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3,11,1.0,1.0,1.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut depth_wfcb =
            depth_weight_parse(&mut depth_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut less_depth_wfcb =
            weight_less_depth_parse(&mut less_depth_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut nl_wfcb = nl_weight_parse(&mut nl_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut sym_wfcb =
            sym_type_weight_parse(&mut sym_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(depth_wfcb.compute_eval(&bank, &clause), 966.0);
        assert_close(less_depth_wfcb.compute_eval(&bank, &clause), 63.0);
        assert_close(nl_wfcb.compute_eval(&bank, &clause), 6.0);
        assert_close(sym_wfcb.compute_eval(&bank, &clause), 6.0);
        assert_eq!(depth_wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(
            less_depth_wfcb.compute_priority(&bank, &clause),
            PRIO_NORMAL
        );
        assert_eq!(nl_wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(sym_wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(depth_scanner.current_token().literal(), "tail");
        assert_eq!(less_depth_scanner.current_token().literal(), "tail");
        assert_eq!(nl_scanner.current_token().literal(), "tail");
        assert_eq!(sym_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn structural_varweight_parsers_use_banked_wfcb_callbacks() {
        let mut bank = test_bank();

        let depth_param = depth_weight_init(2, 1, 3.0, 5.0, 7.0, 11.0, 1.0);
        let depth_clause = ordering_clause(&mut bank, "depth_banked");
        let mut depth_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,5.0,7.0,11.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut depth_wfcb =
            depth_weight_parse(&mut depth_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(&mut bank, depth_clause, &mut depth_wfcb, |clause, _| {
            depth_weight_compute(&depth_param, clause)
        });
        assert_eq!(depth_scanner.current_token().literal(), "tail");

        let less_depth_param = weight_less_depth_init(2, 1, 3.0, 5.0, 7.0, 0.5, 1.0);
        let less_depth_clause = ordering_clause(&mut bank, "less_depth_banked");
        let mut less_depth_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3.0,5.0,7.0,0.5) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut less_depth_wfcb =
            weight_less_depth_parse(&mut less_depth_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(
            &mut bank,
            less_depth_clause,
            &mut less_depth_wfcb,
            |clause, _| weight_less_depth_compute(&less_depth_param, clause),
        );
        assert_eq!(less_depth_scanner.current_token().literal(), "tail");

        let nl_param = nl_weight_init(2, 7, 1, 3.0, 5.0, 7.0, 1.0);
        let nl_clause = ordering_clause(&mut bank, "nl_banked");
        let mut nl_scanner = Scanner::from_user_string("(ConstPrio,2,7,1,3.0,5.0,7.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut nl_wfcb = nl_weight_parse(&mut nl_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(&mut bank, nl_clause, &mut nl_wfcb, |clause, bank| {
            nl_weight_compute(&nl_param, bank, clause)
        });
        assert_eq!(nl_scanner.current_token().literal(), "tail");

        let sym_param = sym_type_weight_init(2, 1, 3, 11, 3.0, 5.0, 7.0, 1.0);
        let sym_clause = ordering_clause(&mut bank, "sym_banked");
        let mut sym_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,3,11,3.0,5.0,7.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut sym_wfcb =
            sym_type_weight_parse(&mut sym_scanner).unwrap_or_else(|err| panic!("{err}"));
        assert_banked_wfcb_matches_manual(&mut bank, sym_clause, &mut sym_wfcb, |clause, _| {
            sym_type_weight_compute(&sym_param, clause)
        });
        assert_eq!(sym_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn pn_refined_weight_uses_negative_weights_only_for_negative_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let positive = literal(&mut bank, &a, &b, true);
        let negative = literal(&mut bank, &a, &b, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![positive, negative]));
        let param = pn_refined_weight_init(2, 1, 13, 17, 1.0, 1.0, 1.0, 1.0);

        assert_close(pn_refined_weight_compute(&param, &bank, &clause), 45.0);
        assert_eq!(param.nfweight(), 13);
        assert_eq!(param.nvweight(), 17);
    }

    #[test]
    fn pn_refined_weight_parse_wraps_positive_and_negative_weights() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let positive = literal(&mut bank, &a, &b, true);
        let negative = literal(&mut bank, &a, &b, false);
        let clause = Clause::alloc(EqnList::from_vec(vec![positive, negative]));
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,2,1,13,17,1.0,1.0,1.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = pn_refined_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 45.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn pn_refined_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = test_bank();
        let param = pn_refined_weight_init(2, 1, 13, 17, 3.0, 5.0, 7.0, 1.0);
        let clause = ordering_clause(&mut bank, "pn_banked");
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,2,1,13,17,3.0,5.0,7.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = pn_refined_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_banked_wfcb_matches_manual(&mut bank, clause, &mut wfcb, |clause, bank| {
            pn_refined_weight_compute(&param, bank, clause)
        });
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn pn_refined_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = test_bank();
        let positive_param = pn_refined_weight_init(2, 1, 13, 17, 3.0, 5.0, 7.0, 1.0);
        let positive_clause = ordering_clause(&mut bank, "pn");

        assert_ocb_helper_matches_manual(
            &bank,
            positive_clause,
            |clause| pn_refined_weight_compute(&positive_param, &bank, clause),
            |ocb, clause| pn_refined_weight_compute_with_ocb(&positive_param, ocb, &bank, clause),
        );
    }

    #[test]
    fn clause_weight_age_adds_create_date_after_weight_multiplier() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut clause = unit_clause(&mut bank, &a, &b, true);
        clause.set_create_date(17);
        let param = clause_weight_age_init(2, 1, 1.0, 4.0, 1.0);

        assert_close(clause_weight_age_compute(&param, &bank, &clause), 41.0);
        assert_close(param.weight_multiplier(), 4.0);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 1);
        assert_close(param.max_term_multiplier(), 0.0);
        assert_close(param.max_literal_multiplier(), 0.0);
    }

    #[test]
    fn clause_weight_age_parse_wraps_age_adjusted_weight() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let mut clause = unit_clause(&mut bank, &a, &b, true);
        clause.set_create_date(17);
        let mut scanner = Scanner::from_user_string("(ConstPrio,2,1,1.0,4.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = clause_weight_age_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 41.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn staggered_weight_uses_truncated_limit_and_integer_division() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let axiom = unit_clause(&mut bank, &a, &b, true);
        let axioms = ClauseSet::from_clauses([axiom]);
        let target = unit_clause(&mut bank, &f_of_a, &g_of_b, true);
        let param = staggered_weight_init(0.5, &axioms);

        assert_eq!(param.stagger_limit(), 2);
        assert_close(staggered_weight_compute(&param, &target), 4.0);

        let empty_param = staggered_weight_init(10.0, &ClauseSet::new());
        assert_eq!(empty_param.stagger_limit(), 1);
        assert_close(staggered_weight_compute(&empty_param, &target), 8.0);
    }

    #[test]
    fn staggered_weight_parse_uses_axiom_set_limit() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let f_of_a = typed_unary(&mut bank, "f", &a);
        let g_of_b = typed_unary(&mut bank, "g", &b);
        let axiom = unit_clause(&mut bank, &a, &b, true);
        let axioms = ClauseSet::from_clauses([axiom]);
        let target = unit_clause(&mut bank, &f_of_a, &g_of_b, true);
        let mut scanner = Scanner::from_user_string("(ConstPrio,0.5) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb =
            staggered_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &target), 4.0);
        assert_eq!(wfcb.compute_priority(&bank, &target), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn empty_param_matches_c_zeroed_allocation_shape() {
        assert_eq!(VarWeightParam::empty().stagger_limit(), 0);
    }
}
