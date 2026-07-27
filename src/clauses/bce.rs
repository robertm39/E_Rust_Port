use crate::basics::error::Diagnostic;
use crate::basics::min_heap::MinHeap;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::ClauseDerivationRef;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::clauses::tautologies::clause_is_tautology_real;
use crate::terms::functypes::FunCode;
use crate::terms::match_mgu::subst_mgu_complete_with_bank;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::DerefType;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

type SymbolMap = BTreeMap<FunCode, Vec<ClauseDerivationRef>>;
type BceTaskCmp = fn(&BceTask, &BceTask) -> Ordering;
type BceTaskQueue = MinHeap<BceTask, BceTaskCmp>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BceEliminationResult {
    pub start_count: i64,
    pub eliminated_count: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BceTask {
    orig_ref: ClauseDerivationRef,
    parent_index: usize,
    lit_index: usize,
    candidates: Option<Vec<ClauseDerivationRef>>,
    processed_cands: usize,
}

impl BceTask {
    fn remaining_candidates(&self) -> usize {
        self.candidates
            .as_ref()
            .map_or(0, |candidates| candidates.len() - self.processed_cands)
    }
}

/// Eliminates blocked clauses by moving them from `passive` to `archive`.
///
/// This ports the clause-level `EliminateBlockedClauses` algorithm. The source
/// `bank` is used for disjoint parent copies and temporary L-resolvent
/// construction, while `tmp_bank` is used by the ground-completion tautology
/// check, matching the C temporary-bank split.
///
/// # Errors
///
/// Returns a diagnostic if a disjoint copy, temporary L-resolvent literal, or
/// tautology check cannot be constructed with the supplied term banks.
pub fn eliminate_blocked_clauses(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    max_occs: i32,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<BceEliminationResult, Diagnostic> {
    let start_count = passive.members();
    let mut eq_found = false;
    let occurrence_limit = usize::try_from(max_occs).ok().filter(|limit| *limit > 0);
    let sym_occs = make_sym_map(passive, bank, occurrence_limit, &mut eq_found);
    let (mut task_queue, fresh_clauses) = make_bce_queue(passive, bank, &sym_occs)?;
    let eliminated_count = do_eliminate_clauses(
        &mut task_queue,
        passive,
        archive,
        &fresh_clauses,
        eq_found,
        bank,
        tmp_bank,
    )?;

    Ok(BceEliminationResult {
        start_count,
        eliminated_count,
    })
}

/// Eliminates blocked clauses and writes the C `EliminateBlockedClauses`
/// progress lines.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`eliminate_blocked_clauses`], or a formatting error if `output` rejects a
/// write.
pub fn eliminate_blocked_clauses_with_output(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    max_occs: i32,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    output: &mut impl fmt::Write,
) -> Result<BceEliminationResult, Diagnostic> {
    let start_count = passive.members();
    writeln!(output, "% BCE start: {start_count}").map_err(bce_write_error)?;
    let result = eliminate_blocked_clauses(passive, archive, max_occs, bank, tmp_bank)?;
    writeln!(output, "% BCE eliminated: {}.", result.eliminated_count).map_err(bce_write_error)?;
    Ok(result)
}

fn make_sym_map(
    set: &ClauseSet,
    bank: &TermBank,
    occ_limit: Option<usize>,
    eq_found: &mut bool,
) -> SymbolMap {
    let mut result = BTreeMap::new();
    for clause in set.iter() {
        for literal in clause.literals().as_slice() {
            if literal.is_equ_lit(bank) {
                *eq_found = true;
                continue;
            }

            let f_code = signed_pred_code(literal);
            if is_blocked_stack(result.get(&f_code)) {
                continue;
            }

            let same_count = occ_count(result.get(&f_code));
            let opposite_count = occ_count(result.get(&-f_code));
            if occ_limit.is_some_and(|limit| same_count + opposite_count >= limit) {
                result.entry(f_code).or_default().clear();
                result.entry(-f_code).or_default().clear();
            } else {
                let clauses = result.entry(f_code).or_default();
                let clause_ref = ClauseDerivationRef::from(clause);
                if clauses.last().copied() != Some(clause_ref) {
                    clauses.push(clause_ref);
                }
            }
        }
    }
    result
}

fn make_bce_queue(
    set: &ClauseSet,
    bank: &mut TermBank,
    sym_map: &SymbolMap,
) -> Result<(BceTaskQueue, Vec<Clause>), Diagnostic> {
    let mut task_queue = MinHeap::new(compare_tasks as fn(&BceTask, &BceTask) -> Ordering);
    let mut fresh_clauses = Vec::new();

    for clause in set.iter() {
        let parent_index = fresh_clauses.len();
        fresh_clauses.push(clause.copy_disjoint(bank)?);
        for (lit_index, literal) in fresh_clauses[parent_index]
            .literals()
            .as_slice()
            .iter()
            .enumerate()
        {
            if literal.is_equ_lit(bank) {
                continue;
            }

            let f_code = signed_pred_code(literal);
            let candidates = sym_map.get(&-f_code);
            if is_blocked_stack(candidates) {
                continue;
            }
            task_queue.add_ptr(BceTask {
                orig_ref: ClauseDerivationRef::from(clause),
                parent_index,
                lit_index,
                candidates: candidates.cloned(),
                processed_cands: 0,
            });
        }
    }

    Ok((task_queue, fresh_clauses))
}

fn do_eliminate_clauses(
    task_queue: &mut BceTaskQueue,
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    fresh_clauses: &[Clause],
    has_eq: bool,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<i64, Diagnostic> {
    let mut blocker_map: BTreeMap<ClauseDerivationRef, Vec<BceTask>> = BTreeMap::new();
    let mut archived = BTreeSet::new();
    let mut eliminated = 0;

    while let Some(mut min_task) = task_queue.pop_min() {
        if archived.contains(&min_task.orig_ref)
            || passive.find_by_derivation_ref(min_task.orig_ref).is_none()
        {
            continue;
        }

        check_candidates(
            &mut min_task,
            passive,
            &archived,
            fresh_clauses,
            has_eq,
            bank,
            tmp_bank,
        )?;
        if min_task
            .candidates
            .as_ref()
            .is_none_or(|candidates| min_task.processed_cands == candidates.len())
        {
            if let Some(clause) = passive.extract_by_derivation_ref(min_task.orig_ref) {
                archived.insert(min_task.orig_ref);
                archive.insert(clause);
                eliminated += 1;
            }
            if let Some(blocked) = blocker_map.remove(&min_task.orig_ref) {
                for mut task in blocked.into_iter().rev() {
                    task.processed_cands += 1;
                    task_queue.add_ptr(task);
                }
            }
        } else if let Some(candidates) = &min_task.candidates {
            let offending = candidates[min_task.processed_cands];
            blocker_map.entry(offending).or_default().push(min_task);
        }
    }

    Ok(eliminated)
}

fn check_candidates(
    task: &mut BceTask,
    passive: &ClauseSet,
    archived: &BTreeSet<ClauseDerivationRef>,
    fresh_clauses: &[Clause],
    has_eq: bool,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    let Some(candidates) = &task.candidates else {
        return Ok(());
    };

    while task.processed_cands < candidates.len() {
        let candidate_ref = candidates[task.processed_cands];
        let candidate_blocks =
            if candidate_ref == task.orig_ref || archived.contains(&candidate_ref) {
                false
            } else if let Some(candidate) = passive.find_by_derivation_ref(candidate_ref) {
                !check_blockedness(task, candidate, fresh_clauses, has_eq, bank, tmp_bank)?
            } else {
                false
            };

        if candidate_blocks {
            break;
        }
        task.processed_cands += 1;
    }

    Ok(())
}

fn check_blockedness(
    task: &BceTask,
    partner: &Clause,
    fresh_clauses: &[Clause],
    has_eq: bool,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    if has_eq {
        check_blockedness_eq(task, partner, fresh_clauses, bank, tmp_bank)
    } else {
        check_blockedness_neq(task, partner, fresh_clauses, bank)
    }
}

fn check_blockedness_neq(
    task: &BceTask,
    partner: &Clause,
    fresh_clauses: &[Clause],
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let parent = &fresh_clauses[task.parent_index];
    let lit = &parent.literals().as_slice()[task.lit_index];
    debug_assert!(!lit.is_equ_lit(bank));

    let (unifiable, nonunifiable) = split_partner_literals(lit, partner, bank)?;
    let mut result = true;
    for index in 0..unifiable.len() {
        let mut processed = vec![index];
        result = check_l_resolvents_neq(
            parent,
            task.lit_index,
            partner,
            &unifiable,
            &nonunifiable,
            &mut processed,
            bank,
        )?;
        if !result {
            break;
        }
    }

    Ok(result)
}

fn split_partner_literals(
    lit: &Eqn,
    partner: &Clause,
    bank: &mut TermBank,
) -> Result<(Vec<usize>, Vec<usize>), Diagnostic> {
    let mut unifiable = Vec::new();
    let mut nonunifiable = Vec::new();
    let mut subst = Substitution::new();

    for (index, partner_lit) in partner.literals().as_slice().iter().enumerate() {
        let unified = if lit.is_positive() != partner_lit.is_positive()
            && !partner_lit.is_equ_lit(bank)
        {
            match subst_mgu_complete_with_bank(bank, lit.left(), partner_lit.left(), &mut subst) {
                Ok(unified) => unified,
                Err(error) => {
                    subst.backtrack();
                    return Err(error);
                }
            }
        } else {
            false
        };
        if unified {
            unifiable.push(index);
            subst.backtrack();
        } else {
            nonunifiable.push(index);
        }
    }
    subst.delete();

    Ok((unifiable, nonunifiable))
}

fn check_l_resolvents_neq(
    parent: &Clause,
    lit_index: usize,
    partner: &Clause,
    unifiable: &[usize],
    nonunifiable: &[usize],
    processed: &mut Vec<usize>,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    debug_assert!(!processed.is_empty());
    let lit = &parent.literals().as_slice()[lit_index];
    let partner_literals = partner.literals().as_slice();
    let mut subst = Substitution::new();
    let first_lit =
        &partner_literals[unifiable[*processed.last().expect("processed is non-empty")]];
    let unified = match subst_mgu_complete_with_bank(bank, lit.left(), first_lit.left(), &mut subst)
    {
        Ok(unified) => unified,
        Err(error) => {
            subst.backtrack();
            return Err(error);
        }
    };
    debug_assert!(unified);

    let mut result = false;
    loop {
        let comp_found = nonunifiable.iter().any(|&index| {
            parent.literals().find_comp_lit_except(
                Some(lit_index),
                &partner_literals[index],
                DerefType::Always,
                DerefType::Always,
            )
        });
        if comp_found {
            result = true;
            break;
        }

        if processed.len() == unifiable.len() {
            break;
        }

        let prev_try = processed.len();
        for index in 0..unifiable.len() {
            if !processed.contains(&index)
                && parent.literals().find_comp_lit_except(
                    Some(lit_index),
                    &partner_literals[unifiable[index]],
                    DerefType::Always,
                    DerefType::Always,
                )
            {
                processed.push(index);
            }
        }

        if prev_try == processed.len() {
            break;
        }

        let mut unifiable_group = true;
        for processed_index in processed.iter().skip(prev_try) {
            let other = &partner_literals[unifiable[*processed_index]];
            match subst_mgu_complete_with_bank(bank, lit.left(), other.left(), &mut subst) {
                Ok(true) => {}
                Ok(false) => {
                    unifiable_group = false;
                    break;
                }
                Err(error) => {
                    subst.backtrack();
                    return Err(error);
                }
            }
        }

        if !unifiable_group {
            result = true;
            break;
        }
    }

    subst.delete();
    Ok(result)
}

fn check_blockedness_eq(
    task: &BceTask,
    partner: &Clause,
    fresh_clauses: &[Clause],
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    let parent = &fresh_clauses[task.parent_index];
    let lit = &parent.literals().as_slice()[task.lit_index];
    debug_assert!(!lit.is_equ_lit(bank));
    let mut same_head = Vec::new();
    let mut others = EqnList::new();

    for (index, partner_lit) in partner.literals().as_slice().iter().enumerate() {
        if lit.is_positive() != partner_lit.is_positive()
            && !partner_lit.is_equ_lit(bank)
            && lit.left().f_code() == partner_lit.left().f_code()
        {
            same_head.push(index);
        } else {
            others.insert_first(partner_lit.copy_to_bank(bank)?);
        }
    }

    while let Some(index) = same_head.pop() {
        let partner_lit = &partner.literals().as_slice()[index];
        let mut cond = EqnList::new();
        for arg_index in 0..partner_lit.left().arity() {
            let partner_arg = partner_lit
                .left()
                .argument(arg_index)
                .unwrap_or_else(|| panic!("predicate argument {arg_index} is initialized"));
            let task_arg = lit
                .left()
                .argument(arg_index)
                .unwrap_or_else(|| panic!("predicate argument {arg_index} is initialized"));
            cond.insert_first(Eqn::alloc(partner_arg, task_arg, bank, false)?);
        }
        cond.append(
            parent
                .literals()
                .copy_except_index(Some(task.lit_index), bank)?,
        );
        cond.append(others.copy_to_bank(bank)?);
        let tmp_clause = Clause::alloc(cond);
        if !clause_is_tautology_real(tmp_bank, &tmp_clause, false)? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn compare_tasks(left: &BceTask, right: &BceTask) -> Ordering {
    left.remaining_candidates()
        .cmp(&right.remaining_candidates())
}

fn signed_pred_code(literal: &Eqn) -> FunCode {
    literal.left().f_code() * if literal.is_positive() { 1 } else { -1 }
}

fn is_blocked_stack(stack: Option<&Vec<ClauseDerivationRef>>) -> bool {
    stack.is_some_and(Vec::is_empty)
}

fn occ_count(stack: Option<&Vec<ClauseDerivationRef>>) -> usize {
    stack.map_or(0, Vec::len)
}

fn bce_write_error(error: fmt::Error) -> Diagnostic {
    Diagnostic::new(
        crate::basics::error::ErrorCode::SYSTEM_ERROR,
        format!("Could not write BCE output: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        eliminate_blocked_clauses, eliminate_blocked_clauses_with_output, split_partner_literals,
        BceEliminationResult,
    };
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::ClauseDerivationRef;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::lambda::apply_terms;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
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
        set_problem_type(problem_type).unwrap();
        ProblemTypeReset
    }

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn tmp_bank(bank: &TermBank) -> TermBank {
        TermBank::new(bank.signature().clone()).unwrap()
    }

    fn object_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn predicate_atom(bank: &mut TermBank, name: &str, args: &[Term]) -> Term {
        let i_type = bank.signature().type_bank().i_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let final_type = if args.is_empty() {
            bool_type.clone()
        } else {
            let mut type_args = vec![i_type; args.len()];
            type_args.push(bool_type.clone());
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(type_args))
        };
        let f_code =
            bank.signature_mut()
                .insert_id(name, i32::try_from(args.len()).unwrap(), false);
        bank.signature_mut()
            .declare_final_type(f_code, final_type)
            .unwrap();
        let term = Term::top_alloc(f_code, args.len());
        term.set_type(Some(bool_type));
        for (index, arg) in args.iter().enumerate() {
            term.set_argument(index, arg.clone());
        }
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term, positive: bool) -> Eqn {
        let true_term = bank.true_term().clone();
        Eqn::alloc(atom.clone(), true_term, bank, positive).unwrap()
    }

    fn equation_literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn ids(set: &ClauseSet) -> Vec<i64> {
        set.iter().map(Clause::ident).collect()
    }

    fn refs(set: &ClauseSet) -> Vec<ClauseDerivationRef> {
        set.iter().map(ClauseDerivationRef::from).collect()
    }

    #[test]
    fn bce_moves_clause_with_no_opposite_candidates() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_no_opp_a");
        let p_a = predicate_atom(&mut bank, "bce_no_opp_p", &[a]);
        let unit = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let unit_id = unit.ident();
        let mut passive = ClauseSet::from_clauses([unit]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, -1, &mut bank, &mut tmp).unwrap();

        assert_eq!(
            result,
            BceEliminationResult {
                start_count: 1,
                eliminated_count: 1
            }
        );
        assert!(passive.is_empty());
        assert_eq!(ids(&archive), vec![unit_id]);
    }

    #[test]
    fn bce_keeps_opposite_unit_pair_with_non_tautological_resolvent() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_pair_a");
        let p_a = predicate_atom(&mut bank, "bce_pair_p", &[a]);
        let positive = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let positive_id = positive.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, -1, &mut bank, &mut tmp).unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert_eq!(ids(&passive), vec![positive_id, negative_id]);
        assert!(archive.is_empty());
    }

    #[test]
    fn bce_distinguishes_same_id_clause_generations() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_same_id_a");
        let p_a = predicate_atom(&mut bank, "bce_same_id_p", &[a]);
        let mut positive = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let mut negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        positive.set_ident(41);
        positive.refresh_derivation_generation();
        negative.set_ident(41);
        negative.refresh_derivation_generation();
        let positive_ref = ClauseDerivationRef::from(&positive);
        let negative_ref = ClauseDerivationRef::from(&negative);
        let mut passive = ClauseSet::from_clauses([positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, -1, &mut bank, &mut tmp).unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert!(refs(&passive).contains(&positive_ref));
        assert!(refs(&passive).contains(&negative_ref));
        assert!(archive.is_empty());
    }

    #[test]
    fn bce_partner_split_uses_banked_higher_order_mgu() {
        let _global_state = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut bank = test_bank();
        let individual = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let unary_predicate =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    bool_type.clone(),
                ]));
        let binary_predicate =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    individual.clone(),
                    individual,
                    bool_type,
                ]));
        let function = bank.vars().get_fresh_var(&unary_predicate);
        let prefix = object_const(&mut bank, "bce_ho_prefix");
        let suffix = object_const(&mut bank, "bce_ho_suffix");
        let rigid_code = bank.signature_mut().insert_id("bce_ho_rigid", 0, false);
        bank.signature_mut()
            .declare_final_type(rigid_code, binary_predicate)
            .unwrap();
        let rigid = bank.create_const_term(rigid_code).unwrap();
        let flex_application =
            apply_terms(&mut bank, &function, std::slice::from_ref(&suffix)).unwrap();
        let rigid_application = apply_terms(&mut bank, &rigid, &[prefix, suffix]).unwrap();
        let literal = predicate_literal(&mut bank, &flex_application, true);
        let partner = clause(vec![predicate_literal(
            &mut bank,
            &rigid_application,
            false,
        )]);

        let (unifiable, nonunifiable) =
            split_partner_literals(&literal, &partner, &mut bank).unwrap();

        assert_eq!(unifiable, vec![0]);
        assert!(nonunifiable.is_empty());
        assert!(function.binding().is_none());
    }

    #[test]
    fn bce_eliminates_when_all_non_equational_l_resolvents_are_tautologies() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_taut_a");
        let p_a = predicate_atom(&mut bank, "bce_taut_p", std::slice::from_ref(&a));
        let q_a = predicate_atom(&mut bank, "bce_taut_q", &[a]);
        let first = clause(vec![
            predicate_literal(&mut bank, &p_a, true),
            predicate_literal(&mut bank, &q_a, true),
        ]);
        let second = clause(vec![
            predicate_literal(&mut bank, &p_a, false),
            predicate_literal(&mut bank, &q_a, false),
        ]);
        let mut passive = ClauseSet::from_clauses([first, second]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, -1, &mut bank, &mut tmp).unwrap();

        assert_eq!(result.eliminated_count, 2);
        assert!(passive.is_empty());
        assert_eq!(archive.len(), 2);
    }

    #[test]
    fn bce_occurrence_limit_blocks_tracking_in_both_polarities() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_limit_a");
        let p_a = predicate_atom(&mut bank, "bce_limit_p", &[a]);
        let positive = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let positive_id = positive.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, 1, &mut bank, &mut tmp).unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert_eq!(ids(&passive), vec![positive_id, negative_id]);
        assert!(archive.is_empty());
    }

    #[test]
    fn bce_equational_checker_uses_tautological_l_resolvents() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_eq_a");
        let p_a = predicate_atom(&mut bank, "bce_eq_p", std::slice::from_ref(&a));
        let first = clause(vec![
            predicate_literal(&mut bank, &p_a, true),
            equation_literal(&mut bank, &a, &a, true),
        ]);
        let second = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let mut passive = ClauseSet::from_clauses([first, second]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let mut output = String::new();

        let result = eliminate_blocked_clauses_with_output(
            &mut passive,
            &mut archive,
            -1,
            &mut bank,
            &mut tmp,
            &mut output,
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 2);
        assert!(passive.is_empty());
        assert_eq!(archive.len(), 2);
        assert_eq!(output, "% BCE start: 2\n% BCE eliminated: 2.\n");
    }

    #[test]
    fn bce_equational_checker_rehomes_predicate_truth_terms() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "bce_eq_pred_a");
        let b = object_const(&mut bank, "bce_eq_pred_b");
        let p_a = predicate_atom(&mut bank, "bce_eq_pred_p", std::slice::from_ref(&a));
        let q_a = predicate_atom(&mut bank, "bce_eq_pred_q", std::slice::from_ref(&a));
        let equality = clause(vec![equation_literal(&mut bank, &a, &b, true)]);
        let positive = clause(vec![
            predicate_literal(&mut bank, &p_a, true),
            predicate_literal(&mut bank, &q_a, true),
        ]);
        let negative = clause(vec![
            predicate_literal(&mut bank, &p_a, false),
            predicate_literal(&mut bank, &q_a, false),
        ]);
        let mut passive = ClauseSet::from_clauses([equality, positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);

        let result =
            eliminate_blocked_clauses(&mut passive, &mut archive, -1, &mut bank, &mut tmp).unwrap();

        assert_eq!(result.eliminated_count, 2);
        assert_eq!(passive.members(), 1);
        assert_eq!(archive.members(), 2);
    }
}
