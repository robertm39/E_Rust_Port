use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clausepos::ClausePos;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnSide, EP_FROM_CLAUSE_LIT, EP_IS_PM_INTO_LIT};
use crate::clauses::eqnlist::EqnList;
use crate::orderings::cto_orderings::to_greater;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::match_mgu::subst_mgu_complete;
use crate::terms::replace::tb_term_pos_replace;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{DerefType, Term};
use crate::terms::termvars::VarBank;
use std::collections::BTreeMap;

/// Computes the first-order C `ComputeOverlap` replacement term.
///
/// On success, `subst` contains the MGU plus any fresh-variable bindings added
/// while normalizing the overlapped term and replacement side. On failure,
/// substitutions created by this helper are removed.
///
/// # Errors
///
/// Returns diagnostics for higher-order paramodulation constraints or term-bank
/// insertion failures.
///
/// # Panics
///
/// Panics if `from` violates the C internal-caller invariants: the selected
/// literal must be positive, the selected side must be legal for orientation,
/// `from` must point at a top term position, and the `into` position must not
/// designate a free variable.
pub fn compute_overlap(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &Term,
    pos: &TermPos,
    subst: &mut Substitution,
    freshvars: &VarBank,
) -> Result<Option<Term>, Diagnostic> {
    if problem_type() == ProblemType::HigherOrder {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "higher-order paramodulation constraints are not ported yet",
        ));
    }

    let from_literal = from
        .literal()
        .expect("paramodulation source position must select a literal");
    assert!(
        from.side() == EqnSide::LeftSide || !from_literal.is_oriented(),
        "oriented paramodulation source can only use its left side"
    );
    assert!(
        from_literal.is_positive(),
        "paramodulation source literal must be positive"
    );
    assert!(
        from.is_top(),
        "paramodulation source side must be selected at top position"
    );

    let sub_into = pos.get_subterm(into);
    assert!(
        !sub_into.is_free_var(),
        "paramodulation target position must not be a free variable"
    );

    let max_side = from
        .get_side()
        .expect("paramodulation source position must select a side");
    let rep_side = from
        .get_other_side()
        .expect("paramodulation source position must select an opposite side");
    let oldstate = subst.len();

    if !subst_mgu_complete(&max_side, &sub_into, subst) {
        subst.backtrack_to_pos(oldstate);
        return Ok(None);
    }

    if !from_literal.is_oriented()
        && to_greater(
            ocb,
            bank.signature(),
            &rep_side,
            &max_side,
            DerefType::Always,
            DerefType::Always,
        )
    {
        subst.backtrack_to_pos(oldstate);
        return Ok(None);
    }

    subst.norm_term(into, freshvars);
    subst.norm_term(&rep_side, freshvars);
    match tb_term_pos_replace(bank, &rep_side, pos, DerefType::Always, 0, Some(&sub_into)) {
        Ok(term) => Ok(Some(term)),
        Err(error) => {
            subst.backtrack_to_pos(oldstate);
            Err(error)
        }
    }
}

/// Computes the first-order C `EqnOrderedParamod` critical-pair literal.
///
/// On success, `subst` is left active for the caller, matching the C helper.
/// On rejection, substitutions created by this helper are removed.
///
/// # Errors
///
/// Returns diagnostics from [`compute_overlap`] or term-bank insertion.
///
/// # Panics
///
/// Panics if either clause position violates the C helper's internal
/// preconditions.
pub fn eqn_ordered_paramod(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
    subst: &mut Substitution,
    freshvars: &VarBank,
) -> Result<Option<Eqn>, Diagnostic> {
    let from_literal = from
        .literal()
        .expect("paramodulation source position must select a literal");
    let into_literal = into
        .literal()
        .expect("paramodulation target position must select a literal");

    assert!(
        from.side() == EqnSide::LeftSide || !from_literal.is_oriented(),
        "oriented paramodulation source can only use its left side"
    );
    assert!(
        from_literal.is_positive(),
        "paramodulation source literal must be positive"
    );
    assert!(
        from.is_top(),
        "paramodulation source side must be selected at top position"
    );
    assert!(
        into.side() == EqnSide::LeftSide || !into_literal.is_oriented(),
        "oriented paramodulation target can only use its left side"
    );

    let lside = into
        .get_side()
        .expect("paramodulation target position must select a side");
    let rside = into
        .get_other_side()
        .expect("paramodulation target position must select an opposite side");
    let oldstate = subst.len();

    let Some(replaced_lhs) =
        compute_overlap(bank, ocb, from, &lside, into.term_pos(), subst, freshvars)?
    else {
        return Ok(None);
    };

    if !into_literal.is_oriented()
        && to_greater(
            ocb,
            bank.signature(),
            &rside,
            &lside,
            DerefType::Always,
            DerefType::Always,
        )
    {
        subst.backtrack_to_pos(oldstate);
        return Ok(None);
    }

    subst.norm_term(&rside, freshvars);
    let instantiated_rhs = match bank.insert(&rside, DerefType::Always) {
        Ok(term) => term,
        Err(error) => {
            subst.backtrack_to_pos(oldstate);
            return Err(error);
        }
    };

    if into_literal.is_positive() && replaced_lhs == instantiated_rhs {
        subst.backtrack_to_pos(oldstate);
        return Ok(None);
    }

    let mut new_cp = match Eqn::alloc(
        replaced_lhs,
        instantiated_rhs,
        bank,
        into_literal.is_positive(),
    ) {
        Ok(literal) => literal,
        Err(error) => {
            subst.backtrack_to_pos(oldstate);
            return Err(error);
        }
    };
    new_cp.set_prop(EP_IS_PM_INTO_LIT);
    Ok(Some(new_cp))
}

/// Builds the first-order C `ClauseOrderedParamod` result for explicit
/// positions.
///
/// This is the low-level clause constructor. It does not push derivation
/// metadata; C adds `DCParamod` / `DCSimParamod` in the higher control wrapper.
///
/// # Errors
///
/// Returns diagnostics from paramodulation, substitution-normalized copying, or
/// term-bank insertion.
///
/// # Panics
///
/// Panics if either position is not backed by a clause/literal or violates the
/// C internal-caller invariants.
pub fn clause_ordered_paramod(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
) -> Result<Option<Clause>, Diagnostic> {
    let from_clause = from
        .clause()
        .expect("paramodulation source position must be backed by a clause");
    let into_clause = into
        .clause()
        .expect("paramodulation target position must be backed by a clause");
    let from_index = from
        .literal_index()
        .expect("paramodulation source position must select a clause literal");
    let into_index = into
        .literal_index()
        .expect("paramodulation target position must select a clause literal");
    let from_literal = from
        .literal()
        .expect("paramodulation source position must select a literal");
    let into_literal = into
        .literal()
        .expect("paramodulation target position must select a literal");

    assert!(
        from_literal.is_maximal(),
        "paramodulation source literal must be maximal"
    );
    assert!(
        !from_literal.is_oriented() || from.side() == EqnSide::LeftSide,
        "oriented paramodulation source can only use its left side"
    );
    assert!(
        !from
            .get_side()
            .expect("paramodulation source position must select a side")
            .is_free_var()
            || into_literal.is_equ_lit(bank)
            || !into.is_top(),
        "free-variable source side cannot paramodulate into non-equational top positions"
    );

    let freshvars = fresh_var_bank_for_clauses(bank, from_clause, into_clause);
    let mut subst = Substitution::new();
    let result = clause_ordered_paramod_with_subst(
        bank,
        ocb,
        from,
        into,
        from_clause,
        into_clause,
        from_index,
        into_index,
        into_literal,
        &freshvars,
        &mut subst,
    );
    subst.backtrack();
    result
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps clause-position state explicit"
)]
fn clause_ordered_paramod_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
    from_clause: &Clause,
    into_clause: &Clause,
    from_index: usize,
    into_index: usize,
    into_literal: &Eqn,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Option<Clause>, Diagnostic> {
    let Some(mut new_literal) = eqn_ordered_paramod(bank, ocb, from, into, subst, freshvars)?
    else {
        return Ok(None);
    };

    let into_is_eligible = (into_literal.is_positive()
        && eqn_is_strictly_maximal_under_subst(ocb, bank, into_clause, into_index))
        || into_literal.is_negative();
    if !into_is_eligible || !eqn_is_strictly_maximal_under_subst(ocb, bank, from_clause, from_index)
    {
        return Ok(None);
    }

    let _ = into_clause
        .literals()
        .subst_norm_except(Some(into_index), subst, freshvars);
    let _ = from_clause
        .literals()
        .subst_norm_except(Some(from_index), subst, freshvars);

    let mut into_copy = into_clause
        .literals()
        .copy_opt_except_index(Some(into_index), bank)?;
    let mut from_copy = from_clause
        .literals()
        .copy_opt_except_index(Some(from_index), bank)?;

    into_copy.del_prop(EP_FROM_CLAUSE_LIT);
    from_copy.set_prop(EP_FROM_CLAUSE_LIT);
    new_literal.set_prop(EP_FROM_CLAUSE_LIT);

    into_copy.append(from_copy);
    into_copy.del_prop(EP_IS_PM_INTO_LIT);

    let mut new_literals = EqnList::new();
    new_literals.push(new_literal);
    new_literals.append(into_copy);
    new_literals.remove_resolved(bank);
    new_literals.remove_duplicates(bank);
    Ok(Some(Clause::alloc(new_literals)))
}

fn eqn_is_strictly_maximal_under_subst(
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
            || !matches!(
                candidate.literal_compare(ocb, bank, target),
                CompareResult::Greater | CompareResult::Equal
            )
    })
}

fn fresh_var_bank_for_clauses(bank: &TermBank, first: &Clause, second: &Clause) -> VarBank {
    let freshvars = VarBank::new(bank.signature().type_bank());
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = first.collect_variables(&mut variables);
    let _ = second.collect_variables(&mut variables);
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
    use super::clause_ordered_paramod;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausepos::ClausePos;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, EP_FROM_CLAUSE_LIT, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_PM_INTO_LIT,
        EP_MAX_IS_UP_TO_DATE,
    };
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

    fn maximal_oriented(literal: &mut Eqn) {
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
    }

    fn top_left_position(clause: &Clause) -> ClausePos {
        let mut position = ClausePos::for_clause(clause.clone());
        assert!(position.set_literal_index(Some(0)));
        position.set_side(EqnSide::LeftSide);
        position
    }

    #[test]
    fn clause_ordered_paramod_replaces_selected_subterm() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "pm_basic_a");
        let b = typed_const(&mut bank, "pm_basic_b");
        let c = typed_const(&mut bank, "pm_basic_c");
        let f_code = typed_unary_code(&mut bank, "pm_basic_f");
        let f_of_a = typed_unary(&mut bank, f_code, &a);
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut from_lit = lit(&mut bank, &a, &b, true);
        let mut into_lit = lit(&mut bank, &f_of_a, &c, true);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut into_lit);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit]));
        let from_pos = top_left_position(&from_clause);
        let mut into_pos = top_left_position(&into_clause);
        into_pos.term_pos_mut().push_component(f_of_a.clone(), 0);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant = clause_ordered_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
            .unwrap()
            .expect("ground equality should paramodulate into selected subterm");

        assert_eq!(paramodulant.literal_number(), 1);
        let generated = &paramodulant.literals().as_slice()[0];
        assert!(generated.is_positive());
        assert!(generated.query_prop(EP_IS_PM_INTO_LIT));
        assert!(generated.query_prop(EP_FROM_CLAUSE_LIT));
        assert_eq!(generated.left(), &f_of_b);
        assert_eq!(generated.right(), &c);
    }

    #[test]
    fn clause_ordered_paramod_preserves_c_context_flag_flow() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_flags_a");
        let source_right = typed_const(&mut bank, "pm_flags_b");
        let target_right = typed_const(&mut bank, "pm_flags_c");
        let context_left = typed_const(&mut bank, "pm_flags_d");
        let context_right = typed_const(&mut bank, "pm_flags_e");
        let f_code = typed_unary_code(&mut bank, "pm_flags_f");
        let f_of_source_left = typed_unary(&mut bank, f_code, &source_left);
        let mut from_lit = lit(&mut bank, &source_left, &source_right, true);
        let mut from_context = lit(&mut bank, &context_left, &context_right, true);
        let mut into_lit = lit(&mut bank, &f_of_source_left, &target_right, true);
        let mut into_context = lit(&mut bank, &target_right, &context_left, true);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut into_lit);
        from_context.set_prop(EP_IS_PM_INTO_LIT);
        into_context.set_prop(EP_FROM_CLAUSE_LIT | EP_IS_PM_INTO_LIT);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit, from_context]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit, into_context]));
        let from_pos = top_left_position(&from_clause);
        let mut into_pos = top_left_position(&into_clause);
        into_pos.term_pos_mut().push_component(f_of_source_left, 0);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant = clause_ordered_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
            .unwrap()
            .expect("context literals should be copied around the generated literal");

        let literals = paramodulant.literals().as_slice();
        assert_eq!(literals.len(), 3);
        assert!(literals[0].query_prop(EP_IS_PM_INTO_LIT));
        assert!(literals[0].query_prop(EP_FROM_CLAUSE_LIT));
        assert!(!literals[1].query_prop(EP_IS_PM_INTO_LIT));
        assert!(!literals[1].query_prop(EP_FROM_CLAUSE_LIT));
        assert!(!literals[2].query_prop(EP_IS_PM_INTO_LIT));
        assert!(literals[2].query_prop(EP_FROM_CLAUSE_LIT));
        assert_eq!(literals[1].left(), &target_right);
        assert_eq!(literals[1].right(), &context_left);
        assert_eq!(literals[2].left(), &context_left);
        assert_eq!(literals[2].right(), &context_right);
    }

    #[test]
    fn clause_ordered_paramod_optimizes_trivial_positive_paramodulants() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "pm_trivial_pos_a");
        let b = typed_const(&mut bank, "pm_trivial_pos_b");
        let mut from_lit = lit(&mut bank, &a, &b, true);
        let mut into_lit = lit(&mut bank, &a, &b, true);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut into_lit);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit]));
        let from_pos = top_left_position(&from_clause);
        let into_pos = top_left_position(&into_clause);
        let mut ocb = kbo_ocb(&bank);

        assert!(
            clause_ordered_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn clause_ordered_paramod_negative_trivial_literal_can_yield_empty_clause() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "pm_empty_a");
        let b = typed_const(&mut bank, "pm_empty_b");
        let mut from_lit = lit(&mut bank, &a, &b, true);
        let mut into_lit = lit(&mut bank, &a, &b, false);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut into_lit);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit]));
        let from_pos = top_left_position(&from_clause);
        let into_pos = top_left_position(&into_clause);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant = clause_ordered_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
            .unwrap()
            .expect("negative trivial paramodulant is cleaned into an empty clause");

        assert_eq!(paramodulant.literal_number(), 0);
        assert!(paramodulant.literals().is_empty());
    }
}
