use crate::basics::error::Diagnostic;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::clausefunc::clause_remove_superfluous_literals;
use crate::clauses::clausesets::{clause_set_list_get_max_date, ClauseSet};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EqnSide, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE, MAX_SIDE,
    MIN_SIDE,
};
use crate::clauses::subterm_index::SubtermIndex;
use crate::clauses::subterm_tree::SubtermOcc;
use crate::orderings::cto_orderings::to_greater;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::replace::{term_add_rw_link, term_follow_top_rw_chain, RwResultType};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_has_unbound_variables, term_is_db_closed};
use crate::terms::termtypes::{
    term_identity_id, DerefType, RewriteDemodulator, RewriteLevel, Term, TP_IS_REWRITABLE,
    TP_IS_REWRITTEN, TP_IS_RREWRITABLE, TP_IS_RREWRITTEN,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};

type LocalRwSystem = HashMap<usize, Term>;

pub static REWRITE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_SUCCESSES: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_UNBOUND_VAR_FAILS: AtomicU64 = AtomicU64::new(0);
pub static REWRITE_UNCACHED: AtomicU64 = AtomicU64::new(0);
pub static BWRW_MATCH_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
pub static BWRW_MATCH_SUCCESSES: AtomicU64 = AtomicU64::new(0);

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
    clause.orient_literals(ocb, bank);

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
    index: &'idx SubtermIndex<'_>,
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
    _prefer_general: bool,
    restricted_rw: bool,
) -> Result<Term, Diagnostic> {
    assert!(!term.is_free_var(), "free variables are not rewritten");
    assert!(
        !term.is_top_rewritten(),
        "top-level rewrite expects no existing top rewrite link"
    );

    REWRITE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    let mut subst = Substitution::new();
    let Some(found) = find_plain_demodulator(
        ocb,
        bank,
        term,
        date,
        demodulators,
        &mut subst,
        restricted_rw,
    )?
    else {
        return Ok(term.clone());
    };

    REWRITE_SUCCESSES.fetch_add(1, Ordering::Relaxed);
    let replacement = bank.insert_instantiated(found.replacement)?;
    subst.backtrack();

    if replacement == *term {
        return Ok(term.clone());
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
    Ok(replacement)
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
    let mut result = term.clone();
    for demodulator_set in demodulators.iter().take(level_count) {
        if term_is_db_closed(term)
            && term
                .nf_date(date_level)
                .is_earlier_than(demodulator_set.date())
        {
            result = rewrite_with_clause_set_plain(
                bank,
                ocb,
                term,
                term.nf_date(date_level),
                demodulator_set,
                prefer_general,
                restricted_rw,
            )?;
            if result != *term {
                break;
            }
        }
    }
    Ok(result)
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
) -> Result<Term, Diagnostic> {
    debug_assert_ne!(level, RewriteLevel::NoRewrite);

    let (mut current, _) = term_follow_top_rw_chain(term, restricted_rw);
    assert!(
        !current.is_top_rewritten() || restricted_rw,
        "unrestricted normal-form traversal must follow top rewrite links"
    );

    let date_level = rewrite_date_level(level);
    if !current.is_rewritten() && !current.nf_date(date_level).is_earlier_than(demod_date) {
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
        )?;

        if !current.is_free_var() {
            let follow_restricted = restricted_rw && !modified;
            let (new_term, _) = if current.is_top_rewritten() {
                term_follow_top_rw_chain(&current, follow_restricted)
            } else {
                let _ = rewrite_with_clause_set_list_plain(
                    bank,
                    ocb,
                    &current,
                    demodulators,
                    level,
                    prefer_general,
                    follow_restricted,
                )?;
                term_follow_top_rw_chain(&current, follow_restricted)
            };
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

#[expect(
    clippy::too_many_arguments,
    reason = "Recursive subterm rewrite mirrors C term_subterm_rewrite context"
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
) -> Result<bool, Diagnostic> {
    if !lambda_demod && term.is_lambda() {
        return Ok(false);
    }

    let new_term = Term::top_copy_without_args(term);
    let mut modified = false;
    for (index, arg) in term.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
        let normalized = term_li_normalform_plain_with_date(
            bank,
            ocb,
            &arg,
            demodulators,
            level,
            demod_date,
            prefer_general,
            false,
            lambda_demod,
        )?;
        if normalized != arg {
            modified = true;
        }
        new_term.set_argument(index, normalized);
    }

    if modified {
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
    }

    Ok(modified)
}

/// Compute plain leftmost-innermost normal forms for an equation's sides.
///
/// This mirrors C `eqn_li_normalform` for term mutation, maximality-cache
/// invalidation, equality-literal normalization when the right side becomes
/// `$true`, and the returned side mask. Derivation recording remains tied to
/// the later proof-object port.
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
    let left_old = eqn.left().clone();
    let right_old = eqn.right().clone();
    let restricted_rw = eqn.is_maximal() && eqn.is_positive() && eqn.is_oriented() && interred_rw;
    let mut result = EqnSide::NoSide;

    let left_new = term_li_normalform_plain(
        bank,
        ocb,
        &left_old,
        demodulators,
        level,
        prefer_general,
        restricted_rw,
        lambda_demod,
    )?;
    if left_new != left_old {
        eqn.set_left_raw(left_new);
        eqn.del_prop(EP_MAX_IS_UP_TO_DATE);
        result = MAX_SIDE;
    }

    let right_new = term_li_normalform_plain(
        bank,
        ocb,
        &right_old,
        demodulators,
        level,
        prefer_general,
        false,
        lambda_demod,
    )?;
    if right_new != right_old {
        eqn.set_right_raw(right_new);
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
    for clause in demodulators.iter() {
        if !clause.is_demodulator() {
            continue;
        }
        if !date.is_earlier_than(clause.date()) {
            continue;
        }

        let eqn = clause
            .literals()
            .as_slice()
            .first()
            .expect("positive unit demodulator has one literal");
        if demodulator_date_blocks_term(term, clause, eqn) {
            continue;
        }

        let backtrack = subst.len();
        if subst_match_complete(eqn.left(), term, subst)
            && (eqn.is_oriented() || instance_is_rule(ocb, bank, eqn.left(), eqn.right(), subst)?)
            && (!restricted_rw || !subst.is_renaming())
        {
            return Ok(Some(PlainDemodulatorMatch {
                clause,
                replacement: eqn.right(),
            }));
        }
        subst.backtrack_to_pos(backtrack);

        if !eqn.is_oriented() {
            let backtrack = subst.len();
            if subst_match_complete(eqn.right(), term, subst)
                && instance_is_rule(ocb, bank, eqn.right(), eqn.left(), subst)?
            {
                return Ok(Some(PlainDemodulatorMatch {
                    clause,
                    replacement: eqn.left(),
                }));
            }
            subst.backtrack_to_pos(backtrack);
        }
    }
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
    index: &'idx SubtermIndex<'_>,
    results: &mut Vec<&'idx Clause>,
    seen: &mut BTreeSet<usize>,
    new_demod: &Clause,
    left: &Term,
    right: &Term,
    oriented: bool,
    _nf_date: SysDate,
) -> Result<i64, Diagnostic> {
    let mut occurrences = Vec::new();
    index.collect_matchable_occurrences(left, &mut occurrences);
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
    seen: &mut BTreeSet<usize>,
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
    if subst_match_complete(left, occurrence.term(), &mut subst) {
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
    seen: &mut BTreeSet<usize>,
    clauses: &'idx BTreeMap<usize, Clause>,
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
    if subst_match_complete(eqn.left(), term, &mut subst) {
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
        if subst_match_complete(eqn.right(), term, &mut subst) {
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

    let replacement = bank.insert_instantiated(replacement_pattern)?;
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

    Ok(to_greater(
        ocb,
        bank.signature(),
        lside,
        rside,
        DerefType::Once,
        DerefType::Once,
    ))
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
    RewriteDemodulator::new(id.max(1))
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
        clause_local_rw, eqn_has_rw_side, eqn_li_normalform_plain, find_rewritable_clauses,
        find_rewritable_clauses_indexed, rewrite_with_clause_set_list_plain,
        rewrite_with_clause_set_plain, term_li_normalform_plain, BWRW_MATCH_ATTEMPTS,
        BWRW_MATCH_SUCCESSES, REWRITE_ATTEMPTS, REWRITE_SUCCESSES, REWRITE_UNBOUND_VAR_FAILS,
        REWRITE_UNCACHED,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EqnSide, EP_IS_EQU_LITERAL, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_POSITIVE,
        EP_MAX_IS_UP_TO_DATE,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::subterm_index::SubtermIndex;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::idx_fp::index_fp1_create;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, RewriteLevel, Term, TP_IS_REWRITABLE};
    use crate::terms::typebanks::TypeBank;
    use std::sync::atomic::Ordering;

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

    fn reset_backward_rewrite_counters() {
        REWRITE_ATTEMPTS.store(0, Ordering::Relaxed);
        REWRITE_SUCCESSES.store(0, Ordering::Relaxed);
        REWRITE_UNCACHED.store(0, Ordering::Relaxed);
        BWRW_MATCH_ATTEMPTS.store(0, Ordering::Relaxed);
        BWRW_MATCH_SUCCESSES.store(0, Ordering::Relaxed);
        REWRITE_UNBOUND_VAR_FAILS.store(0, Ordering::Relaxed);
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
    }

    #[test]
    fn plain_backward_rewrite_scan_links_matching_child_terms() {
        reset_backward_rewrite_counters();
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
        reset_backward_rewrite_counters();
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
        let index_sig = bank.signature().clone();
        let mut index = SubtermIndex::new(index_fp1_create, &index_sig);
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
        let index_sig = bank.signature().clone();
        let mut index = SubtermIndex::new(index_fp1_create, &index_sig);
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
    fn plain_clause_set_rewrite_links_first_matching_demodulator() {
        reset_backward_rewrite_counters();
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

        assert_eq!(rewritten, a);
        assert_eq!(f_b.rw_replace_field(), Some(a));
        assert!(f_b.is_top_rewritten());
        assert!(!f_b.is_rrewritten());
        assert!(REWRITE_ATTEMPTS.load(Ordering::Relaxed) >= 1);
        assert!(REWRITE_SUCCESSES.load(Ordering::Relaxed) >= 1);
        assert!(REWRITE_UNCACHED.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn plain_clause_set_rewrite_respects_normal_form_dates() {
        reset_backward_rewrite_counters();
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
        reset_backward_rewrite_counters();
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
        reset_backward_rewrite_counters();
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
