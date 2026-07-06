//! Initial port of `PCL2/pcl_proofcheck`.

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_HYPOTHESIS;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::{eqn_string, Eqn, EqnPrintOptions};
use crate::clauses::eqnlist::EqnList;
use crate::inout::scanner::IoFormat;
use crate::inout::tempfile::{temp_file_name, temp_file_remove};
use crate::pcl2::expressions::PclOpCode;
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStep, PclStepLogic};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::{Command, Stdio};

pub const E_EXEC_DEFAULT: &str = "eprover";
pub const OTTER_EXEC_DEFAULT: &str = "otter";
pub const SPASS_EXEC_DEFAULT: &str = "SPASS-0.55";
pub const FOF_PROOFCHECK_WARNING: &str = "Cannot currently handle full first-order format!";
const C_PROOFCHECK_FGETS_TEXT_LIMIT: usize = 179;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PclCheckType {
    Fail,
    Ok,
    ByAssumption,
    NotImplemented,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProverType {
    NoProver,
    EProver,
    Spass,
    Setheo,
    Otter,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PclCheckSummary {
    pub checked: i64,
    pub unchecked: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProverProblemFileUse {
    Argument,
    Stdin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProverInvocation {
    pub executable: String,
    pub args: Vec<String>,
    pub problem: String,
    pub problem_file_use: ProverProblemFileUse,
    pub suppress_stderr: bool,
    pub success_marker: String,
}

#[derive(Debug)]
pub struct ProofcheckWarningOutput<'a, W: IoWrite + ?Sized> {
    writer: &'a mut W,
    program_name: &'a str,
}

impl<'a, W: IoWrite + ?Sized> ProofcheckWarningOutput<'a, W> {
    #[must_use]
    pub fn new(writer: &'a mut W, program_name: &'a str) -> Self {
        Self {
            writer,
            program_name,
        }
    }
}

/// C `PCLCollectPreconds`.
///
/// # Errors
///
/// Returns diagnostics for dangling full-protocol references, mini identifiers,
/// or clause copy failures.
pub fn collect_preconditions(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    collect_preconditions_with_warning_callback(protocol, step_id, set, || Ok(()))
}

/// C `PCLCollectPreconds`, including the full-FOF warning side channel.
///
/// # Errors
///
/// Returns diagnostics for dangling full-protocol references, mini identifiers,
/// clause copy failures, or warning writes.
pub fn collect_preconditions_with_warnings(
    warning: &mut (impl IoWrite + ?Sized),
    program_name: &str,
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    collect_preconditions_with_warning_callback(protocol, step_id, set, || {
        write_fof_proofcheck_warning(warning, program_name)
    })
}

fn collect_preconditions_with_warning_callback(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
    mut warn_fof: impl FnMut() -> Result<(), Diagnostic>,
) -> Result<i64, Diagnostic> {
    let Some(step) = protocol.find_step(step_id) else {
        return Err(proofcheck_error("PCL proofcheck step not found"));
    };
    let parent_ids = protocol.collect_preconditions(step.just())?;
    let mut count = 0;

    for parent_id in parent_ids {
        let Some(parent) = protocol.find_step(&parent_id) else {
            continue;
        };
        let Some(clause) = step_clause(parent).cloned() else {
            if parent.is_fof() {
                warn_fof()?;
            }
            continue;
        };
        let copied = clause.copy_to_bank(protocol.term_bank_mut())?;
        set.insert(copied);
        count += 1;
    }
    Ok(count)
}

/// C `PCLNegSkolemizeClause`.
///
/// # Errors
///
/// Returns diagnostics from skolemization or literal allocation.
pub fn neg_skolemize_clause(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    neg_skolemize_clause_with_warning_callback(protocol, step_id, set, || Ok(()))
}

/// C `PCLNegSkolemizeClause`, including the full-FOF warning side channel.
///
/// # Errors
///
/// Returns diagnostics from skolemization, literal allocation, or warning
/// writes.
pub fn neg_skolemize_clause_with_warnings(
    warning: &mut (impl IoWrite + ?Sized),
    program_name: &str,
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    neg_skolemize_clause_with_warning_callback(protocol, step_id, set, || {
        write_fof_proofcheck_warning(warning, program_name)
    })
}

fn neg_skolemize_clause_with_warning_callback(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
    mut warn_fof: impl FnMut() -> Result<(), Diagnostic>,
) -> Result<i64, Diagnostic> {
    let Some(clause) = protocol.find_step(step_id).and_then(step_clause).cloned() else {
        if protocol.find_step(step_id).is_some_and(PclStep::is_fof) {
            warn_fof()?;
        }
        return Ok(0);
    };
    let skolemized = clause.skolemize(protocol.term_bank_mut())?;
    let mut count = 0;

    for literal in skolemized.literals().as_slice() {
        let flipped = Eqn::alloc(
            literal.left().clone(),
            literal.right().clone(),
            protocol.term_bank_mut(),
            !literal.is_positive(),
        )?;
        let mut new_clause = Clause::alloc(EqnList::from_vec(vec![flipped]));
        new_clause.set_tptp_type(CP_TYPE_HYPOTHESIS);
        set.insert(new_clause);
        count += 1;
    }
    Ok(count)
}

/// C `PCLGenerateCheck`.
///
/// Returns `Ok(None)` for assumption/initial steps with no clausal
/// preconditions.
///
/// # Errors
///
/// Returns diagnostics from precondition collection, clause copying, or
/// skolemization.
pub fn generate_check(
    protocol: &mut PclProtocol,
    step_id: &PclId,
) -> Result<Option<ClauseSet>, Diagnostic> {
    generate_check_with_warning_callback(protocol, step_id, || Ok(()))
}

/// C `PCLGenerateCheck`, including FOF warning side effects from its helpers.
///
/// # Errors
///
/// Returns diagnostics from precondition collection, clause copying,
/// skolemization, or warning writes.
pub fn generate_check_with_warnings(
    warning: &mut (impl IoWrite + ?Sized),
    program_name: &str,
    protocol: &mut PclProtocol,
    step_id: &PclId,
) -> Result<Option<ClauseSet>, Diagnostic> {
    generate_check_with_warning_callback(protocol, step_id, || {
        write_fof_proofcheck_warning(warning, program_name)
    })
}

fn generate_check_with_warning_callback(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    mut warn_fof: impl FnMut() -> Result<(), Diagnostic>,
) -> Result<Option<ClauseSet>, Diagnostic> {
    let mut set = ClauseSet::new();
    if collect_preconditions_with_warning_callback(protocol, step_id, &mut set, &mut warn_fof)? == 0
    {
        return Ok(None);
    }
    let _ = neg_skolemize_clause_with_warning_callback(protocol, step_id, &mut set, &mut warn_fof)?;
    Ok(Some(set))
}

/// C `pcl_verify_eprover` problem-file body.
#[must_use]
pub fn eprover_problem_string(problem: &ClauseSet, bank: &TermBank) -> String {
    problem.print_tptp_format_string(bank)
}

/// C `clause_set_print_otter`.
#[must_use]
pub fn otter_clause_set_string(problem: &ClauseSet, bank: &TermBank) -> String {
    let mut output = String::new();
    for clause in problem.iter() {
        output.push_str(&otter_clause_string(clause, bank));
        output.push('\n');
    }
    output
}

/// C `pcl_verify_otter` problem-file body.
#[must_use]
pub fn otter_problem_string(problem: &ClauseSet, bank: &TermBank, time_limit: i64) -> String {
    let mut output = format!(
        "set(prolog_style_variables).\n\
         clear(print_kept).\n\
         clear(print_new_demod).\n\
         clear(print_back_demod).\n\
         clear(print_back_sub).\n\
         set(auto).\n\
         set(input_sos_first).\n\
         assign(max_seconds, {time_limit}).\n\n\
         assign(max_mem, 100000).\n\n\
         list(usable).\n\n\
         equal(X,X).\n",
    );
    output.push_str(&otter_clause_set_string(problem, bank));
    output.push_str("end_of_list.\n");
    output
}

/// C `sig_print_dfg`.
#[must_use]
pub fn dfg_signature_string(problem: &ClauseSet, signature: &Signature) -> String {
    let symbol_distribution = symbol_distribution(problem, signature);
    let mut output = String::from("list_of_symbols.\nfunctions[(spass_hack,0)");
    append_dfg_symbol_list(&mut output, signature, &symbol_distribution, false);
    output.push_str("].\npredicates[(spass_pred_dummy,0)");
    append_dfg_symbol_list(&mut output, signature, &symbol_distribution, true);
    output.push_str("].\nend_of_list.\n");
    output
}

/// C `clause_set_print_dfg`.
#[must_use]
pub fn dfg_clause_set_string(problem: &ClauseSet, bank: &TermBank) -> String {
    let mut output = String::new();
    for clause in problem.iter() {
        output.push_str(&dfg_clause_string(clause, bank));
        output.push('\n');
    }
    output
}

/// C `pcl_verify_spass` problem-file body.
#[must_use]
pub fn spass_problem_string(problem: &ClauseSet, bank: &TermBank, time_limit: i64) -> String {
    let mut output = String::from("begin_problem(Unknown).\n");
    output.push_str(&dfg_signature_string(problem, bank.signature()));
    output.push_str("list_of_clauses(axioms,cnf).\n");
    output.push_str(&dfg_clause_set_string(problem, bank));
    let _ = write!(
        output,
        "end_of_list.\n\
         list_of_settings(SPASS).\n\
         set_flag(TimeLimit, {time_limit}).\n\
         end_of_list.\n\
         end_problem.\n"
    );
    output
}

#[must_use]
pub fn prover_invocation_for_problem(
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
    problem: &ClauseSet,
    bank: &TermBank,
) -> Option<ProverInvocation> {
    match prover {
        ProverType::EProver => Some(ProverInvocation {
            executable: executable.unwrap_or(E_EXEC_DEFAULT).to_owned(),
            args: vec![
                "--tptp-in".to_owned(),
                "--prefer-initial-clauses".to_owned(),
                "--ac-handling=None".to_owned(),
                format!("--cpu-limit={time_limit}"),
            ],
            problem: eprover_problem_string(problem, bank),
            problem_file_use: ProverProblemFileUse::Argument,
            suppress_stderr: false,
            success_marker: format!("{DEFAULT_COMCHAR_RAW} Proof found!"),
        }),
        ProverType::Otter => Some(ProverInvocation {
            executable: executable.unwrap_or(OTTER_EXEC_DEFAULT).to_owned(),
            args: Vec::new(),
            problem: otter_problem_string(problem, bank, time_limit),
            problem_file_use: ProverProblemFileUse::Stdin,
            suppress_stderr: true,
            success_marker: "-------- PROOF --------".to_owned(),
        }),
        ProverType::Spass => Some(ProverInvocation {
            executable: executable.unwrap_or(SPASS_EXEC_DEFAULT).to_owned(),
            args: Vec::new(),
            problem: spass_problem_string(problem, bank, time_limit),
            problem_file_use: ProverProblemFileUse::Argument,
            suppress_stderr: false,
            success_marker: "Proof found.".to_owned(),
        }),
        ProverType::NoProver | ProverType::Setheo => None,
    }
}

/// C `pcl_run_prover` over an explicit argument-vector invocation.
///
/// # Errors
///
/// Returns diagnostics for temporary-file creation/writing/removal, stdin-file
/// opening, or process spawning/output collection failures.
pub fn run_prover_invocation(invocation: &ProverInvocation) -> Result<bool, Diagnostic> {
    let problem_file = temp_file_name()?;
    let result = run_prover_invocation_with_file(invocation, &problem_file);
    let cleanup = temp_file_remove(&problem_file);

    match (result, cleanup) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(found), Ok(_)) => Ok(found),
    }
}

/// C `pcl_run_prover` output side effects over an explicit invocation.
///
/// # Errors
///
/// Returns diagnostics for output writes, temporary-file handling, or process
/// spawning/output collection failures.
pub fn run_prover_invocation_with_output(
    output: &mut (impl IoWrite + ?Sized),
    output_level: i64,
    invocation: &ProverInvocation,
) -> Result<bool, Diagnostic> {
    let problem_file = temp_file_name()?;
    let result =
        run_prover_invocation_with_file_and_output(output, output_level, invocation, &problem_file);
    let cleanup = temp_file_remove(&problem_file);

    match (result, cleanup) {
        (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        (Ok(found), Ok(_)) => Ok(found),
    }
}

/// Initial C `PCLStepCheck` port.
///
/// `NoProver` and `Setheo` remain unchecked because the C proof checker has no
/// implemented external verifier for those variants.
///
/// # Errors
///
/// Returns diagnostics from check-problem generation or prover invocation.
pub fn step_check(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckType, Diagnostic> {
    step_check_with_runner(
        protocol,
        step_id,
        prover,
        executable,
        time_limit,
        generate_check,
        run_prover_invocation,
    )
}

/// C `PCLStepCheck`, including FOF warning side effects from check generation.
///
/// # Errors
///
/// Returns diagnostics from check-problem generation, warning writes, or prover
/// invocation.
pub fn step_check_with_warnings(
    warning: &mut (impl IoWrite + ?Sized),
    program_name: &str,
    protocol: &mut PclProtocol,
    step_id: &PclId,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckType, Diagnostic> {
    step_check_with_runner(
        protocol,
        step_id,
        prover,
        executable,
        time_limit,
        |protocol, step_id| generate_check_with_warnings(warning, program_name, protocol, step_id),
        run_prover_invocation,
    )
}

fn step_check_with_runner(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
    mut generate: impl FnMut(&mut PclProtocol, &PclId) -> Result<Option<ClauseSet>, Diagnostic>,
    mut run_prover: impl FnMut(&ProverInvocation) -> Result<bool, Diagnostic>,
) -> Result<PclCheckType, Diagnostic> {
    let Some(step) = protocol.find_step(step_id) else {
        return Err(proofcheck_error("PCL proofcheck step not found"));
    };
    if step.just().op() == PclOpCode::SplitClause {
        return Ok(PclCheckType::NotImplemented);
    }

    let Some(problem) = generate(protocol, step_id)? else {
        return Ok(PclCheckType::ByAssumption);
    };
    let Some(invocation) = prover_invocation_for_problem(
        prover,
        executable,
        time_limit,
        &problem,
        protocol.term_bank(),
    ) else {
        return Ok(PclCheckType::NotImplemented);
    };

    if run_prover(&invocation)? {
        Ok(PclCheckType::Ok)
    } else {
        Ok(PclCheckType::Fail)
    }
}

/// Initial C `PCLProtCheck` port.
///
/// # Errors
///
/// Returns diagnostics from step-level check generation.
pub fn protocol_check(
    protocol: &mut PclProtocol,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckSummary, Diagnostic> {
    let mut summary = PclCheckSummary::default();
    for id in protocol.step_ids() {
        match step_check(protocol, &id, prover, executable, time_limit)? {
            PclCheckType::ByAssumption | PclCheckType::Ok => summary.checked += 1,
            PclCheckType::NotImplemented => summary.unchecked += 1,
            PclCheckType::Fail => {}
        }
    }
    Ok(summary)
}

/// C `PCLProtCheck` with explicit `GlobalOut`/`OutputLevel` rendering.
///
/// # Errors
///
/// Returns diagnostics from step rendering, check generation, prover
/// invocation, or output writes.
pub fn protocol_check_with_output(
    output: &mut (impl IoWrite + ?Sized),
    output_level: i64,
    protocol: &mut PclProtocol,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckSummary, Diagnostic> {
    let mut summary = PclCheckSummary::default();
    for id in protocol.step_ids() {
        if output_level != 0 {
            let Some(step) = protocol.find_step(&id).cloned() else {
                return Err(proofcheck_error("PCL proofcheck step not found"));
            };
            let rendered =
                step.print_extra_string(protocol.term_bank_mut(), ProblemType::FirstOrder, false)?;
            proofcheck_write_all(
                output,
                format!("{DEFAULT_COMCHAR_RAW} Checking {rendered}\n").as_bytes(),
            )?;
        }

        let check = step_check_with_runner(
            protocol,
            &id,
            prover,
            executable,
            time_limit,
            generate_check,
            |invocation| run_prover_invocation_with_output(output, output_level, invocation),
        )?;
        update_summary_and_write_check_result(output, output_level, &mut summary, check)?;
    }
    Ok(summary)
}

/// C `PCLProtCheck` with explicit `GlobalOut`/`OutputLevel` rendering and
/// warning output.
///
/// # Errors
///
/// Returns diagnostics from step rendering, check generation, warning writes,
/// prover invocation, or output writes.
pub fn protocol_check_with_output_and_warnings(
    output: &mut (impl IoWrite + ?Sized),
    warning: &mut ProofcheckWarningOutput<'_, impl IoWrite + ?Sized>,
    output_level: i64,
    protocol: &mut PclProtocol,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckSummary, Diagnostic> {
    let mut summary = PclCheckSummary::default();
    for id in protocol.step_ids() {
        if output_level != 0 {
            let Some(step) = protocol.find_step(&id).cloned() else {
                return Err(proofcheck_error("PCL proofcheck step not found"));
            };
            let rendered =
                step.print_extra_string(protocol.term_bank_mut(), ProblemType::FirstOrder, false)?;
            proofcheck_write_all(
                output,
                format!("{DEFAULT_COMCHAR_RAW} Checking {rendered}\n").as_bytes(),
            )?;
        }

        let check = step_check_with_runner(
            protocol,
            &id,
            prover,
            executable,
            time_limit,
            |protocol, step_id| {
                generate_check_with_warnings(
                    &mut *warning.writer,
                    warning.program_name,
                    protocol,
                    step_id,
                )
            },
            |invocation| run_prover_invocation_with_output(output, output_level, invocation),
        )?;
        update_summary_and_write_check_result(output, output_level, &mut summary, check)?;
    }
    Ok(summary)
}

fn step_clause(step: &PclStep) -> Option<&Clause> {
    match step.logic() {
        PclStepLogic::Clause(clause) => Some(clause),
        PclStepLogic::Shell | PclStepLogic::Formula(_) => None,
    }
}

fn run_prover_invocation_with_file(
    invocation: &ProverInvocation,
    problem_file: &Path,
) -> Result<bool, Diagnostic> {
    write_prover_problem_file(invocation, problem_file)?;
    let output = execute_prover_invocation(invocation, problem_file)?;

    Ok(prover_output_contains_success_marker(
        &output,
        &invocation.success_marker,
    ))
}

fn run_prover_invocation_with_file_and_output(
    output: &mut (impl IoWrite + ?Sized),
    output_level: i64,
    invocation: &ProverInvocation,
    problem_file: &Path,
) -> Result<bool, Diagnostic> {
    write_prover_problem_file(invocation, problem_file)?;
    if output_level > 1 {
        let display = prover_display_command(invocation, problem_file);
        proofcheck_write_all(
            output,
            format!("{DEFAULT_COMCHAR_RAW} Running {display}\n").as_bytes(),
        )?;
    }

    let prover_output = execute_prover_invocation(invocation, problem_file)?;
    if output_level >= 3 {
        write_prover_output_trace(output, &prover_output)?;
    }

    let found = prover_output_contains_success_marker(&prover_output, &invocation.success_marker);
    if !found {
        proofcheck_write_all(
            output,
            format!("{DEFAULT_COMCHAR_RAW} ------------Problem begin--------------\n").as_bytes(),
        )?;
        proofcheck_write_all(output, invocation.problem.as_bytes())?;
        proofcheck_write_all(
            output,
            format!("{DEFAULT_COMCHAR_RAW} ------------Problem end----------------\n").as_bytes(),
        )?;
    }
    Ok(found)
}

fn write_prover_problem_file(
    invocation: &ProverInvocation,
    problem_file: &Path,
) -> Result<(), Diagnostic> {
    fs::write(problem_file, invocation.problem.as_bytes()).map_err(|error| {
        proofcheck_file_error(format!(
            "Could not write proofcheck problem file {}: {error}",
            problem_file.display()
        ))
    })
}

fn execute_prover_invocation(
    invocation: &ProverInvocation,
    problem_file: &Path,
) -> Result<Vec<u8>, Diagnostic> {
    let mut command = Command::new(&invocation.executable);
    command.args(&invocation.args);
    match invocation.problem_file_use {
        ProverProblemFileUse::Argument => {
            command.arg(problem_file);
        }
        ProverProblemFileUse::Stdin => {
            let stdin = File::open(problem_file).map_err(|error| {
                proofcheck_file_error(format!(
                    "Could not open proofcheck problem file {}: {error}",
                    problem_file.display()
                ))
            })?;
            command.stdin(Stdio::from(stdin));
        }
    }
    if invocation.suppress_stderr {
        command.stderr(Stdio::null());
    }

    let output = command.output().map_err(|error| {
        proofcheck_system_error(format!(
            "Cannot run proofcheck prover {}: {error}",
            invocation.executable
        ))
    })?;

    Ok(output.stdout)
}

fn prover_output_contains_success_marker(output: &[u8], success_marker: &str) -> bool {
    let success_marker = success_marker.as_bytes();
    if success_marker.is_empty() {
        return true;
    }

    c_proofcheck_fgets_chunks(output).any(|chunk| {
        let line = c_string_chunk(chunk);
        line.windows(success_marker.len())
            .any(|window| window == success_marker)
    })
}

fn prover_display_command(invocation: &ProverInvocation, problem_file: &Path) -> String {
    let mut output = invocation.executable.clone();
    for arg in &invocation.args {
        output.push(' ');
        output.push_str(arg);
    }
    match invocation.problem_file_use {
        ProverProblemFileUse::Argument => {
            output.push(' ');
            output.push_str(&problem_file.display().to_string());
        }
        ProverProblemFileUse::Stdin => {
            output.push_str(" < ");
            output.push_str(&problem_file.display().to_string());
        }
    }
    if invocation.suppress_stderr {
        output.push_str(" 2> /dev/null");
    }
    output
}

fn write_prover_output_trace(
    output: &mut (impl IoWrite + ?Sized),
    prover_output: &[u8],
) -> Result<(), Diagnostic> {
    for chunk in c_proofcheck_fgets_chunks(prover_output) {
        proofcheck_write_all(output, format!("{DEFAULT_COMCHAR_RAW}> ").as_bytes())?;
        proofcheck_write_all(output, c_string_chunk(chunk))?;
    }
    Ok(())
}

fn c_proofcheck_fgets_chunks(output: &[u8]) -> impl Iterator<Item = &[u8]> {
    let mut start = 0;
    std::iter::from_fn(move || {
        if start >= output.len() {
            return None;
        }

        let remaining = &output[start..];
        let chunk_len = remaining
            .iter()
            .take(C_PROOFCHECK_FGETS_TEXT_LIMIT)
            .position(|byte| *byte == b'\n')
            .map_or(
                remaining.len().min(C_PROOFCHECK_FGETS_TEXT_LIMIT),
                |index| index + 1,
            );
        let end = start + chunk_len;
        let chunk = &output[start..end];
        start = end;
        Some(chunk)
    })
}

fn c_string_chunk(chunk: &[u8]) -> &[u8] {
    let end = chunk
        .iter()
        .position(|byte| *byte == b'\0')
        .unwrap_or(chunk.len());
    &chunk[..end]
}

fn otter_clause_string(clause: &Clause, bank: &TermBank) -> String {
    if clause.is_empty() {
        return "$F.".to_owned();
    }

    let mut output = String::new();
    let mut literals = clause.literals().as_slice().iter();
    if let Some(first) = literals.next() {
        output.push_str(&otter_eqn_string(first, bank));
        for literal in literals {
            output.push_str("|\n");
            output.push_str(&otter_eqn_string(literal, bank));
        }
        output.push_str(".\n");
    }
    output
}

fn otter_eqn_string(literal: &Eqn, bank: &TermBank) -> String {
    if literal.is_equ_lit(bank) {
        if literal.is_positive() {
            return eqn_string(bank, literal, false, true, EqnPrintOptions::default());
        }
        return format!(
            "-{}",
            eqn_string(bank, literal, true, true, EqnPrintOptions::default())
        );
    }

    if literal.left() == bank.true_term() {
        debug_assert_eq!(literal.right(), bank.true_term());
        if literal.is_positive() {
            return "$T".to_owned();
        }
        return "$F".to_owned();
    }

    let mut output = String::new();
    output.push(if literal.is_negative() { '-' } else { ' ' });
    output.push_str(&bank.term_string(literal.left(), true));
    output
}

fn dfg_clause_string(clause: &Clause, bank: &TermBank) -> String {
    let mut output = String::from("clause(");
    let mut variables = BTreeMap::new();
    let variable_count = clause.collect_variables(&mut variables);
    if variable_count != 0 {
        let mut variables = variables.into_values().collect::<Vec<_>>();
        variables.sort_by_key(|variable| Reverse(variable.f_code()));
        output.push_str("forall([");
        for (index, variable) in variables.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push_str(&bank.term_string(variable, true));
        }
        output.push_str("],");
    }

    output.push_str("or(");
    let mut literals = clause.literals().as_slice().iter();
    if let Some(first) = literals.next() {
        output.push_str(&dfg_eqn_string(first, bank));
        for literal in literals {
            output.push(',');
            output.push_str(&dfg_eqn_string(literal, bank));
        }
    } else {
        output.push_str("not(equal(spass_hack,spass_hack))");
    }
    output.push(')');
    output.push(if variable_count != 0 { ')' } else { ' ' });
    let _ = write!(output, ", c{} ).", clause.ident());
    output
}

fn dfg_eqn_string(literal: &Eqn, bank: &TermBank) -> String {
    let mut output = String::new();
    if literal.is_negative() {
        output.push_str("not(");
    }
    if literal.left() == bank.true_term() {
        debug_assert_eq!(literal.right(), bank.true_term());
        output.push_str("equal(spass_hack,spass_hack)");
    } else {
        output.push_str(&eqn_string(
            bank,
            literal,
            literal.is_negative(),
            true,
            EqnPrintOptions {
                output_format: IoFormat::Lop,
                use_infix: false,
                ..EqnPrintOptions::default()
            },
        ));
    }
    if literal.is_negative() {
        output.push(')');
    }
    output
}

fn symbol_distribution(problem: &ClauseSet, signature: &Signature) -> Vec<i64> {
    let len = usize::try_from(signature.f_count() + 1).expect("signature size fits usize");
    let mut distribution = vec![0; len];
    for clause in problem.iter() {
        clause.add_symbol_distribution(&mut distribution);
    }
    distribution
}

fn append_dfg_symbol_list(
    output: &mut String,
    signature: &Signature,
    symbol_distribution: &[i64],
    predicates: bool,
) {
    for f_code in (signature.internal_symbols() + 1)..=signature.f_count() {
        if symbol_is_used(symbol_distribution, f_code)
            && signature.is_predicate(f_code) == predicates
        {
            let name = signature
                .find_name(f_code)
                .expect("valid f-code has a printable name");
            let arity = signature
                .find_arity(f_code)
                .expect("valid f-code has an arity");
            let _ = write!(output, ",({name},{arity})");
        }
    }
}

fn symbol_is_used(symbol_distribution: &[i64], f_code: FunCode) -> bool {
    usize::try_from(f_code)
        .ok()
        .and_then(|index| symbol_distribution.get(index))
        .is_some_and(|count| *count != 0)
}

fn proofcheck_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

fn proofcheck_file_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn proofcheck_system_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYSTEM_ERROR, message)
}

fn proofcheck_write_error(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Error writing output: {error}"),
    )
}

fn proofcheck_write_all(
    output: &mut (impl IoWrite + ?Sized),
    bytes: &[u8],
) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| proofcheck_write_error(&error))
}

fn write_fof_proofcheck_warning(
    warning: &mut (impl IoWrite + ?Sized),
    program_name: &str,
) -> Result<(), Diagnostic> {
    let diagnostic = Diagnostic::new(ErrorCode::OTHER_ERROR, FOF_PROOFCHECK_WARNING);
    proofcheck_write_all(warning, diagnostic.render_warning(program_name).as_bytes())
}

fn update_summary_and_write_check_result(
    output: &mut (impl IoWrite + ?Sized),
    output_level: i64,
    summary: &mut PclCheckSummary,
    check: PclCheckType,
) -> Result<(), Diagnostic> {
    match check {
        PclCheckType::ByAssumption => {
            if output_level >= 1 {
                proofcheck_write_all(
                    output,
                    format!("{DEFAULT_COMCHAR_RAW} Checked (by assumption)\n\n").as_bytes(),
                )?;
            }
            summary.checked += 1;
        }
        PclCheckType::Ok => {
            if output_level >= 1 {
                proofcheck_write_all(
                    output,
                    format!("{DEFAULT_COMCHAR_RAW} Checked (by prover)\n\n").as_bytes(),
                )?;
            }
            summary.checked += 1;
        }
        PclCheckType::Fail => {
            if output_level >= 1 {
                proofcheck_write_all(
                    output,
                    format!("{DEFAULT_COMCHAR_RAW} FAILED\n\n").as_bytes(),
                )?;
            }
        }
        PclCheckType::NotImplemented => {
            if output_level >= 1 {
                proofcheck_write_all(
                    output,
                    format!("{DEFAULT_COMCHAR_RAW} Check not implemented, assuming true!\n\n")
                        .as_bytes(),
                )?;
            }
            summary.unchecked += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        collect_preconditions, collect_preconditions_with_warnings, dfg_clause_set_string,
        dfg_signature_string, eprover_problem_string, generate_check, generate_check_with_warnings,
        neg_skolemize_clause, otter_clause_set_string, otter_problem_string, protocol_check,
        protocol_check_with_output, protocol_check_with_output_and_warnings,
        prover_invocation_for_problem, prover_output_contains_success_marker,
        run_prover_invocation, run_prover_invocation_with_output, spass_problem_string, step_check,
        step_check_with_runner, write_prover_output_trace, PclCheckType, ProofcheckWarningOutput,
        ProverInvocation, ProverProblemFileUse, ProverType, FOF_PROOFCHECK_WARNING,
    };
    use crate::basics::defines::DEFAULT_COMCHAR_RAW;
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::inout::tempfile::temp_file_test_lock;
    use crate::pcl2::idents::PclId;
    use crate::pcl2::protocol::PclProtocol;
    use crate::pcl2::steps::PclStepParseOptions;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};

    struct TmpDirGuard {
        previous: Option<OsString>,
    }

    impl Drop for TmpDirGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("TMPDIR", value),
                None => std::env::remove_var("TMPDIR"),
            }
        }
    }

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_protocol(source: &str) -> PclProtocol {
        let mut protocol = PclProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        protocol
            .parse(
                &mut scanner,
                PclStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                    ..PclStepParseOptions::default()
                },
            )
            .unwrap();
        protocol
    }

    fn target_dir() -> PathBuf {
        std::env::current_dir().unwrap().join("target")
    }

    fn set_tmpdir(path: &Path) -> TmpDirGuard {
        let previous = std::env::var_os("TMPDIR");
        std::env::set_var("TMPDIR", path);
        TmpDirGuard { previous }
    }

    #[cfg(windows)]
    fn stdout_copy_invocation(problem_file_use: ProverProblemFileUse) -> ProverInvocation {
        let args = match problem_file_use {
            ProverProblemFileUse::Argument => vec!["/C".to_owned(), "type".to_owned()],
            ProverProblemFileUse::Stdin => vec!["/C".to_owned(), "more".to_owned()],
        };
        ProverInvocation {
            executable: "cmd".to_owned(),
            args,
            problem: "payload\nPROOF-SUCCESS\n".to_owned(),
            problem_file_use,
            suppress_stderr: true,
            success_marker: "PROOF-SUCCESS".to_owned(),
        }
    }

    #[cfg(not(windows))]
    fn stdout_copy_invocation(problem_file_use: ProverProblemFileUse) -> ProverInvocation {
        ProverInvocation {
            executable: "cat".to_owned(),
            args: Vec::new(),
            problem: "payload\nPROOF-SUCCESS\n".to_owned(),
            problem_file_use,
            suppress_stderr: true,
            success_marker: "PROOF-SUCCESS".to_owned(),
        }
    }

    #[test]
    fn collect_preconditions_copies_unique_clausal_parent_clauses() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : q(a) : initial\n\
             3 : : [++r(a)] : pm(1,2)\n\
             4 : : [++s(a)] : pm(1,3)",
        );
        let mut set = ClauseSet::new();

        let count = collect_preconditions(&mut protocol, &parse_id("4"), &mut set).unwrap();

        assert_eq!(count, 2);
        assert_eq!(set.members(), 2);
        assert!(set.iter().all(|clause| !clause.is_empty()));
    }

    #[test]
    fn collect_preconditions_with_warnings_reports_fof_parent_steps() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : q(a) : initial\n\
             3 : : [++r(a)] : pm(1,2)",
        );
        let mut set = ClauseSet::new();
        let mut warning = Vec::new();

        let count = collect_preconditions_with_warnings(
            &mut warning,
            "eprover",
            &mut protocol,
            &parse_id("3"),
            &mut set,
        )
        .unwrap();

        assert_eq!(count, 1);
        assert_eq!(set.members(), 1);
        assert_eq!(
            String::from_utf8(warning).unwrap(),
            format!("eprover: Warning: {FOF_PROOFCHECK_WARNING}\n")
        );
    }

    #[test]
    fn neg_skolemize_clause_adds_one_flipped_hypothesis_unit_per_literal() {
        let mut protocol = parse_protocol("1 : : [++p(X),--q(a)] : initial\n2 : : [++r(a)] : 1");
        let mut set = ClauseSet::new();

        let count = neg_skolemize_clause(&mut protocol, &parse_id("1"), &mut set).unwrap();

        assert_eq!(count, 2);
        assert_eq!(set.members(), 2);
        let clauses = set.iter().collect::<Vec<_>>();
        assert!(clauses.iter().all(|clause| clause.is_hypothesis()));
        assert_eq!(
            clauses
                .iter()
                .map(|clause| clause.literal_number())
                .collect::<Vec<_>>(),
            [1, 1]
        );
        assert_eq!(
            clauses
                .iter()
                .filter(|clause| clause.literals().as_slice()[0].is_positive())
                .count(),
            1
        );
    }

    #[test]
    fn generate_check_with_warnings_reports_fof_target_steps() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : q(a) : 1");
        let mut warning = Vec::new();

        let problem =
            generate_check_with_warnings(&mut warning, "eprover", &mut protocol, &parse_id("2"))
                .unwrap()
                .unwrap();

        assert_eq!(problem.members(), 1);
        assert_eq!(
            String::from_utf8(warning).unwrap(),
            format!("eprover: Warning: {FOF_PROOFCHECK_WARNING}\n")
        );
    }

    #[test]
    fn generate_check_combines_copied_preconditions_and_negated_goal_units() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : [++q(a),--r(a)] : 1");

        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        assert_eq!(problem.members(), 3);
        assert_eq!(
            problem
                .iter()
                .map(Clause::literal_number)
                .collect::<Vec<_>>(),
            [1, 1, 1]
        );
    }

    #[test]
    fn generate_check_returns_none_for_assumption_without_clausal_preconditions() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial");

        assert!(generate_check(&mut protocol, &parse_id("1"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn eprover_problem_string_delegates_to_tptp_clause_set_rendering() {
        let mut protocol = parse_protocol("1 : : [++p(a),--q(a)] : initial\n2 : : [++r(a)] : 1");

        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        assert_eq!(
            eprover_problem_string(&problem, protocol.term_bank()),
            problem.print_tptp_format_string(protocol.term_bank())
        );
    }

    #[test]
    fn otter_problem_string_matches_c_header_and_clause_layout() {
        let mut protocol = parse_protocol("1 : : [++p(a),--q(a)] : initial\n2 : : [++r(a)] : 1");
        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        let rendered = otter_problem_string(&problem, protocol.term_bank(), 7);

        assert!(rendered.starts_with("set(prolog_style_variables).\nclear(print_kept).\n"));
        assert!(rendered.contains("assign(max_seconds, 7).\n\nassign(max_mem, 100000).\n\n"));
        assert!(rendered.contains("list(usable).\n\nequal(X,X).\n"));
        assert!(rendered.contains(" p(a)|\n-q(a).\n\n"));
        assert!(rendered.contains("-r(a).\n\n"));
        assert!(rendered.ends_with("end_of_list.\n"));
    }

    #[test]
    fn spass_problem_string_matches_c_dfg_wrapper_and_symbol_lists() {
        let mut protocol = parse_protocol("1 : : [++p(X),--q(X)] : initial\n2 : : [++r(a)] : 1");
        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        let signature = dfg_signature_string(&problem, protocol.term_bank().signature());
        let clauses = dfg_clause_set_string(&problem, protocol.term_bank());
        let rendered = spass_problem_string(&problem, protocol.term_bank(), 11);

        assert!(signature.starts_with("list_of_symbols.\nfunctions[(spass_hack,0)"));
        assert!(signature.contains(",(a,0)"));
        assert!(signature.contains("predicates[(spass_pred_dummy,0)"));
        assert!(signature.contains(",(p,1)"));
        assert!(signature.contains(",(q,1)"));
        assert!(signature.contains(",(r,1)"));
        assert!(clauses.contains("forall([X1],or(p(X1),not(q(X1))))"));
        assert!(clauses.contains("or(not(r(a)))"));
        assert!(rendered.starts_with("begin_problem(Unknown).\nlist_of_symbols.\n"));
        assert!(rendered.contains("list_of_clauses(axioms,cnf).\n"));
        assert!(rendered.contains("set_flag(TimeLimit, 11).\n"));
        assert!(rendered.ends_with("end_problem.\n"));
    }

    #[test]
    fn otter_and_dfg_render_c_truth_literal_hacks() {
        let mut protocol = PclProtocol::new().unwrap();
        let true_term = protocol.term_bank().true_term().clone();
        let positive = Eqn::alloc(
            true_term.clone(),
            true_term.clone(),
            protocol.term_bank_mut(),
            true,
        )
        .unwrap();
        let negative = Eqn::alloc(
            true_term.clone(),
            true_term,
            protocol.term_bank_mut(),
            false,
        )
        .unwrap();
        let set =
            ClauseSet::from_clauses([Clause::alloc(EqnList::from_vec(vec![positive, negative]))]);

        assert_eq!(
            otter_clause_set_string(&set, protocol.term_bank()),
            "$T|\n$F.\n\n"
        );
        assert!(dfg_clause_set_string(&set, protocol.term_bank())
            .contains("or(equal(spass_hack,spass_hack),not(equal(spass_hack,spass_hack)))"));
    }

    #[test]
    fn prover_invocation_for_problem_matches_c_command_shapes() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1");
        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        let eprover = prover_invocation_for_problem(
            ProverType::EProver,
            Some("custom-e"),
            17,
            &problem,
            protocol.term_bank(),
        )
        .unwrap();
        assert_eq!(eprover.executable, "custom-e");
        assert_eq!(
            eprover.args,
            [
                "--tptp-in",
                "--prefer-initial-clauses",
                "--ac-handling=None",
                "--cpu-limit=17"
            ]
        );
        assert_eq!(eprover.problem_file_use, ProverProblemFileUse::Argument);
        assert!(!eprover.suppress_stderr);
        assert_eq!(
            eprover.success_marker,
            format!("{DEFAULT_COMCHAR_RAW} Proof found!")
        );
        assert!(eprover.problem.contains("input_clause("));

        let otter = prover_invocation_for_problem(
            ProverType::Otter,
            None,
            19,
            &problem,
            protocol.term_bank(),
        )
        .unwrap();
        assert_eq!(otter.executable, super::OTTER_EXEC_DEFAULT);
        assert!(otter.args.is_empty());
        assert_eq!(otter.problem_file_use, ProverProblemFileUse::Stdin);
        assert!(otter.suppress_stderr);
        assert_eq!(otter.success_marker, "-------- PROOF --------");
        assert!(otter.problem.contains("assign(max_seconds, 19)."));

        let spass = prover_invocation_for_problem(
            ProverType::Spass,
            None,
            23,
            &problem,
            protocol.term_bank(),
        )
        .unwrap();
        assert_eq!(spass.executable, super::SPASS_EXEC_DEFAULT);
        assert!(spass.args.is_empty());
        assert_eq!(spass.problem_file_use, ProverProblemFileUse::Argument);
        assert!(!spass.suppress_stderr);
        assert_eq!(spass.success_marker, "Proof found.");
        assert!(spass.problem.contains("set_flag(TimeLimit, 23)."));

        assert!(prover_invocation_for_problem(
            ProverType::NoProver,
            None,
            1,
            &problem,
            protocol.term_bank()
        )
        .is_none());
        assert!(prover_invocation_for_problem(
            ProverType::Setheo,
            None,
            1,
            &problem,
            protocol.term_bank()
        )
        .is_none());
    }

    #[test]
    fn run_prover_invocation_writes_problem_and_scans_stdout() {
        let _guard = temp_file_test_lock();
        fs::create_dir_all(target_dir()).unwrap();
        let _tmpdir = set_tmpdir(&target_dir());

        assert!(
            run_prover_invocation(&stdout_copy_invocation(ProverProblemFileUse::Argument)).unwrap()
        );
        assert!(
            run_prover_invocation(&stdout_copy_invocation(ProverProblemFileUse::Stdin)).unwrap()
        );
    }

    #[test]
    fn run_prover_invocation_with_output_traces_and_dumps_failed_problem() {
        let _guard = temp_file_test_lock();
        fs::create_dir_all(target_dir()).unwrap();
        let _tmpdir = set_tmpdir(&target_dir());
        let mut invocation = stdout_copy_invocation(ProverProblemFileUse::Argument);
        invocation.success_marker = "MISSING-SUCCESS".to_owned();
        let mut output = Vec::new();

        assert!(!run_prover_invocation_with_output(&mut output, 3, &invocation).unwrap());

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("% Running "));
        assert!(output.contains("%> payload\n"));
        assert!(output.contains("%> PROOF-SUCCESS\n"));
        assert!(output.contains("% ------------Problem begin--------------\n"));
        assert!(output.contains("payload\nPROOF-SUCCESS\n"));
        assert!(output.contains("% ------------Problem end----------------\n"));
    }

    #[test]
    fn prover_success_marker_scans_c_fgets_chunks() {
        let mut split_marker = vec![b'a'; 178];
        split_marker.extend_from_slice(b"PROOF-SUCCESS\n");
        assert!(!prover_output_contains_success_marker(
            &split_marker,
            "PROOF-SUCCESS"
        ));

        let mut contained_marker = vec![b'a'; 166];
        contained_marker.extend_from_slice(b"PROOF-SUCCESS\n");
        assert!(prover_output_contains_success_marker(
            &contained_marker,
            "PROOF-SUCCESS"
        ));
    }

    #[test]
    fn prover_success_marker_and_trace_use_c_string_view() {
        assert!(!prover_output_contains_success_marker(
            b"prefix\0PROOF-SUCCESS\n",
            "PROOF-SUCCESS"
        ));

        let mut output = Vec::new();
        write_prover_output_trace(&mut output, b"prefix\0hidden\n").unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "%> prefix");
    }

    #[test]
    fn prover_output_trace_does_not_add_missing_final_newline() {
        let mut output = Vec::new();

        write_prover_output_trace(&mut output, b"unterminated").unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "%> unterminated");
    }

    #[test]
    fn prover_output_trace_splits_like_c_fgets_buffer() {
        let mut input = vec![b'a'; 180];
        input.push(b'\n');
        let mut output = Vec::new();

        write_prover_output_trace(&mut output, &input).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert_eq!(output, format!("%> {}%> a\n", "a".repeat(179)));
    }

    #[test]
    fn step_check_uses_external_runner_result_for_supported_provers() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1");
        let mut seen_invocation = None;

        let check = step_check_with_runner(
            &mut protocol,
            &parse_id("2"),
            ProverType::EProver,
            Some("fake-e"),
            29,
            generate_check,
            |invocation| {
                seen_invocation = Some(invocation.clone());
                Ok(true)
            },
        )
        .unwrap();

        assert_eq!(check, PclCheckType::Ok);
        let invocation = seen_invocation.unwrap();
        assert_eq!(invocation.executable, "fake-e");
        assert!(invocation.args.contains(&"--cpu-limit=29".to_owned()));

        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1");
        assert_eq!(
            step_check_with_runner(
                &mut protocol,
                &parse_id("2"),
                ProverType::EProver,
                None,
                29,
                generate_check,
                |_| Ok(false),
            )
            .unwrap(),
            PclCheckType::Fail
        );
    }

    #[test]
    fn protocol_check_with_output_reports_progress_and_results() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1\n3 : : [++r(a)] : split(2)",
        );
        let mut output = Vec::new();

        let summary = protocol_check_with_output(
            &mut output,
            1,
            &mut protocol,
            ProverType::NoProver,
            None,
            10,
        )
        .unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.unchecked, 2);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("% Checking       1 :  : [++p(a)] : initial\n"));
        assert!(output.contains("% Checked (by assumption)\n\n"));
        assert!(output.contains("% Checking       2 :  : [++q(a)] : 1\n"));
        assert!(output.contains("% Check not implemented, assuming true!\n\n"));
        assert!(output.contains("% Checking       3 :  : [++r(a)] : split(2)\n"));

        let mut protocol = parse_protocol("1 : : [++p(a)] : initial");
        let mut silent = Vec::new();
        let summary = protocol_check_with_output(
            &mut silent,
            0,
            &mut protocol,
            ProverType::NoProver,
            None,
            10,
        )
        .unwrap();
        assert_eq!(summary.checked, 1);
        assert!(silent.is_empty());
    }

    #[test]
    fn protocol_check_with_output_and_warnings_reports_fof_generation_warnings() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : q(a) : 1");
        let mut output = Vec::new();
        let mut warning = Vec::new();

        let summary = protocol_check_with_output_and_warnings(
            &mut output,
            &mut ProofcheckWarningOutput::new(&mut warning, "eprover"),
            1,
            &mut protocol,
            ProverType::NoProver,
            None,
            10,
        )
        .unwrap();

        assert_eq!(summary.checked, 1);
        assert_eq!(summary.unchecked, 1);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("% Checking       1 :  : [++p(a)] : initial\n"));
        assert!(output.contains("% Checking       2 :  : q(a) : 1\n"));
        assert!(output.contains("% Check not implemented, assuming true!\n\n"));
        assert_eq!(
            String::from_utf8(warning).unwrap(),
            format!("eprover: Warning: {FOF_PROOFCHECK_WARNING}\n")
        );
    }

    #[test]
    fn step_and_protocol_check_report_assumptions_and_unimplemented_external_checks() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1\n3 : : [++r(a)] : split(2)",
        );

        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("1"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::ByAssumption
        );
        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("2"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::NotImplemented
        );
        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("3"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::NotImplemented
        );

        let summary = protocol_check(&mut protocol, ProverType::NoProver, None, 10).unwrap();
        assert_eq!(summary.checked, 1);
        assert_eq!(summary.unchecked, 2);
    }
}
