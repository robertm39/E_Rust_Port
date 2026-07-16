use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{
    record_waited_child_resource_usage, record_waited_child_resource_usage_with_report,
    ResourceUsage,
};
use crate::basics::simple_stuff::ProverResult;
use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};
use crate::control::proc_ctrl::{
    SZS_CONTRAAX_STR, SZS_COUNTERSAT_STR, SZS_SATSTR_STR, SZS_THEOREM_STR, SZS_UNSAT_STR,
};
use crate::inout::signals::terminate_process;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const EGPCTRL_BUFSIZE: usize = 1024;
pub const EGPCTRL_SET_WAIT_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug)]
enum GenericProcessOutputMessage {
    Chunk(Vec<u8>),
    Eof,
    Error(String),
}

#[derive(Debug)]
pub struct EGPCtrl {
    name: Option<String>,
    pid: Option<u32>,
    descriptor: Option<Descriptor>,
    exit_status: i32,
    cpu_limit: u64,
    cores: usize,
    result: ProverResult,
    output: DynamicString,
    child: Option<Child>,
    output_rx: Option<Receiver<GenericProcessOutputMessage>>,
    output_thread: Option<JoinHandle<()>>,
    output_eof: bool,
}

impl EGPCtrl {
    #[must_use]
    pub fn new(cores: usize) -> Self {
        Self {
            name: None,
            pid: None,
            descriptor: None,
            exit_status: 0,
            cpu_limit: 0,
            cores,
            result: ProverResult::NoResult,
            output: DynamicString::new(),
            child: None,
            output_rx: None,
            output_thread: None,
            output_eof: false,
        }
    }

    #[must_use]
    pub fn with_descriptor(name: impl Into<String>, descriptor: Descriptor, cores: usize) -> Self {
        let mut control = Self::new(cores);
        control.name = Some(name.into());
        control.descriptor = Some(descriptor);
        control
    }

    pub fn spawn_command(
        command: Command,
        name: impl Into<String>,
        cores: usize,
        cpu_limit: u64,
    ) -> Result<Self, Diagnostic> {
        Self::spawn_command_reporting(command, name, cores, cpu_limit, &mut std::io::sink())
    }

    pub fn spawn_command_reporting<W: Write + ?Sized>(
        mut command: Command,
        name: impl Into<String>,
        cores: usize,
        cpu_limit: u64,
        startup_output: &mut W,
    ) -> Result<Self, Diagnostic> {
        let name = name.into();
        writeln!(
            startup_output,
            "% Starting {name} with {cpu_limit}s ({cores}) cores"
        )
        .map_err(|error| output_error(&error))?;
        command.stdout(Stdio::piped());
        let mut child = command.spawn().map_err(|error| {
            gproc_ctrl_system_error(format!("Cannot start generic subprocess: {error}"))
        })?;
        let Some(stdout) = child.stdout.take() else {
            cleanup_child(&mut child);
            return Err(gproc_ctrl_error("Cannot capture generic subprocess output"));
        };
        let descriptor = match descriptor_from_child_stdout(&stdout) {
            Ok(descriptor) => descriptor,
            Err(error) => {
                cleanup_child(&mut child);
                return Err(error);
            }
        };
        let (output_rx, output_thread) = spawn_output_reader(stdout);
        let mut control = Self::new(cores);
        control.name = Some(name);
        control.pid = Some(child.id());
        control.descriptor = Some(descriptor);
        control.cpu_limit = cpu_limit;
        control.child = Some(child);
        control.output_rx = Some(output_rx);
        control.output_thread = Some(output_thread);
        Ok(control)
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: Option<String>) {
        self.name = name;
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
    pub const fn exit_status(&self) -> i32 {
        self.exit_status
    }

    #[must_use]
    pub const fn cpu_limit(&self) -> u64 {
        self.cpu_limit
    }

    pub fn set_cpu_limit(&mut self, cpu_limit: u64) {
        self.cpu_limit = cpu_limit;
    }

    #[must_use]
    pub const fn cores(&self) -> usize {
        self.cores
    }

    #[must_use]
    pub const fn result(&self) -> ProverResult {
        self.result
    }

    #[must_use]
    pub fn output(&self) -> &DynamicString {
        &self.output
    }

    #[must_use]
    pub const fn has_child(&self) -> bool {
        self.child.is_some()
    }

    pub fn cleanup(&mut self) -> Result<(), Diagnostic> {
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
        Ok(())
    }

    pub fn get_result_from_optional_chunk<W: Write + ?Sized>(
        &mut self,
        chunk: Option<&[u8]>,
        completion_output: &mut W,
    ) -> Result<bool, Diagnostic> {
        if let Some(chunk) = chunk {
            self.output.append_bytes_with_str_growth(chunk);
            return Ok(false);
        }

        self.update_result_from_output();
        self.exit_status = self.wait_for_child_exit();
        let pid = self.pid.unwrap_or(0);
        let name = self.name.as_deref().unwrap_or("");
        writeln!(
            completion_output,
            "% {name} with pid {pid} completed with status {}",
            self.exit_status
        )
        .map_err(|error| output_error(&error))?;
        self.pid = None;
        Ok(true)
    }

    pub fn read_result_chunk<W: Write + ?Sized>(
        &mut self,
        buffer: &mut Vec<u8>,
        completion_output: &mut W,
    ) -> Result<bool, Diagnostic> {
        if self.output_eof {
            buffer.clear();
            return self.get_result_from_optional_chunk(None, completion_output);
        }
        buffer.clear();
        let message = self
            .output_rx
            .as_ref()
            .ok_or_else(|| gproc_ctrl_error("Cannot read from closed generic subprocess pipe"))?
            .recv()
            .map_err(|_| gproc_ctrl_error("Generic subprocess output reader closed"))?;
        self.apply_output_message(message, buffer, completion_output)
    }

    pub fn try_read_result_chunk<W: Write + ?Sized>(
        &mut self,
        buffer: &mut Vec<u8>,
        completion_output: &mut W,
    ) -> Result<Option<bool>, Diagnostic> {
        if self.output_eof {
            buffer.clear();
            return self
                .get_result_from_optional_chunk(None, completion_output)
                .map(Some);
        }
        let message = match self
            .output_rx
            .as_ref()
            .ok_or_else(|| gproc_ctrl_error("Cannot read from closed generic subprocess pipe"))?
            .try_recv()
        {
            Ok(message) => message,
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Disconnected) => {
                return Err(gproc_ctrl_error("Generic subprocess output reader closed"));
            }
        };
        self.apply_output_message(message, buffer, completion_output)
            .map(Some)
    }

    fn apply_output_message<W: Write + ?Sized>(
        &mut self,
        message: GenericProcessOutputMessage,
        buffer: &mut Vec<u8>,
        completion_output: &mut W,
    ) -> Result<bool, Diagnostic> {
        buffer.clear();
        match message {
            GenericProcessOutputMessage::Chunk(chunk) => {
                buffer.extend_from_slice(&chunk);
                self.get_result_from_optional_chunk(Some(buffer), completion_output)
            }
            GenericProcessOutputMessage::Eof => {
                self.output_eof = true;
                self.get_result_from_optional_chunk(None, completion_output)
            }
            GenericProcessOutputMessage::Error(error) => Err(gproc_ctrl_system_error(format!(
                "Cannot read generic subprocess output: {error}"
            ))),
        }
    }

    fn update_result_from_output(&mut self) {
        let output = self.output.view();
        if output.contains(SZS_THEOREM_STR) || output.contains(SZS_CONTRAAX_STR) {
            self.result = ProverResult::Theorem;
        } else if output.contains(SZS_UNSAT_STR) {
            self.result = ProverResult::Unsatisfiable;
        } else if output.contains(SZS_SATSTR_STR) {
            self.result = ProverResult::Satisfiable;
        } else if output.contains(SZS_COUNTERSAT_STR) {
            self.result = ProverResult::CounterSatisfiable;
        } else {
            self.result = ProverResult::Failure;
        }
    }

    fn wait_for_child_exit(&mut self) -> i32 {
        let Some(mut child) = self.child.take() else {
            return self.exit_status;
        };
        let reported_usage = last_reported_resource_usage(&self.output.view());
        let status = child.wait();
        record_waited_child_resource_usage_with_report(&child, reported_usage);
        match status {
            Ok(status) => status.code().unwrap_or(-1),
            Err(_) => -1,
        }
    }
}

fn last_reported_resource_usage(output: &str) -> Option<ResourceUsage> {
    let mut user_time_seconds = None;
    let mut system_time_seconds = None;
    for line in output.lines() {
        if let Some(value) = line.strip_prefix("% User time                : ") {
            user_time_seconds = value.strip_suffix(" s")?.parse::<f64>().ok();
        } else if let Some(value) = line.strip_prefix("% System time              : ") {
            system_time_seconds = value.strip_suffix(" s")?.parse::<f64>().ok();
        }
    }
    Some(ResourceUsage {
        user_time_seconds: user_time_seconds?,
        system_time_seconds: system_time_seconds?,
        max_resident_pages: 0,
    })
}

impl Drop for EGPCtrl {
    fn drop(&mut self) {
        let _cleanup_result = self.cleanup();
    }
}

#[derive(Debug, Default)]
pub struct EGPCtrlSet {
    cores_reserved: usize,
    procs: BTreeMap<Descriptor, EGPCtrl>,
    buffer: Vec<u8>,
}

impl EGPCtrlSet {
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
    pub const fn cores_reserved(&self) -> usize {
        self.cores_reserved
    }

    #[must_use]
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn add_proc(&mut self, control: EGPCtrl) -> Result<Option<EGPCtrl>, Diagnostic> {
        let descriptor = control.descriptor().ok_or_else(|| {
            gproc_ctrl_error(format!(
                "Cannot add generic process control {:?} without a descriptor",
                control.name()
            ))
        })?;
        self.cores_reserved += control.cores();
        let previous = self.procs.insert(descriptor, control);
        if let Some(previous) = &previous {
            self.cores_reserved = self.cores_reserved.saturating_sub(previous.cores());
        }
        Ok(previous)
    }

    #[must_use]
    pub fn find_proc(&self, descriptor: Descriptor) -> Option<&EGPCtrl> {
        self.procs.get(&descriptor)
    }

    pub fn find_proc_mut(&mut self, descriptor: Descriptor) -> Option<&mut EGPCtrl> {
        self.procs.get_mut(&descriptor)
    }

    pub fn delete_proc(
        &mut self,
        descriptor: Descriptor,
        kill_proc: bool,
    ) -> Result<Option<EGPCtrl>, Diagnostic> {
        let Some(mut control) = self.procs.remove(&descriptor) else {
            return Ok(None);
        };
        if kill_proc {
            control.cleanup()?;
        }
        self.cores_reserved = self.cores_reserved.saturating_sub(control.cores());
        Ok(Some(control))
    }

    pub fn clear(&mut self, kill_proc: bool) -> Result<(), Diagnostic> {
        let descriptors = self.procs.keys().copied().collect::<Vec<_>>();
        for descriptor in descriptors {
            let _deleted = self.delete_proc(descriptor, kill_proc)?;
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
        output: &mut impl Write,
        mut read_result: F,
    ) -> Result<Option<Descriptor>, Diagnostic>
    where
        F: FnMut(&mut EGPCtrl, &mut Vec<u8>, &mut dyn Write) -> Result<bool, Diagnostic>,
    {
        let ready_descriptors = self
            .procs
            .keys()
            .copied()
            .filter(|descriptor| ready.contains_read(*descriptor))
            .collect::<Vec<_>>();

        for descriptor in ready_descriptors {
            self.buffer.clear();
            let eof = {
                let Some(control) = self.procs.get_mut(&descriptor) else {
                    continue;
                };
                read_result(control, &mut self.buffer, output)?
            };
            if eof {
                if let Some(descriptor) = self.handle_eof_result(descriptor)? {
                    return Ok(Some(descriptor));
                }
            }
        }
        Ok(None)
    }

    pub fn get_result_from_pipes(
        &mut self,
        ready: &DescriptorInterestSet,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        self.get_result_from_ready(ready, output, |control, buffer, output| {
            control.read_result_chunk(buffer, output)
        })
    }

    pub fn get_result_from_available_pipes(
        &mut self,
        output: &mut impl Write,
    ) -> Result<(Option<Descriptor>, bool), Diagnostic> {
        let descriptors = self.procs.keys().copied().collect::<Vec<_>>();
        let mut saw_output = false;

        for descriptor in descriptors {
            self.buffer.clear();
            let Some(eof) = ({
                let Some(control) = self.procs.get_mut(&descriptor) else {
                    continue;
                };
                control.try_read_result_chunk(&mut self.buffer, output)?
            }) else {
                continue;
            };
            saw_output = true;
            if eof {
                if let Some(descriptor) = self.handle_eof_result(descriptor)? {
                    return Ok((Some(descriptor), saw_output));
                }
            }
        }

        Ok((None, saw_output))
    }

    pub fn get_result_from_pipes_timeout(
        &mut self,
        timeout: Duration,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        let start = Instant::now();
        loop {
            let (result_descriptor, saw_output) = self.get_result_from_available_pipes(output)?;
            if result_descriptor.is_some() || saw_output {
                return Ok(result_descriptor);
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

    pub fn get_result(
        &mut self,
        output: &mut impl Write,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        self.get_result_from_pipes_timeout(EGPCTRL_SET_WAIT_TIMEOUT, output)
    }

    fn handle_eof_result(
        &mut self,
        descriptor: Descriptor,
    ) -> Result<Option<Descriptor>, Diagnostic> {
        match self
            .procs
            .get(&descriptor)
            .map_or(ProverResult::NoResult, EGPCtrl::result)
        {
            ProverResult::Satisfiable
            | ProverResult::CounterSatisfiable
            | ProverResult::Theorem
            | ProverResult::Unsatisfiable => Ok(Some(descriptor)),
            ProverResult::Failure => {
                let _deleted = self.delete_proc(descriptor, true)?;
                Ok(None)
            }
            ProverResult::NoResult | ProverResult::GaveUp => Err(gproc_ctrl_error(
                "Generic process control reached impossible result state",
            )),
        }
    }
}

impl Drop for EGPCtrlSet {
    fn drop(&mut self) {
        let _cleanup_result = self.clear(true);
    }
}

impl SessionProcessSet for EGPCtrlSet {
    fn init_read_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor {
        self.init_fd_set(interests)
    }
}

fn gproc_ctrl_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

fn gproc_ctrl_system_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYSTEM_ERROR, message)
}

fn output_error(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write generic process control output: {error}"),
    )
}

fn cleanup_child(child: &mut Child) {
    if !terminate_process(child.id()) {
        let _kill_result = child.kill();
    }
    let _wait_result = child.wait();
    record_waited_child_resource_usage(child);
}

fn spawn_output_reader(
    mut stdout: ChildStdout,
) -> (Receiver<GenericProcessOutputMessage>, JoinHandle<()>) {
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut buffer = vec![0_u8; EGPCTRL_BUFSIZE - 1];
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => {
                    let _send_result = sender.send(GenericProcessOutputMessage::Eof);
                    break;
                }
                Ok(read) => {
                    if sender
                        .send(GenericProcessOutputMessage::Chunk(buffer[..read].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    let _send_result =
                        sender.send(GenericProcessOutputMessage::Error(error.to_string()));
                    break;
                }
            }
        }
    });
    (receiver, handle)
}

#[cfg(unix)]
fn descriptor_from_child_stdout(stdout: &ChildStdout) -> Result<Descriptor, Diagnostic> {
    use std::os::fd::AsRawFd;

    let raw = stdout.as_raw_fd();
    u64::try_from(raw)
        .map(Descriptor::new)
        .map_err(|_| gproc_ctrl_error(format!("Invalid generic pipe descriptor: {raw}")))
}

#[cfg(windows)]
fn descriptor_from_child_stdout(stdout: &ChildStdout) -> Result<Descriptor, Diagnostic> {
    use std::os::windows::io::AsRawHandle;

    let raw = stdout.as_raw_handle() as usize;
    if raw == 0 {
        Err(gproc_ctrl_error(
            "Invalid generic pipe descriptor: null handle",
        ))
    } else {
        Ok(Descriptor::new(u64::try_from(raw).unwrap_or(u64::MAX)))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        last_reported_resource_usage, EGPCtrl, EGPCtrlSet, GenericProcessOutputMessage,
        EGPCTRL_BUFSIZE, EGPCTRL_SET_WAIT_TIMEOUT,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProverResult;
    use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};
    use std::process::Command;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn allocation_defaults_match_c_initialization() {
        let control = EGPCtrl::new(3);

        assert_eq!(control.name(), None);
        assert_eq!(control.pid(), None);
        assert_eq!(control.descriptor(), None);
        assert_eq!(control.exit_status(), 0);
        assert_eq!(control.cpu_limit(), 0);
        assert_eq!(control.cores(), 3);
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(control.output().is_empty());
        assert!(!control.has_child());
    }

    #[test]
    fn reported_resource_usage_uses_last_nested_summary() {
        let output = "% User time                : 1.250 s\n\
                      % System time              : 0.500 s\n\
                      % User time                : 3.750 s\n\
                      % System time              : 1.125 s\n";

        let usage = last_reported_resource_usage(output).unwrap();

        assert_eq!(usage.user_time_seconds, 3.75);
        assert_eq!(usage.system_time_seconds, 1.125);
    }

    #[test]
    fn setters_update_c_surface_fields() {
        let mut control = EGPCtrl::new(1);

        control.set_name(Some("worker".to_owned()));
        control.set_pid(Some(99));
        control.set_descriptor(Some(Descriptor::new(4)));
        control.set_cpu_limit(12);

        assert_eq!(control.name(), Some("worker"));
        assert_eq!(control.pid(), Some(99));
        assert_eq!(control.descriptor(), Some(Descriptor::new(4)));
        assert_eq!(control.cpu_limit(), 12);
    }

    #[test]
    fn result_is_detected_from_complete_output_at_eof() {
        let mut control = EGPCtrl::with_descriptor("worker", Descriptor::new(5), 2);
        let mut status_output = Vec::new();

        assert!(!control
            .get_result_from_optional_chunk(
                Some(b"prefix % SZS status Theorem\n"),
                &mut status_output
            )
            .unwrap());
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(control
            .get_result_from_optional_chunk(None, &mut status_output)
            .unwrap());

        assert_eq!(control.result(), ProverResult::Theorem);
        assert_eq!(control.exit_status(), 0);
        assert_eq!(
            String::from_utf8(status_output).unwrap(),
            "% worker with pid 0 completed with status 0\n"
        );
    }

    #[test]
    fn eof_without_status_becomes_failure() {
        let mut control = EGPCtrl::with_descriptor("worker", Descriptor::new(5), 2);
        let mut status_output = Vec::new();

        assert!(!control
            .get_result_from_optional_chunk(Some(b"no result here"), &mut status_output)
            .unwrap());
        assert!(control
            .get_result_from_optional_chunk(None, &mut status_output)
            .unwrap());

        assert_eq!(control.result(), ProverResult::Failure);
    }

    #[test]
    fn process_set_tracks_descriptors_and_reserved_cores() {
        let mut set = EGPCtrlSet::new();
        set.add_proc(EGPCtrl::with_descriptor("a", Descriptor::new(4), 2))
            .unwrap();
        set.add_proc(EGPCtrl::with_descriptor("b", Descriptor::new(9), 3))
            .unwrap();
        let mut interests = DescriptorInterestSet::default();

        assert_eq!(set.cardinality(), 2);
        assert_eq!(set.cores_reserved(), 5);
        assert_eq!(set.init_fd_set(&mut interests), Descriptor::new(9));
        assert!(interests.contains_read(Descriptor::new(4)));
        assert!(interests.contains_read(Descriptor::new(9)));
        assert_eq!(
            SessionProcessSet::init_read_fd_set(&set, &mut DescriptorInterestSet::default()),
            Descriptor::new(9)
        );

        let deleted = set.delete_proc(Descriptor::new(4), false).unwrap();
        assert!(deleted.is_some());
        assert_eq!(set.cores_reserved(), 3);
    }

    #[test]
    fn process_set_returns_first_success_and_deletes_failures() {
        let mut set = EGPCtrlSet::new();
        set.add_proc(EGPCtrl::with_descriptor("failure", Descriptor::new(2), 1))
            .unwrap();
        set.add_proc(EGPCtrl::with_descriptor("success", Descriptor::new(5), 1))
            .unwrap();
        set.add_proc(EGPCtrl::with_descriptor("later", Descriptor::new(7), 1))
            .unwrap();
        let mut ready = DescriptorInterestSet::default();
        ready.set_read(Descriptor::new(2));
        ready.set_read(Descriptor::new(5));
        ready.set_read(Descriptor::new(7));
        let mut output = Vec::new();

        let result = set
            .get_result_from_ready(&ready, &mut output, |control, _buffer, output| {
                if control.name() == Some("success") {
                    let _done = control.get_result_from_optional_chunk(
                        Some(b"% SZS status Satisfiable\n"),
                        output,
                    )?;
                } else if control.name() == Some("later") {
                    let _done = control
                        .get_result_from_optional_chunk(Some(b"% SZS status Theorem\n"), output)?;
                }
                control.get_result_from_optional_chunk(None, output)
            })
            .unwrap();

        assert_eq!(result, Some(Descriptor::new(5)));
        assert!(set.find_proc(Descriptor::new(2)).is_none());
        assert!(set.find_proc(Descriptor::new(5)).is_some());
        assert!(set.find_proc(Descriptor::new(7)).is_some());
        assert_eq!(set.cores_reserved(), 2);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% failure with pid 0 completed with status 0\n% success with pid 0 completed with status 0\n"
        );
    }

    #[test]
    fn spawn_command_reporting_reads_chunks_and_exit_status() {
        let mut startup_output = Vec::new();
        let mut control = EGPCtrl::spawn_command_reporting(
            status_command("% SZS status CounterSatisfiable", 0),
            "spawned",
            2,
            11,
            &mut startup_output,
        )
        .unwrap();
        let mut buffer = Vec::with_capacity(EGPCTRL_BUFSIZE);
        let mut completion_output = Vec::new();

        assert!(control.pid().is_some());
        assert!(control.descriptor().is_some());
        assert!(control.has_child());
        assert_eq!(
            String::from_utf8(startup_output).unwrap(),
            "% Starting spawned with 11s (2) cores\n"
        );

        while !control
            .read_result_chunk(&mut buffer, &mut completion_output)
            .unwrap()
        {}

        assert_eq!(control.result(), ProverResult::CounterSatisfiable);
        assert_eq!(control.exit_status(), 0);
        assert!(control
            .output()
            .view()
            .contains("% SZS status CounterSatisfiable"));
        assert!(String::from_utf8(completion_output)
            .unwrap()
            .contains("% spawned with pid "));
        assert!(!control.has_child());
        control.cleanup().unwrap();
    }

    #[test]
    fn process_set_timeout_poll_reads_child_success() {
        let success = EGPCtrl::spawn_command(
            status_command("% SZS status Unsatisfiable", 0),
            "success",
            2,
            3,
        )
        .unwrap();
        let success_descriptor = success.descriptor().unwrap();
        let mut set = EGPCtrlSet::new();
        set.add_proc(success).unwrap();
        let mut output = Vec::new();
        let mut result = None;

        for _ in 0..10 {
            result = set
                .get_result_from_pipes_timeout(Duration::from_millis(500), &mut output)
                .unwrap();
            if result.is_some() {
                break;
            }
        }

        assert_eq!(result, Some(success_descriptor));
        assert!(set.find_proc(success_descriptor).is_some());
        assert_eq!(set.cores_reserved(), 2);
    }

    #[test]
    fn c_compatible_poll_uses_500ms_and_reads_one_chunk_per_process() {
        assert_eq!(EGPCTRL_SET_WAIT_TIMEOUT, Duration::from_millis(500));

        let descriptor = Descriptor::new(4);
        let (sender, receiver) = mpsc::channel();
        sender
            .send(GenericProcessOutputMessage::Chunk(
                b"% SZS status Theorem\n".to_vec(),
            ))
            .unwrap();
        sender.send(GenericProcessOutputMessage::Eof).unwrap();
        let mut control = EGPCtrl::with_descriptor("proof", descriptor, 1);
        control.output_rx = Some(receiver);
        let mut set = EGPCtrlSet::new();
        set.add_proc(control).unwrap();
        let mut output = Vec::new();

        assert_eq!(set.get_result(&mut output).unwrap(), None);
        let control = set.find_proc(descriptor).unwrap();
        assert_eq!(control.result(), ProverResult::NoResult);
        assert!(!control.output_eof);

        assert_eq!(set.get_result(&mut output).unwrap(), Some(descriptor));
        assert!(set.find_proc(descriptor).unwrap().output_eof);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% proof with pid 0 completed with status 0\n"
        );
    }

    #[test]
    fn c_compatible_poll_surfaces_pipe_read_error_as_system_error() {
        let descriptor = Descriptor::new(6);
        let (sender, receiver) = mpsc::channel();
        sender
            .send(GenericProcessOutputMessage::Error("broken pipe".to_owned()))
            .unwrap();
        let mut control = EGPCtrl::with_descriptor("failed", descriptor, 1);
        control.output_rx = Some(receiver);
        let mut set = EGPCtrlSet::new();
        set.add_proc(control).unwrap();
        let mut output = Vec::new();

        let error = set.get_result(&mut output).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYSTEM_ERROR);
        assert_eq!(
            error.message(),
            "Cannot read generic subprocess output: broken pipe"
        );
        assert!(set.find_proc(descriptor).is_some());
        assert!(output.is_empty());
    }

    #[test]
    fn c_compatible_poll_preserves_empty_set_timeout() {
        let mut set = EGPCtrlSet::new();
        let mut output = Vec::new();
        let timeout = Duration::from_millis(20);
        let start = std::time::Instant::now();

        assert_eq!(
            set.get_result_from_pipes_timeout(timeout, &mut output)
                .unwrap(),
            None
        );
        assert!(start.elapsed() >= timeout);
        assert!(output.is_empty());
    }

    #[test]
    fn add_proc_rejects_missing_descriptor() {
        let mut set = EGPCtrlSet::new();
        let error = set.add_proc(EGPCtrl::new(1)).unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
    }

    #[cfg(windows)]
    fn status_command(status: &str, exit_code: i32) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("echo {status}& exit /B {exit_code}")]);
        command
    }

    #[cfg(unix)]
    fn status_command(status: &str, exit_code: i32) -> Command {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            &format!("printf '%s\\n' '{status}'; exit {exit_code}"),
        ]);
        command
    }
}
