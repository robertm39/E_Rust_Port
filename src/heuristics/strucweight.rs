use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
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
use crate::terms::termfunc::term_weight_compute;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::Term;
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrucDistanceParams {
    var_mismatch: f64,
    sym_mismatch: f64,
    inst_factor: f64,
    gen_factor: f64,
}

impl StrucDistanceParams {
    #[must_use]
    pub const fn new(
        var_mismatch: f64,
        sym_mismatch: f64,
        inst_factor: f64,
        gen_factor: f64,
    ) -> Self {
        Self {
            var_mismatch,
            sym_mismatch,
            inst_factor,
            gen_factor,
        }
    }

    #[must_use]
    pub const fn var_mismatch(self) -> f64 {
        self.var_mismatch
    }

    #[must_use]
    pub const fn sym_mismatch(self) -> f64 {
        self.sym_mismatch
    }

    #[must_use]
    pub const fn inst_factor(self) -> f64 {
        self.inst_factor
    }

    #[must_use]
    pub const fn gen_factor(self) -> f64 {
        self.gen_factor
    }
}

#[must_use]
pub const fn struc_distance_init(
    var_mismatch: f64,
    sym_mismatch: f64,
    inst_factor: f64,
    gen_factor: f64,
) -> StrucDistanceParams {
    StrucDistanceParams::new(var_mismatch, sym_mismatch, inst_factor, gen_factor)
}

#[derive(Clone, Debug)]
pub struct StrucWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    distance: StrucDistanceParams,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vars: Option<VarBank>,
    terms: Option<Vec<Term>>,
}

impl StrucWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureStrucDistanceWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        distance: StrucDistanceParams,
        ext_style: TermWeightExtensionStyle,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
    ) -> Self {
        Self {
            axioms: axioms.clone(),
            var_norm,
            rel_terms,
            distance,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vars: None,
            terms: None,
        }
    }

    #[must_use]
    pub const fn distance(&self) -> StrucDistanceParams {
        self.distance
    }

    #[must_use]
    pub const fn rel_terms(&self) -> RelatedTermSet {
        self.rel_terms
    }

    #[must_use]
    pub fn terms(&self) -> Option<&[Term]> {
        self.terms.as_deref()
    }

    fn ensure_init(&mut self, signature: &Signature) {
        if self.terms.is_some() {
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
        self.vars = Some(vars);
        self.terms = Some(terms);
    }

    fn term_weight(&self, term: &Term) -> f64 {
        let vars = self.vars.as_ref().unwrap_or_else(|| {
            panic!("ConjectureStrucDistanceWeight variables must be initialized")
        });
        let terms = self
            .terms
            .as_deref()
            .unwrap_or_else(|| panic!("ConjectureStrucDistanceWeight terms must be initialized"));
        let norm = term_copy_normalize_vars(vars, term, self.var_norm);
        struc_term_weight(&norm, terms, &self.distance)
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors StrucWeightParamAlloc fields"
)]
pub fn struc_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    distance: StrucDistanceParams,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> StrucWeightParam {
    StrucWeightParam::new(
        axioms,
        var_norm,
        rel_terms,
        distance,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureStrucDistanceWeightInit parameters without OCB"
)]
pub fn conjecture_struc_distance_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    distance: StrucDistanceParams,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<StrucWeightParam> {
    wfcb_alloc_with_bank(
        conjecture_struc_distance_weight_wfcb_compute,
        conjecture_struc_distance_weight_wfcb_compute_with_bank,
        prio_fun,
        struc_weight_exit,
        Some(struc_weight_param_alloc(
            axioms,
            var_norm,
            rel_terms,
            distance,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn conjecture_struc_distance_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<StrucWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_norm = parse_var_norm_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let rel_terms = parse_related_term_set(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_mismatch = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let sym_mismatch = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let inst_factor = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let gen_factor = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ext_style = parse_term_weight_extension_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_struc_distance_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        struc_distance_init(var_mismatch, sym_mismatch, inst_factor, gen_factor),
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if the lazy conjecture-term initialization fails, matching the C
/// WFCB invariant that compute is only called with initialized data.
pub fn conjecture_struc_distance_weight_compute(
    param: &mut StrucWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        struc_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureStrucDistanceWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn conjecture_struc_distance_weight_compute_with_ocb(
    param: &mut StrucWeightParam,
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
        struc_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureStrucDistanceWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn conjecture_struc_distance_weight_compute_with_bank(
    param: &mut StrucWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(conjecture_struc_distance_weight_compute(
        param, bank, clause,
    ))
}

/// Computes C `strc_terms_distance` for already-normalized terms.
///
/// # Panics
///
/// Panics if the C fall-through recursive case needs an argument that is not
/// initialized on either term. This matches the C helper's unchecked argument
/// access when top symbols have the same arity or the same f-code.
#[must_use]
pub fn struc_terms_distance(left: &Term, right: &Term, param: &StrucDistanceParams) -> f64 {
    if left.is_free_var() {
        if right.is_free_var() {
            return if left.f_code() == right.f_code() {
                0.0
            } else {
                (param.inst_factor + param.gen_factor).min(param.var_mismatch)
            };
        }
        return param.inst_factor * term_c_weight(right);
    }

    if right.is_free_var() {
        return param.gen_factor * term_c_weight(left);
    }

    if left.f_code() != right.f_code() && left.arity() != right.arity() {
        return param.gen_factor * term_c_weight(left) + param.inst_factor * term_c_weight(right);
    }

    let mut arg_distance = 0.0;
    for index in 0..left.arity() {
        let left_arg = left
            .argument(index)
            .unwrap_or_else(|| panic!("left term argument {index} is uninitialized"));
        let right_arg = right
            .argument(index)
            .unwrap_or_else(|| panic!("right term argument {index} is uninitialized"));
        arg_distance += struc_terms_distance(&left_arg, &right_arg, param);
    }

    let geninst = param.gen_factor * term_c_weight(left) + param.inst_factor * term_c_weight(right);
    let factor = if left.f_code() == right.f_code() {
        1.0
    } else {
        param.sym_mismatch
    };
    (factor * arg_distance).min(geninst)
}

/// Scores `term` against normalized conjecture terms using structural distance.
///
/// # Panics
///
/// Panics under the same conditions as [`struc_terms_distance`].
#[must_use]
pub fn struc_term_weight(
    term: &Term,
    conjecture_terms: &[Term],
    param: &StrucDistanceParams,
) -> f64 {
    let mut minimum = f64::MAX;
    for conjecture in conjecture_terms {
        minimum = minimum.min(struc_terms_distance(term, conjecture, param));
    }
    minimum
}

#[allow(clippy::cast_precision_loss)]
fn term_c_weight(term: &Term) -> f64 {
    term_weight_compute(term, 1, 1) as f64
}

fn conjecture_struc_distance_weight_wfcb_compute(
    data: Option<&mut StrucWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_struc_distance_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureStrucDistanceWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

fn conjecture_struc_distance_weight_wfcb_compute_with_bank(
    data: Option<&mut StrucWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    conjecture_struc_distance_weight_compute_with_bank(
        data.unwrap_or_else(|| {
            panic!("ConjectureStrucDistanceWeight WFCB requires initialized parameters")
        }),
        ocb,
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn struc_weight_extension(term: &Term, data: &&StrucWeightParam) -> f64 {
    data.term_weight(term)
}

fn struc_weight_exit(_data: StrucWeightParam) {}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_struc_distance_weight_compute,
        conjecture_struc_distance_weight_compute_with_ocb, conjecture_struc_distance_weight_parse,
        struc_distance_init, struc_term_weight, struc_terms_distance, struc_weight_param_alloc,
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
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::VarNormStyle;
    use crate::terms::termtypes::Term;
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
    }

    fn params() -> super::StrucDistanceParams {
        struc_distance_init(5.0, 10.0, 2.0, 3.0)
    }

    fn unary(code: FunCode, arg: &Term) -> Term {
        let term = Term::top_alloc(code, 1);
        term.set_argument(0, arg.clone());
        term
    }

    fn binary(code: FunCode, left: &Term, right: &Term) -> Term {
        let term = Term::top_alloc(code, 2);
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        term
    }

    fn parse_in_bank(bank: &mut TermBank, source: &str) -> Term {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        bank.parse_term_simple(&mut scanner).unwrap()
    }

    fn clause(bank: &mut TermBank, left: &str, right: &str, positive: bool) -> Clause {
        let left = parse_in_bank(bank, left);
        let right = parse_in_bank(bank, right);
        Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            left, right, bank, positive,
        )
        .unwrap()]))
    }

    fn negated_conjecture_axioms(bank: &mut TermBank) -> ClauseSet {
        let mut clause = clause(bank, "f(a)", "b", false);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        ClauseSet::from_clauses([clause])
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
    fn init_preserves_parameters() {
        let param = params();

        assert_f64_bits_eq(param.var_mismatch(), 5.0);
        assert_f64_bits_eq(param.sym_mismatch(), 10.0);
        assert_f64_bits_eq(param.inst_factor(), 2.0);
        assert_f64_bits_eq(param.gen_factor(), 3.0);
    }

    #[test]
    fn variable_cases_match_c_formula() {
        let x = Term::const_cell_alloc(-2);
        let x_again = Term::const_cell_alloc(-2);
        let y = Term::const_cell_alloc(-4);
        let fa = unary(10, &Term::const_cell_alloc(1));

        assert_f64_bits_eq(struc_terms_distance(&x, &x_again, &params()), 0.0);
        assert_f64_bits_eq(struc_terms_distance(&x, &y, &params()), 5.0);
        assert_f64_bits_eq(struc_terms_distance(&x, &fa, &params()), 4.0);
        assert_f64_bits_eq(struc_terms_distance(&fa, &x, &params()), 6.0);
    }

    #[test]
    fn symbol_and_arity_fallback_matches_c_condition() {
        let fa = unary(10, &Term::const_cell_alloc(1));
        let gab = binary(11, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_terms_distance(&fa, &gab, &params()), 12.0);
    }

    #[test]
    fn different_same_arity_symbols_can_have_zero_distance() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let fa = unary(10, &a);
        let gb = unary(11, &b);

        assert_f64_bits_eq(struc_terms_distance(&a, &b, &params()), 0.0);
        assert_f64_bits_eq(struc_terms_distance(&fa, &gb, &params()), 0.0);
    }

    #[test]
    fn same_symbol_extra_right_arguments_are_ignored_by_left_arity_loop() {
        let left = unary(10, &Term::const_cell_alloc(1));
        let right = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_terms_distance(&left, &right, &params()), 0.0);
    }

    #[test]
    #[should_panic(expected = "right term argument 1 is uninitialized")]
    fn same_symbol_missing_right_argument_panics_like_unchecked_c_access() {
        let left = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let right = unary(10, &Term::const_cell_alloc(1));

        let _ = struc_terms_distance(&left, &right, &params());
    }

    #[test]
    fn term_weight_returns_minimum_or_dbl_max() {
        let term = unary(10, &Term::const_cell_alloc(1));
        let exact = unary(10, &Term::const_cell_alloc(1));
        let fallback = binary(11, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));

        assert_f64_bits_eq(struc_term_weight(&term, &[fallback], &params()), 12.0);
        assert_f64_bits_eq(
            struc_term_weight(&term, &[Term::const_cell_alloc(-2), exact], &params()),
            0.0,
        );
        assert_f64_bits_eq(struc_term_weight(&term, &[], &params()), f64::MAX);
    }

    #[test]
    fn conjecture_struc_weight_compute_initializes_terms_and_scores_clause_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "h(c,d)", true);
        let mut param = struc_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            params(),
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(param.terms().is_none());
        assert_f64_bits_eq(
            conjecture_struc_distance_weight_compute(&mut param, &bank, &target),
            11.0,
        );
        assert_eq!(param.terms().expect("terms should be initialized").len(), 2);
    }

    #[test]
    fn conjecture_struc_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = struc_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            params(),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_struc_distance_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut actual_param = struc_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            params(),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let mut ocb = kbo_ocb(&bank);

        let actual = conjecture_struc_distance_weight_compute_with_ocb(
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
    fn conjecture_struc_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = struc_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            params(),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_struc_distance_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,5.0,10.0,2.0,3.0,0,1.0,7.0,1.0) tail", false)
                .unwrap();
        let mut wfcb = conjecture_struc_distance_weight_parse(&mut scanner, &axioms)
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
    fn conjecture_struc_weight_parse_wraps_wfcb_compute() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "h(c,d)", true);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,5.0,10.0,2.0,3.0,0,1.0,1.0,1.0) tail", false)
                .unwrap();
        let mut wfcb = conjecture_struc_distance_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), 11.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }
}
