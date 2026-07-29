use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A clonable, thread-safe cancellation signal for one or more SAT solves.
#[derive(Clone, Debug, Default)]
pub struct SatCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl SatCancellationToken {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// A request for an independently checked proof of an UNSAT result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SatProofRequest {
    pub trace_path: PathBuf,
    pub checker_path: PathBuf,
}

/// The retained evidence for an independently checked UNSAT proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SatVerifiedProof {
    pub format: &'static str,
    pub scope_path: PathBuf,
    pub trace_path: PathBuf,
    pub checker_path: PathBuf,
}

/// Per-call resource and certificate controls.
#[derive(Clone, Debug, Default)]
pub struct SatSolveOptions {
    pub decision_limit: Option<u64>,
    pub deadline: Option<Duration>,
    pub cancellation: SatCancellationToken,
    pub external_stop: Option<fn() -> bool>,
    pub proof: Option<SatProofRequest>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatUnknownReason {
    DecisionLimit,
    Deadline,
    Cancelled,
    ExternalStop,
    Backend,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SatServiceError {
    LiteralIsZero,
    LiteralOutOfRange(i32),
    DecisionLimitOutOfRange(u64),
    DeadlineOutOfRange,
    Backend(String),
    CallbackPanicked,
    IncompleteModel { variable: i32 },
    InvalidModel,
    InvalidFailedCore,
    ProofUnsupported,
    ProofPath(String),
    ProofReplay(String),
    ProofChecker(String),
}

impl fmt::Display for SatServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LiteralIsZero => formatter.write_str("SAT literals must be nonzero"),
            Self::LiteralOutOfRange(literal) => {
                write!(
                    formatter,
                    "SAT literal {literal} cannot be represented safely"
                )
            }
            Self::DecisionLimitOutOfRange(limit) => {
                write!(
                    formatter,
                    "SAT decision limit {limit} exceeds backend range"
                )
            }
            Self::DeadlineOutOfRange => {
                formatter.write_str("SAT deadline cannot be represented by the monotonic clock")
            }
            Self::Backend(message) => write!(formatter, "SAT backend failed: {message}"),
            Self::CallbackPanicked => formatter.write_str("SAT cancellation callback panicked"),
            Self::IncompleteModel { variable } => {
                write!(
                    formatter,
                    "SAT backend did not assign declared variable {variable}"
                )
            }
            Self::InvalidModel => {
                formatter.write_str("SAT backend returned a model that does not satisfy the query")
            }
            Self::InvalidFailedCore => formatter
                .write_str("SAT backend returned a failed-assumption core that is not UNSAT"),
            Self::ProofUnsupported => {
                formatter.write_str("SAT backend cannot produce a checkable UNSAT proof")
            }
            Self::ProofPath(message) => write!(formatter, "SAT proof path failed: {message}"),
            Self::ProofReplay(message) => write!(formatter, "SAT proof replay failed: {message}"),
            Self::ProofChecker(message) => write!(formatter, "SAT proof checker failed: {message}"),
        }
    }
}

impl std::error::Error for SatServiceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SatSolveOutcome {
    Sat {
        model: Vec<i32>,
    },
    Unsat {
        failed_assumptions: Vec<i32>,
        proof: Option<SatVerifiedProof>,
    },
    Unknown(SatUnknownReason),
    Error(SatServiceError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SatCapabilitySupport {
    Unsupported,
    Supported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SatServiceCapabilities {
    pub complete_models: SatCapabilitySupport,
    pub failed_assumptions: SatCapabilitySupport,
    pub decision_limits: SatCapabilitySupport,
    pub cancellation: SatCapabilitySupport,
    pub checked_proofs: SatCapabilitySupport,
}

/// Backend-neutral ownership boundary for an incremental SAT session.
///
/// A service exclusively owns its backend and permanent clause database.
/// `&mut self` makes clause mutation impossible while `solve` is active.
/// Assumptions apply only to one call. Implementations must never fabricate
/// models, failed cores, resource enforcement, or proof verification.
pub trait IncrementalSatService {
    fn backend_name(&self) -> &'static str;

    fn capabilities(&self) -> SatServiceCapabilities;

    fn add_clause(&mut self, clause: &[i32]) -> Result<(), SatServiceError>;

    fn solve(&mut self, assumptions: &[i32], options: &SatSolveOptions) -> SatSolveOutcome;

    fn reset(&mut self) -> Result<(), SatServiceError>;

    fn permanent_clause_count(&self) -> usize;
}

#[derive(Debug, Default)]
pub struct InternalSatService {
    clauses: Vec<Vec<i32>>,
    max_variable: i32,
}

impl InternalSatService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl IncrementalSatService for InternalSatService {
    fn backend_name(&self) -> &'static str {
        "internal-dpll"
    }

    fn capabilities(&self) -> SatServiceCapabilities {
        SatServiceCapabilities {
            complete_models: SatCapabilitySupport::Supported,
            failed_assumptions: SatCapabilitySupport::Supported,
            decision_limits: SatCapabilitySupport::Supported,
            cancellation: SatCapabilitySupport::Supported,
            checked_proofs: SatCapabilitySupport::Unsupported,
        }
    }

    fn add_clause(&mut self, clause: &[i32]) -> Result<(), SatServiceError> {
        let clause_max = validate_literals(clause)?;
        self.max_variable = self.max_variable.max(clause_max);
        self.clauses.push(clause.to_vec());
        Ok(())
    }

    fn solve(&mut self, assumptions: &[i32], options: &SatSolveOptions) -> SatSolveOutcome {
        let assumption_max = match validate_literals(assumptions) {
            Ok(max_variable) => max_variable,
            Err(error) => return SatSolveOutcome::Error(error),
        };
        let max_variable = self.max_variable.max(assumption_max);
        let Some(assignment_len) = usize::try_from(max_variable)
            .ok()
            .and_then(|value| value.checked_add(1))
        else {
            return SatSolveOutcome::Error(SatServiceError::LiteralOutOfRange(max_variable));
        };
        let Ok(deadline) = solve_deadline(options.deadline) else {
            return SatSolveOutcome::Error(SatServiceError::DeadlineOutOfRange);
        };

        let mut clauses = self.clauses.clone();
        clauses.extend(assumptions.iter().map(|literal| vec![*literal]));
        let mut assignment = vec![None; assignment_len];
        let mut trail = Vec::new();
        let mut control = InternalSearchControl {
            decisions_remaining: options.decision_limit,
            deadline,
            cancellation: &options.cancellation,
            external_stop: options.external_stop,
        };
        match internal_dpll(&clauses, &mut assignment, &mut trail, &mut control) {
            InternalSearchOutcome::Sat => {
                let model = complete_model(&assignment);
                if model_satisfies(&clauses, &model) {
                    SatSolveOutcome::Sat { model }
                } else {
                    SatSolveOutcome::Error(SatServiceError::InvalidModel)
                }
            }
            InternalSearchOutcome::Unsat => {
                if options.proof.is_some() {
                    SatSolveOutcome::Error(SatServiceError::ProofUnsupported)
                } else {
                    SatSolveOutcome::Unsat {
                        failed_assumptions: minimize_internal_failed_assumptions(
                            &self.clauses,
                            assumptions,
                            max_variable,
                            options.decision_limit,
                        ),
                        proof: None,
                    }
                }
            }
            InternalSearchOutcome::Unknown(reason) => SatSolveOutcome::Unknown(reason),
            InternalSearchOutcome::Error(error) => SatSolveOutcome::Error(error),
        }
    }

    fn reset(&mut self) -> Result<(), SatServiceError> {
        self.clauses.clear();
        self.max_variable = 0;
        Ok(())
    }

    fn permanent_clause_count(&self) -> usize {
        self.clauses.len()
    }
}

fn solve_deadline(duration: Option<Duration>) -> Result<Option<Instant>, SatServiceError> {
    if let Some(duration) = duration {
        Instant::now()
            .checked_add(duration)
            .map(Some)
            .ok_or(SatServiceError::DeadlineOutOfRange)
    } else {
        Ok(None)
    }
}

fn minimize_internal_failed_assumptions(
    permanent_clauses: &[Vec<i32>],
    assumptions: &[i32],
    max_variable: i32,
    decision_limit: Option<u64>,
) -> Vec<i32> {
    let mut core = assumptions.to_vec();
    let mut index = 0;
    while index < core.len() {
        let mut trial = core.clone();
        trial.remove(index);
        let mut clauses = permanent_clauses.to_vec();
        clauses.extend(trial.iter().map(|literal| vec![*literal]));
        if internal_formula_is_unsat(&clauses, max_variable, decision_limit) {
            core = trial;
        } else {
            index += 1;
        }
    }
    core
}

fn internal_formula_is_unsat(
    clauses: &[Vec<i32>],
    max_variable: i32,
    decision_limit: Option<u64>,
) -> bool {
    let Some(assignment_len) = usize::try_from(max_variable)
        .ok()
        .and_then(|value| value.checked_add(1))
    else {
        return false;
    };
    let cancellation = SatCancellationToken::new();
    let mut assignment = vec![None; assignment_len];
    let mut trail = Vec::new();
    let mut control = InternalSearchControl {
        decisions_remaining: decision_limit,
        deadline: None,
        cancellation: &cancellation,
        external_stop: None,
    };
    internal_dpll(clauses, &mut assignment, &mut trail, &mut control)
        == InternalSearchOutcome::Unsat
}

pub(crate) fn validate_literals(literals: &[i32]) -> Result<i32, SatServiceError> {
    let mut max_variable = 0;
    for &literal in literals {
        if literal == 0 {
            return Err(SatServiceError::LiteralIsZero);
        }
        let Some(variable) = literal.checked_abs() else {
            return Err(SatServiceError::LiteralOutOfRange(literal));
        };
        max_variable = max_variable.max(variable);
    }
    Ok(max_variable)
}

fn complete_model(assignment: &[Option<bool>]) -> Vec<i32> {
    assignment
        .iter()
        .enumerate()
        .skip(1)
        .map(|(variable, value)| {
            let variable = i32::try_from(variable).unwrap_or(i32::MAX);
            if value.unwrap_or(false) {
                variable
            } else {
                -variable
            }
        })
        .collect()
}

pub(crate) fn model_satisfies(clauses: &[Vec<i32>], model: &[i32]) -> bool {
    clauses.iter().all(|clause| {
        clause.iter().any(|literal| {
            let Some(variable) = literal
                .checked_abs()
                .and_then(|value| usize::try_from(value).ok())
            else {
                return false;
            };
            model
                .get(variable.saturating_sub(1))
                .is_some_and(|value| *value == *literal)
        })
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InternalSearchOutcome {
    Sat,
    Unsat,
    Unknown(SatUnknownReason),
    Error(SatServiceError),
}

struct InternalSearchControl<'a> {
    decisions_remaining: Option<u64>,
    deadline: Option<Instant>,
    cancellation: &'a SatCancellationToken,
    external_stop: Option<fn() -> bool>,
}

impl InternalSearchControl<'_> {
    fn poll(&self) -> Result<Option<SatUnknownReason>, SatServiceError> {
        if self.cancellation.is_cancelled() {
            return Ok(Some(SatUnknownReason::Cancelled));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Ok(Some(SatUnknownReason::Deadline));
        }
        let stopped = self.external_stop.map(|callback| {
            std::panic::catch_unwind(callback).map_err(|_| SatServiceError::CallbackPanicked)
        });
        match stopped {
            Some(Ok(true)) => Ok(Some(SatUnknownReason::ExternalStop)),
            Some(Ok(false)) | None => Ok(None),
            Some(Err(error)) => Err(error),
        }
    }

    fn take_decision(&mut self) -> bool {
        let Some(remaining) = &mut self.decisions_remaining else {
            return true;
        };
        if *remaining == 0 {
            false
        } else {
            *remaining -= 1;
            true
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InternalClauseStatus {
    Satisfied,
    Conflict,
    Unit(i32),
    Open,
}

fn internal_dpll(
    clauses: &[Vec<i32>],
    assignment: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    control: &mut InternalSearchControl<'_>,
) -> InternalSearchOutcome {
    match control.poll() {
        Ok(Some(reason)) => return InternalSearchOutcome::Unknown(reason),
        Ok(None) => {}
        Err(error) => return InternalSearchOutcome::Error(error),
    }

    loop {
        let mut changed = false;
        for clause in clauses {
            match internal_clause_status(clause, assignment) {
                InternalClauseStatus::Satisfied | InternalClauseStatus::Open => {}
                InternalClauseStatus::Conflict => return InternalSearchOutcome::Unsat,
                InternalClauseStatus::Unit(literal) => {
                    if !internal_assign_literal(assignment, trail, literal) {
                        return InternalSearchOutcome::Unsat;
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
        match control.poll() {
            Ok(Some(reason)) => return InternalSearchOutcome::Unknown(reason),
            Ok(None) => {}
            Err(error) => return InternalSearchOutcome::Error(error),
        }
    }

    if clauses
        .iter()
        .all(|clause| internal_clause_status(clause, assignment) == InternalClauseStatus::Satisfied)
    {
        return InternalSearchOutcome::Sat;
    }

    let Some(branch_literal) = internal_first_open_literal(clauses, assignment) else {
        return InternalSearchOutcome::Unsat;
    };
    if !control.take_decision() {
        return InternalSearchOutcome::Unknown(SatUnknownReason::DecisionLimit);
    }

    let preferred = branch_literal > 0;
    for value in [preferred, !preferred] {
        let checkpoint = trail.len();
        if !internal_assign_variable(assignment, trail, branch_literal.abs(), value) {
            continue;
        }
        let result = internal_dpll(clauses, assignment, trail, control);
        if result == InternalSearchOutcome::Sat {
            return result;
        }
        internal_undo_assignments(assignment, trail, checkpoint);
        if !matches!(result, InternalSearchOutcome::Unsat) {
            return result;
        }
    }
    InternalSearchOutcome::Unsat
}

fn internal_clause_status(clause: &[i32], assignment: &[Option<bool>]) -> InternalClauseStatus {
    let mut open_literal = None;
    for &literal in clause {
        let variable = usize::try_from(literal.abs()).unwrap_or(usize::MAX);
        match assignment.get(variable).copied().flatten() {
            Some(value) if value == (literal > 0) => return InternalClauseStatus::Satisfied,
            Some(_) => {}
            None if open_literal.is_none() => open_literal = Some(literal),
            None => return InternalClauseStatus::Open,
        }
    }
    open_literal.map_or(InternalClauseStatus::Conflict, InternalClauseStatus::Unit)
}

fn internal_first_open_literal(clauses: &[Vec<i32>], assignment: &[Option<bool>]) -> Option<i32> {
    clauses
        .iter()
        .filter(|clause| {
            internal_clause_status(clause, assignment) != InternalClauseStatus::Satisfied
        })
        .flat_map(|clause| clause.iter().copied())
        .find(|literal| {
            usize::try_from(literal.abs())
                .ok()
                .and_then(|variable| assignment.get(variable))
                .is_some_and(Option::is_none)
        })
}

fn internal_assign_literal(
    assignment: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    literal: i32,
) -> bool {
    internal_assign_variable(assignment, trail, literal.abs(), literal > 0)
}

fn internal_assign_variable(
    assignment: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    variable: i32,
    value: bool,
) -> bool {
    let index = usize::try_from(variable).unwrap_or(usize::MAX);
    let Some(slot) = assignment.get_mut(index) else {
        return false;
    };
    if let Some(existing) = *slot {
        existing == value
    } else {
        *slot = Some(value);
        trail.push(index);
        true
    }
}

fn internal_undo_assignments(
    assignment: &mut [Option<bool>],
    trail: &mut Vec<usize>,
    checkpoint: usize,
) {
    while trail.len() > checkpoint {
        let variable = trail.pop().expect("SAT assignment trail is nonempty");
        assignment[variable] = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IncrementalSatService, InternalSatService, SatCancellationToken, SatProofRequest,
        SatServiceError, SatSolveOptions, SatSolveOutcome, SatUnknownReason,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn internal_service_keeps_permanent_clauses_and_drops_assumptions() {
        let mut service = InternalSatService::new();
        service.add_clause(&[1, 2]).unwrap();
        service.add_clause(&[-1, 2]).unwrap();

        assert_eq!(
            service.solve(&[-2], &SatSolveOptions::default()),
            SatSolveOutcome::Unsat {
                failed_assumptions: vec![-2],
                proof: None,
            }
        );
        assert_eq!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Sat { model: vec![1, 2] }
        );
        assert_eq!(service.permanent_clause_count(), 2);
    }

    #[test]
    fn internal_service_returns_complete_valid_model() {
        let mut service = InternalSatService::new();
        service.add_clause(&[3]).unwrap();

        let SatSolveOutcome::Sat { model } = service.solve(&[], &SatSolveOptions::default()) else {
            panic!("expected SAT");
        };

        assert_eq!(model, vec![-1, -2, 3]);
    }

    #[test]
    fn internal_service_removes_redundant_failed_assumptions() {
        let mut service = InternalSatService::new();
        service.add_clause(&[1]).unwrap();

        assert_eq!(
            service.solve(&[1, 2, -2], &SatSolveOptions::default()),
            SatSolveOutcome::Unsat {
                failed_assumptions: vec![2, -2],
                proof: None,
            }
        );
    }

    #[test]
    fn internal_service_honors_decision_limit_and_cancellation() {
        let mut service = InternalSatService::new();
        service.add_clause(&[1, 2]).unwrap();
        service.add_clause(&[-1, 2]).unwrap();
        service.add_clause(&[1, -2]).unwrap();

        assert_eq!(
            service.solve(
                &[],
                &SatSolveOptions {
                    decision_limit: Some(0),
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Unknown(SatUnknownReason::DecisionLimit)
        );

        let token = SatCancellationToken::new();
        token.cancel();
        assert_eq!(
            service.solve(
                &[],
                &SatSolveOptions {
                    cancellation: token,
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Unknown(SatUnknownReason::Cancelled)
        );
    }

    #[test]
    fn internal_service_polls_external_stop_and_deadline() {
        static POLLS: AtomicUsize = AtomicUsize::new(0);
        fn stop() -> bool {
            POLLS.fetch_add(1, Ordering::Relaxed) >= 1
        }

        POLLS.store(0, Ordering::Relaxed);
        let mut service = InternalSatService::new();
        service.add_clause(&[1, 2]).unwrap();
        service.add_clause(&[-1, 2]).unwrap();
        service.add_clause(&[1, -2]).unwrap();
        assert_eq!(
            service.solve(
                &[],
                &SatSolveOptions {
                    external_stop: Some(stop),
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Unknown(SatUnknownReason::ExternalStop)
        );

        assert_eq!(
            service.solve(
                &[],
                &SatSolveOptions {
                    deadline: Some(Duration::ZERO),
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Unknown(SatUnknownReason::Deadline)
        );
    }

    #[test]
    fn reset_is_deterministic_and_clears_all_permanent_state() {
        let mut service = InternalSatService::new();
        service.add_clause(&[]).unwrap();
        assert!(matches!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Unsat { .. }
        ));

        service.reset().unwrap();

        assert_eq!(service.permanent_clause_count(), 0);
        assert_eq!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Sat { model: Vec::new() }
        );
    }

    #[test]
    fn invalid_literals_and_unsupported_proofs_are_explicit_errors() {
        let mut service = InternalSatService::new();
        assert_eq!(
            service.add_clause(&[0]),
            Err(SatServiceError::LiteralIsZero)
        );
        assert_eq!(
            service.add_clause(&[i32::MIN]),
            Err(SatServiceError::LiteralOutOfRange(i32::MIN))
        );
        service.add_clause(&[]).unwrap();
        assert_eq!(
            service.solve(
                &[],
                &SatSolveOptions {
                    proof: Some(SatProofRequest {
                        trace_path: PathBuf::from("unused.drat"),
                        checker_path: PathBuf::from("unused-checker"),
                    }),
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Error(SatServiceError::ProofUnsupported)
        );
    }
}
