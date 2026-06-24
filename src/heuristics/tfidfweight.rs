use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prefixweight::{
    prefix_code_match_counts, prefix_code_ref_count, prefix_compute_term_code, PrefixToken,
};
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::termweights::{
    parse_c_int, parse_related_term_set, parse_term_weight_extension_style, parse_var_norm_style,
    tb_count_term_freqs, tb_insert_clause_terms_normalized, RelatedTermSet, TermFrequencyTree,
};
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::Term;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};
use std::cell::RefCell;

#[derive(Clone, Debug)]
struct TfIdfEvalState {
    eval_bank: TermBank,
    eval_freqs: TermFrequencyTree,
    document_terms: Vec<Term>,
    document_codes: Vec<Vec<PrefixToken>>,
}

impl TfIdfEvalState {
    fn new(signature: &Signature) -> Self {
        Self {
            eval_bank: TermBank::new(signature.clone())
                .unwrap_or_else(|err| panic!("ConjectureTermTfIdfWeight eval bank init: {err}")),
            eval_freqs: TermFrequencyTree::new(),
            document_terms: Vec::new(),
            document_codes: Vec::new(),
        }
    }

    fn document_count(&self) -> usize {
        debug_assert_eq!(self.document_terms.len(), self.document_codes.len());
        self.document_terms.len()
    }
}

#[derive(Clone, Debug)]
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
        let (_, remains) = prefix_code_match_counts(&code, &state.document_codes);
        let df = if remains == 0 {
            prefix_code_ref_count(&code, &state.document_codes)
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
    wfcb_alloc(
        conjecture_term_tfidf_weight_wfcb_compute,
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
    let code = prefix_compute_term_code(&norm);
    state.document_terms.push(norm);
    state.document_codes.push(code);
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
        conjecture_term_tfidf_weight_compute, conjecture_term_tfidf_weight_parse,
        tfidf_weight_param_alloc,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::termweights::RelatedTermSet;
    use crate::inout::scanner::Scanner;
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
