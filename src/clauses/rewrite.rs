use crate::basics::error::Diagnostic;
use crate::basics::sysdate::SysDate;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_IS_ORIENTED;
use crate::clauses::clausefunc::clause_remove_superfluous_literals;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{
    EqnSide, EP_IS_EQU_LITERAL, EP_IS_ORIENTED, EP_IS_POSITIVE, EP_MAX_IS_UP_TO_DATE, MAX_SIDE,
    MIN_SIDE,
};
use crate::orderings::cto_orderings::to_greater;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::replace::{term_add_rw_link, RwResultType};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::term_has_unbound_variables;
use crate::terms::termtypes::{
    term_identity_id, DerefType, RewriteDemodulator, RewriteLevel, Term, TP_IS_REWRITABLE,
    TP_IS_REWRITTEN, TP_IS_RREWRITABLE, TP_IS_RREWRITTEN,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

type LocalRwSystem = HashMap<usize, Term>;

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
            add_top_rewrite_link_if_needed(
                bank,
                term,
                eqn.right(),
                new_demod.is_sos(),
                demodulator,
                result,
            )?;
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
                add_top_rewrite_link_if_needed(
                    bank,
                    term,
                    eqn.left(),
                    new_demod.is_sos(),
                    demodulator,
                    result,
                )?;
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
) -> Result<(), Diagnostic> {
    if term.is_rewritten() && result != RwResultType::AlwaysRewritable {
        return Ok(());
    }

    let replacement = bank.insert_instantiated(replacement_pattern)?;
    if replacement == *term {
        term.del_prop(TP_IS_REWRITABLE | TP_IS_RREWRITABLE);
    } else {
        term_add_rw_link(term, &replacement, Some(demodulator), sos, result);
        REWRITE_UNCACHED.fetch_add(1, Ordering::Relaxed);
    }
    Ok(())
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
        clause_local_rw, eqn_has_rw_side, find_rewritable_clauses, BWRW_MATCH_ATTEMPTS,
        BWRW_MATCH_SUCCESSES, REWRITE_UNBOUND_VAR_FAILS,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_IS_ORIENTED;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EqnSide, EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_POSITIVE};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
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
