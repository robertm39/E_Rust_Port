use crate::clauses::satservice::{
    model_satisfies, validate_literals, IncrementalSatService, SatCancellationToken,
    SatCapabilitySupport, SatProofRequest, SatServiceCapabilities, SatServiceError,
    SatSolveOptions, SatSolveOutcome, SatUnknownReason, SatVerifiedProof,
};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr::NonNull;
use std::rc::Rc;
use std::time::Instant;

const CADICAL_SATISFIABLE: c_int = 10;
const CADICAL_UNSATISFIABLE: c_int = 20;
const DECISIONS: c_int = -1;

#[repr(C)]
struct UmlautCadicalOpaque {
    _private: [u8; 0],
}

type CadicalTerminate = unsafe extern "C" fn(*mut c_void) -> c_int;

unsafe extern "C" {
    fn umlaut_cadical_signature() -> *const c_char;
    fn umlaut_cadical_init() -> *mut UmlautCadicalOpaque;
    fn umlaut_cadical_release(solver: *mut UmlautCadicalOpaque);
    fn umlaut_cadical_last_error(solver: *const UmlautCadicalOpaque) -> *const c_char;
    fn umlaut_cadical_set_terminate(
        solver: *mut UmlautCadicalOpaque,
        state: *mut c_void,
        callback: Option<CadicalTerminate>,
    ) -> c_int;
    fn umlaut_cadical_add(solver: *mut UmlautCadicalOpaque, literal: c_int) -> c_int;
    fn umlaut_cadical_assume(solver: *mut UmlautCadicalOpaque, literal: c_int) -> c_int;
    fn umlaut_cadical_limit_decisions(solver: *mut UmlautCadicalOpaque, limit: c_int) -> c_int;
    fn umlaut_cadical_solve(solver: *mut UmlautCadicalOpaque) -> c_int;
    fn umlaut_cadical_val(solver: *mut UmlautCadicalOpaque, literal: c_int) -> c_int;
    fn umlaut_cadical_failed(solver: *mut UmlautCadicalOpaque, literal: c_int) -> c_int;
    fn umlaut_cadical_trace_proof(solver: *mut UmlautCadicalOpaque, path: *const c_char) -> c_int;
    fn umlaut_cadical_conclude(solver: *mut UmlautCadicalOpaque) -> c_int;
    fn umlaut_cadical_close_proof(solver: *mut UmlautCadicalOpaque) -> c_int;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminationCause {
    Cancelled,
    Deadline,
    ExternalStop,
}

struct TerminationState {
    cancellation: SatCancellationToken,
    deadline: Cell<Option<Instant>>,
    external_stop: Cell<Option<fn() -> bool>>,
    cause: Cell<Option<TerminationCause>>,
    callback_panicked: Cell<bool>,
}

impl Default for TerminationState {
    fn default() -> Self {
        Self {
            cancellation: SatCancellationToken::new(),
            deadline: Cell::new(None),
            external_stop: Cell::new(None),
            cause: Cell::new(None),
            callback_panicked: Cell::new(false),
        }
    }
}

/// `CaDiCaL` calls this function synchronously from the thread executing
/// `solve`. `state` is a stable `Box<TerminationState>` owned by
/// `CadicalCore`; the callback is disconnected by solver destruction before
/// that box is dropped. No Rust panic is allowed to cross the C++ ABI.
unsafe extern "C" fn termination_callback(state: *mut c_void) -> c_int {
    if state.is_null() {
        return 1;
    }
    // SAFETY: CadicalCore registers the pointer from its pinned-by-ownership
    // Box and keeps that allocation alive until after the solver is released.
    let state = unsafe { &*state.cast::<TerminationState>() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if state.cancellation.is_cancelled() {
            state.cause.set(Some(TerminationCause::Cancelled));
            return true;
        }
        if state
            .deadline
            .get()
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            state.cause.set(Some(TerminationCause::Deadline));
            return true;
        }
        if state.external_stop.get().is_some_and(|callback| callback()) {
            state.cause.set(Some(TerminationCause::ExternalStop));
            return true;
        }
        false
    }));
    match result {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => {
            state.callback_panicked.set(true);
            1
        }
    }
}

struct CadicalCore {
    solver: NonNull<UmlautCadicalOpaque>,
    termination: Box<TerminationState>,
    proof_open: bool,
    // CaDiCaL instances and callback state are intentionally thread-confined.
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl CadicalCore {
    fn new() -> Result<Self, SatServiceError> {
        let mut termination = Box::<TerminationState>::default();
        // SAFETY: the constructor takes no Rust pointers and returns either a
        // uniquely owned solver pointer or NULL. The C++ shim catches every
        // exception before it can cross the ABI.
        let solver = NonNull::new(unsafe { umlaut_cadical_init() }).ok_or_else(|| {
            SatServiceError::Backend("umlaut_cadical_init returned NULL".to_owned())
        })?;
        // SAFETY: solver is the unique live handle above. The state pointer
        // refers to a Box allocation whose address does not change when the
        // Box itself moves into CadicalCore. The callback type exactly matches
        // the shim header.
        let installed = unsafe {
            umlaut_cadical_set_terminate(
                solver.as_ptr(),
                std::ptr::from_mut(&mut *termination).cast(),
                Some(termination_callback),
            )
        };
        if installed == 0 {
            let error = last_backend_error(solver);
            // SAFETY: solver is still uniquely owned here and was returned by
            // umlaut_cadical_init.
            unsafe { umlaut_cadical_release(solver.as_ptr()) };
            return Err(error);
        }
        Ok(Self {
            solver,
            termination,
            proof_open: false,
            _not_send_or_sync: PhantomData,
        })
    }

    fn add_clause(&mut self, clause: &[i32]) -> Result<(), SatServiceError> {
        for &literal in clause {
            self.call_literal(umlaut_cadical_add, literal)?;
        }
        self.call_literal(umlaut_cadical_add, 0)
    }

    fn assume(&mut self, literal: i32) -> Result<(), SatServiceError> {
        self.call_literal(umlaut_cadical_assume, literal)
    }

    fn solve(
        &mut self,
        options: &SatSolveOptions,
    ) -> Result<(c_int, Option<TerminationCause>), SatServiceError> {
        let decision_limit = match options.decision_limit {
            Some(limit) => c_int::try_from(limit)
                .map_err(|_| SatServiceError::DecisionLimitOutOfRange(limit))?,
            None => DECISIONS,
        };
        // SAFETY: solver is live and exclusively borrowed; the shim converts
        // the named decision limit to CaDiCaL's one-call native limit.
        if unsafe { umlaut_cadical_limit_decisions(self.solver.as_ptr(), decision_limit) } == 0 {
            return Err(last_backend_error(self.solver));
        }
        self.termination.cancellation = options.cancellation.clone();
        self.termination.deadline.set(match options.deadline {
            Some(duration) => Some(
                Instant::now()
                    .checked_add(duration)
                    .ok_or(SatServiceError::DeadlineOutOfRange)?,
            ),
            None => None,
        });
        self.termination.external_stop.set(options.external_stop);
        self.termination.cause.set(None);
        self.termination.callback_panicked.set(false);

        // SAFETY: solver is live and no other Rust reference can mutate it
        // during this &mut self call. The registered callback state is live.
        let result = unsafe { umlaut_cadical_solve(self.solver.as_ptr()) };
        self.termination.deadline.set(None);
        self.termination.external_stop.set(None);
        if self.termination.callback_panicked.get() {
            return Err(SatServiceError::CallbackPanicked);
        }
        if result < 0 {
            return Err(last_backend_error(self.solver));
        }
        Ok((result, self.termination.cause.get()))
    }

    fn value(&mut self, variable: i32) -> Result<i32, SatServiceError> {
        // SAFETY: this is called only after SAT, while the exclusive solver
        // borrow preserves CaDiCaL's model-query state.
        let value = unsafe { umlaut_cadical_val(self.solver.as_ptr(), variable) };
        if value == 0 {
            Err(SatServiceError::IncompleteModel { variable })
        } else {
            Ok(value)
        }
    }

    fn failed(&mut self, assumption: i32) -> Result<bool, SatServiceError> {
        // SAFETY: this is called only after assumption-dependent UNSAT and
        // only with a literal from that solve's active assumption slice.
        match unsafe { umlaut_cadical_failed(self.solver.as_ptr(), assumption) } {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(last_backend_error(self.solver)),
        }
    }

    fn start_proof(&mut self, path: &Path) -> Result<(), SatServiceError> {
        let path = path_to_c_string(path)?;
        // SAFETY: path is NUL-terminated for the duration of the call, solver
        // is live and still configuring, and the shim opens/owns the file.
        if unsafe { umlaut_cadical_trace_proof(self.solver.as_ptr(), path.as_ptr()) } == 0 {
            return Err(last_backend_error(self.solver));
        }
        self.proof_open = true;
        Ok(())
    }

    fn conclude(&mut self) -> Result<(), SatServiceError> {
        // SAFETY: callers invoke this only after a completed proof-replay
        // solve, as required by the CaDiCaL public API.
        if unsafe { umlaut_cadical_conclude(self.solver.as_ptr()) } == 0 {
            Err(last_backend_error(self.solver))
        } else {
            Ok(())
        }
    }

    fn close_proof(&mut self) -> Result<(), SatServiceError> {
        if !self.proof_open {
            return Ok(());
        }
        // SAFETY: the proof stream is owned by the live shim instance. Closing
        // finalizes it before an independent process reads the path.
        if unsafe { umlaut_cadical_close_proof(self.solver.as_ptr()) } == 0 {
            return Err(last_backend_error(self.solver));
        }
        self.proof_open = false;
        Ok(())
    }

    fn call_literal(
        &mut self,
        function: unsafe extern "C" fn(*mut UmlautCadicalOpaque, c_int) -> c_int,
        literal: i32,
    ) -> Result<(), SatServiceError> {
        // SAFETY: function is one of the two exact literal-taking functions
        // declared by the tracked shim; solver is live and exclusively owned.
        if unsafe { function(self.solver.as_ptr(), literal) } == 0 {
            Err(last_backend_error(self.solver))
        } else {
            Ok(())
        }
    }
}

impl Drop for CadicalCore {
    fn drop(&mut self) {
        if self.proof_open {
            // SAFETY: best-effort finalization of the stream owned by this
            // still-live solver. Errors cannot be reported from Drop.
            let _ = unsafe { umlaut_cadical_close_proof(self.solver.as_ptr()) };
            self.proof_open = false;
        }
        // SAFETY: this is the unique pointer returned by init. Releasing it
        // disconnects the callback before TerminationState is dropped.
        unsafe { umlaut_cadical_release(self.solver.as_ptr()) };
    }
}

pub struct CadicalSatService {
    core: CadicalCore,
    clauses: Vec<Vec<i32>>,
    max_variable: i32,
}

impl CadicalSatService {
    /// Creates an empty `CaDiCaL` 3.0.1 incremental service.
    ///
    /// # Errors
    ///
    /// Returns an error if the statically linked solver cannot be allocated
    /// or its synchronous termination callback cannot be installed.
    pub fn new() -> Result<Self, SatServiceError> {
        Ok(Self {
            core: CadicalCore::new()?,
            clauses: Vec::new(),
            max_variable: 0,
        })
    }

    #[must_use]
    pub fn signature() -> Option<String> {
        // SAFETY: the shim returns a process-static NUL-terminated signature
        // or NULL and does not retain any Rust data.
        let signature = unsafe { umlaut_cadical_signature() };
        if signature.is_null() {
            None
        } else {
            // SAFETY: non-null data follows the shim's const char* contract
            // and is copied into an owned String immediately.
            Some(
                unsafe { CStr::from_ptr(signature) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }

    fn complete_model(
        &mut self,
        max_variable: i32,
        assumptions: &[i32],
    ) -> Result<Vec<i32>, SatServiceError> {
        let mut model = Vec::with_capacity(usize::try_from(max_variable).unwrap_or(0));
        for variable in 1..=max_variable {
            let value = self.core.value(variable)?;
            model.push(if value < 0 { -variable } else { variable });
        }
        let mut scoped_clauses = self.clauses.clone();
        scoped_clauses.extend(assumptions.iter().map(|literal| vec![*literal]));
        if model_satisfies(&scoped_clauses, &model) {
            Ok(model)
        } else {
            Err(SatServiceError::InvalidModel)
        }
    }

    fn failed_core(&mut self, assumptions: &[i32]) -> Result<Vec<i32>, SatServiceError> {
        assumptions
            .iter()
            .copied()
            .filter_map(|assumption| match self.core.failed(assumption) {
                Ok(true) => Some(Ok(assumption)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
    }

    fn validate_failed_core(&self, core: &[i32]) -> Result<(), SatServiceError> {
        let mut verifier = CadicalCore::new()?;
        for clause in &self.clauses {
            verifier.add_clause(clause)?;
        }
        for &assumption in core {
            verifier.assume(assumption)?;
        }
        let (result, _) = verifier.solve(&SatSolveOptions::default())?;
        if result == CADICAL_UNSATISFIABLE {
            Ok(())
        } else {
            Err(SatServiceError::InvalidFailedCore)
        }
    }

    fn checked_proof(
        &self,
        assumptions: &[i32],
        request: &SatProofRequest,
    ) -> Result<SatVerifiedProof, SatServiceError> {
        if request.trace_path.exists() {
            return Err(SatServiceError::ProofPath(format!(
                "{} already exists",
                request.trace_path.display()
            )));
        }
        let scope_path = proof_scope_path(&request.trace_path);
        write_proof_scope(
            &scope_path,
            self.max_variable.max(validate_literals(assumptions)?),
            &self.clauses,
            assumptions,
        )?;

        let mut replay = CadicalCore::new()?;
        replay.start_proof(&request.trace_path)?;
        for clause in &self.clauses {
            replay.add_clause(clause)?;
        }
        for &assumption in assumptions {
            replay.add_clause(&[assumption])?;
        }
        let (result, _) = replay.solve(&SatSolveOptions::default())?;
        if result != CADICAL_UNSATISFIABLE {
            return Err(SatServiceError::ProofReplay(format!(
                "expected UNSAT, received raw status {result}"
            )));
        }
        replay.conclude()?;
        replay.close_proof()?;

        let output = Command::new(&request.checker_path)
            .arg(&scope_path)
            .arg(&request.trace_path)
            .output()
            .map_err(|error| SatServiceError::ProofChecker(error.to_string()))?;
        if !output.status.success() {
            return Err(SatServiceError::ProofChecker(format!(
                "checker exited with {}; stdout: {}; stderr: {}",
                output.status,
                bounded_output(&output.stdout),
                bounded_output(&output.stderr)
            )));
        }
        Ok(SatVerifiedProof {
            format: "DRAT",
            scope_path,
            trace_path: request.trace_path.clone(),
            checker_path: request.checker_path.clone(),
        })
    }
}

impl IncrementalSatService for CadicalSatService {
    fn backend_name(&self) -> &'static str {
        "cadical-3.0.1-static"
    }

    fn capabilities(&self) -> SatServiceCapabilities {
        SatServiceCapabilities {
            complete_models: SatCapabilitySupport::Supported,
            failed_assumptions: SatCapabilitySupport::Supported,
            decision_limits: SatCapabilitySupport::Supported,
            cancellation: SatCapabilitySupport::Supported,
            checked_proofs: SatCapabilitySupport::Supported,
        }
    }

    fn add_clause(&mut self, clause: &[i32]) -> Result<(), SatServiceError> {
        let clause_max = validate_literals(clause)?;
        self.core.add_clause(clause)?;
        self.max_variable = self.max_variable.max(clause_max);
        self.clauses.push(clause.to_vec());
        Ok(())
    }

    fn solve(&mut self, assumptions: &[i32], options: &SatSolveOptions) -> SatSolveOutcome {
        let assumption_max = match validate_literals(assumptions) {
            Ok(max_variable) => max_variable,
            Err(error) => return SatSolveOutcome::Error(error),
        };
        for &assumption in assumptions {
            if let Err(error) = self.core.assume(assumption) {
                return SatSolveOutcome::Error(error);
            }
        }
        let (result, cause) = match self.core.solve(options) {
            Ok(result) => result,
            Err(error) => return SatSolveOutcome::Error(error),
        };
        match result {
            CADICAL_SATISFIABLE => {
                match self.complete_model(self.max_variable.max(assumption_max), assumptions) {
                    Ok(model) => SatSolveOutcome::Sat { model },
                    Err(error) => SatSolveOutcome::Error(error),
                }
            }
            CADICAL_UNSATISFIABLE => {
                let failed_assumptions = match self.failed_core(assumptions) {
                    Ok(core) => core,
                    Err(error) => return SatSolveOutcome::Error(error),
                };
                if let Err(error) = self.validate_failed_core(&failed_assumptions) {
                    return SatSolveOutcome::Error(error);
                }
                let proof = match &options.proof {
                    Some(request) => match self.checked_proof(assumptions, request) {
                        Ok(proof) => Some(proof),
                        Err(error) => return SatSolveOutcome::Error(error),
                    },
                    None => None,
                };
                SatSolveOutcome::Unsat {
                    failed_assumptions,
                    proof,
                }
            }
            0 => SatSolveOutcome::Unknown(match cause {
                Some(TerminationCause::Cancelled) => SatUnknownReason::Cancelled,
                Some(TerminationCause::Deadline) => SatUnknownReason::Deadline,
                Some(TerminationCause::ExternalStop) => SatUnknownReason::ExternalStop,
                None if options.decision_limit.is_some() => SatUnknownReason::DecisionLimit,
                None => SatUnknownReason::Backend,
            }),
            other => SatSolveOutcome::Error(SatServiceError::Backend(format!(
                "unexpected CaDiCaL solve status {other}"
            ))),
        }
    }

    fn reset(&mut self) -> Result<(), SatServiceError> {
        let replacement = CadicalCore::new()?;
        self.core = replacement;
        self.clauses.clear();
        self.max_variable = 0;
        Ok(())
    }

    fn permanent_clause_count(&self) -> usize {
        self.clauses.len()
    }
}

fn last_backend_error(solver: NonNull<UmlautCadicalOpaque>) -> SatServiceError {
    // SAFETY: solver is live, and the shim-owned diagnostic buffer remains
    // valid until the next call on this exclusively borrowed solver.
    let error = unsafe { umlaut_cadical_last_error(solver.as_ptr()) };
    let message = if error.is_null() {
        "CaDiCaL reported no diagnostic".to_owned()
    } else {
        // SAFETY: the non-null pointer is NUL-terminated shim storage and is
        // copied before any subsequent backend call.
        unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned()
    };
    SatServiceError::Backend(message)
}

fn path_to_c_string(path: &Path) -> Result<CString, SatServiceError> {
    let text = path.to_str().ok_or_else(|| {
        SatServiceError::ProofPath(format!("{} is not valid UTF-8", path.display()))
    })?;
    CString::new(text).map_err(|_| {
        SatServiceError::ProofPath(format!("{} contains an interior NUL", path.display()))
    })
}

fn proof_scope_path(trace_path: &Path) -> PathBuf {
    let mut path = trace_path.as_os_str().to_owned();
    path.push(".cnf");
    PathBuf::from(path)
}

fn write_proof_scope(
    path: &Path,
    max_variable: i32,
    clauses: &[Vec<i32>],
    assumptions: &[i32],
) -> Result<(), SatServiceError> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| SatServiceError::ProofPath(error.to_string()))?;
    let mut output = BufWriter::new(file);
    writeln!(
        output,
        "p cnf {} {}",
        max_variable,
        clauses.len().saturating_add(assumptions.len())
    )
    .map_err(|error| SatServiceError::ProofPath(error.to_string()))?;
    for clause in clauses {
        write_dimacs_clause(&mut output, clause)?;
    }
    for assumption in assumptions {
        write_dimacs_clause(&mut output, &[*assumption])?;
    }
    output
        .flush()
        .map_err(|error| SatServiceError::ProofPath(error.to_string()))
}

fn write_dimacs_clause(
    output: &mut BufWriter<File>,
    clause: &[i32],
) -> Result<(), SatServiceError> {
    for literal in clause {
        write!(output, "{literal} ")
            .map_err(|error| SatServiceError::ProofPath(error.to_string()))?;
    }
    writeln!(output, "0").map_err(|error| SatServiceError::ProofPath(error.to_string()))
}

fn bounded_output(bytes: &[u8]) -> String {
    const LIMIT: usize = 512;
    String::from_utf8_lossy(&bytes[..bytes.len().min(LIMIT)]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::CadicalSatService;
    #[cfg(unix)]
    use crate::clauses::satservice::SatServiceError;
    use crate::clauses::satservice::{
        IncrementalSatService, SatCancellationToken, SatProofRequest, SatSolveOptions,
        SatSolveOutcome, SatUnknownReason,
    };
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static UNIQUE_PATH: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn signature_pins_selected_cadical_version() {
        assert!(CadicalSatService::signature()
            .as_deref()
            .is_some_and(|signature| signature.contains("3.0.1")));
    }

    #[test]
    fn permanent_clauses_survive_and_assumptions_expire() {
        let mut service = CadicalSatService::new().unwrap();
        service.add_clause(&[1, 2]).unwrap();
        service.add_clause(&[-1, 2]).unwrap();

        assert!(matches!(
            service.solve(&[-2], &SatSolveOptions::default()),
            SatSolveOutcome::Unsat {
                failed_assumptions,
                proof: None
            } if failed_assumptions == vec![-2]
        ));
        assert!(matches!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Sat { model } if model.len() == 2 && model[1] == 2
        ));
        assert_eq!(service.permanent_clause_count(), 2);
    }

    #[test]
    fn complete_models_and_failed_cores_are_validated() {
        let mut service = CadicalSatService::new().unwrap();
        service.add_clause(&[3]).unwrap();
        assert_eq!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Sat {
                model: vec![-1, -2, 3],
            }
        );

        service.reset().unwrap();
        service.add_clause(&[1, 2]).unwrap();
        assert_eq!(
            service.solve(&[-1, -2], &SatSolveOptions::default()),
            SatSolveOutcome::Unsat {
                failed_assumptions: vec![-1, -2],
                proof: None,
            }
        );
    }

    #[test]
    fn cancellation_and_reset_are_deterministic() {
        let mut service = CadicalSatService::new().unwrap();
        add_pigeonhole(&mut service, 8, 7);
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

        service.reset().unwrap();
        assert_eq!(service.permanent_clause_count(), 0);
        assert_eq!(
            service.solve(&[], &SatSolveOptions::default()),
            SatSolveOutcome::Sat { model: Vec::new() }
        );
    }

    #[test]
    fn decision_limit_reports_unknown_without_fabricating_a_status() {
        let mut service = CadicalSatService::new().unwrap();
        add_pigeonhole(&mut service, 8, 7);
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
    }

    #[test]
    fn checked_proof_passes_configured_independent_checker() {
        let Some(checker) = env::var_os("UMLAUT_CADICAL_TEST_CHECKER").map(PathBuf::from) else {
            return;
        };
        let trace = unique_temp_path("checked", "drat");
        let scope = PathBuf::from(format!("{}.cnf", trace.display()));
        let mut service = CadicalSatService::new().unwrap();
        service.add_clause(&[1]).unwrap();
        service.add_clause(&[-1]).unwrap();

        let result = service.solve(
            &[],
            &SatSolveOptions {
                proof: Some(SatProofRequest {
                    trace_path: trace.clone(),
                    checker_path: checker.clone(),
                }),
                ..SatSolveOptions::default()
            },
        );

        assert!(matches!(
            result,
            SatSolveOutcome::Unsat {
                proof: Some(proof),
                ..
            } if proof.trace_path == trace && proof.scope_path == scope && proof.checker_path == checker
        ));
        let _ = fs::remove_file(trace);
        let _ = fs::remove_file(scope);
    }

    #[cfg(unix)]
    #[test]
    fn proof_checker_failure_returns_error_instead_of_unsat() {
        let checker = known_failing_executable();
        let trace = unique_temp_path("rejected", "drat");
        let scope = PathBuf::from(format!("{}.cnf", trace.display()));
        let mut service = CadicalSatService::new().unwrap();
        service.add_clause(&[1]).unwrap();
        service.add_clause(&[-1]).unwrap();

        assert!(matches!(
            service.solve(
                &[],
                &SatSolveOptions {
                    proof: Some(SatProofRequest {
                        trace_path: trace.clone(),
                        checker_path: checker,
                    }),
                    ..SatSolveOptions::default()
                }
            ),
            SatSolveOutcome::Error(SatServiceError::ProofChecker(_))
        ));
        let _ = fs::remove_file(trace);
        let _ = fs::remove_file(scope);
    }

    fn add_pigeonhole(service: &mut CadicalSatService, pigeons: i32, holes: i32) {
        let variable = |pigeon: i32, hole: i32| pigeon * holes + hole + 1;
        for pigeon in 0..pigeons {
            service
                .add_clause(
                    &(0..holes)
                        .map(|hole| variable(pigeon, hole))
                        .collect::<Vec<_>>(),
                )
                .unwrap();
        }
        for hole in 0..holes {
            for first in 0..pigeons {
                for second in (first + 1)..pigeons {
                    service
                        .add_clause(&[-variable(first, hole), -variable(second, hole)])
                        .unwrap();
                }
            }
        }
    }

    fn unique_temp_path(stem: &str, extension: &str) -> PathBuf {
        let serial = UNIQUE_PATH.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!(
            "umlaut-cadical-{stem}-{}-{serial}.{extension}",
            std::process::id()
        ))
    }

    #[cfg(unix)]
    fn known_failing_executable() -> PathBuf {
        PathBuf::from("/bin/false")
    }
}
