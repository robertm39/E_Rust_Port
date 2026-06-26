use crate::basics::partial_orderings::CompareResult;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use crate::{basics::error::Diagnostic, terms::termtypes::Term};
use std::collections::BTreeMap;

/// One C `ClausePosFirst/NextOrderedFactorLiterals` candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrderedFactorPosition {
    first_literal_index: usize,
    second_literal_index: usize,
    second_side: EqnSide,
}

impl OrderedFactorPosition {
    #[must_use]
    pub const fn new(
        first_literal_index: usize,
        second_literal_index: usize,
        second_side: EqnSide,
    ) -> Self {
        Self {
            first_literal_index,
            second_literal_index,
            second_side,
        }
    }

    #[must_use]
    pub const fn first_literal_index(self) -> usize {
        self.first_literal_index
    }

    #[must_use]
    pub const fn second_literal_index(self) -> usize {
        self.second_literal_index
    }

    #[must_use]
    pub const fn second_side(self) -> EqnSide {
        self.second_side
    }
}

/// Returns all C ordered-factor literal candidates in cursor order.
#[must_use]
pub fn ordered_factor_positions(clause: &Clause) -> Vec<OrderedFactorPosition> {
    let literals = clause.literals().as_slice();
    let mut positions = Vec::new();

    for (first_index, first) in literals.iter().enumerate() {
        if !is_ordered_factor_candidate(first) {
            continue;
        }

        for (offset, second) in literals[first_index.saturating_add(1)..].iter().enumerate() {
            if !is_ordered_factor_candidate(second) {
                continue;
            }
            let second_index = first_index + offset + 1;
            positions.push(OrderedFactorPosition::new(
                first_index,
                second_index,
                EqnSide::LeftSide,
            ));
            if !first.is_oriented() || !second.is_oriented() {
                positions.push(OrderedFactorPosition::new(
                    first_index,
                    second_index,
                    EqnSide::RightSide,
                ));
            }
        }
    }

    positions
}

/// Builds the first-order C `ComputeOrderedFactor` result for one candidate.
///
/// The higher-order equality-factoring path is separate in C and is not handled
/// here. For `RightSide` candidates, Rust swaps a local clone of the second
/// literal instead of temporarily mutating the source clause literal.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the factor.
///
/// # Panics
///
/// Panics if the candidate indices are invalid, select the same literal, select
/// non-positive/non-maximal literals, or use an invalid side. These preserve the
/// internal-caller invariants encoded by the C clause-position API.
pub fn compute_ordered_factor(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: OrderedFactorPosition,
) -> Result<Option<Clause>, Diagnostic> {
    let literals = clause.literals().as_slice();
    assert_ne!(
        position.first_literal_index, position.second_literal_index,
        "ordered factoring expects two distinct literals"
    );
    assert!(
        matches!(position.second_side, EqnSide::LeftSide | EqnSide::RightSide),
        "ordered factoring second side must be left or right"
    );

    let first = literals
        .get(position.first_literal_index)
        .expect("ordered-factor first literal index must be valid");
    let second = literals
        .get(position.second_literal_index)
        .expect("ordered-factor second literal index must be valid");
    assert!(
        first.is_positive() && first.is_maximal(),
        "ordered factoring expects a maximal positive first literal"
    );
    assert!(
        second.is_positive() && second.is_maximal(),
        "ordered factoring expects a maximal positive second literal"
    );

    let mut oriented_second = second.clone();
    if position.second_side == EqnSide::RightSide {
        oriented_second.swap_sides_simple();
    }

    let mut subst = Substitution::new();
    if !first.unify_directed(&oriented_second, &mut subst) {
        return Ok(None);
    }

    let result = if eqn_is_maximal_under_subst(ocb, bank, clause, position.first_literal_index) {
        build_ordered_factor(bank, clause, position.second_literal_index, &mut subst)
    } else {
        Ok(None)
    };
    subst.backtrack();
    result
}

/// Computes all first-order ordered factors and inserts them into `store`.
///
/// This mirrors C `ComputeAllOrderedFactors` for the first-order ordered
/// factoring path. Proof-documentation and derivation-stack side effects remain
/// pending until clause derivation ownership is ported.
///
/// # Errors
///
/// Returns diagnostics from [`compute_ordered_factor`].
pub fn compute_all_ordered_factors(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let mut factor_count = 0;
    if clause.is_horn() || clause.query_prop(CP_NO_GENERATION) {
        return Ok(factor_count);
    }

    for position in ordered_factor_positions(clause) {
        if let Some(mut factor) = compute_ordered_factor(bank, ocb, clause, position)? {
            factor_count += 1;
            factor.set_proof_depth(clause.proof_depth().saturating_add(1));
            factor.set_proof_size(clause.proof_size().saturating_add(1));
            factor.set_tptp_type(clause.query_tptp_type());
            factor.set_prop(clause.give_props(CP_IS_SOS));
            store.insert(factor);
        }
    }

    Ok(factor_count)
}

fn is_ordered_factor_candidate(literal: &Eqn) -> bool {
    literal.is_positive() && literal.is_maximal()
}

fn eqn_is_maximal_under_subst(
    ocb: &mut OrderControlBlock,
    bank: &TermBank,
    clause: &Clause,
    target_index: usize,
) -> bool {
    let literals = clause.literals().as_slice();
    let target = literals
        .get(target_index)
        .expect("maximality target index must be valid");
    literals.iter().enumerate().all(|(index, candidate)| {
        index == target_index
            || !candidate.is_maximal()
            || candidate.literal_compare(ocb, bank, target) != CompareResult::Greater
    })
}

fn build_ordered_factor(
    bank: &mut TermBank,
    clause: &Clause,
    removed_literal_index: usize,
    subst: &mut Substitution,
) -> Result<Option<Clause>, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    let backtrack =
        clause
            .literals()
            .subst_norm_except(Some(removed_literal_index), subst, &freshvars);
    let mut new_literals = clause
        .literals()
        .copy_opt_except_index(Some(removed_literal_index), bank)?;
    subst.backtrack_to_pos(backtrack);
    new_literals.remove_resolved(bank);
    new_literals.remove_duplicates(bank);
    Ok(Some(Clause::alloc(new_literals)))
}

fn fresh_var_bank_for_clause(bank: &TermBank, clause: &Clause) -> VarBank {
    let freshvars = VarBank::new(bank.signature().type_bank());
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    let max_var = variables
        .values()
        .map(|variable| -variable.f_code())
        .max()
        .unwrap_or(0);
    let default_type = bank.signature().type_bank().default_type();
    while freshvars.fresh_count() < max_var {
        let _ = freshvars.get_fresh_var(&default_type);
    }
    freshvars.set_v_counts_to_used();
    freshvars
}

#[cfg(test)]
mod tests {
    use super::{
        compute_all_ordered_factors, compute_ordered_factor, ordered_factor_positions,
        OrderedFactorPosition,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        let term = Term::const_cell_alloc(f_code);
        term.set_type(Some(type_));
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_unary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
                .unwrap();
        }
        f_code
    }

    fn typed_unary(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn lit(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn maximal(literal: &mut Eqn) {
        literal.set_prop(EP_IS_MAXIMAL);
    }

    #[test]
    fn ordered_factor_positions_follow_c_literal_and_side_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "of_iter_a");
        let b = typed_const(&mut bank, "of_iter_b");
        let c = typed_const(&mut bank, "of_iter_c");
        let d = typed_const(&mut bank, "of_iter_d");
        let mut negative_max = lit(&mut bank, &a, &d, false);
        let mut first = lit(&mut bank, &a, &b, true);
        let plain = lit(&mut bank, &b, &c, true);
        let mut second = lit(&mut bank, &b, &c, true);
        let mut third = lit(&mut bank, &c, &d, true);

        negative_max.set_prop(EP_IS_MAXIMAL);
        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        second.set_prop(EP_IS_MAXIMAL);
        third.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let clause = Clause::alloc(EqnList::from_vec(vec![
            negative_max,
            first,
            plain,
            second,
            third,
        ]));

        assert_eq!(
            ordered_factor_positions(&clause),
            vec![
                OrderedFactorPosition::new(0, 2, EqnSide::LeftSide),
                OrderedFactorPosition::new(0, 2, EqnSide::RightSide),
                OrderedFactorPosition::new(0, 3, EqnSide::LeftSide),
                OrderedFactorPosition::new(2, 3, EqnSide::LeftSide),
                OrderedFactorPosition::new(2, 3, EqnSide::RightSide),
            ]
        );
    }

    #[test]
    fn compute_ordered_factor_instantiates_and_removes_second_literal() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "of_build_a");
        let b = typed_const(&mut bank, "of_build_b");
        let c = typed_const(&mut bank, "of_build_c");
        let mut factor_lit = lit(&mut bank, &x, &b, true);
        let mut removed_lit = lit(&mut bank, &a, &b, true);
        let rest = lit(&mut bank, &x, &c, true);
        maximal(&mut factor_lit);
        maximal(&mut removed_lit);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, factor_lit, removed_lit]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_ordered_factor(
            &mut bank,
            &mut ocb,
            &clause,
            OrderedFactorPosition::new(1, 2, EqnSide::LeftSide),
        )
        .unwrap()
        .expect("directed positive equalities should factor");

        assert!(x.binding().is_none());
        assert_eq!(factor.literal_number(), 2);
        assert_eq!(factor.literals().as_slice()[0].left(), &a);
        assert_eq!(factor.literals().as_slice()[0].right(), &c);
        assert_eq!(factor.literals().as_slice()[1].left(), &a);
        assert_eq!(factor.literals().as_slice()[1].right(), &b);
    }

    #[test]
    fn compute_ordered_factor_honors_swapped_second_side() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "of_swap_a");
        let b = typed_const(&mut bank, "of_swap_b");
        let mut factor_lit = lit(&mut bank, &x, &a, true);
        let mut removed_lit = lit(&mut bank, &a, &b, true);
        maximal(&mut factor_lit);
        maximal(&mut removed_lit);
        let clause = Clause::alloc(EqnList::from_vec(vec![factor_lit, removed_lit]));
        let mut ocb = kbo_ocb(&bank);

        assert!(compute_ordered_factor(
            &mut bank,
            &mut ocb,
            &clause,
            OrderedFactorPosition::new(0, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .is_none());
        let factor = compute_ordered_factor(
            &mut bank,
            &mut ocb,
            &clause,
            OrderedFactorPosition::new(0, 1, EqnSide::RightSide),
        )
        .unwrap()
        .expect("swapped second literal should factor");

        assert_eq!(factor.literal_number(), 1);
        assert_eq!(factor.literals().as_slice()[0].left(), &b);
        assert_eq!(factor.literals().as_slice()[0].right(), &a);
    }

    #[test]
    fn compute_ordered_factor_rechecks_maximality_after_unification() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "of_max_a");
        let b = typed_const(&mut bank, "of_max_b");
        let f_code = typed_unary_code(&mut bank, "of_max_f");
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut dominating = lit(&mut bank, &f_of_b, &a, true);
        let mut factor_lit = lit(&mut bank, &x, &a, true);
        let mut removed_lit = lit(&mut bank, &b, &a, true);
        maximal(&mut dominating);
        maximal(&mut factor_lit);
        maximal(&mut removed_lit);
        let clause = Clause::alloc(EqnList::from_vec(vec![dominating, factor_lit, removed_lit]));
        let mut ocb = kbo_ocb(&bank);

        assert!(compute_ordered_factor(
            &mut bank,
            &mut ocb,
            &clause,
            OrderedFactorPosition::new(1, 2, EqnSide::LeftSide),
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn compute_all_ordered_factors_inserts_metadata_and_honors_gates() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "of_all_a");
        let b = typed_const(&mut bank, "of_all_b");
        let mut factor_lit = lit(&mut bank, &x, &a, true);
        let mut removed_lit = lit(&mut bank, &b, &a, true);
        factor_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        removed_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![factor_lit, removed_lit]));
        clause.set_proof_depth(3);
        clause.set_proof_size(5);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause.set_prop(CP_IS_SOS);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_ordered_factors(&mut bank, &mut ocb, &clause, &mut store).unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store.iter().next().expect("one ordered factor inserted");
        assert_eq!(stored.proof_depth(), 4);
        assert_eq!(stored.proof_size(), 6);
        assert_eq!(stored.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(stored.query_prop(CP_IS_SOS));

        let horn = Clause::alloc(EqnList::from_vec(vec![lit(&mut bank, &a, &b, true)]));
        let mut horn_store = ClauseSet::new();
        assert_eq!(
            compute_all_ordered_factors(&mut bank, &mut ocb, &horn, &mut horn_store).unwrap(),
            0
        );
        assert!(horn_store.is_empty());

        let mut blocked = clause.clone();
        blocked.set_prop(CP_NO_GENERATION);
        let mut blocked_store = ClauseSet::new();
        assert_eq!(
            compute_all_ordered_factors(&mut bank, &mut ocb, &blocked, &mut blocked_store).unwrap(),
            0
        );
        assert!(blocked_store.is_empty());
    }
}
