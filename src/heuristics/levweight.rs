use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::termweights::{
    collect_related_conjecture_terms, parse_c_int, parse_related_term_set,
    parse_term_weight_extension_style, parse_var_norm_style, RelatedTermSet,
};
use crate::heuristics::wfcb::{wfcb_alloc, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::Term;
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LevDistanceCosts {
    pub ins_cost: i32,
    pub del_cost: i32,
    pub ch_cost: i32,
}

impl LevDistanceCosts {
    #[must_use]
    pub const fn new(ins_cost: i32, del_cost: i32, ch_cost: i32) -> Self {
        Self {
            ins_cost,
            del_cost,
            ch_cost,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LevWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: LevDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vars: Option<VarBank>,
    codes: Option<Vec<Vec<FunCode>>>,
}

impl LevWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureLevDistanceWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        costs: LevDistanceCosts,
        ext_style: TermWeightExtensionStyle,
        max_term_multiplier: f64,
        max_literal_multiplier: f64,
        pos_multiplier: f64,
    ) -> Self {
        Self {
            axioms: axioms.clone(),
            var_norm,
            rel_terms,
            costs,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
            vars: None,
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
    pub const fn costs(&self) -> LevDistanceCosts {
        self.costs
    }

    #[must_use]
    pub const fn ext_style(&self) -> TermWeightExtensionStyle {
        self.ext_style
    }

    #[must_use]
    pub fn codes(&self) -> Option<&[Vec<FunCode>]> {
        self.codes.as_deref()
    }

    fn ensure_init(&mut self, signature: &Signature) {
        if self.codes.is_some() {
            return;
        }

        let vars = VarBank::new(signature.type_bank());
        let related = collect_related_conjecture_terms(
            &self.axioms,
            &vars,
            signature,
            self.var_norm,
            self.rel_terms,
        );
        let codes = related
            .iter()
            .map(lev_compute_term_code)
            .collect::<Vec<_>>();
        self.vars = Some(vars);
        self.codes = Some(codes);
    }

    fn term_weight(&self, term: &Term) -> f64 {
        let vars = self
            .vars
            .as_ref()
            .unwrap_or_else(|| panic!("ConjectureLevDistanceWeight variables must be initialized"));
        let codes = self
            .codes
            .as_deref()
            .unwrap_or_else(|| panic!("ConjectureLevDistanceWeight codes must be initialized"));
        let norm = term_copy_normalize_vars(vars, term, self.var_norm);
        let term_code = lev_compute_term_code(&norm);
        let mut minimum = f64::MAX;
        for conjecture_code in codes {
            minimum = minimum.min(lev_codes_distance(&term_code, conjecture_code, self.costs));
        }
        minimum
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors LevWeightParamAlloc fields"
)]
pub fn lev_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: LevDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> LevWeightParam {
    LevWeightParam::new(
        axioms,
        var_norm,
        rel_terms,
        costs,
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    )
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors ConjectureLevDistanceWeightInit parameters without OCB"
)]
pub fn conjecture_lev_distance_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: LevDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<LevWeightParam> {
    wfcb_alloc(
        conjecture_lev_distance_weight_wfcb_compute,
        prio_fun,
        lev_weight_exit,
        Some(lev_weight_param_alloc(
            axioms,
            var_norm,
            rel_terms,
            costs,
            ext_style,
            max_term_multiplier,
            max_literal_multiplier,
            pos_multiplier,
        )),
    )
}

pub fn conjecture_lev_distance_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<LevWeightParam>, Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let prio_fun = parse_prio_fun(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let var_norm = parse_var_norm_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let rel_terms = parse_related_term_set(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ins_cost = parse_c_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let del_cost = parse_c_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ch_cost = parse_c_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let ext_style = parse_term_weight_extension_style(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_term_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let max_literal_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let pos_multiplier = parse_float(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    Ok(conjecture_lev_distance_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        LevDistanceCosts::new(ins_cost, del_cost, ch_cost),
        ext_style,
        max_term_multiplier,
        max_literal_multiplier,
        pos_multiplier,
    ))
}

#[must_use]
/// # Panics
///
/// Panics if the lazy conjecture-code initialization fails, matching the C
/// WFCB invariant that compute is only called with initialized data.
pub fn conjecture_lev_distance_weight_compute(
    param: &mut LevWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        lev_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Extracts the f-code sequence produced by C `TermLRTraverseNext`.
///
/// # Panics
///
/// Panics if a traversed non-leaf term has an uninitialized argument, matching
/// the C traversal precondition that all argument slots contain valid terms.
#[must_use]
pub fn lev_compute_term_code(term: &Term) -> Vec<FunCode> {
    let mut code = Vec::new();
    let mut stack = vec![term.clone()];

    while let Some(current) = stack.pop() {
        code.push(current.f_code());
        if current.is_top_level_free_var() {
            continue;
        }

        let start = usize::from(current.is_lambda() || current.is_applied_db_var());
        for index in (start..current.arity()).rev() {
            let arg = current
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            stack.push(arg);
        }
    }

    code
}

/// Computes the C Levenshtein distance over term-code sequences.
///
/// # Panics
///
/// Panics if either code sequence length does not fit C's `unsigned int`
/// loop counters.
#[must_use]
pub fn lev_codes_distance(code1: &[FunCode], code2: &[FunCode], costs: LevDistanceCosts) -> f64 {
    let ins_cost = c_int_to_uint(costs.ins_cost);
    let del_cost = c_int_to_uint(costs.del_cost);
    let ch_cost = c_int_to_uint(costs.ch_cost);
    let s1_len = code1.len();
    let s2_len = code2.len();
    let mut column = vec![0_u32; s1_len + 1];

    for (index, value) in column.iter_mut().enumerate() {
        *value = usize_to_c_uint(index).wrapping_mul(del_cost);
    }
    for x in 1..=s2_len {
        column[0] = usize_to_c_uint(x).wrapping_mul(ins_cost);
        let mut last_diag = usize_to_c_uint(x - 1).wrapping_mul(ins_cost);
        for y in 1..=s1_len {
            let old_diag = column[y];
            let del = column[y].wrapping_add(del_cost);
            let ins = column[y - 1].wrapping_add(ins_cost);
            let ch = last_diag.wrapping_add(if code1[y - 1] == code2[x - 1] {
                0
            } else {
                ch_cost
            });
            column[y] = del.min(ins).min(ch);
            last_diag = old_diag;
        }
    }

    f64::from(column[s1_len])
}

/// Computes the C Levenshtein distance between two terms' LR traversal codes.
///
/// # Panics
///
/// Panics under the same conditions as [`lev_compute_term_code`] and
/// [`lev_codes_distance`].
#[must_use]
pub fn lev_term_distance(left: &Term, right: &Term, costs: LevDistanceCosts) -> f64 {
    let left_code = lev_compute_term_code(left);
    let right_code = lev_compute_term_code(right);
    lev_codes_distance(&left_code, &right_code, costs)
}

/// Scores `term` against precomputed conjecture term-code sequences.
///
/// # Panics
///
/// Panics under the same conditions as [`lev_compute_term_code`] and
/// [`lev_codes_distance`].
#[must_use]
pub fn lev_term_weight(
    term: &Term,
    conjecture_codes: &[Vec<FunCode>],
    costs: LevDistanceCosts,
) -> f64 {
    let term_code = lev_compute_term_code(term);
    let mut minimum = f64::MAX;
    for conj_code in conjecture_codes {
        minimum = minimum.min(lev_codes_distance(&term_code, conj_code, costs));
    }
    minimum
}

fn conjecture_lev_distance_weight_wfcb_compute(
    data: Option<&mut LevWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_lev_distance_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureLevDistanceWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn lev_weight_extension(term: &Term, data: &&LevWeightParam) -> f64 {
    data.term_weight(term)
}

fn lev_weight_exit(_data: LevWeightParam) {}

fn c_int_to_uint(value: i32) -> u32 {
    u32::from_ne_bytes(value.to_ne_bytes())
}

fn usize_to_c_uint(value: usize) -> u32 {
    u32::try_from(value).expect("C Levenshtein sequence length fits unsigned int")
}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_lev_distance_weight_compute, conjecture_lev_distance_weight_parse,
        lev_codes_distance, lev_compute_term_code, lev_term_distance, lev_term_weight,
        lev_weight_param_alloc, LevDistanceCosts,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_NEG_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::termweights::RelatedTermSet;
    use crate::inout::scanner::Scanner;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::VarNormStyle;
    use crate::terms::termtypes::{Term, TP_IS_DB_VAR};
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn costs(ins_cost: i32, del_cost: i32, ch_cost: i32) -> LevDistanceCosts {
        LevDistanceCosts::new(ins_cost, del_cost, ch_cost)
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
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

    #[test]
    fn lev_term_code_uses_c_left_to_right_preorder() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let g = Term::top_alloc(20, 1);
        g.set_argument(0, a);
        let h = Term::top_alloc(21, 1);
        h.set_argument(0, b);
        let root = binary(10, &g, &h);

        assert_eq!(lev_compute_term_code(&root), vec![10, 20, 1, 21, 2]);
    }

    #[test]
    fn lev_term_code_preserves_c_top_level_variable_skips() {
        let applied_free = binary(
            SIG_PHONY_APP_CODE,
            &Term::const_cell_alloc(-2),
            &Term::const_cell_alloc(8),
        );
        assert_eq!(
            lev_compute_term_code(&applied_free),
            vec![SIG_PHONY_APP_CODE]
        );

        let db_head = Term::const_cell_alloc(0);
        db_head.set_prop(TP_IS_DB_VAR);
        let applied_db = binary(SIG_PHONY_APP_CODE, &db_head, &Term::const_cell_alloc(9));
        assert_eq!(
            lev_compute_term_code(&applied_db),
            vec![SIG_PHONY_APP_CODE, 9]
        );

        let lambda = binary(
            SIG_DB_LAMBDA_CODE,
            &Term::const_cell_alloc(0),
            &Term::const_cell_alloc(10),
        );
        assert_eq!(lev_compute_term_code(&lambda), vec![SIG_DB_LAMBDA_CODE, 10]);
    }

    #[test]
    fn lev_codes_distance_matches_c_dynamic_program_shape() {
        assert_f64_bits_eq(
            lev_codes_distance(&[1, 2, 3], &[1, 2, 3], costs(2, 3, 5)),
            0.0,
        );
        assert_f64_bits_eq(
            lev_codes_distance(&[1, 2, 3], &[1, 4, 3], costs(3, 4, 2)),
            2.0,
        );
        assert_f64_bits_eq(lev_codes_distance(&[1, 2], &[1, 2, 3], costs(2, 3, 5)), 3.0);
        assert_f64_bits_eq(lev_codes_distance(&[1, 2, 3], &[1, 3], costs(2, 3, 5)), 2.0);
    }

    #[test]
    fn lev_codes_distance_preserves_unsigned_negative_cost_wrap() {
        assert_f64_bits_eq(
            lev_codes_distance(&[1], &[], costs(1, -1, 1)),
            4_294_967_295.0,
        );
    }

    #[test]
    fn lev_term_distance_uses_extracted_codes() {
        let left = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let right = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(3));

        assert_f64_bits_eq(lev_term_distance(&left, &right, costs(2, 3, 7)), 5.0);
    }

    #[test]
    fn lev_term_weight_returns_minimum_or_dbl_max() {
        let term = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let exact = lev_compute_term_code(&term);
        let close = vec![10, 1, 3];

        assert_f64_bits_eq(lev_term_weight(&term, &[close], costs(2, 3, 7)), 5.0);
        assert_f64_bits_eq(
            lev_term_weight(&term, &[vec![99], exact], costs(2, 3, 7)),
            0.0,
        );
        assert_f64_bits_eq(lev_term_weight(&term, &[], costs(2, 3, 7)), f64::MAX);
    }

    #[test]
    fn conjecture_lev_weight_compute_initializes_codes_and_scores_clause_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "c", true);
        let mut param = lev_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            costs(1, 1, 5),
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(param.codes().is_none());
        assert_f64_bits_eq(
            conjecture_lev_distance_weight_compute(&mut param, &bank, &target),
            2.0,
        );
        assert_eq!(param.codes().expect("codes should be initialized").len(), 2);
    }

    #[test]
    fn conjecture_lev_weight_parse_wraps_wfcb_compute() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "c", true);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0) tail", false).unwrap();
        let mut wfcb =
            conjecture_lev_distance_weight_parse(&mut scanner, &axioms).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), 2.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn conjecture_lev_weight_parse_rejects_unknown_extension_style() {
        let axioms = ClauseSet::new();
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,1,1,5,99,1.0,1.0,1.0)", false).unwrap();

        let Err(error) = conjecture_lev_distance_weight_parse(&mut scanner, &axioms) else {
            panic!("invalid extension style should fail");
        };

        assert!(error
            .to_string()
            .contains("unsupported term weight extension style"));
    }
}
