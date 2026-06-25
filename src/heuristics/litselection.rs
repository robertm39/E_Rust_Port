use crate::clauses::clause::Clause;
use crate::clauses::eqn_props::EP_IS_SELECTED;
use crate::orderings::ocb::OrderControlBlock;

pub const NO_SELECTION: &str = "NoSelection";
pub const NO_GENERATION: &str = "NoGeneration";
pub const SELECT_NEGATIVE_LITERALS: &str = "SelectNegativeLiterals";
pub const P_SELECT_NEGATIVE_LITERALS: &str = "PSelectNegativeLiterals";
pub const SELECT_PURE_VAR_NEG_LITERALS: &str = "SelectPureVarNegLiterals";
pub const P_SELECT_PURE_VAR_NEG_LITERALS: &str = "PSelectPureVarNegLiterals";
pub const SELECT_LARGEST_NEG_LIT: &str = "SelectLargestNegLit";
pub const P_SELECT_LARGEST_NEG_LIT: &str = "PSelectLargestNegLit";
pub const SELECT_SMALLEST_NEG_LIT: &str = "SelectSmallestNegLit";
pub const P_SELECT_SMALLEST_NEG_LIT: &str = "PSelectSmallestNegLit";
pub const SELECT_DIFF_NEG_LIT: &str = "SelectDiffNegLit";
pub const P_SELECT_DIFF_NEG_LIT: &str = "PSelectDiffNegLit";
pub const SELECT_GROUND_NEG_LIT: &str = "SelectGroundNegLit";
pub const P_SELECT_GROUND_NEG_LIT: &str = "PSelectGroundNegLit";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedLiteralSelection {
    strategy: String,
}

impl UnsupportedLiteralSelection {
    #[must_use]
    pub fn new(strategy: impl Into<String>) -> Self {
        Self {
            strategy: strategy.into(),
        }
    }

    #[must_use]
    pub fn strategy(&self) -> &str {
        &self.strategy
    }
}

impl std::fmt::Display for UnsupportedLiteralSelection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "literal selection strategy '{}' is not ported yet",
            self.strategy
        )
    }
}

/// C `SelectNoLiterals`: assert that no literal is selected and otherwise do
/// nothing.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

/// C `SelectNoGeneration`: same no-op body as `SelectNoLiterals`.
///
/// # Panics
///
/// Panics in debug builds if the caller has not already cleared selected
/// literal properties.
pub fn select_no_generation(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

pub fn select_negative_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.is_negative() {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
}

pub fn p_select_negative_literals(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    clause.literals_mut().set_prop(EP_IS_SELECTED);
}

pub fn select_first_variable_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if let Some(index) = clause.literals().find_neg_pure_var_lit_index() {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_first_variable_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    if clause.literals().find_neg_pure_var_lit_index().is_some() {
        select_positive_literals(clause);
    }
}

pub fn select_largest_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let mut selected = None;
    let mut select_weight = 0;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.standard_weight() > select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_largest_negative_literal(
    _ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    let mut selected = None;
    let mut select_weight = 0;

    for (index, literal) in clause.literals_mut().as_mut_slice().iter_mut().enumerate() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        } else if literal.standard_weight() > select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_smallest_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && literal.standard_weight() < select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_smallest_negative_literal(
    _ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) {
    let mut selected = None;
    let mut select_weight = i64::MAX;

    for (index, literal) in clause.literals_mut().as_mut_slice().iter_mut().enumerate() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        } else if literal.standard_weight() < select_weight {
            select_weight = literal.standard_weight();
            selected = Some(index);
        }
    }

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_diff_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, false);

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_diff_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, false);
    select_positive_literals(clause);

    debug_assert!(
        selected.is_some(),
        "literal-selection wrapper guarantees a negative literal"
    );
    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn select_ground_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, true);

    if let Some(index) = selected {
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    }
}

pub fn p_select_ground_negative_literal(_ocb: Option<&mut OrderControlBlock>, clause: &mut Clause) {
    let selected = find_max_diff_negative_literal(clause, true);

    if let Some(index) = selected {
        select_positive_literals(clause);
        clause.literals_mut().as_mut_slice()[index].set_prop(EP_IS_SELECTED);
    } else {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    }
}

/// Applies the subset of literal-selection functions that has been ported.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` for valid C selector names whose
/// selector bodies have not been ported yet.
pub fn apply_ported_literal_selector(
    name: &str,
    ocb: Option<&mut OrderControlBlock>,
    clause: &mut Clause,
) -> Result<(), UnsupportedLiteralSelection> {
    match name {
        NO_SELECTION => {
            select_no_literals(ocb, clause);
            Ok(())
        }
        NO_GENERATION => {
            select_no_generation(ocb, clause);
            Ok(())
        }
        SELECT_NEGATIVE_LITERALS => {
            select_negative_literals(ocb, clause);
            Ok(())
        }
        P_SELECT_NEGATIVE_LITERALS => {
            p_select_negative_literals(ocb, clause);
            Ok(())
        }
        SELECT_PURE_VAR_NEG_LITERALS => {
            select_first_variable_literal(ocb, clause);
            Ok(())
        }
        P_SELECT_PURE_VAR_NEG_LITERALS => {
            p_select_first_variable_literal(ocb, clause);
            Ok(())
        }
        SELECT_LARGEST_NEG_LIT => {
            select_largest_negative_literal(ocb, clause);
            Ok(())
        }
        P_SELECT_LARGEST_NEG_LIT => {
            p_select_largest_negative_literal(ocb, clause);
            Ok(())
        }
        SELECT_SMALLEST_NEG_LIT => {
            select_smallest_negative_literal(ocb, clause);
            Ok(())
        }
        P_SELECT_SMALLEST_NEG_LIT => {
            p_select_smallest_negative_literal(ocb, clause);
            Ok(())
        }
        SELECT_DIFF_NEG_LIT => {
            select_diff_negative_literal(ocb, clause);
            Ok(())
        }
        P_SELECT_DIFF_NEG_LIT => {
            p_select_diff_negative_literal(ocb, clause);
            Ok(())
        }
        SELECT_GROUND_NEG_LIT => {
            select_ground_negative_literal(ocb, clause);
            Ok(())
        }
        P_SELECT_GROUND_NEG_LIT => {
            p_select_ground_negative_literal(ocb, clause);
            Ok(())
        }
        _ => Err(UnsupportedLiteralSelection::new(name)),
    }
}

fn select_positive_literals(clause: &mut Clause) {
    debug_assert_ne!(clause.negative_literal_count(), 0);
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.is_positive() {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
}

fn find_max_diff_negative_literal(clause: &Clause, ground_only: bool) -> Option<usize> {
    let mut selected = None;
    let mut select_weight = -1;

    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_negative() && (!ground_only || literal.is_ground()) {
            let weight = literal_selection_diff_weight(literal);
            if weight > select_weight {
                select_weight = weight;
                selected = Some(index);
            }
        }
    }

    selected
}

fn literal_selection_diff_weight(literal: &crate::clauses::eqn::Eqn) -> i64 {
    100 * literal.standard_diff() + literal.standard_weight()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_ported_literal_selector, p_select_diff_negative_literal,
        p_select_first_variable_literal, p_select_ground_negative_literal,
        p_select_largest_negative_literal, p_select_negative_literals,
        p_select_smallest_negative_literal, select_diff_negative_literal,
        select_first_variable_literal, select_ground_negative_literal,
        select_largest_negative_literal, select_negative_literals,
        select_smallest_negative_literal, NO_GENERATION, NO_SELECTION, P_SELECT_DIFF_NEG_LIT,
        P_SELECT_GROUND_NEG_LIT, P_SELECT_LARGEST_NEG_LIT, P_SELECT_NEGATIVE_LITERALS,
        P_SELECT_PURE_VAR_NEG_LITERALS, P_SELECT_SMALLEST_NEG_LIT, SELECT_DIFF_NEG_LIT,
        SELECT_GROUND_NEG_LIT, SELECT_LARGEST_NEG_LIT, SELECT_NEGATIVE_LITERALS,
        SELECT_PURE_VAR_NEG_LITERALS, SELECT_SMALLEST_NEG_LIT,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_SELECTED;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    fn const_term(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn var_term(code: i64) -> Term {
        Term::const_cell_alloc(code)
    }

    fn unary(code: i64, arg: &Term) -> Term {
        let term = Term::top_alloc(code, 1);
        term.set_argument(0, arg.clone());
        term
    }

    fn shared_const(bank: &mut TermBank, name: &str) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.create_const_term(f_code).unwrap()
    }

    fn shared_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.insert(&unary(f_code, arg), DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn select_mask(clause: &Clause) -> Vec<bool> {
        clause
            .literals()
            .as_slice()
            .iter()
            .map(|literal| literal.query_prop(EP_IS_SELECTED))
            .collect()
    }

    fn clear_selection(clause: &mut Clause) {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    }

    fn simple_mixed_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(10);
        let b = const_term(11);
        let c = const_term(12);
        let d = const_term(13);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &c, &d, true),
            literal(&mut bank, &b, &c, false),
        ]))
    }

    fn weighted_mixed_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(20);
        let b = const_term(21);
        let c = const_term(22);
        let d = const_term(23);
        let f_a = unary(30, &a);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &c, &d, true),
            literal(&mut bank, &f_a, &b, false),
        ]))
    }

    fn pure_var_clause() -> Clause {
        let mut bank = test_bank();
        let a = const_term(40);
        let b = const_term(41);
        let x = var_term(-2);
        let y = var_term(-4);
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &a, &b, false),
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &x, &y, false),
        ]))
    }

    fn ground_and_nonground_clause(include_ground_negative: bool) -> Clause {
        let mut bank = test_bank();
        let a = shared_const(&mut bank, "ls_ground_a");
        let b = shared_const(&mut bank, "ls_ground_b");
        let c = shared_const(&mut bank, "ls_ground_c");
        let x = var_term(-2);
        x.set_type(Some(bank.signature().type_bank().default_type()));
        let f_a = shared_unary(&mut bank, "ls_ground_f", &a);
        let mut literals = vec![
            literal(&mut bank, &a, &b, true),
            literal(&mut bank, &x, &b, false),
        ];
        if include_ground_negative {
            literals.push(literal(&mut bank, &f_a, &c, false));
        }
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn no_selection_and_no_generation_are_noop_selectors() {
        let mut clause = Clause::empty();

        apply_ported_literal_selector(NO_SELECTION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });
        apply_ported_literal_selector(NO_GENERATION, None, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn negative_literal_selectors_mark_negative_or_all_literals() {
        let mut clause = simple_mixed_clause();

        select_negative_literals(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, true]);

        clear_selection(&mut clause);
        p_select_negative_literals(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, true]);
    }

    #[test]
    fn pure_variable_selectors_preserve_c_positive_variant_shape() {
        let mut clause = pure_var_clause();

        select_first_variable_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_first_variable_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, false]);
    }

    #[test]
    fn largest_and_smallest_negative_selectors_use_first_standard_weight_best() {
        let mut clause = weighted_mixed_clause();

        select_largest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_largest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);

        clear_selection(&mut clause);
        select_smallest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, true, false]);

        clear_selection(&mut clause);
        p_select_smallest_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, true, false]);
    }

    #[test]
    fn diff_selectors_prefer_unbalanced_negative_literal() {
        let mut clause = weighted_mixed_clause();

        select_diff_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_diff_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);
    }

    #[test]
    fn ground_negative_selectors_skip_or_clear_without_ground_candidate() {
        let mut clause = ground_and_nonground_clause(true);

        select_ground_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![false, false, true]);

        clear_selection(&mut clause);
        p_select_ground_negative_literal(None, &mut clause);
        assert_eq!(select_mask(&clause), vec![true, false, true]);

        let mut no_ground = ground_and_nonground_clause(false);
        no_ground.literals_mut().set_prop(EP_IS_SELECTED);

        select_ground_negative_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![true, true]);

        p_select_ground_negative_literal(None, &mut no_ground);
        assert_eq!(select_mask(&no_ground), vec![false, false]);
    }

    #[test]
    fn simple_selectors_are_available_by_c_strategy_name() {
        for name in [
            SELECT_NEGATIVE_LITERALS,
            P_SELECT_NEGATIVE_LITERALS,
            SELECT_PURE_VAR_NEG_LITERALS,
            P_SELECT_PURE_VAR_NEG_LITERALS,
            SELECT_LARGEST_NEG_LIT,
            P_SELECT_LARGEST_NEG_LIT,
            SELECT_SMALLEST_NEG_LIT,
            P_SELECT_SMALLEST_NEG_LIT,
            SELECT_DIFF_NEG_LIT,
            P_SELECT_DIFF_NEG_LIT,
            SELECT_GROUND_NEG_LIT,
            P_SELECT_GROUND_NEG_LIT,
        ] {
            let mut clause = ground_and_nonground_clause(true);
            apply_ported_literal_selector(name, None, &mut clause).unwrap_or_else(|err| {
                panic!("{err}");
            });
        }
    }

    #[test]
    fn unported_selector_reports_name() {
        let mut clause = Clause::empty();
        let error =
            apply_ported_literal_selector("SelectOptimalLit", None, &mut clause).unwrap_err();

        assert_eq!(error.strategy(), "SelectOptimalLit");
        assert!(error.to_string().contains("not ported yet"));
    }
}
