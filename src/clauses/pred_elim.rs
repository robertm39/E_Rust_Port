use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::min_heap::MinHeap;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_PE_RESOLVE};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::clauses::tautologies::clause_is_tautology;
use crate::terms::functypes::FunCode;
use crate::terms::match_mgu::subst_mgu_complete;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

type ClauseId = i64;
type PredicateTaskCmp = fn(&PredicateEliminationTask, &PredicateEliminationTask) -> Ordering;
type PredicateTaskQueue = MinHeap<PredicateEliminationTask, PredicateTaskCmp>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredicateEliminationConfig {
    pub max_occs: i64,
    pub tolerance: i64,
    pub force_mu_decrease: bool,
    pub ignore_conj_syms: bool,
}

impl Default for PredicateEliminationConfig {
    fn default() -> Self {
        Self {
            max_occs: -1,
            tolerance: 0,
            force_mu_decrease: false,
            ignore_conj_syms: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PredicateEliminationResult {
    pub start_count: i64,
    pub eliminated_count: i64,
    pub generated_count: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LastCheck {
    num_lit: i64,
    sq_vars: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct PredicateEliminationTask {
    sym: FunCode,
    positive_singular: BTreeSet<ClauseId>,
    negative_singular: BTreeSet<ClauseId>,
    offending_cls: BTreeSet<ClauseId>,
    num_lit: i64,
    sq_vars: f64,
    size: i64,
    blocked: bool,
}

impl PredicateEliminationTask {
    fn new(sym: FunCode) -> Self {
        Self {
            sym,
            positive_singular: BTreeSet::new(),
            negative_singular: BTreeSet::new(),
            offending_cls: BTreeSet::new(),
            num_lit: 0,
            sq_vars: 0.0,
            size: 0,
            blocked: false,
        }
    }

    fn can_schedule(&self) -> bool {
        !self.blocked && self.offending_cls.is_empty()
    }

    fn max_cardinality(&self) -> usize {
        self.positive_singular.len() * self.negative_singular.len()
    }

    fn signed_occurrences(&self, positive: bool) -> usize {
        self.offending_cls.len()
            + if positive {
                self.positive_singular.len()
            } else {
                self.negative_singular.len()
            }
    }

    fn add_clause_stats(&mut self, clause: &Clause) {
        let stats = clause_measure(clause);
        self.num_lit += stats.num_lit;
        self.size += 1;
        self.sq_vars += stats.sq_vars;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClauseMeasure {
    num_lit: i64,
    set_size: i64,
    sq_vars: f64,
}

impl ClauseMeasure {
    fn zero() -> Self {
        Self {
            num_lit: 0,
            set_size: 0,
            sq_vars: 0.0,
        }
    }

    fn add_clause(&mut self, clause: &Clause) {
        let stats = clause_measure(clause);
        self.num_lit += stats.num_lit;
        self.set_size += stats.set_size;
        self.sq_vars += stats.sq_vars;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RequiredSign {
    Negative,
    Positive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolverKind {
    NonEquational,
    Equational,
}

/// Performs the currently ported singular predicate-elimination path.
///
/// This mirrors the `ccl_pred_elim` singular branch with gate recognition
/// disabled. If any equality literal is present in the passive set, the C code
/// globally switches to the equality-aware resolver; Rust preserves that
/// behavior.
///
/// # Errors
///
/// Returns a diagnostic if term-bank insertion, tautology checking, or
/// variable normalization fails.
pub fn eliminate_predicates_singular(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
) -> Result<PredicateEliminationResult, Diagnostic> {
    let start_count = passive.members();
    let mut last_checks = BTreeMap::new();
    let mut blocked_symbols = BTreeSet::new();
    let mut generated_count = 0;

    loop {
        let (tasks, eqn_found) = build_task_map(passive, bank, &config, &mut blocked_symbols);
        let resolver = if eqn_found {
            ResolverKind::Equational
        } else {
            ResolverKind::NonEquational
        };

        let mut task_queue = build_task_queue(tasks, &last_checks);
        let mut changed = false;
        while let Some(task) = task_queue.pop_min() {
            last_checks.insert(
                task.sym,
                LastCheck {
                    num_lit: task.num_lit,
                    sq_vars: task.sq_vars,
                },
            );
            let mut generated = do_singular_elimination(&task, passive, bank, tmp_bank, resolver)?;
            if measure_decreases(
                &task,
                &generated,
                config.tolerance,
                config.force_mu_decrease,
            ) {
                move_task_clauses_to_archive(&task, passive, archive);
                generated_count += i64_from_usize(generated.len());
                while let Some(mut clause) = generated.pop() {
                    clause.normalize_vars(bank, fresh_vars)?;
                    passive.insert(clause);
                }
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    Ok(PredicateEliminationResult {
        start_count,
        eliminated_count: start_count - passive.members(),
        generated_count,
    })
}

/// Performs singular predicate elimination and writes the C
/// `PredicateElimination` progress lines.
///
/// # Errors
///
/// Returns a diagnostic under the same conditions as
/// [`eliminate_predicates_singular`], or if `output` rejects a write.
pub fn eliminate_predicates_singular_with_output(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
    output: &mut impl fmt::Write,
) -> Result<PredicateEliminationResult, Diagnostic> {
    let start_count = passive.members();
    writeln!(output, "% PE start: {start_count}").map_err(predicate_elim_write_error)?;
    let result =
        eliminate_predicates_singular(passive, archive, bank, tmp_bank, fresh_vars, config)?;
    writeln!(output, "% PE eliminated: {}", result.eliminated_count)
        .map_err(predicate_elim_write_error)?;
    Ok(result)
}

/// Compatibility alias for the initial non-equational helper name.
///
/// The implementation now also supports the C equality-aware singular resolver
/// branch when equality literals are present in the passive set.
pub fn eliminate_predicates_singular_non_equational(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
) -> Result<PredicateEliminationResult, Diagnostic> {
    eliminate_predicates_singular(passive, archive, bank, tmp_bank, fresh_vars, config)
}

/// Compatibility alias for the initial non-equational output helper name.
pub fn eliminate_predicates_singular_non_equational_with_output(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
    output: &mut impl fmt::Write,
) -> Result<PredicateEliminationResult, Diagnostic> {
    eliminate_predicates_singular_with_output(
        passive, archive, bank, tmp_bank, fresh_vars, config, output,
    )
}

fn build_task_map(
    passive: &ClauseSet,
    bank: &TermBank,
    config: &PredicateEliminationConfig,
    blocked_symbols: &mut BTreeSet<FunCode>,
) -> (BTreeMap<FunCode, PredicateEliminationTask>, bool) {
    let mut tasks = BTreeMap::new();
    let mut eqn_found = false;
    for clause in passive.iter() {
        scan_clause_for_predicates(
            clause,
            bank,
            config,
            blocked_symbols,
            &mut tasks,
            &mut eqn_found,
        );
    }
    (tasks, eqn_found)
}

fn scan_clause_for_predicates(
    clause: &Clause,
    bank: &TermBank,
    config: &PredicateEliminationConfig,
    blocked_symbols: &mut BTreeSet<FunCode>,
    tasks: &mut BTreeMap<FunCode, PredicateEliminationTask>,
    eqn_found: &mut bool,
) {
    let clause_is_conjecture = clause.query_tptp_type() == CP_TYPE_CONJECTURE || clause.is_goal();
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_equ_lit(bank) {
            *eqn_found = true;
            continue;
        }

        let sym = literal.left().f_code();
        if blocked_symbols.contains(&sym) {
            continue;
        }

        let task = tasks
            .entry(sym)
            .or_insert_with(|| PredicateEliminationTask::new(sym));
        let occurrences = task.signed_occurrences(literal.is_positive());
        if (config.ignore_conj_syms && clause_is_conjecture)
            || config.max_occs > 0 && occurrences >= usize_from_i64(config.max_occs)
        {
            task.blocked = true;
            blocked_symbols.insert(sym);
            continue;
        }

        let inserted = if clause_has_other_predicate_literal(clause, bank, literal_index, sym) {
            task.offending_cls.insert(clause.ident())
        } else if literal.is_positive() {
            task.positive_singular.insert(clause.ident())
        } else {
            task.negative_singular.insert(clause.ident())
        };

        if inserted {
            task.add_clause_stats(clause);
        }
    }
}

fn build_task_queue(
    tasks: BTreeMap<FunCode, PredicateEliminationTask>,
    last_checks: &BTreeMap<FunCode, LastCheck>,
) -> PredicateTaskQueue {
    let mut task_queue = MinHeap::new(compare_tasks as PredicateTaskCmp);
    for task in tasks.into_values() {
        if task.can_schedule() && should_schedule(&task, last_checks.get(&task.sym)) {
            task_queue.add_ptr(task);
        }
    }
    task_queue
}

fn should_schedule(task: &PredicateEliminationTask, last_check: Option<&LastCheck>) -> bool {
    last_check.is_none_or(|last| task.num_lit < last.num_lit && task.sq_vars < last.sq_vars)
}

fn do_singular_elimination(
    task: &PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    resolver: ResolverKind,
) -> Result<Vec<Clause>, Diagnostic> {
    let mut generated = Vec::new();
    for positive_id in &task.positive_singular {
        let Some(positive_clause) = passive.find_by_id(*positive_id) else {
            continue;
        };
        for negative_id in &task.negative_singular {
            let Some(negative_clause) = passive.find_by_id(*negative_id) else {
                continue;
            };
            let resolvent = match resolver {
                ResolverKind::NonEquational => {
                    build_neq_resolvent(positive_clause, negative_clause, task.sym, bank)?
                }
                ResolverKind::Equational => Some(build_eq_resolvent(
                    positive_clause,
                    negative_clause,
                    task.sym,
                    bank,
                )?),
            };
            if let Some(resolvent) = resolvent {
                if !clause_is_tautology(tmp_bank, &resolvent)? {
                    generated.push(resolvent);
                }
            }
        }
    }
    Ok(generated)
}

fn build_neq_resolvent(
    positive_clause: &Clause,
    negative_clause: &Clause,
    sym: FunCode,
    bank: &mut TermBank,
) -> Result<Option<Clause>, Diagnostic> {
    debug_assert_ne!(positive_clause.ident(), negative_clause.ident());

    let positive_copy = positive_clause.copy_disjoint(bank)?;
    let Some((positive_literal, positive_rest)) =
        split_first_literal_with_head(positive_copy, bank, sym, RequiredSign::Positive)
    else {
        return Ok(None);
    };
    let negative_copy = negative_clause.copy_to_bank(bank)?;
    let Some((negative_literal, negative_rest)) =
        split_first_literal_with_head(negative_copy, bank, sym, RequiredSign::Negative)
    else {
        return Ok(None);
    };

    let mut subst = Substitution::new();
    if !subst_mgu_complete(negative_literal.left(), positive_literal.left(), &mut subst) {
        subst.delete();
        return Ok(None);
    }

    let result = instantiate_resolvent_rest(&positive_rest, &negative_rest, bank).map(|literals| {
        let mut resolvent = Clause::alloc(literals);
        clause_push_derivation(
            &mut resolvent,
            DC_PE_RESOLVE,
            Some(positive_clause),
            Some(negative_clause),
        );
        resolvent
    });
    subst.delete();
    result.map(Some)
}

fn build_eq_resolvent(
    positive_clause: &Clause,
    negative_clause: &Clause,
    sym: FunCode,
    bank: &mut TermBank,
) -> Result<Clause, Diagnostic> {
    debug_assert_ne!(positive_clause.ident(), negative_clause.ident());

    let positive_copy = positive_clause.copy_disjoint(bank)?;
    let Some((positive_literal, positive_rest)) =
        split_first_literal_with_head(positive_copy, bank, sym, RequiredSign::Positive)
    else {
        panic!("positive predicate-elimination parent must contain the pivot symbol");
    };
    let negative_copy = negative_clause.copy_to_bank(bank)?;
    let Some((negative_literal, negative_rest)) =
        split_first_literal_with_head(negative_copy, bank, sym, RequiredSign::Negative)
    else {
        panic!("negative predicate-elimination parent must contain the pivot symbol");
    };

    let literals = if unique_distinct_vars(positive_literal.left())
        || unique_distinct_vars(negative_literal.left())
    {
        let mut subst = Substitution::new();
        let unified =
            subst_mgu_complete(positive_literal.left(), negative_literal.left(), &mut subst);
        debug_assert!(
            unified,
            "distinct-variable predicate pivot should always unify"
        );
        let result = instantiate_resolvent_rest(&positive_rest, &negative_rest, bank);
        subst.delete();
        result?
    } else {
        let mut result =
            argument_disequalities(positive_literal.left(), negative_literal.left(), bank)?;
        result.append(positive_rest.copy_to_bank(bank)?);
        result.append(negative_rest.copy_to_bank(bank)?);
        result.remove_resolved(bank);
        result.remove_duplicates(bank);
        result
    };

    let mut resolvent = Clause::alloc(literals);
    clause_push_derivation(
        &mut resolvent,
        DC_PE_RESOLVE,
        Some(positive_clause),
        Some(negative_clause),
    );
    Ok(resolvent)
}

fn split_first_literal_with_head(
    clause: Clause,
    bank: &TermBank,
    sym: FunCode,
    required_sign: RequiredSign,
) -> Option<(Eqn, EqnList)> {
    let mut selected = None;
    let mut rest = EqnList::new();
    for literal in clause.into_literals().into_vec() {
        if selected.is_none()
            && !literal.is_equ_lit(bank)
            && literal.left().f_code() == sym
            && required_sign.matches(literal.is_positive())
        {
            selected = Some(literal);
        } else {
            rest.insert_first(literal);
        }
    }
    selected.map(|literal| (literal, rest))
}

impl RequiredSign {
    const fn matches(self, positive: bool) -> bool {
        match self {
            Self::Negative => !positive,
            Self::Positive => positive,
        }
    }
}

fn unique_distinct_vars(term: &crate::terms::termtypes::Term) -> bool {
    let mut variables = Vec::new();
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("predicate argument {index} is initialized"));
        if !arg.is_free_var() || variables.iter().any(|seen| seen == &arg) {
            return false;
        }
        variables.push(arg);
    }
    true
}

fn argument_disequalities(
    positive_predicate: &crate::terms::termtypes::Term,
    negative_predicate: &crate::terms::termtypes::Term,
    bank: &mut TermBank,
) -> Result<EqnList, Diagnostic> {
    assert_eq!(
        positive_predicate.arity(),
        negative_predicate.arity(),
        "predicate pivots with the same head have equal arity"
    );
    let mut result = EqnList::new();
    for index in 0..positive_predicate.arity() {
        let positive_arg = positive_predicate
            .argument(index)
            .unwrap_or_else(|| panic!("positive predicate argument {index} is initialized"));
        let negative_arg = negative_predicate
            .argument(index)
            .unwrap_or_else(|| panic!("negative predicate argument {index} is initialized"));
        result.insert_first(Eqn::alloc(positive_arg, negative_arg, bank, false)?);
    }
    Ok(result)
}

fn instantiate_resolvent_rest(
    positive_rest: &EqnList,
    negative_rest: &EqnList,
    bank: &mut TermBank,
) -> Result<EqnList, Diagnostic> {
    let mut result = positive_rest.copy_to_bank(bank)?;
    result.append(negative_rest.copy_to_bank(bank)?);
    result.remove_resolved(bank);
    result.remove_duplicates(bank);
    Ok(result)
}

fn measure_decreases(
    task: &PredicateEliminationTask,
    new_clauses: &[Clause],
    tolerance: i64,
    force_mu_decrease: bool,
) -> bool {
    let mut new_measure = ClauseMeasure::zero();
    for clause in new_clauses {
        new_measure.add_clause(clause);
    }

    let lit_clause_decrease = new_measure.num_lit < task.num_lit + tolerance
        || new_measure.set_size < task.size + tolerance;
    let mu_decrease = new_measure.sq_vars < task.sq_vars + i64_to_f64(tolerance);
    if force_mu_decrease {
        mu_decrease && lit_clause_decrease
    } else {
        mu_decrease || lit_clause_decrease
    }
}

fn move_task_clauses_to_archive(
    task: &PredicateEliminationTask,
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
) {
    let mut ids = task.positive_singular.clone();
    ids.extend(&task.negative_singular);
    ids.extend(&task.offending_cls);
    for ident in ids {
        if let Some(clause) = passive.extract_by_id(ident) {
            archive.insert(clause);
        }
    }
}

fn clause_has_other_predicate_literal(
    clause: &Clause,
    bank: &TermBank,
    except_index: usize,
    sym: FunCode,
) -> bool {
    clause
        .literals()
        .as_slice()
        .iter()
        .enumerate()
        .any(|(index, literal)| {
            index != except_index && !literal.is_equ_lit(bank) && literal.left().f_code() == sym
        })
}

fn clause_measure(clause: &Clause) -> ClauseMeasure {
    let mut variables = BTreeMap::new();
    clause.collect_variables(&mut variables);
    let var_count = i64_from_usize(variables.len());
    ClauseMeasure {
        num_lit: i64_from_usize(clause.literal_number()),
        set_size: 1,
        sq_vars: i64_to_f64(var_count * var_count),
    }
}

fn compare_tasks(left: &PredicateEliminationTask, right: &PredicateEliminationTask) -> Ordering {
    left.max_cardinality()
        .cmp(&right.max_cardinality())
        .then_with(|| left.sym.cmp(&right.sym))
}

fn predicate_elim_write_error(error: fmt::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYSTEM_ERROR,
        format!("Could not write predicate-elimination output: {error}"),
    )
}

fn i64_from_usize(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn usize_from_i64(value: i64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn i64_to_f64(value: i64) -> f64 {
    value as f64
}

#[cfg(test)]
mod tests {
    use super::{
        eliminate_predicates_singular, eliminate_predicates_singular_with_output,
        PredicateEliminationConfig, PredicateEliminationResult,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{ClauseDerivationRef, DerivationEntry, DC_PE_RESOLVE};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn tmp_bank(bank: &TermBank) -> TermBank {
        TermBank::new(bank.signature().clone()).unwrap()
    }

    fn fresh_vars(bank: &TermBank) -> VarBank {
        VarBank::new(bank.signature().type_bank())
    }

    fn object_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn object_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn object_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
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

    fn equality_literal(bank: &mut TermBank, left: &Term, right: &Term) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn ids(set: &ClauseSet) -> Vec<i64> {
        set.iter().map(Clause::ident).collect()
    }

    #[test]
    fn singular_elimination_replaces_opposite_predicate_pair_with_resolvent() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_pair_a");
        let b = object_const(&mut bank, "pe_pair_b");
        let c = object_const(&mut bank, "pe_pair_c");
        let p_a = predicate_atom(&mut bank, "pe_pair_p", std::slice::from_ref(&a));
        let q_a = predicate_atom(&mut bank, "pe_pair_q", &[a]);
        let q_b = predicate_atom(&mut bank, "pe_pair_q", &[b]);
        let q_c = predicate_atom(&mut bank, "pe_pair_q", &[c]);
        let positive = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let negative = clause(vec![
            predicate_literal(&mut bank, &q_a, true),
            predicate_literal(&mut bank, &p_a, false),
        ]);
        let q_offending = clause(vec![
            predicate_literal(&mut bank, &q_b, true),
            predicate_literal(&mut bank, &q_c, true),
        ]);
        let positive_id = positive.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([positive, negative, q_offending]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
        )
        .unwrap();

        assert_eq!(
            result,
            PredicateEliminationResult {
                start_count: 3,
                eliminated_count: 1,
                generated_count: 1
            }
        );
        assert_eq!(ids(&archive), vec![positive_id, negative_id]);
        let resolvents = passive
            .iter()
            .filter(|clause| clause.derivation().is_some())
            .collect::<Vec<_>>();
        assert_eq!(resolvents.len(), 1);
        assert_eq!(passive.len(), 2);
        assert_eq!(
            resolvents[0].literals().as_slice()[0].left().f_code(),
            q_a.f_code()
        );
        assert_eq!(
            resolvents[0].derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_PE_RESOLVE),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(positive_id, 0)),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(negative_id, 0)),
            ]
        );
    }

    #[test]
    fn singular_elimination_removes_one_polarity_predicates_without_resolvents() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_pure_a");
        let p_a = predicate_atom(&mut bank, "pe_pure_p", &[a]);
        let unit = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let unit_id = unit.ident();
        let mut passive = ClauseSet::from_clauses([unit]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 1);
        assert_eq!(result.generated_count, 0);
        assert!(passive.is_empty());
        assert_eq!(ids(&archive), vec![unit_id]);
    }

    #[test]
    fn repeated_predicate_head_makes_task_unschedulable() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_repeat_a");
        let b = object_const(&mut bank, "pe_repeat_b");
        let p_a = predicate_atom(&mut bank, "pe_repeat_p", &[a]);
        let p_b = predicate_atom(&mut bank, "pe_repeat_p", &[b]);
        let repeated = clause(vec![
            predicate_literal(&mut bank, &p_a, true),
            predicate_literal(&mut bank, &p_b, true),
        ]);
        let negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let repeated_id = repeated.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([repeated, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert_eq!(ids(&passive), vec![repeated_id, negative_id]);
        assert!(archive.is_empty());
    }

    #[test]
    fn max_occurrence_limit_permanently_blocks_symbol() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_limit_a");
        let b = object_const(&mut bank, "pe_limit_b");
        let p_a = predicate_atom(&mut bank, "pe_limit_p", &[a]);
        let p_b = predicate_atom(&mut bank, "pe_limit_p", &[b]);
        let first = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let second = clause(vec![predicate_literal(&mut bank, &p_b, true)]);
        let negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let first_id = first.ident();
        let second_id = second.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([first, second, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig {
                max_occs: 1,
                ..PredicateEliminationConfig::default()
            },
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert_eq!(ids(&passive), vec![first_id, second_id, negative_id]);
        assert!(archive.is_empty());
    }

    #[test]
    fn ignore_conjecture_symbols_blocks_goal_predicate() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_conj_a");
        let p_a = predicate_atom(&mut bank, "pe_conj_p", &[a]);
        let positive = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let mut negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        negative.set_tptp_type(CP_TYPE_CONJECTURE);
        let positive_id = positive.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig {
                ignore_conj_syms: true,
                ..PredicateEliminationConfig::default()
            },
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 0);
        assert_eq!(ids(&passive), vec![positive_id, negative_id]);
        assert!(archive.is_empty());
    }

    #[test]
    fn predicate_elimination_output_matches_c_progress_shape() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_output_a");
        let p_a = predicate_atom(&mut bank, "pe_output_p", &[a]);
        let unit = clause(vec![predicate_literal(&mut bank, &p_a, true)]);
        let mut passive = ClauseSet::from_clauses([unit]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);
        let mut output = String::new();

        let result = eliminate_predicates_singular_with_output(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
            &mut output,
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 1);
        assert_eq!(output, "% PE start: 1\n% PE eliminated: 1\n");
    }

    #[test]
    fn equality_resolver_adds_argument_disequalities_for_non_pattern_pivots() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_eq_a");
        let fa = object_unary(&mut bank, "pe_eq_f", &a);
        let ga = object_unary(&mut bank, "pe_eq_g", &a);
        let positive_atom = predicate_atom(&mut bank, "pe_eq_p", std::slice::from_ref(&fa));
        let negative_atom = predicate_atom(&mut bank, "pe_eq_p", std::slice::from_ref(&ga));
        let trigger = clause(vec![equality_literal(&mut bank, &a, &a)]);
        let positive = clause(vec![predicate_literal(&mut bank, &positive_atom, true)]);
        let negative = clause(vec![predicate_literal(&mut bank, &negative_atom, false)]);
        let positive_id = positive.ident();
        let negative_id = negative.ident();
        let mut passive = ClauseSet::from_clauses([trigger, positive, negative]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
        )
        .unwrap();

        assert_eq!(
            result,
            PredicateEliminationResult {
                start_count: 3,
                eliminated_count: 1,
                generated_count: 1
            }
        );
        assert!(ids(&archive).contains(&positive_id));
        assert!(ids(&archive).contains(&negative_id));
        let resolvents = passive
            .iter()
            .filter(|clause| clause.derivation().is_some())
            .collect::<Vec<_>>();
        assert_eq!(resolvents.len(), 1);
        let literal = &resolvents[0].literals().as_slice()[0];
        assert!(literal.is_negative());
        assert!(literal.is_equ_lit(&bank));
        assert_eq!(literal.left(), &fa);
        assert_eq!(literal.right(), &ga);
    }

    #[test]
    fn equality_resolver_instantiates_rest_when_pivot_has_distinct_variables() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_eq_pattern_a");
        let b = object_const(&mut bank, "pe_eq_pattern_b");
        let c = object_const(&mut bank, "pe_eq_pattern_c");
        let x = object_var(&bank, -2);
        let p_x = predicate_atom(&mut bank, "pe_eq_pattern_p", std::slice::from_ref(&x));
        let p_a = predicate_atom(&mut bank, "pe_eq_pattern_p", std::slice::from_ref(&a));
        let s_x = predicate_atom(&mut bank, "pe_eq_pattern_s", std::slice::from_ref(&x));
        let s_a = predicate_atom(&mut bank, "pe_eq_pattern_s", std::slice::from_ref(&a));
        let s_b = predicate_atom(&mut bank, "pe_eq_pattern_s", std::slice::from_ref(&b));
        let s_c = predicate_atom(&mut bank, "pe_eq_pattern_s", std::slice::from_ref(&c));
        let trigger = clause(vec![equality_literal(&mut bank, &a, &a)]);
        let positive = clause(vec![
            predicate_literal(&mut bank, &p_x, true),
            predicate_literal(&mut bank, &s_x, true),
        ]);
        let negative = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let s_offending = clause(vec![
            predicate_literal(&mut bank, &s_b, true),
            predicate_literal(&mut bank, &s_c, true),
        ]);
        let mut passive = ClauseSet::from_clauses([trigger, positive, negative, s_offending]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig::default(),
        )
        .unwrap();

        assert_eq!(result.eliminated_count, 1);
        assert_eq!(result.generated_count, 1);
        let resolvents = passive
            .iter()
            .filter(|clause| clause.derivation().is_some())
            .collect::<Vec<_>>();
        assert_eq!(resolvents.len(), 1);
        assert_eq!(resolvents[0].literal_number(), 1);
        let literal = &resolvents[0].literals().as_slice()[0];
        assert!(!literal.is_equ_lit(&bank));
        assert_eq!(literal.left(), &s_a);
    }
}
