use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProverResult;
use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};
use crate::inout::tempfile::temp_file_remove;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EPCtrl {
    pid: Option<u32>,
    descriptor: Option<Descriptor>,
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
        self.pid = None;
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
                match self
                    .procs
                    .get(&descriptor)
                    .map_or(ProverResult::NoResult, EPCtrl::result)
                {
                    ProverResult::NoResult => {}
                    ProverResult::Theorem | ProverResult::Unsatisfiable => {
                        proof_descriptor = Some(descriptor);
                    }
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
                    }
                    ProverResult::GaveUp => {
                        return Err(proc_ctrl_error(
                            "Process control reached impossible GaveUp result state",
                        ));
                    }
                }
            }
        }
        Ok(proof_descriptor)
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

#[cfg(test)]
mod tests {
    use super::{
        e_ctrl_command, e_ctrl_default_command, prover_result_table_entry, EPCtrl, EPCtrlSet,
        E_OPTIONS, E_OPTIONS_BASE, SZS_CONTRAAX_STR, SZS_COUNTERSAT_STR, SZS_FAILURE_STR,
        SZS_GAVEUP_STR, SZS_SATSTR_STR, SZS_THEOREM_STR, SZS_UNSAT_STR,
    };
    use crate::basics::simple_stuff::ProverResult;
    use crate::control::esession::{Descriptor, DescriptorInterestSet, SessionProcessSet};

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
}
