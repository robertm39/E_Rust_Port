use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::CompareResult;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{tptp_types_combine, CP_IS_SOS, CP_NO_GENERATION};
use crate::clauses::clausecpos::{unpack_clause_pos, unpack_clause_pos_literal};
use crate::clauses::clausepos::ClausePos;
use crate::clauses::clausepos_tree::{clause_key, ClauseTPos};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, set_is_ho, DC_PARAMOD, DC_SIM_PARAMOD};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnSide, EP_FROM_CLAUSE_LIT, EP_IS_MAXIMAL, EP_IS_PM_INTO_LIT};
use crate::clauses::eqnlist::EqnList;
use crate::clauses::inferencedoc::{
    ClauseCreationInference, ClauseCreationParents, ProofDocSession,
};
use crate::clauses::overlap_index::{
    clause_collect_from_terms_pos, clause_collect_into_terms_pos, OverlapIndex,
};
use crate::clauses::subterm_tree::SubtermOcc;
use crate::heuristics::to_params::TermOrdering;
#[cfg(not(target_os = "linux"))]
use crate::inout::signals::{time_is_up, time_limit_expired_kind};
use crate::orderings::cto_orderings::to_greater_with_bank;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::ho_csu::CsuIterator;
use crate::terms::match_mgu::{
    subst_mgu_complete_with_bank, term_has_higher_order_unification_surface,
};
use crate::terms::replace::{make_rewritten_term, tb_term_pos_replace};
use crate::terms::signature::Signature;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_standard_weight;
use crate::terms::termpos::TermPos;
use crate::terms::termtypes::{term_deref, DerefType, Term, TP_POTENTIAL_PARAMOD};
use crate::terms::termvars::VarBank;
use std::{collections::BTreeMap, fmt};

pub const PARAMOD_OVERLAP_NON_EQ_LITERALS: bool = true;
#[cfg(not(target_os = "linux"))]
const PARAMODULATION_TIME_CHECK_INTERVAL: usize = 64;

#[inline]
fn paramodulation_time_is_up_before_next_insert(store: &ClauseSet) -> bool {
    #[cfg(target_os = "linux")]
    {
        let _ = store;
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        if time_limit_expired_kind().is_some() {
            return true;
        }
        let next_len = store.len().saturating_add(1);
        next_len.is_multiple_of(PARAMODULATION_TIME_CHECK_INTERVAL) && time_is_up()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParamodulationType {
    Plain,
    Simultaneous,
    OrientedSimultaneous,
    SuperSimultaneous,
    OrientedSuperSimultaneous,
    DecreasingSimultaneous,
    SizeDecreasingSimultaneous,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamodulationPair {
    from: ClausePos,
    into: ClausePos,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SimParamodReplacement {
    SharedTarget,
    InstantiatedTargetCopy,
}

impl ParamodulationPair {
    #[must_use]
    pub const fn new(from: ClausePos, into: ClausePos) -> Self {
        Self { from, into }
    }

    #[must_use]
    pub const fn source(&self) -> &ClausePos {
        &self.from
    }

    #[must_use]
    pub const fn target(&self) -> &ClausePos {
        &self.into
    }
}

/// Returns C `ClausePosFirst/NextParamodFromSide` source-side candidates.
///
/// The current C defaults allow overlap from non-equational positive maximal
/// literals, so the only active filter beyond maximal positive side iteration
/// is selected-literal rejection.
#[must_use]
pub fn paramod_from_side_positions(bank: &TermBank, from: &Clause) -> Vec<ClausePos> {
    let mut positions = Vec::new();
    let mut position = ClausePos::for_clause(from.clone());
    let mut current = position.find_first_maximal_side(true);

    while current.is_some() {
        if from_side_allows_paramod(bank, &position) {
            positions.push(position.clone());
        }
        current = position.find_next_maximal_side(true);
    }

    positions
}

/// Returns C `ClausePosFirst/NextParamodInto` target-position candidates for a
/// fixed source side.
#[must_use]
pub fn paramod_into_positions(
    bank: &TermBank,
    into: &Clause,
    from_pos: &ClausePos,
    no_top: bool,
    pm_type: ParamodulationType,
) -> Vec<ClausePos> {
    let mut positions = Vec::new();
    let mut position = ClausePos::for_clause(into.clone());
    let mut current = first_paramod_into_candidate(bank, &mut position, from_pos, no_top);

    if pm_type != ParamodulationType::Plain && current.is_some() {
        mark_potential_paramod_terms_from_position(&position);
    }

    while current.is_some() {
        positions.push(position.clone());
        current = next_paramod_into_candidate(bank, &mut position, from_pos, no_top);
    }

    positions
}

/// Returns C `ClausePosFirst/NextParamodPair` candidates in cursor order.
#[must_use]
pub fn paramodulation_pair_positions(
    bank: &TermBank,
    from: &Clause,
    into: &Clause,
    no_top: bool,
    pm_type: ParamodulationType,
) -> Vec<ParamodulationPair> {
    let mut pairs = Vec::new();
    for from_pos in paramod_from_side_positions(bank, from) {
        for into_pos in paramod_into_positions(bank, into, &from_pos, no_top, pm_type) {
            pairs.push(ParamodulationPair::new(from_pos.clone(), into_pos));
        }
    }
    pairs
}

/// Computes all currently ported first-order paramodulants between two clauses
/// and inserts them into `store`.
///
/// `parent_alias` carries the source-parent metadata for C callers that
/// paramodulate from a temporary clause view but document the original parent.
///
/// # Errors
///
/// Returns diagnostics from the low-level paramodulation constructor. Plain,
/// simultaneous, and super-simultaneous paramodulation are currently supported
/// for first-order unindexed generation and the KBO6 first-order-shaped
/// higher-order selected-overlap subset.
pub fn compute_clause_clause_paramodulants(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with: &Clause,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_clause_clause_paramodulants_impl::<String>(
        bank,
        ocb,
        clause,
        parent_alias,
        with,
        store,
        pm_type,
        None,
    )
}

/// Computes first-order paramodulants between two clauses while emitting
/// represented C `DocClauseCreationDefault(..., inf_paramod/inf_sim_paramod,
/// ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_clause_clause_paramodulants`],
/// plus any proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible docs wrapper mirrors ComputeClauseClauseParamodulants inputs"
)]
pub fn compute_clause_clause_paramodulants_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with: &Clause,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_clause_clause_paramodulants_impl(
        bank,
        ocb,
        clause,
        parent_alias,
        with,
        store,
        pm_type,
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps source, target, and optional proof docs explicit"
)]
fn compute_clause_clause_paramodulants_impl<W: fmt::Write>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with: &Clause,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    debug_assert!(clause.literals().query_prop_number(EP_IS_MAXIMAL) != 0);
    debug_assert_eq!(
        clause.literals().query_prop_number(EP_IS_MAXIMAL),
        parent_alias.literals().query_prop_number(EP_IS_MAXIMAL)
    );
    debug_assert!(with.literals().query_prop_number(EP_IS_MAXIMAL) != 0);

    if clause.query_prop(CP_NO_GENERATION) || with.query_prop(CP_NO_GENERATION) {
        return Ok(0);
    }

    let mut paramod_count = compute_directed_clause_paramodulants(
        bank,
        ocb,
        clause,
        parent_alias,
        with,
        store,
        false,
        parent_alias,
        with,
        pm_type,
        &mut doc_context,
    )?;

    if !std::ptr::eq(parent_alias, with) {
        paramod_count += compute_directed_clause_paramodulants(
            bank,
            ocb,
            with,
            with,
            clause,
            store,
            true,
            with,
            parent_alias,
            pm_type,
            &mut doc_context,
        )?;
    }

    Ok(paramod_count)
}

/// Computes all currently ported first-order paramodulants between one clause
/// and every clause in `with_set`.
///
/// # Errors
///
/// Returns diagnostics from [`compute_clause_clause_paramodulants`].
pub fn compute_all_paramodulants(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with_set: &ClauseSet,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_impl::<String>(
        bank,
        ocb,
        clause,
        parent_alias,
        with_set,
        store,
        pm_type,
        None,
    )
}

/// Computes first-order paramodulants against a clause set while emitting
/// represented C `DocClauseCreationDefault(..., inf_paramod/inf_sim_paramod,
/// ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_paramodulants`], plus any
/// proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible docs wrapper mirrors ComputeAllParamodulants inputs"
)]
pub fn compute_all_paramodulants_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with_set: &ClauseSet,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_impl(
        bank,
        ocb,
        clause,
        parent_alias,
        with_set,
        store,
        pm_type,
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps selected clause, partner set, and optional proof docs explicit"
)]
fn compute_all_paramodulants_impl<W: fmt::Write>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    with_set: &ClauseSet,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    for with in with_set.iter() {
        paramod_count += compute_clause_clause_paramodulants_impl(
            bank,
            ocb,
            clause,
            parent_alias,
            with,
            store,
            pm_type,
            doc_context
                .as_mut()
                .map(|(output, session)| (&mut **output, &mut **session)),
        )?;
    }
    Ok(paramod_count)
}

/// Computes all currently ported first-order paramodulants between one
/// selected clause and clauses stored in the global paramodulation indexes.
///
/// This mirrors C `ComputeAllParamodulantsIndexed`: source-side positions of
/// `clause` are queried against the into/negative-predicate indexes, then
/// target positions of `clause` are queried against the from-index while
/// skipping positive top-level targets already covered by the first pass.
///
/// `parent_alias` carries metadata for C callers that paramodulate from a
/// temporary selected-clause copy but document the original selected clause.
///
/// # Errors
///
/// Returns diagnostics from the low-level paramodulation constructor. Plain,
/// simultaneous, and super-simultaneous indexed paramodulation are supported.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible wrapper mirrors ComputeAllParamodulantsIndexed inputs"
)]
pub fn compute_all_paramodulants_indexed(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_indexed_impl::<String>(
        bank,
        ocb,
        clause,
        parent_alias,
        into_index,
        negp_index,
        from_index,
        store,
        pm_type,
        None,
        None,
    )
}

/// Computes indexed paramodulants using a caller-owned C `freshvars` bank.
///
/// The caller-owned bank is reset before each candidate construction, matching
/// C `ClauseParamodConstruct` and the ordered-paramodulation constructors.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_paramodulants_indexed`].
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible wrapper mirrors ComputeAllParamodulantsIndexed inputs"
)]
pub fn compute_all_paramodulants_indexed_with_fresh_vars(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_indexed_impl::<String>(
        bank,
        ocb,
        clause,
        parent_alias,
        into_index,
        negp_index,
        from_index,
        store,
        pm_type,
        Some(freshvars),
        None,
    )
}

/// Computes indexed first-order paramodulants while emitting represented C
/// `DocClauseCreationDefault(..., inf_paramod/inf_sim_paramod, ...)` output.
///
/// # Errors
///
/// Returns the same diagnostics as [`compute_all_paramodulants_indexed`], plus
/// any proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible docs wrapper mirrors ComputeAllParamodulantsIndexed inputs"
)]
pub fn compute_all_paramodulants_indexed_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_indexed_impl(
        bank,
        ocb,
        clause,
        parent_alias,
        into_index,
        negp_index,
        from_index,
        store,
        pm_type,
        None,
        Some((output, session)),
    )
}

/// Computes indexed paramodulants using a caller-owned C `freshvars` bank
/// while emitting represented proof-documentation output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`compute_all_paramodulants_indexed_with_fresh_vars`], plus any
/// proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible docs wrapper mirrors ComputeAllParamodulantsIndexed inputs"
)]
pub fn compute_all_paramodulants_indexed_with_fresh_vars_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    freshvars: &VarBank,
) -> Result<i64, Diagnostic> {
    compute_all_paramodulants_indexed_impl(
        bank,
        ocb,
        clause,
        parent_alias,
        into_index,
        negp_index,
        from_index,
        store,
        pm_type,
        Some(freshvars),
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed wrapper keeps source clause, indexes, and optional proof docs explicit"
)]
fn compute_all_paramodulants_indexed_impl<W: fmt::Write>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = compute_into_paramodulants_indexed(
        bank,
        ocb,
        clause,
        parent_alias,
        into_index,
        negp_index,
        store,
        pm_type,
        freshvars,
        &mut doc_context,
    )?;
    paramod_count += compute_from_paramodulants_indexed(
        bank,
        ocb,
        clause,
        parent_alias,
        from_index,
        store,
        pm_type,
        freshvars,
        &mut doc_context,
    )?;
    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed wrapper keeps source clause and indexes explicit"
)]
fn compute_into_paramodulants_indexed(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    into_index: &OverlapIndex,
    negp_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    let mut positions = Vec::new();
    let _ = clause_collect_from_terms_pos(clause, &mut positions);

    for entry in positions.iter().rev() {
        let from_pos = unpack_clause_pos(entry.pos(), clause.clone());
        paramod_count += compute_from_position_into_index(
            bank,
            ocb,
            &from_pos,
            entry.term(),
            negp_index,
            store,
            parent_alias,
            pm_type,
            freshvars,
            doc_context,
        )?;
        if from_pos
            .literal()
            .expect("indexed from position must select a literal")
            .is_equ_lit(bank)
        {
            paramod_count += compute_from_position_into_index(
                bank,
                ocb,
                &from_pos,
                entry.term(),
                into_index,
                store,
                parent_alias,
                pm_type,
                freshvars,
                doc_context,
            )?;
        }
    }

    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed wrapper keeps selected source, index, and optional docs explicit"
)]
fn compute_from_paramodulants_indexed(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    parent_alias: &Clause,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    let mut positions = Vec::new();
    let _ = clause_collect_into_terms_pos(clause, &mut positions);

    for entry in positions.iter().rev() {
        let into_pos = unpack_clause_pos(entry.pos(), clause.clone());
        let into_literal = into_pos
            .literal()
            .expect("indexed into position must select a literal");
        if into_literal.is_negative() || !into_pos.is_top() {
            paramod_count += compute_indexed_sources_into_position(
                bank,
                ocb,
                entry.term(),
                &into_pos,
                from_index,
                store,
                parent_alias,
                pm_type,
                freshvars,
                doc_context,
            )?;
        }
    }

    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed wrapper keeps selected source and target index explicit"
)]
fn compute_from_position_into_index(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
    overlap_term: &Term,
    index: &OverlapIndex,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    for occurrence in unifiable_occurrences(index, overlap_term, bank.signature()) {
        paramod_count += compute_from_position_into_occurrence(
            bank,
            ocb,
            from_pos,
            occurrence,
            store,
            parent_alias,
            pm_type,
            freshvars,
            doc_context,
        )?;
    }
    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed helper keeps source occurrence, target occurrence, and optional docs explicit"
)]
fn compute_from_position_into_occurrence(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
    occurrence: &SubtermOcc,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    if problem_type() == ProblemType::HigherOrder {
        return compute_from_position_into_occurrence_csu(
            bank,
            ocb,
            from_pos,
            occurrence,
            store,
            parent_alias,
            pm_type,
            freshvars,
            doc_context,
        );
    }

    let from_term = from_pos
        .get_side()
        .expect("indexed source position must select a side");
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[&from_term, occurrence.term()],
        || higher_order_paramod_diagnostic_for_type(pm_type),
    )?;

    let mut subst = Substitution::new();
    let result = (|| {
        if !subst_mgu_complete_with_bank(bank, &from_term, occurrence.term(), &mut subst)?
            || !indexed_source_allows_under_subst(bank, ocb, from_pos)?
        {
            return Ok(0);
        }
        let effective_pm_type = effective_paramodulation_type(bank, ocb, from_pos, pm_type)?;
        let mut paramod_count = 0;
        for target_entry in occurrence.position_clauses().entries() {
            paramod_count += compute_from_position_into_target_clause_entry_with_subst(
                bank,
                ocb,
                from_pos,
                target_entry,
                store,
                parent_alias,
                &mut subst,
                false,
                effective_pm_type,
                pm_type,
                freshvars,
                doc_context,
            )?;
        }
        Ok(paramod_count)
    })();
    subst.backtrack();
    result
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed wrapper keeps selected target and source index explicit"
)]
fn compute_indexed_sources_into_position(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    overlap_term: &Term,
    into_pos: &ClausePos,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    if problem_type() == ProblemType::HigherOrder {
        return compute_indexed_sources_into_position_csu(
            bank,
            ocb,
            overlap_term,
            into_pos,
            from_index,
            store,
            parent_alias,
            pm_type,
            freshvars,
            doc_context,
        );
    }

    let mut paramod_count = 0;
    let parent_key = clause_key(parent_alias);
    let into_clause = into_pos
        .clause()
        .expect("indexed target position must be backed by its working clause");
    for occurrence in unifiable_occurrences(from_index, overlap_term, bank.signature()) {
        let mut subst = Substitution::new();
        let unified =
            match subst_mgu_complete_with_bank(bank, overlap_term, occurrence.term(), &mut subst) {
                Ok(unified) => unified,
                Err(error) => {
                    subst.backtrack();
                    return Err(error);
                }
            };
        if !unified {
            subst.backtrack();
            continue;
        }

        let generated = (|| {
            if !indexed_target_allows_under_subst(bank, ocb, into_pos, into_clause)? {
                return Ok(0);
            }
            let mut generated = 0;
            for source_entry in occurrence.position_clauses().entries() {
                if source_entry.clause_key() == parent_key {
                    continue;
                }
                generated += compute_indexed_sources_from_clause_entry_with_subst(
                    bank,
                    ocb,
                    source_entry,
                    into_pos,
                    store,
                    parent_alias,
                    &mut subst,
                    false,
                    pm_type,
                    freshvars,
                    doc_context,
                )?;
            }
            Ok(generated)
        })();
        subst.backtrack();
        paramod_count += generated?;
    }
    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed CSU helper keeps source occurrence, target occurrence, mode, and optional docs explicit"
)]
fn compute_from_position_into_occurrence_csu(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
    occurrence: &SubtermOcc,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let from_term = from_pos
        .get_side()
        .expect("indexed source position must select a side");
    let from_other = from_pos
        .get_other_side()
        .expect("indexed source position must select an opposite side");
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[&from_term, &from_other, occurrence.term()],
        || higher_order_paramod_diagnostic_for_type(pm_type),
    )?;

    let mut subst = Substitution::new();
    let mut iter = CsuIterator::new(&from_term, occurrence.term(), &subst);
    let mut paramod_count = 0;

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

        if !indexed_source_allows_under_subst(bank, ocb, from_pos)? {
            continue;
        }
        let subst_is_ho = subst.has_ho_binding();
        let effective_pm_type = effective_paramodulation_type(bank, ocb, from_pos, pm_type)?;

        for into_clause_pos in occurrence.position_clauses().entries() {
            let generated = match compute_from_position_into_target_clause_entry_with_subst(
                bank,
                ocb,
                from_pos,
                into_clause_pos,
                store,
                parent_alias,
                &mut subst,
                subst_is_ho,
                effective_pm_type,
                pm_type,
                freshvars,
                doc_context,
            ) {
                Ok(generated) => generated,
                Err(err) => {
                    iter.destroy(&mut subst);
                    return Err(err);
                }
            };
            paramod_count += generated;
        }
    }

    iter.destroy(&mut subst);
    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed helper keeps source, target entry, active substitution, and optional docs explicit"
)]
fn compute_from_position_into_target_clause_entry_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
    target_entry: &ClauseTPos,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    subst: &mut Substitution,
    subst_is_ho: bool,
    effective_pm_type: ParamodulationType,
    requested_pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    let is_simultaneous = paramodulation_is_simultaneous(effective_pm_type);
    let mut marked_term = None;
    for into_cpos in target_entry.positions() {
        if paramodulation_time_is_up_before_next_insert(store) {
            break;
        }
        let into_pos = unpack_clause_pos_literal(*into_cpos, target_entry.clause());
        ensure_indexed_paramodulation_ordering_supported(
            ocb,
            from_pos,
            &into_pos,
            requested_pm_type,
        )?;
        if is_simultaneous {
            let into_term = into_pos
                .get_subterm()
                .expect("indexed target position must select a subterm");
            if marked_term.is_none() {
                into_term.set_prop(TP_POTENTIAL_PARAMOD);
                marked_term = Some(into_term.clone());
            } else if !into_term.query_prop(TP_POTENTIAL_PARAMOD) {
                break;
            }
        }
        if !indexed_target_allows_under_subst(bank, ocb, &into_pos, target_entry.clause())? {
            continue;
        }

        let from_clause = from_pos
            .clause()
            .expect("indexed source position must be backed by a clause");
        let scratch_freshvars = freshvars
            .is_none()
            .then(|| fresh_var_bank_for_clauses(bank, from_clause, target_entry.clause()));
        let freshvars = freshvars.unwrap_or_else(|| {
            scratch_freshvars
                .as_ref()
                .expect("missing indexed paramodulation scratch variable bank")
        });
        freshvars.reset_v_counts();
        let paramodulant = indexed_paramod_construct_with_subst(
            bank,
            ocb,
            from_pos,
            &into_pos,
            from_clause,
            target_entry.clause(),
            freshvars,
            subst,
            effective_pm_type,
        );
        let paramodulant = match paramodulant {
            Ok(paramodulant) => paramodulant,
            Err(err) => {
                if let Some(term) = marked_term {
                    term.del_prop(TP_POTENTIAL_PARAMOD);
                }
                return Err(err);
            }
        };
        let Some(mut paramodulant) = paramodulant else {
            continue;
        };
        paramod_count += 1;
        update_paramodulant_info(&mut paramodulant, target_entry.clause(), parent_alias);
        if let Err(err) = document_paramodulant_creation(
            doc_context,
            bank,
            &mut paramodulant,
            effective_pm_type,
            target_entry.clause(),
            parent_alias,
        ) {
            if let Some(term) = marked_term {
                term.del_prop(TP_POTENTIAL_PARAMOD);
            }
            return Err(err);
        }
        clause_push_derivation(
            &mut paramodulant,
            paramodulation_derivation_code_with_ho(effective_pm_type, subst_is_ho),
            Some(target_entry.clause()),
            Some(parent_alias),
        );
        store.insert(paramodulant);
        if is_simultaneous {
            break;
        }
    }
    if let Some(term) = marked_term {
        term.del_prop(TP_POTENTIAL_PARAMOD);
    }
    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed CSU helper keeps selected target, source occurrence, and optional docs explicit"
)]
fn compute_indexed_sources_into_position_csu(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    overlap_term: &Term,
    into_pos: &ClausePos,
    from_index: &OverlapIndex,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let into_side = into_pos
        .get_side()
        .expect("indexed target position must select a side");
    let into_other = into_pos
        .get_other_side()
        .expect("indexed target position must select an opposite side");
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[overlap_term, &into_side, &into_other],
        || higher_order_paramod_diagnostic_for_type(pm_type),
    )?;

    let mut paramod_count = 0;
    let parent_key = clause_key(parent_alias);
    let into_clause = into_pos
        .clause()
        .expect("indexed target position must be backed by its working clause");
    for occurrence in unifiable_occurrences(from_index, overlap_term, bank.signature()) {
        ensure_higher_order_paramodulation_ordering_supported(
            ocb,
            &[overlap_term, occurrence.term()],
            || higher_order_paramod_diagnostic_for_type(pm_type),
        )?;
        let mut subst = Substitution::new();
        let mut iter = CsuIterator::new(overlap_term, occurrence.term(), &subst);

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

            if !indexed_target_allows_under_subst(bank, ocb, into_pos, into_clause)? {
                continue;
            }
            let subst_is_ho = subst.has_ho_binding();

            for from_clause_pos in occurrence.position_clauses().entries() {
                if from_clause_pos.clause_key() == parent_key {
                    continue;
                }
                let generated = match compute_indexed_sources_from_clause_entry_with_subst(
                    bank,
                    ocb,
                    from_clause_pos,
                    into_pos,
                    store,
                    parent_alias,
                    &mut subst,
                    subst_is_ho,
                    pm_type,
                    freshvars,
                    doc_context,
                ) {
                    Ok(generated) => generated,
                    Err(err) => {
                        iter.destroy(&mut subst);
                        return Err(err);
                    }
                };
                paramod_count += generated;
            }
        }

        iter.destroy(&mut subst);
    }

    Ok(paramod_count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible indexed helper keeps source entry, target position, active substitution, and optional docs explicit"
)]
fn compute_indexed_sources_from_clause_entry_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    source_entry: &ClauseTPos,
    into_pos: &ClausePos,
    store: &mut ClauseSet,
    parent_alias: &Clause,
    subst: &mut Substitution,
    subst_is_ho: bool,
    pm_type: ParamodulationType,
    freshvars: Option<&VarBank>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    let into_clause = into_pos
        .clause()
        .expect("indexed target position must be backed by its working clause");
    for source_cpos in source_entry.positions() {
        if paramodulation_time_is_up_before_next_insert(store) {
            break;
        }
        let source_pos = unpack_clause_pos(*source_cpos, source_entry.clause().clone());
        ensure_indexed_paramodulation_ordering_supported(ocb, &source_pos, into_pos, pm_type)?;
        if !indexed_source_allows_under_subst(bank, ocb, &source_pos)? {
            continue;
        }

        let effective_pm_type = effective_paramodulation_type(bank, ocb, &source_pos, pm_type)?;
        let marked_term = paramodulation_is_simultaneous(effective_pm_type).then(|| {
            let into_term = into_pos
                .get_subterm()
                .expect("indexed target position must select a subterm");
            into_term.set_prop(TP_POTENTIAL_PARAMOD);
            into_term
        });
        let scratch_freshvars = freshvars
            .is_none()
            .then(|| fresh_var_bank_for_clauses(bank, source_entry.clause(), into_clause));
        let freshvars = freshvars.unwrap_or_else(|| {
            scratch_freshvars
                .as_ref()
                .expect("missing indexed paramodulation scratch variable bank")
        });
        freshvars.reset_v_counts();
        let paramodulant = indexed_paramod_construct_with_subst(
            bank,
            ocb,
            &source_pos,
            into_pos,
            source_entry.clause(),
            into_clause,
            freshvars,
            subst,
            effective_pm_type,
        );
        if let Some(term) = marked_term {
            term.del_prop(TP_POTENTIAL_PARAMOD);
        }
        let paramodulant = paramodulant?;
        let Some(mut paramodulant) = paramodulant else {
            continue;
        };

        paramod_count += 1;
        update_paramodulant_info(&mut paramodulant, source_entry.clause(), parent_alias);
        document_paramodulant_creation(
            doc_context,
            bank,
            &mut paramodulant,
            effective_pm_type,
            parent_alias,
            source_entry.clause(),
        )?;
        clause_push_derivation(
            &mut paramodulant,
            paramodulation_derivation_code_with_ho(effective_pm_type, subst_is_ho),
            Some(parent_alias),
            Some(source_entry.clause()),
        );
        store.insert(paramodulant);
    }
    Ok(paramod_count)
}

fn indexed_source_allows_under_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
) -> Result<bool, Diagnostic> {
    let from_clause = from_pos
        .clause()
        .expect("indexed source position must be backed by a clause");
    let from_index = from_pos
        .literal_index()
        .expect("indexed source position must select a clause literal");
    let from_literal = from_pos
        .literal()
        .expect("indexed source position must select a literal");
    let from_term = from_pos
        .get_side()
        .expect("indexed source position must select a side");
    let from_other = from_pos
        .get_other_side()
        .expect("indexed source position must select an opposite side");

    if !from_literal.is_oriented()
        && to_greater_with_bank(
            ocb,
            bank,
            &from_other,
            &from_term,
            DerefType::Always,
            DerefType::Always,
        )?
    {
        return Ok(false);
    }

    eqn_is_strictly_maximal_under_subst(ocb, bank, from_clause, from_index)
}

fn indexed_target_allows_under_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    into_pos: &ClausePos,
    into_clause: &Clause,
) -> Result<bool, Diagnostic> {
    let into_index = into_pos
        .literal_index()
        .expect("indexed target position must select a clause literal");
    let into_literal = into_pos
        .literal()
        .expect("indexed target position must select a literal");
    let into_side = into_pos
        .get_side()
        .expect("indexed target position must select a side");
    let into_other = into_pos
        .get_other_side()
        .expect("indexed target position must select an opposite side");

    if !into_literal.is_oriented()
        && to_greater_with_bank(
            ocb,
            bank,
            &into_other,
            &into_side,
            DerefType::Always,
            DerefType::Always,
        )?
    {
        return Ok(false);
    }

    if into_literal.is_positive() {
        eqn_is_strictly_maximal_under_subst(ocb, bank, into_clause, into_index)
    } else if into_literal.is_negative() {
        eqn_is_maximal_under_subst(ocb, bank, into_clause, into_index)
    } else {
        Ok(false)
    }
}

fn ensure_indexed_paramodulation_ordering_supported(
    ocb: &OrderControlBlock,
    from_pos: &ClausePos,
    into_pos: &ClausePos,
    pm_type: ParamodulationType,
) -> Result<(), Diagnostic> {
    let from_term = from_pos
        .get_side()
        .expect("indexed source position must select a side");
    let from_other = from_pos
        .get_other_side()
        .expect("indexed source position must select an opposite side");
    let into_subterm = into_pos
        .get_subterm()
        .expect("indexed target position must select a subterm");
    let into_side = into_pos
        .get_side()
        .expect("indexed target position must select a side");
    let into_other = into_pos
        .get_other_side()
        .expect("indexed target position must select an opposite side");

    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[
            &from_term,
            &from_other,
            &into_subterm,
            &into_side,
            &into_other,
        ],
        || higher_order_paramod_diagnostic_for_type(pm_type),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible active-substitution dispatcher keeps source and target positions explicit"
)]
fn indexed_paramod_construct_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
    from_clause: &Clause,
    into_clause: &Clause,
    freshvars: &VarBank,
    subst: &mut Substitution,
    pm_type: ParamodulationType,
) -> Result<Option<Clause>, Diagnostic> {
    match pm_type {
        ParamodulationType::Plain => indexed_plain_paramod_construct_with_subst(
            bank,
            from,
            into,
            from_clause,
            into_clause,
            freshvars,
            subst,
        ),
        ParamodulationType::Simultaneous => indexed_sim_paramod_construct_with_subst(
            bank,
            ocb,
            from,
            into,
            from_clause,
            into_clause,
            freshvars,
            subst,
            SimParamodReplacement::SharedTarget,
        ),
        ParamodulationType::SuperSimultaneous => indexed_sim_paramod_construct_with_subst(
            bank,
            ocb,
            from,
            into,
            from_clause,
            into_clause,
            freshvars,
            subst,
            SimParamodReplacement::InstantiatedTargetCopy,
        ),
        ParamodulationType::OrientedSimultaneous
        | ParamodulationType::OrientedSuperSimultaneous
        | ParamodulationType::DecreasingSimultaneous
        | ParamodulationType::SizeDecreasingSimultaneous => {
            unreachable!("effective paramodulation type must be concrete")
        }
    }
}

fn indexed_plain_paramod_construct_with_subst(
    bank: &mut TermBank,
    from: &ClausePos,
    into: &ClausePos,
    from_clause: &Clause,
    into_clause: &Clause,
    freshvars: &VarBank,
    subst: &mut Substitution,
) -> Result<Option<Clause>, Diagnostic> {
    let into_index = into
        .literal_index()
        .expect("indexed target position must select a clause literal");
    let from_index = from
        .literal_index()
        .expect("indexed source position must select a clause literal");
    let into_literal = into
        .literal()
        .expect("indexed target position must select a literal");

    let backtrack = subst.len();
    let result = (|| {
        let from_rhs = from
            .get_other_side()
            .expect("indexed source position must select an opposite side");
        let into_lhs = into
            .get_side()
            .expect("indexed target position must select a side");
        let into_rhs = into
            .get_other_side()
            .expect("indexed target position must select an opposite side");
        let into_subterm = into
            .get_subterm()
            .expect("indexed target position must select a subterm");

        // C ComputeOverlap normalizes these terms before constructing the
        // critical pair; the order determines shared variable-cell identity.
        subst.norm_term(&into_lhs, freshvars);
        subst.norm_term(&from_rhs, freshvars);
        let new_lhs = tb_term_pos_replace(
            bank,
            &from_rhs,
            into.term_pos(),
            DerefType::Always,
            0,
            Some(&into_subterm),
        )?;
        subst.norm_term(&into_rhs, freshvars);
        let new_rhs = bank.insert(&into_rhs, DerefType::Always)?;

        if into_literal.is_positive() && new_lhs == new_rhs {
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
        if into_copy.find_true(bank).is_some() {
            return Ok(None);
        }
        let from_copy = from_clause
            .literals()
            .copy_opt_except_index(Some(from_index), bank)?;
        if from_copy.find_true(bank).is_some() {
            return Ok(None);
        }

        into_copy.append(from_copy);
        let pm_lit = Eqn::alloc(new_lhs, new_rhs, bank, into_literal.is_positive())?;
        let mut new_literals = EqnList::new();
        new_literals.push(pm_lit);
        new_literals.append(into_copy);
        new_literals.lambda_normalize(bank)?;
        new_literals.remove_resolved(bank);
        new_literals.remove_duplicates(bank);
        Ok(Some(Clause::alloc(new_literals)))
    })();
    subst.backtrack_to_pos(backtrack);
    result
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible active-substitution constructor keeps source and target positions explicit"
)]
fn indexed_sim_paramod_construct_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
    from_clause: &Clause,
    into_clause: &Clause,
    freshvars: &VarBank,
    subst: &mut Substitution,
    replacement: SimParamodReplacement,
) -> Result<Option<Clause>, Diagnostic> {
    let from_index = from
        .literal_index()
        .expect("indexed source position must select a clause literal");
    let into_index = into
        .literal_index()
        .expect("indexed target position must select a clause literal");
    let from_literal = from
        .literal()
        .expect("indexed source position must select a literal");
    let into_literal = into
        .literal()
        .expect("indexed target position must select a literal");
    let from_term = from
        .get_side()
        .expect("indexed source position must select a side");
    let from_other = from
        .get_other_side()
        .expect("indexed source position must select an opposite side");
    let into_term = into
        .get_subterm()
        .expect("indexed target position must select a subterm");
    let into_side = into
        .get_side()
        .expect("indexed target position must select a side");
    let into_other = into
        .get_other_side()
        .expect("indexed target position must select an opposite side");

    clause_ordered_sim_paramod_active_subst(
        bank,
        ocb,
        from_clause,
        into_clause,
        from_index,
        into_index,
        from_literal,
        into_literal,
        &from_term,
        &from_other,
        &into_term,
        &into_side,
        &into_other,
        freshvars,
        subst,
        replacement,
    )
}

const fn paramodulation_is_simultaneous(pm_type: ParamodulationType) -> bool {
    matches!(
        pm_type,
        ParamodulationType::Simultaneous | ParamodulationType::SuperSimultaneous
    )
}

const fn paramodulation_type_requests_simultaneous(pm_type: ParamodulationType) -> bool {
    matches!(
        pm_type,
        ParamodulationType::Simultaneous
            | ParamodulationType::OrientedSimultaneous
            | ParamodulationType::SuperSimultaneous
            | ParamodulationType::OrientedSuperSimultaneous
            | ParamodulationType::DecreasingSimultaneous
            | ParamodulationType::SizeDecreasingSimultaneous
    )
}

fn unifiable_occurrences<'index>(
    index: &'index OverlapIndex,
    term: &Term,
    signature: &Signature,
) -> Vec<&'index SubtermOcc> {
    let mut occurrences = Vec::new();
    let _ = index.find_unifiable_occurrences(term, signature, &mut occurrences);
    occurrences
}

fn ensure_higher_order_paramodulation_ordering_supported(
    ocb: &OrderControlBlock,
    terms: &[&Term],
    diagnostic: impl Fn() -> Diagnostic,
) -> Result<(), Diagnostic> {
    if problem_type() != ProblemType::HigherOrder {
        return Ok(());
    }
    if !matches!(
        ocb.ordering_type,
        TermOrdering::Kbo
            | TermOrdering::Kbo6
            | TermOrdering::Lpo
            | TermOrdering::LpoCopy
            | TermOrdering::Lpo4
            | TermOrdering::Lpo4Copy
    ) && terms
        .iter()
        .any(|term| term_has_higher_order_unification_surface(term))
    {
        return Err(diagnostic());
    }
    Ok(())
}

fn higher_order_paramod_diagnostic_for_type(pm_type: ParamodulationType) -> Diagnostic {
    if paramodulation_type_requests_simultaneous(pm_type) {
        higher_order_sim_paramod_diagnostic()
    } else {
        higher_order_paramod_diagnostic()
    }
}

fn higher_order_paramod_diagnostic() -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "selected term ordering does not support this higher-order paramodulation surface",
    )
}

fn higher_order_sim_paramod_diagnostic() -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "selected term ordering does not support this higher-order simultaneous paramodulation surface",
    )
}

/// Computes the first-order C `ComputeOverlap` replacement term.
///
/// On success, `subst` contains the MGU plus any fresh-variable bindings added
/// while normalizing the overlapped term and replacement side. On failure,
/// substitutions created by this helper are removed.
///
/// # Errors
///
/// Returns diagnostics when the selected ordering cannot handle the overlap's
/// higher-order surface, or when term-bank insertion fails.
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
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[&max_side, &rep_side, into, &sub_into],
        higher_order_paramod_diagnostic,
    )?;
    let oldstate = subst.len();

    let unified = match subst_mgu_complete_with_bank(bank, &max_side, &sub_into, subst) {
        Ok(unified) => unified,
        Err(error) => {
            subst.backtrack_to_pos(oldstate);
            return Err(error);
        }
    };
    if !unified {
        subst.backtrack_to_pos(oldstate);
        return Ok(None);
    }
    if !from_literal.is_oriented() {
        let blocked = match to_greater_with_bank(
            ocb,
            bank,
            &rep_side,
            &max_side,
            DerefType::Always,
            DerefType::Always,
        ) {
            Ok(blocked) => blocked,
            Err(error) => {
                subst.backtrack_to_pos(oldstate);
                return Err(error);
            }
        };
        if blocked {
            subst.backtrack_to_pos(oldstate);
            return Ok(None);
        }
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
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[&rside],
        higher_order_paramod_diagnostic,
    )?;
    let oldstate = subst.len();

    let Some(replaced_lhs) =
        compute_overlap(bank, ocb, from, &lside, into.term_pos(), subst, freshvars)?
    else {
        return Ok(None);
    };

    if !into_literal.is_oriented()
        && to_greater_with_bank(
            ocb,
            bank,
            &rside,
            &lside,
            DerefType::Always,
            DerefType::Always,
        )?
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

/// Builds the first-order C `ClauseOrderedSimParamod` result for explicit
/// positions.
///
/// This constructor checks the C `TPPotentialParamod` marker on the target
/// subterm and, on success, rewrites every copied occurrence of that marked
/// target term in the target clause.
///
/// # Errors
///
/// Returns diagnostics from unsupported higher-order ordering surfaces,
/// substitution-normalized copying, or term-bank insertion.
///
/// # Panics
///
/// Panics if either position is not backed by a clause/literal or violates the
/// C internal-caller invariants.
pub fn clause_ordered_sim_paramod(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
) -> Result<Option<Clause>, Diagnostic> {
    clause_ordered_sim_paramod_variant(bank, ocb, from, into, SimParamodReplacement::SharedTarget)
}

/// Builds the first-order C `ClauseOrderedSuperSimParamod` result for explicit
/// positions.
///
/// The super-simultaneous constructor first instantiates a copy of the target
/// clause, then replaces every copied occurrence of the instantiated target
/// term. This matches C's `TBInsert` + `EqnListCopy` + `EqnListCopyRepl` path.
///
/// # Errors
///
/// Returns diagnostics from unsupported higher-order ordering surfaces,
/// substitution-normalized copying, or term-bank insertion.
///
/// # Panics
///
/// Panics if either position is not backed by a clause/literal or violates the
/// C internal-caller invariants.
pub fn clause_ordered_super_sim_paramod(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
) -> Result<Option<Clause>, Diagnostic> {
    clause_ordered_sim_paramod_variant(
        bank,
        ocb,
        from,
        into,
        SimParamodReplacement::InstantiatedTargetCopy,
    )
}

fn clause_ordered_sim_paramod_variant(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from: &ClausePos,
    into: &ClausePos,
    replacement: SimParamodReplacement,
) -> Result<Option<Clause>, Diagnostic> {
    let from_clause = from
        .clause()
        .expect("simultaneous paramodulation source position must be backed by a clause");
    let into_clause = into
        .clause()
        .expect("simultaneous paramodulation target position must be backed by a clause");
    let from_index = from
        .literal_index()
        .expect("simultaneous paramodulation source position must select a clause literal");
    let into_index = into
        .literal_index()
        .expect("simultaneous paramodulation target position must select a clause literal");
    let from_literal = from
        .literal()
        .expect("simultaneous paramodulation source position must select a literal");
    let into_literal = into
        .literal()
        .expect("simultaneous paramodulation target position must select a literal");

    assert!(
        from_literal.is_maximal(),
        "simultaneous paramodulation source literal must be maximal"
    );
    assert!(
        !from_literal.is_oriented() || from.side() == EqnSide::LeftSide,
        "oriented simultaneous paramodulation source can only use its left side"
    );

    let into_term = into
        .get_subterm()
        .expect("simultaneous paramodulation target position must select a subterm");
    if !into_term.query_prop(TP_POTENTIAL_PARAMOD) {
        return Ok(None);
    }

    let from_term = from
        .get_side()
        .expect("simultaneous paramodulation source position must select a side");
    let from_other = from
        .get_other_side()
        .expect("simultaneous paramodulation source position must select an opposite side");
    let into_side = into
        .get_side()
        .expect("simultaneous paramodulation target position must select a side");
    let into_other = into
        .get_other_side()
        .expect("simultaneous paramodulation target position must select an opposite side");
    ensure_higher_order_paramodulation_ordering_supported(
        ocb,
        &[&from_term, &from_other, &into_term, &into_side, &into_other],
        higher_order_sim_paramod_diagnostic,
    )?;

    let freshvars = fresh_var_bank_for_clauses(bank, from_clause, into_clause);
    let mut subst = Substitution::new();
    let result = clause_ordered_sim_paramod_with_subst(
        bank,
        ocb,
        from_clause,
        into_clause,
        from_index,
        into_index,
        from_literal,
        into_literal,
        &from_term,
        &from_other,
        &into_term,
        &into_side,
        &into_other,
        &freshvars,
        &mut subst,
        replacement,
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
        && eqn_is_strictly_maximal_under_subst(ocb, bank, into_clause, into_index)?)
        || into_literal.is_negative();
    if !into_is_eligible
        || !eqn_is_strictly_maximal_under_subst(ocb, bank, from_clause, from_index)?
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

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps clause-position state explicit"
)]
fn clause_ordered_sim_paramod_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_clause: &Clause,
    into_clause: &Clause,
    from_index: usize,
    into_index: usize,
    from_literal: &Eqn,
    into_literal: &Eqn,
    from_term: &Term,
    from_other: &Term,
    into_term: &Term,
    into_side: &Term,
    into_other: &Term,
    freshvars: &VarBank,
    subst: &mut Substitution,
    replacement: SimParamodReplacement,
) -> Result<Option<Clause>, Diagnostic> {
    let oldstate = subst.len();
    let unified = match subst_mgu_complete_with_bank(bank, from_term, into_term, subst) {
        Ok(unified) => unified,
        Err(error) => {
            subst.backtrack_to_pos(oldstate);
            return Err(error);
        }
    };
    if !unified {
        subst.backtrack_to_pos(oldstate);
        into_term.del_prop(TP_POTENTIAL_PARAMOD);
        return Ok(None);
    }

    clause_ordered_sim_paramod_active_subst(
        bank,
        ocb,
        from_clause,
        into_clause,
        from_index,
        into_index,
        from_literal,
        into_literal,
        from_term,
        from_other,
        into_term,
        into_side,
        into_other,
        freshvars,
        subst,
        replacement,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps active substitution and clause-position state explicit"
)]
fn clause_ordered_sim_paramod_active_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_clause: &Clause,
    into_clause: &Clause,
    from_index: usize,
    into_index: usize,
    from_literal: &Eqn,
    into_literal: &Eqn,
    from_term: &Term,
    from_other: &Term,
    into_term: &Term,
    into_side: &Term,
    into_other: &Term,
    freshvars: &VarBank,
    subst: &mut Substitution,
    replacement: SimParamodReplacement,
) -> Result<Option<Clause>, Diagnostic> {
    if !from_literal.is_oriented()
        && to_greater_with_bank(
            ocb,
            bank,
            from_other,
            from_term,
            DerefType::Always,
            DerefType::Always,
        )?
    {
        into_term.del_prop(TP_POTENTIAL_PARAMOD);
        return Ok(None);
    }

    if !into_literal.is_oriented()
        && to_greater_with_bank(
            ocb,
            bank,
            into_other,
            into_side,
            DerefType::Always,
            DerefType::Always,
        )?
    {
        return Ok(None);
    }

    if !eqn_is_strictly_maximal_under_subst(ocb, bank, from_clause, from_index)? {
        into_term.del_prop(TP_POTENTIAL_PARAMOD);
        return Ok(None);
    }

    let into_is_eligible = (into_literal.is_positive()
        && eqn_is_strictly_maximal_under_subst(ocb, bank, into_clause, into_index)?)
        || (into_literal.is_negative()
            && eqn_is_maximal_under_subst(ocb, bank, into_clause, into_index)?);
    if !into_is_eligible {
        return Ok(None);
    }

    into_term.del_prop(TP_POTENTIAL_PARAMOD);

    let backtrack = subst.len();
    let result = (|| {
        let _ = into_clause
            .literals()
            .subst_norm_except(None, subst, freshvars);
        let _ = from_clause
            .literals()
            .subst_norm_except(None, subst, freshvars);

        let mut into_deref = DerefType::Always;
        let into_term_instance = term_deref(into_term, &mut into_deref);
        let mut rhs_deref = DerefType::Always;
        let from_other_instance = term_deref(from_other, &mut rhs_deref);
        let rewritten_rhs =
            make_rewritten_term(bank, &into_term_instance, &from_other_instance, 0)?;
        let rhs_instance = bank.insert_no_props(&rewritten_rhs, DerefType::Always)?;
        let mut into_copy = match replacement {
            SimParamodReplacement::SharedTarget => {
                into_clause
                    .literals()
                    .copy_repl(bank, into_term, &rhs_instance)?
            }
            SimParamodReplacement::InstantiatedTargetCopy => {
                let lhs_instance = bank.insert(into_term, DerefType::Always)?;
                let tmp_copy = into_clause.literals().copy_to_bank(bank)?;
                tmp_copy.copy_repl(bank, &lhs_instance, &rhs_instance)?
            }
        };
        if into_copy.find_true(bank).is_some() {
            return Ok(None);
        }

        let mut from_copy = from_clause
            .literals()
            .copy_opt_except_index(Some(from_index), bank)?;
        if from_copy.find_true(bank).is_some() {
            return Ok(None);
        }

        into_copy.del_prop(EP_FROM_CLAUSE_LIT);
        from_copy.set_prop(EP_FROM_CLAUSE_LIT);
        into_copy.append(from_copy);
        into_copy.remove_resolved(bank);
        into_copy.remove_duplicates(bank);
        Ok(Some(Clause::alloc(into_copy)))
    })();
    subst.backtrack_to_pos(backtrack);
    result
}

fn eqn_is_strictly_maximal_under_subst(
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
        if matches!(
            candidate.literal_compare_with_bank(ocb, bank, target)?,
            CompareResult::Greater | CompareResult::Equal
        ) {
            return Ok(false);
        }
    }
    Ok(true)
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

fn fresh_var_bank_for_clauses(bank: &TermBank, first: &Clause, second: &Clause) -> VarBank {
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = first.collect_variables(&mut variables);
    let _ = second.collect_variables(&mut variables);
    let freshvars = VarBank::fresh_normalization_bank(
        bank.signature().type_bank(),
        bank.vars(),
        variables.values(),
    );
    freshvars.reset_v_counts();
    freshvars
}

fn from_side_allows_paramod(bank: &TermBank, position: &ClausePos) -> bool {
    let literal = position
        .literal()
        .expect("source-side position must select a literal");
    (PARAMOD_OVERLAP_NON_EQ_LITERALS || literal.is_equ_lit(bank)) && !literal.is_selected()
}

fn first_paramod_into_candidate(
    bank: &TermBank,
    position: &mut ClausePos,
    from_pos: &ClausePos,
    no_top: bool,
) -> Option<Term> {
    let mut current = if from_uses_full_subterm_iteration(bank, from_pos) {
        position.find_first_maximal_subterm()
    } else {
        find_first_negative_maximal_left_side(position)
    };
    while let Some(term) = current {
        if !is_no_paramod_position(bank, position, from_pos, &term, no_top) {
            return Some(term);
        }
        current = next_paramod_into_candidate_raw(bank, position, from_pos);
    }
    None
}

fn next_paramod_into_candidate(
    bank: &TermBank,
    position: &mut ClausePos,
    from_pos: &ClausePos,
    no_top: bool,
) -> Option<Term> {
    let mut current = next_paramod_into_candidate_raw(bank, position, from_pos);
    while let Some(term) = current {
        if !is_no_paramod_position(bank, position, from_pos, &term, no_top) {
            return Some(term);
        }
        current = next_paramod_into_candidate_raw(bank, position, from_pos);
    }
    None
}

fn next_paramod_into_candidate_raw(
    bank: &TermBank,
    position: &mut ClausePos,
    from_pos: &ClausePos,
) -> Option<Term> {
    if from_uses_full_subterm_iteration(bank, from_pos) {
        position.find_next_maximal_subterm()
    } else {
        advance_position_to_next_literal(position);
        find_first_negative_maximal_left_side(position)
    }
}

fn from_uses_full_subterm_iteration(bank: &TermBank, from_pos: &ClausePos) -> bool {
    from_pos
        .literal()
        .expect("source-side position must select a literal")
        .is_equ_lit(bank)
        || problem_type() == ProblemType::HigherOrder
}

fn is_no_paramod_position(
    bank: &TermBank,
    position: &ClausePos,
    from_pos: &ClausePos,
    term: &Term,
    no_top: bool,
) -> bool {
    let target_literal = position
        .literal()
        .expect("target position must select a literal");
    let source_side = from_pos
        .get_side()
        .expect("source position must select a side");

    term.is_free_var()
        || (target_literal.is_positive() && no_top && position.is_top())
        || (source_side.is_free_var()
            && problem_type() == ProblemType::FirstOrder
            && !target_literal.is_equ_lit(bank)
            && position.is_top())
}

fn find_first_negative_maximal_left_side(position: &mut ClausePos) -> Option<Term> {
    let found = {
        let clause = position.clause()?;
        let start = position.literal_index()?;
        (start..clause.literals().len()).find(|&index| {
            let literal = &clause.literals().as_slice()[index];
            literal.is_maximal() && literal.is_negative()
        })
    };

    position.set_literal_index(found);
    if found.is_some() {
        position.set_side(EqnSide::LeftSide);
        position.term_pos_mut().clear();
        position.get_side()
    } else {
        None
    }
}

fn advance_position_to_next_literal(position: &mut ClausePos) {
    let next = {
        let Some(clause) = position.clause() else {
            position.set_literal_index(None);
            return;
        };
        position.literal_index().and_then(|index| {
            let next = index.saturating_add(1);
            (next < clause.literals().len()).then_some(next)
        })
    };
    position.set_literal_index(next);
}

fn mark_potential_paramod_terms_from_position(position: &ClausePos) {
    let Some(clause) = position.clause() else {
        return;
    };
    let Some(start) = position.literal_index() else {
        return;
    };
    for literal in &clause.literals().as_slice()[start..] {
        literal.term_set_prop(TP_POTENTIAL_PARAMOD);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible helper keeps source, target, and metadata parents explicit"
)]
fn compute_directed_clause_paramodulants(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    source: &Clause,
    source_parent: &Clause,
    target: &Clause,
    store: &mut ClauseSet,
    no_top: bool,
    metadata_parent1: &Clause,
    metadata_parent2: &Clause,
    pm_type: ParamodulationType,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut paramod_count = 0;
    for pair in paramodulation_pair_positions(bank, source, target, no_top, pm_type) {
        if paramodulation_time_is_up_before_next_insert(store) {
            break;
        }
        let effective_pm_type = effective_paramodulation_type(bank, ocb, pair.source(), pm_type)?;
        let paramodulant = match effective_pm_type {
            ParamodulationType::Plain => {
                clause_ordered_paramod(bank, ocb, pair.source(), pair.target())?
            }
            ParamodulationType::Simultaneous => {
                clause_ordered_sim_paramod(bank, ocb, pair.source(), pair.target())?
            }
            ParamodulationType::SuperSimultaneous => {
                clause_ordered_super_sim_paramod(bank, ocb, pair.source(), pair.target())?
            }
            ParamodulationType::OrientedSimultaneous
            | ParamodulationType::OrientedSuperSimultaneous
            | ParamodulationType::DecreasingSimultaneous
            | ParamodulationType::SizeDecreasingSimultaneous => {
                unreachable!("per-source paramodulation mode must reduce to plain or simultaneous");
            }
        };
        let Some(mut paramodulant) = paramodulant else {
            continue;
        };
        paramod_count += 1;
        update_paramodulant_info(&mut paramodulant, metadata_parent1, metadata_parent2);
        document_paramodulant_creation(
            doc_context,
            bank,
            &mut paramodulant,
            effective_pm_type,
            metadata_parent2,
            source_parent,
        )?;
        // C ComputeClauseClauseParamodulants pushes this derivation onto its
        // temporary selected-clause copy, which is freed after generation.
        // Leaving the child without that entry is observable in orphan filtering.
        store.insert(paramodulant);
    }
    Ok(paramod_count)
}

fn document_paramodulant_creation(
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
    bank: &TermBank,
    paramodulant: &mut Clause,
    pm_type: ParamodulationType,
    parent1: &Clause,
    parent2: &Clause,
) -> Result<(), Diagnostic> {
    if let Some((output, session)) = doc_context.as_mut() {
        session.doc_clause_creation(
            &mut **output,
            bank,
            paramodulant,
            paramodulation_creation_inference(pm_type),
            ClauseCreationParents::binary(parent1, parent2),
            None,
        )?;
    }
    Ok(())
}

const fn paramodulation_creation_inference(pm_type: ParamodulationType) -> ClauseCreationInference {
    if matches!(pm_type, ParamodulationType::Plain) {
        ClauseCreationInference::Paramodulation
    } else {
        ClauseCreationInference::SimultaneousParamodulation
    }
}

fn update_paramodulant_info(child: &mut Clause, parent1: &Clause, parent2: &Clause) {
    child.set_proof_size(
        parent1
            .proof_size()
            .saturating_add(parent2.proof_size())
            .saturating_add(1),
    );
    child.set_proof_depth(
        parent1
            .proof_depth()
            .max(parent2.proof_depth())
            .saturating_add(1),
    );
    child.set_tptp_type(parent1.query_tptp_type());
    child.set_prop(parent1.give_props(CP_IS_SOS) | parent2.give_props(CP_IS_SOS));
    if !std::ptr::eq(parent1, parent2) {
        child.set_tptp_type(tptp_types_combine(
            child.query_tptp_type(),
            parent2.query_tptp_type(),
        ));
    }
}

const fn paramodulation_derivation_code(pm_type: ParamodulationType) -> i64 {
    if matches!(pm_type, ParamodulationType::Plain) {
        DC_PARAMOD
    } else {
        DC_SIM_PARAMOD
    }
}

const fn paramodulation_derivation_code_with_ho(
    pm_type: ParamodulationType,
    subst_is_ho: bool,
) -> i64 {
    let code = paramodulation_derivation_code(pm_type);
    if subst_is_ho {
        set_is_ho(code)
    } else {
        code
    }
}

fn effective_paramodulation_type(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    from_pos: &ClausePos,
    pm_type: ParamodulationType,
) -> Result<ParamodulationType, Diagnostic> {
    match pm_type {
        ParamodulationType::Plain => Ok(ParamodulationType::Plain),
        ParamodulationType::Simultaneous => Ok(ParamodulationType::Simultaneous),
        ParamodulationType::OrientedSimultaneous => {
            let literal = from_pos
                .literal()
                .expect("paramodulation source position must select a literal");
            if literal.is_oriented() {
                Ok(ParamodulationType::Simultaneous)
            } else {
                Ok(ParamodulationType::Plain)
            }
        }
        ParamodulationType::DecreasingSimultaneous => {
            let max_side = from_pos
                .get_side()
                .expect("paramodulation source position must select a side");
            let rep_side = from_pos
                .get_other_side()
                .expect("paramodulation source position must select an opposite side");
            if to_greater_with_bank(
                ocb,
                bank,
                &max_side,
                &rep_side,
                DerefType::Always,
                DerefType::Always,
            )? {
                Ok(ParamodulationType::Simultaneous)
            } else {
                Ok(ParamodulationType::Plain)
            }
        }
        ParamodulationType::SizeDecreasingSimultaneous => {
            let max_side = from_pos
                .get_side()
                .expect("paramodulation source position must select a side");
            let rep_side = from_pos
                .get_other_side()
                .expect("paramodulation source position must select an opposite side");
            if term_standard_weight(&max_side) > term_standard_weight(&rep_side) {
                Ok(ParamodulationType::Simultaneous)
            } else {
                Ok(ParamodulationType::Plain)
            }
        }
        ParamodulationType::SuperSimultaneous => Ok(ParamodulationType::SuperSimultaneous),
        ParamodulationType::OrientedSuperSimultaneous => {
            let literal = from_pos
                .literal()
                .expect("paramodulation source position must select a literal");
            if literal.is_oriented() {
                Ok(ParamodulationType::SuperSimultaneous)
            } else {
                Ok(ParamodulationType::Plain)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clause_ordered_paramod, clause_ordered_sim_paramod, clause_ordered_super_sim_paramod,
        compute_all_paramodulants, compute_all_paramodulants_indexed,
        compute_all_paramodulants_indexed_with_fresh_vars, compute_all_paramodulants_with_docs,
        compute_clause_clause_paramodulants, effective_paramodulation_type,
        fresh_var_bank_for_clauses, indexed_plain_paramod_construct_with_subst,
        paramod_from_side_positions, paramod_into_positions, paramodulation_pair_positions,
        ParamodulationType,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_IS_SOS, CP_NO_GENERATION, CP_TYPE_NEG_CONJECTURE};
    use crate::clauses::clausepos::ClausePos;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        set_is_ho, ClauseDerivationRef, DerivationEntry, DC_PARAMOD, DC_SIM_PARAMOD,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, EP_FROM_CLAUSE_LIT, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_PM_INTO_LIT,
        EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::global_indices::GlobalIndices;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::heuristics::hcb::{HeuristicParmsCell, UnifMode};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::ho_csu::init_unif_limits;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::match_mgu::subst_mgu_complete_with_bank;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::subst::Substitution;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_POTENTIAL_PARAMOD};
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

    fn typed_binary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
                )
                .unwrap();
        }
        f_code
    }

    fn typed_binary(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    #[test]
    fn paramodulation_fresh_variables_restart_at_canonical_codes() {
        let mut bank = test_bank();
        let canonical = typed_var(&bank, -2);
        let source_var = typed_var(&bank, -40);
        let target_var = typed_var(&bank, -42);
        let a = typed_const(&mut bank, "pm_canonical_a");
        let source = Clause::alloc(EqnList::from_vec(vec![lit(
            &mut bank,
            &source_var,
            &a,
            true,
        )]));
        let target = Clause::alloc(EqnList::from_vec(vec![lit(
            &mut bank,
            &target_var,
            &a,
            true,
        )]));

        let freshvars = fresh_var_bank_for_clauses(&bank, &source, &target);
        let normalized = freshvars.get_fresh_var(
            &canonical
                .type_()
                .expect("canonical variable must retain its type"),
        );

        assert_eq!(normalized.f_code(), canonical.f_code());
    }

    #[test]
    fn indexed_plain_paramodulation_preserves_c_variable_normalization_order() {
        let mut bank = test_bank();
        for f_code in [-2, -4, -6, -8] {
            let _ = typed_var(&bank, f_code);
        }
        let source_x = typed_var(&bank, -20);
        let source_y = typed_var(&bank, -22);
        let target_x = typed_var(&bank, -24);
        let target_y = typed_var(&bank, -26);
        let overlap = typed_const(&mut bank, "pm_norm_overlap");
        let pair_code = typed_binary_code(&mut bank, "pm_norm_pair");
        let target_code = typed_binary_code(&mut bank, "pm_norm_target");
        let replacement = typed_binary(&mut bank, pair_code, &source_x, &source_y);
        let target_left = typed_binary(&mut bank, target_code, &overlap, &target_x);
        let mut source_literal = lit(&mut bank, &overlap, &replacement, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_y, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let source_pos = top_left_position(&source);
        let mut target_pos = top_left_position(&target);
        target_pos.term_pos_mut().push_component(target_left, 0);
        let freshvars = fresh_var_bank_for_clauses(&bank, &source, &target);
        let mut subst = Substitution::new();

        let generated = indexed_plain_paramod_construct_with_subst(
            &mut bank,
            &source_pos,
            &target_pos,
            &source,
            &target,
            &freshvars,
            &mut subst,
        )
        .unwrap()
        .expect("ground overlap should generate a critical pair");

        let literal = &generated.literals().as_slice()[0];
        let left_args = literal.left().argument_clones();
        let replacement_args = left_args[0]
            .as_ref()
            .expect("replacement argument must exist")
            .argument_clones();
        assert_eq!(replacement_args[0].as_ref().map(Term::f_code), Some(-4));
        assert_eq!(replacement_args[1].as_ref().map(Term::f_code), Some(-6));
        assert_eq!(left_args[1].as_ref().map(Term::f_code), Some(-2));
        assert_eq!(literal.right().f_code(), -8);
        assert!(subst.is_empty());
    }

    fn unary_predicate_code(bank: &mut TermBank, name: &str) -> i64 {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_type(f_code, alloc_arrow_type(vec![arg_type, bool_type]))
                .unwrap();
        }
        f_code
    }

    fn unary_predicate(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unary_predicate_var(bank: &mut TermBank, f_code: i64) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let type_ = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type]));
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn lit(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn maximal_oriented(literal: &mut Eqn) {
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
    }

    fn maximal(literal: &mut Eqn) {
        literal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
    }

    fn top_left_position(clause: &Clause) -> ClausePos {
        let mut position = ClausePos::for_clause(clause.clone());
        assert!(position.set_literal_index(Some(0)));
        position.set_side(EqnSide::LeftSide);
        position
    }

    fn eta_expanded_arrow_const(bank: &mut TermBank, head: &Term) -> Term {
        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = apply_terms(bank, head, std::slice::from_ref(&db0)).unwrap();
        close_with_type_prefix(bank, std::slice::from_ref(&i_type), &matrix).unwrap()
    }

    #[test]
    fn paramod_from_side_positions_follow_c_side_order_and_skip_selected() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "pm_from_left");
        let right = typed_const(&mut bank, "pm_from_right");
        let extra_left = typed_const(&mut bank, "pm_from_extra_left");
        let extra_right = typed_const(&mut bank, "pm_from_extra_right");
        let selected_left = typed_const(&mut bank, "pm_from_selected_left");
        let selected_right = typed_const(&mut bank, "pm_from_selected_right");
        let mut selected = lit(&mut bank, &selected_left, &selected_right, true);
        let mut unoriented = lit(&mut bank, &left, &right, true);
        let mut oriented = lit(&mut bank, &extra_left, &extra_right, true);
        selected.set_prop(EP_IS_MAXIMAL | EP_IS_SELECTED);
        maximal(&mut unoriented);
        maximal_oriented(&mut oriented);
        let clause = Clause::alloc(EqnList::from_vec(vec![selected, unoriented, oriented]));

        let positions = paramod_from_side_positions(&bank, &clause);

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].literal_index(), Some(1));
        assert_eq!(positions[0].side(), EqnSide::LeftSide);
        assert_eq!(positions[1].literal_index(), Some(1));
        assert_eq!(positions[1].side(), EqnSide::RightSide);
        assert_eq!(positions[2].literal_index(), Some(2));
        assert_eq!(positions[2].side(), EqnSide::LeftSide);
    }

    #[test]
    fn effective_paramodulation_type_decreasing_uses_banked_lambda_ordering() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let f = typed_arrow_const(&mut bank, "pm_effective_eta_f");
        let eta_f = eta_expanded_arrow_const(&mut bank, &f);
        let mut source = lit(&mut bank, &eta_f, &f, true);
        maximal(&mut source);
        let clause = Clause::alloc(EqnList::from_vec(vec![source]));
        let from_pos = top_left_position(&clause);
        let mut ocb = kbo6_lambda_ocb(&bank);

        assert_eq!(
            effective_paramodulation_type(
                &mut bank,
                &mut ocb,
                &from_pos,
                ParamodulationType::DecreasingSimultaneous,
            )
            .unwrap(),
            ParamodulationType::Plain
        );
    }

    #[test]
    fn paramod_into_positions_skip_positive_roots_when_no_top_is_set() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_into_source_left");
        let source_right = typed_const(&mut bank, "pm_into_source_right");
        let target_arg = typed_const(&mut bank, "pm_into_target_arg");
        let target_rhs = typed_const(&mut bank, "pm_into_target_rhs");
        let neg_arg = typed_const(&mut bank, "pm_into_neg_arg");
        let neg_rhs = typed_const(&mut bank, "pm_into_neg_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_into_f");
        let g_code = typed_unary_code(&mut bank, "pm_into_g");
        let f_of_target = typed_unary(&mut bank, f_code, &target_arg);
        let g_of_negative = typed_unary(&mut bank, g_code, &neg_arg);
        let mut from_lit = lit(&mut bank, &source_left, &source_right, true);
        let mut positive_target = lit(&mut bank, &f_of_target, &target_rhs, true);
        let mut negative_target = lit(&mut bank, &g_of_negative, &neg_rhs, false);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut positive_target);
        maximal_oriented(&mut negative_target);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![positive_target, negative_target]));
        let from_pos = top_left_position(&from_clause);

        let positions = paramod_into_positions(
            &bank,
            &into_clause,
            &from_pos,
            true,
            ParamodulationType::Plain,
        );

        assert_eq!(positions.len(), 3);
        assert_eq!(positions[0].literal_index(), Some(0));
        assert_eq!(positions[0].side(), EqnSide::LeftSide);
        assert!(!positions[0].is_top());
        assert_eq!(positions[1].literal_index(), Some(1));
        assert_eq!(positions[1].side(), EqnSide::LeftSide);
        assert!(!positions[1].is_top());
        assert_eq!(positions[2].literal_index(), Some(1));
        assert_eq!(positions[2].side(), EqnSide::LeftSide);
        assert!(positions[2].is_top());
    }

    #[test]
    fn paramodulation_pair_positions_nest_into_positions_under_each_source_side() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_pair_source_left");
        let source_right = typed_const(&mut bank, "pm_pair_source_right");
        let target_arg = typed_const(&mut bank, "pm_pair_target_arg");
        let target_rhs = typed_const(&mut bank, "pm_pair_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_pair_f");
        let f_of_target = typed_unary(&mut bank, f_code, &target_arg);
        let mut from_lit = lit(&mut bank, &source_left, &source_right, true);
        let mut into_lit = lit(&mut bank, &f_of_target, &target_rhs, true);
        maximal(&mut from_lit);
        maximal_oriented(&mut into_lit);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit]));

        let pairs = paramodulation_pair_positions(
            &bank,
            &from_clause,
            &into_clause,
            true,
            ParamodulationType::Plain,
        );

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].source().side(), EqnSide::LeftSide);
        assert_eq!(pairs[0].target().literal_index(), Some(0));
        assert!(!pairs[0].target().is_top());
        assert_eq!(pairs[1].source().side(), EqnSide::RightSide);
        assert_eq!(pairs[1].target().literal_index(), Some(0));
        assert!(!pairs[1].target().is_top());
    }

    #[test]
    fn compute_clause_clause_paramodulants_inserts_plain_metadata() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_cc_source_left");
        let source_right = typed_const(&mut bank, "pm_cc_source_right");
        let target_rhs = typed_const(&mut bank, "pm_cc_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_cc_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        source.set_proof_depth(2);
        source.set_proof_size(7);
        source.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        source.set_prop(CP_IS_SOS);
        target.set_proof_depth(5);
        target.set_proof_size(11);
        target.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store.iter().next().expect("one paramodulant inserted");
        assert_eq!(stored.proof_depth(), 6);
        assert_eq!(stored.proof_size(), 19);
        assert_eq!(stored.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(stored.query_prop(CP_IS_SOS));
        assert_eq!(stored.literal_number(), 1);
        assert_eq!(stored.literals().as_slice()[0].left(), &f_of_replacement);
        assert_eq!(stored.literals().as_slice()[0].right(), &target_rhs);
        assert!(stored.derivation().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_carries_unrelated_surface_literal() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_ho_mixed_source_left");
        let source_right = typed_const(&mut bank, "pm_ho_mixed_source_right");
        let target_rhs = typed_const(&mut bank, "pm_ho_mixed_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_ho_mixed_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let predicate = unary_predicate_var(&mut bank, -2_401);
        let arg = typed_const(&mut bank, "pm_ho_mixed_arg");
        let applied = apply_terms(&mut bank, &predicate, std::slice::from_ref(&arg)).unwrap();
        let truth = bank.true_term().clone();

        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        let unrelated_literal = lit(&mut bank, &applied, &truth, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal, unrelated_literal]));
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store.iter().next().expect("one paramodulant inserted");
        assert_eq!(stored.literal_number(), 2);
        assert!(stored.literals().as_slice().iter().any(|literal| {
            literal.left() == &f_of_replacement && literal.right() == &target_rhs
        }));
        assert!(stored
            .literals()
            .as_slice()
            .iter()
            .any(|literal| { literal.left().is_applied_free_var() && literal.right() == &truth }));
    }

    fn assert_unindexed_paramodulation_preserves_copied_beta_literal(pm_type: ParamodulationType) {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_unindexed_norm_source_left");
        let source_right = typed_const(&mut bank, "pm_unindexed_norm_source_right");
        let target_rhs = typed_const(&mut bank, "pm_unindexed_norm_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_unindexed_norm_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);

        let i_type = bank.signature().type_bank().default_type();
        let db0 = bank.request_db_var(&i_type, 0);
        let identity_lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &db0).unwrap();
        let beta_arg = typed_const(&mut bank, "pm_unindexed_norm_arg");
        let beta_applied =
            apply_terms(&mut bank, &identity_lambda, std::slice::from_ref(&beta_arg)).unwrap();
        let beta_rhs = typed_const(&mut bank, "pm_unindexed_norm_beta_rhs");

        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        let copied_literal = lit(&mut bank, &beta_applied, &beta_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal, copied_literal]));
        let mut ocb = kbo6_lambda_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank, &mut ocb, &source, &source, &target, &mut store, pm_type,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store.iter().next().expect("one paramodulant inserted");
        assert!(stored
            .literals()
            .as_slice()
            .iter()
            .any(|literal| { literal.left() == &beta_applied && literal.right() == &beta_rhs }));
        assert!(!stored
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.left() == &beta_arg && literal.right() == &beta_rhs));
    }

    #[test]
    fn unindexed_plain_paramodulation_preserves_c_copied_beta_literals() {
        assert_unindexed_paramodulation_preserves_copied_beta_literal(ParamodulationType::Plain);
    }

    #[test]
    fn unindexed_sim_paramodulation_preserves_c_copied_beta_literals() {
        assert_unindexed_paramodulation_preserves_copied_beta_literal(
            ParamodulationType::Simultaneous,
        );
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_allows_first_order_shaped_binding() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let source_left = typed_arrow_var(&mut bank, -2_403);
        let source_right = typed_arrow_const(&mut bank, "pm_ho_plain_source_right");
        let target_left = typed_arrow_const(&mut bank, "pm_ho_plain_target_left");
        let target_right = typed_arrow_const(&mut bank, "pm_ho_plain_target_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store
            .iter()
            .next()
            .expect("one unindexed higher-order paramodulant");
        assert_eq!(stored.literal_number(), 1);
        let generated = &stored.literals().as_slice()[0];
        assert!(generated.is_positive());
        assert_eq!(generated.left(), &source_right);
        assert_eq!(generated.right(), &target_right);
        assert!(stored.derivation().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_simultaneous_allows_binding() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let source_left = typed_arrow_var(&mut bank, -2_404);
        let source_right = typed_arrow_const(&mut bank, "pm_ho_sim_source_right");
        let target_left = typed_arrow_const(&mut bank, "pm_ho_sim_target_left");
        let target_right = typed_arrow_const(&mut bank, "pm_ho_sim_target_right");
        let target_extra_right = typed_arrow_const(&mut bank, "pm_ho_sim_target_extra_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        let target_extra = lit(&mut bank, &target_left, &target_extra_right, false);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal, target_extra]));
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Simultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store
            .iter()
            .next()
            .expect("one unindexed higher-order simultaneous paramodulant");
        assert_eq!(stored.literal_number(), 2);
        let generated = stored.literals().as_slice();
        assert!(generated[0].is_positive());
        assert_eq!(generated[0].left(), &source_right);
        assert_eq!(generated[0].right(), &target_right);
        assert!(!generated[1].is_positive());
        assert_eq!(generated[1].left(), &source_right);
        assert_eq!(generated[1].right(), &target_extra_right);
        assert!(stored.derivation().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_unifies_actual_surface_overlap() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::Kbo6);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_lpo4_unifies_actual_surface_overlap() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::Lpo4);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_kbo_matches_release_surface() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::Kbo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_lpo_matches_release_surface() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::Lpo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_lpo_copy_matches_release_surface() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::LpoCopy);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_lpo4_copy_matches_release_surface() {
        assert_higher_order_surface_overlap_paramodulates(TermOrdering::Lpo4Copy);
    }

    fn assert_higher_order_surface_overlap_paramodulates(ordering: TermOrdering) {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let function = typed_arrow_var(&mut bank, -2_402);
        let prefix = typed_const(&mut bank, "pm_ho_surface_prefix");
        let suffix = typed_const(&mut bank, "pm_ho_surface_suffix");
        let individual = bank.signature().type_bank().default_type();
        let binary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual,
            ]));
        let rigid_code = bank
            .signature_mut()
            .insert_id("pm_ho_surface_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, binary)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let applied = apply_terms(&mut bank, &function, std::slice::from_ref(&suffix)).unwrap();
        let source_right = typed_const(&mut bank, "pm_ho_surface_source_right");
        let target_left = apply_terms(&mut bank, &rigid, &[prefix, suffix]).unwrap();
        let target_right = typed_const(&mut bank, "pm_ho_surface_target_right");
        let mut source_literal = lit(&mut bank, &applied, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb =
            OrderControlBlock::alloc(ordering, true, bank.signature(), HoOrderKind::LfhoOrder);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let generated = store.iter().next().expect("one higher-order paramodulant");
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.literals().as_slice()[0].left(), &source_right);
        assert_eq!(generated.literals().as_slice()[0].right(), &target_right);
        assert!(function.binding().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_kbo6() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::Kbo6);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_lpo4() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::Lpo4);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_kbo() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::Kbo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_lpo() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::Lpo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_lpo_copy() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::LpoCopy);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_flex_flex_reorients_for_lpo4_copy() {
        assert_higher_order_flex_flex_overlap_paramodulates(TermOrdering::Lpo4Copy);
    }

    fn assert_higher_order_flex_flex_overlap_paramodulates(ordering: TermOrdering) {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let unary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let binary = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
                individual,
            ]));
        let short_head = bank.vars().var_assert_alloc(-2_405, &unary);
        let long_head = bank.vars().var_assert_alloc(-2_406, &binary);
        let prefix = typed_const(&mut bank, "pm_ho_flex_prefix");
        let suffix = typed_const(&mut bank, "pm_ho_flex_suffix");
        let source_left =
            apply_terms(&mut bank, &short_head, std::slice::from_ref(&suffix)).unwrap();
        let target_left = apply_terms(&mut bank, &long_head, &[prefix, suffix]).unwrap();
        let source_right = typed_const(&mut bank, "pm_ho_flex_source_right");
        let target_right = typed_const(&mut bank, "pm_ho_flex_target_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb =
            OrderControlBlock::alloc(ordering, true, bank.signature(), HoOrderKind::LfhoOrder);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let generated = store
            .iter()
            .next()
            .expect("one flex-flex higher-order paramodulant");
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.literals().as_slice()[0].left(), &source_right);
        assert_eq!(generated.literals().as_slice()[0].right(), &target_right);
        assert!(short_head.binding().is_none());
        assert!(long_head.binding().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_kbo6() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::Kbo6);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_lpo4() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::Lpo4);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_kbo() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::Kbo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_lpo() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::Lpo);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_lpo_copy() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::LpoCopy);
    }

    #[test]
    fn compute_clause_clause_paramodulants_higher_order_eta_reduces_db_overlap_for_lpo4_copy() {
        assert_higher_order_eta_db_overlap_paramodulates(TermOrdering::Lpo4Copy);
    }

    fn assert_higher_order_eta_db_overlap_paramodulates(ordering: TermOrdering) {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let function = typed_arrow_var(&mut bank, -2_407);
        let rigid_head = typed_arrow_const(&mut bank, "pm_ho_eta_head");
        let eta_head = eta_expanded_arrow_const(&mut bank, &rigid_head);
        let individual = bank.signature().type_bank().default_type();
        let unary = typed_arrow_type(&mut bank);
        let wrapper_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![unary, individual.clone()]));
        let wrapper_code = bank
            .signature_mut()
            .insert_id("pm_ho_eta_wrapper", 0, false);
        bank.signature_mut()
            .declare_final_type(wrapper_code, wrapper_type)
            .unwrap();
        let wrapper = bank.create_const_term(wrapper_code).unwrap();
        let source_left =
            apply_terms(&mut bank, &wrapper, std::slice::from_ref(&function)).unwrap();
        let target_left = Term::top_alloc(wrapper_code, 1);
        target_left.set_type(Some(individual));
        target_left.set_argument(0, eta_head);
        let target_left = bank.term_top_insert(target_left).unwrap();
        assert!(target_left
            .argument(0)
            .is_some_and(|argument| argument.is_lambda()));
        let mut direct_subst = Substitution::new();
        assert!(subst_mgu_complete_with_bank(
            &mut bank,
            &source_left,
            &target_left,
            &mut direct_subst,
        )
        .unwrap());
        direct_subst.backtrack();
        let source_right = typed_const(&mut bank, "pm_ho_eta_source_right");
        let target_right = typed_const(&mut bank, "pm_ho_eta_target_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb =
            OrderControlBlock::alloc(ordering, true, bank.signature(), HoOrderKind::LfhoOrder);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let generated = store
            .iter()
            .next()
            .expect("one eta-reduced higher-order paramodulant");
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.literals().as_slice()[0].left(), &source_right);
        assert_eq!(generated.literals().as_slice()[0].right(), &target_right);
        assert!(function.binding().is_none());
    }

    #[test]
    fn compute_clause_clause_paramodulants_resolves_predicate_rule_with_ground_unit() {
        let mut bank = test_bank();
        let mut ocb = kbo_ocb(&bank);
        let socrates = typed_const(&mut bank, "pm_socrates");
        let x = typed_var(&bank, -1);
        let human_code = unary_predicate_code(&mut bank, "pm_human");
        let mortal_code = unary_predicate_code(&mut bank, "pm_mortal");
        let human_socrates = unary_predicate(&mut bank, human_code, &socrates);
        let human_x = unary_predicate(&mut bank, human_code, &x);
        let mortal_x = unary_predicate(&mut bank, mortal_code, &x);
        let mortal_socrates = unary_predicate(&mut bank, mortal_code, &socrates);
        let truth = bank.true_term().clone();

        let mut fact_lit = lit(&mut bank, &human_socrates, &truth, true);
        maximal(&mut fact_lit);
        let fact = Clause::alloc(EqnList::from_vec(vec![fact_lit]));

        let mut rule_head = lit(&mut bank, &mortal_x, &truth, true);
        let mut rule_tail = lit(&mut bank, &human_x, &truth, false);
        maximal(&mut rule_head);
        maximal(&mut rule_tail);
        let rule = Clause::alloc(EqnList::from_vec(vec![rule_head, rule_tail]));
        let mut store = ClauseSet::new();
        let from_positions = paramod_from_side_positions(&bank, &fact);
        assert_eq!(from_positions.len(), 2);
        assert_eq!(from_positions[0].literal_index(), Some(0));
        assert_eq!(from_positions[0].side(), EqnSide::LeftSide);
        assert_eq!(from_positions[1].literal_index(), Some(0));
        assert_eq!(from_positions[1].side(), EqnSide::RightSide);
        let into_positions = paramod_into_positions(
            &bank,
            &rule,
            &from_positions[0],
            false,
            ParamodulationType::Plain,
        );
        assert_eq!(into_positions.len(), 1);
        assert_eq!(into_positions[0].literal_index(), Some(1));
        assert_eq!(into_positions[0].side(), EqnSide::LeftSide);
        assert!(into_positions[0].is_top());

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &fact,
            &fact,
            &rule,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let generated = store.iter().next().expect("one paramodulant");
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &mortal_socrates);
        assert_eq!(literal.right(), &truth);
    }

    #[test]
    fn compute_clause_clause_paramodulants_honors_no_generation_gate() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_gate_source_left");
        let source_right = typed_const(&mut bank, "pm_gate_source_right");
        let target_rhs = typed_const(&mut bank, "pm_gate_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_gate_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        source.set_prop(CP_NO_GENERATION);
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 0);
        assert!(store.is_empty());
    }

    #[test]
    fn compute_clause_clause_paramodulants_simultaneous_rewrites_all_occurrences() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_sim_source_left");
        let source_right = typed_const(&mut bank, "pm_sim_source_right");
        let f_code = typed_unary_code(&mut bank, "pm_sim_f");
        let g_code = typed_unary_code(&mut bank, "pm_sim_g");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let g_of_source = typed_unary(&mut bank, g_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let g_of_replacement = typed_unary(&mut bank, g_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &g_of_source, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_clause_clause_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &target,
            &mut store,
            ParamodulationType::Simultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store.iter().next().expect("one simultaneous paramodulant");
        assert_eq!(stored.literal_number(), 1);
        let generated = &stored.literals().as_slice()[0];
        assert!(generated.is_positive());
        assert_eq!(generated.left(), &f_of_replacement);
        assert_eq!(generated.right(), &g_of_replacement);
        assert!(stored.derivation().is_none());
    }

    #[test]
    fn clause_ordered_sim_paramod_preserves_c_non_normalized_generated_literal_list() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_lambda_sim_a");
        let source_right = typed_const(&mut bank, "pm_lambda_sim_b");
        let target_rhs = typed_const(&mut bank, "pm_lambda_sim_c");
        let i_type = bank.signature().type_bank().default_type();
        let f_code = typed_unary_code(&mut bank, "pm_lambda_sim_f");
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = typed_unary(&mut bank, f_code, &db0);
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&source_left)).unwrap();
        let expected =
            apply_terms(&mut bank, &lambda, std::slice::from_ref(&source_right)).unwrap();
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &applied, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let from_pos = top_left_position(&source);
        let mut into_pos = top_left_position(&target);
        into_pos.term_pos_mut().push_component(applied, 1);
        source_left.set_prop(TP_POTENTIAL_PARAMOD);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant = clause_ordered_sim_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
            .unwrap()
            .expect("simultaneous paramodulation should rewrite the lambda argument");

        assert_eq!(paramodulant.literal_number(), 1);
        let generated = &paramodulant.literals().as_slice()[0];
        assert!(generated.is_positive());
        assert_eq!(generated.left(), &expected);
        assert_eq!(generated.right(), &target_rhs);
    }

    #[test]
    fn clause_ordered_super_sim_paramod_replaces_instantiated_target_occurrences() {
        let mut bank = test_bank();
        let source_arg = typed_const(&mut bank, "pm_super_source_arg");
        let replacement = typed_const(&mut bank, "pm_super_replacement");
        let variable = typed_var(&bank, -10);
        let f_code = typed_unary_code(&mut bank, "pm_super_f");
        let h_code = typed_unary_code(&mut bank, "pm_super_h");
        let k_code = typed_unary_code(&mut bank, "pm_super_k");
        let f_of_source_arg = typed_unary(&mut bank, f_code, &source_arg);
        let f_of_variable = typed_unary(&mut bank, f_code, &variable);
        let h_of_variable_instance = typed_unary(&mut bank, h_code, &f_of_variable);
        let k_of_source_instance = typed_unary(&mut bank, k_code, &f_of_source_arg);
        let h_of_replacement = typed_unary(&mut bank, h_code, &replacement);
        let k_of_replacement = typed_unary(&mut bank, k_code, &replacement);
        let mut source_literal = lit(&mut bank, &f_of_source_arg, &replacement, true);
        let mut target_literal = lit(
            &mut bank,
            &h_of_variable_instance,
            &k_of_source_instance,
            true,
        );
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let from_pos = top_left_position(&source);
        let mut into_pos = top_left_position(&target);
        into_pos
            .term_pos_mut()
            .push_component(h_of_variable_instance, 0);
        f_of_variable.set_prop(TP_POTENTIAL_PARAMOD);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant =
            clause_ordered_super_sim_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
                .unwrap()
                .expect("super-simultaneous paramodulation should replace both instances");

        assert_eq!(paramodulant.literal_number(), 1);
        let literal = &paramodulant.literals().as_slice()[0];
        assert_eq!(literal.left(), &h_of_replacement);
        assert_eq!(literal.right(), &k_of_replacement);
    }

    #[test]
    fn compute_all_paramodulants_iterates_with_set() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_all_source_left");
        let source_right = typed_const(&mut bank, "pm_all_source_right");
        let first_rhs = typed_const(&mut bank, "pm_all_first_rhs");
        let second_rhs = typed_const(&mut bank, "pm_all_second_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_all_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut first_target = lit(&mut bank, &f_of_source, &first_rhs, true);
        let mut second_target = lit(&mut bank, &f_of_source, &second_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut first_target);
        maximal_oriented(&mut second_target);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let with_set = ClauseSet::from_clauses([
            Clause::alloc(EqnList::from_vec(vec![first_target])),
            Clause::alloc(EqnList::from_vec(vec![second_target])),
        ]);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &with_set,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
    }

    #[test]
    fn compute_all_paramodulants_with_docs_prints_plain_creation_step() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_doc_source_left");
        let source_right = typed_const(&mut bank, "pm_doc_source_right");
        let target_rhs = typed_const(&mut bank, "pm_doc_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_doc_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        source.set_ident(70);
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        target.set_ident(71);
        let with_set = ClauseSet::from_clauses([target.clone()]);
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let count = compute_all_paramodulants_with_docs(
            &mut output,
            &mut session,
            &mut bank,
            &mut ocb,
            &source,
            &source,
            &with_set,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert!(output.contains(" : pm(71,70)\n"));
        let stored = store.iter().next().expect("one documented paramodulant");
        assert_eq!(stored.ident(), 1);
        assert_eq!(stored.literals().as_slice()[0].left(), &f_of_replacement);
        assert!(stored.derivation().is_none());
    }

    #[test]
    fn compute_all_paramodulants_indexed_queries_into_index() {
        let mut bank = test_bank();
        let freshvars = VarBank::new(bank.signature().type_bank());
        bank.vars().pair_shadow(&freshvars);
        let individual = bank.signature().type_bank().default_type();
        let _ = freshvars.get_fresh_var(&individual);
        let _ = freshvars.get_fresh_var(&individual);
        assert_eq!(freshvars.v_count_for_type(&individual), 2);
        let source_left = typed_const(&mut bank, "pm_idx_into_source_left");
        let source_right = typed_const(&mut bank, "pm_idx_into_source_right");
        let target_rhs = typed_const(&mut bank, "pm_idx_into_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_idx_into_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        source.set_proof_depth(3);
        source.set_proof_size(5);
        target.set_proof_depth(4);
        target.set_proof_size(8);
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed_with_fresh_vars(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Plain,
            &freshvars,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(freshvars.v_count_for_type(&individual), 0);
        assert_eq!(store.members(), 1);
        let stored = store.iter().next().expect("one indexed paramodulant");
        assert_eq!(stored.proof_depth(), 5);
        assert_eq!(stored.proof_size(), 14);
        assert_eq!(stored.literal_number(), 1);
        assert_eq!(stored.literals().as_slice()[0].left(), &f_of_replacement);
        assert_eq!(stored.literals().as_slice()[0].right(), &target_rhs);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_PARAMOD),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&target)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&source)),
            ]
        );
    }

    #[test]
    fn compute_all_paramodulants_indexed_reuses_binding_for_shared_target_term() {
        let mut bank = test_bank();
        let source_variable = typed_var(&bank, -2_470);
        let argument = typed_const(&mut bank, "pm_idx_shared_argument");
        let target_rhs = typed_const(&mut bank, "pm_idx_shared_target_rhs");
        let source_code = typed_unary_code(&mut bank, "pm_idx_shared_source");
        let replacement_code = typed_unary_code(&mut bank, "pm_idx_shared_replacement");
        let target_code = typed_binary_code(&mut bank, "pm_idx_shared_target");
        let source_left = typed_unary(&mut bank, source_code, &source_variable);
        let source_right = typed_unary(&mut bank, replacement_code, &source_variable);
        let indexed_term = typed_unary(&mut bank, source_code, &argument);
        let replacement = typed_unary(&mut bank, replacement_code, &argument);
        let target_left = typed_binary(&mut bank, target_code, &indexed_term, &indexed_term);
        let expected_first = typed_binary(&mut bank, target_code, &replacement, &indexed_term);
        let expected_second = typed_binary(&mut bank, target_code, &indexed_term, &replacement);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
        let generated_lefts = store
            .iter()
            .map(|clause| clause.literals().as_slice()[0].left().clone())
            .collect::<Vec<_>>();
        assert!(generated_lefts.contains(&expected_first));
        assert!(generated_lefts.contains(&expected_second));
        assert!(store
            .iter()
            .all(|clause| clause.literals().as_slice()[0].right() == &target_rhs));
        assert!(source_variable.binding().is_none());
    }

    #[test]
    fn compute_all_paramodulants_indexed_reuses_target_unifier_across_source_clauses() {
        let mut bank = test_bank();
        let argument = typed_const(&mut bank, "pm_idx_shared_source_argument");
        let target_variable = typed_var(&bank, -2_472);
        let first_replacement = typed_const(&mut bank, "pm_idx_shared_first_replacement");
        let second_replacement = typed_const(&mut bank, "pm_idx_shared_second_replacement");
        let target_rhs = typed_const(&mut bank, "pm_idx_shared_reverse_rhs");
        let source_code = typed_unary_code(&mut bank, "pm_idx_shared_reverse_source");
        let target_code = typed_unary_code(&mut bank, "pm_idx_shared_reverse_target");
        let source_left = typed_unary(&mut bank, source_code, &argument);
        let target_subterm = typed_unary(&mut bank, source_code, &target_variable);
        let target_left = typed_unary(&mut bank, target_code, &target_subterm);
        let expected_first = typed_unary(&mut bank, target_code, &first_replacement);
        let expected_second = typed_unary(&mut bank, target_code, &second_replacement);
        let mut first_source_literal = lit(&mut bank, &source_left, &first_replacement, true);
        let mut second_source_literal = lit(&mut bank, &source_left, &second_replacement, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_rhs, true);
        maximal_oriented(&mut first_source_literal);
        maximal_oriented(&mut second_source_literal);
        maximal_oriented(&mut target_literal);
        let mut first_source = Clause::alloc(EqnList::from_vec(vec![first_source_literal]));
        let mut second_source = Clause::alloc(EqnList::from_vec(vec![second_source_literal]));
        first_source.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        second_source.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let selected = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let selected_for_paramod = selected.copy_disjoint(&mut bank).unwrap();
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut first_source, &bank, false);
        indices.insert_clause(&mut second_source, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &selected_for_paramod,
            &selected,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 2);
        assert_eq!(store.members(), 2);
        let generated_lefts = store
            .iter()
            .map(|clause| clause.literals().as_slice()[0].left().clone())
            .collect::<Vec<_>>();
        assert!(generated_lefts.contains(&expected_first));
        assert!(generated_lefts.contains(&expected_second));
        assert!(store
            .iter()
            .all(|clause| clause.literals().as_slice()[0].right() == &target_rhs));
        assert!(store
            .iter()
            .all(|clause| clause.query_tptp_type() == CP_TYPE_NEG_CONJECTURE));
        assert!(target_variable.binding().is_none());
    }

    #[test]
    fn compute_all_paramodulants_indexed_higher_order_plain_uses_csu_and_tags_derivation() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let source_left = typed_arrow_var(&mut bank, -2_450);
        let source_right = typed_arrow_const(&mut bank, "pm_idx_ho_plain_source_right");
        let target_left = typed_arrow_const(&mut bank, "pm_idx_ho_plain_target_left");
        let target_right = typed_arrow_const(&mut bank, "pm_idx_ho_plain_target_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        source.set_proof_depth(2);
        source.set_proof_size(4);
        target.set_proof_depth(5);
        target.set_proof_size(7);
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store.iter().next().expect("one indexed paramodulant");
        assert_eq!(stored.proof_depth(), 6);
        assert_eq!(stored.proof_size(), 12);
        assert_eq!(stored.literal_number(), 1);
        let generated = &stored.literals().as_slice()[0];
        assert_eq!(generated.left(), &source_right);
        assert_eq!(generated.right(), &target_right);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(set_is_ho(DC_PARAMOD)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&target)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&source)),
            ]
        );
    }

    #[test]
    fn compute_all_paramodulants_indexed_higher_order_simultaneous_uses_csu() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let source_left = typed_arrow_var(&mut bank, -2_451);
        let source_right = typed_arrow_const(&mut bank, "pm_idx_ho_sim_source_right");
        let target_left = typed_arrow_const(&mut bank, "pm_idx_ho_sim_target_left");
        let target_right = typed_arrow_const(&mut bank, "pm_idx_ho_sim_target_right");
        let target_extra_right = typed_arrow_const(&mut bank, "pm_idx_ho_sim_extra_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        let target_extra = lit(&mut bank, &target_left, &target_extra_right, false);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal, target_extra]));
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Simultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store
            .iter()
            .next()
            .expect("one indexed higher-order simultaneous paramodulant");
        assert_eq!(stored.literal_number(), 2);
        let generated = stored.literals().as_slice();
        assert_eq!(generated[0].left(), &source_right);
        assert_eq!(generated[0].right(), &target_right);
        assert_eq!(generated[1].left(), &source_right);
        assert_eq!(generated[1].right(), &target_extra_right);
        assert!(!generated[1].is_positive());
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(set_is_ho(DC_SIM_PARAMOD)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&target)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&source)),
            ]
        );
    }

    #[test]
    fn indexed_single_replaces_applied_head_in_disjoint_selected_copy() {
        assert_indexed_single_replaces_applied_head(TermOrdering::Kbo6);
    }

    #[test]
    fn indexed_single_lpo4_replaces_applied_head_in_disjoint_selected_copy() {
        assert_indexed_single_replaces_applied_head(TermOrdering::Lpo4);
    }

    fn assert_indexed_single_replaces_applied_head(ordering: TermOrdering) {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Single);
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let binary_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual.clone(),
                    individual.clone(),
                ]));
        let f_code = bank
            .signature_mut()
            .insert_id("pm_idx_single_applied_f", 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, binary_type.clone())
            .unwrap();
        let f = Term::const_cell_alloc(f_code);
        f.set_type(Some(binary_type));
        let f = bank.insert(&f, DerefType::Never).unwrap();
        let a = typed_const(&mut bank, "pm_idx_single_applied_a");
        let source_var = typed_var(&bank, -2_460);
        let source_left = apply_terms(&mut bank, &f, &[a, source_var]).unwrap();
        let source_right = typed_const(&mut bank, "pm_idx_single_applied_d");

        let target_head = typed_arrow_var(&mut bank, -2_462);
        let target_var = typed_var(&bank, -2_464);
        let target_applied =
            apply_terms(&mut bank, &target_head, std::slice::from_ref(&target_var)).unwrap();
        let wrapper_code = typed_unary_code(&mut bank, "pm_idx_single_applied_wrapper");
        let target_left = typed_unary(&mut bank, wrapper_code, &target_applied);
        let target_right = typed_const(&mut bank, "pm_idx_single_applied_rhs");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let target_for_paramod = target.copy_disjoint(&mut bank).unwrap();
        let mut indices = GlobalIndices::new("NoIndex", "FP7", "FP7", 0);
        indices.insert_clause(&mut source, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb =
            OrderControlBlock::alloc(ordering, true, bank.signature(), HoOrderKind::LfhoOrder);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &target_for_paramod,
            &target,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Simultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store
            .iter()
            .next()
            .expect("one applied-head simultaneous paramodulant");
        assert_eq!(stored.literal_number(), 1);
        let expected_left = typed_unary(&mut bank, wrapper_code, &source_right);
        assert_eq!(stored.literals().as_slice()[0].left(), &expected_left);
        assert_eq!(stored.literals().as_slice()[0].right(), &target_right);
    }

    #[test]
    fn compute_all_paramodulants_indexed_higher_order_super_sim_uses_csu_from_index() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        init_unif_limits_for_test(UnifMode::Multi);
        let mut bank = test_bank();
        let source_left = typed_arrow_var(&mut bank, -2_452);
        let source_right = typed_arrow_const(&mut bank, "pm_idx_ho_super_source_right");
        let target_left = typed_arrow_const(&mut bank, "pm_idx_ho_super_target_left");
        let target_right = typed_arrow_const(&mut bank, "pm_idx_ho_super_target_right");
        let target_extra_right = typed_arrow_const(&mut bank, "pm_idx_ho_super_extra_right");
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &target_left, &target_right, false);
        let target_extra = lit(&mut bank, &target_left, &target_extra_right, false);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut indexed_source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let selected = Clause::alloc(EqnList::from_vec(vec![target_literal, target_extra]));
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_source, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo6_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &selected,
            &selected,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::SuperSimultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        let stored = store
            .iter()
            .next()
            .expect("one indexed higher-order super-simultaneous paramodulant");
        assert_eq!(stored.literal_number(), 2);
        let generated = stored.literals().as_slice();
        assert_eq!(generated[0].left(), &source_right);
        assert_eq!(generated[0].right(), &target_right);
        assert!(!generated[0].is_positive());
        assert_eq!(generated[1].left(), &source_right);
        assert_eq!(generated[1].right(), &target_extra_right);
        assert!(!generated[1].is_positive());
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(set_is_ho(DC_SIM_PARAMOD)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&selected)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&indexed_source)),
            ]
        );
    }

    #[test]
    fn compute_all_paramodulants_indexed_simultaneous_rewrites_target_clause_once() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_idx_sim_source_left");
        let source_right = typed_const(&mut bank, "pm_idx_sim_source_right");
        let f_code = typed_unary_code(&mut bank, "pm_idx_sim_f");
        let g_code = typed_unary_code(&mut bank, "pm_idx_sim_g");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let g_of_source = typed_unary(&mut bank, g_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let g_of_replacement = typed_unary(&mut bank, g_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &g_of_source, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Simultaneous,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store
            .iter()
            .next()
            .expect("one indexed simultaneous paramodulant");
        let generated = &stored.literals().as_slice()[0];
        assert_eq!(generated.left(), &f_of_replacement);
        assert_eq!(generated.right(), &g_of_replacement);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SIM_PARAMOD),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&target)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&source)),
            ]
        );
    }

    #[test]
    fn compute_all_paramodulants_indexed_super_sim_replaces_instantiated_target_occurrences() {
        let mut bank = test_bank();
        let source_arg = typed_const(&mut bank, "pm_idx_super_source_arg");
        let replacement = typed_const(&mut bank, "pm_idx_super_replacement");
        let variable = typed_var(&bank, -20);
        let f_code = typed_unary_code(&mut bank, "pm_idx_super_f");
        let h_code = typed_unary_code(&mut bank, "pm_idx_super_h");
        let k_code = typed_unary_code(&mut bank, "pm_idx_super_k");
        let f_of_source_arg = typed_unary(&mut bank, f_code, &source_arg);
        let f_of_variable = typed_unary(&mut bank, f_code, &variable);
        let h_of_variable_instance = typed_unary(&mut bank, h_code, &f_of_variable);
        let k_of_source_instance = typed_unary(&mut bank, k_code, &f_of_source_arg);
        let h_of_replacement = typed_unary(&mut bank, h_code, &replacement);
        let k_of_replacement = typed_unary(&mut bank, k_code, &replacement);
        let mut source_literal = lit(&mut bank, &f_of_source_arg, &replacement, true);
        let mut target_literal = lit(
            &mut bank,
            &h_of_variable_instance,
            &k_of_source_instance,
            true,
        );
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut target, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &source,
            &source,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::SuperSimultaneous,
        )
        .unwrap();

        assert!(count >= 1);
        assert!(store.iter().any(|clause| {
            clause.literal_number() == 1 && {
                let literal = &clause.literals().as_slice()[0];
                literal.left() == &h_of_replacement
                    && literal.right() == &k_of_replacement
                    && clause.derivation().unwrap().as_slice()[0]
                        == DerivationEntry::Operation(DC_SIM_PARAMOD)
            }
        }));
    }

    #[test]
    fn compute_all_paramodulants_indexed_queries_from_index_for_non_top_targets() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_idx_from_source_left");
        let source_right = typed_const(&mut bank, "pm_idx_from_source_right");
        let target_rhs = typed_const(&mut bank, "pm_idx_from_target_rhs");
        let f_code = typed_unary_code(&mut bank, "pm_idx_from_f");
        let f_of_source = typed_unary(&mut bank, f_code, &source_left);
        let f_of_replacement = typed_unary(&mut bank, f_code, &source_right);
        let mut source_literal = lit(&mut bank, &source_left, &source_right, true);
        let mut target_literal = lit(&mut bank, &f_of_source, &target_rhs, true);
        maximal_oriented(&mut source_literal);
        maximal_oriented(&mut target_literal);
        let mut indexed_source = Clause::alloc(EqnList::from_vec(vec![source_literal]));
        let selected = Clause::alloc(EqnList::from_vec(vec![target_literal]));
        indexed_source.set_proof_depth(6);
        indexed_source.set_proof_size(10);
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_source, &bank, false);
        let (into_index, negp_index, from_index) =
            indices.pm_paramodulation_indexes().expect("PM indexes");
        let mut ocb = kbo_ocb(&bank);
        let mut store = ClauseSet::new();

        let count = compute_all_paramodulants_indexed(
            &mut bank,
            &mut ocb,
            &selected,
            &selected,
            into_index,
            negp_index,
            from_index,
            &mut store,
            ParamodulationType::Plain,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(store.members(), 1);
        let stored = store
            .iter()
            .next()
            .expect("one indexed reverse paramodulant");
        assert_eq!(stored.proof_depth(), 7);
        assert_eq!(stored.proof_size(), 11);
        assert_eq!(stored.literal_number(), 1);
        assert_eq!(stored.literals().as_slice()[0].left(), &f_of_replacement);
        assert_eq!(stored.literals().as_slice()[0].right(), &target_rhs);
        assert_eq!(
            stored.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_PARAMOD),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&selected)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&indexed_source)),
            ]
        );
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
    fn clause_ordered_paramod_preserves_c_non_normalized_generated_literal_list() {
        let mut bank = test_bank();
        let source_left = typed_const(&mut bank, "pm_lambda_plain_a");
        let source_right = typed_const(&mut bank, "pm_lambda_plain_b");
        let target_rhs = typed_const(&mut bank, "pm_lambda_plain_c");
        let i_type = bank.signature().type_bank().default_type();
        let f_code = typed_unary_code(&mut bank, "pm_lambda_plain_f");
        let db0 = bank.request_db_var(&i_type, 0);
        let matrix = typed_unary(&mut bank, f_code, &db0);
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&i_type), &matrix).unwrap();
        let applied = apply_terms(&mut bank, &lambda, std::slice::from_ref(&source_left)).unwrap();
        let expected =
            apply_terms(&mut bank, &lambda, std::slice::from_ref(&source_right)).unwrap();
        let mut from_lit = lit(&mut bank, &source_left, &source_right, true);
        let mut into_lit = lit(&mut bank, &applied, &target_rhs, true);
        maximal_oriented(&mut from_lit);
        maximal_oriented(&mut into_lit);
        let from_clause = Clause::alloc(EqnList::from_vec(vec![from_lit]));
        let into_clause = Clause::alloc(EqnList::from_vec(vec![into_lit]));
        let from_pos = top_left_position(&from_clause);
        let mut into_pos = top_left_position(&into_clause);
        into_pos.term_pos_mut().push_component(applied, 1);
        let mut ocb = kbo_ocb(&bank);

        let paramodulant = clause_ordered_paramod(&mut bank, &mut ocb, &from_pos, &into_pos)
            .unwrap()
            .expect("paramodulation into lambda application argument should generate a clause");

        assert_eq!(paramodulant.literal_number(), 1);
        let generated = &paramodulant.literals().as_slice()[0];
        assert!(generated.is_positive());
        assert_eq!(generated.left(), &expected);
        assert_eq!(generated.right(), &target_rhs);
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
