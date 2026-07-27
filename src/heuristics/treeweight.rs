use crate::basics::error::Diagnostic;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::prio_funs::parse_prio_fun;
use crate::heuristics::termweights::{
    collect_related_conjecture_terms, parse_c_int, parse_related_term_set,
    parse_term_weight_extension_style, parse_var_norm_style, RelatedTermSet,
};
use crate::heuristics::wfcb::{wfcb_alloc_with_bank, ClausePrioFun, Wfcb};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_weight;
use crate::terms::termfunc::{term_copy_normalize_vars, VarNormStyle};
use crate::terms::termtypes::Term;
use crate::terms::termvars::VarBank;
use crate::terms::termweightext::{TermWeightExtension, TermWeightExtensionStyle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreeDistanceCosts {
    pub ins_cost: i64,
    pub del_cost: i64,
    pub ch_cost: i64,
}

impl TreeDistanceCosts {
    #[must_use]
    pub const fn new(ins_cost: i64, del_cost: i64, ch_cost: i64) -> Self {
        Self {
            ins_cost,
            del_cost,
            ch_cost,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TreeWeightParam {
    axioms: ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: TreeDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
    vars: Option<VarBank>,
    terms: Option<Vec<Term>>,
}

impl TreeWeightParam {
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "C-compatible parameter cell mirrors ConjectureTreeDistanceWeightInit"
    )]
    pub fn new(
        axioms: &ClauseSet,
        var_norm: VarNormStyle,
        rel_terms: RelatedTermSet,
        costs: TreeDistanceCosts,
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
            terms: None,
        }
    }

    #[must_use]
    pub const fn costs(&self) -> TreeDistanceCosts {
        self.costs
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
            panic!("ConjectureTreeDistanceWeight variables must be initialized")
        });
        let terms = self
            .terms
            .as_deref()
            .unwrap_or_else(|| panic!("ConjectureTreeDistanceWeight terms must be initialized"));
        let norm = term_copy_normalize_vars(vars, term, self.var_norm);
        ted_term_weight(&norm, terms, self.costs)
    }
}

#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper mirrors TreeWeightParamAlloc fields"
)]
pub fn tree_weight_param_alloc(
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: TreeDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> TreeWeightParam {
    TreeWeightParam::new(
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
    reason = "C-compatible helper mirrors ConjectureTreeDistanceWeightInit parameters without OCB"
)]
pub fn conjecture_tree_distance_weight_init(
    prio_fun: ClausePrioFun,
    axioms: &ClauseSet,
    var_norm: VarNormStyle,
    rel_terms: RelatedTermSet,
    costs: TreeDistanceCosts,
    ext_style: TermWeightExtensionStyle,
    max_term_multiplier: f64,
    max_literal_multiplier: f64,
    pos_multiplier: f64,
) -> Wfcb<TreeWeightParam> {
    wfcb_alloc_with_bank(
        conjecture_tree_distance_weight_wfcb_compute,
        conjecture_tree_distance_weight_wfcb_compute_with_bank,
        prio_fun,
        tree_weight_exit,
        Some(tree_weight_param_alloc(
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

pub fn conjecture_tree_distance_weight_parse(
    scanner: &mut Scanner,
    axioms: &ClauseSet,
) -> Result<Wfcb<TreeWeightParam>, Diagnostic> {
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

    Ok(conjecture_tree_distance_weight_init(
        prio_fun,
        axioms,
        var_norm,
        rel_terms,
        TreeDistanceCosts::new(i64::from(ins_cost), i64::from(del_cost), i64::from(ch_cost)),
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
pub fn conjecture_tree_distance_weight_compute(
    param: &mut TreeWeightParam,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    param.ensure_init(bank.signature());
    let extension = TermWeightExtension::new(
        param.max_term_multiplier,
        param.max_literal_multiplier,
        param.pos_multiplier,
        param.ext_style,
        tree_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureTreeDistanceWeightCompute` with the OCB-backed
/// `ClauseCondMarkMaximalTerms` side effect.
///
/// This no-bank compatibility entry point uses the legacy immutable-bank
/// ordering path; WFCB callers that own the active bank use the banked callback.
#[must_use]
pub fn conjecture_tree_distance_weight_compute_with_ocb(
    param: &mut TreeWeightParam,
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
        tree_weight_extension,
        &*param,
    );
    clause.term_ext_weight(&extension)
}

/// Computes C `ConjectureTreeDistanceWeightCompute` with bank-backed ordering
/// preparation.
///
/// # Errors
///
/// Returns a diagnostic if bank-backed maximal-term marking fails.
pub fn conjecture_tree_distance_weight_compute_with_bank(
    param: &mut TreeWeightParam,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    param.ensure_init(bank.signature());
    clause.cond_mark_maximal_terms_with_bank(ocb, bank)?;
    Ok(conjecture_tree_distance_weight_compute(param, bank, clause))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TedTraversal {
    leftmost: Vec<usize>,
    code: Vec<FunCode>,
    key_roots: Vec<usize>,
    fresh: usize,
}

/// Computes C `ted_term_distance` for already-normalized terms.
///
/// # Panics
///
/// Panics if a traversed non-free, non-constant term has no argument 0, or if
/// any traversed compound term has an uninitialized argument. This mirrors the
/// unchecked C traversal in `ted_lmld_kr`.
#[must_use]
pub fn ted_term_distance(left: &Term, right: &Term, costs: TreeDistanceCosts) -> f64 {
    let left_traversal = ted_traversal(left);
    let right_traversal = ted_traversal(right);
    let left_len = left_traversal.code.len() - 1;
    let right_len = right_traversal.code.len() - 1;
    let mut tree_distance = vec![vec![0_i64; right_len + 1]; left_len + 1];

    for &left_root in &left_traversal.key_roots {
        for &right_root in &right_traversal.key_roots {
            ted_forest_distance(
                left_root,
                right_root,
                &left_traversal,
                &right_traversal,
                &mut tree_distance,
                costs,
            );
        }
    }

    i64_to_f64(tree_distance[left_len][right_len])
}

/// Scores `term` against normalized conjecture terms using tree edit distance.
///
/// # Panics
///
/// Panics under the same conditions as [`ted_term_distance`].
#[must_use]
pub fn ted_term_weight(term: &Term, conjecture_terms: &[Term], costs: TreeDistanceCosts) -> f64 {
    let mut minimum = f64::MAX;
    for conjecture in conjecture_terms {
        minimum = minimum.min(ted_term_distance(term, conjecture, costs));
    }
    minimum
}

fn ted_traversal(term: &Term) -> TedTraversal {
    let len = term_node_count(term);
    let mut traversal = TedTraversal {
        leftmost: vec![0; len + 1],
        code: vec![0; len + 1],
        key_roots: Vec::new(),
        fresh: 1,
    };
    let _ = ted_lmld_key_roots(term, &mut traversal, true);
    debug_assert_eq!(traversal.fresh, len + 1);
    traversal
}

fn ted_lmld_key_roots(term: &Term, traversal: &mut TedTraversal, is_root: bool) -> usize {
    let (idx, leftmost_idx) = if term.is_free_var() || term.is_const() {
        let idx = traversal.fresh;
        traversal.fresh += 1;
        traversal.leftmost[idx] = idx;
        (idx, idx)
    } else {
        let first = term
            .argument(0)
            .unwrap_or_else(|| panic!("tree-distance term argument 0 is uninitialized"));
        let first_leftmost = ted_lmld_key_roots(&first, traversal, false);
        for index in 1..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("tree-distance term argument {index} is uninitialized"));
            let _ = ted_lmld_key_roots(&arg, traversal, true);
        }

        let idx = traversal.fresh;
        traversal.fresh += 1;
        traversal.leftmost[idx] = traversal.leftmost[first_leftmost];
        (idx, first_leftmost)
    };

    traversal.code[idx] = term.f_code();
    if is_root {
        traversal.key_roots.push(idx);
    }
    leftmost_idx
}

fn ted_forest_distance(
    i: usize,
    j: usize,
    left: &TedTraversal,
    right: &TedTraversal,
    tree_distance: &mut [Vec<i64>],
    costs: TreeDistanceCosts,
) {
    let mut forest_distance = vec![vec![0_i64; j + 1]; i + 1];
    let left_base = left.leftmost[i] - 1;
    let right_base = right.leftmost[j] - 1;

    forest_distance[left_base][right_base] = 0;
    for di in left.leftmost[i]..=i {
        forest_distance[di][right_base] = forest_distance[di - 1][right_base] + costs.del_cost;
    }
    for dj in right.leftmost[j]..=j {
        forest_distance[left_base][dj] = forest_distance[left_base][dj - 1] + costs.ins_cost;
    }
    for di in left.leftmost[i]..=i {
        for dj in right.leftmost[j]..=j {
            if left.leftmost[di] == left.leftmost[i] && right.leftmost[dj] == right.leftmost[j] {
                forest_distance[di][dj] = min3(
                    forest_distance[di - 1][dj] + costs.del_cost,
                    forest_distance[di][dj - 1] + costs.ins_cost,
                    forest_distance[di - 1][dj - 1]
                        + if left.code[di] == right.code[dj] {
                            0
                        } else {
                            costs.ch_cost
                        },
                );
                tree_distance[di][dj] = forest_distance[di][dj];
            } else {
                forest_distance[di][dj] = min3(
                    forest_distance[di - 1][dj] + costs.del_cost,
                    forest_distance[di][dj - 1] + costs.ins_cost,
                    forest_distance[left.leftmost[di] - 1][right.leftmost[dj] - 1]
                        + tree_distance[di][dj],
                );
            }
        }
    }
}

fn term_node_count(term: &Term) -> usize {
    usize::try_from(term_weight(term, 1, 1)).expect("tree-distance term node count fits usize")
}

fn min3(left: i64, middle: i64, right: i64) -> i64 {
    left.min(middle).min(right)
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

fn conjecture_tree_distance_weight_wfcb_compute(
    data: Option<&mut TreeWeightParam>,
    bank: &TermBank,
    clause: &Clause,
) -> f64 {
    conjecture_tree_distance_weight_compute(
        data.unwrap_or_else(|| {
            panic!("ConjectureTreeDistanceWeight WFCB requires initialized parameters")
        }),
        bank,
        clause,
    )
}

fn conjecture_tree_distance_weight_wfcb_compute_with_bank(
    data: Option<&mut TreeWeightParam>,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<f64, Diagnostic> {
    conjecture_tree_distance_weight_compute_with_bank(
        data.unwrap_or_else(|| {
            panic!("ConjectureTreeDistanceWeight WFCB requires initialized parameters")
        }),
        ocb,
        bank,
        clause,
    )
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn tree_weight_extension(term: &Term, data: &&TreeWeightParam) -> f64 {
    data.term_weight(term)
}

fn tree_weight_exit(_data: TreeWeightParam) {}

#[cfg(test)]
mod tests {
    use super::{
        conjecture_tree_distance_weight_compute, conjecture_tree_distance_weight_compute_with_ocb,
        conjecture_tree_distance_weight_parse, ted_term_distance, ted_term_weight, ted_traversal,
        tree_weight_param_alloc, TreeDistanceCosts,
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
    use crate::terms::termtypes::{Term, TP_IS_DB_VAR};
    use crate::terms::termweightext::TermWeightExtensionStyle;
    use crate::terms::typebanks::TypeBank;

    fn costs(ins_cost: i64, del_cost: i64, ch_cost: i64) -> TreeDistanceCosts {
        TreeDistanceCosts::new(ins_cost, del_cost, ch_cost)
    }

    fn assert_f64_bits_eq(actual: f64, expected: f64) {
        assert_eq!(actual.to_bits(), expected.to_bits());
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
    fn traversal_matches_c_leftmost_leaf_and_key_root_shape() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let g = unary(20, &a);
        let root = binary(10, &g, &b);
        let traversal = ted_traversal(&root);

        assert_eq!(traversal.code, vec![0, 1, 20, 2, 10]);
        assert_eq!(traversal.leftmost, vec![0, 1, 1, 3, 1]);
        assert_eq!(traversal.key_roots, vec![3, 4]);
    }

    #[test]
    fn term_distance_handles_leaf_change_insert_and_delete_costs() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let fa = unary(10, &a);

        assert_f64_bits_eq(ted_term_distance(&a, &a, costs(2, 3, 5)), 0.0);
        assert_f64_bits_eq(ted_term_distance(&a, &b, costs(2, 3, 7)), 5.0);
        assert_f64_bits_eq(ted_term_distance(&a, &fa, costs(2, 3, 7)), 2.0);
        assert_f64_bits_eq(ted_term_distance(&fa, &a, costs(2, 3, 7)), 3.0);
    }

    #[test]
    fn term_distance_uses_tree_structure_and_root_change() {
        let a = Term::const_cell_alloc(1);
        let b = Term::const_cell_alloc(2);
        let c = Term::const_cell_alloc(3);
        let fab = binary(10, &a, &b);
        let fac = binary(10, &a, &c);
        let gab = binary(11, &a, &b);

        assert_f64_bits_eq(ted_term_distance(&fab, &fac, costs(2, 3, 4)), 4.0);
        assert_f64_bits_eq(ted_term_distance(&fab, &gab, costs(2, 3, 4)), 4.0);
    }

    #[test]
    fn term_weight_returns_minimum_or_dbl_max() {
        let term = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let exact = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(2));
        let close = binary(10, &Term::const_cell_alloc(1), &Term::const_cell_alloc(3));

        assert_f64_bits_eq(ted_term_weight(&term, &[close], costs(2, 3, 4)), 4.0);
        assert_f64_bits_eq(
            ted_term_weight(&term, &[Term::const_cell_alloc(99), exact], costs(2, 3, 4)),
            0.0,
        );
        assert_f64_bits_eq(ted_term_weight(&term, &[], costs(2, 3, 4)), f64::MAX);
    }

    #[test]
    #[should_panic(expected = "tree-distance term argument 0 is uninitialized")]
    fn bare_db_variable_follows_c_non_leaf_path_and_panics() {
        let db = Term::const_cell_alloc(0);
        db.set_prop(TP_IS_DB_VAR);

        let _ = ted_term_distance(&db, &Term::const_cell_alloc(1), costs(1, 1, 1));
    }

    #[test]
    fn conjecture_tree_weight_compute_initializes_terms_and_scores_clause_terms() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "c", true);
        let mut param = tree_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            costs(1, 1, 5),
            TermWeightExtensionStyle::Simple,
            1.0,
            1.0,
            1.0,
        );

        assert!(param.terms().is_none());
        assert_f64_bits_eq(
            conjecture_tree_distance_weight_compute(&mut param, &bank, &target),
            2.0,
        );
        assert_eq!(param.terms().expect("terms should be initialized").len(), 2);
    }

    #[test]
    fn conjecture_tree_weight_compute_with_ocb_marks_clause_like_c() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = tree_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            costs(1, 1, 5),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_tree_distance_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut actual_param = tree_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            costs(1, 1, 5),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let mut ocb = kbo_ocb(&bank);

        let actual = conjecture_tree_distance_weight_compute_with_ocb(
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
    fn conjecture_tree_weight_parse_uses_banked_wfcb_callback() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let mut target = clause(&mut bank, "a", "f(a)", true);
        let mut manually_marked = target.clone();
        let mut manual_ocb = kbo_ocb(&bank);
        assert!(manually_marked.cond_mark_maximal_terms(&mut manual_ocb, &bank));
        let mut expected_param = tree_weight_param_alloc(
            &axioms,
            VarNormStyle::Univar,
            RelatedTermSet::ConjectureTerms,
            costs(1, 1, 5),
            TermWeightExtensionStyle::Simple,
            1.0,
            7.0,
            1.0,
        );
        let expected =
            conjecture_tree_distance_weight_compute(&mut expected_param, &bank, &manually_marked);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,1,1,5,0,1.0,7.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_tree_distance_weight_parse(&mut scanner, &axioms)
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
    fn conjecture_tree_weight_parse_wraps_wfcb_compute() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let axioms = negated_conjecture_axioms(&mut bank);
        let target = clause(&mut bank, "f(a)", "c", true);
        let mut scanner =
            Scanner::from_user_string("(ConstPrio,0,0,1,1,5,0,1.0,1.0,1.0) tail", false).unwrap();
        let mut wfcb = conjecture_tree_distance_weight_parse(&mut scanner, &axioms)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_f64_bits_eq(wfcb.compute_eval(&bank, &target), 2.0);
        assert_eq!(scanner.current_token().literal(), "tail");
    }
}
