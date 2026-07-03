use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::min_heap::MinHeap;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_PE_RESOLVE};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::clauses::picosat::PicoSat;
use crate::clauses::satinterface::{picosat_error_to_diagnostic, SatClauseSet};
use crate::clauses::tautologies::{clause_is_tautology, clause_is_tautology_real};
use crate::terms::functypes::FunCode;
use crate::terms::match_mgu::{subst_match_complete, subst_mgu_complete};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_id, Term};
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
    pub recognize_gates: bool,
}

impl Default for PredicateEliminationConfig {
    fn default() -> Self {
        Self {
            max_occs: -1,
            tolerance: 0,
            force_mu_decrease: false,
            ignore_conj_syms: false,
            recognize_gates: false,
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
    positive_gates: BTreeSet<ClauseId>,
    negative_gates: BTreeSet<ClauseId>,
    gate_status: GateStatus,
    num_lit: i64,
    sq_vars: f64,
    size: i64,
    blocked: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GateStatus {
    Unknown,
    IsGate,
    NotGate,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PotentialGateSigns {
    positive: bool,
    negative: bool,
}

impl PotentialGateSigns {
    const fn has_both(self) -> bool {
        self.positive && self.negative
    }

    fn add(&mut self, positive: bool) {
        if positive {
            self.positive = true;
        } else {
            self.negative = true;
        }
    }
}

impl PredicateEliminationTask {
    fn new(sym: FunCode) -> Self {
        Self {
            sym,
            positive_singular: BTreeSet::new(),
            negative_singular: BTreeSet::new(),
            offending_cls: BTreeSet::new(),
            positive_gates: BTreeSet::new(),
            negative_gates: BTreeSet::new(),
            gate_status: GateStatus::Unknown,
            num_lit: 0,
            sq_vars: 0.0,
            size: 0,
            blocked: false,
        }
    }

    fn can_schedule(&self) -> bool {
        !self.blocked && (self.offending_cls.is_empty() || self.gate_status == GateStatus::IsGate)
    }

    fn max_cardinality(&self) -> usize {
        if self.gate_status == GateStatus::IsGate {
            self.positive_gates.len() * self.negative_singular.len()
                + self.negative_gates.len() * self.positive_singular.len()
                + self.positive_gates.len() * self.offending_cls.len()
                + self.negative_gates.len() * self.offending_cls.len()
        } else {
            self.positive_singular.len() * self.negative_singular.len()
        }
    }

    fn signed_occurrences(&self, positive: bool) -> usize {
        self.offending_cls.len()
            + if positive {
                self.positive_singular.len() + self.positive_gates.len()
            } else {
                self.negative_singular.len() + self.negative_gates.len()
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
    Any,
    Negative,
    Positive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolverKind {
    NonEquational,
    Equational,
}

#[derive(Debug, Default)]
struct TaskElimination {
    generated: Vec<Clause>,
    archive_intermediates: Vec<Clause>,
}

enum GateValidationBackend<'a> {
    Internal,
    PicoSat(&'a mut PicoSat),
}

/// Performs the currently ported predicate-elimination path.
///
/// This mirrors the `ccl_pred_elim` singular branch and the first-order
/// SAT-core gate-recognition branch. If any equality literal is present in the
/// passive set, the C code globally switches singular elimination to the
/// equality-aware resolver; Rust preserves that behavior.
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
    eliminate_predicates_singular_impl(
        passive,
        archive,
        bank,
        tmp_bank,
        fresh_vars,
        config,
        GateValidationBackend::Internal,
    )
}

/// Performs predicate elimination with runtime-loaded `PicoSAT` gate validation.
///
/// The caller owns the solver library handle. This wrapper resets the solver
/// around each gate-core check to mirror C's per-check `picosat_init` /
/// `picosat_reset` lifecycle while leaving existing internal-solver callers
/// unchanged.
///
/// # Errors
///
/// Returns a diagnostic if predicate elimination fails, or if `PicoSAT` reset,
/// export, solving, or core extraction fails.
pub fn eliminate_predicates_singular_with_picosat(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
    solver: &mut PicoSat,
) -> Result<PredicateEliminationResult, Diagnostic> {
    eliminate_predicates_singular_impl(
        passive,
        archive,
        bank,
        tmp_bank,
        fresh_vars,
        config,
        GateValidationBackend::PicoSat(solver),
    )
}

fn eliminate_predicates_singular_impl(
    passive: &mut ClauseSet,
    archive: &mut ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    config: PredicateEliminationConfig,
    mut gate_backend: GateValidationBackend<'_>,
) -> Result<PredicateEliminationResult, Diagnostic> {
    let start_count = passive.members();
    let mut last_checks = BTreeMap::new();
    let mut blocked_symbols = BTreeSet::new();
    let mut generated_count = 0;

    loop {
        let (tasks, eqn_found) = build_task_map(
            passive,
            bank,
            tmp_bank,
            &config,
            &mut blocked_symbols,
            &mut gate_backend,
        )?;
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
            let mut elimination =
                do_task_elimination(&task, passive, bank, tmp_bank, fresh_vars, resolver)?;
            while let Some(clause) = elimination.archive_intermediates.pop() {
                archive.insert(clause);
            }
            if measure_decreases(
                &task,
                &elimination.generated,
                config.tolerance,
                config.force_mu_decrease,
            ) {
                move_task_clauses_to_archive(&task, passive, archive);
                generated_count += i64_from_usize(elimination.generated.len());
                while let Some(mut clause) = elimination.generated.pop() {
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

/// Reports whether C's gate-recognition branch would need SAT-backed gate
/// validation rather than falling back to the ordinary singular task sets.
#[must_use]
pub fn predicate_elimination_needs_gate_validation(
    passive: &ClauseSet,
    bank: &TermBank,
    config: PredicateEliminationConfig,
) -> bool {
    let mut blocked_symbols = BTreeSet::new();
    let mut potential_gates = BTreeMap::new();
    for clause in passive.iter() {
        scan_clause_for_potential_gates(
            clause,
            bank,
            &config,
            &mut blocked_symbols,
            &mut potential_gates,
        );
        if potential_gates
            .values()
            .any(|signs: &PotentialGateSigns| signs.has_both())
        {
            return true;
        }
    }
    false
}

fn scan_clause_for_potential_gates(
    clause: &Clause,
    bank: &TermBank,
    config: &PredicateEliminationConfig,
    blocked_symbols: &mut BTreeSet<FunCode>,
    potential_gates: &mut BTreeMap<FunCode, PotentialGateSigns>,
) {
    let clause_is_conjecture = clause.query_tptp_type() == CP_TYPE_CONJECTURE || clause.is_goal();
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        if literal.is_equ_lit(bank) {
            continue;
        }

        let sym = literal.left().f_code();
        if blocked_symbols.contains(&sym) {
            continue;
        }

        let occurrences = potential_gates.get(&sym).map_or(0, |signs| {
            usize::from(signs.positive) + usize::from(signs.negative)
        });
        if (config.ignore_conj_syms && clause_is_conjecture)
            || config.max_occs > 0 && occurrences >= usize_from_i64(config.max_occs)
        {
            blocked_symbols.insert(sym);
            continue;
        }

        if is_potential_gate_clause(clause, bank, literal_index, literal) {
            potential_gates
                .entry(sym)
                .or_default()
                .add(literal.is_positive());
        }
    }
}

fn is_potential_gate_clause(
    clause: &Clause,
    bank: &TermBank,
    literal_index: usize,
    literal: &Eqn,
) -> bool {
    let sym = literal.left().f_code();
    if clause_has_other_predicate_literal(clause, bank, literal_index, sym) {
        return false;
    }
    let Some(vars) = unique_distinct_arg_vars(literal.left()) else {
        return false;
    };
    clause
        .literals()
        .as_slice()
        .iter()
        .enumerate()
        .all(|(index, other)| {
            index == literal_index
                || (term_vars_from_set(other.left(), &vars)
                    && term_vars_from_set(other.right(), &vars))
        })
}

fn unique_distinct_arg_vars(term: &Term) -> Option<BTreeSet<usize>> {
    let mut variables = BTreeSet::new();
    for index in 0..term.arity() {
        let arg = term
            .argument(index)
            .unwrap_or_else(|| panic!("predicate argument {index} is initialized"));
        if !arg.is_free_var() || !variables.insert(term_identity_id(&arg)) {
            return None;
        }
    }
    Some(variables)
}

fn term_vars_from_set(term: &Term, vars: &BTreeSet<usize>) -> bool {
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.is_free_var() {
            if !vars.contains(&term_identity_id(&current)) {
                return false;
            }
        } else {
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    true
}

fn build_task_map(
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    config: &PredicateEliminationConfig,
    blocked_symbols: &mut BTreeSet<FunCode>,
    gate_backend: &mut GateValidationBackend<'_>,
) -> Result<(BTreeMap<FunCode, PredicateEliminationTask>, bool), Diagnostic> {
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
    if config.recognize_gates {
        update_gate_status(&mut tasks, passive, bank, tmp_bank, gate_backend)?;
    } else {
        for task in tasks.values_mut() {
            declare_not_gate(task);
        }
    }
    Ok((tasks, eqn_found))
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
        } else {
            let mut clause_inserted = false;
            if config.recognize_gates
                && is_potential_gate_clause(clause, bank, literal_index, literal)
            {
                clause_inserted = if literal.is_positive() {
                    task.positive_gates.insert(clause.ident())
                } else {
                    task.negative_gates.insert(clause.ident())
                };
            }
            let singular_inserted = if literal.is_positive() {
                task.positive_singular.insert(clause.ident())
            } else {
                task.negative_singular.insert(clause.ident())
            };
            singular_inserted || clause_inserted
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

fn update_gate_status(
    tasks: &mut BTreeMap<FunCode, PredicateEliminationTask>,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    gate_backend: &mut GateValidationBackend<'_>,
) -> Result<(), Diagnostic> {
    for task in tasks.values_mut() {
        if !task.positive_gates.is_empty() && !task.negative_gates.is_empty() {
            check_unsat_and_tauto(task, passive, bank, tmp_bank, gate_backend)?;
        } else {
            declare_not_gate(task);
        }
    }
    Ok(())
}

fn declare_not_gate(task: &mut PredicateEliminationTask) {
    task.positive_gates.clear();
    task.negative_gates.clear();
    task.gate_status = GateStatus::NotGate;
}

fn check_unsat_and_tauto(
    task: &mut PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    gate_backend: &mut GateValidationBackend<'_>,
) -> Result<(), Diagnostic> {
    let mut gate_ids = task
        .positive_gates
        .iter()
        .chain(&task.negative_gates)
        .copied()
        .collect::<Vec<_>>();
    let Some(pivot_id) = gate_ids.pop() else {
        declare_not_gate(task);
        return Ok(());
    };
    let Some(pivot) = passive.find_by_id(pivot_id) else {
        declare_not_gate(task);
        return Ok(());
    };

    let pivot_fresh = pivot.copy_disjoint(bank)?;
    let Some((fresh_lit, rest_fresh)) =
        split_first_literal_with_head(pivot_fresh, bank, task.sym, RequiredSign::Any)
    else {
        declare_not_gate(task);
        return Ok(());
    };

    let mut environment = SatClauseSet::new();
    let pivot_environment = Clause::alloc(rest_fresh);
    environment.import_clause_with_source(bank, &pivot_environment, pivot.clone())?;

    let mut subst = Substitution::new();
    for gate_id in gate_ids {
        let Some(clause) = passive.find_by_id(gate_id) else {
            subst.delete();
            declare_not_gate(task);
            return Ok(());
        };
        let Some(sym_index) =
            first_literal_index_with_head(clause, bank, task.sym, RequiredSign::Any)
        else {
            subst.delete();
            declare_not_gate(task);
            return Ok(());
        };
        let sym_term = clause.literals().as_slice()[sym_index].left().clone();
        let matched = subst_match_complete(&sym_term, fresh_lit.left(), &mut subst);
        debug_assert!(
            matched,
            "potential-gate predicate patterns should match the fresh pivot"
        );
        if !matched {
            subst.delete();
            declare_not_gate(task);
            return Ok(());
        }

        let rest = clause.literals().copy_except_index(Some(sym_index), bank)?;
        subst.backtrack();
        let environment_clause = Clause::alloc(rest);
        environment.import_clause_with_source(bank, &environment_clause, clause.clone())?;
    }
    subst.delete();

    if let Some(core) = check_and_get_gate_core(&mut environment, gate_backend)? {
        check_gate_core_tautologies(task, &core, bank, tmp_bank)
    } else {
        declare_not_gate(task);
        Ok(())
    }
}

fn check_and_get_gate_core(
    environment: &mut SatClauseSet,
    gate_backend: &mut GateValidationBackend<'_>,
) -> Result<Option<Vec<Clause>>, Diagnostic> {
    match gate_backend {
        GateValidationBackend::Internal => Ok(environment.check_and_get_core()),
        GateValidationBackend::PicoSat(solver) => {
            solver
                .reset()
                .map_err(|error| picosat_error_to_diagnostic(&error))?;
            let core = environment
                .check_and_get_core_with_picosat(solver)
                .map_err(|error| picosat_error_to_diagnostic(&error))?;
            solver
                .reset()
                .map_err(|error| picosat_error_to_diagnostic(&error))?;
            Ok(core)
        }
    }
}

fn check_gate_core_tautologies(
    task: &mut PredicateEliminationTask,
    unsat_core: &[Clause],
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for clause in unsat_core {
        if let Some(index) =
            first_literal_index_with_head(clause, bank, task.sym, RequiredSign::Any)
        {
            if clause.literals().as_slice()[index].is_positive() {
                positive.push(clause.clone());
            } else {
                negative.push(clause.clone());
            }
        }
    }

    let mut all_tautologies = true;
    if let (Some(positive_clause), Some(negative_clause)) = (positive.first(), negative.first()) {
        for _ in 0..negative.len() {
            let Some(resolvent) =
                build_neq_resolvent(positive_clause, negative_clause, task.sym, bank)?
            else {
                all_tautologies = false;
                break;
            };
            all_tautologies = clause_is_tautology_real(tmp_bank, &resolvent, false)?;
            if !all_tautologies {
                break;
            }
        }
    }

    declare_not_gate(task);
    if all_tautologies {
        task.gate_status = GateStatus::IsGate;
        for clause in positive {
            let ident = clause.ident();
            task.positive_gates.insert(ident);
            task.positive_singular.remove(&ident);
        }
        for clause in negative {
            let ident = clause.ident();
            task.negative_gates.insert(ident);
            task.negative_singular.remove(&ident);
        }
    }
    Ok(())
}

fn do_task_elimination(
    task: &PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    resolver: ResolverKind,
) -> Result<TaskElimination, Diagnostic> {
    if task.gate_status == GateStatus::IsGate {
        do_gate_elimination(task, passive, bank, tmp_bank, fresh_vars)
    } else {
        debug_assert!(task.offending_cls.is_empty());
        Ok(TaskElimination {
            generated: do_singular_elimination(task, passive, bank, tmp_bank, resolver)?,
            archive_intermediates: Vec::new(),
        })
    }
}

fn do_gate_elimination(
    task: &PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
) -> Result<TaskElimination, Diagnostic> {
    let mut generated = do_singular_elimination_for_sets(
        &task.positive_gates,
        &task.negative_singular,
        task.sym,
        passive,
        bank,
        tmp_bank,
        ResolverKind::NonEquational,
    )?;
    generated.extend(do_singular_elimination_for_sets(
        &task.positive_singular,
        &task.negative_gates,
        task.sym,
        passive,
        bank,
        tmp_bank,
        ResolverKind::NonEquational,
    )?);

    let mut archive_intermediates = Vec::new();
    do_gates_against_offending(
        task,
        passive,
        bank,
        tmp_bank,
        fresh_vars,
        &mut generated,
        &mut archive_intermediates,
    )?;
    Ok(TaskElimination {
        generated,
        archive_intermediates,
    })
}

fn do_singular_elimination(
    task: &PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    resolver: ResolverKind,
) -> Result<Vec<Clause>, Diagnostic> {
    do_singular_elimination_for_sets(
        &task.positive_singular,
        &task.negative_singular,
        task.sym,
        passive,
        bank,
        tmp_bank,
        resolver,
    )
}

fn do_singular_elimination_for_sets(
    positive_ids: &BTreeSet<ClauseId>,
    negative_ids: &BTreeSet<ClauseId>,
    sym: FunCode,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    resolver: ResolverKind,
) -> Result<Vec<Clause>, Diagnostic> {
    let mut generated = Vec::new();
    for positive_id in positive_ids {
        let Some(positive_clause) = passive.find_by_id(*positive_id) else {
            continue;
        };
        for negative_id in negative_ids {
            let Some(negative_clause) = passive.find_by_id(*negative_id) else {
                continue;
            };
            let resolvent = match resolver {
                ResolverKind::NonEquational => {
                    build_neq_resolvent(positive_clause, negative_clause, sym, bank)?
                }
                ResolverKind::Equational => Some(build_eq_resolvent(
                    positive_clause,
                    negative_clause,
                    sym,
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

fn do_gates_against_offending(
    task: &PredicateEliminationTask,
    passive: &ClauseSet,
    bank: &mut TermBank,
    tmp_bank: &mut TermBank,
    fresh_vars: &VarBank,
    generated: &mut Vec<Clause>,
    archive_intermediates: &mut Vec<Clause>,
) -> Result<(), Diagnostic> {
    let mut worklist = task
        .offending_cls
        .iter()
        .map(|ident| WorkClause::Original(*ident))
        .collect::<Vec<_>>();

    while let Some(work) = worklist.pop() {
        let (offending, original_id) = match work {
            WorkClause::Original(ident) => {
                let Some(clause) = passive.find_by_id(ident) else {
                    continue;
                };
                (clause.clone(), Some(ident))
            }
            WorkClause::Intermediate(clause) => (*clause, None),
        };

        let Some(sym_index) =
            first_literal_index_with_head(&offending, bank, task.sym, RequiredSign::Any)
        else {
            generated.push(offending);
            continue;
        };

        let positive = offending.literals().as_slice()[sym_index].is_positive();
        let gate_set = if positive {
            &task.negative_gates
        } else {
            &task.positive_gates
        };
        for gate_id in gate_set {
            let Some(gate_clause) = passive.find_by_id(*gate_id) else {
                continue;
            };
            let resolvent = if positive {
                build_neq_resolvent(&offending, gate_clause, task.sym, bank)?
            } else {
                build_neq_resolvent(gate_clause, &offending, task.sym, bank)?
            };
            if let Some(mut resolvent) = resolvent {
                if !clause_is_tautology(tmp_bank, &resolvent)? {
                    resolvent.normalize_vars(bank, fresh_vars)?;
                    worklist.push(WorkClause::Intermediate(Box::new(resolvent)));
                }
            }
        }

        if original_id.is_none() {
            archive_intermediates.push(offending);
        }
    }
    Ok(())
}

#[derive(Debug)]
enum WorkClause {
    Original(ClauseId),
    Intermediate(Box<Clause>),
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

fn first_literal_index_with_head(
    clause: &Clause,
    bank: &TermBank,
    sym: FunCode,
    required_sign: RequiredSign,
) -> Option<usize> {
    clause
        .literals()
        .as_slice()
        .iter()
        .enumerate()
        .find_map(|(index, literal)| {
            if !literal.is_equ_lit(bank)
                && literal.left().f_code() == sym
                && required_sign.matches(literal.is_positive())
            {
                Some(index)
            } else {
                None
            }
        })
}

impl RequiredSign {
    const fn matches(self, positive: bool) -> bool {
        match self {
            Self::Any => true,
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
    ids.extend(&task.positive_gates);
    ids.extend(&task.negative_gates);
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
        .then_with(|| {
            match (
                left.gate_status == GateStatus::IsGate,
                right.gate_status == GateStatus::IsGate,
            ) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        })
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
        predicate_elimination_needs_gate_validation, PredicateEliminationConfig,
        PredicateEliminationResult,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_CONJECTURE;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{ClauseDerivationRef, DerivationEntry, DC_PE_RESOLVE};
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::tautologies::clause_is_tautology_real;
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
    fn gate_recognition_without_bidirectional_potential_gate_needs_no_validation() {
        let mut bank = test_bank();
        let a = object_const(&mut bank, "pe_no_gate_a");
        let x = object_var(&bank, -2);
        let p_a = predicate_atom(&mut bank, "pe_no_gate_p", std::slice::from_ref(&a));
        let p_x = predicate_atom(&mut bank, "pe_no_gate_p", std::slice::from_ref(&x));
        let q_x = predicate_atom(&mut bank, "pe_no_gate_q", std::slice::from_ref(&x));
        let positive_potential = clause(vec![
            predicate_literal(&mut bank, &p_x, true),
            predicate_literal(&mut bank, &q_x, true),
        ]);
        let negative_not_potential = clause(vec![predicate_literal(&mut bank, &p_a, false)]);
        let passive = ClauseSet::from_clauses([positive_potential, negative_not_potential]);

        assert!(!predicate_elimination_needs_gate_validation(
            &passive,
            &bank,
            PredicateEliminationConfig::default()
        ));
    }

    #[test]
    fn gate_recognition_with_both_potential_signs_needs_validation() {
        let mut bank = test_bank();
        let x = object_var(&bank, -2);
        let p_x = predicate_atom(&mut bank, "pe_gate_p", std::slice::from_ref(&x));
        let q_x = predicate_atom(&mut bank, "pe_gate_q", std::slice::from_ref(&x));
        let r_x = predicate_atom(&mut bank, "pe_gate_r", std::slice::from_ref(&x));
        let positive_potential = clause(vec![
            predicate_literal(&mut bank, &p_x, true),
            predicate_literal(&mut bank, &q_x, true),
        ]);
        let negative_potential = clause(vec![
            predicate_literal(&mut bank, &p_x, false),
            predicate_literal(&mut bank, &r_x, true),
        ]);
        let passive = ClauseSet::from_clauses([positive_potential, negative_potential]);

        assert!(predicate_elimination_needs_gate_validation(
            &passive,
            &bank,
            PredicateEliminationConfig::default()
        ));
    }

    #[test]
    fn recognized_gate_eliminates_offending_predicate_occurrences() {
        let mut bank = test_bank();
        let x = object_var(&bank, -2);
        let a = object_const(&mut bank, "pe_gate_off_a");
        let b = object_const(&mut bank, "pe_gate_off_b");
        let p_x = predicate_atom(&mut bank, "pe_gate_off_p", std::slice::from_ref(&x));
        let p_a = predicate_atom(&mut bank, "pe_gate_off_p", std::slice::from_ref(&a));
        let p_b = predicate_atom(&mut bank, "pe_gate_off_p", std::slice::from_ref(&b));
        let positive_gate = clause(vec![
            predicate_literal(&mut bank, &p_x, true),
            Eqn::alloc(a.clone(), a.clone(), &mut bank, true).unwrap(),
        ]);
        let negative_gate = clause(vec![
            predicate_literal(&mut bank, &p_x, false),
            Eqn::alloc(a.clone(), a.clone(), &mut bank, false).unwrap(),
        ]);
        let offending = clause(vec![
            predicate_literal(&mut bank, &p_a, true),
            predicate_literal(&mut bank, &p_b, false),
        ]);
        let positive_gate_id = positive_gate.ident();
        let negative_gate_id = negative_gate.ident();
        let offending_id = offending.ident();
        let p_code = p_x.f_code();
        let mut passive = ClauseSet::from_clauses([positive_gate, negative_gate, offending]);
        let mut archive = ClauseSet::new();
        let mut tmp = tmp_bank(&bank);
        let fresh = fresh_vars(&bank);
        let equality_tautology = clause(vec![
            Eqn::alloc(a.clone(), a.clone(), &mut bank, true).unwrap(),
            Eqn::alloc(a.clone(), a.clone(), &mut bank, false).unwrap(),
        ]);
        assert!(clause_is_tautology_real(&mut tmp, &equality_tautology, false).unwrap());

        let result = eliminate_predicates_singular(
            &mut passive,
            &mut archive,
            &mut bank,
            &mut tmp,
            &fresh,
            PredicateEliminationConfig {
                recognize_gates: true,
                ..PredicateEliminationConfig::default()
            },
        )
        .unwrap();

        assert_eq!(
            result,
            PredicateEliminationResult {
                start_count: 3,
                eliminated_count: 3,
                generated_count: 0
            }
        );
        assert!(passive.is_empty());
        assert!(ids(&archive).contains(&positive_gate_id));
        assert!(ids(&archive).contains(&negative_gate_id));
        assert!(ids(&archive).contains(&offending_id));
        assert!(passive.iter().all(|clause| clause
            .literals()
            .as_slice()
            .iter()
            .all(|literal| literal.is_equ_lit(&bank) || literal.left().f_code() != p_code)));
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
