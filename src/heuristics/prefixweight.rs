use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
pub use crate::clauses::pdtrees::{
    prefix_code_match_counts, prefix_code_ref_count, prefix_compute_term_code, prefix_match_counts,
    PdTree, PrefixToken,
};
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::termweights::{
    collect_related_conjecture_terms, parse_related_term_set, parse_term_weight_extension_style,
    parse_var_norm_style, RelatedTermSet,
};
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::VarNormStyle;
use crate::terms::termtypes::Term;
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};

#[derive(Clone, Debug)]
pub struct PrefixWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    match_weight: f64,
    miss_weight: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vars: Option<VarBank>,
    terms: Option<PdTree>,
    codes: Option<Vec<Vec<PrefixToken>>>,
}

impl PrefixWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureTermPrefixWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        match_weight: f64,
        miss_weight: f64,
        ext_style: TermWeightExtensionStyle,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
    ) -> Self {
        Self {
            axioms: axioms.clone(),
            var_norm,
            rel_terms,
            match_weight,
            miss_weight,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vars: None,
            terms: None,
            codes: None,
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
    pub fn codes(&self) -> Option<&[Vec<PrefixToken>]> {
        self.codes.as_deref()
    }

    fn ensure_init(&mut self, signature: &Signature) {
        if self.codes.is_some() {
            return;
        }

        let vars = VarBank::new(signature.type_bank());
        let terms = collect_related_conjecture_terms(
            &self.axioms,
            &vars,
            signature,
            self.var_norm,
            self.rel_terms,
        );
        let codes = terms
            .iter()
            .map(prefix_compute_term_code)
            .collect::<Vec<_>>();
        let mut tree = PdTree::new();
        for term in &terms {
            tree.insert_term(term);
        }

        self.vars = Some(vars);
        self.terms = Some(tree);
        self.codes = Some(codes);
    }

    fn term_weight(&self, term: &Term) -> f64 {
        let terms = self
            .terms
            .as_ref()
            .unwrap_or_else(|| panic!("ConjectureTermPrefixWeight terms must be initialized"));
        let prefix_match = terms.match_prefix(term);
        (usize_to_f64(prefix_match.matched) * self.match_weight)
            + (usize_to_f64(prefix_match.remains) * self.miss_weight)
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors PrefixWeightParamAlloc fields"
)]
pub fn prefix_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    match_weight: f64,
    miss_weight: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> PrefixWeightParam {
    PrefixWeightParam::new(
        axioms,
        var_norm,
        rel_terms,
        match_weight,
        miss_weight,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureTermPrefixWeightInit parameters without OCB"
)]
pub fn conjecture_term_prefix_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    match_weight: f64,
    miss_weight: f64,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<PrefixWeightParam> {
    wfcb_alloc_with_bank(
        conjecture_term_prefix_weight_wfcb_compute,
        conjecture_term_prefix_weight_wfcb_compute_with_bank,
        prio_fun,
        prefix_weight_exit,
        Some(prefix_weight_param_alloc(
            axioms,
            var_norm,
            rel_terms,
            match_weight,
            miss_weight,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn conjecture_term_prefix_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<PrefixWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_norm = parse_var_norm_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let rel_terms = parse_related_term_set(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let match_weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let miss_weight = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ext_style = parse_term_weight_extension_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_term_prefix_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        match_weight,
        miss_weight,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if the lazy conjecture-prefix initialization fails, matching the C
/// WFCB invariant that compute is only called with initialized data.
pub fn conjecture_term_prefix_weight_compute(
    param: &mut PrefixWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        prefix_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureTermPrefixWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn conjecture_term_prefix_weight_compute_with_ocb(
    param: &mut PrefixWeightParam,
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
        prefix_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureTermPrefixWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn conjecture_term_prefix_weight_compute_with_bank(
    param: &mut PrefixWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(conjecture_term_prefix_weight_compute(param, bank, clause))
}

#[must_use]
pub fn prefix_term_weight(
    term: &Term,
    prefixes: &[Vec<PrefixToken>],
    match_weight: f64,
    miss_weight: f64,
) -> f64 {
    let (matches, misses) = prefix_match_counts(term, prefixes);
    (usize_to_f64(matches) * match_weight) + (usize_to_f64(misses) * miss_weight)
}

fn conjecture_term_prefix_weight_wfcb_compute(
    data: Option<&mut PrefixWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_term_prefix_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureTermPrefixWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

fn conjecture_term_prefix_weight_wfcb_compute_with_bank(
    data: Option<&mut PrefixWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    conjecture_term_prefix_weight_compute_with_bank(
        data.unwrap_or_else(|| {
            panic!("ConjectureTermPrefixWeight WFCB requires initialized parameters")
        }),
        ocb,
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn prefix_weight_extension(term: &Term, data: &&PrefixWeightParam) -> f64 {
    data.term_weight(term)
}

fn prefix_weight_exit(_data: PrefixWeightParam) {}

#[allow(clippy::cast_precision_loss)]
fn usize_to_f64(value: usize) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_term_prefix_weight_compute, conjecture_term_prefix_weight_compute_with_ocb,
        conjecture_term_prefix_weight_parse, prefix_compute_term_code, prefix_match_counts,
        prefix_term_weight, prefix_weight_param_alloc, PrefixToken,
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
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> crate::terms::termtypes::Term {
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

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    #[test]
    fn term_code_uses_left_right_traversal_f_codes_for_first_order_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let term = parse_in_bank(&mut bank, "f(a,g(b))");
        let code = prefix_compute_term_code(&term);

        assert_eq!(
            code,
            vec![
                PrefixToken::Fun(bank.signature().find_f_code("f")),
                PrefixToken::Fun(bank.signature().find_f_code("a")),
                PrefixToken::Fun(bank.signature().find_f_code("g")),
                PrefixToken::Fun(bank.signature().find_f_code("b")),
            ]
        );
    }

    #[test]
    fn match_counts_follow_pdtree_path_prefix_not_stored_term_prefixes_only() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let stored = parse_in_bank(&mut bank, "f(a,b)");
        let candidate = parse_in_bank(&mut bank, "f(a)");
        let stored_codes = vec![prefix_compute_term_code(&stored)];

        assert_eq!(prefix_match_counts(&candidate, &stored_codes), (2, 0));
        assert_f64_bits_eq(prefix_term_weight(&candidate, &stored_codes, 0.5, 5.0), 1.0);
    }

    #[test]
    fn conjecture_prefix_weight_compute_initializes_terms_and_scores_clause_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut param = prefix_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureSubterms,
            0.5,
            5.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(param.codes().is_none());
        assert_f64_bits_eq(
            conjecture_term_prefix_weight_compute(&mut param, &bank, &target),
            6.0,
        );
        assert_eq!(param.codes().expect("codes should be initialized").len(), 3);
    }

    #[test]
    fn conjecture_prefix_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = prefix_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            0.5,
            5.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_term_prefix_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut actual_param = prefix_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            0.5,
            5.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let mut ocb = kbo_ocb(&bank);

        let actual = conjecture_term_prefix_weight_compute_with_ocb(
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
    fn conjecture_prefix_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = unit_clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = prefix_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            0.5,
            5.0,
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_term_prefix_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,0.5,5.0,0,1.0,7.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_term_prefix_weight_parse(&mut scanner, &axioms)
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
    fn conjecture_prefix_weight_parse_wraps_wfcb_compute() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = unit_clause(&mut bank, "f(a)", "c", false);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,1,0.5,5.0,0,1.0,1.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_term_prefix_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), 6.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }
}
