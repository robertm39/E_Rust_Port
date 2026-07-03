use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_bool, parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
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
#[expect(
    clippy::too_many_arguments,
    clippy::fn_params_excessive_bools,
    reason = "C-compatible helper mirrors DAGWeightInit"
)]
pub fn dag_weight_wfcb_init(
    prio_fun: ClausePrioFun,
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
) -> Wfcb<DagWeightParam> {
    wfcb_alloc(
        dag_weight_wfcb_compute,
        prio_fun,
        dag_weight_exit,
        Some(dag_weight_init(
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
        )),
    )
}

pub fn dag_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<DagWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let dup_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_use_dag = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_term_reset = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_eqn_reset = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let neg_use_dag = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let neg_term_reset = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let neg_eqn_reset = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_neg_reset = parse_bool(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(dag_weight_wfcb_init(
        prio_fun,
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
    ))
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
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors RDAGWeightInit without the OCB pointer"
)]
pub fn rdag_weight_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    dup_weight: i64,
    uniqmax_term_multiplier: f64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<RDagWeightParam> {
    wfcb_alloc_with_bank(
        rdag_weight_wfcb_compute,
        rdag_weight_wfcb_compute_with_bank,
        prio_fun,
        rdag_weight_exit,
        Some(rdag_weight_init(
            fweight,
            vweight,
            dup_weight,
            uniqmax_term_multiplier,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn rdag_weight_parse(scanner: &mut Scanner) -> Result<Wfcb<RDagWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let dup_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let uniqmax_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(rdag_weight_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        dup_weight,
        uniqmax_term_multiplier,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
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

/// Computes C `RDAGWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// The existing WFCB compute callback cannot mutate clauses yet, so this
/// explicit entry point is used by callers that already own a mutable clause.
#[must_use]
pub fn rdag_weight_compute_with_ocb(
    param: &RDagWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    clause.cond_mark_maximal_terms(ocb, bank);
    rdag_weight_compute(param, clause)
}

/// Computes C `RDAGWeightCompute` with bank-backed ordering preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn rdag_weight_compute_with_bank(
    param: &RDagWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(rdag_weight_compute(param, clause))
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
pub fn rdag_weight2_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    dup_weight: i64,
    max_term_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<RDagWeightParam> {
    wfcb_alloc(
        rdag_weight2_wfcb_compute,
        prio_fun,
        rdag_weight_exit,
        Some(rdag_weight2_init(
            fweight,
            vweight,
            dup_weight,
            max_term_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn rdag_weight2_parse(scanner: &mut Scanner) -> Result<Wfcb<RDagWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let dup_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(rdag_weight2_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        dup_weight,
        max_term_multiplier,
        pos_multiplier,
    ))
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
#[expect(
    clippy::too_many_arguments,
    clippy::similar_names,
    reason = "C-compatible helper mirrors RDAGWeight3Init without the OCB pointer"
)]
pub fn rdag_weight3_wfcb_init(
    prio_fun: ClausePrioFun,
    fweight: i64,
    vweight: i64,
    nfweight: i64,
    nvweight: i64,
    dup_weight: i64,
    max_term_multiplier: f64,
    pos_multiplier: f64,
    pneq_multiplier: f64,
    nneq_multiplier: f64,
) -> Wfcb<RDagWeightParam> {
    wfcb_alloc(
        rdag_weight3_wfcb_compute,
        prio_fun,
        rdag_weight_exit,
        Some(rdag_weight3_init(
            fweight,
            vweight,
            nfweight,
            nvweight,
            dup_weight,
            max_term_multiplier,
            pos_multiplier,
            pneq_multiplier,
            nneq_multiplier,
        )),
    )
}

#[expect(
    clippy::similar_names,
    reason = "C-compatible parser keeps normal and negative weight names close to RDAGWeight3Parse"
)]
pub fn rdag_weight3_parse(scanner: &mut Scanner) -> Result<Wfcb<RDagWeightParam>, Diagnostic> {
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
    let dup_weight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pneq_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let nneq_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(rdag_weight3_wfcb_init(
        prio_fun,
        fweight,
        vweight,
        nfweight,
        nvweight,
        dup_weight,
        max_term_multiplier,
        pos_multiplier,
        pneq_multiplier,
        nneq_multiplier,
    ))
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

fn dag_weight_wfcb_compute(
    data: Option<&mut DagWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => dag_weight_compute(data, bank, clause),
        None => panic!("DAGweight WFCB requires initialized weight parameters"),
    }
}

fn rdag_weight_wfcb_compute(
    data: Option<&mut RDagWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => rdag_weight_compute(data, clause),
        None => panic!("RDAGweight WFCB requires initialized weight parameters"),
    }
}

fn rdag_weight_wfcb_compute_with_bank(
    data: Option<&mut RDagWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    match data {
        Some(data) => rdag_weight_compute_with_bank(data, ocb, bank, clause),
        None => panic!("RDAGweight WFCB requires initialized weight parameters"),
    }
}

fn rdag_weight2_wfcb_compute(
    data: Option<&mut RDagWeightParam>,
    _bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => rdag_weight2_compute(data, clause),
        None => panic!("RDAGweight2 WFCB requires initialized weight parameters"),
    }
}

fn rdag_weight3_wfcb_compute(
    data: Option<&mut RDagWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => rdag_weight3_compute(data, bank, clause),
        None => panic!("RDAGweight3 WFCB requires initialized weight parameters"),
    }
}

fn dag_weight_exit(_data: DagWeightParam) {}

fn rdag_weight_exit(_data: RDagWeightParam) {}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        dag_weight_compute, dag_weight_init, dag_weight_parse, rdag_weight2_compute,
        rdag_weight2_init, rdag_weight2_parse, rdag_weight3_compute, rdag_weight3_init,
        rdag_weight3_parse, rdag_weight_compute, rdag_weight_compute_with_ocb, rdag_weight_init,
        rdag_weight_parse, DEFAULT_DAG_DUP_WEIGHT,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_ORIENTED;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
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

    fn oriented_positive_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let fa = typed_unary(bank, "f", &a);
        let ga = typed_unary(bank, "g", &a);
        let mut positive = Eqn::alloc(fa, ga, bank, true).unwrap();
        positive.set_prop(EP_IS_ORIENTED);
        Clause::alloc(EqnList::from_vec(vec![positive]))
    }

    fn positive_equation_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let fa = typed_unary(bank, "f", &a);
        let ga = typed_unary(bank, "g", &a);
        let positive = Eqn::alloc(fa, ga, bank, true).unwrap();
        Clause::alloc(EqnList::from_vec(vec![positive]))
    }

    fn mixed_equation_predicate_clause(bank: &mut TermBank) -> Clause {
        let a = typed_const(bank, "a");
        let b = typed_const(bank, "b");
        let positive_eq = Eqn::alloc(a.clone(), b.clone(), bank, true).unwrap();
        let negative_eq = Eqn::alloc(a, b, bank, false).unwrap();
        let pred = typed_pred_const(bank, "p");
        let pred_lit = Eqn::alloc(pred, bank.true_term().clone(), bank, true).unwrap();
        Clause::alloc(EqnList::from_vec(vec![positive_eq, pred_lit, negative_eq]))
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
        let clause = oriented_positive_clause(&mut bank);
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
    fn refined_dag_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = test_bank();
        let mut target = positive_equation_clause(&mut bank);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = rdag_weight_init(10, 3, 1, 5.0, 2.0, 7.0, 4.0);
        let expected = rdag_weight_compute(&param, &manually_marked);
        let mut ocb = kbo_ocb(&bank);

        let actual = rdag_weight_compute_with_ocb(&param, &mut ocb, &bank, &mut target);

        assert_close(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn refined_dag_weight_parse_banked_callback_marks_clause_like_c() {
        let mut bank = test_bank();
        let mut target = positive_equation_clause(&mut bank);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let param = rdag_weight_init(10, 3, 1, 5.0, 2.0, 7.0, 4.0);
        let expected = rdag_weight_compute(&param, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,10,3,1,5.0,2.0,7.0,4.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = rdag_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));
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
    fn refined_dag_weight2_boosts_larger_side_without_orientation() {
        let mut bank = test_bank();
        let clause = positive_equation_clause(&mut bank);
        let param = rdag_weight2_init(10, 3, 1, 4.0, 2.0);

        assert_close(rdag_weight2_compute(&param, &clause), 200.0);
    }

    #[test]
    fn refined_dag_weight3_preserves_equational_multiplier_condition() {
        let mut bank = test_bank();
        let clause = mixed_equation_predicate_clause(&mut bank);
        let param = rdag_weight3_init(2, 1, 13, 17, 1, 3.0, 5.0, 7.0, 11.0);

        assert_close(rdag_weight3_compute(&param, &bank, &clause), 606.0);
        assert_eq!(param.nfweight(), 13);
        assert_eq!(param.nvweight(), 17);
        assert_close(param.pneq_multiplier(), 7.0);
        assert_close(param.nneq_multiplier(), 11.0);
    }

    #[test]
    fn dag_weight_parse_wraps_boolean_reset_modes() {
        let mut bank = test_bank();
        let clause = shared_positive_negative_clause(&mut bank);
        let mut scanner = Scanner::from_user_string(
            "(ConstPrio,2,1,3.0,1,true,false,false,true,false,false,false) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = dag_weight_parse(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), 17.0);
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn refined_dag_weight_parsers_wrap_existing_scoring_cores() {
        let mut oriented_bank = test_bank();
        let mut plain_bank = test_bank();
        let mut mixed_bank = test_bank();
        let oriented = oriented_positive_clause(&mut oriented_bank);
        let plain_positive = positive_equation_clause(&mut plain_bank);
        let mixed = mixed_equation_predicate_clause(&mut mixed_bank);
        let mut oriented_scanner =
            Scanner::from_user_string("(ConstPrio,10,3,1,5.0,2.0,7.0,4.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut plain_scanner = Scanner::from_user_string("(ConstPrio,10,3,1,4.0,2.0) tail", false)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut mixed_scanner =
            Scanner::from_user_string("(ConstPrio,2,1,13,17,1,3.0,5.0,7.0,11.0) tail", false)
                .unwrap_or_else(|err| panic!("{err}"));
        let mut rdag =
            rdag_weight_parse(&mut oriented_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut rdag2 =
            rdag_weight2_parse(&mut plain_scanner).unwrap_or_else(|err| panic!("{err}"));
        let mut rdag3 =
            rdag_weight3_parse(&mut mixed_scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_close(rdag.compute_eval(&oriented_bank, &oriented), 880.0);
        assert_close(rdag2.compute_eval(&plain_bank, &plain_positive), 200.0);
        assert_close(rdag3.compute_eval(&mixed_bank, &mixed), 606.0);
        assert_eq!(
            rdag.compute_priority(&oriented_bank, &oriented),
            PRIO_NORMAL
        );
        assert_eq!(
            rdag2.compute_priority(&plain_bank, &plain_positive),
            PRIO_NORMAL
        );
        assert_eq!(rdag3.compute_priority(&mixed_bank, &mixed), PRIO_NORMAL);
        assert_eq!(oriented_scanner.current_token().literal(), "tail");
        assert_eq!(plain_scanner.current_token().literal(), "tail");
        assert_eq!(mixed_scanner.current_token().literal(), "tail");
    }
}
