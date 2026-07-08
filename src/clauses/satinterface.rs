use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProverResult;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_CNF_ADD_ARG, DC_SAT_GEN};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::PatEqnDirection;
use crate::clauses::picosat::{PicoSat, PicoSatError, PicoSatSolveResult};
use crate::clauses::proofstate::ProofState;
use crate::heuristics::hcb::GroundingStrategy;
use crate::terms::functypes::FunCode;
use crate::terms::replace::term_follow_rw_chain;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_cmp, Term};
use std::collections::BTreeMap;
use std::fmt;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub struct SatCheckReport {
    pub result: ProverResult,
    pub empty: Option<Clause>,
    pub full_size: u64,
    pub actual_size: u64,
    pub core_size: u64,
    pub encoding_time: f64,
    pub solver_time: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SatClause {
    literals: Vec<i32>,
    source: Clause,
    has_pure_lit: bool,
}

impl SatClause {
    #[must_use]
    pub fn literals(&self) -> &[i32] {
        &self.literals
    }

    #[must_use]
    pub const fn source(&self) -> &Clause {
        &self.source
    }

    #[must_use]
    pub const fn has_pure_lit(&self) -> bool {
        self.has_pure_lit
    }

    /// Writes this propositional clause in C `SatClausePrint` DIMACS shape.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if `output` rejects a write.
    pub fn write_dimacs(&self, output: &mut impl fmt::Write) -> fmt::Result {
        for literal in &self.literals {
            write!(output, "{literal} ")?;
        }
        writeln!(output, "0")
    }

    #[must_use]
    pub fn dimacs_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_dimacs(&mut output);
        output
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SatClauseSet {
    renumber_index: BTreeMap<i64, i32>,
    max_lit: i32,
    clauses: Vec<SatClause>,
    exported: Vec<usize>,
    core: Vec<usize>,
    core_size: u64,
    set_size_limit: i64,
}

impl Default for SatClauseSet {
    fn default() -> Self {
        Self {
            renumber_index: BTreeMap::new(),
            max_lit: 0,
            clauses: Vec::new(),
            exported: Vec::new(),
            core: Vec::new(),
            core_size: 0,
            set_size_limit: -1,
        }
    }
}

impl SatClauseSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_max_clauses(&mut self, limit: i64) {
        self.set_size_limit = limit;
    }

    #[must_use]
    pub const fn max_lit(&self) -> i32 {
        self.max_lit
    }

    #[must_use]
    pub fn cardinality(&self) -> usize {
        self.clauses.len()
    }

    #[must_use]
    pub fn non_pure_cardinality(&self) -> usize {
        self.exported.len()
    }

    #[must_use]
    pub const fn core_size(&self) -> u64 {
        self.core_size
    }

    #[must_use]
    pub fn limit_reached(&self) -> bool {
        self.set_size_limit == usize_to_i64(self.clauses.len())
    }

    #[must_use]
    pub fn clauses(&self) -> &[SatClause] {
        &self.clauses
    }

    /// Encodes one instantiated clause and appends it unless the C insertion
    /// limit has been reached.
    ///
    /// Returns `Ok(false)` when `set_size_limit != -1` and the current
    /// cardinality is greater than or equal to that signed limit, matching
    /// `SatClauseCreateAndStore` returning `NULL`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal instantiation or SAT atom encoding
    /// fails.
    pub fn import_clause(
        &mut self,
        bank: &mut TermBank,
        clause: &Clause,
    ) -> Result<bool, Diagnostic> {
        self.import_clause_with_source(bank, clause, clause.clone())
    }

    /// Encodes `clause` but records `source` as the clause returned for unsat
    /// core extraction.
    ///
    /// C gate-recognition builds fresh SAT environment clauses and then maps
    /// the extracted core back to the original clauses. This helper keeps that
    /// ownership transition explicit for Rust callers.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal instantiation or SAT atom encoding
    /// fails.
    pub fn import_clause_with_source(
        &mut self,
        bank: &mut TermBank,
        clause: &Clause,
        source: Clause,
    ) -> Result<bool, Diagnostic> {
        if self.set_size_limit != -1 && usize_to_i64(self.clauses.len()) >= self.set_size_limit {
            return Ok(false);
        }

        let mut literals = Vec::with_capacity(clause.literal_number());
        for literal in clause.literals().as_slice() {
            literals.push(self.translate_literal(bank, literal)?);
        }
        self.clauses.push(SatClause {
            literals,
            source,
            has_pure_lit: false,
        });
        Ok(true)
    }

    /// Imports clauses in set order until insertion reaches the C limit.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic from [`Self::import_clause`] if any literal cannot
    /// be encoded.
    pub fn import_clause_set(
        &mut self,
        bank: &mut TermBank,
        clauses: &ClauseSet,
    ) -> Result<i64, Diagnostic> {
        let mut added = 0_i64;
        for clause in clauses.iter() {
            if !self.import_clause(bank, clause)? {
                break;
            }
            added = added.saturating_add(1);
        }
        Ok(added)
    }

    /// Writes this propositional clause set in C `SatClauseSetPrint` DIMACS shape.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if `output` rejects a write.
    pub fn write_dimacs(&self, output: &mut impl fmt::Write) -> fmt::Result {
        writeln!(output, "p cnf {} {}", self.max_lit, self.clauses.len())?;
        for clause in &self.clauses {
            clause.write_dimacs(output)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn dimacs_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_dimacs(&mut output);
        output
    }

    /// Marks clauses with pure literals and refreshes the C `exported` subset
    /// to contain only clauses without pure literals.
    ///
    /// Returns the number of clauses marked as having at least one pure
    /// literal, matching `SatClauseSetMarkPure`.
    #[must_use]
    pub fn mark_pure_and_export_non_pure(&mut self) -> u64 {
        let pure = self.mark_pure();
        self.export_non_pure();
        pure
    }

    pub fn export_all_to_solver_clauses(&mut self) -> Vec<Vec<i32>> {
        self.exported = (0..self.clauses.len()).collect();
        self.solver_clauses_for_indices(&self.exported)
    }

    pub fn export_non_pure_to_solver_clauses(&mut self) -> Vec<Vec<i32>> {
        let _ = self.mark_pure_and_export_non_pure();
        self.solver_clauses_for_indices(&self.exported)
    }

    #[must_use]
    pub fn exported_indices(&self) -> &[usize] {
        &self.exported
    }

    /// Checks for unsatisfiability and returns the extracted core clauses.
    ///
    /// This mirrors `SatClauseSetCheckAndGetCore`: it refreshes pure-literal
    /// marks, exports only non-pure clauses, uses C's fixed decision limit of
    /// 10000, and returns `None` for satisfiable or gave-up solver results.
    /// The default internal solver supplies a deletion-minimized core in
    /// exported-clause order; callers with a runtime-loaded `PicoSAT` backend
    /// use `check_and_get_core_with_picosat` to read solver-reported core
    /// indices.
    #[must_use]
    pub fn check_and_get_core(&mut self) -> Option<Vec<Clause>> {
        let solver_clauses = self.export_non_pure_to_solver_clauses();
        if solve_sat(&solver_clauses, self.max_lit, 10_000) != SolverStatus::Unsat {
            return None;
        }
        let core = self.minimize_exported_core(10_000);
        Some(self.clauses_for_indices(&core))
    }

    pub fn check_and_get_core_with_picosat(
        &mut self,
        solver: &mut PicoSat,
    ) -> Result<Option<Vec<Clause>>, PicoSatError> {
        let solver_clauses = self.export_non_pure_to_solver_clauses();
        solver.add_clauses(&solver_clauses)?;
        if solver.solve(10_000) != PicoSatSolveResult::Unsatisfiable {
            return Ok(None);
        }
        let solver_core = solver.core_indices(self.exported.len())?;
        let core = self.exported_core_from_solver_core(&solver_core);
        Ok(Some(self.clauses_for_indices(&core)))
    }

    fn translate_literal(&mut self, bank: &mut TermBank, literal: &Eqn) -> Result<i32, Diagnostic> {
        let atom_term = if literal.is_equ_lit(bank) {
            let left = bank.insert_instantiated(literal.left())?;
            let right = bank.insert_instantiated(literal.right())?;
            let direction = if term_identity_cmp(&left, &right) > 0 {
                PatEqnDirection::Normal
            } else {
                PatEqnDirection::Reverse
            };
            Eqn::terms_tb_term_encode(bank, &left, &right, true, direction)?
        } else {
            bank.insert_instantiated(literal.left())?
        };

        let atom = self.renumber_atom(atom_term.entry_no())?;
        Ok(if literal.is_positive() { atom } else { -atom })
    }

    fn renumber_atom(&mut self, lit_code: i64) -> Result<i32, Diagnostic> {
        if let Some(atom) = self.renumber_index.get(&lit_code) {
            return Ok(*atom);
        }
        let next = self.max_lit.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "SAT literal renumbering exceeded C int range",
            )
        })?;
        self.max_lit = next;
        self.renumber_index.insert(lit_code, next);
        Ok(next)
    }

    fn check_unsat(&mut self, decision_limit: i32) -> (ProverResult, Option<Clause>) {
        let _ = self.mark_pure_and_export_non_pure();
        self.core.clear();
        self.core_size = 0;

        let solver_clauses = self.export_non_pure_to_solver_clauses();
        match solve_sat(&solver_clauses, self.max_lit, decision_limit) {
            SolverStatus::Sat => (ProverResult::Satisfiable, None),
            SolverStatus::GaveUp => (ProverResult::GaveUp, None),
            SolverStatus::Unsat => {
                self.core = self.minimize_exported_core(decision_limit);
                self.core_size = usize_to_u64(self.core.len());
                (
                    ProverResult::Unsatisfiable,
                    Some(self.empty_clause_from_core()),
                )
            }
        }
    }

    pub fn check_unsat_with_picosat(
        &mut self,
        solver: &mut PicoSat,
        decision_limit: i32,
    ) -> Result<(ProverResult, Option<Clause>), PicoSatError> {
        self.core.clear();
        self.core_size = 0;

        let solver_clauses = self.export_non_pure_to_solver_clauses();
        solver.add_clauses(&solver_clauses)?;
        match solver.solve(decision_limit) {
            PicoSatSolveResult::Satisfiable => Ok((ProverResult::Satisfiable, None)),
            PicoSatSolveResult::GaveUp => Ok((ProverResult::GaveUp, None)),
            PicoSatSolveResult::Unsatisfiable => {
                let solver_core = solver.core_indices(self.exported.len())?;
                self.core = self.exported_core_from_solver_core(&solver_core);
                self.core_size = usize_to_u64(self.core.len());
                Ok((
                    ProverResult::Unsatisfiable,
                    Some(self.empty_clause_from_core()),
                ))
            }
        }
    }

    fn solver_clauses_for_indices(&self, indices: &[usize]) -> Vec<Vec<i32>> {
        indices
            .iter()
            .map(|index| self.clauses[*index].literals.clone())
            .collect()
    }

    fn clauses_for_indices(&self, indices: &[usize]) -> Vec<Clause> {
        indices
            .iter()
            .map(|index| self.clauses[*index].source.clone())
            .collect()
    }

    fn exported_core_from_solver_core(&self, solver_core: &[usize]) -> Vec<usize> {
        solver_core
            .iter()
            .map(|solver_index| self.exported[*solver_index])
            .collect()
    }

    fn minimize_exported_core(&self, decision_limit: i32) -> Vec<usize> {
        if self.exported.len() <= 1 {
            return self.exported.clone();
        }

        let mut core = self.exported.clone();
        let mut index = 0;
        while index < core.len() {
            let mut trial = core.clone();
            trial.remove(index);
            if trial.is_empty() {
                index += 1;
                continue;
            }

            let solver_clauses = self.solver_clauses_for_indices(&trial);
            if solve_sat(&solver_clauses, self.max_lit, decision_limit) == SolverStatus::Unsat {
                core = trial;
            } else {
                index += 1;
            }
        }
        core
    }

    fn mark_pure(&mut self) -> u64 {
        let mut lit_state = BTreeMap::<i32, u8>::new();
        for clause in &mut self.clauses {
            clause.has_pure_lit = false;
            for lit in &clause.literals {
                let atom = lit.abs();
                let flag = if *lit > 0 { 1 } else { 2 };
                lit_state
                    .entry(atom)
                    .and_modify(|state| *state |= flag)
                    .or_insert(flag);
            }
        }

        let mut pure_clauses = 0_u64;
        for clause in &mut self.clauses {
            if clause.literals.iter().any(|lit| {
                lit_state
                    .get(&lit.abs())
                    .copied()
                    .is_some_and(|state| state != 3)
            }) {
                clause.has_pure_lit = true;
                pure_clauses = pure_clauses.saturating_add(1);
            }
        }
        pure_clauses
    }

    fn export_non_pure(&mut self) {
        self.exported = self
            .clauses
            .iter()
            .enumerate()
            .filter_map(|(index, clause)| (!clause.has_pure_lit).then_some(index))
            .collect();
    }

    fn empty_clause_from_core(&self) -> Clause {
        let mut empty = Clause::empty();
        let mut sources = self
            .core
            .iter()
            .rev()
            .map(|index| &self.clauses[*index].source);
        if let Some(parent) = sources.next() {
            clause_push_derivation(&mut empty, DC_SAT_GEN, Some(parent), None);
            for parent in sources {
                clause_push_derivation(&mut empty, DC_CNF_ADD_ARG, Some(parent), None);
            }
        }
        empty
    }
}

pub fn sat_check_proof_state(
    state: &mut ProofState,
    grounding: GroundingStrategy,
    norm_const: bool,
    decision_limit: i32,
) -> Result<SatCheckReport, Diagnostic> {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::SatTimer);
    let (mut satset, encoding_time) = encode_sat_check_set(state, grounding, norm_const)?;
    let solver_start = Instant::now();
    let (result, empty) = satset.check_unsat(decision_limit);
    let solver_time = solver_start.elapsed().as_secs_f64();

    Ok(sat_check_report(
        &satset,
        result,
        empty,
        encoding_time,
        solver_time,
    ))
}

pub fn sat_check_proof_state_with_picosat(
    state: &mut ProofState,
    grounding: GroundingStrategy,
    norm_const: bool,
    decision_limit: i32,
    solver: &mut PicoSat,
) -> Result<SatCheckReport, Diagnostic> {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::SatTimer);
    let (mut satset, encoding_time) = encode_sat_check_set(state, grounding, norm_const)?;
    let solver_start = Instant::now();
    let (result, empty) = satset
        .check_unsat_with_picosat(solver, decision_limit)
        .map_err(|error| picosat_error_to_diagnostic(&error))?;
    let solver_time = solver_start.elapsed().as_secs_f64();

    Ok(sat_check_report(
        &satset,
        result,
        empty,
        encoding_time,
        solver_time,
    ))
}

#[must_use]
pub fn picosat_error_to_diagnostic(error: &PicoSatError) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::INTERFACE_ERROR,
        format!("PicoSAT communication failed: {error}"),
    )
}

fn encode_sat_check_set(
    state: &mut ProofState,
    grounding: GroundingStrategy,
    norm_const: bool,
) -> Result<(SatClauseSet, f64), Diagnostic> {
    let encoding_start = Instant::now();
    let source_clauses = proof_state_sat_source_clauses(state);
    let mut dist_array = signature_distribution_array(state);
    let mut conj_dist_array = signature_distribution_array(state);
    state.axioms().add_symbol_distribution(&mut dist_array);
    state
        .axioms()
        .add_conj_symbol_distribution(&mut conj_dist_array);

    let mut satset = SatClauseSet::new();
    {
        let bank = state.terms_mut();
        let mut substitution = pseudo_ground_substitution(
            bank,
            grounding,
            norm_const,
            &mut conj_dist_array,
            &dist_array,
        )?;
        for clause in &source_clauses {
            let _ = satset.import_clause(bank, clause)?;
        }
        substitution.backtrack();
    }
    let encoding_time = encoding_start.elapsed().as_secs_f64();

    Ok((satset, encoding_time))
}

fn sat_check_report(
    satset: &SatClauseSet,
    result: ProverResult,
    empty: Option<Clause>,
    encoding_time: f64,
    solver_time: f64,
) -> SatCheckReport {
    SatCheckReport {
        result,
        empty,
        full_size: usize_to_u64(satset.cardinality()),
        actual_size: usize_to_u64(satset.non_pure_cardinality()),
        core_size: satset.core_size(),
        encoding_time,
        solver_time,
    }
}

fn proof_state_sat_source_clauses(state: &ProofState) -> Vec<Clause> {
    let mut clauses = Vec::new();
    append_clause_set(&mut clauses, state.processed_pos_rules());
    append_clause_set(&mut clauses, state.processed_pos_eqns());
    append_clause_set(&mut clauses, state.processed_neg_units());
    append_clause_set(&mut clauses, state.processed_non_units());
    append_clause_set(&mut clauses, state.unprocessed());
    clauses
}

fn append_clause_set(clauses: &mut Vec<Clause>, set: &ClauseSet) {
    clauses.extend(set.iter().cloned());
}

fn signature_distribution_array(state: &ProofState) -> Vec<i64> {
    usize::try_from(state.terms().signature().f_count())
        .unwrap_or(usize::MAX.saturating_sub(1))
        .saturating_add(1)
        .checked_add(1)
        .map_or_else(Vec::new, |len| vec![0; len])
}

fn pseudo_ground_substitution(
    bank: &mut TermBank,
    grounding: GroundingStrategy,
    norm_const: bool,
    conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> Result<Substitution, Diagnostic> {
    let varstacks = bank.vars().normal_variables_by_sort();
    let mut substitution = Substitution::new();
    for vars in varstacks.values() {
        let Some(first) = vars.first() else {
            continue;
        };
        let type_ = first
            .type_()
            .expect("varbank variables must have initialized types");
        let norm = match grounding {
            GroundingStrategy::NoGrounding | GroundingStrategy::PseudoVar => first.clone(),
            GroundingStrategy::FirstConst => {
                normalize_grounding_or_first(bank.get_first_const_term(&type_)?, first, norm_const)
            }
            GroundingStrategy::ConjMinMinFreq => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_conj_min_min_freq,
                )?,
                first,
                norm_const,
            ),
            GroundingStrategy::ConjMaxMinFreq => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_conj_max_min_freq,
                )?,
                first,
                norm_const,
            ),
            GroundingStrategy::ConjMinMaxFreq => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_conj_min_max_freq,
                )?,
                first,
                norm_const,
            ),
            GroundingStrategy::ConjMaxMaxFreq => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_conj_max_max_freq,
                )?,
                first,
                norm_const,
            ),
            GroundingStrategy::GlobalMax => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_global_max_freq,
                )?,
                first,
                norm_const,
            ),
            GroundingStrategy::GlobalMin => normalize_grounding_or_first(
                bank.get_freq_const_term(
                    &type_,
                    conj_dist_array,
                    dist_array,
                    prefer_global_min_freq,
                )?,
                first,
                norm_const,
            ),
        };
        for var in vars {
            if var.binding().is_none() {
                substitution.add_binding(var, &norm);
            }
        }
    }
    Ok(substitution)
}

fn normalize_grounding_or_first(selected: Option<Term>, first: &Term, norm_const: bool) -> Term {
    selected.map_or_else(|| first.clone(), |term| maybe_follow_rw(term, norm_const))
}

fn maybe_follow_rw(term: Term, norm_const: bool) -> Term {
    if norm_const {
        term_follow_rw_chain(&term)
    } else {
        term
    }
}

fn prefer_conj_min_max_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    if left_conj != 0 && right_conj == 0 {
        return true;
    }
    if left_conj == 0 && right_conj != 0 {
        return false;
    }
    left_conj < right_conj
        || (assign_conj_dist(conj_dist_array, left, right_conj) != 0
            && dist_at(dist_array, left) > dist_at(dist_array, right))
}

fn prefer_conj_max_max_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    left_conj > right_conj
        || (assign_conj_dist(conj_dist_array, left, right_conj) != 0
            && dist_at(dist_array, left) > dist_at(dist_array, right))
}

fn prefer_conj_min_min_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    if left_conj != 0 && right_conj == 0 {
        return true;
    }
    if left_conj == 0 && right_conj != 0 {
        return false;
    }
    left_conj < right_conj
        || (assign_conj_dist(conj_dist_array, left, right_conj) != 0
            && dist_at(dist_array, left) < dist_at(dist_array, right))
}

fn prefer_conj_max_min_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    left_conj > right_conj
        || (assign_conj_dist(conj_dist_array, left, right_conj) != 0
            && dist_at(dist_array, left) < dist_at(dist_array, right))
}

fn prefer_global_max_freq(
    left: FunCode,
    right: FunCode,
    _conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    dist_at(dist_array, left) > dist_at(dist_array, right)
}

fn prefer_global_min_freq(
    left: FunCode,
    right: FunCode,
    _conj_dist_array: &mut [i64],
    dist_array: &[i64],
) -> bool {
    dist_at(dist_array, left) < dist_at(dist_array, right)
}

fn dist_at(dist_array: &[i64], f_code: FunCode) -> i64 {
    usize::try_from(f_code)
        .ok()
        .and_then(|index| dist_array.get(index))
        .copied()
        .unwrap_or(0)
}

fn assign_conj_dist(dist_array: &mut [i64], f_code: FunCode, value: i64) -> i64 {
    let Some(slot) = usize::try_from(f_code)
        .ok()
        .and_then(|index| dist_array.get_mut(index))
    else {
        return 0;
    };
    *slot = value;
    *slot
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SolverStatus {
    Sat,
    Unsat,
    GaveUp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClauseStatus {
    Satisfied,
    Conflict,
    Unit(i32),
    Open,
}

fn solve_sat(clauses: &[Vec<i32>], max_lit: i32, decision_limit: i32) -> SolverStatus {
    let len = usize::try_from(max_lit)
        .unwrap_or(usize::MAX.saturating_sub(1))
        .saturating_add(1);
    let mut assignment = vec![None; len];
    let mut budget = (decision_limit >= 0).then_some(i64::from(decision_limit));
    dpll(clauses, &mut assignment, &mut budget)
}

fn dpll(
    clauses: &[Vec<i32>],
    assignment: &mut [Option<bool>],
    budget: &mut Option<i64>,
) -> SolverStatus {
    loop {
        let mut changed = false;
        for clause in clauses {
            match clause_status(clause, assignment) {
                ClauseStatus::Satisfied | ClauseStatus::Open => {}
                ClauseStatus::Conflict => return SolverStatus::Unsat,
                ClauseStatus::Unit(lit) => {
                    if !assign_literal(assignment, lit) {
                        return SolverStatus::Unsat;
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    if clauses
        .iter()
        .all(|clause| clause_status(clause, assignment) == ClauseStatus::Satisfied)
    {
        return SolverStatus::Sat;
    }

    let Some(branch_lit) = first_open_literal(clauses, assignment) else {
        return SolverStatus::Unsat;
    };
    if let Some(remaining) = budget {
        if *remaining == 0 {
            return SolverStatus::GaveUp;
        }
        *remaining -= 1;
    }

    let preferred = branch_lit > 0;
    for value in [preferred, !preferred] {
        let mut next = assignment.to_vec();
        if !assign_var(&mut next, branch_lit.abs(), value) {
            continue;
        }
        match dpll(clauses, &mut next, budget) {
            SolverStatus::Sat => return SolverStatus::Sat,
            SolverStatus::GaveUp => return SolverStatus::GaveUp,
            SolverStatus::Unsat => {}
        }
    }
    SolverStatus::Unsat
}

fn clause_status(clause: &[i32], assignment: &[Option<bool>]) -> ClauseStatus {
    let mut open_lit = None;
    for &lit in clause {
        let atom = lit_index(lit);
        match assignment.get(atom).copied().flatten() {
            Some(value) if value == (lit > 0) => return ClauseStatus::Satisfied,
            Some(_) => {}
            None if open_lit.is_none() => open_lit = Some(lit),
            None => return ClauseStatus::Open,
        }
    }
    open_lit.map_or(ClauseStatus::Conflict, ClauseStatus::Unit)
}

fn first_open_literal(clauses: &[Vec<i32>], assignment: &[Option<bool>]) -> Option<i32> {
    clauses
        .iter()
        .filter(|clause| clause_status(clause, assignment) != ClauseStatus::Satisfied)
        .flat_map(|clause| clause.iter().copied())
        .find(|lit| assignment.get(lit_index(*lit)).is_some_and(Option::is_none))
}

fn assign_literal(assignment: &mut [Option<bool>], lit: i32) -> bool {
    assign_var(assignment, lit.abs(), lit > 0)
}

fn assign_var(assignment: &mut [Option<bool>], atom: i32, value: bool) -> bool {
    let index = usize::try_from(atom).unwrap_or(usize::MAX);
    let Some(slot) = assignment.get_mut(index) else {
        return false;
    };
    if let Some(existing) = *slot {
        existing == value
    } else {
        *slot = Some(value);
        true
    }
}

fn lit_index(lit: i32) -> usize {
    usize::try_from(lit.abs()).unwrap_or(usize::MAX)
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        prefer_conj_max_max_freq, prefer_conj_max_min_freq, prefer_conj_min_max_freq,
        prefer_conj_min_min_freq, sat_check_proof_state, solve_sat, SatClause, SatClauseSet,
        SolverStatus,
    };
    use crate::basics::simple_stuff::ProverResult;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        derivation_entries, ClauseDerivationRef, DerivationEntry, DC_CNF_ADD_ARG, DC_SAT_GEN,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::proofstate::proof_state_alloc;
    use crate::heuristics::hcb::GroundingStrategy;
    use crate::terms::signature::FP_IGNORE_PROPS;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn unit_clause(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Clause {
        Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
            left.clone(),
            right.clone(),
            bank,
            positive,
        )
        .unwrap()]))
    }

    #[test]
    fn pure_literal_marking_filters_clauses_with_any_pure_literal() {
        let mut set = SatClauseSet {
            max_lit: 4,
            clauses: vec![
                SatClause {
                    literals: vec![1, 2],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1, 3],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1, 1],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
            ],
            ..SatClauseSet::default()
        };

        assert_eq!(set.mark_pure_and_export_non_pure(), 2);
        assert!(set.clauses[0].has_pure_lit);
        assert!(set.clauses[1].has_pure_lit);
        assert!(!set.clauses[2].has_pure_lit);
        assert_eq!(set.exported, vec![2]);
        assert_eq!(set.non_pure_cardinality(), 1);
    }

    #[test]
    fn solver_export_helpers_preserve_c_stack_order_and_filter_shape() {
        let mut set = SatClauseSet {
            max_lit: 4,
            clauses: vec![
                SatClause {
                    literals: vec![1, 2],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1, 3],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1, 1],
                    source: Clause::empty(),
                    has_pure_lit: false,
                },
            ],
            ..SatClauseSet::default()
        };

        assert_eq!(
            set.export_all_to_solver_clauses(),
            vec![vec![1, 2], vec![-1, 3], vec![-1, 1]]
        );
        assert_eq!(set.exported_indices(), &[0, 1, 2]);

        assert_eq!(set.export_non_pure_to_solver_clauses(), vec![vec![-1, 1]]);
        assert_eq!(set.exported_indices(), &[2]);
        assert!(set.clauses[0].has_pure_lit);
        assert!(set.clauses[1].has_pure_lit);
        assert!(!set.clauses[2].has_pure_lit);

        assert_eq!(
            set.export_all_to_solver_clauses(),
            vec![vec![1, 2], vec![-1, 3], vec![-1, 1]]
        );
        assert_eq!(set.exported_indices(), &[0, 1, 2]);
    }

    #[test]
    fn sat_clause_and_set_dimacs_rendering_matches_c_shape() {
        let clause = SatClause {
            literals: vec![1, -2],
            source: Clause::empty(),
            has_pure_lit: false,
        };
        let empty = SatClause {
            literals: Vec::new(),
            source: Clause::empty(),
            has_pure_lit: false,
        };
        let set = SatClauseSet {
            max_lit: 2,
            clauses: vec![clause.clone(), empty],
            ..SatClauseSet::default()
        };

        assert_eq!(clause.dimacs_string(), "1 -2 0\n");
        assert_eq!(set.dimacs_string(), "p cnf 2 2\n1 -2 0\n0\n");
    }

    #[test]
    fn sat_clause_set_import_honors_c_signed_limit() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first, second) = {
            let bank = state.terms_mut();
            let a = typed_const(bank, "sat_limit_a");
            let b = typed_const(bank, "sat_limit_b");
            (
                unit_clause(bank, &a, &a, true),
                unit_clause(bank, &b, &b, true),
            )
        };
        let clauses = ClauseSet::from_clauses([first.clone(), second]);
        let mut satset = SatClauseSet::new();

        satset.set_max_clauses(1);
        assert!(!satset.limit_reached());
        assert_eq!(
            satset
                .import_clause_set(state.terms_mut(), &clauses)
                .unwrap(),
            1
        );
        assert_eq!(satset.cardinality(), 1);
        assert!(satset.limit_reached());
        assert_eq!(
            satset
                .import_clause_set(state.terms_mut(), &clauses)
                .unwrap(),
            0
        );

        satset.set_max_clauses(-2);
        assert!(!satset.limit_reached());
        assert!(!satset.import_clause(state.terms_mut(), &first).unwrap());
        assert_eq!(satset.cardinality(), 1);
    }

    #[test]
    fn dpll_detects_unit_contradiction_and_models() {
        assert_eq!(solve_sat(&[vec![1], vec![-1]], 1, -1), SolverStatus::Unsat);
        assert_eq!(
            solve_sat(&[vec![1, 2], vec![-1, 2]], 2, -1),
            SolverStatus::Sat
        );
    }

    #[test]
    fn dpll_honors_zero_decision_limit_after_propagation() {
        assert_eq!(
            solve_sat(&[vec![1, 2], vec![-1, 2], vec![1, -2]], 2, 0),
            SolverStatus::GaveUp
        );
        assert_eq!(solve_sat(&[vec![], vec![1]], 1, 0), SolverStatus::Unsat);
    }

    #[test]
    fn conjecture_frequency_comparators_preserve_c_assignment_tie_breaks() {
        let mut min_max_conj = vec![0, 0, 7, 5];
        let min_max_dist = vec![0, 0, 30, 20];
        assert!(prefer_conj_min_max_freq(
            2,
            3,
            &mut min_max_conj,
            &min_max_dist
        ));
        assert_eq!(min_max_conj[2], 5);

        let mut min_min_conj = vec![0, 0, 7, 5];
        let min_min_dist = vec![0, 0, 10, 20];
        assert!(prefer_conj_min_min_freq(
            2,
            3,
            &mut min_min_conj,
            &min_min_dist
        ));
        assert_eq!(min_min_conj[2], 5);

        let mut max_max_conj = vec![0, 0, 5, 7];
        let max_max_dist = vec![0, 0, 30, 20];
        assert!(prefer_conj_max_max_freq(
            2,
            3,
            &mut max_max_conj,
            &max_max_dist
        ));
        assert_eq!(max_max_conj[2], 7);

        let mut max_min_conj = vec![0, 0, 5, 7];
        let max_min_dist = vec![0, 0, 10, 20];
        assert!(prefer_conj_max_min_freq(
            2,
            3,
            &mut max_min_conj,
            &max_min_dist
        ));
        assert_eq!(max_min_conj[2], 7);
    }

    #[test]
    fn unsat_empty_clause_derivation_uses_c_core_stack_pop_order() {
        let mut first = Clause::empty();
        first.set_ident(101);
        first.set_csscpa_source(1);
        let mut second = Clause::empty();
        second.set_ident(102);
        second.set_csscpa_source(1);
        let mut set = SatClauseSet {
            max_lit: 1,
            clauses: vec![
                SatClause {
                    literals: vec![1],
                    source: first.clone(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1],
                    source: second.clone(),
                    has_pure_lit: false,
                },
            ],
            ..SatClauseSet::default()
        };

        let (result, empty) = set.check_unsat(-1);
        let empty = empty.unwrap();

        assert_eq!(result, ProverResult::Unsatisfiable);
        assert_eq!(
            derivation_entries(&empty),
            &[
                DerivationEntry::Operation(DC_SAT_GEN),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&second)),
                DerivationEntry::Operation(DC_CNF_ADD_ARG),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&first)),
            ]
        );
    }

    #[test]
    fn unsat_core_minimization_keeps_exported_size_separate() {
        let mut redundant_pos = Clause::empty();
        redundant_pos.set_ident(201);
        redundant_pos.set_csscpa_source(1);
        let mut redundant_neg = Clause::empty();
        redundant_neg.set_ident(202);
        redundant_neg.set_csscpa_source(1);
        let mut positive = Clause::empty();
        positive.set_ident(203);
        positive.set_csscpa_source(1);
        let mut negative = Clause::empty();
        negative.set_ident(204);
        negative.set_csscpa_source(1);
        let mut set = SatClauseSet {
            max_lit: 2,
            clauses: vec![
                SatClause {
                    literals: vec![1, 2],
                    source: redundant_pos,
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![1, -2],
                    source: redundant_neg,
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![1],
                    source: positive.clone(),
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1],
                    source: negative.clone(),
                    has_pure_lit: false,
                },
            ],
            ..SatClauseSet::default()
        };

        let (result, empty) = set.check_unsat(-1);
        let empty = empty.unwrap();

        assert_eq!(result, ProverResult::Unsatisfiable);
        assert_eq!(set.exported, vec![0, 1, 2, 3]);
        assert_eq!(set.core, vec![2, 3]);
        assert_eq!(set.core_size, 2);
        assert_eq!(
            derivation_entries(&empty),
            &[
                DerivationEntry::Operation(DC_SAT_GEN),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&negative)),
                DerivationEntry::Operation(DC_CNF_ADD_ARG),
                DerivationEntry::ClauseParent(ClauseDerivationRef::from(&positive)),
            ]
        );
    }

    #[test]
    fn picosat_core_positions_map_through_exported_subset() {
        let set = SatClauseSet {
            exported: vec![1, 3, 4],
            ..SatClauseSet::default()
        };

        assert_eq!(
            set.exported_core_from_solver_core(&[2, 0, 1]),
            vec![4, 1, 3]
        );
    }

    #[test]
    fn check_and_get_core_returns_minimized_core_without_core_size_side_effect() {
        let mut redundant_pos = Clause::empty();
        redundant_pos.set_ident(301);
        redundant_pos.set_csscpa_source(1);
        let mut redundant_neg = Clause::empty();
        redundant_neg.set_ident(302);
        redundant_neg.set_csscpa_source(1);
        let mut positive = Clause::empty();
        positive.set_ident(303);
        positive.set_csscpa_source(1);
        let mut negative = Clause::empty();
        negative.set_ident(304);
        negative.set_csscpa_source(1);
        let mut set = SatClauseSet {
            max_lit: 2,
            clauses: vec![
                SatClause {
                    literals: vec![1, 2],
                    source: redundant_pos,
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![1, -2],
                    source: redundant_neg,
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![1],
                    source: positive,
                    has_pure_lit: false,
                },
                SatClause {
                    literals: vec![-1],
                    source: negative,
                    has_pure_lit: false,
                },
            ],
            ..SatClauseSet::default()
        };

        let core = set.check_and_get_core().unwrap();

        assert_eq!(set.exported, vec![0, 1, 2, 3]);
        assert_eq!(set.core_size(), 0);
        assert_eq!(
            core.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![303, 304]
        );

        let mut sat = SatClauseSet {
            max_lit: 2,
            clauses: vec![SatClause {
                literals: vec![1, 2],
                source: Clause::empty(),
                has_pure_lit: false,
            }],
            ..SatClauseSet::default()
        };
        assert!(sat.check_and_get_core().is_none());
        assert!(sat.exported.is_empty());
    }

    #[test]
    fn sat_check_pseudo_var_grounding_backtracks_self_bindings() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let x = typed_var(state.terms(), -2);
        let clause = unit_clause(state.terms_mut(), &x, &x, true);
        state.unprocessed_mut().insert(clause);

        let report =
            sat_check_proof_state(&mut state, GroundingStrategy::PseudoVar, false, -1).unwrap();

        assert_eq!(report.result, ProverResult::Satisfiable);
        assert!(report.empty.is_none());
        assert_eq!(report.full_size, 1);
        assert_eq!(report.actual_size, 0);
        assert!(x.binding().is_none());
    }

    #[test]
    fn sat_check_first_const_grounding_refutes_opposite_variable_units() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let a = typed_const(state.terms_mut(), "sat_first_const_a");
        let x = typed_var(state.terms(), -2);
        let y = typed_var(state.terms(), -4);
        let positive = unit_clause(state.terms_mut(), &x, &a, true);
        let negative = unit_clause(state.terms_mut(), &y, &a, false);
        state.unprocessed_mut().insert(positive);
        state.unprocessed_mut().insert(negative);

        let report =
            sat_check_proof_state(&mut state, GroundingStrategy::FirstConst, false, -1).unwrap();

        assert_eq!(report.result, ProverResult::Unsatisfiable);
        assert!(report.empty.as_ref().is_some_and(Clause::is_empty));
        assert_eq!(report.full_size, 2);
        assert_eq!(report.actual_size, 2);
        assert_eq!(report.core_size, 2);
        assert!(x.binding().is_none());
        assert!(y.binding().is_none());
    }
}
