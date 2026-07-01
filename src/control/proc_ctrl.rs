use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProverResult;
use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};
use crate::inout::tempfile::temp_file_remove;
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const EPCTRL_BUFSIZE: usize = 200;
pub const MAX_CORES: usize = 8;

pub const SZS_THEOREM_STR: &str = "% SZS status Theorem";
pub const SZS_CONTRAAX_STR: &str = "% SZS status ContradictoryAxioms";
pub const SZS_UNSAT_STR: &str = "% SZS status Unsatisfiable";
pub const SZS_SATSTR_STR: &str = "% SZS status Satisfiable";
pub const SZS_COUNTERSAT_STR: &str = "% SZS status CounterSatisfiable";
pub const SZS_GAVEUP_STR: &str = "% SZS status GaveUp";
pub const SZS_FAILURE_STR: &str = "% Failure:";

pub const E_OPTIONS_BASE: &str = " --print-pid -s -R  --memory-limit=2048 --proof-object ";
pub const E_OPTIONS: &str = "--satauto-schedule --assume-incompleteness";

#[derive(Debug)]
enum ProcessOutputMessage {
    Line(String),
    Eof,
    Error(String),
}

#[must_use]
pub const fn prover_result_table_entry(result: ProverResult) -> Option<&'static str> {
    match result {
        ProverResult::NoResult => None,
        ProverResult::Theorem => Some(SZS_THEOREM_STR),
        ProverResult::Unsatisfiable => Some(SZS_UNSAT_STR),
        ProverResult::Satisfiable => Some(SZS_SATSTR_STR),
        ProverResult::CounterSatisfiable => Some(SZS_COUNTERSAT_STR),
        ProverResult::Failure => Some(SZS_FAILURE_STR),
        ProverResult::GaveUp => Some(SZS_GAVEUP_STR),
    }
}

#[derive(Debug)]
pub struct EPCtrl {
    pid: Option<u32>,
    descriptor: Option<Descriptor>,
    child: Option<Child>,
    output_rx: Option<Receiver<ProcessOutputMessage>>,
    output_thread: Option<JoinHandle<()>>,
    output_eof: bool,
    input_file: Option<PathBuf>,
    name: String,
    start_time: i64,
    prob_time: i64,
    result: ProverResult,
    output: DynamicString,
}

impl EPCtrl {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            pid: None,
            descriptor: None,
            child: None,
            output_rx: None,
            output_thread: None,
            output_eof: false,
            input_file: None,
            name: name.into(),
            start_time: 0,
            prob_time: 0,
            result: ProverResult::NoResult,
            output: DynamicString::new(),
        }
    }

    #[must_use]
    pub fn with_descriptor(name: impl Into<String>, descriptor: Descriptor) -> Self {
        let mut control = Self::new(name);
        control.descriptor = Some(descriptor);
        control
    }

    pub fn create(
        prover: &str,
        name: &str,
        extra_options: &str,
        cpu_limit: i64,
        file: impl Into<PathBuf>,
    ) -> Result<Self, Diagnostic> {
        Self::create_generic(prover, name, E_OPTIONS, extra_options, cpu_limit, file)
    }

    pub fn create_generic(
        prover: &str,
        name: &str,
        options: &str,
        extra_options: &str,
        cpu_limit: i64,
        file: impl Into<PathBuf>,
    ) -> Result<Self, Diagnostic> {
        let input_file = file.into();
        let mut command = Command::new(prover);
        command.args(E_OPTIONS_BASE.split_whitespace());
        command.args(options.split_whitespace());
        command.args(extra_options.split_whitespace());
        command.arg(format!("--cpu-limit={cpu_limit}"));
        command.arg(&input_file);

        let proc_name = format!("{name} => {options}");
        let mut control = Self::spawn_command(command, proc_name, Some(input_file), cpu_limit)?;
        control.start_time = current_sec_time();
        Ok(control)
    }

    pub fn spawn_command(
        mut command: Command,
        name: impl Into<String>,
        input_file: Option<PathBuf>,
        prob_time: i64,
    ) -> Result<Self, Diagnostic> {
        command.stdout(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            proc_ctrl_system_error(format!("Cannot start eprover subprocess: {error}"))
        })?;
        let Some(stdout) = child.stdout.take() else {
            cleanup_child(&mut child);
            return Err(proc_ctrl_error("Cannot capture eprover subprocess output"));
        };
        let descriptor = match descriptor_from_child_stdout(&stdout) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                cleanup_child(&mut child);
                return Err(error);
            }
        };
        let mut stdout = BufReader::new(stdout);
        let mut pid_line = String::new();
        let read = stdout
            .read_line(&mut pid_line)
            .map_err(|error| proc_ctrl_error(format!("Cannot read eprover PID line: {error}")))?;
        if read == 0 {
            cleanup_child(&mut child);
            return Err(proc_ctrl_error("Cannot read eprover PID line"));
        }
        let pid = match parse_pid_line(&pid_line) {
            Ok(pid) => pid,
            Err(error) => {
                cleanup_child(&mut child);
                return Err(error);
            }
        };

        let mut control = Self::new(name);
        control.pid = Some(pid);
        control.descriptor = Some(descriptor);
        control.child = Some(child);
        let (output_rx, output_thread) = spawn_output_reader(stdout);
        control.output_rx = Some(output_rx);
        control.output_thread = Some(output_thread);
        control.input_file = input_file;
        control.start_time = current_sec_time();
        control.prob_time = prob_time;
        control.output.append_str(&pid_line);
        Ok(control)
    }

    #[must_use]
    pub const fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn set_pid(&mut self, pid: Option<u32>) {
        self.pid = pid;
    }

    #[must_use]
    pub const fn descriptor(&self) -> Option<Descriptor> {
        self.descriptor
    }

    pub fn set_descriptor(&mut self, descriptor: Option<Descriptor>) {
        self.descriptor = descriptor;
    }

    #[must_use]
    pub const fn has_child(&self) -> bool {
        self.child.is_some()
    }

    #[must_use]
    pub fn input_file(&self) -> Option<&Path> {
        self.input_file.as_deref()
    }

    pub fn set_input_file(&mut self, input_file: Option<PathBuf>) {
        self.input_file = input_file;
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn start_time(&self) -> i64 {
        self.start_time
    }

    pub fn set_start_time(&mut self, start_time: i64) {
        self.start_time = start_time;
    }

    #[must_use]
    pub const fn prob_time(&self) -> i64 {
        self.prob_time
    }

    pub fn set_prob_time(&mut self, prob_time: i64) {
        self.prob_time = prob_time;
    }

    #[must_use]
    pub const fn result(&self) -> ProverResult {
        self.result
    }

    #[must_use]
    pub fn output(&self) -> &DynamicString {
        &self.output
    }

    pub fn cleanup(&mut self, delete_file: bool) -> Result<(), Diagnostic> {
        if let Some(mut child) = self.child.take() {
            cleanup_child(&mut child);
        }
        if let Some(output_thread) = self.output_thread.take() {
            let _join_result = output_thread.join();
        }
        self.output_rx = None;
        self.output_eof = false;
        self.pid = None;
        self.descriptor = None;
        if delete_file {
            if let Some(input_file) = self.input_file.take() {
                let _removed_from_registry = temp_file_remove(&input_file)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn get_result_from_optional_line(&mut self, line: Option<&str>) -> bool {
        if let Some(line) = line {
            self.output.append_str(line);
            self.update_result_from_line(line);
            false
        } else {
            if self.result == ProverResult::NoResult {
                self.result = ProverResult::Failure;
            }
            true
        }
    }

    pub fn read_result_line(&mut self, buffer: &mut String) -> Result<bool, Diagnostic> {
        if self.output_eof {
            buffer.clear();
            return Ok(self.get_result_from_optional_line(None));
        }
        buffer.clear();
        let message = self
            .output_rx
            .as_ref()
            .ok_or_else(|| proc_ctrl_error("Cannot read from closed eprover subprocess pipe"))?
            .recv()
            .map_err(|_| proc_ctrl_error("Eprover subprocess output reader closed"))?;
        self.apply_output_message(message, buffer)
    }

    pub fn try_read_result_line(
        &mut self,
        buffer: &mut String,
    ) -> Result<Option<bool>, Diagnostic> {
        if self.output_eof {
            buffer.clear();
            return Ok(Some(self.get_result_from_optional_line(None)));
        }
        let message = match self
            .output_rx
            .as_ref()
            .ok_or_else(|| proc_ctrl_error("Cannot read from closed eprover subprocess pipe"))?
            .try_recv()
        {
            Ok(message) => message,
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) => {
                return Err(proc_ctrl_error("Eprover subprocess output reader closed"));
            }
        };
        self.apply_output_message(message, buffer).map(Some)
    }

    fn apply_output_message(
        &mut self,
        message: ProcessOutputMessage,
        buffer: &mut String,
    ) -> Result<bool, Diagnostic> {
        buffer.clear();
        match message {
            ProcessOutputMessage::Line(line) => {
                buffer.push_str(&line);
                Ok(self.get_result_from_optional_line(Some(buffer)))
            }
            ProcessOutputMessage::Eof => {
                self.output_eof = true;
                Ok(self.get_result_from_optional_line(None))
            }
            ProcessOutputMessage::Error(error) => Err(proc_ctrl_error(format!(
                "Cannot read eprover subprocess output: {error}"
            ))),
        }
    }

    fn update_result_from_line(&mut self, line: &str) {
        if line.contains(SZS_THEOREM_STR) || line.contains(SZS_CONTRAAX_STR) {
            self.result = ProverResult::Theorem;
        } else if line.contains(SZS_UNSAT_STR) {
            self.result = ProverResult::Unsatisfiable;
        } else if line.contains(SZS_SATSTR_STR) {
            self.result = ProverResult::Satisfiable;
        } else if line.contains(SZS_COUNTERSAT_STR) {
            self.result = ProverResult::CounterSatisfiable;
        }
    }
}

impl Drop for EPCtrl {
    fn drop(&mut self) {
        let _cleanup_result = self.cleanup(false);
    }
}

#[derive(Debug, Default)]
pub struct EPCtrlSet {
    procs: BTreeMap<Descriptor, EPCtrl>,
    buffer: String,
}

impl EPCtrlSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.procs.is_empty()
    }

    #[must_use]
    pub fn cardinality(&self) -> usize {
        self.procs.len()
    }

    #[must_use]
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    pub fn add_proc(&mut self, proc: EPCtrl) -> Result<Option<EPCtrl>, Diagnostic> {
        let descriptor = proc.descriptor().ok_or_else(|| {
            proc_ctrl_error(format!(
                "Cannot add process control {} without a descriptor",
                proc.name()
            ))
        })?;
        Ok(self.procs.insert(descriptor, proc))
    }

    #[must_use]
    pub fn find_proc(&self, descriptor: Descriptor) -> Option<&EPCtrl> {
        self.procs.get(&descriptor)
    }

    pub fn find_proc_mut(&mut self, descriptor: Descriptor) -> Option<&mut EPCtrl> {
        self.procs.get_mut(&descriptor)
    }

    pub fn delete_proc(
        &mut self,
        descriptor: Descriptor,
        delete_file: bool,
    ) -> Result<Option<EPCtrl>, Diagnostic> {
        let Some(mut proc) = self.procs.remove(&descriptor) else {
            return Ok(None);
        };
        proc.cleanup(delete_file)?;
        Ok(Some(proc))
    }

    pub fn clear(&mut self, delete_files: bool) -> Result<(), Diagnostic> {
        let descriptors = self.procs.keys().copied().collect::<Vec<_>>();
        for descriptor in descriptors {
            let _deleted = self.delete_proc(descriptor, delete_files)?;
        }
        Ok(())
    }

    pub fn init_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor {
        let mut max_descriptor = Descriptor::ZERO;
        for descriptor in self.procs.keys().copied() {
            interests.set_read(descriptor);
            max_descriptor = descriptor;
        }
        max_descriptor
    }

    pub fn get_result_from_ready<F>(
        &mut self,
        ready: &DescriptorInterestSet,
        delete_files: bool,
        output: &mut impl Write,
        mut read_result: F,
    ) -> Result<Option<Descriptor>, Diagnostic>
    where
        F: FnMut(&mut EPCtrl, &mut String) -> Result<bool, Diagnostic>,
    {
        let ready_descriptors = self
            .procs
            .keys()
            .copied()
            .filter(|descriptor| ready.contains_read(*descriptor))
            .collect::<Vec<_>>();

        let mut proof_descriptor = None;
        for descriptor in ready_descriptors {
            self.buffer.clear();
            let eof = {
                let Some(proc) = self.procs.get_mut(&descriptor) else {
                    continue;
                };
                read_result(proc, &mut self.buffer)?
            };
            if eof {
                if let Some(descriptor) =
                    self.handle_eof_result(descriptor, delete_files, output)?
                {
                    proof_descriptor = Some(descriptor);
                }
            }
        }
        Ok(proof_descriptor)
    }

    pub fn get_result_from_pipes(
        &mut self,
        ready: &DescriptorInterestSet,
        delete_files: bool,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        self.get_result_from_ready(ready, delete_files, output, EPCtrl::read_result_line)
    }

    pub fn get_result_from_available_pipes(
        &mut self,
        delete_files: bool,
        output: &mut impl Write,
    ) -> Result<(Option<Descriptor>, bool), Diagnostic> {
        let descriptors = self.procs.keys().copied().collect::<Vec<_>>();
        let mut proof_descriptor = None;
        let mut saw_output = false;

        for descriptor in descriptors {
            self.buffer.clear();
            let Some(eof) = ({
                let Some(proc) = self.procs.get_mut(&descriptor) else {
                    continue;
                };
                proc.try_read_result_line(&mut self.buffer)?
            }) else {
                continue;
            };
            saw_output = true;
            if eof {
                if let Some(descriptor) =
                    self.handle_eof_result(descriptor, delete_files, output)?
                {
                    proof_descriptor = Some(descriptor);
                }
            }
        }

        Ok((proof_descriptor, saw_output))
    }

    pub fn get_result_from_pipes_timeout(
        &mut self,
        timeout: Duration,
        delete_files: bool,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        let start = Instant::now();
        loop {
            let (proof_descriptor, saw_output) =
                self.get_result_from_available_pipes(delete_files, output)?;
            if proof_descriptor.is_some() || saw_output || self.is_empty() {
                return Ok(proof_descriptor);
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            let Some(remaining) = timeout.checked_sub(elapsed) else {
                return Ok(None);
            };
            thread::sleep(remaining.min(Duration::from_millis(10)));
        }
    }

    fn handle_eof_result(
        &mut self,
        descriptor: Descriptor,
        delete_files: bool,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        match self
            .procs
            .get(&descriptor)
            .map_or(ProverResult::NoResult, EPCtrl::result)
        {
            ProverResult::NoResult => Ok(None),
            ProverResult::Theorem | ProverResult::Unsatisfiable => Ok(Some(descriptor)),
            ProverResult::Satisfiable
            | ProverResult::CounterSatisfiable
            | ProverResult::Failure => {
                let name = self
                    .procs
                    .get(&descriptor)
                    .map(|proc| proc.name().to_owned())
                    .unwrap_or_default();
                writeln!(output, "% No proof found by {name}")
                    .map_err(|error| output_error(&error))?;
                let _deleted = self.delete_proc(descriptor, delete_files)?;
                Ok(None)
            }
            ProverResult::GaveUp => Err(proc_ctrl_error(
                "Process control reached impossible GaveUp result state",
            )),
        }
    }
}

impl SessionProcessSet for EPCtrlSet {
    fn init_read_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor {
        self.init_fd_set(interests)
    }
}

#[must_use]
pub fn e_ctrl_command(
    prover: &str,
    options: &str,
    extra_options: &str,
    cpu_limit: i64,
    file: &str,
) -> String {
    format!("{prover}{E_OPTIONS_BASE}{options} {extra_options} --cpu-limit={cpu_limit} {file}")
}

#[must_use]
pub fn e_ctrl_default_command(
    prover: &str,
    extra_options: &str,
    cpu_limit: i64,
    file: &str,
) -> String {
    e_ctrl_command(prover, E_OPTIONS, extra_options, cpu_limit, file)
}

fn proc_ctrl_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

fn output_error(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write process control output: {error}"),
    )
}

fn proc_ctrl_system_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYSTEM_ERROR, message)
}

fn current_sec_time() -> i64 {
    crate::basics::os_wrapper::get_usec_time() / 1_000_000
}

fn cleanup_child(child: &mut Child) {
    let _kill_result = child.kill();
    let _wait_result = child.wait();
}

fn spawn_output_reader(
    mut stdout: BufReader<ChildStdout>,
) -> (Receiver<ProcessOutputMessage>, JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || loop {
        let mut line = String::new();
        match stdout.read_line(&mut line) {
            Ok(0) => {
                let _send_result = sender.send(ProcessOutputMessage::Eof);
                break;
            }
            Ok(_) => {
                if sender.send(ProcessOutputMessage::Line(line)).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _send_result = sender.send(ProcessOutputMessage::Error(error.to_string()));
                break;
            }
        }
    });
    (receiver, handle)
}

fn parse_pid_line(line: &str) -> Result<u32, Diagnostic> {
    if !line.contains("% Pid: ") {
        return Err(proc_ctrl_error("Cannot get eprover PID"));
    }
    let rest = line.get(7..).unwrap_or_default();
    let digits = rest
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        Ok(0)
    } else {
        digits
            .parse::<u32>()
            .map_err(|error| proc_ctrl_error(format!("Cannot parse eprover PID: {error}")))
    }
}

#[cfg(unix)]
fn descriptor_from_child_stdout(stdout: &ChildStdout) -> Result<Descriptor, Diagnostic> {
    use std::os::fd::AsRawFd;

    let raw = stdout.as_raw_fd();
    u64::try_from(raw)
        .map(Descriptor::new)
        .map_err(|_| proc_ctrl_error(format!("Invalid eprover pipe descriptor: {raw}")))
}

#[cfg(windows)]
fn descriptor_from_child_stdout(stdout: &ChildStdout) -> Result<Descriptor, Diagnostic> {
    use std::os::windows::io::AsRawHandle;

    let raw = stdout.as_raw_handle() as usize;
    if raw == 0 {
        Err(proc_ctrl_error(
            "Invalid eprover pipe descriptor: null handle",
        ))
    } else {
        Ok(Descriptor::new(u64::try_from(raw).unwrap_or(u64::MAX)))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        e_ctrl_command, e_ctrl_default_command, prover_result_table_entry, EPCtrl, EPCtrlSet,
        EPCTRL_BUFSIZE, E_OPTIONS, E_OPTIONS_BASE, SZS_CONTRAAX_STR, SZS_COUNTERSAT_STR,
        SZS_FAILURE_STR, SZS_GAVEUP_STR, SZS_SATSTR_STR, SZS_THEOREM_STR, SZS_UNSAT_STR,
    };
    use crate::basics::simple_stuff::ProverResult;
    use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};
    use std::process::Command;
    use std::time::Duration;

    #[test]
    fn result_table_matches_c_surface() {
        assert_eq!(prover_result_table_entry(ProverResult::NoResult), None);
        assert_eq!(
            prover_result_table_entry(ProverResult::Theorem),
            Some(SZS_THEOREM_STR)
        );
        assert_eq!(
            prover_result_table_entry(ProverResult::Unsatisfiable),
            Some(SZS_UNSAT_STR)
        );
        assert_eq!(
            prover_result_table_entry(ProverResult::Satisfiable),
            Some(SZS_SATSTR_STR)
        );
        assert_eq!(
            prover_result_table_entry(ProverResult::CounterSatisfiable),
            Some(SZS_COUNTERSAT_STR)
        );
        assert_eq!(
            prover_result_table_entry(ProverResult::Failure),
            Some(SZS_FAILURE_STR)
        );
        assert_eq!(
            prover_result_table_entry(ProverResult::GaveUp),
            Some(SZS_GAVEUP_STR)
        );
    }

    #[test]
    fn allocation_defaults_match_initialized_c_fields() {
        let control = EPCtrl::new("worker");

        assert_eq!(control.pid(), None);
        assert_eq!(control.descriptor(), None);
        assert!(!control.has_child());
        assert_eq!(control.input_file(), None);
        assert_eq!(control.name(), "worker");
        assert_eq!(control.start_time(), 0);
        assert_eq!(control.prob_time(), 0);
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(control.output().is_empty());
    }

    #[test]
    fn result_lines_update_status_and_accumulate_output() {
        let mut control = EPCtrl::new("worker");

        assert!(!control.get_result_from_optional_line(Some("% SZS status Satisfiable\n")));
        assert_eq!(control.result(), ProverResult::Satisfiable);
        assert!(!control.get_result_from_optional_line(Some("% SZS status CounterSatisfiable\n")));
        assert_eq!(control.result(), ProverResult::CounterSatisfiable);
        assert!(!control.get_result_from_optional_line(Some("% SZS status Unsatisfiable\n")));
        assert_eq!(control.result(), ProverResult::Unsatisfiable);
        assert!(!control.get_result_from_optional_line(Some("% SZS status ContradictoryAxioms\n")));
        assert_eq!(control.result(), ProverResult::Theorem);
        assert!(control.output().view().contains(SZS_CONTRAAX_STR));
    }

    #[test]
    fn result_parser_leaves_failure_and_gaveup_lines_to_eof_fallback() {
        let mut control = EPCtrl::new("worker");

        assert!(!control.get_result_from_optional_line(Some("% Failure: bad\n")));
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(!control.get_result_from_optional_line(Some("% SZS status GaveUp\n")));
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(control.get_result_from_optional_line(None));
        assert_eq!(control.result(), ProverResult::Failure);
    }

    #[test]
    fn eof_preserves_successful_result() {
        let mut control = EPCtrl::new("worker");

        assert!(!control.get_result_from_optional_line(Some("% SZS status Theorem\n")));
        assert!(control.get_result_from_optional_line(None));

        assert_eq!(control.result(), ProverResult::Theorem);
    }

    #[test]
    fn command_builder_preserves_c_spacing() {
        assert_eq!(
            e_ctrl_command("eprover", " --auto", "--extra", 5, "problem.p"),
            format!("eprover{E_OPTIONS_BASE} --auto --extra --cpu-limit=5 problem.p")
        );
        assert_eq!(
            e_ctrl_default_command("eprover", "", 7, "problem.p"),
            format!("eprover{E_OPTIONS_BASE}{E_OPTIONS}  --cpu-limit=7 problem.p")
        );
    }

    #[test]
    fn process_set_indexes_by_descriptor_and_sets_read_interest() {
        let mut set = EPCtrlSet::new();
        set.add_proc(EPCtrl::with_descriptor("a", Descriptor::new(4)))
            .unwrap();
        set.add_proc(EPCtrl::with_descriptor("b", Descriptor::new(9)))
            .unwrap();
        let mut interests = DescriptorInterestSet::default();

        assert_eq!(set.cardinality(), 2);
        assert_eq!(set.init_fd_set(&mut interests), Descriptor::new(9));
        assert!(interests.contains_read(Descriptor::new(4)));
        assert!(interests.contains_read(Descriptor::new(9)));
        assert_eq!(
            SessionProcessSet::init_read_fd_set(&set, &mut DescriptorInterestSet::default()),
            Descriptor::new(9)
        );
    }

    #[test]
    fn process_set_get_result_returns_last_success_and_deletes_failures() {
        let mut set = EPCtrlSet::new();
        set.add_proc(EPCtrl::with_descriptor("success", Descriptor::new(2)))
            .unwrap();
        set.add_proc(EPCtrl::with_descriptor("failure", Descriptor::new(5)))
            .unwrap();
        set.add_proc(EPCtrl::with_descriptor("later_success", Descriptor::new(7)))
            .unwrap();
        let mut ready = DescriptorInterestSet::default();
        ready.set_read(Descriptor::new(2));
        ready.set_read(Descriptor::new(5));
        ready.set_read(Descriptor::new(7));
        let mut output = Vec::new();

        let result = set
            .get_result_from_ready(&ready, false, &mut output, |proc, _buffer| {
                match proc.name() {
                    "success" => {
                        let _eof =
                            proc.get_result_from_optional_line(Some("% SZS status Theorem\n"));
                    }
                    "later_success" => {
                        let _eof = proc
                            .get_result_from_optional_line(Some("% SZS status Unsatisfiable\n"));
                    }
                    _ => {}
                }
                Ok(proc.get_result_from_optional_line(None))
            })
            .unwrap();

        assert_eq!(result, Some(Descriptor::new(7)));
        assert_eq!(set.cardinality(), 2);
        assert!(set.find_proc(Descriptor::new(5)).is_none());
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% No proof found by failure\n"
        );
    }

    #[test]
    fn spawn_command_parses_pid_line_and_reads_status_lines() {
        let mut control = EPCtrl::spawn_command(
            pid_status_command("% SZS status Unsatisfiable"),
            "spawned",
            None,
            3,
        )
        .unwrap();
        let mut buffer = String::with_capacity(EPCTRL_BUFSIZE);

        assert_eq!(control.pid(), Some(123));
        assert!(control.descriptor().is_some());
        assert!(control.has_child());
        assert!(control.output().view().contains("% Pid: 123"));

        assert!(!control.read_result_line(&mut buffer).unwrap());
        assert_eq!(control.result(), ProverResult::Unsatisfiable);
        assert!(control.read_result_line(&mut buffer).unwrap());
        assert_eq!(control.result(), ProverResult::Unsatisfiable);
        control.cleanup(false).unwrap();
        assert!(!control.has_child());
    }

    #[test]
    fn spawn_command_rejects_missing_pid_line() {
        let error = EPCtrl::spawn_command(no_pid_command(), "bad", None, 3).unwrap_err();
        assert_eq!(
            error.code(),
            crate::basics::error::ErrorCode::INTERFACE_ERROR
        );
        assert_eq!(error.message(), "Cannot get eprover PID");
    }

    #[test]
    fn process_set_timeout_poll_reads_child_proof() {
        let proof =
            EPCtrl::spawn_command(pid_status_command("% SZS status Theorem"), "proof", None, 3)
                .unwrap();
        let proof_descriptor = proof.descriptor().unwrap();
        let mut set = EPCtrlSet::new();
        set.add_proc(proof).unwrap();
        let mut output = Vec::new();
        let mut result = None;

        for _ in 0..10 {
            result = set
                .get_result_from_pipes_timeout(Duration::from_millis(500), false, &mut output)
                .unwrap();
            if result.is_some() {
                break;
            }
        }

        assert_eq!(result, Some(proof_descriptor));
        assert!(set.find_proc(proof_descriptor).is_some());
        assert!(String::from_utf8(output).unwrap().is_empty());
    }

    #[cfg(windows)]
    fn pid_status_command(status: &str) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("echo % Pid: 123& echo {status}")]);
        command
    }

    #[cfg(unix)]
    fn pid_status_command(status: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", &format!("printf '%s\\n' '% Pid: 123' '{status}'")]);
        command
    }

    #[cfg(windows)]
    fn no_pid_command() -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", "echo no pid"]);
        command
    }

    #[cfg(unix)]
    fn no_pid_command() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "printf '%s\\n' 'no pid'"]);
        command
    }
}
