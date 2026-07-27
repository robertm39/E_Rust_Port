use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::pdtrees::{prefix_compute_term_code, PdTree};
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::termweights::{
    parse_c_int, parse_related_term_set, parse_term_weight_extension_style, parse_var_norm_style,
    tb_count_term_freqs, tb_insert_clause_terms_normalized, RelatedTermSet, TermFrequencyTree,
};
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::Term;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};
use std::cell::RefCell;

#[derive(Debug)]
struct TfIdfEvalState {
    eval_bank: TermBank,
    eval_freqs: TermFrequencyTree,
    document_index: PdTree,
}

impl TfIdfEvalState {
    fn new(signature: &Signature) -> Self {
        Self {
            eval_bank: TermBank::new(signature.clone())
                .unwrap_or_else(|err| panic!("ConjectureTermTfIdfWeight eval bank init: {err}")),
            eval_freqs: TermFrequencyTree::new(),
            document_index: PdTree::new(),
        }
    }

    fn document_count(&self) -> usize {
        self.document_index.term_count()
    }
}

#[derive(Debug)]
pub struct TfIdfWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    update_docs: i32,
    tf_fact: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    eval: RefCell<Option<TfIdfEvalState>>,
}

impl TfIdfWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureTermTfIdfWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        update_docs: i32,
        tf_fact: f64,
        ext_style: TermWeightExtensionStyle,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
    ) -> Self {
        Self {
            axioms: axioms.clone(),
            var_norm,
            rel_terms,
            update_docs,
            tf_fact,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            eval: RefCell::new(None),
        }
    }

    #[must_use]
    pub const fn var_norm(&self) -> VarNormStyle {
        self.var_norm
    }

    #[must_use]
    pub const fn rel_terms(&self) -> RelatedTermSet {
        self.rel_terms
    }

    #[must_use]
    pub const fn update_docs(&self) -> i32 {
        self.update_docs
    }

    #[must_use]
    pub const fn tf_fact(&self) -> f64 {
        self.tf_fact
    }

    #[must_use]
    pub const fn ext_style(&self) -> TermWeightExtensionStyle {
        self.ext_style
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.eval.borrow().is_some()
    }

    #[must_use]
    pub fn document_count(&self) -> Option<usize> {
        self.eval
            .borrow()
            .as_ref()
            .map(TfIdfEvalState::document_count)
    }

    fn ensure_init(&self, signature: &Signature) {
        if self.eval.borrow().is_some() {
            return;
        }

        let mut state = TfIdfEvalState::new(signature);
        for clause in self.axioms.iter() {
            if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
                tb_insert_clause_terms_normalized(
                    &mut state.eval_bank,
                    clause,
                    self.var_norm,
                    self.rel_terms,
                );
            } else {
                tfidf_documents_add_clause_to_state(&mut state, clause, self.var_norm);
            }
        }
        state.eval_freqs = tb_count_term_freqs(&state.eval_bank);

        *self.eval.borrow_mut() = Some(state);
    }

    fn term_weight(&self, term: &Term) -> f64 {
        let mut eval = self.eval.borrow_mut();
        let state = eval
            .as_mut()
            .unwrap_or_else(|| panic!("ConjectureTermTfIdfWeight eval bank must be initialized"));
        let norm = term_copy_normalize_vars(state.eval_bank.vars(), term, self.var_norm);
        let term_frequency = state
            .eval_bank
            .find_repr(&norm)
            .and_then(|repr| {
                state
                    .eval_freqs
                    .find(repr.entry_no())
                    .map(|entry| entry.val1)
            })
            .unwrap_or(0);
        let tf = (self.tf_fact * (i64_to_f64(term_frequency) - 1.0)) + 1.0;
        let code = prefix_compute_term_code(&norm);
        let prefix_match = state.document_index.match_code_prefix(&code);
        let df = if prefix_match.remains == 0 {
            state.document_index.prefix_ref_count(&code)
        } else {
            0
        };
        let idf = ((1.0 + usize_to_f64(state.document_count())) / (1.0 + usize_to_f64(df))).ln();
        let tfidf = tf * idf;
        1.0 / (1.0 + tfidf)
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors TfIdfWeightParamAlloc fields"
)]
pub fn tfidf_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    update_docs: i32,
    tf_fact: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> TfIdfWeightParam {
    TfIdfWeightParam::new(
        axioms,
        var_norm,
        rel_terms,
        update_docs,
        tf_fact,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureTermTfIdfWeightInit parameters without OCB"
)]
pub fn conjecture_term_tfidf_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    update_docs: i32,
    tf_fact: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<TfIdfWeightParam> {
    wfcb_alloc_with_bank(
        conjecture_term_tfidf_weight_wfcb_compute,
        conjecture_term_tfidf_weight_wfcb_compute_with_bank,
        prio_fun,
        tfidf_weight_exit,
        Some(tfidf_weight_param_alloc(
            axioms,
            var_norm,
            rel_terms,
            update_docs,
            tf_fact,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn conjecture_term_tfidf_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<TfIdfWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_norm = parse_var_norm_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let rel_terms = parse_related_term_set(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let update_docs = parse_c_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let tf_fact = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ext_style = parse_term_weight_extension_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_term_tfidf_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        update_docs,
        tf_fact,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if lazy evaluation-bank initialization fails or if normalized terms
/// violate the term-bank invariants expected by the C helper.
pub fn conjecture_term_tfidf_weight_compute(
    param: &mut TfIdfWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        tfidf_weight_extension,
        &*param,
    );
    let result = clause.term_ext_weight(&extension);
    if param.update_docs != 0 {
        let mut eval = param.eval.borrow_mut();
        let state = eval
            .as_mut()
            .unwrap_or_else(|| panic!("ConjectureTermTfIdfWeight eval bank must be initialized"));
        tfidf_documents_add_clause_to_state(state, clause, param.var_norm);
    }
    result
}

/// Computes C `ConjectureTermTfIdfWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
/// As in C, generated-document updates happen after term-extension scoring.
///
/// # Panics
///
/// Panics if TF-IDF initialization did not populate the evaluation bank while
/// generated-document updates are enabled.
#[must_use]
pub fn conjecture_term_tfidf_weight_compute_with_ocb(
    param: &mut TfIdfWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &mut Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms(ocb, bank);
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        tfidf_weight_extension,
        &*param,
    );
    let result = clause.term_ext_weight(&extension);
    if param.update_docs != 0 {
        let mut eval = param.eval.borrow_mut();
        let state = eval
            .as_mut()
            .unwrap_or_else(|| panic!("ConjectureTermTfIdfWeight eval bank must be initialized"));
        tfidf_documents_add_clause_to_state(state, clause, param.var_norm);
    }
    result
}

/// Computes C `ConjectureTermTfIdfWeightCompute` with bank-backed ordering
/// preparation.
///
/// As in C, generated-document updates happen after conditional marking and
/// term-extension scoring.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
///
/// # Panics
///
/// Panics if TF-IDF initialization did not populate the evaluation bank while
/// generated-document updates are enabled.
pub fn conjecture_term_tfidf_weight_compute_with_bank(
    param: &mut TfIdfWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(conjecture_term_tfidf_weight_compute(param, bank, clause))
}

fn conjecture_term_tfidf_weight_wfcb_compute(
    data: Option<&mut TfIdfWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_term_tfidf_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureTermTfIdfWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

fn conjecture_term_tfidf_weight_wfcb_compute_with_bank(
    data: Option<&mut TfIdfWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    conjecture_term_tfidf_weight_compute_with_bank(
        data.unwrap_or_else(|| {
            panic!("ConjectureTermTfIdfWeight WFCB requires initialized parameters")
        }),
        ocb,
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn tfidf_weight_extension(term: &Term, data: &&TfIdfWeightParam) -> f64 {
    data.term_weight(term)
}

fn tfidf_weight_exit(_data: TfIdfWeightParam) {}

fn tfidf_documents_add_clause_to_state(
    state: &mut TfIdfEvalState,
    clause: &Clause,
    var_norm: VarNormStyle,
) {
    for literal in clause.literals().as_slice() {
        tfidf_documents_add_subterms_to_state(state, literal.left(), var_norm);
        tfidf_documents_add_subterms_to_state(state, literal.right(), var_norm);
    }
}

fn tfidf_documents_add_subterms_to_state(
    state: &mut TfIdfEvalState,
    term: &Term,
    var_norm: VarNormStyle,
) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }
        tfidf_documents_add_term_to_state(state, &subterm, var_norm);
        stack.extend(subterm.argument_clones().into_iter().flatten());
    }
}

fn tfidf_documents_add_term_to_state(
    state: &mut TfIdfEvalState,
    term: &Term,
    var_norm: VarNormStyle,
) {
    let norm = term_copy_normalize_vars(state.eval_bank.vars(), term, var_norm);
    state.document_index.insert_term(&norm);
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_term_tfidf_weight_compute, conjecture_term_tfidf_weight_compute_with_ocb,
        conjecture_term_tfidf_weight_parse, tfidf_weight_param_alloc,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_ORIENTED, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::termweights::RelatedTermSet;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::VarNormStyle;
    use crate::terms::termtypes::Term;
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn unit_clause(bank: &mut TermBank, left: &str, right: &str, positive: bool) -> Clause {
        let left = parse_in_bank(bank, left);
        let right = parse_in_bank(bank, right);
        let literal = Eqn::alloc(left, right, bank, positive).unwrap();
        Clause::alloc(EqnList::from_vec(vec![literal]))
    }

    fn mixed_axioms(bank: &mut TermBank) -> ClauseSet {
        let mut conjecture = unit_clause(bank, "f(a)", "b", false);
        conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let document = unit_clause(bank, "f(a)", "d", false);
        ClauseSet::from_clauses([conjecture, document])
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn conjecture_tfidf_weight_compute_initializes_eval_bank_and_scores_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
            0,
            1.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(!param.is_initialized());
        assert_eq!(param.document_count(), None);
        let expected = 1.0 + (1.0 / (1.0 + (4.0_f64 / 2.0).ln()));

        assert_f64_bits_eq(
            conjecture_term_tfidf_weight_compute(&mut param, &bank, &target),
            expected,
        );
        assert!(param.is_initialized());
        assert_eq!(param.document_count(), Some(3));
    }

    #[test]
    fn tf_fact_zero_makes_missing_conjecture_frequency_use_idf_only() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let target = unit_clause(&mut bank, "d", "c", false);
        let mut param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
            0,
            0.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );
        let d_weight = 1.0 / (1.0 + (4.0_f64 / 2.0).ln());
        let c_weight = 1.0 / (1.0 + (4.0_f64 / 1.0).ln());

        assert_f64_bits_eq(
            conjecture_term_tfidf_weight_compute(&mut param, &bank, &target),
            d_weight + c_weight,
        );
    }

    #[test]
    fn update_docs_adds_generated_clause_after_current_scoring() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let target = unit_clause(&mut bank, "b", "c", false);
        let mut param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
            1,
            1.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );
        let first_expected = 1.0 + (1.0 / (1.0 + (4.0_f64 / 1.0).ln()));
        let second_expected = 1.0 + (1.0 / (1.0 + (6.0_f64 / 2.0).ln()));

        assert_f64_bits_eq(
            conjecture_term_tfidf_weight_compute(&mut param, &bank, &target),
            first_expected,
        );
        assert_eq!(param.document_count(), Some(5));
        assert_f64_bits_eq(
            conjecture_term_tfidf_weight_compute(&mut param, &bank, &target),
            second_expected,
        );
        assert_eq!(param.document_count(), Some(7));
    }

    #[test]
    fn conjecture_tfidf_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureTerms,
            1,
            1.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_term_tfidf_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut actual_param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureTerms,
            1,
            1.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let mut ocb = kbo_ocb(&bank);

        let actual = conjecture_term_tfidf_weight_compute_with_ocb(
            &mut actual_param,
            &mut ocb,
            &bank,
            &mut target,
        );

        assert_f64_bits_eq(actual, expected);
        assert_eq!(
            actual_param.document_count(),
            expected_param.document_count()
        );
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn conjecture_tfidf_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = tfidf_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureTerms,
            1,
            1.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_term_tfidf_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,-1,0,1,1.0,0,1.0,7.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_term_tfidf_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));
        let mut ocb = kbo_ocb(&bank);

        let actual = wfcb
            .compute_eval_with_bank(&mut ocb, &mut bank, &mut target)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_f64_bits_eq(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn conjecture_tfidf_weight_parse_wraps_wfcb_compute() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = mixed_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,1,0,1.0,0,1.0,1.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_term_tfidf_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));
        let expected = 1.0 + (1.0 / (1.0 + (4.0_f64 / 2.0).ln()));

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), expected);
        assert_eq!(scanner.current_token().literal(), "tail");
    }
}
