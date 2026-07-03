use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::fixdarrays::FixedDArray;
use crate::basics::numtrees::NumTree;
use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::basicparser::parse_int;
use crate::inout::scanner::{token_pos_rep, Scanner};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::{DerefType, Term, TP_PRED_POS, TP_TOP_POS};
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};
use std::cell::RefCell;
use std::fmt::Write as _;

pub const TERM_MAX_GENS: usize = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum RelatedTermSet {
    ConjectureTerms = 0,
    ConjectureSubterms = 1,
    ConjectureSubtermsTopGens = 2,
    ConjectureSubtermsAllGens = 3,
}

impl RelatedTermSet {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::ConjectureTerms),
            1 => Some(Self::ConjectureSubterms),
            2 => Some(Self::ConjectureSubtermsTopGens),
            3 => Some(Self::ConjectureSubtermsAllGens),
            _ => None,
        }
    }
}

pub type TermFrequencyTree = NumTree<i64, i64>;

#[derive(Clone, Debug)]
struct TermWeightEvalState {
    bank: TermBank,
    freqs: TermFrequencyTree,
}

#[derive(Clone, Debug)]
pub struct TermWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    eval: RefCell<Option<TermWeightEvalState>>,
}

impl TermWeightParam {
    #[must_use]
    #[expect(
        clippy::similar_names,
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureRelativeTermWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        vweight: i64,
        fweight: i64,
        cweight: i64,
        pweight: i64,
        conj_fweight: i64,
        conj_cweight: i64,
        conj_pweight: i64,
        ext_style: TermWeightExtensionStyle,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
    ) -> Self {
        Self {
            axioms: axioms.clone(),
            var_norm,
            rel_terms,
            vweight,
            fweight,
            cweight,
            pweight,
            conj_fweight,
            conj_cweight,
            conj_pweight,
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
    pub const fn ext_style(&self) -> TermWeightExtensionStyle {
        self.ext_style
    }

    #[must_use]
    pub const fn fweight(&self) -> i64 {
        self.fweight
    }

    #[must_use]
    pub const fn conj_fweight(&self) -> i64 {
        self.conj_fweight
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.eval.borrow().is_some()
    }

    fn ensure_init(&self, signature: &Signature) {
        if self.eval.borrow().is_some() {
            return;
        }

        let mut eval_bank = TermBank::new(signature.clone())
            .unwrap_or_else(|err| panic!("ConjectureRelativeTermWeight eval bank init: {err}"));
        for clause in self.axioms.iter() {
            if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
                tb_insert_clause_terms_normalized(
                    &mut eval_bank,
                    clause,
                    self.var_norm,
                    self.rel_terms,
                );
            }
        }
        let freqs = tb_count_term_freqs(&eval_bank);

        *self.eval.borrow_mut() = Some(TermWeightEvalState {
            bank: eval_bank,
            freqs,
        });
    }

    fn term_weight(&self, term: &Term) -> f64 {
        let mut eval = self.eval.borrow_mut();
        let state = eval.as_mut().unwrap_or_else(|| {
            panic!("ConjectureRelativeTermWeight eval bank must be initialized")
        });
        let repr = termweight_insert(&mut state.bank, term, self.var_norm);
        termweight_update_conjecture_freqs(&mut state.bank, &mut state.freqs, &repr, self.var_norm);

        if repr.is_free_var() {
            return c_long_to_double(self.vweight);
        }

        let freq = state
            .freqs
            .find(repr.entry_no())
            .map_or(0, |entry| entry.val1);
        if repr.is_const() {
            return c_long_to_double(if freq > 0 {
                self.conj_cweight
            } else {
                self.cweight
            });
        }
        if repr.query_prop(TP_PRED_POS) {
            c_long_to_double(if freq > 0 {
                self.conj_pweight
            } else {
                self.pweight
            })
        } else if freq > 0 {
            c_long_to_double(self.conj_fweight)
        } else {
            c_long_to_double(self.fweight)
        }
    }
}

#[must_use]
#[expect(
    clippy::similar_names,
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors TermWeightParamAlloc fields"
)]
pub fn term_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> TermWeightParam {
    TermWeightParam::new(
        axioms,
        var_norm,
        rel_terms,
        vweight,
        fweight,
        cweight,
        pweight,
        conj_fweight,
        conj_cweight,
        conj_pweight,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    )
}

#[must_use]
#[expect(
    clippy::similar_names,
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureRelativeTermWeightInit parameters without OCB"
)]
pub fn conjecture_relative_term_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    vweight: i64,
    fweight: i64,
    cweight: i64,
    pweight: i64,
    conj_fweight: i64,
    conj_cweight: i64,
    conj_pweight: i64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<TermWeightParam> {
    wfcb_alloc_with_bank(
        conjecture_relative_term_weight_wfcb_compute,
        conjecture_relative_term_weight_wfcb_compute_with_bank,
        prio_fun,
        term_weight_exit,
        Some(term_weight_param_alloc(
            axioms,
            var_norm,
            rel_terms,
            vweight,
            fweight,
            cweight,
            pweight,
            conj_fweight,
            conj_cweight,
            conj_pweight,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

#[expect(
    clippy::similar_names,
    reason = "C parser uses parallel function/constant/predicate weight fields"
)]
pub fn conjecture_relative_term_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<TermWeightParam>, Diagnostic> {
    scanner.accept_tok(crate::inout::scanner::TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let var_norm = parse_var_norm_style(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let rel_terms = parse_related_term_set(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let conj_multiplier = parse_float(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let raw_fweight = parse_c_int(scanner)?;
    let fweight = i64::from(raw_fweight);
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let raw_cweight = parse_c_int(scanner)?;
    let cweight = i64::from(raw_cweight);
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let raw_pweight = parse_c_int(scanner)?;
    let pweight = i64::from(raw_pweight);
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let vweight = i64::from(parse_c_int(scanner)?);
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let ext_style = parse_term_weight_extension_style(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(crate::inout::scanner::TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_relative_term_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        vweight,
        fweight,
        cweight,
        pweight,
        c_double_to_long(conj_multiplier * f64::from(raw_fweight)),
        c_double_to_long(conj_multiplier * f64::from(raw_cweight)),
        c_double_to_long(conj_multiplier * f64::from(raw_pweight)),
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if lazy evaluation-bank initialization fails, matching the C WFCB
/// invariant that compute is only called with initialized term/signature state.
pub fn conjecture_relative_term_weight_compute(
    param: &mut TermWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        relative_term_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureRelativeTermWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn conjecture_relative_term_weight_compute_with_ocb(
    param: &mut TermWeightParam,
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
        relative_term_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureRelativeTermWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn conjecture_relative_term_weight_compute_with_bank(
    param: &mut TermWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(conjecture_relative_term_weight_compute(param, bank, clause))
}

fn conjecture_relative_term_weight_wfcb_compute(
    data: Option<&mut TermWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_relative_term_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureRelativeTermWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

fn conjecture_relative_term_weight_wfcb_compute_with_bank(
    data: Option<&mut TermWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    conjecture_relative_term_weight_compute_with_bank(
        data.unwrap_or_else(|| {
            panic!("ConjectureRelativeTermWeight WFCB requires initialized parameters")
        }),
        ocb,
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn relative_term_weight_extension(term: &Term, data: &&TermWeightParam) -> f64 {
    data.term_weight(term)
}

fn term_weight_exit(_data: TermWeightParam) {}

#[must_use]
pub fn compute_subterms_generalizations(term: &Term, vars: &VarBank) -> PStack<Term> {
    let mut all = PStack::new();
    let mut term_vars = NumTree::<Vec<Term>, ()>::new();
    let mut fresh_var_code: FunCode = -2;

    let _gens = compute_subterms_generalizations_inner(
        term,
        vars,
        &mut all,
        &mut term_vars,
        &mut fresh_var_code,
    );

    all
}

#[must_use]
/// # Panics
///
/// Panics if a traversed compound term has an uninitialized argument, if the
/// signature f-count or symbol arity does not fit the Rust target size, or if
/// an occurred signature symbol has a negative arity. These match the C helper
/// assumptions that terms are fully initialized and occurred symbols have
/// ordinary top-cell arities.
pub fn compute_top_generalizations(term: &Term, vars: &VarBank, sig: &Signature) -> PStack<Term> {
    let occurs_len = usize::try_from(sig.f_count() + 1).expect("signature f-count fits in usize");
    let mut occurs = vec![false; occurs_len];
    let mut stack = vec![term.clone()];

    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() || subterm.is_const() {
            continue;
        }

        if let Ok(index) = usize::try_from(subterm.f_code()) {
            if let Some(slot) = occurs.get_mut(index) {
                *slot = true;
            }
        }

        for index in 1..subterm.arity() {
            let arg = subterm
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    let mut topgens = PStack::new();
    for code in 1..=sig.f_count() {
        let index = usize::try_from(code).expect("positive f-code fits in usize");
        if !occurs[index] {
            continue;
        }

        let arity = sig
            .find_arity(code)
            .and_then(|arity| usize::try_from(arity).ok())
            .expect("occurred symbol has non-negative arity");
        let topgen = Term::top_alloc(code, arity);
        let var_type = sig.type_bank().i_type();
        for arg_index in 0..arity {
            let offset = FunCode::try_from(arg_index + 1).expect("argument index fits f-code");
            topgen.set_argument(arg_index, vars.var_assert_alloc(-2 * offset, &var_type));
        }
        if sig.is_predicate(code) {
            topgen.set_prop(TP_PRED_POS);
            topgen.set_type(Some(sig.type_bank().bool_type()));
        } else {
            topgen.set_type(Some(sig.type_bank().i_type()));
        }
        topgens.push(topgen);
    }

    topgens
}

pub fn free_generalizations(gens: PStack<Term>) {
    drop(gens);
}

#[must_use]
pub fn tuple_init(cur: &mut FixedDArray) -> bool {
    cur.initialize(0);
    cur.size() > 0
}

/// Advances `cur` to the next C-style tuple under inclusive component maxima.
///
/// # Panics
///
/// Panics if `cur` and `max` have different sizes. The C helper assumes
/// matching fixed-array sizes.
#[must_use]
pub fn tuple_next(cur: &mut FixedDArray, max: &FixedDArray) -> bool {
    assert_eq!(cur.size(), max.size());
    if cur.size() == 0 {
        return false;
    }

    let mut increment_index = None;
    for index in (0..cur.size()).rev() {
        if cur.as_slice()[index] < max.as_slice()[index] {
            increment_index = Some(index);
            break;
        }
    }

    let Some(index) = increment_index else {
        return false;
    };

    cur.as_mut_slice()[index] += 1;
    for value in &mut cur.as_mut_slice()[index + 1..] {
        *value = 0;
    }
    true
}

#[must_use]
pub fn tuple_print_string(tuple: &FixedDArray) -> String {
    let mut result = "(".to_owned();
    for value in tuple.as_slice() {
        let write_result = write!(&mut result, "{value},");
        debug_assert!(write_result.is_ok());
    }
    result.push_str(")\n");
    result
}

/// # Panics
///
/// Panics if a traversed compound term has an uninitialized argument, matching
/// the C helper's valid-term precondition.
pub fn tb_inc_subterms_freqs(term: &Term, freqs: &mut TermFrequencyTree) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }

        let key = subterm.entry_no();
        if let Some(entry) = freqs.find_mut(key) {
            entry.val1 += 1;
        } else {
            freqs.store(key, 1, 1);
        }

        for index in 0..subterm.arity() {
            let arg = subterm
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }
}

#[must_use]
pub fn tb_count_term_freqs(bank: &TermBank) -> TermFrequencyTree {
    let mut freqs = TermFrequencyTree::new();
    for term in bank.stored_terms() {
        if term.query_prop(TP_TOP_POS) {
            tb_inc_subterms_freqs(&term, &mut freqs);
        }
    }
    freqs
}

/// Inserts one clause's related terms into a private evaluation bank after
/// applying the selected C variable-normalization style.
///
/// # Panics
///
/// Panics if term insertion or related-term construction violates the term-bank
/// invariants for valid clause terms. This matches the C helper's unchecked
/// `TBInsertClauseTermsNormalized` preconditions.
pub fn tb_insert_clause_terms_normalized(
    bank: &mut TermBank,
    clause: &Clause,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
) {
    for literal in clause.literals().as_slice() {
        match rel_terms {
            RelatedTermSet::ConjectureTerms => {
                termweight_insert(bank, literal.left(), var_norm);
                termweight_insert(bank, literal.right(), var_norm);
            }
            RelatedTermSet::ConjectureSubterms => {
                termweight_insert_subterms(bank, literal.left(), var_norm);
                termweight_insert_subterms(bank, literal.right(), var_norm);
            }
            RelatedTermSet::ConjectureSubtermsTopGens => {
                termweight_insert_subterms(bank, literal.left(), var_norm);
                termweight_insert_subterms(bank, literal.right(), var_norm);
                termweight_insert_topgens(bank, literal.left(), var_norm);
                termweight_insert_topgens(bank, literal.right(), var_norm);
            }
            RelatedTermSet::ConjectureSubtermsAllGens => {
                termweight_insert_subgens(bank, literal.left(), var_norm);
                termweight_insert_subgens(bank, literal.right(), var_norm);
            }
        }
    }
}

/// Collects normalized terms related to negated conjecture clauses.
///
/// Unlike the term-frequency based C helper, this preserves duplicates and
/// encounter order. That is the shape used by the Levenshtein/tree/structural
/// distance initializers before any strategy-specific deduplication.
///
/// # Panics
///
/// Panics if related subterm traversal or generalization construction hits an
/// uninitialized argument. This matches the C helper preconditions for valid
/// clause terms.
#[must_use]
pub fn collect_related_conjecture_terms(
    axioms: &ClauseSet,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
) -> Vec<Term> {
    let mut related = Vec::new();
    for clause in axioms.iter() {
        if clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE {
            collect_clause_related_terms(clause, vars, sig, var_norm, rel_terms, &mut related);
        }
    }
    related
}

fn collect_clause_related_terms(
    clause: &Clause,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    related: &mut Vec<Term>,
) {
    for literal in clause.literals().as_slice() {
        collect_term_related_terms(literal.left(), vars, sig, var_norm, rel_terms, related);
        collect_term_related_terms(literal.right(), vars, sig, var_norm, rel_terms, related);
    }
}

fn collect_term_related_terms(
    term: &Term,
    vars: &VarBank,
    sig: &Signature,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    related: &mut Vec<Term>,
) {
    match rel_terms {
        RelatedTermSet::ConjectureTerms => push_normalized_term(related, vars, term, var_norm),
        RelatedTermSet::ConjectureSubterms => {
            collect_normalized_subterms(term, vars, var_norm, related);
        }
        RelatedTermSet::ConjectureSubtermsTopGens => {
            collect_normalized_subterms(term, vars, var_norm, related);
            let topgens = compute_top_generalizations(term, vars, sig);
            for topgen in topgens.as_slice() {
                push_normalized_term(related, vars, topgen, var_norm);
            }
            free_generalizations(topgens);
        }
        RelatedTermSet::ConjectureSubtermsAllGens => {
            let subgens = compute_subterms_generalizations(term, vars);
            for subgen in subgens.as_slice() {
                push_normalized_term(related, vars, subgen, var_norm);
            }
            free_generalizations(subgens);
        }
    }
}

fn collect_normalized_subterms(
    term: &Term,
    vars: &VarBank,
    var_norm: VarNormStyle,
    related: &mut Vec<Term>,
) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }
        push_normalized_term(related, vars, &subterm, var_norm);
        stack.extend(subterm.argument_clones().into_iter().flatten());
    }
}

fn push_normalized_term(
    related: &mut Vec<Term>,
    vars: &VarBank,
    term: &Term,
    var_norm: VarNormStyle,
) {
    related.push(term_copy_normalize_vars(vars, term, var_norm));
}

fn termweight_insert(bank: &mut TermBank, term: &Term, var_norm: VarNormStyle) -> Term {
    let copy = term_copy_normalize_vars(bank.vars(), term, var_norm);
    let repr = bank
        .insert(&copy, DerefType::Never)
        .unwrap_or_else(|err| panic!("ConjectureRelativeTermWeight term insertion failed: {err}"));
    repr.set_prop(TP_TOP_POS);
    repr
}

fn termweight_insert_subterms(bank: &mut TermBank, term: &Term, var_norm: VarNormStyle) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }
        termweight_insert(bank, &subterm, var_norm);
        stack.extend(subterm.argument_clones().into_iter().flatten());
    }
}

fn termweight_insert_topgens(bank: &mut TermBank, term: &Term, var_norm: VarNormStyle) {
    let topgens = compute_top_generalizations(term, bank.vars(), bank.signature());
    for topgen in topgens.as_slice() {
        termweight_insert(bank, topgen, var_norm);
    }
    free_generalizations(topgens);
}

fn termweight_insert_subgens(bank: &mut TermBank, term: &Term, var_norm: VarNormStyle) {
    let subgens = compute_subterms_generalizations(term, bank.vars());
    for subgen in subgens.as_slice() {
        termweight_insert(bank, subgen, var_norm);
    }
    free_generalizations(subgens);
}

fn termweight_update_conjecture_freqs(
    bank: &mut TermBank,
    freqs: &mut TermFrequencyTree,
    term: &Term,
    var_norm: VarNormStyle,
) {
    let mut stack = vec![term.clone()];
    while let Some(subterm) = stack.pop() {
        if subterm.is_free_var() {
            continue;
        }
        let subnorm = term_copy_normalize_vars(bank.vars(), &subterm, var_norm);
        if let Some(subrepr) = bank.find_repr(&subnorm) {
            if let Some(cell) = freqs.find(subrepr.entry_no()) {
                if cell.val1 > 0 {
                    let val1 = cell.val1;
                    let val2 = cell.val2;
                    freqs.store(subterm.entry_no(), val1, val2);
                }
            }
        }

        stack.extend(subterm.argument_clones().into_iter().flatten());
    }
}

pub fn parse_var_norm_style(scanner: &mut Scanner) -> Result<VarNormStyle, Diagnostic> {
    let token = scanner.current_token().clone();
    let raw = parse_int(scanner)?;
    VarNormStyle::from_c_value(i64_to_i32(raw)).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{} unsupported variable normalization style {raw}",
                token_pos_rep(&token)
            ),
        )
    })
}

pub fn parse_related_term_set(scanner: &mut Scanner) -> Result<RelatedTermSet, Diagnostic> {
    let token = scanner.current_token().clone();
    let raw = parse_int(scanner)?;
    RelatedTermSet::from_c_value(i64_to_i32(raw)).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{} unsupported related term set {raw}",
                token_pos_rep(&token)
            ),
        )
    })
}

pub fn parse_term_weight_extension_style(
    scanner: &mut Scanner,
) -> Result<TermWeightExtensionStyle, Diagnostic> {
    let token = scanner.current_token().clone();
    let raw = parse_int(scanner)?;
    TermWeightExtensionStyle::from_c_value(i64_to_i32(raw)).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{} unsupported term weight extension style {raw}",
                token_pos_rep(&token)
            ),
        )
    })
}

pub fn parse_c_int(scanner: &mut Scanner) -> Result<i32, Diagnostic> {
    parse_int(scanner).map(i64_to_i32)
}

#[allow(clippy::cast_possible_truncation)]
fn i64_to_i32(value: i64) -> i32 {
    value as i32
}

#[allow(clippy::cast_possible_truncation)]
fn c_double_to_long(value: f64) -> i64 {
    value as i64
}

#[allow(clippy::cast_precision_loss)]
fn c_long_to_double(value: i64) -> f64 {
    value as f64
}

fn compute_subterms_generalizations_inner(
    term: &Term,
    vars: &VarBank,
    all: &mut PStack<Term>,
    term_vars: &mut NumTree<Vec<Term>, ()>,
    fresh_var_code: &mut FunCode,
) -> Vec<Term> {
    let mut gens = get_subterm_generalizing_vars(term, vars, term_vars, fresh_var_code);

    if term.is_any_var() {
        return gens;
    }

    if term.is_const() {
        let copy = term_top_copy_with_all_properties(term);
        gens.push(copy.clone());
        all.push(copy);
        return gens;
    }

    assert!(term.arity() > 0);
    let mut subterm_gens = Vec::with_capacity(term.arity());
    let mut max = FixedDArray::new(term.arity());
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let child_gens =
            compute_subterms_generalizations_inner(&arg, vars, all, term_vars, fresh_var_code);
        assert!(!child_gens.is_empty());
        max.as_mut_slice()[index] =
            i64::try_from(child_gens.len() - 1).expect("generalization count fits in i64");
        subterm_gens.push(child_gens);
    }

    let mut cur = FixedDArray::new(term.arity());
    let mut iter_counter = 0_usize;
    let mut is_current = tuple_init(&mut cur);
    while is_current {
        if iter_counter > TERM_MAX_GENS {
            break;
        }

        let copy = term_top_copy_with_all_properties(term);
        for (index, child_gens) in subterm_gens.iter().enumerate() {
            let gen_index =
                usize::try_from(cur.as_slice()[index]).expect("tuple component fits in usize");
            copy.set_argument(index, child_gens[gen_index].clone());
        }
        gens.push(copy.clone());
        all.push(copy);

        iter_counter += 1;
        is_current = tuple_next(&mut cur, &max);
    }

    gens
}

fn get_subterm_generalizing_vars(
    term: &Term,
    vars: &VarBank,
    term_vars: &mut NumTree<Vec<Term>, ()>,
    fresh_var_code: &mut FunCode,
) -> Vec<Term> {
    let fresh_var = vars.var_assert_alloc(*fresh_var_code, &vars.default_type());
    *fresh_var_code -= 2;
    let key = term.entry_no();
    if let Some(entry) = term_vars.find_mut(key) {
        entry.val1.push(fresh_var);
        return entry.val1.clone();
    }

    let gen_vars = vec![fresh_var];
    let inserted = term_vars.store(key, gen_vars.clone(), ());
    debug_assert!(inserted);
    gen_vars
}

fn term_top_copy_with_all_properties(term: &Term) -> Term {
    let copy = Term::top_alloc(term.f_code(), term.arity());
    copy.set_properties(term.properties());
    copy.set_type(term.type_());
    copy
}

#[cfg(test)]
mod tests {
    use super::{
        collect_related_conjecture_terms, compute_subterms_generalizations,
        compute_top_generalizations, conjecture_relative_term_weight_compute,
        conjecture_relative_term_weight_compute_with_ocb, conjecture_relative_term_weight_parse,
        free_generalizations, tb_count_term_freqs, tb_inc_subterms_freqs, term_weight_param_alloc,
        tuple_init, tuple_next, tuple_print_string, RelatedTermSet, TERM_MAX_GENS,
    };
    use crate::basics::fixdarrays::FixedDArray;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_ORIENTED, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::VarNormStyle;
    use crate::terms::termtypes::{DerefType, Term, TP_CHECK_FLAG, TP_PRED_POS, TP_TOP_POS};
    use crate::terms::termvars::VarBank;
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn array(values: &[i64]) -> FixedDArray {
        let mut array = FixedDArray::new(values.len());
        array.as_mut_slice().copy_from_slice(values);
        array
    }

    fn parse_simple(source: &str) -> (TermBank, Term) {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = bank.parse_term_simple(&mut scanner).unwrap();
        (bank, term)
    }

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

    fn negated_conjecture_axioms(bank: &mut TermBank) -> ClauseSet {
        let mut conjecture = unit_clause(bank, "f(a)", "b", false);
        conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        ClauseSet::from_clauses([conjecture])
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn term_names(bank: &TermBank, terms: &[Term]) -> Vec<String> {
        terms
            .iter()
            .map(|term| {
                bank.signature()
                    .find_name(term.f_code())
                    .unwrap_or("<var>")
                    .to_owned()
            })
            .collect()
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn constants_and_related_term_set_discriminants_match_c_header() {
        assert_eq!(TERM_MAX_GENS, 1000);
        assert_eq!(RelatedTermSet::ConjectureTerms as i32, 0);
        assert_eq!(RelatedTermSet::ConjectureSubterms as i32, 1);
        assert_eq!(RelatedTermSet::ConjectureSubtermsTopGens as i32, 2);
        assert_eq!(RelatedTermSet::ConjectureSubtermsAllGens as i32, 3);
        assert_eq!(
            RelatedTermSet::from_c_value(0),
            Some(RelatedTermSet::ConjectureTerms)
        );
        assert_eq!(
            RelatedTermSet::from_c_value(3),
            Some(RelatedTermSet::ConjectureSubtermsAllGens)
        );
        assert_eq!(RelatedTermSet::from_c_value(4), None);
    }

    #[test]
    fn related_conjecture_term_collection_preserves_c_subterm_order() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let left = parse_in_bank(&mut bank, "f(a,g(b))");
        let right = parse_in_bank(&mut bank, "h(c)");
        let literal = Eqn::alloc(left, right, &mut bank, false).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let ignored = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            parse_in_bank(&mut bank, "ignored"),
            parse_in_bank(&mut bank, "a"),
            &mut bank,
            false,
        )
        .unwrap()]));
        let axioms = ClauseSet::from_clauses([ignored, clause]);
        let vars = VarBank::new(bank.signature().type_bank());

        let top_terms = collect_related_conjecture_terms(
            &axioms,
            &vars,
            bank.signature(),
            VarNormStyle::None,
            RelatedTermSet::ConjectureTerms,
        );
        assert_eq!(term_names(&bank, &top_terms), vec!["f", "h"]);

        let subterms = collect_related_conjecture_terms(
            &axioms,
            &vars,
            bank.signature(),
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
        );
        assert_eq!(
            term_names(&bank, &subterms),
            vec!["f", "g", "b", "a", "h", "c"]
        );
    }

    #[test]
    fn tuple_helpers_follow_c_lexicographic_order_and_print_shape() {
        let mut cur = array(&[9, 9]);
        let max = array(&[1, 2]);

        assert!(tuple_init(&mut cur));
        let mut seen = vec![cur.as_slice().to_vec()];
        while tuple_next(&mut cur, &max) {
            seen.push(cur.as_slice().to_vec());
        }

        assert_eq!(
            seen,
            vec![
                vec![0, 0],
                vec![0, 1],
                vec![0, 2],
                vec![1, 0],
                vec![1, 1],
                vec![1, 2]
            ]
        );
        assert_eq!(tuple_print_string(&cur), "(1,2,)\n");

        let mut empty = FixedDArray::new(0);
        assert!(!tuple_init(&mut empty));
        assert!(!tuple_next(&mut empty, &FixedDArray::new(0)));
        assert_eq!(tuple_print_string(&empty), "()\n");
    }

    #[test]
    fn subterm_frequency_counter_skips_free_variables_and_counts_repeated_terms() {
        let (_bank, term) = parse_simple("f(a,X,g(a))");
        let a = term.argument(0).unwrap();
        let variable = term.argument(1).unwrap();
        let g = term.argument(2).unwrap();
        let mut freqs = super::TermFrequencyTree::new();

        tb_inc_subterms_freqs(&term, &mut freqs);

        assert_eq!(freqs.find(term.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(a.entry_no()).unwrap().val1, 2);
        assert!(freqs.find(variable.entry_no()).is_none());
        assert_eq!(freqs.find(g.entry_no()).unwrap().val1, 1);
    }

    #[test]
    fn bank_frequency_counter_scans_only_top_position_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut scanner = Scanner::from_user_string("f(a,g(a))", false).unwrap();
        let first = bank.parse_term_simple(&mut scanner).unwrap();
        let mut scanner = Scanner::from_user_string("h(a)", false).unwrap();
        let second = bank.parse_term_simple(&mut scanner).unwrap();
        first.set_prop(TP_TOP_POS);
        second.set_prop(TP_TOP_POS);
        let a = first.argument(0).unwrap();
        let g = first.argument(1).unwrap();
        g.set_prop(TP_CHECK_FLAG);

        let freqs = tb_count_term_freqs(&bank);

        assert_eq!(freqs.find(first.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(second.entry_no()).unwrap().val1, 1);
        assert_eq!(freqs.find(a.entry_no()).unwrap().val1, 3);
        assert_eq!(freqs.find(g.entry_no()).unwrap().val1, 1);
    }

    #[test]
    fn relative_term_weight_compute_initializes_eval_bank_and_scores_known_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut param = term_weight_param_alloc(
            &axioms,
            VarNormStyle::None,
            RelatedTermSet::ConjectureSubterms,
            7,
            10,
            2,
            20,
            100,
            20,
            200,
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(!param.is_initialized());
        assert_f64_bits_eq(
            conjecture_relative_term_weight_compute(&mut param, &bank, &target),
            102.0,
        );
        assert!(param.is_initialized());
    }

    #[test]
    fn relative_term_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = term_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_relative_term_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut actual_param = term_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let mut ocb = kbo_ocb(&bank);

        let actual = conjecture_relative_term_weight_compute_with_ocb(
            &mut actual_param,
            &mut ocb,
            &bank,
            &mut target,
        );

        assert_f64_bits_eq(actual, expected);
        assert!(target.query_prop(CP_IS_ORIENTED));
        assert!(target.literals().as_slice()[0].is_maximal());
    }

    #[test]
    fn relative_term_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = term_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            7,
            10,
            2,
            20,
            50,
            10,
            100,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_relative_term_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,5.0,10,2,20,7,0,1.0,7.0,1.0) tail", false)
                .unwrap();
        let mut wfcb = conjecture_relative_term_weight_parse(&mut scanner, &axioms)
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
    fn relative_term_weight_parse_wraps_wfcb_compute_and_scales_conjecture_weights() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,1,5.0,10,2,20,7,0,1.0,1.0,1.0) tail", false)
                .unwrap();
        let mut wfcb = conjecture_relative_term_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), 52.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn top_generalizations_preserve_argument_zero_skip_quirk() {
        let (bank, term) = parse_simple("f(g(a),h(b))");
        let gens = compute_top_generalizations(&term, bank.vars(), bank.signature());
        let f_code = bank.signature().find_f_code("f");
        let h_code = bank.signature().find_f_code("h");
        let g_code = bank.signature().find_f_code("g");
        let seen = gens.as_slice().iter().map(Term::f_code).collect::<Vec<_>>();

        assert!(seen.contains(&f_code));
        assert!(seen.contains(&h_code));
        assert!(!seen.contains(&g_code));
        assert!(gens.as_slice().iter().all(|gen| gen
            .argument_clones()
            .into_iter()
            .flatten()
            .all(|arg| arg.is_free_var())));
    }

    #[test]
    fn top_generalizations_assign_predicate_type_and_property() {
        let mut sig = Signature::new(TypeBank::new());
        let individual = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let pred = sig.insert_id("p", 1, false);
        sig.declare_final_type(pred, alloc_arrow_type(vec![individual, bool_type.clone()]))
            .unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let arg_code = bank.signature_mut().insert_id("a", 0, false);
        let individual = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(arg_code, individual)
            .unwrap();
        let arg = bank.create_const_term(arg_code).unwrap();
        let term = Term::top_alloc(pred, 1);
        term.set_argument(0, arg);
        let term = bank.insert(&term, DerefType::Never).unwrap();

        let gens = compute_top_generalizations(&term, bank.vars(), bank.signature());
        let gen = gens
            .as_slice()
            .iter()
            .find(|gen| gen.f_code() == pred)
            .unwrap();

        assert!(gen.query_prop(TP_PRED_POS));
        assert_eq!(gen.type_(), Some(bool_type));
    }

    #[test]
    fn subterm_generalizations_allocate_variables_per_repeated_entry_visit() {
        let (bank, term) = parse_simple("f(a,a)");
        let gens = compute_subterms_generalizations(&term, bank.vars());
        let f_code = bank.signature().find_f_code("f");
        let a_code = bank.signature().find_f_code("a");
        let constants = gens
            .as_slice()
            .iter()
            .filter(|gen| gen.f_code() == a_code)
            .count();
        let f_gens = gens
            .as_slice()
            .iter()
            .filter(|gen| gen.f_code() == f_code)
            .collect::<Vec<_>>();

        assert_eq!(constants, 2);
        assert_eq!(f_gens.len(), 6);
        assert!(f_gens.iter().any(|gen| {
            gen.argument(0).unwrap().is_free_var() && gen.argument(1).unwrap().f_code() == a_code
        }));
        free_generalizations(gens);
    }
}
