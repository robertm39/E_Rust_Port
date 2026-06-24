use std::collections::BTreeSet;

use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS};
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_float, parse_int};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_depth, term_weight_compute};
use crate::terms::termtypes::Term;

const APP_VAR_MULT_DEFAULT: f64 = 1.0;

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
    wfcb_alloc(
        pn_refined_weight_wfcb_compute,
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

    let mut app_var_mult = APP_VAR_MULT_DEFAULT;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        app_var_mult = parse_float(scanner)?;
    }

    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
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

fn pn_refined_weight_wfcb_compute(
    data: Option<&mut VarWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    match data {
        Some(data) => pn_refined_weight_compute(data, bank, clause),
        None => panic!("PNRefinedweight WFCB requires initialized weight parameters"),
    }
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
pub fn staggered_weight_compute(param: &VarWeightParam, clause: &Clause) -> f64 {
    i64_to_f64(clause.standard_weight() / param.stagger_limit)
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
        depth_weight_compute, depth_weight_init, nl_weight_compute, nl_weight_init,
        pn_refined_weight_compute, pn_refined_weight_init, pn_refined_weight_parse,
        proof_weight_compute, proof_weight_init, sig_weight_compute, sig_weight_init,
        staggered_weight_compute, staggered_weight_init, sym_type_weight_compute,
        sym_type_weight_init, tptp_type_weight_compute, tptp_type_weight_init,
        weight_less_depth_compute, weight_less_depth_init, VarWeightParam,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::inout::scanner::Scanner;
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
    fn empty_param_matches_c_zeroed_allocation_shape() {
        assert_eq!(VarWeightParam::empty().stagger_limit(), 0);
    }
}
