use crate::basics::error::Diagnostic;
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::basics::sysdate::SysDate;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{
    CP_INITIAL, CP_INPUT_FORMULA, CP_IS_D_INDEXED, CP_IS_ORIENTED, CP_IS_SOS, CP_IS_S_INDEXED,
    CP_LIMITED_RW,
};
use crate::clauses::clausefunc::clause_remove_superfluous_literals;
use crate::clauses::clausepos::{term_compute_rw_sequence, ClausePos, RewriteSequenceEntry};
use crate::clauses::clausesets::{clause_set_list_get_max_date, ClauseSet};
use crate::clauses::derivation::{DC_LOCAL_REWRITE, DC_REWRITE};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EqnSide, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE, MAX_SIDE,
    MIN_SIDE,
};
use crate::clauses::inferencedoc::ProofDocSession;
use crate::clauses::subterm_index::SubtermIndex;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::orderings::cto_orderings::to_greater_with_bank;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::match_mgu::subst_match_complete_with_bank;
use crate::terms::replace::{
    make_rewritten_term, term_add_rw_link, term_delete_rw_link, term_follow_top_rw_chain,
    term_follow_top_rw_chain_with_steps, RwResultType,
};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_has_unbound_variables, term_is_db_closed};
use crate::terms::termtypes::{
    term_identity_id, DerefType, RewriteDemodulator, RewriteLevel, Term, TP_IS_REWRITABLE,
    TP_IS_REWRITTEN, TP_IS_RREWRITABLE, TP_IS_RREWRITTEN,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

type LocalRwSystem = HashMap<usize, Term>;
type RewriteDocCallback<'a> =
    dyn FnMut(&TermBank, &mut Clause, usize, EqnSide, &Term) -> Result<(), Diagnostic> + 'a;

pub static REWRITE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_UNBOUND_VAR_FAILS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_UNCACHED: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_CACHE_LINK_LOOKUPS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_CACHE_LINK_HITS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_CACHE_LINK_EDGES: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_CACHE_NF_DATE_CHECKS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_CACHE_NF_DATE_HITS: AtomicU64 = AtomicU64::new(0);
pub static BWRW_MATCH_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static BWRW_MATCH_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static REWRITE_CACHE_TELEMETRY_ENABLED: AtomicBool = AtomicBool::new(false);

pub(crate) struct RewriteCacheTelemetryGuard;

impl Drop for RewriteCacheTelemetryGuard {
    fn drop(&mut self) {
        REWRITE_CACHE_TELEMETRY_ENABLED.store(false, Ordering::Relaxed);
    }
}

#[must_use]
pub(crate) fn enable_rewrite_cache_telemetry() -> RewriteCacheTelemetryGuard {
    REWRITE_CACHE_TELEMETRY_ENABLED.store(true, Ordering::Relaxed);
    RewriteCacheTelemetryGuard
}

#[derive(Default)]
struct PlainRewriteTrace {
    sos_rewritten: bool,
    subst: Substitution,
}

/// Rewrites a clause with local rules extracted from that same clause.
///
/// This ports C `ClauseLocalRW`: orient literals, collect negative oriented
/// equalities `s != t` as `s -> t`, collect positive predicate literals as
/// `p -> $false`, then rewrite all non-rule-source literals with the resulting
/// pointer-identity map.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion fails while recursively
/// rebuilding a changed term.
///
/// # Panics
///
/// Panics if term mapping violates the C term-bank sharing/type invariants, or
/// if literal cleanup is requested while the clause is still indexed.
pub fn clause_local_rw(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    clause.orient_literals_with_bank(ocb, bank)?;

    let rw_sys = collect_local_rw_system(bank, clause);
    let source_literals: Vec<bool> = clause
        .literals()
        .as_slice()
        .iter()
        .map(|literal| is_local_rw_source(literal, bank))
        .collect();
    let true_term = bank.true_term().clone();
    let false_term = bank.false_term().clone();
    let mut modified = false;

    for (literal, is_source) in clause
        .literals_mut()
        .as_mut_slice()
        .iter_mut()
        .zip(source_literals)
    {
        if is_source {
            continue;
        }

        let left = replace_term(&rw_sys, bank, literal.left())?;
        let right = replace_term(&rw_sys, bank, literal.right())?;
        modified |= map_literal_terms(literal, &true_term, &false_term, left, right);
    }

    if modified {
        clause.recompute_lit_counts();
        let _ = clause_remove_superfluous_literals(clause, bank);
        clause.del_prop(CP_IS_ORIENTED);
        clause
            .ensure_derivation()
            .push(RewriteSequenceEntry::Operation(DC_LOCAL_REWRITE));
        clause.set_weight(clause.standard_weight());
    }

    Ok(modified)
}

/// Find clauses rewritable by the new demodulator using the plain set scan.
///
/// This ports C `FindRewritableClauses`/`find_rewritable_clauses`: every
/// clause in `set` is scanned in set order, rewrite flags and links are stored
/// on affected terms as in C, and references to rewritable clauses are appended
/// to `results`.
///
/// # Errors
///
/// Returns a diagnostic if a replacement or designated minimum term cannot be
/// inserted in the term bank.
///
/// # Panics
///
/// Panics if `new_demod` is not a positive unit demodulator, matching the C
/// assertion.
pub fn find_rewritable_clauses<'a>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    set: &'a ClauseSet,
    results: &mut Vec<&'a Clause>,
    new_demod: &Clause,
    nf_date: SysDate,
) -> Result<bool, Diagnostic> {
    assert!(
        new_demod.is_demodulator(),
        "new demodulator must be a positive unit clause"
    );

    let mut found = false;
    for clause in set.iter() {
        if clause_is_rewritable(bank, ocb, clause, new_demod, nf_date)? {
            results.push(clause);
            found = true;
        }
    }
    Ok(found)
}

/// Find clauses rewritable by the new demodulator using a subterm index.
///
/// This ports C `FindRewritableClausesIndexed`: fingerprint-matchable
/// occurrences are checked with a complete match, affected terms receive the
/// same rewrite flags/links as the plain scan, and rewritable clauses are
/// appended once per call.
///
/// # Errors
///
/// Returns a diagnostic if a replacement or designated minimum term cannot be
/// inserted in the term bank.
///
/// # Panics
///
/// Panics if `new_demod` is not a positive unit demodulator, matching the C
/// assertion.
pub fn find_rewritable_clauses_indexed<'idx>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    index: &'idx SubtermIndex,
    results: &mut Vec<&'idx Clause>,
    new_demod: &Clause,
    nf_date: SysDate,
) -> Result<i64, Diagnostic> {
    assert!(
        new_demod.is_demodulator(),
        "new demodulator must be a positive unit clause"
    );

    let eqn = new_demod
        .literals()
        .as_slice()
        .first()
        .expect("positive unit demodulator has one literal");
    let mut seen = BTreeSet::new();
    let mut count = find_rewritable_clauses_indexed_direction(
        bank,
        ocb,
        index,
        results,
        &mut seen,
        new_demod,
        eqn.left(),
        eqn.right(),
        eqn.is_oriented(),
        nf_date,
    )?;

    if !eqn.is_oriented() {
        count += find_rewritable_clauses_indexed_direction(
            bank,
            ocb,
            index,
            results,
            &mut seen,
            new_demod,
            eqn.right(),
            eqn.left(),
            false,
            nf_date,
        )?;
    }

    Ok(count)
}

/// Rewrite a term at the top position with demodulators from a plain set scan.
///
/// This ports the decision rules of C `indexed_find_demodulator` and
/// `rewrite_with_clause_set`, but uses set-order scanning until the perfect
/// discrimination tree is ported.
///
/// # Errors
///
/// Returns a diagnostic if ordering-side checks or replacement insertion need
/// to create terms and fail.
///
/// # Panics
///
/// Panics if `term` is a free variable or already has a top rewrite link,
/// matching the C preconditions.
pub fn rewrite_with_clause_set_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    date: SysDate,
    demodulators: &ClauseSet,
    prefer_general: bool,
    restricted_rw: bool,
) -> Result<Term, Diagnostic> {
    let mut subst = Substitution::new();
    Ok(rewrite_with_clause_set_plain_with_subst(
        bank,
        ocb,
        term,
        date,
        demodulators,
        prefer_general,
        restricted_rw,
        &mut subst,
    )?
    .unwrap_or_else(|| term.clone()))
}

#[expect(
    clippy::too_many_arguments,
    reason = "C rewrite descriptors own a reusable substitution stack"
)]
fn rewrite_with_clause_set_plain_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    date: SysDate,
    demodulators: &ClauseSet,
    prefer_general: bool,
    restricted_rw: bool,
    subst: &mut Substitution,
) -> Result<Option<Term>, Diagnostic> {
    assert!(!term.is_free_var(), "free variables are not rewritten");
    assert!(
        !term.is_top_rewritten(),
        "top-level rewrite expects no existing top rewrite link"
    );
    debug_assert!(subst.is_empty(), "rewrite substitution must be backtracked");

    REWRITE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    demodulators.record_demod_index_search_init_with_bank(bank, term, date, prefer_general)?;

    let found = if demodulators.demod_index_search_may_have_match() {
        match find_plain_demodulator(ocb, bank, term, date, demodulators, subst, restricted_rw) {
            Ok(found) => found,
            Err(error) => {
                subst.backtrack();
                demodulators.record_demod_index_search_exit();
                return Err(error);
            }
        }
    } else {
        None
    };
    demodulators.record_demod_index_search_exit();
    let Some(found) = found else {
        debug_assert!(subst.is_empty(), "failed rewrite must backtrack bindings");
        return Ok(None);
    };

    REWRITE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    let active_problem_type = problem_type();
    let replacement = bank
        .insert_instantiated_for_problem(found.replacement, active_problem_type)
        .and_then(|replacement| {
            if active_problem_type == ProblemType::HigherOrder {
                make_rewritten_term(bank, term, &replacement, 0)
            } else {
                Ok(replacement)
            }
        });
    subst.backtrack();
    let replacement = replacement?;

    if replacement == *term {
        return Ok(None);
    }

    let result_type = if restricted_rw {
        RwResultType::AlwaysRewritable
    } else {
        RwResultType::LimitedRewritable
    };
    term_add_rw_link(
        term,
        &replacement,
        Some(rewrite_demodulator_handle(found.clause)),
        found.clause.is_sos(),
        result_type,
    );
    REWRITE_UNCACHED.fetch_add(1, Ordering::Relaxed);
    Ok(Some(replacement))
}

/// Rewrite a term at top level with the active prefix of demodulator sets.
///
/// This mirrors C `rewrite_with_clause_set_list`: only DB-closed terms are
/// considered, each set is skipped when the term's normal-form date for the
/// selected level is current enough, and scanning stops after the first
/// replacement.
///
/// # Errors
///
/// Returns a diagnostic if a selected demodulator needs ordering checks or term
/// creation that fails.
///
/// # Panics
///
/// Panics if `level` is `NoRewrite`, if the demodulator slice is shorter than
/// the active rewrite level, if `term` is a free variable, or if `term` already
/// has a top rewrite link. These are the C caller preconditions.
pub fn rewrite_with_clause_set_list_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    restricted_rw: bool,
) -> Result<Term, Diagnostic> {
    let mut subst = Substitution::new();
    Ok(rewrite_with_clause_set_list_plain_with_subst(
        bank,
        ocb,
        term,
        demodulators,
        level,
        prefer_general,
        restricted_rw,
        &mut subst,
    )?
    .unwrap_or_else(|| term.clone()))
}

#[expect(
    clippy::too_many_arguments,
    reason = "C rewrite descriptors own a reusable substitution stack"
)]
fn rewrite_with_clause_set_list_plain_with_subst(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    restricted_rw: bool,
    subst: &mut Substitution,
) -> Result<Option<Term>, Diagnostic> {
    let level_count = rewrite_level_count(level);
    assert!(level_count != 0, "rewrite level must be active");
    assert!(
        level_count <= demodulators.len(),
        "demodulator set prefix must cover the rewrite level"
    );
    assert!(!term.is_free_var(), "free variables are not rewritten");
    assert!(
        !term.is_top_rewritten(),
        "top-level rewrite expects no existing top rewrite link"
    );

    let date_level = rewrite_date_level(level);
    for demodulator_set in demodulators.iter().take(level_count) {
        if term_is_db_closed(term)
            && term
                .nf_date(date_level)
                .is_earlier_than(demodulator_set.date())
        {
            let result = rewrite_with_clause_set_plain_with_subst(
                bank,
                ocb,
                term,
                term.nf_date(date_level),
                demodulator_set,
                prefer_general,
                restricted_rw,
                subst,
            )?;
            if result.is_some() {
                return Ok(result);
            }
        }
    }
    Ok(None)
}

/// Compute a plain leftmost-innermost normal form with a set-list scan.
///
/// This ports C `term_li_normalform` around the plain demodulator primitive.
/// The public wrapper computes `demod_date` as the maximum date of the active
/// demodulator-set prefix, matching the usual `RWDesc` setup.
///
/// # Errors
///
/// Returns a diagnostic if rewriting, ordering checks, or term-bank insertion
/// fail.
///
/// # Panics
///
/// Panics if the active rewrite level exceeds the demodulator slice length.
#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C term_li_normalform inputs before RWDesc ownership is ported"
)]
pub fn term_li_normalform_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    restricted_rw: bool,
    lambda_demod: bool,
) -> Result<Term, Diagnostic> {
    if level == RewriteLevel::NoRewrite {
        return Ok(term.clone());
    }
    let level_count = rewrite_level_count(level);
    let demod_date = clause_set_list_get_max_date(demodulators, level_count);
    let mut trace = PlainRewriteTrace::default();
    term_li_normalform_plain_with_date(
        bank,
        ocb,
        term,
        demodulators,
        level,
        demod_date,
        prefer_general,
        restricted_rw,
        lambda_demod,
        &mut trace,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Private recursive worker keeps the computed demodulator date stable"
)]
fn term_li_normalform_plain_with_date(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    demod_date: SysDate,
    prefer_general: bool,
    restricted_rw: bool,
    lambda_demod: bool,
    trace: &mut PlainRewriteTrace,
) -> Result<Term, Diagnostic> {
    debug_assert_ne!(level, RewriteLevel::NoRewrite);

    let (mut current, sos_rewritten) = follow_existing_top_rewrite_link(term, restricted_rw);
    trace.sos_rewritten |= sos_rewritten;
    assert!(
        !current.is_top_rewritten() || restricted_rw,
        "unrestricted normal-form traversal must follow top rewrite links"
    );

    let date_level = rewrite_date_level(level);
    if normal_form_date_is_current(&current, date_level, demod_date) {
        return Ok(current);
    }
    if current.is_free_var() {
        assert!(
            !current.is_rewritten(),
            "rewritten free variables are outside the C rewrite contract"
        );
        return Ok(current);
    }

    let mut modified = true;
    while modified {
        modified = term_subterm_rewrite_plain(
            bank,
            ocb,
            &mut current,
            demodulators,
            level,
            demod_date,
            prefer_general,
            lambda_demod,
            trace,
        )?;

        if !current.is_free_var() {
            let follow_restricted = restricted_rw && !modified;
            clear_existing_rewrite_link_for_ablation(&current);
            let (new_term, sos_rewritten) = if current.is_top_rewritten() {
                follow_existing_top_rewrite_link(&current, follow_restricted)
            } else {
                let _ = rewrite_with_clause_set_list_plain_with_subst(
                    bank,
                    ocb,
                    &current,
                    demodulators,
                    level,
                    prefer_general,
                    follow_restricted,
                    &mut trace.subst,
                )?;
                term_follow_top_rw_chain(&current, follow_restricted)
            };
            trace.sos_rewritten |= sos_rewritten;
            if current != new_term {
                modified = true;
                current = new_term;
            }
        }
    }

    if !current.is_rewritten() && !restricted_rw {
        current.set_nf_date(RewriteLevel::RuleRewrite, demod_date);
        if level == RewriteLevel::FullRewrite {
            current.set_nf_date(RewriteLevel::FullRewrite, demod_date);
        }
    }
    Ok(current)
}

fn follow_existing_top_rewrite_link(term: &Term, restricted_rw: bool) -> (Term, bool) {
    record_rewrite_cache_counter(&REWRITE_CACHE_LINK_LOOKUPS, 1);
    clear_existing_rewrite_link_for_ablation(term);

    let (current, sos_rewritten, followed_edges) =
        term_follow_top_rw_chain_with_steps(term, restricted_rw);
    if followed_edges != 0 {
        record_rewrite_cache_counter(&REWRITE_CACHE_LINK_HITS, 1);
        record_rewrite_cache_counter(&REWRITE_CACHE_LINK_EDGES, followed_edges);
    }
    (current, sos_rewritten)
}

fn normal_form_date_is_current(term: &Term, level: RewriteLevel, demod_date: SysDate) -> bool {
    if cfg!(umlaut_rewrite_cache_ablation) || term.is_rewritten() {
        return false;
    }

    record_rewrite_cache_counter(&REWRITE_CACHE_NF_DATE_CHECKS, 1);
    let current = !term.nf_date(level).is_earlier_than(demod_date);
    if current {
        record_rewrite_cache_counter(&REWRITE_CACHE_NF_DATE_HITS, 1);
    }
    current
}

fn clear_existing_rewrite_link_for_ablation(term: &Term) {
    if cfg!(umlaut_rewrite_cache_ablation) && term.is_rewritten() {
        term_delete_rw_link(term);
    }
}

fn record_rewrite_cache_counter(counter: &AtomicU64, increment: u64) {
    if REWRITE_CACHE_TELEMETRY_ENABLED.load(Ordering::Relaxed) {
        counter.fetch_add(increment, Ordering::Relaxed);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Recursive subterm rewrite mirrors C term_subterm_rewrite context"
)]
#[allow(
    unsafe_code,
    reason = "rewriting retains the immutable source and initializes a distinct unshared copy"
)]
fn term_subterm_rewrite_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &mut Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    demod_date: SysDate,
    prefer_general: bool,
    lambda_demod: bool,
    trace: &mut PlainRewriteTrace,
) -> Result<bool, Diagnostic> {
    if !lambda_demod && term.is_lambda() {
        return Ok(false);
    }

    let rewritten_term = {
        // SAFETY: The source owner remains live and its argument slots are
        // structurally unchanged while normalized children are inspected.
        let source_args = unsafe { term.arguments() };
        let mut rewritten_term: Option<Term> = None;
        for (index, arg) in source_args.iter().enumerate() {
            let arg = arg
                .as_ref()
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let normalized = term_li_normalform_plain_with_date(
                bank,
                ocb,
                arg,
                demodulators,
                level,
                demod_date,
                prefer_general,
                false,
                lambda_demod,
                trace,
            )?;

            if let Some(new_term) = &rewritten_term {
                new_term.set_argument(index, normalized);
            } else if normalized != *arg {
                let new_term = bank.alloc_top_copy_without_args(term);
                {
                    // SAFETY: `new_term` is a fresh, unshared copy distinct
                    // from the immutable source and has no other argument
                    // borrows while these slots are initialized.
                    let target_args = unsafe { new_term.arguments_mut() };
                    for (previous, arg) in source_args[..index].iter().enumerate() {
                        target_args[previous] = Some(arg.clone().unwrap_or_else(|| {
                            panic!("term argument {previous} is uninitialized")
                        }));
                    }
                    target_args[index] = Some(normalized);
                }
                rewritten_term = Some(new_term);
            }
        }
        rewritten_term
    };

    if let Some(new_term) = rewritten_term {
        let replacement = bank.term_top_insert(new_term)?;
        assert_ne!(
            replacement, *term,
            "changed subterms must produce a different shared term"
        );
        term_add_rw_link(
            term,
            &replacement,
            None,
            false,
            RwResultType::AlwaysRewritable,
        );
        *term = replacement;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Compute plain leftmost-innermost normal forms for an equation's sides.
///
/// This mirrors C `eqn_li_normalform` for term mutation, maximality-cache
/// invalidation, equality-literal normalization when the right side becomes
/// `$true`, and the returned side mask.
///
/// # Errors
///
/// Returns a diagnostic if side normalization fails.
///
/// # Panics
///
/// Panics if the active rewrite level exceeds the demodulator slice length.
#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C eqn_li_normalform inputs before ClausePos/RWDesc ownership is ported"
)]
pub fn eqn_li_normalform_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    eqn: &mut Eqn,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    interred_rw: bool,
    lambda_demod: bool,
) -> Result<EqnSide, Diagnostic> {
    let mut rewrite_trace = PlainRewriteTrace::default();
    eqn_li_normalform_plain_trace(
        bank,
        ocb,
        eqn,
        demodulators,
        level,
        prefer_general,
        interred_rw,
        lambda_demod,
        &mut rewrite_trace,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Private worker mirrors C eqn_li_normalform inputs"
)]
fn eqn_li_normalform_plain_trace(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    eqn: &mut Eqn,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    interred_rw: bool,
    lambda_demod: bool,
    rewrite_trace: &mut PlainRewriteTrace,
) -> Result<EqnSide, Diagnostic> {
    let left_old = eqn.left().clone();
    let right_old = eqn.right().clone();
    let restricted_rw = eqn.is_maximal() && eqn.is_positive() && eqn.is_oriented() && interred_rw;
    let mut result = EqnSide::NoSide;

    let left_new = term_li_normalform_plain_with_trace(
        bank,
        ocb,
        &left_old,
        demodulators,
        level,
        prefer_general,
        restricted_rw,
        lambda_demod,
        rewrite_trace,
    )?;
    if left_new != left_old {
        eqn.set_left_raw(left_new.clone());
        eqn.del_prop(EP_MAX_IS_UP_TO_DATE);
        result = MAX_SIDE;
    }

    let right_new = term_li_normalform_plain_with_trace(
        bank,
        ocb,
        &right_old,
        demodulators,
        level,
        prefer_general,
        false,
        lambda_demod,
        rewrite_trace,
    )?;
    if right_new != right_old {
        eqn.set_right_raw(right_new.clone());
        if eqn.query_prop(EP_IS_EQU_LITERAL) && eqn.right() == bank.true_term() {
            eqn.del_prop(EP_IS_EQU_LITERAL);
        }
        if eqn.is_oriented() {
            result = eqn_side_union(result, MIN_SIDE);
        } else {
            result = eqn_side_union(result, MAX_SIDE);
            eqn.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
    }

    Ok(result)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Trace-bearing wrapper keeps public TermComputeLINormalform signature unchanged"
)]
fn term_li_normalform_plain_with_trace(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    restricted_rw: bool,
    lambda_demod: bool,
    trace: &mut PlainRewriteTrace,
) -> Result<Term, Diagnostic> {
    if level == RewriteLevel::NoRewrite {
        return Ok(term.clone());
    }
    let level_count = rewrite_level_count(level);
    let demod_date = clause_set_list_get_max_date(demodulators, level_count);
    term_li_normalform_plain_with_date(
        bank,
        ocb,
        term,
        demodulators,
        level,
        demod_date,
        prefer_general,
        restricted_rw,
        lambda_demod,
        trace,
    )
}

/// Compute plain leftmost-innermost normal forms for every literal in a clause.
///
/// This ports C `ClauseComputeLINormalform`: each literal side is normalized,
/// the compact rewrite derivation stack is extended from recovered term rewrite
/// chains, `CPLimitedRW` is cleared and the scan repeated when C does so, and
/// the return value is the derivation-stack delta divided by two.
///
/// # Errors
///
/// Returns a diagnostic if side normalization fails.
///
/// # Panics
///
/// Panics if the clause is already demodulation- or simplification-indexed, or
/// if the active rewrite level exceeds the demodulator slice length.
pub fn clause_compute_li_normalform_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &mut Clause,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    lambda_demod: bool,
) -> Result<i64, Diagnostic> {
    clause_compute_li_normalform_plain_impl(
        bank,
        ocb,
        clause,
        demodulators,
        level,
        prefer_general,
        lambda_demod,
        None,
    )
}

/// Compute plain normal forms and emit C `DocClauseRewrite` steps when active.
///
/// C calls `DocClauseRewriteDefault` from `eqn_li_normalform` only when
/// `OutputLevel >= 4`. This wrapper preserves that outer gate: lower output
/// levels normalize exactly like [`clause_compute_li_normalform_plain`] and do
/// not clear `CPInputFormula` or allocate proof-documentation ids.
///
/// # Errors
///
/// Returns diagnostics from side normalization or proof-documentation rendering.
///
/// # Panics
///
/// Panics under the same preconditions as
/// [`clause_compute_li_normalform_plain`], and if rewrite documentation is
/// requested for an invalid rewritten side.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible normalization plus proof-documentation plumbing keeps caller state explicit"
)]
pub fn clause_compute_li_normalform_plain_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &mut Clause,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    lambda_demod: bool,
) -> Result<i64, Diagnostic> {
    if session.output_level < 4 {
        return clause_compute_li_normalform_plain(
            bank,
            ocb,
            clause,
            demodulators,
            level,
            prefer_general,
            lambda_demod,
        );
    }

    let mut write_doc = |bank: &TermBank,
                         clause: &mut Clause,
                         literal_index: usize,
                         side: EqnSide,
                         old_term: &Term|
     -> Result<(), Diagnostic> {
        write_clause_rewrite_doc(output, session, bank, clause, literal_index, side, old_term)
    };

    clause_compute_li_normalform_plain_impl(
        bank,
        ocb,
        clause,
        demodulators,
        level,
        prefer_general,
        lambda_demod,
        Some(&mut write_doc),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "Shared implementation mirrors C ClauseComputeLINormalform state"
)]
fn clause_compute_li_normalform_plain_impl(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &mut Clause,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    lambda_demod: bool,
    mut doc_rewrite: Option<&mut RewriteDocCallback<'_>>,
) -> Result<i64, Diagnostic> {
    assert!(
        !clause.is_any_prop_set(CP_IS_D_INDEXED | CP_IS_S_INDEXED),
        "indexed clauses must be removed from rewrite indexes before normalization"
    );

    let old_deriv_sp = clause.derivation_stack_pointer();
    let mut rewrite_trace = PlainRewriteTrace::default();
    let mut done = false;
    while !done {
        done = true;
        for index in 0..clause.literals().len() {
            let interred_rw = clause.query_prop(CP_LIMITED_RW);
            let side = clause_literal_li_normalform_plain(
                bank,
                ocb,
                clause,
                index,
                demodulators,
                level,
                prefer_general,
                interred_rw,
                lambda_demod,
                &mut rewrite_trace,
                &mut doc_rewrite,
            )?;

            let literal = &clause.literals().as_slice()[index];
            if eqn_side_contains(side, MAX_SIDE)
                && literal.is_positive()
                && literal.is_maximal()
                && clause.query_prop(CP_LIMITED_RW)
            {
                clause.del_prop(CP_LIMITED_RW);
                done = false;
            }
        }
    }

    if rewrite_trace.sos_rewritten {
        clause.set_prop(CP_IS_SOS);
    }

    let new_deriv_sp = clause.derivation_stack_pointer();
    let rewrite_steps = i64::try_from((new_deriv_sp - old_deriv_sp) / 2)
        .unwrap_or_else(|_| panic!("rewrite derivation stack delta does not fit in i64"));
    if rewrite_steps != 0 {
        clause.del_prop(CP_INITIAL);
    }

    Ok(rewrite_steps)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C eqn_li_normalform while keeping live clause documentation state"
)]
fn clause_literal_li_normalform_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &mut Clause,
    literal_index: usize,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    interred_rw: bool,
    lambda_demod: bool,
    rewrite_trace: &mut PlainRewriteTrace,
    doc_rewrite: &mut Option<&mut RewriteDocCallback<'_>>,
) -> Result<EqnSide, Diagnostic> {
    let (left_old, restricted_rw) = {
        let literal = &clause.literals().as_slice()[literal_index];
        (
            literal.left().clone(),
            literal.is_maximal() && literal.is_positive() && literal.is_oriented() && interred_rw,
        )
    };
    let mut result = EqnSide::NoSide;

    let left_new = term_li_normalform_plain_with_trace(
        bank,
        ocb,
        &left_old,
        demodulators,
        level,
        prefer_general,
        restricted_rw,
        lambda_demod,
        rewrite_trace,
    )?;
    if left_new != left_old {
        {
            let literal = &mut clause.literals_mut().as_mut_slice()[literal_index];
            literal.set_left_raw(left_new.clone());
            literal.del_prop(EP_MAX_IS_UP_TO_DATE);
        }
        result = MAX_SIDE;
        if let Some(doc_rewrite) = doc_rewrite.as_mut() {
            doc_rewrite(bank, clause, literal_index, EqnSide::LeftSide, &left_old)?;
        }
        record_term_normalform_trace(clause.ensure_derivation(), &left_old, &left_new);
    }

    let right_old = clause.literals().as_slice()[literal_index].right().clone();
    let right_new = term_li_normalform_plain_with_trace(
        bank,
        ocb,
        &right_old,
        demodulators,
        level,
        prefer_general,
        false,
        lambda_demod,
        rewrite_trace,
    )?;
    if right_new != right_old {
        let right_side = {
            let literal = &mut clause.literals_mut().as_mut_slice()[literal_index];
            literal.set_right_raw(right_new.clone());
            if literal.query_prop(EP_IS_EQU_LITERAL) && literal.right() == bank.true_term() {
                literal.del_prop(EP_IS_EQU_LITERAL);
            }
            if literal.is_oriented() {
                MIN_SIDE
            } else {
                literal.del_prop(EP_MAX_IS_UP_TO_DATE);
                MAX_SIDE
            }
        };
        result = eqn_side_union(result, right_side);
        if let Some(doc_rewrite) = doc_rewrite.as_mut() {
            doc_rewrite(bank, clause, literal_index, EqnSide::RightSide, &right_old)?;
        }
        record_term_normalform_trace(clause.ensure_derivation(), &right_old, &right_new);
    }

    Ok(result)
}

fn write_clause_rewrite_doc(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    bank: &TermBank,
    clause: &mut Clause,
    literal_index: usize,
    side: EqnSide,
    old_term: &Term,
) -> Result<(), Diagnostic> {
    let mut position: ClausePos<()> = ClausePos::for_clause(clause.clone());
    assert!(
        position.set_literal_index(Some(literal_index)),
        "rewrite documentation side must refer to an existing literal"
    );
    position.set_side(side);

    let _ = session.doc_clause_rewrite(output, bank, &mut position, old_term, None)?;
    let documented = position
        .clause()
        .expect("rewrite documentation position must remain clause-backed");
    clause.set_ident(documented.ident());
    if !documented.query_prop(CP_INPUT_FORMULA) {
        clause.del_prop(CP_INPUT_FORMULA);
    }
    Ok(())
}

/// Compute plain leftmost-innermost normal forms for every clause in a set.
///
/// This ports C `ClauseSetComputeLINormalform`: each clause is normalized in
/// set iteration order, the returned rewrite-step counts are summed, and a
/// rewritten clause's cached standard weight is refreshed.
///
/// # Errors
///
/// Returns a diagnostic if any clause normalization fails.
///
/// # Panics
///
/// Panics under the same preconditions as [`clause_compute_li_normalform_plain`].
pub fn clause_set_compute_li_normalform_plain(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    set: &mut ClauseSet,
    demodulators: &[&ClauseSet],
    level: RewriteLevel,
    prefer_general: bool,
    lambda_demod: bool,
) -> Result<i64, Diagnostic> {
    let mut result = 0;
    for clause in set.iter_mut() {
        let steps = clause_compute_li_normalform_plain(
            bank,
            ocb,
            clause,
            demodulators,
            level,
            prefer_general,
            lambda_demod,
        )?;
        if steps != 0 {
            clause.set_weight(clause.standard_weight());
        }
        result += steps;
    }
    Ok(result)
}

fn record_term_normalform_trace(stack: &mut PStack<RewriteSequenceEntry>, old: &Term, new: &Term) {
    let recorded = term_compute_rw_sequence(stack, old, new, DC_REWRITE);
    debug_assert!(
        recorded,
        "changed side should expose at least one rewrite link"
    );
}

fn eqn_side_contains(side: EqnSide, needle: EqnSide) -> bool {
    ((side as i32) & (needle as i32)) != 0
}

fn eqn_side_union(left: EqnSide, right: EqnSide) -> EqnSide {
    match (left as i32) | (right as i32) {
        0 => EqnSide::NoSide,
        1 => EqnSide::LeftSide,
        2 => EqnSide::RightSide,
        3 => EqnSide::BothSides,
        _ => unreachable!("EqnSide only uses the low two bits"),
    }
}

struct PlainDemodulatorMatch<'a> {
    clause: &'a Clause,
    replacement: &'a Term,
}

fn find_plain_demodulator<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    demodulators: &'a ClauseSet,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    if demodulators.demod_index_search_uses_exact_candidates() {
        return find_indexed_demodulator(ocb, bank, term, date, demodulators, subst, restricted_rw);
    }

    find_set_order_demodulator(ocb, bank, term, date, demodulators, subst, restricted_rw)
}

fn find_indexed_demodulator<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    demodulators: &'a ClauseSet,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    if problem_type() != ProblemType::FirstOrder {
        return find_materialized_indexed_demodulator(
            ocb,
            bank,
            term,
            date,
            demodulators,
            subst,
            restricted_rw,
        );
    }

    while let Some(candidate) =
        demodulators.demod_index_search_next_candidate_side_with_subst(subst)
    {
        let Some(clause) = demodulators.find_indexed_by_derivation_ref(candidate.clause_ref())
        else {
            continue;
        };
        let Some(match_) = try_pre_matched_demodulator_clause_side(
            ocb,
            bank,
            term,
            date,
            clause,
            candidate.side,
            subst,
            restricted_rw,
        )?
        else {
            continue;
        };
        return Ok(Some(match_));
    }
    Ok(None)
}

fn find_materialized_indexed_demodulator<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    demodulators: &'a ClauseSet,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    while let Some(candidate) = demodulators.demod_index_search_next_candidate_side() {
        let Some(clause) = demodulators.find_indexed_by_derivation_ref(candidate.clause_ref())
        else {
            continue;
        };
        let Some(match_) = try_demodulator_clause_side(
            ocb,
            bank,
            term,
            date,
            clause,
            candidate.side,
            subst,
            restricted_rw,
        )?
        else {
            continue;
        };
        return Ok(Some(match_));
    }
    Ok(None)
}

fn find_set_order_demodulator<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    demodulators: &'a ClauseSet,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    for clause in demodulators.iter() {
        if !clause.is_demodulator() {
            continue;
        }

        if let Some(match_) = try_demodulator_clause_side(
            ocb,
            bank,
            term,
            date,
            clause,
            EqnSide::LeftSide,
            subst,
            restricted_rw,
        )? {
            return Ok(Some(match_));
        }

        if let Some(match_) = try_demodulator_clause_side(
            ocb,
            bank,
            term,
            date,
            clause,
            EqnSide::RightSide,
            subst,
            restricted_rw,
        )? {
            return Ok(Some(match_));
        }
    }
    Ok(None)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C indexed side checks with the PDTree substitution live"
)]
fn try_pre_matched_demodulator_clause_side<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    clause: &'a Clause,
    side: EqnSide,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    if !clause.is_demodulator() || !date.is_earlier_than(clause.date()) {
        return Ok(None);
    }

    let eqn = clause
        .literals()
        .as_slice()
        .first()
        .expect("positive unit demodulator has one literal");
    if demodulator_date_blocks_term(term, clause, eqn) {
        return Ok(None);
    }

    let replacement = match side {
        EqnSide::LeftSide
            if (eqn.is_oriented()
                || instance_is_rule(ocb, bank, eqn.left(), eqn.right(), subst)?)
                && (!restricted_rw || !subst.is_renaming()) =>
        {
            Some(eqn.right())
        }
        EqnSide::RightSide
            if !eqn.is_oriented()
                && instance_is_rule(ocb, bank, eqn.right(), eqn.left(), subst)? =>
        {
            Some(eqn.left())
        }
        _ => None,
    };
    Ok(replacement.map(|replacement| PlainDemodulatorMatch {
        clause,
        replacement,
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C indexed side checks with explicit state"
)]
fn try_demodulator_clause_side<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    date: SysDate,
    clause: &'a Clause,
    side: EqnSide,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    if !clause.is_demodulator() || !date.is_earlier_than(clause.date()) {
        return Ok(None);
    }

    let eqn = clause
        .literals()
        .as_slice()
        .first()
        .expect("positive unit demodulator has one literal");
    if demodulator_date_blocks_term(term, clause, eqn) {
        return Ok(None);
    }

    match side {
        EqnSide::NoSide => Ok(None),
        EqnSide::LeftSide => {
            try_left_demodulator_side(ocb, bank, term, clause, eqn, subst, restricted_rw)
        }
        EqnSide::RightSide => try_right_demodulator_side(ocb, bank, term, clause, eqn, subst),
        EqnSide::BothSides => {
            if let Some(match_) =
                try_left_demodulator_side(ocb, bank, term, clause, eqn, subst, restricted_rw)?
            {
                return Ok(Some(match_));
            }
            try_right_demodulator_side(ocb, bank, term, clause, eqn, subst)
        }
    }
}

fn try_left_demodulator_side<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    clause: &'a Clause,
    eqn: &'a Eqn,
    subst: &mut Substitution,
    restricted_rw: bool,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    let backtrack = subst.len();
    if subst_match_complete_with_bank(bank, eqn.left(), term, subst)?
        && (eqn.is_oriented() || instance_is_rule(ocb, bank, eqn.left(), eqn.right(), subst)?)
        && (!restricted_rw || !subst.is_renaming())
    {
        return Ok(Some(PlainDemodulatorMatch {
            clause,
            replacement: eqn.right(),
        }));
    }
    subst.backtrack_to_pos(backtrack);
    Ok(None)
}

fn try_right_demodulator_side<'a>(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    term: &Term,
    clause: &'a Clause,
    eqn: &'a Eqn,
    subst: &mut Substitution,
) -> Result<Option<PlainDemodulatorMatch<'a>>, Diagnostic> {
    if eqn.is_oriented() {
        return Ok(None);
    }

    let backtrack = subst.len();
    if subst_match_complete_with_bank(bank, eqn.right(), term, subst)?
        && instance_is_rule(ocb, bank, eqn.right(), eqn.left(), subst)?
    {
        return Ok(Some(PlainDemodulatorMatch {
            clause,
            replacement: eqn.left(),
        }));
    }
    subst.backtrack_to_pos(backtrack);
    Ok(None)
}

fn demodulator_date_blocks_term(term: &Term, clause: &Clause, eqn: &Eqn) -> bool {
    let level = if eqn.is_oriented() {
        RewriteLevel::RuleRewrite
    } else {
        RewriteLevel::FullRewrite
    };
    !term.nf_date(level).is_earlier_than(clause.date())
}

fn rewrite_level_count(level: RewriteLevel) -> usize {
    match level {
        RewriteLevel::NoRewrite => 0,
        RewriteLevel::RuleRewrite => 1,
        RewriteLevel::FullRewrite => 2,
    }
}

fn rewrite_date_level(level: RewriteLevel) -> RewriteLevel {
    match level {
        RewriteLevel::NoRewrite => panic!("no rewrite level has no normal-form date"),
        RewriteLevel::RuleRewrite => RewriteLevel::RuleRewrite,
        RewriteLevel::FullRewrite => RewriteLevel::FullRewrite,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors C find_rewritable_clauses_indexed direction parameters"
)]
fn find_rewritable_clauses_indexed_direction<'idx>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    index: &'idx SubtermIndex,
    results: &mut Vec<&'idx Clause>,
    seen: &mut BTreeSet<i64>,
    new_demod: &Clause,
    left: &Term,
    right: &Term,
    oriented: bool,
    _nf_date: SysDate,
) -> Result<i64, Diagnostic> {
    let mut occurrences = Vec::new();
    index.collect_matchable_occurrences(left, bank.signature(), &mut occurrences);
    let mut count = 0;
    for occurrence in occurrences {
        count += term_find_rw_clauses_indexed(
            bank, ocb, occurrence, results, seen, new_demod, left, right, oriented,
        )?;
    }
    Ok(count)
}

#[expect(
    clippy::too_many_arguments,
    reason = "Keeps the indexed term check aligned with C term_find_rw_clauses"
)]
fn term_find_rw_clauses_indexed<'idx>(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    occurrence: &'idx SubtermOcc,
    results: &mut Vec<&'idx Clause>,
    seen: &mut BTreeSet<i64>,
    new_demod: &Clause,
    left: &Term,
    right: &Term,
    oriented: bool,
) -> Result<i64, Diagnostic> {
    assert!(
        !occurrence.term().is_free_var(),
        "free variables are not indexed for backward rewriting"
    );

    let mut subst = Substitution::new();
    let mut count = 0;

    BWRW_MATCH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if subst_match_complete_with_bank(bank, left, occurrence.term(), &mut subst)? {
        BWRW_MATCH_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        if oriented || instance_is_rule(ocb, bank, left, right, &mut subst)? {
            let result = if !oriented || !subst.is_renaming() {
                occurrence
                    .term()
                    .set_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
                count += push_indexed_clause_map(results, seen, occurrence.full_clauses());
                count += push_indexed_clause_map(results, seen, occurrence.restricted_clauses());
                RwResultType::AlwaysRewritable
            } else {
                occurrence.term().set_prop(TP_IS_REWRITABLE);
                count += push_indexed_clause_map(results, seen, occurrence.full_clauses());
                RwResultType::LimitedRewritable
            };

            let _ = add_top_rewrite_link_if_needed(
                bank,
                occurrence.term(),
                right,
                new_demod.is_sos(),
                rewrite_demodulator_handle(new_demod),
                result,
            )?;
        }
        subst.backtrack();
    }

    Ok(count)
}

fn push_indexed_clause_map<'idx>(
    results: &mut Vec<&'idx Clause>,
    seen: &mut BTreeSet<i64>,
    clauses: &'idx BTreeMap<i64, Clause>,
) -> i64 {
    let mut count = 0;
    for (key, clause) in clauses {
        if seen.insert(*key) {
            results.push(clause);
            count += 1;
        }
    }
    count
}

fn clause_is_rewritable(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    clause: &Clause,
    new_demod: &Clause,
    nf_date: SysDate,
) -> Result<bool, Diagnostic> {
    let mut rewritable = false;
    for literal in clause.literals().as_slice() {
        if eqn_has_rw_side(bank, ocb, literal, new_demod, nf_date)? != EqnSide::NoSide {
            rewritable = true;
        }
    }
    Ok(rewritable)
}

fn eqn_has_rw_side(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    eqn: &Eqn,
    new_demod: &Clause,
    nf_date: SysDate,
) -> Result<EqnSide, Diagnostic> {
    let restricted_rw = eqn.is_maximal() && eqn.is_positive() && eqn.is_oriented();
    let left_rewritable =
        term_is_rewritable(bank, ocb, eqn.left(), new_demod, nf_date, restricted_rw)?;
    let right_rewritable = term_is_rewritable(bank, ocb, eqn.right(), new_demod, nf_date, false)?;

    if left_rewritable {
        Ok(MAX_SIDE)
    } else if right_rewritable {
        Ok(if eqn.is_oriented() {
            MIN_SIDE
        } else {
            MAX_SIDE
        })
    } else {
        Ok(EqnSide::NoSide)
    }
}

fn term_is_rewritable(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    new_demod: &Clause,
    nf_date: SysDate,
    restricted_rw: bool,
) -> Result<bool, Diagnostic> {
    if term.is_free_var() {
        return Ok(false);
    }
    if term.query_prop(TP_IS_RREWRITABLE) || (!restricted_rw && term.query_prop(TP_IS_REWRITABLE)) {
        return Ok(true);
    }
    if term.nf_date(RewriteLevel::FullRewrite) == nf_date {
        return Ok(false);
    }
    if !term.is_lambda() {
        for index in 0..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            if term_is_rewritable(bank, ocb, &arg, new_demod, nf_date, false)? {
                term.set_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
                return Ok(true);
            }
        }
    }

    match term_is_top_rewritable(bank, ocb, term, new_demod, restricted_rw)? {
        RwResultType::LimitedRewritable => return Ok(!restricted_rw),
        RwResultType::AlwaysRewritable => return Ok(true),
        RwResultType::NotRewritable => {}
    }

    if !restricted_rw
        && !term.is_any_prop_set(
            TP_IS_REWRITABLE | TP_IS_RREWRITABLE | TP_IS_REWRITTEN | TP_IS_RREWRITTEN,
        )
    {
        term.set_nf_date(RewriteLevel::RuleRewrite, nf_date);
        term.set_nf_date(RewriteLevel::FullRewrite, nf_date);
    }
    Ok(false)
}

fn term_is_top_rewritable(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    new_demod: &Clause,
    restricted_rw: bool,
) -> Result<RwResultType, Diagnostic> {
    assert!(
        new_demod.is_demodulator(),
        "new demodulator must be a positive unit clause"
    );
    assert!(!term.is_free_var(), "free variables are not top-rewritable");

    let eqn = new_demod
        .literals()
        .as_slice()
        .first()
        .expect("positive unit demodulator has one literal");
    let demodulator = rewrite_demodulator_handle(new_demod);
    let mut subst = Substitution::new();
    let mut result = RwResultType::NotRewritable;

    BWRW_MATCH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if subst_match_complete_with_bank(bank, eqn.left(), term, &mut subst)? {
        BWRW_MATCH_SUCCESSES.fetch_add(1, Ordering::Relaxed);
        if eqn.is_oriented() || instance_is_rule(ocb, bank, eqn.left(), eqn.right(), &mut subst)? {
            result = if !eqn.is_oriented() || !subst.is_renaming() {
                term.set_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
                RwResultType::AlwaysRewritable
            } else {
                term.set_prop(TP_IS_REWRITABLE);
                RwResultType::LimitedRewritable
            };
            if !add_top_rewrite_link_if_needed(
                bank,
                term,
                eqn.right(),
                new_demod.is_sos(),
                demodulator,
                result,
            )? {
                result = RwResultType::NotRewritable;
            }
        }
        subst.backtrack();
    }

    if !matches!(result, RwResultType::AlwaysRewritable)
        && (restricted_rw || result != RwResultType::LimitedRewritable)
        && !eqn.is_oriented()
    {
        BWRW_MATCH_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        if subst_match_complete_with_bank(bank, eqn.right(), term, &mut subst)? {
            BWRW_MATCH_SUCCESSES.fetch_add(1, Ordering::Relaxed);
            if instance_is_rule(ocb, bank, eqn.right(), eqn.left(), &mut subst)? {
                term.set_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
                result = RwResultType::AlwaysRewritable;
                if !add_top_rewrite_link_if_needed(
                    bank,
                    term,
                    eqn.left(),
                    new_demod.is_sos(),
                    demodulator,
                    result,
                )? {
                    result = RwResultType::NotRewritable;
                }
            }
            subst.backtrack();
        }
    }

    Ok(result)
}

fn add_top_rewrite_link_if_needed(
    bank: &mut TermBank,
    term: &Term,
    replacement_pattern: &Term,
    sos: bool,
    demodulator: RewriteDemodulator,
    result: RwResultType,
) -> Result<bool, Diagnostic> {
    if term.is_rewritten() && result != RwResultType::AlwaysRewritable {
        return Ok(true);
    }

    let active_problem_type = problem_type();
    let replacement =
        bank.insert_instantiated_for_problem(replacement_pattern, active_problem_type)?;
    let replacement = if active_problem_type == ProblemType::HigherOrder {
        make_rewritten_term(bank, term, &replacement, 0)?
    } else {
        replacement
    };
    if replacement == *term {
        term.del_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
        Ok(false)
    } else {
        term_add_rw_link(term, &replacement, Some(demodulator), sos, result);
        REWRITE_UNCACHED.fetch_add(1, Ordering::Relaxed);
        Ok(true)
    }
}

fn instance_is_rule(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    lside: &Term,
    rside: &Term,
    subst: &mut Substitution,
) -> Result<bool, Diagnostic> {
    if ocb.rewrite_strong_rhs_inst {
        subst_complete_min_instance(ocb, bank, subst, rside)?;
    } else if term_has_unbound_variables(rside) {
        REWRITE_UNBOUND_VAR_FAILS.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }
    if subst.is_renaming() {
        return Ok(false);
    }

    to_greater_with_bank(ocb, bank, lside, rside, DerefType::Once, DerefType::Once)
}

fn subst_complete_min_instance(
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    subst: &mut Substitution,
    term: &Term,
) -> Result<(), Diagnostic> {
    if term.is_free_var() {
        if term.binding().is_none() {
            let type_ = term.type_().expect("free variable must have a type");
            let binding = ocb.designated_min_term(bank, &type_)?;
            subst.add_binding(term, &binding);
        }
    } else {
        for index in 0..term.arity() {
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            subst_complete_min_instance(ocb, bank, subst, &arg)?;
        }
    }
    Ok(())
}

fn rewrite_demodulator_handle(clause: &Clause) -> RewriteDemodulator {
    let ident = clause.ident();
    let id = if ident > 0 {
        usize::try_from(ident).unwrap_or(usize::MAX)
    } else {
        usize::try_from(ident.unsigned_abs())
            .unwrap_or(usize::MAX)
            .saturating_add(1)
    };
    RewriteDemodulator::new_with_generation(id.max(1), clause.derivation_generation())
}

fn collect_local_rw_system(bank: &TermBank, clause: &Clause) -> LocalRwSystem {
    let mut rw_sys = LocalRwSystem::new();

    for literal in clause.literals().as_slice() {
        if literal.is_negative() && literal.is_oriented() {
            rw_sys.insert(term_identity_id(literal.left()), literal.right().clone());
        } else if !literal.is_equ_lit(bank) && literal.is_positive() {
            debug_assert_eq!(literal.right(), bank.true_term());
            rw_sys.insert(term_identity_id(literal.left()), bank.false_term().clone());
        }
    }

    rw_sys
}

fn is_local_rw_source(literal: &Eqn, bank: &TermBank) -> bool {
    (literal.is_negative() && literal.is_oriented())
        || (!literal.is_equ_lit(bank) && literal.is_positive())
}

fn replace_term(
    rw_sys: &LocalRwSystem,
    bank: &mut TermBank,
    term: &Term,
) -> Result<Term, Diagnostic> {
    bank.map_term(term, &mut |_, candidate| {
        Ok(Some(
            rw_sys
                .get(&term_identity_id(candidate))
                .cloned()
                .unwrap_or_else(|| candidate.clone()),
        ))
    })
}

fn map_literal_terms(
    literal: &mut Eqn,
    true_term: &Term,
    false_term: &Term,
    mut left: Term,
    mut right: Term,
) -> bool {
    let old_left = literal.left().clone();
    let old_right = literal.right().clone();
    let mut negate = false;

    if left == *false_term {
        left = true_term.clone();
        negate = !negate;
    }
    if right == *false_term {
        right = true_term.clone();
        negate = !negate;
    }
    if left == *true_term {
        std::mem::swap(&mut left, &mut right);
    }
    if right == *true_term {
        literal.del_prop(EP_IS_EQU_LITERAL);
    } else {
        literal.set_prop(EP_IS_EQU_LITERAL);
    }

    if negate {
        literal.flip_prop(EP_IS_POSITIVE);
    }

    if left != old_left {
        literal.del_prop(EP_MAX_IS_UP_TO_DATE);
        literal.del_prop(EP_IS_ORIENTED);
    }

    literal.set_left_raw(left);
    literal.set_right_raw(right);

    literal.left() != &old_left || literal.right() != &old_right
}

#[cfg(test)]
mod tests {
    use super::{
        clause_compute_li_normalform_plain, clause_compute_li_normalform_plain_with_docs,
        clause_local_rw, clause_set_compute_li_normalform_plain, enable_rewrite_cache_telemetry,
        eqn_has_rw_side, eqn_li_normalform_plain, find_rewritable_clauses,
        find_rewritable_clauses_indexed, rewrite_with_clause_set_list_plain,
        rewrite_with_clause_set_plain, term_is_top_rewritable, term_li_normalform_plain,
        RwResultType, BWRW_MATCH_ATTEMPTS, BWRW_MATCH_SUCCESSES, REWRITE_ATTEMPTS,
        REWRITE_CACHE_LINK_EDGES, REWRITE_CACHE_LINK_HITS, REWRITE_CACHE_LINK_LOOKUPS,
        REWRITE_CACHE_NF_DATE_CHECKS, REWRITE_CACHE_NF_DATE_HITS, REWRITE_SUCCESSES,
        REWRITE_UNBOUND_VAR_FAILS, REWRITE_UNCACHED,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_INITIAL, CP_INPUT_FORMULA, CP_IS_ORIENTED, CP_IS_SOS, CP_LIMITED_RW,
    };
    use crate::clauses::clausepos::RewriteSequenceEntry;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_POSITIVE,
        EP_MAX_IS_UP_TO_DATE,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::clauses::pdtrees::PdtTraversalOrder;
    use crate::clauses::subterm_index::SubtermIndex;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::idx_fp::index_fp1_create;
    use crate::terms::lambda::{apply_terms, close_with_type_prefix};
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{
        DerefType, RewriteDemodulator, RewriteLevel, Term, TP_IS_REWRITABLE,
    };
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;
    use std::sync::atomic::Ordering;
    use std::sync::{Mutex, MutexGuard};

    static REWRITE_COUNTER_LOCK: Mutex<()> = Mutex::new(());

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

    fn typed_unary_with_return(
        bank: &mut TermBank,
        name: &str,
        arg_type: &Type,
        return_type: Type,
        arg: &Term,
    ) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        let term_type = return_type.clone();
        if bank.signature().get_type(f_code).is_none() {
            let fun_type = bank
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), return_type]));
            bank.signature_mut()
                .declare_final_type(f_code, fun_type)
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(term_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_return(bank, name, &type_, type_.clone(), arg)
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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn bool_predicate(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        typed_unary_with_return(bank, name, &individual, bool_type, arg)
    }

    fn bool_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        typed_unary_with_return(bank, name, &bool_type, bool_type.clone(), arg)
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn oriented_demod(literal: &mut Eqn) {
        literal.set_prop(EP_IS_ORIENTED);
    }

    fn reset_backward_rewrite_counters() -> MutexGuard<'static, ()> {
        let guard = REWRITE_COUNTER_LOCK
            .lock()
            .expect("rewrite counter test lock should not be poisoned");
        REWRITE_ATTEMPTS.store(0, Ordering::Relaxed);
        REWRITE_SUCCESSES.store(0, Ordering::Relaxed);
        REWRITE_UNCACHED.store(0, Ordering::Relaxed);
        REWRITE_CACHE_LINK_LOOKUPS.store(0, Ordering::Relaxed);
        REWRITE_CACHE_LINK_HITS.store(0, Ordering::Relaxed);
        REWRITE_CACHE_LINK_EDGES.store(0, Ordering::Relaxed);
        REWRITE_CACHE_NF_DATE_CHECKS.store(0, Ordering::Relaxed);
        REWRITE_CACHE_NF_DATE_HITS.store(0, Ordering::Relaxed);
        BWRW_MATCH_ATTEMPTS.store(0, Ordering::Relaxed);
        BWRW_MATCH_SUCCESSES.store(0, Ordering::Relaxed);
        REWRITE_UNBOUND_VAR_FAILS.store(0, Ordering::Relaxed);
        guard
    }

    #[test]
    fn negative_oriented_literal_rewrites_other_literals() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_a");
        let c = typed_const(&mut bank, "local_rw_c");
        let f_a = typed_unary(&mut bank, "local_rw_f", &a);
        let g_f_a = typed_unary(&mut bank, "local_rw_g", &f_a);
        let g_a = typed_unary(&mut bank, "local_rw_g", &a);
        let rule = eqn(&mut bank, &f_a, &a, false);
        let target = eqn(&mut bank, &g_f_a, &c, true);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rule, target]));
        clause.set_prop(CP_IS_ORIENTED);
        clause.set_weight(999);

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(modified);
        let rewritten = &clause.literals().as_slice()[0];
        assert_eq!(rewritten.left(), &g_a);
        assert_eq!(rewritten.right(), &c);
        assert!(!rewritten.query_prop(EP_IS_ORIENTED));
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.weight(), clause.standard_weight());
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[RewriteSequenceEntry::Operation(5)]
        );
        let source = &clause.literals().as_slice()[1];
        assert_eq!(source.left(), &f_a);
        assert_eq!(source.right(), &a);
        assert!(source.query_prop(EP_IS_ORIENTED));
    }

    #[test]
    fn positive_atom_rule_rewrites_equational_target_subterms_to_false() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_bool_a");
        let p_a = bool_predicate(&mut bank, "local_rw_p", &a);
        let q_p_a = bool_unary(&mut bank, "local_rw_q", &p_a);
        let false_term = bank.false_term().clone();
        let true_term = bank.true_term().clone();
        let q_false = bool_unary(&mut bank, "local_rw_q", &false_term);
        assert!(p_a.type_().as_ref().is_some_and(Type::is_bool));
        assert_eq!(p_a.type_(), bank.true_term().type_());
        let source = Eqn::alloc(p_a.clone(), true_term.clone(), &mut bank, true).unwrap();
        let target = Eqn::alloc(q_p_a, q_false.clone(), &mut bank, true).unwrap();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![source, target]));

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(modified);
        let source = &clause.literals().as_slice()[0];
        assert_eq!(source.left(), &p_a);
        assert!(source.query_prop(EP_IS_POSITIVE));
        let rewritten = &clause.literals().as_slice()[1];
        assert_eq!(rewritten.left(), &q_false);
        assert_eq!(rewritten.right(), &q_false);
        assert!(rewritten.is_positive());
        assert_eq!(clause.weight(), clause.standard_weight());
    }

    #[test]
    fn returns_false_when_only_rule_sources_are_present() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "local_rw_only_a");
        let f_a = typed_unary(&mut bank, "local_rw_only_f", &a);
        let rule = eqn(&mut bank, &f_a, &a, false);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![rule]));

        let modified = clause_local_rw(&mut kbo_ocb(&bank), &mut bank, &mut clause).unwrap();

        assert!(!modified);
        assert_eq!(clause.literals().as_slice()[0].left(), &f_a);
        assert_eq!(clause.literals().as_slice()[0].right(), &a);
        assert!(clause.derivation().is_none());
    }

    #[test]
    fn plain_backward_rewrite_scan_links_matching_child_terms() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "bwrw_a");
        let b = typed_const(&mut bank, "bwrw_b");
        let c = typed_const(&mut bank, "bwrw_c");
        let f_x = typed_unary(&mut bank, "bwrw_f", &x);
        let f_b = typed_unary(&mut bank, "bwrw_f", &b);
        let g_f_b = typed_unary(&mut bank, "bwrw_g", &f_b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        let target = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &g_f_b, &c, true)]));
        let target_id = target.ident();
        let set = ClauseSet::from_clauses([target]);
        let mut ocb = kbo_ocb(&bank);
        let mut results = Vec::new();

        let found = find_rewritable_clauses(
            &mut bank,
            &mut ocb,
            &set,
            &mut results,
            &demod,
            SysDate::from_raw(7),
        )
        .unwrap();

        assert!(found);
        assert_eq!(
            results
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![target_id]
        );
        assert!(f_b.is_top_rewritten());
        assert_eq!(f_b.rw_replace_field(), Some(a));
        assert!(g_f_b.query_prop(TP_IS_REWRITABLE));
        assert!(BWRW_MATCH_ATTEMPTS.load(Ordering::Relaxed) >= 1);
        assert!(BWRW_MATCH_SUCCESSES.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn plain_backward_rewrite_ignores_self_replacements() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "bwrw_self_a");
        let c = typed_const(&mut bank, "bwrw_self_c");
        let f_x = typed_unary(&mut bank, "bwrw_self_f", &x);
        let f_a = typed_unary(&mut bank, "bwrw_self_f", &a);
        let mut demod_lit = eqn(&mut bank, &f_x, &f_x, true);
        oriented_demod(&mut demod_lit);
        let demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        let target = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &c, true)]));
        let set = ClauseSet::from_clauses([target]);
        let mut ocb = kbo_ocb(&bank);
        let mut results = Vec::new();

        assert!(!find_rewritable_clauses(
            &mut bank,
            &mut ocb,
            &set,
            &mut results,
            &demod,
            SysDate::from_raw(12),
        )
        .unwrap());

        assert!(results.is_empty());
        assert!(!f_a.query_prop(TP_IS_REWRITABLE));
        assert!(!f_a.is_top_rewritten());
    }

    #[test]
    fn indexed_backward_rewrite_deduplicates_full_and_restricted_hits() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "bwrw_idx_a");
        let b = typed_const(&mut bank, "bwrw_idx_b");
        let f_x = typed_unary(&mut bank, "bwrw_idx_f", &x);
        let f_b = typed_unary(&mut bank, "bwrw_idx_f", &b);
        let g_f_b = typed_unary(&mut bank, "bwrw_idx_g", &f_b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        let mut target_lit = eqn(&mut bank, &f_b, &g_f_b, true);
        target_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL);
        let mut target = Clause::alloc(EqnList::from_vec(vec![target_lit]));
        target.set_ident(31);
        let mut index = SubtermIndex::new(index_fp1_create);
        index.insert_clause(&target, false);
        let mut ocb = kbo_ocb(&bank);
        let mut results = Vec::new();

        let count = find_rewritable_clauses_indexed(
            &mut bank,
            &mut ocb,
            &index,
            &mut results,
            &demod,
            SysDate::from_raw(13),
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(
            results
                .iter()
                .map(|clause| clause.ident())
                .collect::<Vec<_>>(),
            vec![31]
        );
        assert!(f_b.is_top_rewritten());
        assert_eq!(f_b.rw_replace_field(), Some(a));
        assert!(BWRW_MATCH_ATTEMPTS.load(Ordering::Relaxed) >= 1);
        assert!(BWRW_MATCH_SUCCESSES.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn indexed_backward_rewrite_checks_reverse_unoriented_direction() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "bwrw_idx_rev_a");
        let b = typed_const(&mut bank, "bwrw_idx_rev_b");
        let c = typed_const(&mut bank, "bwrw_idx_rev_c");
        let f_x = typed_unary(&mut bank, "bwrw_idx_rev_f", &x);
        let f_b = typed_unary(&mut bank, "bwrw_idx_rev_f", &b);
        let demod = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &f_x, true)]));
        let mut target = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_b, &c, true)]));
        target.set_ident(32);
        let mut index = SubtermIndex::new(index_fp1_create);
        index.insert_clause(&target, false);
        let mut ocb = kbo_ocb(&bank);
        let mut results = Vec::new();

        let count = find_rewritable_clauses_indexed(
            &mut bank,
            &mut ocb,
            &index,
            &mut results,
            &demod,
            SysDate::from_raw(14),
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(results[0].ident(), 32);
        assert!(f_b.is_top_rewritten());
        assert_eq!(f_b.rw_replace_field(), Some(a));
    }

    #[test]
    fn indexed_forward_rewrite_preserves_c_shared_variable_match() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let z = typed_var(&bank, -6);
        let w = typed_var(&bank, -8);
        let yz = typed_binary(&mut bank, "rw_idx_cycle_j", &y, &z);
        let yx = typed_binary(&mut bank, "rw_idx_cycle_j", &y, &x);
        let left = typed_binary(&mut bank, "rw_idx_cycle_j", &x, &yz);
        let right = typed_binary(&mut bank, "rw_idx_cycle_j", &z, &yx);
        let g_y = typed_unary(&mut bank, "rw_idx_cycle_g", &y);
        let w_y = typed_binary(&mut bank, "rw_idx_cycle_j", &w, &y);
        let target = typed_binary(&mut bank, "rw_idx_cycle_j", &g_y, &w_y);
        let w_g_y = typed_binary(&mut bank, "rw_idx_cycle_j", &w, &g_y);
        let expected = typed_binary(&mut bank, "rw_idx_cycle_j", &y, &w_g_y);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &left, &right, true)]));
        demod.set_ident(33);
        demod.set_date(SysDate::from_raw(1));
        demod.set_weight(demod.standard_weight());
        let mut demods = ClauseSet::new_demod_indexed();
        demods.indexed_insert_clause_owned(demod, &bank);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );

        demods.record_demod_index_search_init(&target, SysDate::from_raw(0), false);
        let candidates = demods.demod_index_search_candidate_sides().unwrap();
        demods.record_demod_index_search_exit();
        assert!(candidates
            .iter()
            .any(|candidate| { candidate.clause_id == 33 && candidate.side == EqnSide::LeftSide }));

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &target,
            SysDate::from_raw(0),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, expected);
        assert!(demods.demod_index_match_count() > 0);
    }

    #[test]
    fn indexed_forward_rewrite_matches_lusk6ext_clause_680_root() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let z = typed_var(&bank, -6);
        let yz = typed_binary(&mut bank, "rw_lusk680_j", &y, &z);
        let xy = typed_binary(&mut bank, "rw_lusk680_j", &x, &y);
        let yx = typed_binary(&mut bank, "rw_lusk680_j", &y, &x);
        let demod_left = typed_binary(&mut bank, "rw_lusk680_j", &x, &yz);
        let older_demod_right = typed_binary(&mut bank, "rw_lusk680_j", &z, &xy);
        let demod_right = typed_binary(&mut bank, "rw_lusk680_j", &z, &yx);
        let z_y = typed_binary(&mut bank, "rw_lusk680_j", &z, &y);
        let g_z_y = typed_unary(&mut bank, "rw_lusk680_g", &z_y);
        let x_y = typed_binary(&mut bank, "rw_lusk680_j", &x, &y);
        let target = typed_binary(&mut bank, "rw_lusk680_j", &g_z_y, &x_y);
        let x_g_z_y = typed_binary(&mut bank, "rw_lusk680_j", &x, &g_z_y);
        let expected = typed_binary(&mut bank, "rw_lusk680_j", &y, &x_g_z_y);
        let mut older_demod = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &demod_left,
            &older_demod_right,
            true,
        )]));
        older_demod.set_ident(546);
        older_demod.set_date(SysDate::from_raw(1));
        older_demod.set_weight(older_demod.standard_weight());
        let mut demod = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &demod_left,
            &demod_right,
            true,
        )]));
        demod.set_ident(2_574);
        demod.set_date(SysDate::from_raw(2));
        demod.set_weight(demod.standard_weight());
        let mut demods = ClauseSet::new_demod_indexed();
        demods.indexed_insert_clause_owned(older_demod, &bank);
        demods.indexed_insert_clause_owned(demod, &bank);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &target,
            SysDate::creation_time(),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, expected);
        assert_eq!(
            target.rw_demod_field().map(RewriteDemodulator::id),
            Some(2_574)
        );
    }

    #[test]
    fn plain_clause_set_rewrite_links_first_matching_demodulator() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "rw_plain_a");
        let b = typed_const(&mut bank, "rw_plain_b");
        let f_x = typed_unary(&mut bank, "rw_plain_f", &x);
        let f_b = typed_unary(&mut bank, "rw_plain_f", &b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_ident(41);
        demod.set_date(SysDate::from_raw(5));
        demod.set_weight(demod.standard_weight());
        let mut demods = ClauseSet::new_demod_indexed();
        demods.indexed_insert_clause_owned(demod, &bank);
        let mut ocb = kbo_ocb(&bank);

        assert_eq!(demods.demod_index_match_count(), 0);
        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            SysDate::from_raw(0),
            &demods,
            true,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, a);
        assert_eq!(f_b.rw_replace_field(), Some(a));
        assert!(f_b.is_top_rewritten());
        assert!(!f_b.is_rrewritten());
        assert_eq!(demods.demod_index_match_count(), 1);
        assert_eq!(
            demods.demod_index_traversal_order(),
            Some(PdtTraversalOrder::symbols_first())
        );
        assert!(!demods.demod_index_search_active());
        assert_eq!(demods.demod_index_search_state(), None);
        assert!(REWRITE_ATTEMPTS.load(Ordering::Relaxed) >= 1);
        assert!(REWRITE_SUCCESSES.load(Ordering::Relaxed) >= 1);
        assert!(REWRITE_UNCACHED.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn higher_order_root_rewrite_beta_normalizes_instantiated_rhs() {
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
        let k_code = bank
            .signature_mut()
            .insert_id("rw_ho_root_beta_k", 1, false);
        bank.signature_mut()
            .declare_final_type(k_code, unary)
            .unwrap();
        let k = bank.create_const_term(k_code).unwrap();
        let a = typed_const(&mut bank, "rw_ho_root_beta_a");
        let b = typed_const(&mut bank, "rw_ho_root_beta_b");
        let x = typed_var(&bank, -2);
        let f_x = typed_unary(&mut bank, "rw_ho_root_beta_f", &x);
        let f_a = typed_unary(&mut bank, "rw_ho_root_beta_f", &a);
        let f_b = typed_unary(&mut bank, "rw_ho_root_beta_f", &b);
        let db0 = bank.request_db_var(&individual, 0);
        let matrix = apply_terms(&mut bank, &k, std::slice::from_ref(&db0)).unwrap();
        let lambda =
            close_with_type_prefix(&mut bank, std::slice::from_ref(&individual), &matrix).unwrap();
        let beta_redex = apply_terms(&mut bank, &lambda, std::slice::from_ref(&b)).unwrap();
        let expected = apply_terms(&mut bank, &k, std::slice::from_ref(&b)).unwrap();
        assert!(beta_redex.is_beta_reducible());

        let mut demod_lit = eqn(&mut bank, &f_x, &beta_redex, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let demods = ClauseSet::from_clauses([demod]);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_a,
            SysDate::creation_time(),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, expected);
        assert!(!rewritten.is_beta_reducible());
        assert_eq!(f_a.rw_replace_field(), Some(expected.clone()));

        let backward_result = term_is_top_rewritable(
            &mut bank,
            &mut ocb,
            &f_b,
            demods.iter().next().unwrap(),
            false,
        )
        .unwrap();

        assert_eq!(backward_result, RwResultType::AlwaysRewritable);
        assert_eq!(f_b.rw_replace_field(), Some(expected));
    }

    #[test]
    fn higher_order_root_rewrite_matches_applied_variable_patterns() {
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
        let argument = typed_const(&mut bank, "rw_ho_match_argument");
        let replacement = typed_const(&mut bank, "rw_ho_match_replacement");
        let flex_application =
            apply_terms(&mut bank, &flex, std::slice::from_ref(&argument)).unwrap();
        let rigid_application = typed_unary(&mut bank, "rw_ho_match_rigid", &argument);
        let matcher = typed_unary(&mut bank, "rw_ho_match_outer", &flex_application);
        let target = typed_unary(&mut bank, "rw_ho_match_outer", &rigid_application);

        let mut demod_lit = eqn(&mut bank, &matcher, &replacement, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let demods = ClauseSet::from_clauses([demod]);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &target,
            SysDate::creation_time(),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, replacement);
        assert_eq!(target.rw_replace_field(), Some(replacement));
        assert!(target.is_top_rewritten());
        assert!(flex.binding().is_none());
    }

    #[test]
    fn indexed_clause_set_rewrite_uses_pdt_candidate_order_before_set_order() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -12);
        let b = typed_const(&mut bank, "rw_order_b");
        let specific_replacement = typed_const(&mut bank, "rw_order_specific");
        let general_replacement = typed_const(&mut bank, "rw_order_general");
        let f_x = typed_unary(&mut bank, "rw_order_f", &x);
        let f_b = typed_unary(&mut bank, "rw_order_f", &b);
        let mut specific_lit = eqn(&mut bank, &f_b, &specific_replacement, true);
        let mut general_lit = eqn(&mut bank, &f_x, &general_replacement, true);
        oriented_demod(&mut specific_lit);
        oriented_demod(&mut general_lit);
        let mut specific = Clause::alloc(EqnList::from_vec(vec![specific_lit]));
        let mut general = Clause::alloc(EqnList::from_vec(vec![general_lit]));
        let mut demods = ClauseSet::new_demod_indexed();
        let mut ocb = kbo_ocb(&bank);

        specific.set_date(SysDate::from_raw(5));
        general.set_date(SysDate::from_raw(5));
        specific.set_weight(specific.standard_weight());
        general.set_weight(general.standard_weight());
        demods.indexed_insert_clause_owned(specific, &bank);
        demods.indexed_insert_clause_owned(general, &bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            SysDate::from_raw(0),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, general_replacement);
        assert_eq!(f_b.rw_replace_field(), Some(general_replacement));
        assert_eq!(
            demods.demod_index_traversal_order(),
            Some(PdtTraversalOrder::variables_first())
        );
    }

    #[test]
    fn plain_clause_set_rewrite_respects_normal_form_dates() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "rw_plain_nf_a");
        let b = typed_const(&mut bank, "rw_plain_nf_b");
        let f_x = typed_unary(&mut bank, "rw_plain_nf_f", &x);
        let f_b = typed_unary(&mut bank, "rw_plain_nf_f", &b);
        f_b.set_nf_date(RewriteLevel::RuleRewrite, SysDate::from_raw(5));
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let demods = ClauseSet::from_clauses([demod]);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            SysDate::from_raw(0),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, f_b);
        assert!(!f_b.is_top_rewritten());
        assert!(REWRITE_ATTEMPTS.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn plain_clause_set_rewrite_uses_pdt_root_size_prune_for_indexed_sets() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "rw_pdt_prune_a");
        let b = typed_const(&mut bank, "rw_pdt_prune_b");
        let f_a = typed_unary(&mut bank, "rw_pdt_prune_f", &a);
        let mut demod_lit = eqn(&mut bank, &f_a, &b, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        demod.set_weight(demod.standard_weight());
        let mut demods = ClauseSet::new_demod_indexed();
        demods.indexed_insert_clause_owned(demod, &bank);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &a,
            SysDate::creation_time(),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, a);
        assert!(!a.is_top_rewritten());
        assert_eq!(demods.demod_index_search_state(), None);
        assert_eq!(demods.demod_index_match_count(), 1);
        assert!(!demods.demod_index_search_active());
    }

    #[test]
    fn plain_clause_set_rewrite_rejects_restricted_renaming_match() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let f_x = typed_unary(&mut bank, "rw_plain_rest_f", &x);
        let g_x = typed_unary(&mut bank, "rw_plain_rest_g", &x);
        let f_y = typed_unary(&mut bank, "rw_plain_rest_f", &y);
        let mut demod_lit = eqn(&mut bank, &f_x, &g_x, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let demods = ClauseSet::from_clauses([demod]);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_y,
            SysDate::from_raw(0),
            &demods,
            false,
            true,
        )
        .unwrap();

        assert_eq!(rewritten, f_y);
        assert!(!f_y.is_top_rewritten());
        assert!(y.binding().is_none());
    }

    #[test]
    fn plain_clause_set_rewrite_counts_but_does_not_link_self_replacement() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "rw_plain_self_a");
        let f_x = typed_unary(&mut bank, "rw_plain_self_f", &x);
        let f_a = typed_unary(&mut bank, "rw_plain_self_f", &a);
        let mut demod_lit = eqn(&mut bank, &f_x, &f_x, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let demods = ClauseSet::from_clauses([demod]);
        let mut ocb = kbo_ocb(&bank);

        let rewritten = rewrite_with_clause_set_plain(
            &mut bank,
            &mut ocb,
            &f_a,
            SysDate::from_raw(0),
            &demods,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rewritten, f_a);
        assert!(!f_a.is_top_rewritten());
        assert!(REWRITE_ATTEMPTS.load(Ordering::Relaxed) >= 1);
        assert!(REWRITE_SUCCESSES.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn plain_clause_set_list_uses_active_level_prefix() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "rw_list_a");
        let b = typed_const(&mut bank, "rw_list_b");
        let f_x = typed_unary(&mut bank, "rw_list_f", &x);
        let f_b = typed_unary(&mut bank, "rw_list_f", &b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let mut rule_set = ClauseSet::new();
        rule_set.set_date(SysDate::from_raw(5));
        let mut full_set = ClauseSet::from_clauses([demod]);
        full_set.set_date(SysDate::from_raw(5));
        let demodulators = [&rule_set, &full_set];
        let mut ocb = kbo_ocb(&bank);

        let rule_result = rewrite_with_clause_set_list_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(rule_result, f_b);
        assert!(!f_b.is_top_rewritten());

        let full_result = rewrite_with_clause_set_list_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &demodulators,
            RewriteLevel::FullRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(full_result, a);
        assert_eq!(f_b.rw_replace_field(), Some(a));
    }

    #[test]
    fn plain_li_normalform_rewrites_subterm_then_top() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "li_plain_a");
        let b = typed_const(&mut bank, "li_plain_b");
        let c = typed_const(&mut bank, "li_plain_c");
        let f_x = typed_unary(&mut bank, "li_plain_f", &x);
        let f_b = typed_unary(&mut bank, "li_plain_f", &b);
        let h_x = typed_unary(&mut bank, "li_plain_h", &x);
        let h_a = typed_unary(&mut bank, "li_plain_h", &a);
        let h_f_b = typed_unary(&mut bank, "li_plain_h", &f_b);
        let mut inner_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut inner_lit);
        let mut outer_lit = eqn(&mut bank, &h_x, &c, true);
        oriented_demod(&mut outer_lit);
        let mut inner_demod = Clause::alloc(EqnList::from_vec(vec![inner_lit]));
        inner_demod.set_date(SysDate::from_raw(5));
        let mut outer_demod = Clause::alloc(EqnList::from_vec(vec![outer_lit]));
        outer_demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([inner_demod, outer_demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut ocb = kbo_ocb(&bank);

        let normal = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &h_f_b,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(normal, c);
        assert_eq!(f_b.rw_replace_field(), Some(a));
        assert!(f_b.is_top_rewritten());
        assert_eq!(h_f_b.rw_replace_field(), Some(h_a.clone()));
        assert!(h_f_b.is_rewritten());
        assert!(!h_f_b.is_top_rewritten());
        assert_eq!(h_a.rw_replace_field(), Some(c));
        assert!(h_a.is_top_rewritten());
    }

    #[test]
    fn plain_li_normalform_reuses_shared_link_and_records_cache_activity() {
        let _counter_guard = reset_backward_rewrite_counters();
        let _telemetry_guard = enable_rewrite_cache_telemetry();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "li_cache_a");
        let b = typed_const(&mut bank, "li_cache_b");
        let f_x = typed_unary(&mut bank, "li_cache_f", &x);
        let f_b = typed_unary(&mut bank, "li_cache_f", &b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut ocb = kbo_ocb(&bank);

        let first = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();
        let second = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(first, a);
        assert_eq!(second, a);
        assert!(REWRITE_CACHE_LINK_LOOKUPS.load(Ordering::Relaxed) >= 2);
        if cfg!(umlaut_rewrite_cache_ablation) {
            assert_eq!(REWRITE_CACHE_LINK_HITS.load(Ordering::Relaxed), 0);
            assert_eq!(REWRITE_CACHE_LINK_EDGES.load(Ordering::Relaxed), 0);
            assert!(REWRITE_UNCACHED.load(Ordering::Relaxed) >= 2);
        } else {
            assert!(REWRITE_CACHE_LINK_HITS.load(Ordering::Relaxed) >= 1);
            assert!(REWRITE_CACHE_LINK_EDGES.load(Ordering::Relaxed) >= 1);
            assert_eq!(REWRITE_UNCACHED.load(Ordering::Relaxed), 1);
        }
    }

    #[test]
    fn newer_rule_epoch_invalidates_negative_normal_form_date() {
        let _counter_guard = reset_backward_rewrite_counters();
        let _telemetry_guard = enable_rewrite_cache_telemetry();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "li_epoch_a");
        let b = typed_const(&mut bank, "li_epoch_b");
        let f_x = typed_unary(&mut bank, "li_epoch_f", &x);
        let f_b = typed_unary(&mut bank, "li_epoch_f", &b);
        let mut empty = ClauseSet::new();
        empty.set_date(SysDate::from_raw(5));
        let empty_demodulators = [&empty];
        let mut ocb = kbo_ocb(&bank);

        let before = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &empty_demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(before, f_b);
        assert_eq!(f_b.nf_date(RewriteLevel::RuleRewrite), SysDate::from_raw(5));
        let before_cached = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &empty_demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(before_cached, f_b);

        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(8));
        let mut newer = ClauseSet::from_clauses([demod]);
        newer.set_date(SysDate::from_raw(8));
        let newer_demodulators = [&newer];

        let after = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &newer_demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(after, a);
        if !cfg!(umlaut_rewrite_cache_ablation) {
            assert!(REWRITE_CACHE_NF_DATE_CHECKS.load(Ordering::Relaxed) >= 2);
            assert!(REWRITE_CACHE_NF_DATE_HITS.load(Ordering::Relaxed) >= 1);
        }
    }

    #[test]
    fn newer_rule_extends_existing_shared_rewrite_chain() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "li_chain_a");
        let b = typed_const(&mut bank, "li_chain_b");
        let f_x = typed_unary(&mut bank, "li_chain_f", &x);
        let f_b = typed_unary(&mut bank, "li_chain_f", &b);
        let g_x = typed_unary(&mut bank, "li_chain_g", &x);
        let g_b = typed_unary(&mut bank, "li_chain_g", &b);
        let mut first_lit = eqn(&mut bank, &f_x, &g_x, true);
        oriented_demod(&mut first_lit);
        let mut first_demod = Clause::alloc(EqnList::from_vec(vec![first_lit.clone()]));
        first_demod.set_date(SysDate::from_raw(5));
        let mut first_set = ClauseSet::from_clauses([first_demod]);
        first_set.set_date(SysDate::from_raw(5));
        let first_demodulators = [&first_set];
        let mut ocb = kbo_ocb(&bank);

        let first = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &first_demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();
        assert_eq!(first, g_b);

        let mut second_lit = eqn(&mut bank, &g_x, &a, true);
        oriented_demod(&mut second_lit);
        let mut retained_demod = Clause::alloc(EqnList::from_vec(vec![first_lit]));
        retained_demod.set_date(SysDate::from_raw(5));
        let mut second_demod = Clause::alloc(EqnList::from_vec(vec![second_lit]));
        second_demod.set_date(SysDate::from_raw(8));
        let mut extended = ClauseSet::from_clauses([retained_demod, second_demod]);
        extended.set_date(SysDate::from_raw(8));
        let extended_demodulators = [&extended];

        let second = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &f_b,
            &extended_demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(second, a);
        assert_eq!(f_b.rw_replace_field(), Some(g_b.clone()));
        assert_eq!(g_b.rw_replace_field(), Some(a));
    }

    #[test]
    fn plain_li_normalform_rebuilds_binary_parent_from_either_changed_child() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "li_binary_a");
        let b = typed_const(&mut bank, "li_binary_b");
        let c = typed_const(&mut bank, "li_binary_c");
        let f_x = typed_unary(&mut bank, "li_binary_f", &x);
        let f_a = typed_unary(&mut bank, "li_binary_f", &a);
        let f_b = typed_unary(&mut bank, "li_binary_f", &b);
        let left_changed = typed_binary(&mut bank, "li_binary_p", &f_a, &b);
        let left_expected = typed_binary(&mut bank, "li_binary_p", &c, &b);
        let right_changed = typed_binary(&mut bank, "li_binary_q", &a, &f_b);
        let right_expected = typed_binary(&mut bank, "li_binary_q", &a, &c);

        let mut demod_lit = eqn(&mut bank, &f_x, &c, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut ocb = kbo_ocb(&bank);

        let left_normal = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &left_changed,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();
        let right_normal = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &right_changed,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(left_normal, left_expected);
        assert_eq!(right_normal, right_expected);
        assert_eq!(left_changed.rw_replace_field(), Some(left_expected));
        assert_eq!(right_changed.rw_replace_field(), Some(right_expected));
    }

    #[test]
    fn plain_li_normalform_records_full_nf_dates_when_unrewritten() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "li_plain_nf_a");
        a.set_nf_date(RewriteLevel::RuleRewrite, SysDate::from_raw(0));
        a.set_nf_date(RewriteLevel::FullRewrite, SysDate::from_raw(0));
        let mut rule_set = ClauseSet::new();
        rule_set.set_date(SysDate::from_raw(3));
        let mut full_set = ClauseSet::new();
        full_set.set_date(SysDate::from_raw(7));
        let demodulators = [&rule_set, &full_set];
        let mut ocb = kbo_ocb(&bank);

        let normal = term_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &a,
            &demodulators,
            RewriteLevel::FullRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(normal, a);
        assert_eq!(a.nf_date(RewriteLevel::RuleRewrite), SysDate::from_raw(7));
        assert_eq!(a.nf_date(RewriteLevel::FullRewrite), SysDate::from_raw(7));
    }

    #[test]
    fn eqn_li_normalform_rewrites_oriented_left_and_right_sides() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let left_replacement = typed_const(&mut bank, "eqn_nf_a");
        let left_arg = typed_const(&mut bank, "eqn_nf_b");
        let right_replacement = typed_const(&mut bank, "eqn_nf_c");
        let right_arg = typed_const(&mut bank, "eqn_nf_d");
        let f_variable = typed_unary(&mut bank, "eqn_nf_f", &variable);
        let f_left_arg = typed_unary(&mut bank, "eqn_nf_f", &left_arg);
        let g_variable = typed_unary(&mut bank, "eqn_nf_g", &variable);
        let g_right_arg = typed_unary(&mut bank, "eqn_nf_g", &right_arg);
        let mut first_lit = eqn(&mut bank, &f_variable, &left_replacement, true);
        oriented_demod(&mut first_lit);
        let mut second_lit = eqn(&mut bank, &g_variable, &right_replacement, true);
        oriented_demod(&mut second_lit);
        let mut first_demod = Clause::alloc(EqnList::from_vec(vec![first_lit]));
        first_demod.set_date(SysDate::from_raw(5));
        let mut second_demod = Clause::alloc(EqnList::from_vec(vec![second_lit]));
        second_demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([first_demod, second_demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_left_arg, &g_right_arg, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
        let mut ocb = kbo_ocb(&bank);

        let side = eqn_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut literal,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            true,
            false,
        )
        .unwrap();

        assert_eq!(side, EqnSide::BothSides);
        assert_eq!(literal.left(), &left_replacement);
        assert_eq!(literal.right(), &right_replacement);
        assert!(literal.is_oriented());
        assert!(!literal.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn eqn_li_normalform_reports_right_rewrite_as_max_side_when_unoriented() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "eqn_nf_unor_a");
        let b = typed_const(&mut bank, "eqn_nf_unor_b");
        let c = typed_const(&mut bank, "eqn_nf_unor_c");
        let f_x = typed_unary(&mut bank, "eqn_nf_unor_f", &x);
        let f_b = typed_unary(&mut bank, "eqn_nf_unor_f", &b);
        let mut demod_lit = eqn(&mut bank, &f_x, &c, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &a, &f_b, true);
        literal.del_prop(EP_IS_ORIENTED);
        literal.set_prop(EP_MAX_IS_UP_TO_DATE);
        let mut ocb = kbo_ocb(&bank);

        let side = eqn_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut literal,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(side, EqnSide::LeftSide);
        assert_eq!(literal.right(), &c);
        assert!(!literal.query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn eqn_li_normalform_clears_equ_literal_when_right_becomes_true() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let b = typed_const(&mut bank, "eqn_nf_bool_b");
        let p_b = bool_predicate(&mut bank, "eqn_nf_bool_p", &b);
        let q_x = bool_predicate(&mut bank, "eqn_nf_bool_q", &x);
        let q_b = bool_predicate(&mut bank, "eqn_nf_bool_q", &b);
        let true_term = bank.true_term().clone();
        let mut demod_lit = eqn(&mut bank, &q_x, &true_term, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &p_b, &q_b, true);
        assert!(literal.query_prop(EP_IS_EQU_LITERAL));
        let mut ocb = kbo_ocb(&bank);

        let side = eqn_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut literal,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
            false,
        )
        .unwrap();

        assert_eq!(side, EqnSide::LeftSide);
        assert_eq!(literal.right(), &true_term);
        assert!(!literal.query_prop(EP_IS_EQU_LITERAL));
    }

    #[test]
    fn clause_li_normalform_records_derivation_and_clears_initial() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let left_replacement = typed_const(&mut bank, "clause_nf_a");
        let left_arg = typed_const(&mut bank, "clause_nf_b");
        let right_replacement = typed_const(&mut bank, "clause_nf_c");
        let right_arg = typed_const(&mut bank, "clause_nf_d");
        let f_variable = typed_unary(&mut bank, "clause_nf_f", &variable);
        let f_left_arg = typed_unary(&mut bank, "clause_nf_f", &left_arg);
        let g_variable = typed_unary(&mut bank, "clause_nf_g", &variable);
        let g_right_arg = typed_unary(&mut bank, "clause_nf_g", &right_arg);
        let mut first_lit = eqn(&mut bank, &f_variable, &left_replacement, true);
        oriented_demod(&mut first_lit);
        let mut second_lit = eqn(&mut bank, &g_variable, &right_replacement, true);
        oriented_demod(&mut second_lit);
        let mut first_demod = Clause::alloc(EqnList::from_vec(vec![first_lit]));
        first_demod.set_ident(101);
        first_demod.set_date(SysDate::from_raw(5));
        let mut second_demod = Clause::alloc(EqnList::from_vec(vec![second_lit]));
        second_demod.set_ident(102);
        second_demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([first_demod, second_demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_left_arg, &g_right_arg, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_prop(CP_INITIAL);
        let mut ocb = kbo_ocb(&bank);

        let steps = clause_compute_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 2);
        let rewritten = &clause.literals().as_slice()[0];
        assert_eq!(rewritten.left(), &left_replacement);
        assert_eq!(rewritten.right(), &right_replacement);
        assert!(!clause.query_prop(CP_INITIAL));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(101)),
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(102)),
            ]
        );
    }

    #[test]
    fn cached_shared_term_reuse_preserves_each_demodulator_ancestor() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let replacement = typed_const(&mut bank, "clause_cache_a");
        let argument = typed_const(&mut bank, "clause_cache_b");
        let first_rhs = typed_const(&mut bank, "clause_cache_c");
        let second_rhs = typed_const(&mut bank, "clause_cache_d");
        let f_variable = typed_unary(&mut bank, "clause_cache_f", &variable);
        let f_argument = typed_unary(&mut bank, "clause_cache_f", &argument);
        let mut demod_lit = eqn(&mut bank, &f_variable, &replacement, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_ident(201);
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut first = eqn(&mut bank, &f_argument, &first_rhs, true);
        first.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL);
        let mut second = eqn(&mut bank, &f_argument, &second_rhs, true);
        second.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
        let mut ocb = kbo_ocb(&bank);

        let steps = clause_compute_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 2);
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(201)),
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(201)),
            ]
        );
    }

    #[test]
    fn clause_li_normalform_with_docs_emits_c_side_rewrite_steps() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let left_replacement = typed_const(&mut bank, "clause_doc_nf_a");
        let left_arg = typed_const(&mut bank, "clause_doc_nf_b");
        let right_replacement = typed_const(&mut bank, "clause_doc_nf_c");
        let right_arg = typed_const(&mut bank, "clause_doc_nf_d");
        let f_variable = typed_unary(&mut bank, "clause_doc_nf_f", &variable);
        let f_left_arg = typed_unary(&mut bank, "clause_doc_nf_f", &left_arg);
        let g_variable = typed_unary(&mut bank, "clause_doc_nf_g", &variable);
        let g_right_arg = typed_unary(&mut bank, "clause_doc_nf_g", &right_arg);
        let mut first_lit = eqn(&mut bank, &f_variable, &left_replacement, true);
        oriented_demod(&mut first_lit);
        let mut second_lit = eqn(&mut bank, &g_variable, &right_replacement, true);
        oriented_demod(&mut second_lit);
        let mut first_demod = Clause::alloc(EqnList::from_vec(vec![first_lit]));
        first_demod.set_ident(101);
        first_demod.set_date(SysDate::from_raw(5));
        let mut second_demod = Clause::alloc(EqnList::from_vec(vec![second_lit]));
        second_demod.set_ident(102);
        second_demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([first_demod, second_demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_left_arg, &g_right_arg, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(7);
        clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
        let mut ocb = kbo_ocb(&bank);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 4, ProblemType::FirstOrder);
        session.pcl_shell_level = 1;
        let mut rendered = String::new();

        let steps = clause_compute_li_normalform_plain_with_docs(
            &mut rendered,
            &mut session,
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 2);
        assert_eq!(clause.ident(), 2);
        assert!(!clause.query_prop(CP_INITIAL | CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 2);
        assert_eq!(rendered, "     1 : : : rw(7,101)\n     2 : : : rw(1,102)\n");
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(101)),
                RewriteSequenceEntry::Operation(516),
                RewriteSequenceEntry::Demodulator(RewriteDemodulator::new(102)),
            ]
        );
    }

    #[test]
    fn clause_li_normalform_with_docs_preserves_c_output_level_four_gate() {
        let mut bank = test_bank();
        let variable = typed_var(&bank, -2);
        let replacement = typed_const(&mut bank, "clause_doc_gate_a");
        let arg = typed_const(&mut bank, "clause_doc_gate_b");
        let rhs = typed_const(&mut bank, "clause_doc_gate_c");
        let f_variable = typed_unary(&mut bank, "clause_doc_gate_f", &variable);
        let f_arg = typed_unary(&mut bank, "clause_doc_gate_f", &arg);
        let mut demod_lit = eqn(&mut bank, &f_variable, &replacement, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_ident(103);
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_arg, &rhs, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(7);
        clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
        let mut ocb = kbo_ocb(&bank);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 3, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let steps = clause_compute_li_normalform_plain_with_docs(
            &mut rendered,
            &mut session,
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 1);
        assert!(rendered.is_empty());
        assert_eq!(clause.ident(), 7);
        assert!(!clause.query_prop(CP_INITIAL));
        assert!(clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn clause_li_normalform_clears_limited_rewrite_after_max_side_change() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let replacement = typed_const(&mut bank, "clause_nf_limited_a");
        let arg = typed_const(&mut bank, "clause_nf_limited_b");
        let rhs = typed_const(&mut bank, "clause_nf_limited_c");
        let f_x = typed_unary(&mut bank, "clause_nf_limited_f", &x);
        let f_arg = typed_unary(&mut bank, "clause_nf_limited_f", &arg);
        let mut demod_lit = eqn(&mut bank, &f_x, &replacement, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_ident(103);
        demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_arg, &rhs, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_prop(CP_LIMITED_RW | CP_INITIAL);
        let mut ocb = kbo_ocb(&bank);

        let steps = clause_compute_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 1);
        assert!(!clause.query_prop(CP_LIMITED_RW));
        assert!(!clause.query_prop(CP_INITIAL));
        assert_eq!(clause.literals().as_slice()[0].left(), &replacement);
    }

    #[test]
    fn clause_li_normalform_sets_sos_when_demodulator_is_sos() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let replacement = typed_const(&mut bank, "clause_nf_sos_a");
        let arg = typed_const(&mut bank, "clause_nf_sos_b");
        let rhs = typed_const(&mut bank, "clause_nf_sos_c");
        let f_x = typed_unary(&mut bank, "clause_nf_sos_f", &x);
        let f_arg = typed_unary(&mut bank, "clause_nf_sos_f", &arg);
        let mut demod_lit = eqn(&mut bank, &f_x, &replacement, true);
        oriented_demod(&mut demod_lit);
        let mut demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        demod.set_ident(104);
        demod.set_date(SysDate::from_raw(5));
        demod.set_prop(CP_IS_SOS);
        let mut demod_set = ClauseSet::from_clauses([demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];
        let mut literal = eqn(&mut bank, &f_arg, &rhs, true);
        literal.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        let mut ocb = kbo_ocb(&bank);

        let steps = clause_compute_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut clause,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 1);
        assert!(clause.query_prop(CP_IS_SOS));
        assert_eq!(clause.literals().as_slice()[0].left(), &replacement);
    }

    #[test]
    fn clause_set_li_normalform_sums_steps_and_refreshes_rewritten_weights() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let first_replacement = typed_const(&mut bank, "clause_set_nf_a");
        let second_replacement = typed_const(&mut bank, "clause_set_nf_b");
        let first_arg = typed_const(&mut bank, "clause_set_nf_c");
        let second_arg = typed_const(&mut bank, "clause_set_nf_d");
        let untouched = typed_const(&mut bank, "clause_set_nf_e");
        let f_x = typed_unary(&mut bank, "clause_set_nf_f", &x);
        let f_first_arg = typed_unary(&mut bank, "clause_set_nf_f", &first_arg);
        let g_x = typed_unary(&mut bank, "clause_set_nf_g", &x);
        let g_second_arg = typed_unary(&mut bank, "clause_set_nf_g", &second_arg);

        let mut first_demod_lit = eqn(&mut bank, &f_x, &first_replacement, true);
        oriented_demod(&mut first_demod_lit);
        let mut second_demod_lit = eqn(&mut bank, &g_x, &second_replacement, true);
        oriented_demod(&mut second_demod_lit);
        let mut first_demod = Clause::alloc(EqnList::from_vec(vec![first_demod_lit]));
        first_demod.set_ident(201);
        first_demod.set_date(SysDate::from_raw(5));
        let mut second_demod = Clause::alloc(EqnList::from_vec(vec![second_demod_lit]));
        second_demod.set_ident(202);
        second_demod.set_date(SysDate::from_raw(5));
        let mut demod_set = ClauseSet::from_clauses([first_demod, second_demod]);
        demod_set.set_date(SysDate::from_raw(5));
        let demodulators = [&demod_set];

        let mut first_clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &f_first_arg,
            &untouched,
            true,
        )]));
        first_clause.set_prop(CP_INITIAL);
        first_clause.set_weight(-10);
        let first_id = first_clause.ident();
        let mut second_clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &untouched,
            &g_second_arg,
            true,
        )]));
        second_clause.set_prop(CP_INITIAL);
        second_clause.set_weight(-20);
        let second_id = second_clause.ident();
        let mut unchanged_clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank, &untouched, &untouched, true,
        )]));
        unchanged_clause.set_weight(-30);
        let unchanged_id = unchanged_clause.ident();
        let mut set = ClauseSet::from_clauses([first_clause, second_clause, unchanged_clause]);
        let mut ocb = kbo_ocb(&bank);

        let steps = clause_set_compute_li_normalform_plain(
            &mut bank,
            &mut ocb,
            &mut set,
            &demodulators,
            RewriteLevel::RuleRewrite,
            false,
            false,
        )
        .unwrap();

        assert_eq!(steps, 2);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id, unchanged_id]
        );
        let first = set.find_by_id(first_id).unwrap();
        assert_eq!(first.literals().as_slice()[0].left(), &first_replacement);
        assert_eq!(first.weight(), first.standard_weight());
        assert!(!first.query_prop(CP_INITIAL));
        let second = set.find_by_id(second_id).unwrap();
        assert_eq!(second.literals().as_slice()[0].right(), &second_replacement);
        assert_eq!(second.weight(), second.standard_weight());
        assert!(!second.query_prop(CP_INITIAL));
        let unchanged = set.find_by_id(unchanged_id).unwrap();
        assert_eq!(unchanged.weight(), -30);
        assert_eq!(unchanged.literals().as_slice()[0].left(), &untouched);
    }

    #[test]
    fn restricted_max_side_ignores_limited_renaming_rewrites() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let f_x = typed_unary(&mut bank, "bwrw_limited_f", &x);
        let g_x = typed_unary(&mut bank, "bwrw_limited_g", &x);
        let f_y = typed_unary(&mut bank, "bwrw_limited_f", &y);
        let c = typed_const(&mut bank, "bwrw_limited_c");
        let mut demod_lit = eqn(&mut bank, &f_x, &g_x, true);
        oriented_demod(&mut demod_lit);
        let demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        let mut target_lit = eqn(&mut bank, &f_y, &c, true);
        target_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL);
        let mut ocb = kbo_ocb(&bank);

        let side = eqn_has_rw_side(
            &mut bank,
            &mut ocb,
            &target_lit,
            &demod,
            SysDate::from_raw(8),
        )
        .unwrap();

        assert_eq!(side, EqnSide::NoSide);
        assert!(f_y.is_top_rewritten());
        assert!(!f_y.is_rrewritten());
    }

    #[test]
    fn strong_rhs_instantiation_completes_unbound_rhs_variables() {
        let _counter_guard = reset_backward_rewrite_counters();
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let y = typed_var(&bank, -4);
        let a = typed_const(&mut bank, "bwrw_strong_a");
        let f_x = typed_unary(&mut bank, "bwrw_strong_f", &x);
        let f_a = typed_unary(&mut bank, "bwrw_strong_f", &a);
        let c = typed_const(&mut bank, "bwrw_strong_c");
        let demod = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_x, &y, true)]));
        let target = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &f_a, &c, true)]));
        let set = ClauseSet::from_clauses([target]);
        let mut ocb = kbo_ocb(&bank);
        let mut results = Vec::new();

        assert!(!find_rewritable_clauses(
            &mut bank,
            &mut ocb,
            &set,
            &mut results,
            &demod,
            SysDate::from_raw(9),
        )
        .unwrap());
        assert!(REWRITE_UNBOUND_VAR_FAILS.load(Ordering::Relaxed) > 0);
        assert!(results.is_empty());

        let mut strong_ocb = kbo_ocb(&bank);
        strong_ocb.rewrite_strong_rhs_inst = true;
        let mut strong_results = Vec::new();

        assert!(find_rewritable_clauses(
            &mut bank,
            &mut strong_ocb,
            &set,
            &mut strong_results,
            &demod,
            SysDate::from_raw(10),
        )
        .unwrap());

        assert_eq!(strong_results.len(), 1);
        assert!(f_a.is_top_rewritten());
        assert!(f_a.rw_replace_field().is_some());
        assert!(y.binding().is_none());
    }

    #[test]
    fn non_rewritable_terms_record_rule_and_full_nf_dates() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "bwrw_nf_a");
        let b = typed_const(&mut bank, "bwrw_nf_b");
        let c = typed_const(&mut bank, "bwrw_nf_c");
        let f_x = typed_unary(&mut bank, "bwrw_nf_f", &x);
        let g_b = typed_unary(&mut bank, "bwrw_nf_g", &b);
        let mut demod_lit = eqn(&mut bank, &f_x, &a, true);
        oriented_demod(&mut demod_lit);
        let demod = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
        let target_lit = eqn(&mut bank, &g_b, &c, true);
        let mut ocb = kbo_ocb(&bank);
        let nf_date = SysDate::from_raw(11);

        assert_eq!(
            eqn_has_rw_side(&mut bank, &mut ocb, &target_lit, &demod, nf_date).unwrap(),
            EqnSide::NoSide
        );

        assert_eq!(g_b.nf_date(RewriteLevel::RuleRewrite), nf_date);
        assert_eq!(g_b.nf_date(RewriteLevel::FullRewrite), nf_date);
    }
}
