use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProverResult;
use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{clause_push_derivation, DC_CNF_ADD_ARG, DC_SAT_GEN};
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::PatEqnDirection;
use crate::clauses::proofstate::ProofState;
use crate::heuristics::hcb::GroundingStrategy;
use crate::terms::functypes::FunCode;
use crate::terms::replace::term_follow_rw_chain;
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{term_identity_cmp, Term};
use std::collections::BTreeMap;
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
struct SatClause {
    literals: Vec<i32>,
    source: Clause,
    has_pure_lit: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SatClauseSet {
    renumber_index: BTreeMap<i64, i32>,
    max_lit: i32,
    clauses: Vec<SatClause>,
    exported: Vec<usize>,
    core_size: u64,
}

impl SatClauseSet {
    fn import_clause(&mut self, bank: &mut TermBank, clause: &Clause) -> Result<(), Diagnostic> {
        let mut literals = Vec::with_capacity(clause.literal_number());
        for literal in clause.literals().as_slice() {
            literals.push(self.translate_literal(bank, literal)?);
        }
        self.clauses.push(SatClause {
            literals,
            source: clause.clone(),
            has_pure_lit: false,
        });
        Ok(())
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
        self.mark_pure();
        self.exported = self
            .clauses
            .iter()
            .enumerate()
            .filter_map(|(index, clause)| (!clause.has_pure_lit).then_some(index))
            .collect();

        let solver_clauses: Vec<Vec<i32>> = self
            .exported
            .iter()
            .map(|index| self.clauses[*index].literals.clone())
            .collect();
        match solve_sat(&solver_clauses, self.max_lit, decision_limit) {
            SolverStatus::Sat => (ProverResult::Satisfiable, None),
            SolverStatus::GaveUp => (ProverResult::GaveUp, None),
            SolverStatus::Unsat => {
                self.core_size = usize_to_u64(self.exported.len());
                (
                    ProverResult::Unsatisfiable,
                    Some(self.empty_clause_from_exported_core()),
                )
            }
        }
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

    fn empty_clause_from_exported_core(&self) -> Clause {
        let mut empty = Clause::empty();
        let mut sources = self
            .exported
            .iter()
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
    let encoding_start = Instant::now();
    let source_clauses = proof_state_sat_source_clauses(state);
    let mut dist_array = signature_distribution_array(state);
    let mut conj_dist_array = signature_distribution_array(state);
    state.axioms().add_symbol_distribution(&mut dist_array);
    state
        .axioms()
        .add_conj_symbol_distribution(&mut conj_dist_array);

    let mut satset = SatClauseSet::default();
    {
        let bank = state.terms_mut();
        let mut substitution =
            pseudo_ground_substitution(bank, grounding, norm_const, &conj_dist_array, &dist_array)?;
        for clause in &source_clauses {
            satset.import_clause(bank, clause)?;
        }
        substitution.backtrack();
    }
    let encoding_time = encoding_start.elapsed().as_secs_f64();

    let solver_start = Instant::now();
    let (result, empty) = satset.check_unsat(decision_limit);
    let solver_time = solver_start.elapsed().as_secs_f64();

    Ok(SatCheckReport {
        result,
        empty,
        full_size: usize_to_u64(satset.clauses.len()),
        actual_size: usize_to_u64(satset.exported.len()),
        core_size: satset.core_size,
        encoding_time,
        solver_time,
    })
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
    conj_dist_array: &[i64],
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
    conj_dist_array: &[i64],
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
        || (left_conj == right_conj && dist_at(dist_array, left) > dist_at(dist_array, right))
}

fn prefer_conj_max_max_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &[i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    left_conj > right_conj
        || (left_conj == right_conj && dist_at(dist_array, left) > dist_at(dist_array, right))
}

fn prefer_conj_min_min_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &[i64],
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
        || (left_conj == right_conj && dist_at(dist_array, left) < dist_at(dist_array, right))
}

fn prefer_conj_max_min_freq(
    left: FunCode,
    right: FunCode,
    conj_dist_array: &[i64],
    dist_array: &[i64],
) -> bool {
    let left_conj = dist_at(conj_dist_array, left);
    let right_conj = dist_at(conj_dist_array, right);
    left_conj > right_conj
        || (left_conj == right_conj && dist_at(dist_array, left) < dist_at(dist_array, right))
}

fn prefer_global_max_freq(
    left: FunCode,
    right: FunCode,
    _conj_dist_array: &[i64],
    dist_array: &[i64],
) -> bool {
    dist_at(dist_array, left) > dist_at(dist_array, right)
}

fn prefer_global_min_freq(
    left: FunCode,
    right: FunCode,
    _conj_dist_array: &[i64],
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

#[cfg(test)]
mod tests {
    use super::{sat_check_proof_state, solve_sat, SatClause, SatClauseSet, SolverStatus};
    use crate::basics::simple_stuff::ProverResult;
    use crate::clauses::clause::Clause;
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

        assert_eq!(set.mark_pure(), 2);
        assert!(set.clauses[0].has_pure_lit);
        assert!(set.clauses[1].has_pure_lit);
        assert!(!set.clauses[2].has_pure_lit);
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
