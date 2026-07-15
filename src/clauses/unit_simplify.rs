use crate::basics::error::Diagnostic;
use crate::basics::simple_stuff::string_index_c;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROTECTED, CP_IS_SOS, CP_LIMITED_RW};
use crate::clauses::clausefunc::clause_remove_literal_index;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::EqnSide;
use crate::clauses::inferencedoc::{ClauseModificationInference, ProofDocSession};
use crate::clauses::pdtrees::{PdtIndexedOccurrence, PDTREE_IGNORE_NF_DATE};
use crate::clauses::subsumption::{eqn_topsubsumes_termpair, eqn_topsubsumes_termpair_with_bank};
use crate::terms::match_mgu::{subst_match_complete, subst_match_complete_with_bank};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum UnitSimplifyType {
    NoUnitSimplify = 0,
    TopLevelUnitSimplify = 1,
    FullUnitSimplify = 2,
}

pub const UNIT_SIMPLIFY_NAMES: [Option<&str>; 4] = [
    Some("NoSimplify"),
    Some("TopSimplify"),
    Some("FullSimplify"),
    None,
];

#[derive(Clone, Copy, Debug)]
pub struct SimplifyingUnit<'set> {
    clause: &'set Clause,
    literal_index: usize,
}

impl<'set> SimplifyingUnit<'set> {
    #[must_use]
    pub const fn clause(self) -> &'set Clause {
        self.clause
    }

    #[must_use]
    pub const fn literal_index(self) -> usize {
        self.literal_index
    }

    #[must_use]
    pub fn literal(self) -> &'set Eqn {
        &self.clause.literals().as_slice()[self.literal_index]
    }
}

#[must_use]
pub fn trans_unit_simplify_string(name: &str) -> Option<UnitSimplifyType> {
    match string_index_c(name, &UNIT_SIMPLIFY_NAMES) {
        0 => Some(UnitSimplifyType::NoUnitSimplify),
        1 => Some(UnitSimplifyType::TopLevelUnitSimplify),
        2 => Some(UnitSimplifyType::FullUnitSimplify),
        _ => None,
    }
}

#[must_use]
pub fn find_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<SimplifyingUnit<'set>> {
    find_top_simplifying_unit_with_sign(units, left, right, None)
}

#[must_use]
pub fn find_signed_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: bool,
) -> Option<SimplifyingUnit<'set>> {
    find_top_simplifying_unit_with_sign(units, left, right, Some(sign))
}

/// Bank-aware C `FindTopSimplifyingUnit` for higher-order complete matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn find_top_simplifying_unit_with_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    find_top_simplifying_unit_with_sign_and_bank(bank, units, left, right, None)
}

/// Bank-aware C `FindSignedTopSimplifyingUnit` for higher-order complete
/// matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn find_signed_top_simplifying_unit_with_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: bool,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    find_top_simplifying_unit_with_sign_and_bank(bank, units, left, right, Some(sign))
}

#[must_use]
/// # Panics
///
/// Panics if matching nonvariable terms report an arity but do not expose
/// initialized arguments. This is an internal term-bank invariant.
pub fn find_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive_only: bool,
) -> Option<SimplifyingUnit<'set>> {
    if positive_only {
        if let Some(result) = find_signed_top_simplifying_unit(units, left, right, true) {
            return Some(result);
        }
    } else if let Some(result) = find_top_simplifying_unit(units, left, right) {
        return Some(result);
    }

    let mut current_left = left.clone();
    let mut current_right = right.clone();
    while !current_left.is_top_level_free_var()
        && !current_right.is_top_level_free_var()
        && !current_left.is_lambda()
        && !current_right.is_lambda()
        && current_left.f_code() == current_right.f_code()
        && current_left.arity() != 0
    {
        debug_assert_ne!(current_left, current_right);
        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return None;
                }
                differing_pair = Some((next_left, next_right));
            }
        }

        let (next_left, next_right) = differing_pair?;
        current_left = next_left;
        current_right = next_right;
        if let Some(result) =
            find_signed_top_simplifying_unit(units, &current_left, &current_right, true)
        {
            return Some(result);
        }
    }
    None
}

/// Bank-aware C `FindSimplifyingUnit` for higher-order complete matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if matching nonvariable terms report an arity but do not expose
/// initialized arguments. This is an internal term-bank invariant.
pub fn find_simplifying_unit_with_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive_only: bool,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    if positive_only {
        if let Some(result) =
            find_signed_top_simplifying_unit_with_bank(bank, units, left, right, true)?
        {
            return Ok(Some(result));
        }
    } else if let Some(result) = find_top_simplifying_unit_with_bank(bank, units, left, right)? {
        return Ok(Some(result));
    }

    let mut current_left = left.clone();
    let mut current_right = right.clone();
    while !current_left.is_top_level_free_var()
        && !current_right.is_top_level_free_var()
        && !current_left.is_lambda()
        && !current_right.is_lambda()
        && current_left.f_code() == current_right.f_code()
        && current_left.arity() != 0
    {
        debug_assert_ne!(current_left, current_right);
        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return Ok(None);
                }
                differing_pair = Some((next_left, next_right));
            }
        }

        let Some((next_left, next_right)) = differing_pair else {
            return Ok(None);
        };
        current_left = next_left;
        current_right = next_right;
        if let Some(result) = find_signed_top_simplifying_unit_with_bank(
            bank,
            units,
            &current_left,
            &current_right,
            true,
        )? {
            return Ok(Some(result));
        }
    }
    Ok(None)
}

/// Simplifies `clause` with a unit set, matching C
/// `ClauseSimplifyWithUnitSet` aside from full `PDTreeFindNextDemodulator`
/// traversal over live clause positions.
///
/// Returns `false` when a same-signed unit subsumes the clause, otherwise
/// returns `true` after applying all opposite-signed unit cuts.
///
/// # Panics
///
/// Panics for [`UnitSimplifyType::NoUnitSimplify`], matching the C assertion
/// that the caller selects either top-level or full unit simplification.
#[must_use]
pub fn clause_simplify_with_unit_set(
    clause: &mut Clause,
    unit_set: &mut ClauseSet,
    how: UnitSimplifyType,
) -> bool {
    clause_simplify_with_unit_set_impl::<String>(clause, unit_set, how, None)
        .expect("undocumented unit simplification cannot fail")
}

/// Simplifies `clause` with a unit set and emits the proof-documentation side
/// effects from C `ClauseSimplifyWithUnitSet`.
///
/// Same-signed unit subsumption is documented with `DocClauseQuote` at target
/// level 6 and opposite-signed cuts with `DocClauseModification` using
/// simplify-reflect, matching the C comments.
///
/// # Errors
///
/// Returns a diagnostic if the selected proof-documentation renderer fails.
///
/// # Panics
///
/// Panics for [`UnitSimplifyType::NoUnitSimplify`], matching the C assertion
/// that the caller selects either top-level or full unit simplification.
pub fn clause_simplify_with_unit_set_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &TermBank,
    clause: &mut Clause,
    unit_set: &mut ClauseSet,
    how: UnitSimplifyType,
) -> Result<bool, Diagnostic> {
    clause_simplify_with_unit_set_impl(
        clause,
        unit_set,
        how,
        Some(UnitSimplifyDocContext {
            output,
            session,
            bank,
        }),
    )
}

struct UnitSimplifyDocContext<'doc, W: fmt::Write> {
    output: &'doc mut W,
    session: &'doc mut ProofDocSession,
    bank: &'doc TermBank,
}

fn clause_simplify_with_unit_set_impl<W: fmt::Write>(
    clause: &mut Clause,
    unit_set: &mut ClauseSet,
    how: UnitSimplifyType,
    mut doc_context: Option<UnitSimplifyDocContext<'_, W>>,
) -> Result<bool, Diagnostic> {
    assert_ne!(
        how,
        UnitSimplifyType::NoUnitSimplify,
        "unit simplification mode must not be NoUnitSimplify"
    );

    let mut index = 0;
    while index < clause.literal_number() {
        let (left, right, sign) = {
            let literal = &clause.literals().as_slice()[index];
            (
                literal.left().clone(),
                literal.right().clone(),
                literal.is_positive(),
            )
        };
        let simplifier_index = match how {
            UnitSimplifyType::NoUnitSimplify => unreachable!(),
            UnitSimplifyType::TopLevelUnitSimplify => {
                find_top_simplifying_unit_index(unit_set, &left, &right, None)
            }
            UnitSimplifyType::FullUnitSimplify => {
                find_simplifying_unit_index(unit_set, &left, &right, false)
            }
        };

        let Some(simplifier_index) = simplifier_index else {
            index += 1;
            continue;
        };
        let simplifier_sign = unit_set
            .iter()
            .nth(simplifier_index)
            .and_then(unit_literal)
            .expect("simplifying unit index must select a unit literal")
            .is_positive();

        if sign == simplifier_sign {
            if let Some(context) = doc_context.as_mut() {
                let simplifier = unit_set
                    .iter()
                    .nth(simplifier_index)
                    .expect("simplifying unit index must select a clause");
                context.session.doc_clause_quote(
                    context.output,
                    context.bank,
                    6,
                    clause,
                    Some("subsumed by unprocessed unit"),
                    Some(simplifier),
                )?;
            }
            let protect_unit = !clause.is_unit()
                && clause.standard_weight()
                    == unit_set
                        .iter()
                        .nth(simplifier_index)
                        .expect("simplifying unit index must select a clause")
                        .standard_weight();
            let c_sos_as_property = if clause.query_prop(CP_IS_SOS) {
                CP_INITIAL
            } else {
                crate::clauses::clause_props::CP_IGNORE_PROPS
            };
            let simplifier = unit_set
                .iter_mut()
                .nth(simplifier_index)
                .expect("simplifying unit index must select a mutable clause");
            if protect_unit {
                simplifier.set_prop(CP_IS_PROTECTED);
            }
            simplifier.set_prop(c_sos_as_property);
            return Ok(false);
        }

        clause.del_prop(CP_LIMITED_RW);
        let removed = clause_remove_literal_index(clause, index);
        debug_assert!(removed.is_some(), "current literal must be removable");
        if let Some(context) = doc_context.as_mut() {
            let simplifier = unit_set
                .iter()
                .nth(simplifier_index)
                .expect("simplifying unit index must select a clause");
            context.session.doc_clause_modification(
                context.output,
                context.bank,
                clause,
                ClauseModificationInference::SimplifyReflect,
                Some(simplifier),
                Some("cut with unprocessed unit"),
            )?;
        }
    }
    Ok(true)
}

fn find_top_simplifying_unit_with_sign<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    units.record_demod_index_search_init(left, PDTREE_IGNORE_NF_DATE, false);
    let result = if units.demod_index_search_may_have_match() {
        if units.demod_index_search_uses_compact_candidates() {
            find_indexed_top_simplifying_unit(units, left, right, sign)
        } else {
            find_plain_top_simplifying_unit(units, left, right, sign)
        }
    } else {
        None
    };
    units.record_demod_index_search_exit();
    result
}

fn find_top_simplifying_unit_with_sign_and_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    units.record_demod_index_search_init_with_bank(bank, left, PDTREE_IGNORE_NF_DATE, false)?;
    let result = if units.demod_index_search_may_have_match() {
        if units.demod_index_search_uses_compact_candidates() {
            find_indexed_top_simplifying_unit_with_bank(bank, units, left, right, sign)
        } else {
            find_plain_top_simplifying_unit_with_bank(bank, units, left, right, sign)
        }
    } else {
        Ok(None)
    };
    units.record_demod_index_search_exit();
    result
}

fn find_indexed_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    while let Some(candidate) = units.demod_index_search_next_candidate_side() {
        let Some(clause) = units.find_indexed_by_id(candidate.clause_id) else {
            continue;
        };
        let Some(literal) = unit_literal(clause) else {
            continue;
        };
        if sign.is_some_and(|required| literal.is_positive() != required) {
            continue;
        }
        if unit_literal_occurrence_matches_top_pair(candidate, literal, left, right) {
            return Some(SimplifyingUnit {
                clause,
                literal_index: 0,
            });
        }
    }
    None
}

fn find_indexed_top_simplifying_unit_with_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    while let Some(candidate) = units.demod_index_search_next_candidate_side() {
        let Some(clause) = units.find_indexed_by_id(candidate.clause_id) else {
            continue;
        };
        let Some(literal) = unit_literal(clause) else {
            continue;
        };
        if sign.is_some_and(|required| literal.is_positive() != required) {
            continue;
        }
        if unit_literal_occurrence_matches_top_pair_with_bank(
            bank, candidate, literal, left, right,
        )? {
            return Ok(Some(SimplifyingUnit {
                clause,
                literal_index: 0,
            }));
        }
    }
    Ok(None)
}

fn find_plain_top_simplifying_unit<'set>(
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<SimplifyingUnit<'set>> {
    units.iter().find_map(|clause| {
        let literal = unit_literal(clause)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        eqn_topsubsumes_termpair(literal, left, right).then_some(SimplifyingUnit {
            clause,
            literal_index: 0,
        })
    })
}

fn find_plain_top_simplifying_unit_with_bank<'set>(
    bank: &mut TermBank,
    units: &'set ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Result<Option<SimplifyingUnit<'set>>, Diagnostic> {
    for clause in units.iter() {
        let Some(literal) = unit_literal(clause) else {
            continue;
        };
        if sign.is_some_and(|required| literal.is_positive() != required) {
            continue;
        }
        if eqn_topsubsumes_termpair_with_bank(bank, literal, left, right)? {
            return Ok(Some(SimplifyingUnit {
                clause,
                literal_index: 0,
            }));
        }
    }
    Ok(None)
}

fn unit_literal_occurrence_matches_top_pair(
    occurrence: PdtIndexedOccurrence,
    literal: &Eqn,
    left: &Term,
    right: &Term,
) -> bool {
    match occurrence.side {
        EqnSide::NoSide => false,
        EqnSide::LeftSide => {
            unit_literal_side_matches_top_pair(literal.left(), literal.right(), left, right)
        }
        EqnSide::RightSide => {
            unit_literal_side_matches_top_pair(literal.right(), literal.left(), left, right)
        }
        EqnSide::BothSides => {
            unit_literal_side_matches_top_pair(literal.left(), literal.right(), left, right)
                || unit_literal_side_matches_top_pair(literal.right(), literal.left(), left, right)
        }
    }
}

fn unit_literal_occurrence_matches_top_pair_with_bank(
    bank: &mut TermBank,
    occurrence: PdtIndexedOccurrence,
    literal: &Eqn,
    left: &Term,
    right: &Term,
) -> Result<bool, Diagnostic> {
    match occurrence.side {
        EqnSide::NoSide => Ok(false),
        EqnSide::LeftSide => unit_literal_side_matches_top_pair_with_bank(
            bank,
            literal.left(),
            literal.right(),
            left,
            right,
        ),
        EqnSide::RightSide => unit_literal_side_matches_top_pair_with_bank(
            bank,
            literal.right(),
            literal.left(),
            left,
            right,
        ),
        EqnSide::BothSides => {
            if unit_literal_side_matches_top_pair_with_bank(
                bank,
                literal.left(),
                literal.right(),
                left,
                right,
            )? {
                Ok(true)
            } else {
                unit_literal_side_matches_top_pair_with_bank(
                    bank,
                    literal.right(),
                    literal.left(),
                    left,
                    right,
                )
            }
        }
    }
}

fn unit_literal_side_matches_top_pair(
    indexed_side: &Term,
    other_side: &Term,
    left: &Term,
    right: &Term,
) -> bool {
    let mut subst = Substitution::new();
    let result = subst_match_complete(indexed_side, left, &mut subst)
        && subst_match_complete(other_side, right, &mut subst);
    subst.backtrack();
    result
}

fn unit_literal_side_matches_top_pair_with_bank(
    bank: &mut TermBank,
    indexed_side: &Term,
    other_side: &Term,
    left: &Term,
    right: &Term,
) -> Result<bool, Diagnostic> {
    let mut subst = Substitution::new();
    let result = match subst_match_complete_with_bank(bank, indexed_side, left, &mut subst) {
        Ok(true) => subst_match_complete_with_bank(bank, other_side, right, &mut subst),
        Ok(false) => Ok(false),
        Err(error) => Err(error),
    };
    subst.backtrack();
    result
}

fn find_indexed_top_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    while let Some(candidate) = units.demod_index_search_next_candidate_side() {
        let Some((index, clause)) = units.find_indexed_position_by_id(candidate.clause_id) else {
            continue;
        };
        let Some(literal) = unit_literal(clause) else {
            continue;
        };
        if sign.is_some_and(|required| literal.is_positive() != required) {
            continue;
        }
        if unit_literal_occurrence_matches_top_pair(candidate, literal, left, right) {
            return Some(index);
        }
    }
    None
}

fn find_plain_top_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    units.iter().enumerate().find_map(|(index, candidate)| {
        let literal = unit_literal(candidate)?;
        if sign.is_some_and(|required| literal.is_positive() != required) {
            return None;
        }
        eqn_topsubsumes_termpair(literal, left, right).then_some(index)
    })
}

fn find_top_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    sign: Option<bool>,
) -> Option<usize> {
    units.record_demod_index_search_init(left, PDTREE_IGNORE_NF_DATE, false);
    let result = if units.demod_index_search_may_have_match() {
        if units.demod_index_search_uses_compact_candidates() {
            find_indexed_top_simplifying_unit_index(units, left, right, sign)
        } else {
            find_plain_top_simplifying_unit_index(units, left, right, sign)
        }
    } else {
        None
    };
    units.record_demod_index_search_exit();
    result
}

fn find_simplifying_unit_index(
    units: &ClauseSet,
    left: &Term,
    right: &Term,
    positive_only: bool,
) -> Option<usize> {
    if positive_only {
        if let Some(result) = find_top_simplifying_unit_index(units, left, right, Some(true)) {
            return Some(result);
        }
    } else if let Some(result) = find_top_simplifying_unit_index(units, left, right, None) {
        return Some(result);
    }

    let mut current_left = left.clone();
    let mut current_right = right.clone();
    while !current_left.is_top_level_free_var()
        && !current_right.is_top_level_free_var()
        && !current_left.is_lambda()
        && !current_right.is_lambda()
        && current_left.f_code() == current_right.f_code()
        && current_left.arity() != 0
    {
        debug_assert_ne!(current_left, current_right);
        let mut differing_pair = None;
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                if differing_pair.is_some() {
                    return None;
                }
                differing_pair = Some((next_left, next_right));
            }
        }

        let (next_left, next_right) = differing_pair?;
        current_left = next_left;
        current_right = next_right;
        if let Some(result) =
            find_top_simplifying_unit_index(units, &current_left, &current_right, Some(true))
        {
            return Some(result);
        }
    }
    None
}

fn unit_literal(clause: &Clause) -> Option<&Eqn> {
    clause
        .is_unit()
        .then(|| clause.literals().as_slice().first())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::{
        clause_simplify_with_unit_set, clause_simplify_with_unit_set_with_docs,
        find_signed_top_simplifying_unit, find_simplifying_unit, find_top_simplifying_unit,
        find_top_simplifying_unit_with_bank, trans_unit_simplify_string, UnitSimplifyType,
    };
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_IS_PROTECTED, CP_IS_SOS, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::EP_IS_ORIENTED;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::clauses::pdtrees::PdtTraversalOrder;
    use crate::terms::lambda::apply_terms;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_.clone())
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    #[test]
    fn unit_simplify_names_match_c_table() {
        assert_eq!(
            trans_unit_simplify_string("NoSimplify"),
            Some(UnitSimplifyType::NoUnitSimplify)
        );
        assert_eq!(
            trans_unit_simplify_string("TopSimplify"),
            Some(UnitSimplifyType::TopLevelUnitSimplify)
        );
        assert_eq!(
            trans_unit_simplify_string("FullSimplify"),
            Some(UnitSimplifyType::FullUnitSimplify)
        );
        assert_eq!(trans_unit_simplify_string("missing"), None);
    }

    #[test]
    fn find_simplifying_unit_descends_only_after_top_lookup_fails() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "unit_a");
        let b = typed_const(&mut bank, "unit_b");
        let left = typed_unary(&mut bank, "unit_f", &b);
        let right = typed_unary(&mut bank, "unit_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let set = ClauseSet::from_clauses([positive_unit]);

        assert!(find_top_simplifying_unit(&set, &left, &right).is_none());
        assert_eq!(
            find_simplifying_unit(&set, &left, &right, false)
                .map(|unit| unit.literal().is_positive()),
            Some(true)
        );
        assert_eq!(
            find_signed_top_simplifying_unit(&set, &b, &a, false).map(|unit| unit.clause().ident()),
            None
        );
    }

    #[test]
    fn indexed_top_lookup_records_demodulator_search_attempts() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "unit_count_a");
        let b = typed_const(&mut bank, "unit_count_b");
        let mut positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut set = ClauseSet::new_demod_indexed();

        positive_unit.set_weight(positive_unit.standard_weight());
        set.indexed_insert_clause_owned(positive_unit, &bank);

        assert_eq!(set.demod_index_match_count(), 0);
        assert!(find_top_simplifying_unit(&set, &b, &a).is_some());
        assert_eq!(set.demod_index_match_count(), 1);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert!(!set.demod_index_search_active());
        assert_eq!(set.demod_index_search_state(), None);
        assert!(find_signed_top_simplifying_unit(&set, &b, &a, false).is_none());
        assert_eq!(set.demod_index_match_count(), 2);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert!(!set.demod_index_search_active());
    }

    #[test]
    fn banked_top_lookup_matches_higher_order_applied_variables() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let flex = bank.vars().get_fresh_var(&unary);
        let argument = typed_const(&mut bank, "unit_ho_argument");
        let other_side = typed_const(&mut bank, "unit_ho_other_side");
        let flex_application =
            apply_terms(&mut bank, &flex, std::slice::from_ref(&argument)).unwrap();
        let rigid_code = bank.signature_mut().insert_id("unit_ho_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, unary)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let rigid_application =
            apply_terms(&mut bank, &rigid, std::slice::from_ref(&argument)).unwrap();
        let matcher = typed_unary(&mut bank, "unit_ho_outer", &flex_application);
        let target = typed_unary(&mut bank, "unit_ho_outer", &rigid_application);
        let unit = clause_from(vec![literal(&mut bank, &matcher, &other_side, true)]);
        let set = ClauseSet::from_clauses([unit]);

        assert!(find_top_simplifying_unit(&set, &target, &other_side).is_none());
        let found =
            find_top_simplifying_unit_with_bank(&mut bank, &set, &target, &other_side).unwrap();

        assert!(found.is_some());
        assert!(flex.binding().is_none());
    }

    #[test]
    fn indexed_top_lookup_uses_pdt_candidate_side_direction() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "unit_side_a");
        let b = typed_const(&mut bank, "unit_side_b");
        let mut unit_lit = literal(&mut bank, &a, &b, true);
        unit_lit.set_prop(EP_IS_ORIENTED);
        let mut unit = clause_from(vec![unit_lit]);
        unit.set_weight(unit.standard_weight());
        let mut indexed = ClauseSet::new_demod_indexed();

        indexed.indexed_insert_clause_owned(unit.clone(), &bank);

        assert!(find_top_simplifying_unit(&indexed, &a, &b).is_some());
        assert!(find_top_simplifying_unit(&indexed, &b, &a).is_none());

        let plain = ClauseSet::from_clauses([unit]);
        assert!(find_top_simplifying_unit(&plain, &b, &a).is_some());
    }

    #[test]
    fn indexed_top_lookup_uses_pdt_candidate_order_before_set_order() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -11);
        let a = typed_const(&mut bank, "unit_order_a");
        let rhs = typed_const(&mut bank, "unit_order_rhs");
        let mut specific = clause_from(vec![literal(&mut bank, &a, &rhs, true)]);
        let specific_id = specific.ident();
        let mut general = clause_from(vec![literal(&mut bank, &variable, &rhs, true)]);
        let general_id = general.ident();
        let mut indexed = ClauseSet::new_demod_indexed();

        specific.set_weight(specific.standard_weight());
        general.set_weight(general.standard_weight());
        indexed.indexed_insert_clause_owned(specific.clone(), &bank);
        indexed.indexed_insert_clause_owned(general.clone(), &bank);

        assert_eq!(
            find_top_simplifying_unit(&indexed, &a, &rhs).map(|unit| unit.clause().ident()),
            Some(general_id)
        );

        let plain = ClauseSet::from_clauses([specific, general]);
        assert_eq!(
            find_top_simplifying_unit(&plain, &a, &rhs).map(|unit| unit.clause().ident()),
            Some(specific_id)
        );
    }

    #[test]
    fn clause_simplify_with_unit_set_removes_opposite_signed_literals() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "cut_a");
        let b = typed_const(&mut bank, "cut_b");
        let c = typed_const(&mut bank, "cut_c");
        let left = typed_unary(&mut bank, "cut_f", &b);
        let right = typed_unary(&mut bank, "cut_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &c, &b, true),
        ]);
        target.set_weight(target.standard_weight());
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::FullUnitSimplify
        ));

        assert_eq!(target.literal_number(), 1);
        assert!(target.literals().as_slice()[0].is_positive());
        assert!(target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
        assert_eq!(target.weight(), target.standard_weight());
    }

    #[test]
    fn top_level_unit_simplify_does_not_descend() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "top_a");
        let b = typed_const(&mut bank, "top_b");
        let left = typed_unary(&mut bank, "top_f", &b);
        let right = typed_unary(&mut bank, "top_f", &a);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &left, &right, false)]);

        assert!(clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::TopLevelUnitSimplify
        ));

        assert_eq!(target.literal_number(), 1);
        assert!(target.literals().as_slice()[0].is_negative());
    }

    #[test]
    fn same_signed_unit_subsumes_and_preserves_c_sos_property_bug() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "same_a");
        let b = typed_const(&mut bank, "same_b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let unit_id = positive_unit.ident();
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &b, &a, true)]);
        target.set_prop(CP_IS_SOS);

        assert!(!clause_simplify_with_unit_set(
            &mut target,
            &mut unit_set,
            UnitSimplifyType::TopLevelUnitSimplify
        ));

        let unit = unit_set.find_by_id(unit_id).unwrap();
        assert!(unit.query_prop(CP_INITIAL));
        assert!(!unit.query_prop(CP_IS_SOS));
        assert!(!unit.query_prop(CP_IS_PROTECTED));
        assert_eq!(target.literal_number(), 1);
    }

    #[test]
    fn documented_unit_cut_emits_simplify_reflect_modification() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "doc_cut_a");
        let b = typed_const(&mut bank, "doc_cut_b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let unit_id = positive_unit.ident();
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &b, &a, false)]);
        let target_id = target.ident();
        target.set_weight(target.standard_weight());
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        assert!(clause_simplify_with_unit_set_with_docs(
            &mut rendered,
            &mut session,
            &bank,
            &mut target,
            &mut unit_set,
            UnitSimplifyType::FullUnitSimplify,
        )
        .unwrap());

        assert_eq!(target.literal_number(), 0);
        assert_eq!(target.ident(), 1);
        assert!(target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
        assert_eq!(
            rendered,
            format!("     1 : :[] : sr({target_id},{unit_id}) : 'cut with unprocessed unit'\n")
        );
    }

    #[test]
    fn documented_same_signed_unit_subsumption_emits_c_quote() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let a = typed_const(&mut bank, "doc_same_a");
        let b = typed_const(&mut bank, "doc_same_b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &a, true)]);
        let unit_id = positive_unit.ident();
        let mut unit_set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &b, &a, true)]);
        let target_id = target.ident();
        target.set_prop(CP_IS_SOS);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        assert!(!clause_simplify_with_unit_set_with_docs(
            &mut rendered,
            &mut session,
            &bank,
            &mut target,
            &mut unit_set,
            UnitSimplifyType::TopLevelUnitSimplify,
        )
        .unwrap());

        let unit = unit_set.find_by_id(unit_id).unwrap();
        assert_eq!(target.ident(), 1);
        assert_eq!(target.literal_number(), 1);
        assert!(unit.query_prop(CP_INITIAL));
        assert!(!unit.query_prop(CP_IS_SOS));
        assert!(rendered.contains(&format!(
            " : {target_id} : 'subsumed by unprocessed unit({unit_id})'\n"
        )));
    }
}
