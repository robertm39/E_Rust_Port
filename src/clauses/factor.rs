use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_push_derivation, set_is_ho, DC_EQ_FACTOR, DC_ORDERED_FACTOR,
};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::clauses::inferencedoc::{
    ClauseCreationInference, ClauseCreationParents, ProofDocSession,
};
use crate::orderings::cto_orderings::to_greater_with_bank;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::ho_csu::CsuIterator;
use crate::terms::match_mgu::subst_mgu_complete_with_bank;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use crate::{
    basics::error::Diagnostic,
    terms::termtypes::{DerefType, Term},
};
use std::{collections::BTreeMap, fmt};

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

/// One C `ClausePosFirst/NextEqualityFactorSides` candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EqualityFactorPosition {
    first_literal_index: usize,
    first_side: EqnSide,
    second_literal_index: usize,
    second_side: EqnSide,
}

struct EqualityFactorInput {
    max_term: Term,
    with_term: Term,
    min_term: Term,
    second_other: Term,
    first_is_equ_lit: bool,
    second_is_equ_lit: bool,
}

impl EqualityFactorPosition {
    #[must_use]
    pub const fn new(
        first_literal_index: usize,
        first_side: EqnSide,
        second_literal_index: usize,
        second_side: EqnSide,
    ) -> Self {
        Self {
            first_literal_index,
            first_side,
            second_literal_index,
            second_side,
        }
    }

    #[must_use]
    pub const fn first_literal_index(self) -> usize {
        self.first_literal_index
    }

    #[must_use]
    pub const fn first_side(self) -> EqnSide {
        self.first_side
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

/// Returns all C equality-factor side candidates in cursor order.
#[must_use]
pub fn equality_factor_positions(clause: &Clause) -> Vec<EqualityFactorPosition> {
    let literals = clause.literals().as_slice();
    let mut positions = Vec::new();

    for (first_index, first) in literals.iter().enumerate() {
        if !is_ordered_factor_candidate(first) {
            continue;
        }

        push_equality_factor_positions_for_side(
            literals,
            &mut positions,
            first_index,
            EqnSide::LeftSide,
        );
        if !first.is_oriented() {
            push_equality_factor_positions_for_side(
                literals,
                &mut positions,
                first_index,
                EqnSide::RightSide,
            );
        }
    }

    positions
}

fn push_equality_factor_positions_for_side(
    literals: &[Eqn],
    positions: &mut Vec<EqualityFactorPosition>,
    first_index: usize,
    first_side: EqnSide,
) {
    for (second_index, second) in literals.iter().enumerate() {
        if second_index == first_index || !second.is_positive() {
            continue;
        }
        positions.push(EqualityFactorPosition::new(
            first_index,
            first_side,
            second_index,
            EqnSide::LeftSide,
        ));
        positions.push(EqualityFactorPosition::new(
            first_index,
            first_side,
            second_index,
            EqnSide::RightSide,
        ));
    }
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
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_ordered_factor_impl(bank, ocb, clause, position, &freshvars, false)
}

/// Builds the first-order C `ComputeOrderedFactor` result using a caller-owned
/// `freshvars` bank.
///
/// This mirrors C's proof-control path by resetting variable counts before the
/// directed unification attempt.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the factor.
///
/// # Panics
///
/// Panics under the same internal-caller invariant violations as
/// [`compute_ordered_factor`].
pub fn compute_ordered_factor_with_fresh_vars(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: OrderedFactorPosition,
    freshvars: &VarBank,
) -> Result<Option<Clause>, Diagnostic> {
    compute_ordered_factor_impl(bank, ocb, clause, position, freshvars, true)
}

fn compute_ordered_factor_impl(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: OrderedFactorPosition,
    freshvars: &VarBank,
    reset_freshvars: bool,
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
    if reset_freshvars {
        freshvars.reset_v_counts();
    }
    if !first.unify_directed_with_bank(&oriented_second, &mut subst, bank)? {
        return Ok(None);
    }

    let result = if eqn_is_maximal_under_subst(ocb, bank, clause, position.first_literal_index)? {
        build_ordered_factor(
            bank,
            clause,
            position.second_literal_index,
            freshvars,
            &mut subst,
        )
    } else {
        Ok(None)
    };
    subst.backtrack();
    result
}

/// Builds one equality-factor result for one candidate.
///
/// This is a convenience wrapper over C's result-stack shape. First-order mode
/// uses the complete-MGU path, while higher-order mode enumerates the CSU stack
/// and returns the first factor in C wrapper insertion order.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the generated
/// factor or if higher-order CSU limits have not been initialized.
///
/// # Panics
///
/// Panics if the candidate indices are invalid, select the same literal, select
/// a non-positive/non-maximal first literal, select a non-positive second
/// literal, or put an oriented first literal on its right side. These preserve
/// the internal-caller invariants encoded by the C clause-position API.
pub fn compute_equality_factor(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
) -> Result<Option<Clause>, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_equality_factor_impl(bank, ocb, clause, position, &freshvars)
}

/// Builds one equality-factor result using a caller-owned C `freshvars` bank.
///
/// Unlike ordered factoring, C `ComputeEqualityFactor` does not reset
/// `freshvars` before candidate construction; callers control the count state.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while copying the
/// generated factor or if higher-order CSU limits have not been initialized.
///
/// # Panics
///
/// Panics under the same internal-caller invariant violations as
/// [`compute_equality_factor`].
pub fn compute_equality_factor_with_fresh_vars(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    freshvars: &VarBank,
) -> Result<Option<Clause>, Diagnostic> {
    compute_equality_factor_impl(bank, ocb, clause, position, freshvars)
}

fn compute_equality_factor_impl(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    freshvars: &VarBank,
) -> Result<Option<Clause>, Diagnostic> {
    let (mut factors, _) = compute_equality_factor_factors(bank, ocb, clause, position, freshvars)?;
    Ok(factors.pop())
}

fn equality_factor_input(
    bank: &TermBank,
    clause: &Clause,
    position: EqualityFactorPosition,
) -> EqualityFactorInput {
    let literals = clause.literals().as_slice();
    assert_ne!(
        position.first_literal_index, position.second_literal_index,
        "equality factoring expects two distinct literals"
    );
    assert!(
        matches!(position.first_side, EqnSide::LeftSide | EqnSide::RightSide)
            && matches!(position.second_side, EqnSide::LeftSide | EqnSide::RightSide),
        "equality factoring sides must be left or right"
    );

    let first = literals
        .get(position.first_literal_index)
        .expect("equality-factor first literal index must be valid");
    let second = literals
        .get(position.second_literal_index)
        .expect("equality-factor second literal index must be valid");
    assert!(
        first.is_positive() && first.is_maximal(),
        "equality factoring expects a maximal positive first literal"
    );
    assert!(
        second.is_positive(),
        "equality factoring expects a positive second literal"
    );
    assert!(
        !first.is_oriented() || position.first_side == EqnSide::LeftSide,
        "oriented equality-factor first literal can only use its left side"
    );

    EqualityFactorInput {
        max_term: literal_side(first, position.first_side).clone(),
        with_term: literal_side(second, position.second_side).clone(),
        min_term: literal_other_side(first, position.first_side).clone(),
        second_other: literal_other_side(second, position.second_side).clone(),
        first_is_equ_lit: first.is_equ_lit(bank),
        second_is_equ_lit: second.is_equ_lit(bank),
    }
}

fn equality_factor_free_var_guard(input: &EqualityFactorInput) -> bool {
    (input.max_term.is_free_var() && !input.second_is_equ_lit)
        || (input.with_term.is_free_var() && !input.first_is_equ_lit)
}

fn equality_factor_order_allows(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    input: &EqualityFactorInput,
) -> Result<bool, Diagnostic> {
    if to_greater_with_bank(
        ocb,
        bank,
        &input.min_term,
        &input.max_term,
        DerefType::Always,
        DerefType::Always,
    )? {
        return Ok(false);
    }
    eqn_is_maximal_under_subst(ocb, bank, clause, position.first_literal_index)
}

fn compute_equality_factor_mgu(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    freshvars: &VarBank,
) -> Result<(Option<Clause>, bool), Diagnostic> {
    let input = equality_factor_input(bank, clause, position);
    if equality_factor_free_var_guard(&input) {
        return Ok((None, false));
    }

    let mut subst = Substitution::new();
    if !subst_mgu_complete_with_bank(bank, &input.max_term, &input.with_term, &mut subst)? {
        return Ok((None, false));
    }

    let subst_is_ho = subst.has_ho_binding();
    let result = if equality_factor_order_allows(bank, ocb, clause, position, &input)? {
        build_equality_factor(
            bank,
            clause,
            position,
            &input.min_term,
            &input.second_other,
            freshvars,
            &mut subst,
        )
    } else {
        Ok(None)
    };
    subst.backtrack();
    Ok((result?, subst_is_ho))
}

fn compute_equality_factor_csu_factors(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    freshvars: &VarBank,
) -> Result<(Vec<Clause>, bool), Diagnostic> {
    let input = equality_factor_input(bank, clause, position);
    if equality_factor_free_var_guard(&input) {
        return Ok((Vec::new(), false));
    }

    let mut subst = Substitution::new();
    let mut iter = CsuIterator::new(&input.max_term, &input.with_term, &subst);
    let mut factors = Vec::new();
    let mut subst_is_ho = false;

    loop {
        let has_next = match iter.next_csu_element(bank, &mut subst) {
            Ok(has_next) => has_next,
            Err(err) => {
                iter.destroy(&mut subst);
                return Err(err);
            }
        };
        if !has_next {
            break;
        }

        if equality_factor_order_allows(bank, ocb, clause, position, &input)? {
            subst_is_ho = subst.has_ho_binding();
            let factor = match build_equality_factor(
                bank,
                clause,
                position,
                &input.min_term,
                &input.second_other,
                freshvars,
                &mut subst,
            ) {
                Ok(Some(factor)) => factor,
                Ok(None) => continue,
                Err(err) => {
                    iter.destroy(&mut subst);
                    return Err(err);
                }
            };
            factors.push(factor);
        }
    }

    iter.destroy(&mut subst);
    Ok((factors, subst_is_ho))
}

fn compute_equality_factor_factors(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    position: EqualityFactorPosition,
    freshvars: &VarBank,
) -> Result<(Vec<Clause>, bool), Diagnostic> {
    if problem_type() == ProblemType::HigherOrder {
        compute_equality_factor_csu_factors(bank, ocb, clause, position, freshvars)
    } else {
        let (factor, subst_is_ho) =
            compute_equality_factor_mgu(bank, ocb, clause, position, freshvars)?;
        Ok((factor.into_iter().collect(), subst_is_ho))
    }
}

/// Computes all first-order ordered factors and inserts them into `store`.
///
/// This mirrors C `ComputeAllOrderedFactors` for the first-order ordered
/// factoring path, including the C derivation opcode and parent reference.
/// Use [`compute_all_ordered_factors_with_docs`] for represented
/// proof-documentation output.
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
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_all_ordered_factors_impl::<String>(bank, ocb, clause, store, &freshvars, false, None)
}

/// Computes all first-order ordered factors with a caller-owned C `freshvars`
/// bank and inserts them into `store`.
///
/// # Errors
///
/// Returns diagnostics from [`compute_ordered_factor_with_fresh_vars`].
pub fn compute_all_ordered_factors_with_fresh_vars(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_ordered_factors_impl::<String>(bank, ocb, clause, store, freshvars, true, None)
}

/// Computes all first-order ordered factors while emitting represented C
/// `DocClauseCreationDefault(..., inf_factor, ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_ordered_factors`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_ordered_factors_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let freshvars = fresh_var_bank_for_clause(bank, clause);
    compute_all_ordered_factors_impl(
        bank,
        ocb,
        clause,
        store,
        &freshvars,
        false,
        Some((output, session)),
    )
}

/// Computes all first-order ordered factors with a caller-owned C `freshvars`
/// bank while emitting represented C `DocClauseCreationDefault(...,
/// inf_factor, ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`compute_all_ordered_factors_with_fresh_vars`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_ordered_factors_with_fresh_vars_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_ordered_factors_impl(
        bank,
        ocb,
        clause,
        store,
        freshvars,
        true,
        Some((output, session)),
    )
}

fn compute_all_ordered_factors_impl<W: fmt::Write>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: &VarBank,
    reset_freshvars: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut factor_count = 0;
    if clause.is_horn() || clause.query_prop(CP_NO_GENERATION) {
        return Ok(factor_count);
    }

    for position in ordered_factor_positions(clause) {
        if let Some(mut factor) =
            compute_ordered_factor_impl(bank, ocb, clause, position, freshvars, reset_freshvars)?
        {
            factor_count += 1;
            factor.set_proof_depth(clause.proof_depth().saturating_add(1));
            factor.set_proof_size(clause.proof_size().saturating_add(1));
            factor.set_tptp_type(clause.query_tptp_type());
            factor.set_prop(clause.give_props(CP_IS_SOS));
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_creation(
                    &mut **output,
                    bank,
                    &mut factor,
                    ClauseCreationInference::Factoring,
                    ClauseCreationParents::unary(clause),
                    None,
                )?;
            }
            clause_push_derivation(&mut factor, DC_ORDERED_FACTOR, Some(clause), None);
            store.insert(factor);
        }
    }

    Ok(factor_count)
}

/// Computes all equality factors and inserts them into `store`.
///
/// This mirrors C `ComputeAllEqualityFactors`: first-order mode uses the
/// complete-MGU path, while higher-order mode enumerates the CSU stack through
/// `CsuIterator`. Use [`compute_all_equality_factors_with_docs`] for represented
/// proof-documentation output.
///
/// # Errors
///
/// Returns diagnostics from factor construction or higher-order CSU iteration.
pub fn compute_all_equality_factors(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    compute_all_equality_factors_impl::<String>(bank, ocb, clause, store, None, None)
}

/// Computes all equality factors with a caller-owned C `freshvars` bank and
/// inserts them into `store`.
///
/// # Errors
///
/// Returns diagnostics from factor construction or higher-order CSU iteration.
pub fn compute_all_equality_factors_with_fresh_vars(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_equality_factors_impl::<String>(bank, ocb, clause, store, Some(freshvars), None)
}

/// Computes all equality factors while emitting represented C
/// `DocClauseCreationDefault(..., inf_efactor, ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_equality_factors`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_equality_factors_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    compute_all_equality_factors_impl(bank, ocb, clause, store, None, Some((output, session)))
}

/// Computes all equality factors with a caller-owned C `freshvars` bank while
/// emitting represented C `DocClauseCreationDefault(..., inf_efactor, ...)`
/// output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`compute_all_equality_factors_with_fresh_vars`], plus any
/// proof-documentation write diagnostic.
pub fn compute_all_equality_factors_with_fresh_vars_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_equality_factors_impl(
        bank,
        ocb,
        clause,
        store,
        Some(freshvars),
        Some((output, session)),
    )
}

fn compute_all_equality_factors_impl<W: fmt::Write>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    store: &mut ClauseSet,
    freshvars: Option<&VarBank>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut factor_count = 0;
    if clause.is_horn() || clause.query_prop(CP_NO_GENERATION) {
        return Ok(factor_count);
    }

    for position in equality_factor_positions(clause) {
        let (mut factors, subst_is_ho) = if let Some(freshvars) = freshvars {
            compute_equality_factor_factors(bank, ocb, clause, position, freshvars)?
        } else {
            let scratch_freshvars = fresh_var_bank_for_clause(bank, clause);
            compute_equality_factor_factors(bank, ocb, clause, position, &scratch_freshvars)?
        };

        while let Some(mut factor) = factors.pop() {
            factor_count += 1;
            factor.set_proof_depth(clause.proof_depth().saturating_add(1));
            factor.set_proof_size(clause.proof_size().saturating_add(1));
            factor.set_tptp_type(clause.query_tptp_type());
            factor.set_prop(clause.give_props(CP_IS_SOS));
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_creation(
                    &mut **output,
                    bank,
                    &mut factor,
                    ClauseCreationInference::EqualityFactoring,
                    ClauseCreationParents::unary(clause),
                    None,
                )?;
            }
            let operation = if subst_is_ho {
                set_is_ho(DC_EQ_FACTOR)
            } else {
                DC_EQ_FACTOR
            };
            clause_push_derivation(&mut factor, operation, Some(clause), None);
            store.insert(factor);
        }
    }

    Ok(factor_count)
}

fn is_ordered_factor_candidate(literal: &Eqn) -> bool {
    literal.is_positive() && literal.is_maximal()
}

fn literal_side(literal: &Eqn, side: EqnSide) -> &Term {
    if side == EqnSide::LeftSide {
        literal.left()
    } else {
        literal.right()
    }
}

fn literal_other_side(literal: &Eqn, side: EqnSide) -> &Term {
    if side == EqnSide::LeftSide {
        literal.right()
    } else {
        literal.left()
    }
}

fn eqn_is_maximal_under_subst(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &Clause,
    target_index: usize,
) -> Result<bool, Diagnostic> {
    let literals = clause.literals().as_slice();
    let target = literals
        .get(target_index)
        .expect("maximality target index must be valid");
    for (index, candidate) in literals.iter().enumerate() {
        if index == target_index || !candidate.is_maximal() {
            continue;
        }
        if candidate.literal_compare_with_bank(ocb, bank, target)? == CompareResult::Greater {
            return Ok(false);
        }
    }
    Ok(true)
}

fn build_ordered_factor(
    bank: &mut TermBank,
    clause: &Clause,
    removed_literal_index: usize,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Option<Clause>, Diagnostic> {
    let backtrack = subst.len();
    let result = (|| {
        let _ = clause.literals().subst_norm_except_with_bank(
            Some(removed_literal_index),
            subst,
            freshvars,
            bank,
            problem_type(),
        )?;
        let mut new_literals = clause
            .literals()
            .copy_opt_except_index(Some(removed_literal_index), bank)?;
        new_literals.remove_resolved(bank);
        new_literals.remove_duplicates(bank);
        Ok(Some(Clause::alloc(new_literals)))
    })();
    subst.backtrack_to_pos(backtrack);
    result
}

fn build_equality_factor(
    bank: &mut TermBank,
    clause: &Clause,
    position: EqualityFactorPosition,
    min_term: &Term,
    second_other: &Term,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Option<Clause>, Diagnostic> {
    let backtrack = subst.len();
    let result = (|| {
        let _ = clause.literals().subst_norm_except_with_bank(
            Some(position.second_literal_index),
            subst,
            freshvars,
            bank,
            problem_type(),
        )?;
        let condition_left = bank.insert_no_props_cached(min_term, DerefType::Always)?;
        let condition_right = bank.insert_no_props_cached(second_other, DerefType::Always)?;
        let new_condition = Eqn::alloc(condition_left, condition_right, bank, false)?;
        let mut new_literals = clause
            .literals()
            .copy_opt_except_index(Some(position.first_literal_index), bank)?;
        new_literals.insert_first(new_condition);
        new_literals.lambda_normalize(bank)?;
        new_literals.remove_resolved(bank);
        new_literals.remove_duplicates(bank);
        Ok(Some(Clause::alloc(new_literals)))
    })();
    subst.backtrack_to_pos(backtrack);
    result
}

fn fresh_var_bank_for_clause(bank: &TermBank, clause: &Clause) -> VarBank {
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    VarBank::fresh_normalization_bank(
        bank.signature().type_bank(),
        bank.vars(),
        variables.values(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        compute_all_equality_factors, compute_all_equality_factors_with_docs,
        compute_all_ordered_factors, compute_equality_factor,
        compute_equality_factor_with_fresh_vars, compute_ordered_factor,
        compute_ordered_factor_with_fresh_vars, equality_factor_positions,
        ordered_factor_positions, EqualityFactorPosition, OrderedFactorPosition,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        set_is_ho, ClauseDerivationRef, DerivationEntry, DC_EQ_FACTOR, DC_ORDERED_FACTOR,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_MAX_IS_UP_TO_DATE};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::ho_csu::init_unif_limits;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

    fn init_unif_limits_for_test(unif_mode: UnifMode) {
        let mut parms = HeuristicParmsCell {
            unif_mode,
            ..HeuristicParmsCell::default()
        };
        parms.max_unifiers = 8;
        parms.max_unif_steps = 64;
        init_unif_limits(&parms);
    }

    fn init_branching_unif_limits_for_test() {
        let parms = HeuristicParmsCell {
            func_proj_limit: 1,
            imit_limit: 1,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            fixpoint_oracle: false,
            max_unifiers: 4,
            max_unif_steps: 32,
            ..HeuristicParmsCell::default()
        };
        init_unif_limits(&parms);
    }

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

    fn kbo6_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn kbo6_lambda_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LambdaOrder,
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

    fn typed_arrow_type(bank: &mut TermBank) -> crate::terms::simpletypes::Type {
        let type_ = bank.signature().type_bank().default_type();
        bank.signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_]))
    }

    fn typed_arrow_var(bank: &mut TermBank, f_code: i64) -> Term {
        let type_ = typed_arrow_type(bank);
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_arrow_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = typed_arrow_type(bank);
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

    fn typed_unary_n(bank: &mut TermBank, f_code: i64, mut arg: Term, repetitions: usize) -> Term {
        for _ in 0..repetitions {
            arg = typed_unary(bank, f_code, &arg);
        }
        arg
    }

    fn eta_expanded_arrow_const(bank: &mut TermBank, head: &Term) -> Term {
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(bank, head, std::slice::from_ref(&db0)).unwrap();
        close_with_type_prefix(bank, std::slice::from_ref(&i_type), &matrix).unwrap()
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
    fn equality_factor_positions_follow_c_side_and_partner_order() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "ef_iter_a");
        let b = typed_const(&mut bank, "ef_iter_b");
        let c = typed_const(&mut bank, "ef_iter_c");
        let d = typed_const(&mut bank, "ef_iter_d");
        let mut first = lit(&mut bank, &a, &b, true);
        let plain = lit(&mut bank, &b, &c, true);
        let mut second = lit(&mut bank, &c, &d, true);
        let mut negative_max = lit(&mut bank, &a, &d, false);

        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        second.set_prop(EP_IS_MAXIMAL);
        negative_max.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, plain, second, negative_max]));

        assert_eq!(
            equality_factor_positions(&clause),
            vec![
                EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
                EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::RightSide),
                EqualityFactorPosition::new(0, EqnSide::LeftSide, 2, EqnSide::LeftSide),
                EqualityFactorPosition::new(0, EqnSide::LeftSide, 2, EqnSide::RightSide),
                EqualityFactorPosition::new(2, EqnSide::LeftSide, 0, EqnSide::LeftSide),
                EqualityFactorPosition::new(2, EqnSide::LeftSide, 0, EqnSide::RightSide),
                EqualityFactorPosition::new(2, EqnSide::LeftSide, 1, EqnSide::LeftSide),
                EqualityFactorPosition::new(2, EqnSide::LeftSide, 1, EqnSide::RightSide),
                EqualityFactorPosition::new(2, EqnSide::RightSide, 0, EqnSide::LeftSide),
                EqualityFactorPosition::new(2, EqnSide::RightSide, 0, EqnSide::RightSide),
                EqualityFactorPosition::new(2, EqnSide::RightSide, 1, EqnSide::LeftSide),
                EqualityFactorPosition::new(2, EqnSide::RightSide, 1, EqnSide::RightSide),
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
    fn compute_ordered_factor_with_fresh_vars_resets_caller_bank_like_c() {
        let mut bank = test_bank();
        let freshvars = VarBank::new(bank.signature().type_bank());
        bank.vars().pair_shadow(&freshvars);
        let matched_var = typed_var(&bank, -2);
        let residual_var = typed_var(&bank, -4);
        let type_ = bank.signature().type_bank().default_type();
        let _ = freshvars.get_fresh_var(&type_);
        let _ = freshvars.get_fresh_var(&type_);
        let matched_const = typed_const(&mut bank, "of_fresh_a");
        let shared_const = typed_const(&mut bank, "of_fresh_b");
        let residual_const = typed_const(&mut bank, "of_fresh_c");
        let rest = lit(&mut bank, &residual_var, &residual_const, true);
        let mut factor_lit = lit(&mut bank, &matched_var, &shared_const, true);
        let mut removed_lit = lit(&mut bank, &matched_const, &shared_const, true);
        maximal(&mut factor_lit);
        maximal(&mut removed_lit);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, factor_lit, removed_lit]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_ordered_factor_with_fresh_vars(
            &mut bank,
            &mut ocb,
            &clause,
            OrderedFactorPosition::new(1, 2, EqnSide::LeftSide),
            &freshvars,
        )
        .unwrap()
        .expect("directed positive equalities should factor");

        let residual = factor
            .literals()
            .as_slice()
            .iter()
            .find(|literal| literal.right() == &residual_const)
            .expect("unbound residual literal should be copied");
        assert_eq!(residual.left().f_code(), -2);
        assert_eq!(freshvars.v_count_for_type(&type_), 1);
    }

    #[test]
    fn compute_equality_factor_adds_condition_and_removes_first_literal() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "ef_build_a");
        let b = typed_const(&mut bank, "ef_build_b");
        let c = typed_const(&mut bank, "ef_build_c");
        let f_code = typed_unary_code(&mut bank, "ef_build_f");
        let f_of_x = typed_unary(&mut bank, f_code, &x);
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut first = lit(&mut bank, &f_of_x, &a, true);
        let second = lit(&mut bank, &f_of_b, &c, true);
        maximal(&mut first);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_equality_factor(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .expect("matching positive equalities should equality-factor");

        assert!(x.binding().is_none());
        assert_eq!(factor.literal_number(), 2);
        let literals = factor.literals().as_slice();
        assert!(literals[0].is_positive());
        assert_eq!(literals[0].left(), &f_of_b);
        assert_eq!(literals[0].right(), &c);
        assert!(literals[1].is_negative());
        assert_eq!(literals[1].left(), &a);
        assert_eq!(literals[1].right(), &c);
    }

    #[test]
    fn compute_equality_factor_with_fresh_vars_keeps_caller_counts_like_c() {
        let mut bank = test_bank();
        let freshvars = VarBank::new(bank.signature().type_bank());
        bank.vars().pair_shadow(&freshvars);
        let type_ = bank.signature().type_bank().default_type();
        let matched_var = bank.vars().get_fresh_var(&type_);
        let residual_var = bank.vars().get_fresh_var(&type_);
        let _ = freshvars.get_fresh_var(&type_);
        let _ = freshvars.get_fresh_var(&type_);
        assert_eq!(freshvars.v_count_for_type(&type_), 2);
        let condition_const = typed_const(&mut bank, "ef_fresh_a");
        let match_const = typed_const(&mut bank, "ef_fresh_b");
        let partner_const = typed_const(&mut bank, "ef_fresh_c");
        let residual_const = typed_const(&mut bank, "ef_fresh_d");
        let f_code = typed_unary_code(&mut bank, "ef_fresh_f");
        let matched_app = typed_unary(&mut bank, f_code, &matched_var);
        let partner_app = typed_unary(&mut bank, f_code, &match_const);
        let rest = lit(&mut bank, &residual_var, &residual_const, true);
        let mut first = lit(&mut bank, &matched_app, &condition_const, true);
        let second = lit(&mut bank, &partner_app, &partner_const, true);
        maximal(&mut first);
        let clause = Clause::alloc(EqnList::from_vec(vec![rest, first, second]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_equality_factor_with_fresh_vars(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(1, EqnSide::LeftSide, 2, EqnSide::LeftSide),
            &freshvars,
        )
        .unwrap()
        .expect("matching positive equalities should equality-factor");

        let residual = factor
            .literals()
            .as_slice()
            .iter()
            .find(|literal| literal.right() == &residual_const)
            .expect("unbound residual literal should be copied");
        assert_eq!(freshvars.v_count_for_type(&type_), 3);
        assert_eq!(residual.left().f_code(), -6);
    }

    #[test]
    fn compute_equality_factor_lambda_normalizes_generated_literals() {
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let f = typed_arrow_const(&mut bank, "ef_lambda_f");
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(&mut bank, &f, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "ef_lambda_a");
        let c = typed_const(&mut bank, "ef_lambda_c");
        let g_code = typed_unary_code(&mut bank, "ef_lambda_g");
        let g_of_x = typed_unary(&mut bank, g_code, &x);
        let g_of_a = typed_unary(&mut bank, g_code, &a);
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&x)).unwrap();
        let expected = apply_terms(&mut bank, &f, std::slice::from_ref(&a)).unwrap();
        let mut first = lit(&mut bank, &g_of_x, &c, true);
        let second = lit(&mut bank, &g_of_a, &applied, true);
        maximal(&mut first);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_equality_factor(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .expect("matching positive equalities should equality-factor");

        assert!(x.binding().is_none());
        assert_eq!(factor.literal_number(), 2);
        let literals = factor.literals().as_slice();
        assert!(literals[0].is_positive());
        assert_eq!(literals[0].left(), &g_of_a);
        assert_eq!(literals[0].right(), &expected);
        assert!(literals[1].is_negative());
        assert_eq!(literals[1].left(), &c);
        assert_eq!(literals[1].right(), &expected);
    }

    #[test]
    fn compute_equality_factor_rejects_when_other_first_side_is_greater() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "ef_order_a");
        let b = typed_const(&mut bank, "ef_order_b");
        let c = typed_const(&mut bank, "ef_order_c");
        let f_code = typed_unary_code(&mut bank, "ef_order_f");
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut first = lit(&mut bank, &a, &f_of_b, true);
        let second = lit(&mut bank, &a, &c, true);
        maximal(&mut first);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);

        assert!(compute_equality_factor(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .is_none());
    }

    #[test]
    fn compute_equality_factor_uses_banked_lambda_ordering_for_side_check() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let f = typed_arrow_const(&mut bank, "ef_order_eta_f");
        let eta_f = eta_expanded_arrow_const(&mut bank, &f);
        let c = typed_arrow_const(&mut bank, "ef_order_eta_c");
        let mut first = lit(&mut bank, &f, &eta_f, true);
        let second = lit(&mut bank, &f, &c, true);
        maximal(&mut first);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo6_lambda_ocb(&bank);

        let factor = compute_equality_factor(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .expect("eta-equivalent side check should allow equality factoring");

        assert_eq!(factor.literal_number(), 2);
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
    fn compute_all_equality_factors_inserts_metadata_and_honors_gates() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "ef_all_a");
        let b = typed_const(&mut bank, "ef_all_b");
        let c = typed_const(&mut bank, "ef_all_c");
        let f_code = typed_unary_code(&mut bank, "ef_all_f");
        let f_of_x = typed_unary(&mut bank, f_code, &x);
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut first = lit(&mut bank, &f_of_x, &a, true);
        let second = lit(&mut bank, &f_of_b, &c, true);
        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        clause.set_proof_depth(2);
        clause.set_proof_size(7);
        clause.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        clause.set_prop(CP_IS_SOS);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_equality_factors(&mut bank, &mut ocb, &clause, &mut store).unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store.iter().next().expect("one equality factor inserted");
        assert_eq!(stored.proof_depth(), 3);
        assert_eq!(stored.proof_size(), 8);
        assert_eq!(stored.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(stored.query_prop(CP_IS_SOS));
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_EQ_FACTOR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
            ]
        );

        let horn = Clause::alloc(EqnList::from_vec(vec![lit(&mut bank, &a, &b, true)]));
        let mut horn_store = ClauseSet::new();
        assert_eq!(
            compute_all_equality_factors(&mut bank, &mut ocb, &horn, &mut horn_store).unwrap(),
            0
        );
        assert!(horn_store.is_empty());

        let mut blocked = clause.clone();
        blocked.set_prop(CP_NO_GENERATION);
        let mut blocked_store = ClauseSet::new();
        assert_eq!(
            compute_all_equality_factors(&mut bank, &mut ocb, &blocked, &mut blocked_store)
                .unwrap(),
            0
        );
        assert!(blocked_store.is_empty());
    }

    #[test]
    fn compute_all_equality_factors_higher_order_uses_first_order_subset() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Single);
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "ef_ho_fo_a");
        let b = typed_const(&mut bank, "ef_ho_fo_b");
        let c = typed_const(&mut bank, "ef_ho_fo_c");
        let f_code = typed_unary_code(&mut bank, "ef_ho_fo_f");
        let f_of_x = typed_unary(&mut bank, f_code, &x);
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut first = lit(&mut bank, &f_of_x, &a, true);
        let second = lit(&mut bank, &f_of_b, &c, true);
        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_equality_factors(&mut bank, &mut ocb, &clause, &mut store)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(count, 1);
        let factor = store.iter().next().expect("first-order subset factors");
        assert_eq!(factor.literal_number(), 2);
        assert_eq!(
            factor.derivation().unwrap().as_slice()[0],
            DerivationEntry::Operation(DC_EQ_FACTOR)
        );
    }

    #[test]
    fn compute_all_equality_factors_higher_order_enumerates_csu_pattern_results() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let i_type = bank.signature().type_bank().default_type();
        let function = typed_arrow_var(&mut bank, -2);
        let db0 = bank.request_db_var(&i_type, 0);
        let applied = apply_terms(&mut bank, &function, std::slice::from_ref(&db0)).unwrap();
        let a = typed_const(&mut bank, "ef_ho_csu_a");
        let b = typed_const(&mut bank, "ef_ho_csu_b");
        let c = typed_const(&mut bank, "ef_ho_csu_c");
        let mut first = lit(&mut bank, &applied, &a, true);
        let second = lit(&mut bank, &b, &c, true);
        first.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_equality_factors(&mut bank, &mut ocb, &clause, &mut store)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
        for factor in store.iter() {
            assert_eq!(
                factor.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(set_is_ho(DC_EQ_FACTOR)),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
                ]
            );
        }
    }

    #[test]
    fn compute_all_equality_factors_preserves_multi_csu_pop_and_doc_order() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_branching_unif_limits_for_test();
        let mut bank = test_bank();
        let function = typed_arrow_var(&mut bank, -2);
        let shared_argument = typed_const(&mut bank, "ef_multi_a");
        let projection_argument = typed_const(&mut bank, "ef_multi_b");
        let partner_other_side = typed_const(&mut bank, "ef_multi_c");
        let maximal_other_side = typed_const(&mut bank, "ef_multi_d");
        let negative_other_side = typed_const(&mut bank, "ef_multi_e");
        let q_code = typed_unary_code(&mut bank, "ef_multi_q");
        let function_on_shared =
            apply_terms(&mut bank, &function, std::slice::from_ref(&shared_argument)).unwrap();
        let function_on_projection = apply_terms(
            &mut bank,
            &function,
            std::slice::from_ref(&projection_argument),
        )
        .unwrap();
        let wrapped_function = typed_unary_n(&mut bank, q_code, function_on_shared, 3);
        let wrapped_partner = typed_unary_n(&mut bank, q_code, shared_argument, 3);
        let mut first = lit(&mut bank, &wrapped_function, &maximal_other_side, true);
        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let second = lit(&mut bank, &wrapped_partner, &partner_other_side, true);
        let third = lit(
            &mut bank,
            &function_on_projection,
            &negative_other_side,
            false,
        );
        let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second, third]));
        clause.set_ident(44);
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);

        let count = compute_all_equality_factors_with_docs(
            &mut output,
            &mut session,
            &mut bank,
            &mut ocb,
            &clause,
            &mut store,
        )
        .unwrap();

        assert_eq!(count, 2);
        let factors: Vec<_> = store.iter().collect();
        assert_eq!(factors.len(), 2);
        assert_eq!(factors[0].ident(), 1);
        assert_eq!(factors[1].ident(), 2);
        for factor in &factors {
            assert_eq!(
                factor.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(set_is_ho(DC_EQ_FACTOR)),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
                ]
            );
        }
        assert_eq!(
            factors[0]
                .literals()
                .tstp_print_string(&bank, " | ", true, false),
            "ef_multi_q(ef_multi_q(ef_multi_q(ef_multi_a)))=ef_multi_c | ef_multi_d!=ef_multi_c | ef_multi_b!=ef_multi_e"
        );
        assert_eq!(
            factors[1]
                .literals()
                .tstp_print_string(&bank, " | ", true, false),
            "ef_multi_q(ef_multi_q(ef_multi_q(ef_multi_a)))=ef_multi_c | ef_multi_d!=ef_multi_c | ef_multi_a!=ef_multi_e"
        );
        assert_eq!(
            output,
            "     1 : :[++equal(ef_multi_q(ef_multi_q(ef_multi_q(ef_multi_a))), ef_multi_c),--equal(ef_multi_d, ef_multi_c),--equal(ef_multi_b, ef_multi_e)] : ef(44)\n     2 : :[++equal(ef_multi_q(ef_multi_q(ef_multi_q(ef_multi_a))), ef_multi_c),--equal(ef_multi_d, ef_multi_c),--equal(ef_multi_a, ef_multi_e)] : ef(44)\n"
        );
    }

    #[test]
    fn compute_equality_factor_higher_order_arrow_binding_uses_csu_path() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let x = typed_arrow_var(&mut bank, -2);
        let f = typed_arrow_const(&mut bank, "ef_ho_arrow_f");
        let a = typed_arrow_const(&mut bank, "ef_ho_arrow_a");
        let b = typed_arrow_const(&mut bank, "ef_ho_arrow_b");
        let mut first = lit(&mut bank, &x, &a, true);
        let second = lit(&mut bank, &f, &b, true);
        first.set_prop(EP_IS_MAXIMAL);
        let clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);

        let factor = compute_equality_factor(
            &mut bank,
            &mut ocb,
            &clause,
            EqualityFactorPosition::new(0, EqnSide::LeftSide, 1, EqnSide::LeftSide),
        )
        .unwrap()
        .expect("higher-order equality factor should be generated");

        assert_eq!(factor.literal_number(), 2);
    }

    #[test]
    fn compute_all_equality_factors_with_docs_prints_creation_step() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "ef_doc_a");
        let b = typed_const(&mut bank, "ef_doc_b");
        let c = typed_const(&mut bank, "ef_doc_c");
        let f_code = typed_unary_code(&mut bank, "ef_doc_f");
        let f_of_x = typed_unary(&mut bank, f_code, &x);
        let f_of_b = typed_unary(&mut bank, f_code, &b);
        let mut first = lit(&mut bank, &f_of_x, &a, true);
        let second = lit(&mut bank, &f_of_b, &c, true);
        first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        clause.set_ident(44);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let count = compute_all_equality_factors_with_docs(
            &mut output,
            &mut session,
            &mut bank,
            &mut ocb,
            &clause,
            &mut store,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(output.contains(" : ef(44)\n"));
        let stored = store.iter().next().expect("one equality factor inserted");
        assert_eq!(stored.ident(), 1);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_EQ_FACTOR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
            ]
        );
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
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_ORDERED_FACTOR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&clause)),
            ]
        );

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
