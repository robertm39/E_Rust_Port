use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::{parse_filename, parse_float, parse_int};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::learn::annotations::ANNOTATION_DEFAULT_SIZE;
use crate::learn::clauseenc::{flat_encode_clause_list_rep, rec_encode_clause_list_rep};
use crate::learn::indexfunctions::{get_index_type, IndexType};
use crate::learn::numfeatures::{compute_clause_set_num_features, Features};
use crate::learn::patterns::{pattern_clause_compute, PatternSubst};
use crate::learn::tsm::{get_tsm_type, tsm_eval_term, TsmAdmin, TsmType};
use crate::learn::tsmio::{tsm_from_kb, tsm_from_kb_with_target_features};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;

pub const TSM_E_WEIGHT_COUNT: usize = ANNOTATION_DEFAULT_SIZE - 1;

#[derive(Clone, Debug, PartialEq)]
pub struct TsmParam {
    fweight: i64,
    vweight: i64,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    flat_clauses: bool,
    learnweight: f64,
    kb: String,
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    depth: i64,
    e_weights: [f64; TSM_E_WEIGHT_COUNT],
    eval_base: f64,
    eval_scale: f64,
}

#[derive(Debug)]
struct TsmEvalState {
    bank: TsmEvalBank,
    admin: TsmAdmin,
    pat_subst: PatternSubst,
}

#[derive(Debug)]
enum TsmEvalBank {
    ProofState,
    Private(Box<TermBank>),
}

#[derive(Clone, Debug)]
enum TsmTargetSource {
    Axioms(Box<ClauseSet>),
    FeatureSnapshot(Box<Features>),
}

#[derive(Debug)]
pub struct TsmEvaluator {
    param: TsmParam,
    target: Option<TsmTargetSource>,
    eval: Option<TsmEvalState>,
}

impl TsmParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible constructor mirrors tsm_param_init"
    )]
    pub fn new(
        fweight: i64,
        vweight: i64,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
        flat_clauses: bool,
        learnweight: f64,
        kb: String,
        sel_no: i64,
        set_part: f64,
        dist_part: f64,
        index_type: IndexType,
        tsm_type: TsmType,
        depth: i64,
        e_weights: [f64; TSM_E_WEIGHT_COUNT],
    ) -> Self {
        let (eval_base, eval_scale) = eval_normalization(e_weights);
        Self {
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            flat_clauses,
            learnweight,
            kb,
            sel_no,
            set_part,
            dist_part,
            index_type,
            tsm_type,
            depth,
            e_weights,
            eval_base,
            eval_scale,
        }
    }

    #[must_use]
    pub const fn fweight(&self) -> i64 {
        self.fweight
    }

    #[must_use]
    pub const fn vweight(&self) -> i64 {
        self.vweight
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
    pub const fn flat_clauses(&self) -> bool {
        self.flat_clauses
    }

    #[must_use]
    pub const fn learnweight(&self) -> f64 {
        self.learnweight
    }

    #[must_use]
    pub fn kb(&self) -> &str {
        &self.kb
    }

    #[must_use]
    pub const fn sel_no(&self) -> i64 {
        self.sel_no
    }

    #[must_use]
    pub const fn set_part(&self) -> f64 {
        self.set_part
    }

    #[must_use]
    pub const fn dist_part(&self) -> f64 {
        self.dist_part
    }

    #[must_use]
    pub const fn index_type(&self) -> IndexType {
        self.index_type
    }

    #[must_use]
    pub const fn tsm_type(&self) -> TsmType {
        self.tsm_type
    }

    #[must_use]
    pub const fn depth(&self) -> i64 {
        self.depth
    }

    #[must_use]
    pub const fn e_weights(&self) -> [f64; TSM_E_WEIGHT_COUNT] {
        self.e_weights
    }

    #[must_use]
    pub const fn eval_base(&self) -> f64 {
        self.eval_base
    }

    #[must_use]
    pub const fn eval_scale(&self) -> f64 {
        self.eval_scale
    }
}

impl TsmEvaluator {
    #[must_use]
    pub fn new(param: TsmParam, axioms: ClauseSet) -> Self {
        Self {
            param,
            target: Some(TsmTargetSource::Axioms(Box::new(axioms))),
            eval: None,
        }
    }

    #[must_use]
    pub fn new_with_target_features(param: TsmParam, target_features: Features) -> Self {
        Self {
            param,
            target: Some(TsmTargetSource::FeatureSnapshot(Box::new(target_features))),
            eval: None,
        }
    }

    #[must_use]
    pub const fn param(&self) -> &TsmParam {
        &self.param
    }

    #[must_use]
    pub const fn is_initialized(&self) -> bool {
        self.eval.is_some()
    }

    /// Computes the C `TSMWeightCompute`/`TSMRWeightCompute` score.
    ///
    /// # Panics
    ///
    /// Panics if lazy KB loading, adapter-bank allocation, clause copying,
    /// representative pattern computation, clause-representation encoding, or
    /// TSM evaluation violates the same internal invariants that the C path
    /// enforces with fatal errors or assertions. Production proof search uses
    /// [`Self::compute_with_bank`] and does not copy the clause.
    pub fn compute(&mut self, bank: &TermBank, clause: &Clause) -> f64 {
        self.ensure_private_init(bank.signature());
        let param = &self.param;
        let state = self
            .eval
            .as_mut()
            .unwrap_or_else(|| panic!("TSM evaluator state must be initialized"));
        let TsmEvalState {
            bank: eval_bank,
            admin,
            pat_subst,
        } = state;
        match eval_bank {
            TsmEvalBank::Private(eval_bank) => {
                let copied_clause = copy_clause_for_tsm(clause, eval_bank);
                compute_in_bank(param, admin, pat_subst, eval_bank, &copied_clause)
            }
            TsmEvalBank::ProofState => {
                let mut adapter_bank = TermBank::new(bank.signature().clone())
                    .unwrap_or_else(|err| panic!("TSMWeight adapter bank allocation: {err}"));
                let copied_clause = copy_clause_for_tsm(clause, &mut adapter_bank);
                compute_in_bank(param, admin, pat_subst, &mut adapter_bank, &copied_clause)
            }
        }
    }

    /// Computes through C's shared proof-state term-bank owner.
    ///
    /// KB signature declarations and clause representations are installed in
    /// the active bank, matching `TSMWeightCompute`'s use of
    /// `local->state->terms` and avoiding a clause copy on the hot path.
    ///
    /// # Panics
    ///
    /// Panics under the same fatal-error and internal-invariant conditions as
    /// [`Self::compute`].
    pub fn compute_with_bank(&mut self, bank: &mut TermBank, clause: &Clause) -> f64 {
        self.ensure_proof_state_init(bank);
        let param = &self.param;
        let state = self
            .eval
            .as_mut()
            .unwrap_or_else(|| panic!("TSM evaluator state must be initialized"));
        let TsmEvalState {
            bank: eval_bank,
            admin,
            pat_subst,
        } = state;
        match eval_bank {
            TsmEvalBank::ProofState => compute_in_bank(param, admin, pat_subst, bank, clause),
            TsmEvalBank::Private(eval_bank) => {
                let copied_clause = copy_clause_for_tsm(clause, eval_bank);
                compute_in_bank(param, admin, pat_subst, eval_bank, &copied_clause)
            }
        }
    }

    fn ensure_private_init(&mut self, signature: &Signature) {
        if self.eval.is_some() {
            return;
        }

        let mut kb_signature = signature.clone();
        let (admin, pat_subst) = self.load_eval_parts(&mut kb_signature);
        let bank = TermBank::new(kb_signature)
            .unwrap_or_else(|err| panic!("TSMWeight eval bank allocation: {err}"));
        self.eval = Some(TsmEvalState {
            bank: TsmEvalBank::Private(Box::new(bank)),
            admin,
            pat_subst,
        });
    }

    fn ensure_proof_state_init(&mut self, bank: &mut TermBank) {
        if self.eval.is_some() {
            return;
        }

        let (admin, pat_subst) = self.load_eval_parts(bank.signature_mut());
        self.eval = Some(TsmEvalState {
            bank: TsmEvalBank::ProofState,
            admin,
            pat_subst,
        });
    }

    fn load_eval_parts(&mut self, signature: &mut Signature) -> (TsmAdmin, PatternSubst) {
        let target = self.target.as_ref().unwrap_or_else(|| {
            panic!("TSM evaluator requires a lazy target before initialization")
        });
        let admin = match target {
            TsmTargetSource::Axioms(axioms) => tsm_from_kb(
                self.param.flat_clauses,
                &self.param.e_weights,
                &self.param.kb,
                signature,
                axioms,
                self.param.sel_no,
                self.param.set_part,
                self.param.dist_part,
                self.param.index_type,
                self.param.tsm_type,
                tsm_depth_to_i32(self.param.depth),
            ),
            TsmTargetSource::FeatureSnapshot(target_features) => tsm_from_kb_with_target_features(
                self.param.flat_clauses,
                &self.param.e_weights,
                &self.param.kb,
                signature,
                target_features,
                self.param.sel_no,
                self.param.set_part,
                self.param.dist_part,
                self.param.index_type,
                self.param.tsm_type,
                tsm_depth_to_i32(self.param.depth),
            ),
        }
        .unwrap_or_else(|err| panic!("TSMWeight KB initialization: {err}"));
        let pat_subst = PatternSubst::default_subst(signature);
        self.target = None;
        (admin, pat_subst)
    }
}

fn copy_clause_for_tsm(clause: &Clause, bank: &mut TermBank) -> Clause {
    clause
        .copy_to_bank(bank)
        .unwrap_or_else(|err| panic!("TSMWeight clause copy into eval bank: {err}"))
}

fn compute_in_bank(
    param: &TsmParam,
    admin: &mut TsmAdmin,
    pat_subst: &mut PatternSubst,
    bank: &mut TermBank,
    clause: &Clause,
) -> f64 {
    pat_subst.backtrack_to(0);
    let pattern = pattern_clause_compute(clause, pat_subst.clone());
    let raw_factor = if pattern.tries() != 0 {
        let subst = pattern.subst().clone();
        let clauserep = if param.flat_clauses {
            flat_encode_clause_list_rep(bank, pattern.listrep())
        } else {
            rec_encode_clause_list_rep(bank, pattern.listrep())
        }
        .unwrap_or_else(|err| panic!("TSMWeight clause representation encoding: {err}"));
        let factor = tsm_eval_term(admin, &clauserep, &subst);
        *pat_subst = subst;
        factor
    } else {
        admin.limit()
    };
    let factor = (raw_factor - param.eval_base) / param.eval_scale;
    let base = clause.literal_weight(
        bank,
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.vweight,
        param.fweight,
        1.0,
        false,
    );
    ((param.learnweight * factor) + 1.0) * base
}

#[must_use]
pub fn tsm_weight_init(
    prio_fun: ClausePrioFun,
    param: TsmParam,
    axioms: &ClauseSet,
) -> Wfcb<TsmEvaluator> {
    wfcb_alloc_with_bank(
        tsm_weight_wfcb_compute,
        tsm_weight_wfcb_compute_with_bank,
        prio_fun,
        tsm_weight_exit,
        Some(TsmEvaluator::new(param, axioms.clone())),
    )
}

#[must_use]
pub fn tsm_weight_init_with_signature(
    prio_fun: ClausePrioFun,
    param: TsmParam,
    axioms: &ClauseSet,
    signature: &Signature,
) -> Wfcb<TsmEvaluator> {
    let mut target_features = Features::new();
    compute_clause_set_num_features(&mut target_features, axioms, signature);
    wfcb_alloc_with_bank(
        tsm_weight_wfcb_compute,
        tsm_weight_wfcb_compute_with_bank,
        prio_fun,
        tsm_weight_exit,
        Some(TsmEvaluator::new_with_target_features(
            param,
            target_features,
        )),
    )
}

pub fn tsm_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<TsmEvaluator>, Diagnostic> {
    let (prio_fun, param) = tsm_weight_parse_params(scanner)?;
    Ok(tsm_weight_init(prio_fun, param, axioms))
}

pub fn tsm_weight_parse_with_signature(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
    signature: &Signature,
) -> Result<Wfcb<TsmEvaluator>, Diagnostic> {
    let (prio_fun, param) = tsm_weight_parse_params(scanner)?;
    Ok(tsm_weight_init_with_signature(
        prio_fun, param, axioms, signature,
    ))
}

pub fn tsmr_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<TsmEvaluator>, Diagnostic> {
    let (prio_fun, param) = tsmr_weight_parse_params(scanner)?;
    Ok(tsm_weight_init(prio_fun, param, axioms))
}

pub fn tsmr_weight_parse_with_signature(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
    signature: &Signature,
) -> Result<Wfcb<TsmEvaluator>, Diagnostic> {
    let (prio_fun, param) = tsmr_weight_parse_params(scanner)?;
    Ok(tsm_weight_init_with_signature(
        prio_fun, param, axioms, signature,
    ))
}

pub fn tsm_weight_parse_params(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, TsmParam), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let fweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let vweight = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let learnweight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let flat_clauses = parse_flat_or_rec(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let kb = parse_filename(scanner)?;
    let tail = parse_tsm_common_tail(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok((
        prio_fun,
        TsmParam::new(
            fweight,
            vweight,
            1.0,
            1.0,
            1.0,
            flat_clauses,
            learnweight,
            kb,
            tail.sel_no,
            tail.set_part,
            tail.dist_part,
            tail.index_type,
            tail.tsm_type,
            tail.depth,
            tail.e_weights,
        ),
    ))
}

pub fn tsmr_weight_parse_params(
    scanner: &mut Scanner,
) -> Result<(ClausePrioFun, TsmParam), Diagnostic> {
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
    let learnweight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let flat_clauses = parse_flat_or_rec(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let kb = parse_filename(scanner)?;
    let tail = parse_tsm_common_tail(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok((
        prio_fun,
        TsmParam::new(
            fweight,
            vweight,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            flat_clauses,
            learnweight,
            kb,
            tail.sel_no,
            tail.set_part,
            tail.dist_part,
            tail.index_type,
            tail.tsm_type,
            tail.depth,
            tail.e_weights,
        ),
    ))
}

struct TsmTail {
    sel_no: i64,
    set_part: f64,
    dist_part: f64,
    index_type: IndexType,
    tsm_type: TsmType,
    depth: i64,
    e_weights: [f64; TSM_E_WEIGHT_COUNT],
}

fn parse_tsm_common_tail(scanner: &mut Scanner) -> Result<TsmTail, Diagnostic> {
    scanner.accept_tok(TokenType::COMMA)?;
    let sel_no = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let set_part = parse_float(scanner)?;
    if !(0.0..=1.0).contains(&set_part) {
        return Err(learning_parse_error(
            scanner,
            "You need to specify the part of the knowledge base to be used as a fraction between 0.0 and 1.0!",
        ));
    }
    scanner.accept_tok(TokenType::COMMA)?;
    let dist_part = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let tsm_type = parse_tsm_type(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let index_type = parse_index_type(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let depth = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let proofs_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let dist_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let p_simp_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let f_simp_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let p_gen_w = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let f_gen_w = parse_float(scanner)?;

    Ok(TsmTail {
        sel_no,
        set_part,
        dist_part,
        index_type,
        tsm_type,
        depth,
        e_weights: [proofs_w, dist_w, p_simp_w, f_simp_w, p_gen_w, f_gen_w],
    })
}

fn parse_flat_or_rec(scanner: &mut Scanner) -> Result<bool, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let literal = scanner.current_token().literal();
    let flat_clauses = match literal.as_str() {
        "flat" => true,
        "rec" => false,
        _ => return Err(learning_parse_error(scanner, "expected rec|flat")),
    };
    scanner.next_token()?;
    Ok(flat_clauses)
}

fn parse_tsm_type(scanner: &mut Scanner) -> Result<TsmType, Diagnostic> {
    scanner.check_tok(TokenType::NAME)?;
    let literal = scanner.current_token().literal();
    let Some(tsm_type) = get_tsm_type(&literal) else {
        return Err(learning_parse_error(
            scanner,
            "No correct TSM type specified!",
        ));
    };
    if tsm_type == TsmType::NoType {
        return Err(learning_parse_error(
            scanner,
            "No correct TSM type specified!",
        ));
    }
    scanner.next_token()?;
    Ok(tsm_type)
}

fn parse_index_type(scanner: &mut Scanner) -> Result<IndexType, Diagnostic> {
    scanner.check_tok(TokenType::NAME)?;
    let literal = scanner.current_token().literal();
    let Some(index_type) = get_index_type(&literal) else {
        return Err(learning_parse_error(
            scanner,
            "No correct index type specified!",
        ));
    };
    if matches!(index_type, IndexType::NO_INDEX | IndexType::EMPTY) {
        return Err(learning_parse_error(
            scanner,
            "No correct index type specified!",
        ));
    }
    scanner.next_token()?;
    Ok(index_type)
}

fn eval_normalization(e_weights: [f64; TSM_E_WEIGHT_COUNT]) -> (f64, f64) {
    let mut pos_sum = 0.0;
    let mut neg_sum = 0.0;
    for weight in e_weights {
        if weight > 0.0 {
            pos_sum += weight;
        } else {
            neg_sum += weight;
        }
    }
    let scale = pos_sum - neg_sum;
    (neg_sum, if scale == 0.0 { 1.0 } else { scale })
}

fn learning_parse_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{} {message}", token_pos_rep(scanner.current_token())),
    )
}

fn tsm_depth_to_i32(depth: i64) -> i32 {
    i32::try_from(depth).expect("TSM index depth must fit C int")
}

fn tsm_weight_wfcb_compute(
    data: Option<&mut TsmEvaluator>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    data.unwrap_or_else(|| panic!("TSMWeight WFCB requires initialized parameters"))
        .compute(bank, clause)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "banked WFCB callbacks share the diagnostic-returning contract"
)]
fn tsm_weight_wfcb_compute_with_bank(
    data: Option<&mut TsmEvaluator>,
    _ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    Ok(data
        .unwrap_or_else(|| panic!("TSMWeight WFCB requires initialized parameters"))
        .compute_with_bank(bank, clause))
}

fn tsm_weight_exit(_data: TsmEvaluator) {}

#[cfg(test)]
mod tests {
    use super::{
        tsm_weight_init, tsm_weight_parse, tsm_weight_parse_params,
        tsm_weight_parse_with_signature, tsmr_weight_parse, tsmr_weight_parse_params, TsmEvalBank,
        TsmEvaluator, TsmTargetSource, ANNOTATION_DEFAULT_SIZE,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::neweval::PRIO_NORMAL;
    use crate::heuristics::to_params::TermOrdering;
    use crate::heuristics::wfcb::ClausePrioFun;
    use crate::inout::scanner::Scanner;
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::numfeatures::FEATURE_NUMBER;
    use crate::learn::tsm::TsmType;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn assert_f64_array_bits(actual: [f64; 6], expected: [f64; 6]) {
        for (actual_value, expected_value) in actual.into_iter().zip(expected) {
            assert_eq!(actual_value.to_bits(), expected_value.to_bits());
        }
    }

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn assert_const_prio(prio_fun: ClausePrioFun) {
        let bank = term_bank();
        let clause = Clause::empty();
        assert_eq!(prio_fun(&bank, &clause), PRIO_NORMAL);
    }

    #[test]
    fn tsm_weight_parse_params_preserves_c_field_order_and_normalization() {
        let mut scanner = Scanner::from_user_string(
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb/name,10,0.25,0.75,Flat,IndexDynamic,4,1.0,-2.0,3.0,0.0,5.0,-7.0) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        scanner.next_token().unwrap();
        let (prio_fun, param) =
            tsm_weight_parse_params(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_const_prio(prio_fun);
        assert_eq!(param.fweight(), 2);
        assert_eq!(param.vweight(), 3);
        assert_close(param.max_term_multiplier(), 1.0);
        assert_close(param.max_literal_multiplier(), 1.0);
        assert_close(param.pos_multiplier(), 1.0);
        assert!(param.flat_clauses());
        assert_close(param.learnweight(), 0.5);
        assert_eq!(param.kb(), "kb/name");
        assert_eq!(param.sel_no(), 10);
        assert_close(param.set_part(), 0.25);
        assert_close(param.dist_part(), 0.75);
        assert_eq!(param.tsm_type(), TsmType::Flat);
        assert_eq!(param.index_type(), IndexType::DYNAMIC);
        assert_eq!(param.depth(), 4);
        assert_f64_array_bits(param.e_weights(), [1.0, -2.0, 3.0, 0.0, 5.0, -7.0]);
        assert_close(param.eval_base(), -9.0);
        assert_close(param.eval_scale(), 18.0);
        assert_eq!(ANNOTATION_DEFAULT_SIZE, 7);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn tsm_weight_wfcb_lazily_loads_kb_and_scales_clause_weight() {
        let kb_dir = temp_kb_dir("tsm-weight");
        write_tiny_kb(&kb_dir);
        let mut bank = term_bank();
        let clause = unit_equality(&mut bank, "a");
        let expected_base = clause.literal_weight(&bank, 1.0, 1.0, 1.0, 3, 2, 1.0, false);
        let mut scanner = Scanner::from_user_string(
            &format!(
                "(ConstPrio,2,3,0.5,rec,{},1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0) tail",
                kb_arg(&kb_dir)
            ),
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let axioms = ClauseSet::new();
        let mut wfcb =
            tsm_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| panic!("{err}"));

        assert!(!wfcb.data().expect("TSM evaluator data").is_initialized());
        assert!(matches!(
            &wfcb.data().expect("TSM evaluator data").target,
            Some(TsmTargetSource::Axioms(_))
        ));
        assert_close(wfcb.compute_eval(&bank, &clause), expected_base);
        assert!(wfcb.data().expect("TSM evaluator data").is_initialized());
        assert!(wfcb.data().expect("TSM evaluator data").target.is_none());
        assert_eq!(wfcb.compute_priority(&bank, &clause), PRIO_NORMAL);
        assert_eq!(scanner.current_token().literal(), "tail");

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_weight_signature_context_retains_compact_features_until_lazy_init() {
        let kb_dir = temp_kb_dir("tsm-weight-features");
        write_tiny_kb(&kb_dir);
        let mut bank = term_bank();
        let clause = unit_equality(&mut bank, "target");
        let expected_base = clause.literal_weight(&bank, 1.0, 1.0, 1.0, 3, 2, 1.0, false);
        let axioms = ClauseSet::from_clauses([clause.clone()]);
        let mut scanner = Scanner::from_user_string(
            &format!(
                "(ConstPrio,2,3,0.5,rec,{},1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0) tail",
                kb_arg(&kb_dir)
            ),
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = tsm_weight_parse_with_signature(&mut scanner, &axioms, bank.signature())
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(
            &wfcb.data().expect("TSM evaluator data").target,
            Some(TsmTargetSource::FeatureSnapshot(_))
        ));
        assert_close(wfcb.compute_eval(&bank, &clause), expected_base);
        assert!(wfcb.data().expect("TSM evaluator data").target.is_none());
        assert_eq!(scanner.current_token().literal(), "tail");

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_weight_banked_compute_uses_proof_state_term_bank() {
        let kb_dir = temp_kb_dir("tsm-weight-proof-state-bank");
        write_tiny_kb(&kb_dir);
        let mut bank = term_bank();
        let mut clause = unit_equality(&mut bank, "target");
        let expected_base = clause.literal_weight(&bank, 1.0, 1.0, 1.0, 3, 2, 1.0, false);
        let axioms = ClauseSet::from_clauses([clause.clone()]);
        let mut scanner = Scanner::from_user_string(
            &format!(
                "(ConstPrio,2,3,0.5,rec,{},1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0) tail",
                kb_arg(&kb_dir)
            ),
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut wfcb = tsm_weight_parse_with_signature(&mut scanner, &axioms, bank.signature())
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Empty,
            false,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );

        assert_eq!(bank.signature().find_f_code("pattern_sym"), 0);
        let actual = wfcb
            .compute_eval_with_bank(&mut ocb, &mut bank, &mut clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_close(actual, expected_base);
        assert_ne!(bank.signature().find_f_code("pattern_sym"), 0);
        assert!(matches!(
            wfcb.data()
                .expect("TSM evaluator data")
                .eval
                .as_ref()
                .expect("lazy TSM state")
                .bank,
            TsmEvalBank::ProofState
        ));
        assert_eq!(scanner.current_token().literal(), "tail");

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsmr_weight_wfcb_uses_refined_clause_weight_multipliers() {
        let kb_dir = temp_kb_dir("tsmr-weight");
        write_tiny_kb(&kb_dir);
        let mut bank = term_bank();
        let clause = unit_equality(&mut bank, "a");
        let expected_base = clause.literal_weight(&bank, 4.0, 5.0, 6.0, 3, 2, 1.0, false);
        let mut scanner = Scanner::from_user_string(
            &format!(
                "(ConstPrio,2,3,4.0,5.0,6.0,0.5,rec,{},1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0) tail",
                kb_arg(&kb_dir)
            ),
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let axioms = ClauseSet::new();
        let mut wfcb =
            tsmr_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| panic!("{err}"));

        assert_close(wfcb.compute_eval(&bank, &clause), expected_base);
        assert_eq!(scanner.current_token().literal(), "tail");

        remove_dir_if_present(&kb_dir);
    }

    #[test]
    fn tsm_weight_init_keeps_parsed_params_available() {
        let param = tsm_param_for_test("kb");
        let wfcb = tsm_weight_init(
            crate::heuristics::prio_funs::prio_fun_const_prio,
            param.clone(),
            &ClauseSet::new(),
        );

        let data: &TsmEvaluator = wfcb.data().expect("TSM evaluator data");
        assert_eq!(data.param(), &param);
        assert!(!data.is_initialized());
    }

    #[test]
    fn tsmr_weight_parse_params_reads_refined_multipliers() {
        let mut scanner = Scanner::from_user_string(
            "TSMRWeight(ConstPrio,2,3,4.0,5.0,6.0,0.5,rec,kb,10,0.25,0.75,Recursive,IndexTop,4,1.0,2.0,3.0,4.0,5.0,6.0) tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        scanner.next_token().unwrap();
        let (prio_fun, param) =
            tsmr_weight_parse_params(&mut scanner).unwrap_or_else(|err| panic!("{err}"));

        assert_const_prio(prio_fun);
        assert!(!param.flat_clauses());
        assert_close(param.max_term_multiplier(), 4.0);
        assert_close(param.max_literal_multiplier(), 5.0);
        assert_close(param.pos_multiplier(), 6.0);
        assert_eq!(param.tsm_type(), TsmType::Recursive);
        assert_eq!(param.index_type(), IndexType::TOP);
        assert_close(param.eval_base(), 0.0);
        assert_close(param.eval_scale(), 21.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn tsm_weight_parse_params_rejects_c_invalid_name_and_range_cases() {
        for spec in [
            "TSMWeight(ConstPrio,2,3,0.5,bad,kb,10,0.25,0.75,Flat,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,1.25,0.75,Flat,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,NoType,IndexTop,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,Flat,IndexNoIndex,4,1,1,1,1,1,1)",
            "TSMWeight(ConstPrio,2,3,0.5,flat,kb,10,0.25,0.75,Flat,IndexEmpty,4,1,1,1,1,1,1)",
        ] {
            let mut scanner =
                Scanner::from_user_string(spec, false).unwrap_or_else(|err| panic!("{err}"));
            scanner.next_token().unwrap();
            let Err(error) = tsm_weight_parse_params(&mut scanner) else {
                panic!("invalid TSM parser input should fail: {spec}");
            };
            assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        }
    }

    fn unit_equality(bank: &mut TermBank, name: &str) -> Clause {
        let code = bank.signature_mut().insert_id(name, 0, false);
        let sort = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(code, sort)
            .expect("constant type declaration");
        let term = bank.create_const_term(code).expect("constant insertion");
        Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            term.clone(),
            term,
            bank,
            true,
        )
        .expect("equality allocation")]))
    }

    fn tsm_param_for_test(kb: &str) -> super::TsmParam {
        super::TsmParam::new(
            2,
            3,
            1.0,
            1.0,
            1.0,
            true,
            0.5,
            kb.to_owned(),
            1,
            1.0,
            1.0,
            IndexType::ARITY,
            TsmType::Flat,
            0,
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        )
    }

    fn zero_feature_source() -> String {
        let mut result = String::from("PA: () FA: () (0");
        for _ in 1..FEATURE_NUMBER {
            result.push_str(", 0");
        }
        result.push(')');
        result
    }

    fn temp_kb_dir(label: &str) -> std::path::PathBuf {
        std::path::PathBuf::from("target")
            .join("umlaut-tests")
            .join(format!("learning-{label}-{}", std::process::id()))
    }

    fn kb_arg(path: &std::path::Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn write_tiny_kb(kb_dir: &std::path::Path) {
        remove_dir_if_present(kb_dir);
        std::fs::create_dir_all(kb_dir).expect("create temporary KB directory");
        std::fs::write(
            kb_dir.join("clausepatterns"),
            "pattern_sym : 1:(1,1,0,0,0,0,0).",
        )
        .expect("write clausepatterns file");
        std::fs::write(kb_dir.join("signature"), "pattern_sym:0").expect("write signature file");
        std::fs::write(
            kb_dir.join("problems"),
            format!("1: \"only\" {}", zero_feature_source()),
        )
        .expect("write problems file");
    }

    fn remove_dir_if_present(path: &std::path::Path) {
        match std::fs::remove_dir_all(path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => panic!("remove temporary directory {}: {err}", path.display()),
        }
    }
}
