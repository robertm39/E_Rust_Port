use crate::basics::error::Diagnostic;
use crate::basics::pstacks::PStack;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{CP_INITIAL, CP_IS_SOS, CP_LIMITED_RW};
use crate::clauses::clausefunc::clause_remove_literal_index;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_SR};
use crate::clauses::eqn::Eqn;
use crate::clauses::fcvindexing::{fv_index_pack_clause, FvIndex, FvIndexAnchor};
use crate::clauses::freqvectors::FvPackedClause;
use crate::clauses::inferencedoc::{ClauseModificationInference, ProofDocSession};
use crate::clauses::unit_simplify::{
    find_signed_top_simplifying_unit, find_signed_top_simplifying_unit_with_bank,
    find_simplifying_unit, find_simplifying_unit_with_bank, SimplifyingUnit,
};
use crate::terms::match_mgu::{subst_match_complete, subst_match_complete_with_bank};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering};

static CLAUSE_CLAUSE_SUBSUMPTION_CALLS: AtomicI64 = AtomicI64::new(0);
static CLAUSE_CLAUSE_SUBSUMPTION_CALLS_REC: AtomicI64 = AtomicI64::new(0);
static CLAUSE_CLAUSE_SUBSUMPTION_SUCCESSES: AtomicI64 = AtomicI64::new(0);
static UNIT_CLAUSE_CLAUSE_SUBSUMPTION_CALLS: AtomicI64 = AtomicI64::new(0);

#[must_use]
pub fn clause_clause_subsumption_calls() -> i64 {
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS.load(Ordering::SeqCst)
}

#[must_use]
pub fn clause_clause_subsumption_calls_rec() -> i64 {
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS_REC.load(Ordering::SeqCst)
}

#[must_use]
pub fn clause_clause_subsumption_successes() -> i64 {
    CLAUSE_CLAUSE_SUBSUMPTION_SUCCESSES.load(Ordering::SeqCst)
}

#[must_use]
pub fn unit_clause_clause_subsumption_calls() -> i64 {
    UNIT_CLAUSE_CLAUSE_SUBSUMPTION_CALLS.load(Ordering::SeqCst)
}

#[must_use]
pub fn eqn_topsubsumes_termpair(eqn: &Eqn, left: &Term, right: &Term) -> bool {
    let mut subst = Substitution::new();
    let result = if subst_match_complete(eqn.left(), left, &mut subst) {
        subst_match_complete(eqn.right(), right, &mut subst)
    } else {
        subst_match_complete(eqn.left(), right, &mut subst)
            && subst_match_complete(eqn.right(), left, &mut subst)
    };
    subst.backtrack();
    result
}

/// Bank-aware top-level equation subsumption using C `SubstMatchComplete` in
/// higher-order mode.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization. All
/// temporary bindings are removed before returning.
pub fn eqn_topsubsumes_termpair_with_bank(
    bank: &mut TermBank,
    eqn: &Eqn,
    left: &Term,
    right: &Term,
) -> Result<bool, Diagnostic> {
    let mut subst = Substitution::new();
    let result = match subst_match_complete_with_bank(bank, eqn.left(), left, &mut subst) {
        Ok(true) => subst_match_complete_with_bank(bank, eqn.right(), right, &mut subst),
        Ok(false) => match subst_match_complete_with_bank(bank, eqn.left(), right, &mut subst) {
            Ok(true) => subst_match_complete_with_bank(bank, eqn.right(), left, &mut subst),
            Ok(false) => Ok(false),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    subst.backtrack();
    result
}

/// Returns whether `eqn` subsumes the term pair, following the C descent
/// through equal top contexts with at most one differing argument pair.
///
/// # Panics
///
/// Panics if equal-headed non-phony terms have different arities, matching the
/// non-LFHO C assertion in `eqn_subsumes_termpair`.
#[must_use]
pub fn eqn_subsumes_termpair(eqn: &Eqn, left: &Term, right: &Term) -> bool {
    let mut current_left = left.clone();
    let mut current_right = right.clone();

    loop {
        if eqn_topsubsumes_termpair(eqn, &current_left, &current_right) {
            return true;
        }
        if current_left.is_phony_app()
            || current_right.is_phony_app()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            return false;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "non-LFHO subsumption descent expects equal arities"
        );

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
                    return false;
                }
                differing_pair = Some((next_left, next_right));
            }
        }
        let Some((next_left, next_right)) = differing_pair else {
            return true;
        };
        current_left = next_left;
        current_right = next_right;
    }
}

/// Bank-aware C `eqn_subsumes_termpair` descent.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if equal-headed non-phony terms have different arities, matching the
/// non-LFHO C assertion.
pub fn eqn_subsumes_termpair_with_bank(
    bank: &mut TermBank,
    eqn: &Eqn,
    left: &Term,
    right: &Term,
) -> Result<bool, Diagnostic> {
    let mut current_left = left.clone();
    let mut current_right = right.clone();

    loop {
        if eqn_topsubsumes_termpair_with_bank(bank, eqn, &current_left, &current_right)? {
            return Ok(true);
        }
        if current_left.is_phony_app()
            || current_right.is_phony_app()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            return Ok(false);
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "non-LFHO subsumption descent expects equal arities"
        );

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
                    return Ok(false);
                }
                differing_pair = Some((next_left, next_right));
            }
        }
        let Some((next_left, next_right)) = differing_pair else {
            return Ok(true);
        };
        current_left = next_left;
        current_right = next_right;
    }
}

#[must_use]
pub fn literal_subsumes_clause(literal: &Eqn, clause: &Clause) -> bool {
    clause.literals().as_slice().iter().any(|candidate| {
        if literal.is_positive() {
            candidate.is_positive()
                && eqn_subsumes_termpair(literal, candidate.left(), candidate.right())
        } else {
            candidate.is_negative()
                && eqn_topsubsumes_termpair(literal, candidate.left(), candidate.right())
        }
    })
}

/// Bank-aware unit-literal subsumption over a clause.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn literal_subsumes_clause_with_bank(
    bank: &mut TermBank,
    literal: &Eqn,
    clause: &Clause,
) -> Result<bool, Diagnostic> {
    for candidate in clause.literals().as_slice() {
        let matches = if literal.is_positive() {
            candidate.is_positive()
                && eqn_subsumes_termpair_with_bank(
                    bank,
                    literal,
                    candidate.left(),
                    candidate.right(),
                )?
        } else {
            candidate.is_negative()
                && eqn_topsubsumes_termpair_with_bank(
                    bank,
                    literal,
                    candidate.left(),
                    candidate.right(),
                )?
        };
        if matches {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Returns whether a unit clause subsumes `clause`.
///
/// # Panics
///
/// Panics if `unit` does not contain exactly one literal, matching
/// `UnitClauseSubsumesClause`.
#[must_use]
pub fn unit_clause_subsumes_clause(unit: &Clause, clause: &Clause) -> bool {
    assert_eq!(
        unit.literal_number(),
        1,
        "unit subsumption requires one literal"
    );
    UNIT_CLAUSE_CLAUSE_SUBSUMPTION_CALLS.fetch_add(1, Ordering::SeqCst);
    literal_subsumes_clause(&unit.literals().as_slice()[0], clause)
}

/// Bank-aware C `UnitClauseSubsumesClause`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if `unit` does not contain exactly one literal.
pub fn unit_clause_subsumes_clause_with_bank(
    bank: &mut TermBank,
    unit: &Clause,
    clause: &Clause,
) -> Result<bool, Diagnostic> {
    assert_eq!(
        unit.literal_number(),
        1,
        "unit subsumption requires one literal"
    );
    UNIT_CLAUSE_CLAUSE_SUBSUMPTION_CALLS.fetch_add(1, Ordering::SeqCst);
    literal_subsumes_clause_with_bank(bank, &unit.literals().as_slice()[0], clause)
}

/// Returns a unit clause from `set` that subsumes one literal of `clause`.
///
/// This plain-set path mirrors `UnitClauseSetSubsumesClause` without using the
/// C demodulator index.
#[must_use]
pub fn unit_clause_set_subsumes_clause<'set>(
    set: &'set ClauseSet,
    clause: &Clause,
) -> Option<&'set Clause> {
    unit_clause_set_subsumes_clause_with_strong(set, clause, false)
}

/// Returns a unit clause from `set` that subsumes one literal of `clause`.
///
/// When `strong_unit_forward_subsumption` is true, positive target literals use
/// C's `unit_clause_set_strongsubsumes_termpair` descent instead of the ordinary
/// single-difference simplifying-unit descent.
#[must_use]
pub fn unit_clause_set_subsumes_clause_with_strong<'set>(
    set: &'set ClauseSet,
    clause: &Clause,
    strong_unit_forward_subsumption: bool,
) -> Option<&'set Clause> {
    clause.literals().as_slice().iter().find_map(|literal| {
        if literal.is_positive() {
            if strong_unit_forward_subsumption {
                find_strong_unit_simplifier_plain(set, literal.left(), literal.right(), true)
            } else {
                find_positive_unit_simplifier_plain(set, literal.left(), literal.right())
            }
        } else {
            find_negative_top_unit_simplifier_plain(set, literal.left(), literal.right())
        }
    })
}

/// Bank-aware unit-clause-set subsumption using complete higher-order
/// matching where required.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn unit_clause_set_subsumes_clause_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    clause: &Clause,
    strong_unit_forward_subsumption: bool,
) -> Result<Option<&'set Clause>, Diagnostic> {
    for literal in clause.literals().as_slice() {
        let result = if literal.is_positive() {
            if strong_unit_forward_subsumption {
                find_strong_unit_simplifier_plain_with_bank(
                    bank,
                    set,
                    literal.left(),
                    literal.right(),
                    true,
                )?
            } else {
                find_positive_unit_simplifier_plain_with_bank(
                    bank,
                    set,
                    literal.left(),
                    literal.right(),
                )?
            }
        } else {
            find_negative_top_unit_simplifier_plain_with_bank(
                bank,
                set,
                literal.left(),
                literal.right(),
            )?
        };
        if result.is_some() {
            return Ok(result);
        }
    }
    Ok(None)
}

/// Returns the first clause at or after `start_index` subsumed by a unit clause.
///
/// # Panics
///
/// Panics if `subsumer` is not unit, matching `ClauseSetFindUnitSubsumedClause`.
#[must_use]
pub fn clause_set_find_unit_subsumed_clause<'set>(
    set: &'set ClauseSet,
    start_index: usize,
    subsumer: &Clause,
) -> Option<&'set Clause> {
    assert_eq!(
        subsumer.literal_number(),
        1,
        "unit subsumption requires one literal"
    );
    set.iter()
        .skip(start_index)
        .find(|candidate| unit_clause_subsumes_clause(subsumer, candidate))
}

/// Removes negative literals simplified by positive unit clauses in `set`.
///
/// This plain-set path preserves the C mutation semantics but uses linear
/// search until demodulator indexes are available.
#[must_use]
pub fn clause_positive_simplify_reflect(set: &ClauseSet, clause: &mut Clause) -> bool {
    clause_positive_simplify_reflect_with_strong(set, clause, false)
}

/// Removes negative literals simplified by positive unit clauses in `set`.
///
/// When `strong_unit_forward_subsumption` is true, this uses C's strong
/// unit-forward subsumption descent for the positive units.
#[must_use]
pub fn clause_positive_simplify_reflect_with_strong(
    set: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
) -> bool {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_positive() {
                None
            } else if strong_unit_forward_subsumption {
                find_strong_unit_simplifier(set, literal.left(), literal.right(), true)
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            } else {
                find_positive_unit_simplifier(set, literal.left(), literal.right())
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    clause.is_empty()
}

/// Removes negative literals simplified by positive unit clauses in `set` while
/// emitting represented proof documentation.
///
/// # Errors
///
/// Returns a diagnostic if proof-documentation rendering fails.
pub fn clause_positive_simplify_reflect_with_strong_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
) -> Result<bool, Diagnostic> {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_positive() {
                None
            } else if strong_unit_forward_subsumption {
                find_strong_unit_simplifier(set, literal.left(), literal.right(), true)
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            } else {
                find_positive_unit_simplifier(set, literal.left(), literal.right())
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            session.doc_clause_modification(
                output,
                bank,
                clause,
                ClauseModificationInference::SimplifyReflect,
                Some(simplifier),
                None,
            )?;
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    Ok(clause.is_empty())
}

/// Bank-aware positive simplify-reflect for higher-order complete matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn clause_positive_simplify_reflect_with_strong_and_bank(
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
) -> Result<bool, Diagnostic> {
    clause_positive_simplify_reflect_with_bank_impl::<String>(
        bank,
        set,
        clause,
        strong_unit_forward_subsumption,
        None,
    )
}

/// Bank-aware positive simplify-reflect with represented proof documentation.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching, normalization, or proof
/// rendering.
pub fn clause_positive_simplify_reflect_with_strong_and_docs_and_bank(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
) -> Result<bool, Diagnostic> {
    clause_positive_simplify_reflect_with_bank_impl(
        bank,
        set,
        clause,
        strong_unit_forward_subsumption,
        Some((output, session)),
    )
}

fn clause_positive_simplify_reflect_with_bank_impl<W: fmt::Write>(
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_positive() {
                None
            } else if strong_unit_forward_subsumption {
                find_strong_unit_simplifier_with_bank(
                    bank,
                    set,
                    literal.left(),
                    literal.right(),
                    true,
                )?
                .map(|simplifier| (simplifier.is_sos(), simplifier))
            } else {
                find_positive_unit_simplifier_with_bank(bank, set, literal.left(), literal.right())?
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_modification(
                    &mut **output,
                    bank,
                    clause,
                    ClauseModificationInference::SimplifyReflect,
                    Some(simplifier),
                    None,
                )?;
            }
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    Ok(clause.is_empty())
}

/// Removes positive literals simplified by negative unit clauses in `set`.
///
/// This plain-set path preserves the C mutation semantics but uses linear
/// search until demodulator indexes are available.
#[must_use]
pub fn clause_negative_simplify_reflect(set: &ClauseSet, clause: &mut Clause) -> bool {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_negative() {
                None
            } else {
                find_negative_top_unit_simplifier(set, literal.left(), literal.right())
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    clause.is_empty()
}

/// Removes positive literals simplified by negative unit clauses in `set` while
/// emitting represented proof documentation.
///
/// # Errors
///
/// Returns a diagnostic if proof-documentation rendering fails.
pub fn clause_negative_simplify_reflect_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_negative() {
                None
            } else {
                find_negative_top_unit_simplifier(set, literal.left(), literal.right())
                    .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            session.doc_clause_modification(
                output,
                bank,
                clause,
                ClauseModificationInference::SimplifyReflect,
                Some(simplifier),
                None,
            )?;
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    Ok(clause.is_empty())
}

/// Bank-aware negative simplify-reflect for higher-order complete matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn clause_negative_simplify_reflect_with_bank(
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    clause_negative_simplify_reflect_with_bank_impl::<String>(bank, set, clause, None)
}

/// Bank-aware negative simplify-reflect with represented proof documentation.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching, normalization, or proof
/// rendering.
pub fn clause_negative_simplify_reflect_with_docs_and_bank(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    clause_negative_simplify_reflect_with_bank_impl(bank, set, clause, Some((output, session)))
}

fn clause_negative_simplify_reflect_with_bank_impl<W: fmt::Write>(
    bank: &mut TermBank,
    set: &ClauseSet,
    clause: &mut Clause,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    let mut index = 0;
    while index < clause.literal_number() {
        let simplifier = {
            let literal = &clause.literals().as_slice()[index];
            if literal.is_negative() {
                None
            } else {
                find_negative_top_unit_simplifier_with_bank(
                    bank,
                    set,
                    literal.left(),
                    literal.right(),
                )?
                .map(|simplifier| (simplifier.is_sos(), simplifier))
            }
        };

        if let Some((simplifier_sos, simplifier)) = simplifier {
            let _ = clause_remove_literal_index(clause, index);
            if simplifier_sos {
                clause.set_prop(CP_IS_SOS);
            }
            clause.del_prop(CP_INITIAL | CP_LIMITED_RW);
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_modification(
                    &mut **output,
                    bank,
                    clause,
                    ClauseModificationInference::SimplifyReflect,
                    Some(simplifier),
                    None,
                )?;
            }
            clause_push_derivation(clause, DC_SR, Some(simplifier), None);
        } else {
            index += 1;
        }
    }
    Ok(clause.is_empty())
}

pub fn clause_subsume_order_sort_lits(clause: &mut Clause, bank: &TermBank) {
    clause.sort_literals_by(|left, right| {
        i64::from(left.subsume_inverse_refined_compare(right, bank))
    });
}

#[must_use]
pub fn clause_is_subsume_ordered(clause: &Clause, bank: &TermBank) -> bool {
    clause.is_sorted_by(|left, right| i64::from(left.subsume_inverse_compare(right, bank)))
}

/// Returns whether `subsumer` subsumes `sub_candidate`.
///
/// # Panics
///
/// Panics if either clause is not subsumption ordered or if either cached
/// clause weight does not match `ClauseStandardWeight`, matching the C
/// preconditions on `ClauseSubsumesClause`.
#[must_use]
pub fn clause_subsumes_clause(subsumer: &Clause, sub_candidate: &Clause, bank: &TermBank) -> bool {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SubsumeTimer,
    );
    assert!(clause_is_subsume_ordered(subsumer, bank));
    assert!(clause_is_subsume_ordered(sub_candidate, bank));
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    assert_eq!(subsumer.weight(), subsumer.standard_weight());

    if subsumer.is_empty() {
        return true;
    }
    if subsumer.is_unit() {
        return unit_clause_subsumes_clause(subsumer, sub_candidate);
    }
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS.fetch_add(1, Ordering::SeqCst);
    if subsumer.positive_literal_count() > sub_candidate.positive_literal_count()
        || subsumer.negative_literal_count() > sub_candidate.negative_literal_count()
        || subsumer.weight() > sub_candidate.weight()
    {
        return false;
    }
    if (sub_candidate.positive_literal_count() >= 3 || sub_candidate.negative_literal_count() >= 3)
        && !check_subsumption_possibility(subsumer, sub_candidate, bank)
    {
        return false;
    }

    let mut subst = Substitution::new();
    let mut picked = vec![false; sub_candidate.literal_number()];
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS_REC.fetch_add(1, Ordering::SeqCst);
    let result = eqn_list_rec_subsume(
        subsumer.literals().as_slice(),
        sub_candidate.literals().as_slice(),
        &mut subst,
        &mut picked,
        bank,
    );
    subst.backtrack();
    if result {
        CLAUSE_CLAUSE_SUBSUMPTION_SUCCESSES.fetch_add(1, Ordering::SeqCst);
    }
    result
}

/// Bank-aware C `ClauseSubsumesClause` using complete higher-order matching.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if either clause is not subsumption ordered or has a stale cached
/// standard weight, matching the C preconditions.
pub fn clause_subsumes_clause_with_bank(
    subsumer: &Clause,
    sub_candidate: &Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SubsumeTimer,
    );
    assert!(clause_is_subsume_ordered(subsumer, bank));
    assert!(clause_is_subsume_ordered(sub_candidate, bank));
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    assert_eq!(subsumer.weight(), subsumer.standard_weight());

    if subsumer.is_empty() {
        return Ok(true);
    }
    if subsumer.is_unit() {
        return unit_clause_subsumes_clause_with_bank(bank, subsumer, sub_candidate);
    }
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS.fetch_add(1, Ordering::SeqCst);
    if subsumer.positive_literal_count() > sub_candidate.positive_literal_count()
        || subsumer.negative_literal_count() > sub_candidate.negative_literal_count()
        || subsumer.weight() > sub_candidate.weight()
    {
        return Ok(false);
    }
    if (sub_candidate.positive_literal_count() >= 3 || sub_candidate.negative_literal_count() >= 3)
        && !check_subsumption_possibility_with_bank(subsumer, sub_candidate, bank)?
    {
        return Ok(false);
    }

    let mut subst = Substitution::new();
    let mut picked = vec![false; sub_candidate.literal_number()];
    CLAUSE_CLAUSE_SUBSUMPTION_CALLS_REC.fetch_add(1, Ordering::SeqCst);
    let result = eqn_list_rec_subsume_with_bank(
        subsumer.literals().as_slice(),
        sub_candidate.literals().as_slice(),
        &mut subst,
        &mut picked,
        bank,
    );
    subst.backtrack();
    let result = result?;
    if result {
        CLAUSE_CLAUSE_SUBSUMPTION_SUCCESSES.fetch_add(1, Ordering::SeqCst);
    }
    Ok(result)
}

/// Returns the first clause in `set` that subsumes `sub_candidate`.
///
/// This is the plain clause-set path used by `ClauseSetSubsumesClause` when no
/// feature-vector index is attached.
///
/// # Panics
///
/// Panics if `sub_candidate` is unit, if the candidate or any checked set
/// clause is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight, matching the C fallback preconditions.
#[must_use]
pub fn clause_set_subsumes_clause<'set>(
    set: &'set ClauseSet,
    sub_candidate: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert!(
        sub_candidate.literal_number() > 1,
        "plain ClauseSetSubsumesClause expects a non-unit candidate"
    );
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    set.iter()
        .find(|candidate| clause_subsumes_clause(candidate, sub_candidate, bank))
}

/// Bank-aware plain clause-set subsumer lookup.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if the candidate is unit, is not subsumption ordered, or has a stale
/// cached weight, matching C's plain-set preconditions.
pub fn clause_set_subsumes_clause_with_bank<'set>(
    set: &'set ClauseSet,
    sub_candidate: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert!(
        sub_candidate.literal_number() > 1,
        "plain ClauseSetSubsumesClause expects a non-unit candidate"
    );
    assert_eq!(sub_candidate.weight(), sub_candidate.standard_weight());
    for candidate in set.iter() {
        if clause_subsumes_clause_with_bank(candidate, sub_candidate, bank)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Returns the first clause at or after `start_index` subsumed by `subsumer`.
///
/// The index models C's linked-list `set_position` until stable clause handles
/// replace the plain owner.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_subsumed_clause<'set>(
    set: &'set ClauseSet,
    start_index: usize,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    set.iter()
        .skip(start_index)
        .find(|candidate| clause_subsumes_clause(subsumer, candidate, bank))
}

/// Pushes every clause in `set` subsumed by `subsumer` onto `result`.
///
/// Returns the number of newly pushed clauses, matching
/// `ClauseSetFindSubsumedClauses`' stack-pointer delta.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
pub fn clause_set_find_subsumed_clauses<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    result: &mut PStack<&'set Clause>,
    bank: &TermBank,
) -> i64 {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let old_len = result.len();
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    for candidate in set.iter() {
        if clause_subsumes_clause(subsumer, candidate, bank) {
            result.push(candidate);
        }
    }
    usize_to_i64(result.len() - old_len)
}

/// Bank-aware plain lookup of all clauses subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if the subsumer or a checked candidate is not subsumption ordered or
/// has a stale cached weight.
pub fn clause_set_find_subsumed_clauses_with_bank<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    result: &mut PStack<&'set Clause>,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let old_len = result.len();
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    for candidate in set.iter() {
        if clause_subsumes_clause_with_bank(subsumer, candidate, bank)? {
            result.push(candidate);
        }
    }
    Ok(usize_to_i64(result.len() - old_len))
}

/// Returns the first clause in `set` subsumed by `subsumer`.
///
/// # Panics
///
/// Panics if `subsumer` or any checked set clause is not subsumption-ordered, or
/// if any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_first_subsumed_clause<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    clause_set_find_subsumed_clause(set, 0, subsumer, bank)
}

/// Bank-aware plain lookup of the first clause subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if the subsumer or a checked candidate is not subsumption ordered or
/// has a stale cached weight.
pub fn clause_set_find_first_subsumed_clause_with_bank<'set>(
    set: &'set ClauseSet,
    subsumer: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert_eq!(subsumer.weight(), subsumer.standard_weight());
    for candidate in set.iter() {
        if clause_subsumes_clause_with_bank(subsumer, candidate, bank)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Returns the first clause that subsumes `sub_candidate`, using `index` when
/// one is available and otherwise scanning `set`.
///
/// This mirrors the indexed/plain branch in `ClauseSetSubsumesClause`. Until
/// `ClauseSet` owns index lifecycle, callers must pass an anchor that reflects
/// the same set contents.
///
/// # Panics
///
/// Panics if `sub_candidate` is unit, if the query or any checked clause is not
/// subsumption-ordered, or if any checked clause has stale cached standard
/// weight, matching the C preconditions.
#[must_use]
pub fn clause_set_subsumes_clause_with_index<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    sub_candidate: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert!(
        sub_candidate.literal_number() > 1,
        "ClauseSetSubsumesClause expects a non-unit candidate"
    );
    let Some(index) = index else {
        return clause_set_subsumes_clause(set, sub_candidate, bank);
    };
    let packed_candidate = fv_index_pack_clause(sub_candidate.clone(), Some(index));
    fv_index_subsumes_packed_clause(index.index(), &packed_candidate, bank)
}

/// Bank-aware indexed/plain clause-set subsumer lookup.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if the candidate is unit, is not subsumption ordered, or has a stale
/// cached weight.
pub fn clause_set_subsumes_clause_with_index_and_bank<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    sub_candidate: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    assert!(
        sub_candidate.literal_number() > 1,
        "ClauseSetSubsumesClause expects a non-unit candidate"
    );
    let Some(index) = index else {
        return clause_set_subsumes_clause_with_bank(set, sub_candidate, bank);
    };
    let packed_candidate = fv_index_pack_clause(sub_candidate.clone(), Some(index));
    fv_index_subsumes_packed_clause_with_bank(index.index(), &packed_candidate, bank)
}

/// Pushes every clause subsumed by `subsumer`, using `index` when one is
/// available and otherwise scanning `set`.
///
/// Returns the number of newly pushed clauses, matching the
/// `ClauseSetFindSubsumedClauses` stack delta. Until `ClauseSet` owns index
/// lifecycle, callers must pass an anchor that reflects the same set contents.
///
/// # Panics
///
/// Panics if `subsumer` or any checked clause is not subsumption-ordered, or if
/// any checked clause has stale cached standard weight.
pub fn clause_set_find_subsumed_clauses_with_index<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    subsumer: &Clause,
    result: &mut PStack<&'set Clause>,
    bank: &TermBank,
) -> i64 {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let Some(index) = index else {
        return clause_set_find_subsumed_clauses(set, subsumer, result, bank);
    };
    let packed_subsumer = fv_index_pack_clause(subsumer.clone(), Some(index));
    fv_index_find_subsumed_clauses(index.index(), &packed_subsumer, result, bank)
}

/// Bank-aware indexed/plain lookup of all clauses subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn clause_set_find_subsumed_clauses_with_index_and_bank<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    subsumer: &Clause,
    result: &mut PStack<&'set Clause>,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let Some(index) = index else {
        return clause_set_find_subsumed_clauses_with_bank(set, subsumer, result, bank);
    };
    let packed_subsumer = fv_index_pack_clause(subsumer.clone(), Some(index));
    fv_index_find_subsumed_clauses_with_bank(index.index(), &packed_subsumer, result, bank)
}

/// Returns the first clause subsumed by `subsumer`, using `index` when one is
/// available and otherwise scanning `set`.
///
/// Until `ClauseSet` owns index lifecycle, callers must pass an anchor that
/// reflects the same set contents.
///
/// # Panics
///
/// Panics if `subsumer` or any checked clause is not subsumption-ordered, or if
/// any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_first_subsumed_clause_with_index<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'set Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let Some(index) = index else {
        return clause_set_find_first_subsumed_clause(set, subsumer, bank);
    };
    let packed_subsumer = fv_index_pack_clause(subsumer.clone(), Some(index));
    fv_index_find_first_subsumed_clause(index.index(), &packed_subsumer, bank)
}

/// Bank-aware indexed/plain lookup of the first clause subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn clause_set_find_first_subsumed_clause_with_index_and_bank<'set>(
    set: &'set ClauseSet,
    index: Option<&'set FvIndexAnchor>,
    subsumer: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::SetSubsumeTimer,
    );
    let Some(index) = index else {
        return clause_set_find_first_subsumed_clause_with_bank(set, subsumer, bank);
    };
    let packed_subsumer = fv_index_pack_clause(subsumer.clone(), Some(index));
    fv_index_find_first_subsumed_clause_with_bank(index.index(), &packed_subsumer, bank)
}

/// Returns the first indexed clause that is a variant of `clause`.
///
/// This is the packed-query wrapper for C's `ClauseSetFindVariantClause`; the C
/// entry point requires an FV-indexed set, so there is no plain fallback.
///
/// # Panics
///
/// Panics if `clause` or any checked indexed clause is not subsumption-ordered,
/// or if any checked clause has stale cached standard weight.
#[must_use]
pub fn clause_set_find_variant_clause_indexed<'index>(
    index: &'index FvIndexAnchor,
    clause: &Clause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    let packed_clause = fv_index_pack_clause(clause.clone(), Some(index));
    fv_index_find_variant_clause(index.index(), &packed_clause, bank)
}

/// Bank-aware indexed variant lookup.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
pub fn clause_set_find_variant_clause_indexed_with_bank<'index>(
    index: &'index FvIndexAnchor,
    clause: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    let packed_clause = fv_index_pack_clause(clause.clone(), Some(index));
    fv_index_find_variant_clause_with_bank(index.index(), &packed_clause, bank)
}

/// Returns the first indexed clause that subsumes `sub_candidate`.
///
/// This mirrors `clause_set_subsumes_clause_indexed` for callers that already
/// own the `FvIndex` and a packed query clause.
///
/// # Panics
///
/// Panics if `sub_candidate` is unpacked, if the query or any checked indexed
/// clause is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight.
#[must_use]
pub fn fv_index_subsumes_packed_clause<'index>(
    index: &'index FvIndex,
    sub_candidate: &FvPackedClause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = sub_candidate
        .vector()
        .expect("FV-index subsumption requires a packed frequency vector");
    assert_eq!(
        sub_candidate.clause().weight(),
        sub_candidate.clause().standard_weight()
    );
    fv_index_subsumes_clause_rec(index, vector.as_slice(), 0, sub_candidate.clause(), bank)
}

/// Bank-aware FV-index subsumer lookup.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if `sub_candidate` is unpacked or if it or a checked indexed clause
/// violates subsumption-order or cached-weight preconditions.
pub fn fv_index_subsumes_packed_clause_with_bank<'index>(
    index: &'index FvIndex,
    sub_candidate: &FvPackedClause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = sub_candidate
        .vector()
        .expect("FV-index subsumption requires a packed frequency vector");
    assert_eq!(
        sub_candidate.clause().weight(),
        sub_candidate.clause().standard_weight()
    );
    fv_index_subsumes_clause_rec_with_bank(
        index,
        vector.as_slice(),
        0,
        sub_candidate.clause(),
        bank,
    )
}

/// Pushes every indexed clause subsumed by `subsumer` onto `result`.
///
/// Returns the number of newly pushed clauses, matching
/// `ClauseSetFindFVSubsumedClauses`.
///
/// # Panics
///
/// Panics if `subsumer` is unpacked, if the query or any checked indexed clause
/// is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight.
pub fn fv_index_find_subsumed_clauses<'index>(
    index: &'index FvIndex,
    subsumer: &FvPackedClause,
    result: &mut PStack<&'index Clause>,
    bank: &TermBank,
) -> i64 {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let old_len = result.len();
    let vector = subsumer
        .vector()
        .expect("FV-index subsumed-clause lookup requires a packed frequency vector");
    assert_eq!(
        subsumer.clause().weight(),
        subsumer.clause().standard_weight()
    );
    fv_index_find_subsumed_clauses_rec(
        index,
        vector.as_slice(),
        0,
        subsumer.clause(),
        result,
        bank,
    );
    usize_to_i64(result.len() - old_len)
}

/// Bank-aware FV-index lookup of all clauses subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if `subsumer` is unpacked or if it or a checked indexed clause
/// violates subsumption-order or cached-weight preconditions.
pub fn fv_index_find_subsumed_clauses_with_bank<'index>(
    index: &'index FvIndex,
    subsumer: &FvPackedClause,
    result: &mut PStack<&'index Clause>,
    bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let old_len = result.len();
    let vector = subsumer
        .vector()
        .expect("FV-index subsumed-clause lookup requires a packed frequency vector");
    assert_eq!(
        subsumer.clause().weight(),
        subsumer.clause().standard_weight()
    );
    fv_index_find_subsumed_clauses_rec_with_bank(
        index,
        vector.as_slice(),
        0,
        subsumer.clause(),
        result,
        bank,
    )?;
    Ok(usize_to_i64(result.len() - old_len))
}

/// Returns the first indexed clause subsumed by `subsumer`.
///
/// # Panics
///
/// Panics if `subsumer` is unpacked, if the query or any checked indexed clause
/// is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight.
#[must_use]
pub fn fv_index_find_first_subsumed_clause<'index>(
    index: &'index FvIndex,
    subsumer: &FvPackedClause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = subsumer
        .vector()
        .expect("FV-index first-subsumed lookup requires a packed frequency vector");
    assert_eq!(
        subsumer.clause().weight(),
        subsumer.clause().standard_weight()
    );
    fv_index_find_first_subsumed_clause_rec(index, vector.as_slice(), 0, subsumer.clause(), bank)
}

/// Bank-aware FV-index lookup of the first clause subsumed by `subsumer`.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if `subsumer` is unpacked or if it or a checked indexed clause
/// violates subsumption-order or cached-weight preconditions.
pub fn fv_index_find_first_subsumed_clause_with_bank<'index>(
    index: &'index FvIndex,
    subsumer: &FvPackedClause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = subsumer
        .vector()
        .expect("FV-index first-subsumed lookup requires a packed frequency vector");
    assert_eq!(
        subsumer.clause().weight(),
        subsumer.clause().standard_weight()
    );
    fv_index_find_first_subsumed_clause_rec_with_bank(
        index,
        vector.as_slice(),
        0,
        subsumer.clause(),
        bank,
    )
}

/// Returns the first indexed clause that is a variant of `clause`.
///
/// # Panics
///
/// Panics if `clause` is unpacked, if the query or any checked indexed clause
/// is not subsumption-ordered, or if any checked clause has stale cached
/// standard weight.
#[must_use]
pub fn fv_index_find_variant_clause<'index>(
    index: &'index FvIndex,
    clause: &FvPackedClause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = clause
        .vector()
        .expect("FV-index variant lookup requires a packed frequency vector");
    assert_eq!(clause.clause().weight(), clause.clause().standard_weight());
    fv_index_find_variant_clause_rec(index, vector.as_slice(), 0, clause.clause(), bank)
}

/// Bank-aware FV-index variant lookup.
///
/// # Errors
///
/// Returns diagnostics from higher-order matching or normalization.
///
/// # Panics
///
/// Panics if `clause` is unpacked or if it or a checked indexed clause
/// violates subsumption-order or cached-weight preconditions.
pub fn fv_index_find_variant_clause_with_bank<'index>(
    index: &'index FvIndex,
    clause: &FvPackedClause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::FvIndexTimer,
    );
    let vector = clause
        .vector()
        .expect("FV-index variant lookup requires a packed frequency vector");
    assert_eq!(clause.clause().weight(), clause.clause().standard_weight());
    fv_index_find_variant_clause_rec_with_bank(index, vector.as_slice(), 0, clause.clause(), bank)
}

fn check_subsumption_possibility(
    subsumer: &Clause,
    sub_candidate: &Clause,
    bank: &TermBank,
) -> bool {
    subsumer
        .literals()
        .as_slice()
        .iter()
        .all(|literal| find_spec_literal(literal, sub_candidate.literals().as_slice(), bank))
}

fn check_subsumption_possibility_with_bank(
    subsumer: &Clause,
    sub_candidate: &Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    for literal in subsumer.literals().as_slice() {
        if !find_spec_literal_with_bank(literal, sub_candidate.literals().as_slice(), bank)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn find_spec_literal(literal: &Eqn, candidates: &[Eqn], bank: &TermBank) -> bool {
    let mut subst = Substitution::new();
    for candidate in candidates {
        let cmp = literal.subsume_q_order_compare(candidate, bank);
        if cmp > 0 {
            return false;
        }
        if cmp < 0 {
            continue;
        }
        if literal.standard_weight() > candidate.standard_weight() {
            return false;
        }
        if literal_matches_with_subst(literal, candidate, &mut subst) {
            subst.backtrack();
            return true;
        }
        subst.backtrack();
    }
    false
}

fn find_spec_literal_with_bank(
    literal: &Eqn,
    candidates: &[Eqn],
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let mut subst = Substitution::new();
    for candidate in candidates {
        let cmp = literal.subsume_q_order_compare(candidate, bank);
        if cmp > 0 {
            return Ok(false);
        }
        if cmp < 0 {
            continue;
        }
        if literal.standard_weight() > candidate.standard_weight() {
            return Ok(false);
        }
        let result = literal_matches_with_subst_with_bank(literal, candidate, &mut subst, bank);
        subst.backtrack();
        if result? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn eqn_list_rec_subsume(
    subsum_list: &[Eqn],
    sub_cand_list: &[Eqn],
    subst: &mut Substitution,
    picked: &mut [bool],
    bank: &TermBank,
) -> bool {
    let Some((literal, remaining)) = subsum_list.split_first() else {
        return true;
    };

    for (index, candidate) in sub_cand_list.iter().enumerate() {
        if picked[index] {
            continue;
        }
        let cmp = candidate.subsume_q_order_compare(literal, bank);
        if cmp < 0 {
            return false;
        }
        if cmp > 0 {
            continue;
        }
        if candidate.standard_weight() < literal.standard_weight() {
            return false;
        }
        if literal.is_oriented() && !candidate.is_oriented() {
            continue;
        }

        picked[index] = true;
        let state = subst.len();
        if literal_matches_with_subst(literal, candidate, subst)
            && eqn_list_rec_subsume(remaining, sub_cand_list, subst, picked, bank)
        {
            return true;
        }
        subst.backtrack_to_pos(state);
        picked[index] = false;
    }
    false
}

fn eqn_list_rec_subsume_with_bank(
    subsum_list: &[Eqn],
    sub_cand_list: &[Eqn],
    subst: &mut Substitution,
    picked: &mut [bool],
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let Some((literal, remaining)) = subsum_list.split_first() else {
        return Ok(true);
    };

    for (index, candidate) in sub_cand_list.iter().enumerate() {
        if picked[index] {
            continue;
        }
        let cmp = candidate.subsume_q_order_compare(literal, bank);
        if cmp < 0 {
            return Ok(false);
        }
        if cmp > 0 {
            continue;
        }
        if candidate.standard_weight() < literal.standard_weight() {
            return Ok(false);
        }
        if literal.is_oriented() && !candidate.is_oriented() {
            continue;
        }

        picked[index] = true;
        let state = subst.len();
        let matches = literal_matches_with_subst_with_bank(literal, candidate, subst, bank);
        match matches {
            Ok(true) => {
                match eqn_list_rec_subsume_with_bank(remaining, sub_cand_list, subst, picked, bank)
                {
                    Ok(true) => return Ok(true),
                    Ok(false) => {}
                    Err(error) => {
                        subst.backtrack_to_pos(state);
                        picked[index] = false;
                        return Err(error);
                    }
                }
            }
            Ok(false) => {}
            Err(error) => {
                subst.backtrack_to_pos(state);
                picked[index] = false;
                return Err(error);
            }
        }
        subst.backtrack_to_pos(state);
        picked[index] = false;
    }
    Ok(false)
}

fn literal_matches_with_subst(pattern: &Eqn, candidate: &Eqn, subst: &mut Substitution) -> bool {
    pattern.subsume(candidate, subst)
}

fn literal_matches_with_subst_with_bank(
    pattern: &Eqn,
    candidate: &Eqn,
    subst: &mut Substitution,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    pattern.subsume_with_bank(candidate, subst, bank)
}

fn fv_index_subsumes_clause_rec<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    clause: &Clause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    if feature == vector.len() {
        return index
            .clauses()
            .values()
            .find(|candidate| clause_subsumes_clause(candidate, clause, bank));
    }

    for successor in index
        .successors()
        .range(..=vector[feature])
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            if let Some(result) =
                fv_index_subsumes_clause_rec(successor, vector, feature + 1, clause, bank)
            {
                return Some(result);
            }
        }
    }
    None
}

fn fv_index_subsumes_clause_rec_with_bank<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    clause: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    if feature == vector.len() {
        for candidate in index.clauses().values() {
            if clause_subsumes_clause_with_bank(candidate, clause, bank)? {
                return Ok(Some(candidate));
            }
        }
        return Ok(None);
    }

    for successor in index
        .successors()
        .range(..=vector[feature])
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            if let Some(result) = fv_index_subsumes_clause_rec_with_bank(
                successor,
                vector,
                feature + 1,
                clause,
                bank,
            )? {
                return Ok(Some(result));
            }
        }
    }
    Ok(None)
}

fn fv_index_find_subsumed_clauses_rec<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    subsumer: &Clause,
    result: &mut PStack<&'index Clause>,
    bank: &TermBank,
) {
    if feature == vector.len() {
        for candidate in index.clauses().values() {
            if clause_subsumes_clause(subsumer, candidate, bank) {
                result.push(candidate);
            }
        }
        return;
    }

    for successor in index
        .successors()
        .range(vector[feature]..)
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            fv_index_find_subsumed_clauses_rec(
                successor,
                vector,
                feature + 1,
                subsumer,
                result,
                bank,
            );
        }
    }
}

fn fv_index_find_subsumed_clauses_rec_with_bank<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    subsumer: &Clause,
    result: &mut PStack<&'index Clause>,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if feature == vector.len() {
        for candidate in index.clauses().values() {
            if clause_subsumes_clause_with_bank(subsumer, candidate, bank)? {
                result.push(candidate);
            }
        }
        return Ok(());
    }

    for successor in index
        .successors()
        .range(vector[feature]..)
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            fv_index_find_subsumed_clauses_rec_with_bank(
                successor,
                vector,
                feature + 1,
                subsumer,
                result,
                bank,
            )?;
        }
    }
    Ok(())
}

fn fv_index_find_first_subsumed_clause_rec<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    subsumer: &Clause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    if feature == vector.len() {
        return index
            .clauses()
            .values()
            .find(|candidate| clause_subsumes_clause(subsumer, candidate, bank));
    }

    for successor in index
        .successors()
        .range(vector[feature]..)
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            if let Some(result) = fv_index_find_first_subsumed_clause_rec(
                successor,
                vector,
                feature + 1,
                subsumer,
                bank,
            ) {
                return Some(result);
            }
        }
    }
    None
}

fn fv_index_find_first_subsumed_clause_rec_with_bank<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    subsumer: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    if feature == vector.len() {
        for candidate in index.clauses().values() {
            if clause_subsumes_clause_with_bank(subsumer, candidate, bank)? {
                return Ok(Some(candidate));
            }
        }
        return Ok(None);
    }

    for successor in index
        .successors()
        .range(vector[feature]..)
        .map(|(_, node)| node)
    {
        if successor.clause_count() != 0 {
            if let Some(result) = fv_index_find_first_subsumed_clause_rec_with_bank(
                successor,
                vector,
                feature + 1,
                subsumer,
                bank,
            )? {
                return Ok(Some(result));
            }
        }
    }
    Ok(None)
}

fn fv_index_find_variant_clause_rec<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    clause: &Clause,
    bank: &TermBank,
) -> Option<&'index Clause> {
    if feature == vector.len() {
        return index.clauses().values().find(|candidate| {
            clause_subsumes_clause(candidate, clause, bank)
                && clause_subsumes_clause(clause, candidate, bank)
        });
    }

    let successor = index.successors().get(&vector[feature])?;
    if successor.clause_count() == 0 {
        return None;
    }
    fv_index_find_variant_clause_rec(successor, vector, feature + 1, clause, bank)
}

fn fv_index_find_variant_clause_rec_with_bank<'index>(
    index: &'index FvIndex,
    vector: &[i64],
    feature: usize,
    clause: &Clause,
    bank: &mut TermBank,
) -> Result<Option<&'index Clause>, Diagnostic> {
    if feature == vector.len() {
        for candidate in index.clauses().values() {
            if clause_subsumes_clause_with_bank(candidate, clause, bank)?
                && clause_subsumes_clause_with_bank(clause, candidate, bank)?
            {
                return Ok(Some(candidate));
            }
        }
        return Ok(None);
    }

    let Some(successor) = index.successors().get(&vector[feature]) else {
        return Ok(None);
    };
    if successor.clause_count() == 0 {
        return Ok(None);
    }
    fv_index_find_variant_clause_rec_with_bank(successor, vector, feature + 1, clause, bank)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn find_positive_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    find_simplifying_unit(set, left, right, true).map(SimplifyingUnit::clause)
}

fn find_positive_unit_simplifier_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Result<Option<&'set Clause>, Diagnostic> {
    Ok(find_simplifying_unit_with_bank(bank, set, left, right, true)?.map(SimplifyingUnit::clause))
}

fn find_positive_unit_simplifier_plain<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    set.iter().find(|candidate| {
        candidate
            .literals()
            .as_slice()
            .first()
            .is_some_and(|literal| {
                candidate.is_unit()
                    && literal.is_positive()
                    && eqn_subsumes_termpair(literal, left, right)
            })
    })
}

fn find_positive_unit_simplifier_plain_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Result<Option<&'set Clause>, Diagnostic> {
    for candidate in set.iter() {
        let Some(literal) = candidate.literals().as_slice().first() else {
            continue;
        };
        if candidate.is_unit()
            && literal.is_positive()
            && eqn_subsumes_termpair_with_bank(bank, literal, left, right)?
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn find_strong_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Option<&'set Clause> {
    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((current_left, current_right)) = stack.pop() {
        if let Some(result) = find_top_unit_simplifier(set, &current_left, &current_right, positive)
        {
            return Some(result);
        }
        if current_left.is_applied_free_var()
            || current_right.is_applied_free_var()
            || current_left.is_lambda()
            || current_right.is_lambda()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            break;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "strong unit subsumption descent expects equal arities"
        );
        assert_eq!(
            current_left.type_(),
            current_right.type_(),
            "strong unit subsumption descent expects equal types"
        );
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                stack.push((next_left, next_right));
            }
        }
    }
    None
}

fn find_strong_unit_simplifier_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((current_left, current_right)) = stack.pop() {
        if let Some(result) =
            find_top_unit_simplifier_with_bank(bank, set, &current_left, &current_right, positive)?
        {
            return Ok(Some(result));
        }
        if current_left.is_applied_free_var()
            || current_right.is_applied_free_var()
            || current_left.is_lambda()
            || current_right.is_lambda()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            break;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "strong unit subsumption descent expects equal arities"
        );
        assert_eq!(
            current_left.type_(),
            current_right.type_(),
            "strong unit subsumption descent expects equal types"
        );
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                stack.push((next_left, next_right));
            }
        }
    }
    Ok(None)
}

fn find_strong_unit_simplifier_plain<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Option<&'set Clause> {
    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((current_left, current_right)) = stack.pop() {
        if let Some(result) =
            find_top_unit_simplifier_plain(set, &current_left, &current_right, positive)
        {
            return Some(result);
        }
        if current_left.is_applied_free_var()
            || current_right.is_applied_free_var()
            || current_left.is_lambda()
            || current_right.is_lambda()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            break;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "strong unit subsumption descent expects equal arities"
        );
        assert_eq!(
            current_left.type_(),
            current_right.type_(),
            "strong unit subsumption descent expects equal types"
        );
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                stack.push((next_left, next_right));
            }
        }
    }
    None
}

fn find_strong_unit_simplifier_plain_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Result<Option<&'set Clause>, Diagnostic> {
    let mut stack = vec![(left.clone(), right.clone())];
    while let Some((current_left, current_right)) = stack.pop() {
        if let Some(result) = find_top_unit_simplifier_plain_with_bank(
            bank,
            set,
            &current_left,
            &current_right,
            positive,
        )? {
            return Ok(Some(result));
        }
        if current_left.is_applied_free_var()
            || current_right.is_applied_free_var()
            || current_left.is_lambda()
            || current_right.is_lambda()
            || current_left.f_code() != current_right.f_code()
            || current_left.arity() == 0
        {
            break;
        }
        assert_eq!(
            current_left.arity(),
            current_right.arity(),
            "strong unit subsumption descent expects equal arities"
        );
        assert_eq!(
            current_left.type_(),
            current_right.type_(),
            "strong unit subsumption descent expects equal types"
        );
        for index in 0..current_left.arity() {
            let next_left = current_left
                .argument(index)
                .expect("left term arguments must be initialized");
            let next_right = current_right
                .argument(index)
                .expect("right term arguments must be initialized");
            if next_left != next_right {
                stack.push((next_left, next_right));
            }
        }
    }
    Ok(None)
}

fn find_top_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Option<&'set Clause> {
    find_signed_top_simplifying_unit(set, left, right, positive).map(SimplifyingUnit::clause)
}

fn find_top_unit_simplifier_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Result<Option<&'set Clause>, Diagnostic> {
    Ok(
        find_signed_top_simplifying_unit_with_bank(bank, set, left, right, positive)?
            .map(SimplifyingUnit::clause),
    )
}

fn find_top_unit_simplifier_plain<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Option<&'set Clause> {
    set.iter().find(|candidate| {
        candidate
            .literals()
            .as_slice()
            .first()
            .is_some_and(|literal| {
                candidate.is_unit()
                    && literal.is_positive() == positive
                    && eqn_topsubsumes_termpair(literal, left, right)
            })
    })
}

fn find_top_unit_simplifier_plain_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
    positive: bool,
) -> Result<Option<&'set Clause>, Diagnostic> {
    for candidate in set.iter() {
        let Some(literal) = candidate.literals().as_slice().first() else {
            continue;
        };
        if candidate.is_unit()
            && literal.is_positive() == positive
            && eqn_topsubsumes_termpair_with_bank(bank, literal, left, right)?
        {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn find_negative_top_unit_simplifier<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    find_top_unit_simplifier(set, left, right, false)
}

fn find_negative_top_unit_simplifier_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Result<Option<&'set Clause>, Diagnostic> {
    find_top_unit_simplifier_with_bank(bank, set, left, right, false)
}

fn find_negative_top_unit_simplifier_plain<'set>(
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Option<&'set Clause> {
    find_top_unit_simplifier_plain(set, left, right, false)
}

fn find_negative_top_unit_simplifier_plain_with_bank<'set>(
    bank: &mut TermBank,
    set: &'set ClauseSet,
    left: &Term,
    right: &Term,
) -> Result<Option<&'set Clause>, Diagnostic> {
    find_top_unit_simplifier_plain_with_bank(bank, set, left, right, false)
}

#[cfg(test)]
mod tests {
    use super::{
        clause_clause_subsumption_calls, clause_clause_subsumption_calls_rec,
        clause_clause_subsumption_successes, clause_is_subsume_ordered,
        clause_negative_simplify_reflect, clause_positive_simplify_reflect,
        clause_positive_simplify_reflect_with_strong,
        clause_positive_simplify_reflect_with_strong_and_docs,
        clause_set_find_first_subsumed_clause, clause_set_find_first_subsumed_clause_with_index,
        clause_set_find_subsumed_clause, clause_set_find_subsumed_clauses,
        clause_set_find_subsumed_clauses_with_index, clause_set_find_unit_subsumed_clause,
        clause_set_find_variant_clause_indexed, clause_set_subsumes_clause,
        clause_set_subsumes_clause_with_index, clause_subsume_order_sort_lits,
        clause_subsumes_clause, clause_subsumes_clause_with_bank, eqn_subsumes_termpair,
        eqn_topsubsumes_termpair, eqn_topsubsumes_termpair_with_bank,
        fv_index_find_first_subsumed_clause, fv_index_find_subsumed_clauses,
        fv_index_find_variant_clause, fv_index_subsumes_packed_clause, literal_subsumes_clause,
        unit_clause_clause_subsumption_calls, unit_clause_set_subsumes_clause,
        unit_clause_set_subsumes_clause_with_strong, unit_clause_subsumes_clause,
    };
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_INPUT_FORMULA, CP_IS_SOS, CP_LIMITED_RW};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{ClauseDerivationRef, DerivationEntry, DC_SR};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::{fv_index_pack_clause, FvIndexAnchor};
    use crate::clauses::freqvectors::{FvCollect, FvCollectLayout, FvIndexType};
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

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
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

    fn prepare(clause: &mut Clause, bank: &TermBank) {
        clause.set_weight(clause.standard_weight());
        clause_subsume_order_sort_lits(clause, bank);
    }

    fn ac_anchor_for_bank(bank: &TermBank) -> FvIndexAnchor {
        let mut cspec = FvCollect::new(FvCollectLayout::new(FvIndexType::AcFeatures, false, 0, 0));
        cspec.set_max_symbols(usize::try_from(bank.signature().f_count()).unwrap() + 1);
        FvIndexAnchor::new(cspec, None)
    }

    fn insert_indexed_clause(anchor: &mut FvIndexAnchor, clause: &Clause, bank: &TermBank) {
        let mut packed = fv_index_pack_clause(clause.clone(), Some(&*anchor));
        assert!(anchor.insert(&mut packed, bank));
    }

    #[test]
    fn positive_unit_subsumption_descends_through_single_differing_subterm_pair() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let unit_lit = literal(&mut bank, &variable, &constant, true);
        let candidate_lit = literal(&mut bank, &left, &right, true);
        let candidate_clause = clause_from(vec![candidate_lit]);

        assert!(eqn_subsumes_termpair(&unit_lit, &left, &right));
        assert!(literal_subsumes_clause(&unit_lit, &candidate_clause));
    }

    #[test]
    fn negative_unit_subsumption_checks_only_top_pair() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let unit_lit = literal(&mut bank, &variable, &constant, false);
        let nested_candidate = clause_from(vec![literal(&mut bank, &left, &right, false)]);
        let top_candidate = clause_from(vec![literal(&mut bank, &other, &constant, false)]);

        assert!(!eqn_topsubsumes_termpair(&unit_lit, &left, &right));
        assert!(!literal_subsumes_clause(&unit_lit, &nested_candidate));
        assert!(literal_subsumes_clause(&unit_lit, &top_candidate));
    }

    #[test]
    fn top_subsumption_preserves_c_swapped_retry_guard() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let first = typed_const(&mut bank, "top_retry_first");
        let second = typed_const(&mut bank, "top_retry_second");
        let unit_lit = literal(&mut bank, &variable, &first, true);

        assert!(!eqn_topsubsumes_termpair(&unit_lit, &first, &second));
        assert!(
            !eqn_topsubsumes_termpair_with_bank(&mut bank, &unit_lit, &first, &second).unwrap()
        );
    }

    #[test]
    fn clause_subsumption_uses_shared_substitution_and_strict_literal_picking() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let witness = typed_const(&mut bank, "c");
        let different = typed_const(&mut bank, "d");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &other, true),
        ]);
        let mut matching = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &witness, &other, true),
        ]);
        let mut not_matching = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &different, &other, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut matching, &bank);
        prepare(&mut not_matching, &bank);

        assert!(clause_is_subsume_ordered(&subsumer, &bank));
        let calls_before = clause_clause_subsumption_calls();
        let rec_before = clause_clause_subsumption_calls_rec();
        let successes_before = clause_clause_subsumption_successes();
        assert!(clause_subsumes_clause(&subsumer, &matching, &bank));
        assert!(!clause_subsumes_clause(&subsumer, &not_matching, &bank));
        assert!(clause_clause_subsumption_calls() >= calls_before + 2);
        assert!(clause_clause_subsumption_calls_rec() >= rec_before + 2);
        assert!(clause_clause_subsumption_successes() > successes_before);
    }

    #[test]
    fn banked_clause_subsumption_shares_higher_order_applied_variable_binding() {
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
        let argument = typed_const(&mut bank, "subsume_ho_argument");
        let first_rhs = typed_const(&mut bank, "subsume_ho_first_rhs");
        let second_rhs = typed_const(&mut bank, "subsume_ho_second_rhs");
        let flex_application =
            apply_terms(&mut bank, &flex, std::slice::from_ref(&argument)).unwrap();
        let rigid_code = bank.signature_mut().insert_id("subsume_ho_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, unary)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let rigid_application =
            apply_terms(&mut bank, &rigid, std::slice::from_ref(&argument)).unwrap();
        let outer_flex = typed_unary(&mut bank, "subsume_ho_outer", &flex_application);
        let outer_rigid = typed_unary(&mut bank, "subsume_ho_outer", &rigid_application);
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &outer_flex, &first_rhs, true),
            literal(&mut bank, &flex_application, &second_rhs, true),
        ]);
        let mut candidate = clause_from(vec![
            literal(&mut bank, &outer_rigid, &first_rhs, true),
            literal(&mut bank, &rigid_application, &second_rhs, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut candidate, &bank);

        assert!(!clause_subsumes_clause(&subsumer, &candidate, &bank));
        assert!(clause_subsumes_clause_with_bank(&subsumer, &candidate, &mut bank).unwrap());
        assert!(flex.binding().is_none());
    }

    #[test]
    fn unit_clause_wrapper_requires_one_literal_and_uses_literal_subsumption() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let witness = typed_const(&mut bank, "b");
        let mut unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let mut candidate = clause_from(vec![literal(&mut bank, &witness, &constant, true)]);
        prepare(&mut unit, &bank);
        prepare(&mut candidate, &bank);

        let unit_calls_before = unit_clause_clause_subsumption_calls();
        assert!(unit_clause_subsumes_clause(&unit, &candidate));
        assert!(clause_subsumes_clause(&unit, &candidate, &bank));
        assert!(unit_clause_clause_subsumption_calls() >= unit_calls_before + 2);
    }

    #[test]
    fn clause_set_subsumes_clause_returns_first_plain_subsumer() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let expected_right = typed_const(&mut bank, "c");
        let mismatch_left = typed_const(&mut bank, "d");
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &mismatch_left, &expected_right, true),
        ]);
        let mut matching = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut candidate = clause_from(vec![
            literal(&mut bank, &other, &constant, true),
            literal(&mut bank, &other, &expected_right, true),
        ]);
        prepare(&mut non_matching, &bank);
        prepare(&mut matching, &bank);
        prepare(&mut candidate, &bank);
        let matching_id = matching.ident();
        let set = ClauseSet::from_clauses([non_matching, matching]);

        assert_eq!(
            clause_set_subsumes_clause(&set, &candidate, &bank).map(Clause::ident),
            Some(matching_id)
        );
    }

    #[test]
    fn clause_set_find_subsumed_clause_honors_start_index() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut non_matching, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([non_matching, first, second]);

        assert_eq!(
            clause_set_find_subsumed_clause(&set, 0, &subsumer, &bank).map(Clause::ident),
            Some(first_id)
        );
        assert_eq!(
            clause_set_find_subsumed_clause(&set, 2, &subsumer, &bank).map(Clause::ident),
            Some(second_id)
        );
        assert!(clause_set_find_subsumed_clause(&set, 3, &subsumer, &bank).is_none());
        assert_eq!(
            clause_set_find_first_subsumed_clause(&set, &subsumer, &bank).map(Clause::ident),
            Some(first_id)
        );
    }

    #[test]
    fn clause_set_find_subsumed_clauses_pushes_all_and_returns_new_count() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut non_matching, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let non_matching_id = non_matching.ident();
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([non_matching, first, second]);
        let mut result = PStack::new();
        result.push(set.iter().next().unwrap());

        assert_eq!(
            clause_set_find_subsumed_clauses(&set, &subsumer, &mut result, &bank),
            2
        );

        assert_eq!(
            result
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![non_matching_id, first_id, second_id]
        );
    }

    #[test]
    fn clause_set_subsumes_clause_with_index_packs_query_and_falls_back() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let witness = typed_const(&mut bank, "c");
        let mismatch_left = typed_const(&mut bank, "d");
        let mut matching = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &witness, &expected_right, true),
        ]);
        let mut candidate = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &witness, &expected_right, true),
        ]);
        prepare(&mut matching, &bank);
        prepare(&mut non_matching, &bank);
        prepare(&mut candidate, &bank);
        let matching_id = matching.ident();
        let set = ClauseSet::from_clauses([non_matching]);
        let mut anchor = ac_anchor_for_bank(&bank);
        insert_indexed_clause(&mut anchor, &matching, &bank);

        assert_eq!(
            clause_set_subsumes_clause_with_index(&set, Some(&anchor), &candidate, &bank)
                .map(Clause::ident),
            Some(matching_id)
        );
        assert!(clause_set_subsumes_clause_with_index(&set, None, &candidate, &bank).is_none());
    }

    #[test]
    fn clause_set_find_subsumed_clauses_with_index_packs_query_and_falls_back() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        prepare(&mut non_matching, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([non_matching]);
        let mut anchor = ac_anchor_for_bank(&bank);
        insert_indexed_clause(&mut anchor, &first, &bank);
        insert_indexed_clause(&mut anchor, &second, &bank);

        let first_match =
            clause_set_find_first_subsumed_clause_with_index(&set, Some(&anchor), &subsumer, &bank)
                .map(Clause::ident);
        assert!(matches!(first_match, Some(id) if id == first_id || id == second_id));

        let mut result = PStack::new();
        assert_eq!(
            clause_set_find_subsumed_clauses_with_index(
                &set,
                Some(&anchor),
                &subsumer,
                &mut result,
                &bank
            ),
            2
        );
        let mut ids = result
            .as_slice()
            .iter()
            .map(|clause| clause.ident())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec![first_id.min(second_id), first_id.max(second_id)]);

        let mut fallback = PStack::new();
        assert_eq!(
            clause_set_find_subsumed_clauses_with_index(
                &set,
                None,
                &subsumer,
                &mut fallback,
                &bank
            ),
            0
        );
        assert!(
            clause_set_find_first_subsumed_clause_with_index(&set, None, &subsumer, &bank)
                .is_none()
        );
    }

    #[test]
    fn fv_index_subsumes_packed_clause_uses_le_feature_ranges() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let witness = typed_const(&mut bank, "c");
        let mut indexed = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut candidate = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &witness, &expected_right, true),
        ]);
        prepare(&mut indexed, &bank);
        prepare(&mut candidate, &bank);
        let indexed_id = indexed.ident();
        let mut anchor = ac_anchor_for_bank(&bank);
        let mut packed_indexed = fv_index_pack_clause(indexed, Some(&anchor));
        let packed_candidate = fv_index_pack_clause(candidate, Some(&anchor));

        assert!(anchor.insert(&mut packed_indexed, &bank));

        assert_eq!(
            fv_index_subsumes_packed_clause(anchor.index(), &packed_candidate, &bank)
                .map(Clause::ident),
            Some(indexed_id)
        );
    }

    #[test]
    fn fv_index_find_subsumed_clauses_uses_ge_feature_ranges() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let first_witness = typed_const(&mut bank, "c");
        let second_witness = typed_const(&mut bank, "d");
        let mismatch_left = typed_const(&mut bank, "e");
        let mut subsumer = clause_from(vec![
            literal(&mut bank, &variable, &constant, true),
            literal(&mut bank, &variable, &expected_right, true),
        ]);
        let mut first = clause_from(vec![
            literal(&mut bank, &first_witness, &constant, true),
            literal(&mut bank, &first_witness, &expected_right, true),
        ]);
        let mut second = clause_from(vec![
            literal(&mut bank, &second_witness, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        let mut non_matching = clause_from(vec![
            literal(&mut bank, &mismatch_left, &constant, true),
            literal(&mut bank, &second_witness, &expected_right, true),
        ]);
        prepare(&mut subsumer, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        prepare(&mut non_matching, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let mut anchor = ac_anchor_for_bank(&bank);
        let mut packed_first = fv_index_pack_clause(first, Some(&anchor));
        let mut packed_second = fv_index_pack_clause(second, Some(&anchor));
        let mut packed_non_matching = fv_index_pack_clause(non_matching, Some(&anchor));
        let packed_subsumer = fv_index_pack_clause(subsumer, Some(&anchor));

        assert!(anchor.insert(&mut packed_first, &bank));
        assert!(anchor.insert(&mut packed_second, &bank));
        assert!(anchor.insert(&mut packed_non_matching, &bank));

        let first_match =
            fv_index_find_first_subsumed_clause(anchor.index(), &packed_subsumer, &bank)
                .map(Clause::ident);
        assert!(matches!(first_match, Some(id) if id == first_id || id == second_id));

        let mut result = PStack::new();
        assert_eq!(
            fv_index_find_subsumed_clauses(anchor.index(), &packed_subsumer, &mut result, &bank),
            2
        );
        let mut ids = result
            .as_slice()
            .iter()
            .map(|clause| clause.ident())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        assert_eq!(ids, vec![first_id.min(second_id), first_id.max(second_id)]);
    }

    #[test]
    fn fv_index_find_variant_clause_follows_exact_feature_path() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -10);
        let y = typed_var(&bank, -11);
        let z = typed_var(&bank, -12);
        let constant = typed_const(&mut bank, "a");
        let expected_right = typed_const(&mut bank, "b");
        let mut query = clause_from(vec![
            literal(&mut bank, &x, &constant, true),
            literal(&mut bank, &x, &expected_right, true),
        ]);
        let mut non_variant = clause_from(vec![
            literal(&mut bank, &y, &constant, true),
            literal(&mut bank, &z, &expected_right, true),
        ]);
        let mut variant = clause_from(vec![
            literal(&mut bank, &y, &constant, true),
            literal(&mut bank, &y, &expected_right, true),
        ]);
        prepare(&mut query, &bank);
        prepare(&mut non_variant, &bank);
        prepare(&mut variant, &bank);
        let variant_id = variant.ident();
        let mut anchor = ac_anchor_for_bank(&bank);
        let mut packed_non_variant = fv_index_pack_clause(non_variant, Some(&anchor));
        let mut packed_variant = fv_index_pack_clause(variant, Some(&anchor));
        let packed_query = fv_index_pack_clause(query, Some(&anchor));

        assert!(anchor.insert(&mut packed_non_variant, &bank));
        assert!(anchor.insert(&mut packed_variant, &bank));

        assert_eq!(
            fv_index_find_variant_clause(anchor.index(), &packed_query, &bank).map(Clause::ident),
            Some(variant_id)
        );
        assert_eq!(
            clause_set_find_variant_clause_indexed(&anchor, packed_query.clause(), &bank)
                .map(Clause::ident),
            Some(variant_id)
        );
    }

    #[test]
    fn unit_clause_set_subsumes_clause_returns_first_matching_unit() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let positive_id = positive_unit.ident();
        let negative_unit = clause_from(vec![literal(&mut bank, &variable, &constant, false)]);
        let negative_id = negative_unit.ident();
        let set = ClauseSet::from_clauses([positive_unit, negative_unit]);
        let positive_candidate = clause_from(vec![literal(&mut bank, &left, &right, true)]);
        let negative_candidate = clause_from(vec![literal(&mut bank, &other, &constant, false)]);

        assert_eq!(
            unit_clause_set_subsumes_clause(&set, &positive_candidate).map(Clause::ident),
            Some(positive_id)
        );
        assert_eq!(
            unit_clause_set_subsumes_clause(&set, &negative_candidate).map(Clause::ident),
            Some(negative_id)
        );
    }

    #[test]
    fn strong_unit_clause_set_subsumption_checks_multiple_differing_subterms() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let left_other = typed_const(&mut bank, "b");
        let right_other = typed_const(&mut bank, "c");
        let right_match = typed_const(&mut bank, "d");
        let left = typed_binary(&mut bank, "f", &left_other, &right_other);
        let right = typed_binary(&mut bank, "f", &constant, &right_match);
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &right_match, true)]);
        let positive_id = positive_unit.ident();
        let set = ClauseSet::from_clauses([positive_unit]);
        let target = clause_from(vec![literal(&mut bank, &left, &right, true)]);

        assert!(unit_clause_set_subsumes_clause(&set, &target).is_none());
        assert_eq!(
            unit_clause_set_subsumes_clause_with_strong(&set, &target, true).map(Clause::ident),
            Some(positive_id)
        );
    }

    #[test]
    fn clause_set_find_unit_subsumed_clause_honors_start_index() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let first_witness = typed_const(&mut bank, "b");
        let second_witness = typed_const(&mut bank, "c");
        let mut unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let mut first = clause_from(vec![literal(&mut bank, &first_witness, &constant, true)]);
        let mut second = clause_from(vec![literal(&mut bank, &second_witness, &constant, true)]);
        prepare(&mut unit, &bank);
        prepare(&mut first, &bank);
        prepare(&mut second, &bank);
        let first_id = first.ident();
        let second_id = second.ident();
        let set = ClauseSet::from_clauses([first, second]);

        assert_eq!(
            clause_set_find_unit_subsumed_clause(&set, 0, &unit).map(Clause::ident),
            Some(first_id)
        );
        assert_eq!(
            clause_set_find_unit_subsumed_clause(&set, 1, &unit).map(Clause::ident),
            Some(second_id)
        );
        assert!(clause_set_find_unit_subsumed_clause(&set, 2, &unit).is_none());
    }

    #[test]
    fn positive_simplify_reflect_removes_negative_literals_and_propagates_sos() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let unrelated = typed_const(&mut bank, "c");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let mut positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        positive_unit.set_ident(101);
        positive_unit.set_prop(CP_IS_SOS);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &unrelated, &constant, true),
        ]);
        target.set_weight(target.standard_weight());
        let original_weight = target.weight();
        let removed_weight = target.literals().as_slice()[1].standard_weight();
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(!clause_positive_simplify_reflect(&set, &mut target));

        assert_eq!(target.positive_literal_count(), 1);
        assert_eq!(target.negative_literal_count(), 0);
        assert_eq!(target.weight(), original_weight - removed_weight);
        assert!(target.query_prop(CP_IS_SOS));
        assert!(!target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(101, 0)),
            ]
        );
    }

    #[test]
    fn positive_simplify_reflect_uses_demod_index_candidate_order() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "indexed_sr_a");
        let rhs = typed_const(&mut bank, "indexed_sr_rhs");
        let kept = typed_const(&mut bank, "indexed_sr_kept");
        let mut specific = clause_from(vec![literal(&mut bank, &constant, &rhs, true)]);
        let mut general = clause_from(vec![literal(&mut bank, &variable, &rhs, true)]);
        specific.set_ident(106);
        general.set_ident(107);
        specific.set_weight(specific.standard_weight());
        general.set_weight(general.standard_weight());
        let mut set = ClauseSet::new_demod_indexed();
        set.indexed_insert_clause_owned(specific, &bank);
        set.indexed_insert_clause_owned(general, &bank);
        let mut target = clause_from(vec![
            literal(&mut bank, &constant, &rhs, false),
            literal(&mut bank, &kept, &rhs, true),
        ]);
        target.set_weight(target.standard_weight());

        assert_eq!(set.demod_index_match_count(), 0);
        assert!(!clause_positive_simplify_reflect(&set, &mut target));

        assert_eq!(set.demod_index_match_count(), 1);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert_eq!(target.literal_number(), 1);
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(107, 0)),
            ]
        );
    }

    #[test]
    fn positive_simplify_reflect_with_docs_emits_step_before_derivation() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "doc_sr_a");
        let witness = typed_const(&mut bank, "doc_sr_b");
        let kept = typed_const(&mut bank, "doc_sr_c");
        let mut positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        positive_unit.set_ident(104);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &witness, &constant, false),
            literal(&mut bank, &kept, &constant, true),
        ]);
        target.set_ident(205);
        target.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        assert!(!clause_positive_simplify_reflect_with_strong_and_docs(
            &mut rendered,
            &mut session,
            &bank,
            &set,
            &mut target,
            false,
        )
        .unwrap());

        assert_eq!(target.ident(), 1);
        assert_eq!(target.literal_number(), 1);
        assert!(!target.is_any_prop_set(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW));
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("sr(205,104)"));
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(104, 0)),
            ]
        );
    }

    #[test]
    fn strong_positive_simplify_reflect_removes_multi_difference_negative_literal() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let left_other = typed_const(&mut bank, "b");
        let right_other = typed_const(&mut bank, "c");
        let right_match = typed_const(&mut bank, "d");
        let kept_left = typed_const(&mut bank, "e");
        let kept_right = typed_const(&mut bank, "g");
        let left = typed_binary(&mut bank, "f", &left_other, &right_other);
        let right = typed_binary(&mut bank, "f", &constant, &right_match);
        let mut positive_unit =
            clause_from(vec![literal(&mut bank, &variable, &right_match, true)]);
        positive_unit.set_ident(102);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut default_target = clause_from(vec![
            literal(&mut bank, &left, &right, false),
            literal(&mut bank, &kept_left, &kept_right, true),
        ]);
        let mut strong_target = default_target.clone();
        default_target.set_weight(default_target.standard_weight());
        strong_target.set_weight(strong_target.standard_weight());

        assert!(!clause_positive_simplify_reflect(&set, &mut default_target));
        assert_eq!(default_target.negative_literal_count(), 1);
        assert!(default_target.derivation().is_none());

        assert!(!clause_positive_simplify_reflect_with_strong(
            &set,
            &mut strong_target,
            true
        ));
        assert_eq!(strong_target.negative_literal_count(), 0);
        assert_eq!(strong_target.positive_literal_count(), 1);
        assert_eq!(
            strong_target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(102, 0)),
            ]
        );
    }

    #[test]
    fn negative_simplify_reflect_uses_top_level_negative_units_only() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let other = typed_const(&mut bank, "b");
        let left = typed_unary(&mut bank, "f", &other);
        let right = typed_unary(&mut bank, "f", &constant);
        let mut negative_unit = clause_from(vec![literal(&mut bank, &variable, &constant, false)]);
        negative_unit.set_ident(103);
        let set = ClauseSet::from_clauses([negative_unit]);
        let mut target = clause_from(vec![
            literal(&mut bank, &left, &right, true),
            literal(&mut bank, &other, &constant, true),
        ]);
        target.set_weight(target.standard_weight());
        let top_literal_weight = target.literals().as_slice()[1].standard_weight();
        let original_weight = target.weight();
        target.set_prop(CP_INITIAL | CP_LIMITED_RW);

        assert!(!clause_negative_simplify_reflect(&set, &mut target));

        assert_eq!(target.positive_literal_count(), 1);
        assert_eq!(target.negative_literal_count(), 0);
        assert_eq!(target.weight(), original_weight - top_literal_weight);
        assert_eq!(target.literals().as_slice()[0].left(), &left);
        assert!(!target.query_prop(CP_INITIAL));
        assert!(!target.query_prop(CP_LIMITED_RW));
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(103, 0)),
            ]
        );
    }

    #[test]
    fn negative_simplify_reflect_uses_demod_indexed_top_lookup() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "indexed_neg_sr_a");
        let witness = typed_const(&mut bank, "indexed_neg_sr_witness");
        let kept = typed_const(&mut bank, "indexed_neg_sr_kept");
        let mut negative_unit = clause_from(vec![literal(&mut bank, &variable, &constant, false)]);
        negative_unit.set_ident(108);
        negative_unit.set_weight(negative_unit.standard_weight());
        let mut set = ClauseSet::new_demod_indexed();
        set.indexed_insert_clause_owned(negative_unit, &bank);
        let mut target = clause_from(vec![
            literal(&mut bank, &witness, &constant, true),
            literal(&mut bank, &kept, &constant, true),
        ]);
        target.set_weight(target.standard_weight());

        assert_eq!(set.demod_index_match_count(), 0);
        assert!(clause_negative_simplify_reflect(&set, &mut target));

        assert_eq!(set.demod_index_match_count(), 2);
        assert_eq!(
            set.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
        assert_eq!(target.literal_number(), 0);
        assert_eq!(
            target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(108, 0)),
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(108, 0)),
            ]
        );
    }

    #[test]
    fn simplify_reflect_reports_empty_clause_after_last_literal_removed() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -10);
        let constant = typed_const(&mut bank, "a");
        let witness = typed_const(&mut bank, "b");
        let positive_unit = clause_from(vec![literal(&mut bank, &variable, &constant, true)]);
        let set = ClauseSet::from_clauses([positive_unit]);
        let mut target = clause_from(vec![literal(&mut bank, &witness, &constant, false)]);
        target.set_weight(target.standard_weight());

        assert!(clause_positive_simplify_reflect(&set, &mut target));

        assert!(target.is_empty());
        assert_eq!(target.weight(), 0);
    }
}
