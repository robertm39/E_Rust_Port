use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{
    reset_problem_type, set_problem_type, ProblemType, ProverResult,
};
use crate::basics::stringtrees::StrTree;
use crate::clauses::clause::{clause_parse, Clause};
use crate::clauses::clause_props::{
    clause_type_from_identifier, CP_INITIAL, CP_INPUT_FORMULA, CP_TYPE_WATCH_CLAUSE,
};
use crate::clauses::clausefunc::{tcf_tstp_parse, tformula_collect_clause, tformula_has_free_vars};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
use crate::clauses::sine::{pstack_clause_write_tstp, pstack_formula_write_tstp};
use crate::control::esession::{Descriptor, DescriptorInterestSet};
use crate::control::gproc_ctrl::EGPCtrl;
use crate::control::proc_ctrl::{prover_result_table_entry, EPCtrl, EPCtrlSet, MAX_CORES};
use crate::control::sine::{StructFofSpec, StructFofSpecSelection};
use crate::heuristics::axfilter::{AxFilter, AxFilterType};
use crate::inout::basicparser::{
    accept_dotted_id, parse_basic_include, parse_continuous, parse_dotted_id, parse_filename,
    parse_int, parse_skip_parenthesized_expr,
};
use crate::inout::scanner::{
    token_pos_rep, IoFormat, Scanner, TokenType, EMPTY_INCLUDE_SELECTOR_SENTINEL,
};
use crate::inout::tempfile::temp_file_create;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::fs::File;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const BATCH_FILTERS: &[&str] = &[
    "threshold010000",
    "gf600_h_gu_R05_F100_L20000",
    "gf120_h_gu_R02_F100_L20000",
    "gf200_gu_RUU_F100_L20000",
    "gf200_h_gu_R03_F100_L20000",
    "gf120_h_gu_RUU_F100_L00100",
    "gf500_h_gu_R04_F100_L20000",
    "gf150_gu_RUU_F100_L20000",
    "gf120_h_gu_RUU_F100_L00500",
    "gf120_gu_RUU_F100_L01000",
    "gf120_gu_R02_F100_L20000",
    "gf500_gu_R04_F100_L20000",
    "gf600_gu_R05_F100_L20000",
];

pub const BATCH_STRATEGIES: &[&str] = &[
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
];

pub const BATCH_FILTERS_DIV: &[&str] = &[
    "threshold010000",
    "gf600_h_gu_R05_F100_L20000",
    "gf120_h_gu_R02_F100_L20000",
    "gf200_gu_RUU_F100_L20000",
    "gf200_h_gu_R03_F100_L20000",
    "gf120_h_gu_RUU_F100_L00100",
    "gf500_h_gu_R04_F100_L20000",
    "gf150_gu_RUU_F100_L20000",
    "gf120_h_gu_RUU_F100_L00500",
    "gf120_gu_RUU_F100_L01000",
    "gf120_gu_R02_F100_L20000",
    "gf500_gu_R04_F100_L20000",
    "gf600_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
    "gf600_h_gu_R05_F100_L20000",
];

pub const BATCH_STRATEGIES_DIV: &[&str] = &[
    "--auto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "--satauto-schedule --assume-incompleteness",
    "-xAutoSched2 -tAutoSched2 --assume-incompleteness",
    "-xAutoSched3 -tAutoSched3 --assume-incompleteness",
    "-xAutoSched4 -tAutoSched4 --assume-incompleteness",
    "-xAutoSched5 -tAutoSched5 --assume-incompleteness",
];

pub const BATCH_PROCESS_POLL_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum BatchOutputType {
    #[default]
    NoOutput = 0,
    Desired = 1,
    Required = 2,
}

impl BatchOutputType {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSpecHeader {
    pub category: String,
    pub train_dir: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSpec {
    pub executable: String,
    pub format: IoFormat,
    pub category: Option<String>,
    pub train_dir: Option<String>,
    pub ordered: bool,
    pub res_assurance: BatchOutputType,
    pub res_proof: BatchOutputType,
    pub res_model: BatchOutputType,
    pub res_answer: BatchOutputType,
    pub res_list_fof: BatchOutputType,
    pub per_prob_limit: i64,
    pub total_wtc_limit: i64,
    pub includes: Vec<String>,
    pub source_files: Vec<String>,
    pub dest_files: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatchProcessProblemsConfig<'a> {
    pub total_wtc_limit: i64,
    pub default_dir: Option<&'a str>,
    pub dest_dir: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProcessProblemJob<'a> {
    pub index: usize,
    pub wct_limit: i64,
    pub default_dir: Option<&'a str>,
    pub source: &'a str,
    pub dest: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchProcessProblemRecord {
    pub index: usize,
    pub source: String,
    pub dest: String,
    pub wct_limit: i64,
    pub solved: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BatchProblemData {
    pub clauses: ClauseSet,
    pub formulas: FormulaSet,
    pub problem_type: ProblemType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProcessProblemConfig<'a> {
    pub wct_limit: i64,
    pub jobname: &'a str,
    pub interactive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProcessFileConfig<'a> {
    pub wct_limit: i64,
    pub default_dir: Option<&'a str>,
    pub source: &'a str,
    pub dest: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProblemLoadRequest<'a> {
    pub source: &'a str,
    pub default_dir: Option<&'a str>,
    pub format: IoFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProcessVariantsConfig<'a> {
    pub variants: &'a [&'a str],
    pub provers: &'a [&'a str],
    pub start: i64,
    pub default_dir: Option<&'a str>,
    pub outdir: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchVariantProblemJob {
    pub variant_index: usize,
    pub problem_index: usize,
    pub variant: String,
    pub prover: String,
    pub abstract_source: String,
    pub concrete_source: String,
    pub dest: String,
    pub wct_limit: i64,
    pub default_dir: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchVariantProblemOutcome {
    pub solved: bool,
    pub output: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchProcessProblemsReport {
    pub solved: i64,
    pub records: Vec<BatchProcessProblemRecord>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BatchRunnerCreateConfig<'a> {
    pub options: &'a str,
    pub extra_options: &'a str,
    pub cpu_time: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchRunnerProblemConfig {
    pub problem_type: ProblemType,
    pub keep_input_names: bool,
}

impl Default for BatchRunnerProblemConfig {
    fn default() -> Self {
        Self {
            problem_type: ProblemType::FirstOrder,
            keep_input_names: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRunnerRequest {
    pub executable: String,
    pub name: String,
    pub options: String,
    pub extra_options: String,
    pub cpu_time: i64,
    pub selected_count: i64,
    pub selected_clauses: usize,
    pub selected_formulas: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRunnerPreparedRequest {
    pub request: BatchRunnerRequest,
    pub problem_tstp: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchRunnerTempRequest {
    pub request: BatchRunnerRequest,
    pub input_file: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSpawnedRunner {
    pub name: String,
    pub start_time: i64,
    pub prob_time: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchCompletedRunner {
    pub runner: BatchSpawnedRunner,
    pub result: ProverResult,
    pub output: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchProcessProblemReport {
    pub solved: bool,
    pub spawned: usize,
    pub completed: Option<BatchCompletedRunner>,
    pub backtrack: crate::control::sine::StructFofSpecBacktrackReport,
}

pub struct BatchProcessProblemOutputs<'a, WGlobal: Write + ?Sized, WExternal: Write + ?Sized> {
    pub global_output: &'a mut WGlobal,
    pub external_output: Option<&'a mut WExternal>,
    pub socket_output: Option<&'a mut (dyn Write + 'a)>,
}

pub struct BatchProcessFileOutputs<'a, WGlobal: Write + ?Sized, WDest: Write + ?Sized> {
    pub global_output: &'a mut WGlobal,
    pub dest_output: &'a mut WDest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchProcessFileReport {
    pub source: String,
    pub dest: String,
    pub solved: bool,
    pub problem: BatchProcessProblemReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchVariantProblemRecord {
    pub variant_index: usize,
    pub problem_index: usize,
    pub variant: String,
    pub prover: String,
    pub abstract_source: String,
    pub concrete_source: String,
    pub dest: String,
    pub wct_limit: i64,
    pub solved: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchProcessVariantsReport {
    pub solved: i64,
    pub attempted: usize,
    pub records: Vec<BatchVariantProblemRecord>,
}

#[derive(Debug)]
pub struct BatchProcCtrlRunnerSet {
    controls: EPCtrlSet,
    poll_timeout: Duration,
}

pub trait BatchRunnerBackend {
    #[must_use]
    fn active_count(&self) -> usize;

    fn spawn_runner(
        &mut self,
        request: BatchRunnerTempRequest,
    ) -> Result<BatchSpawnedRunner, Diagnostic>;

    fn poll_runner<W: Write>(
        &mut self,
        output: &mut W,
    ) -> Result<Option<BatchCompletedRunner>, Diagnostic>;

    fn clear(&mut self, delete_files: bool) -> Result<(), Diagnostic>;
}

impl Default for BatchProcCtrlRunnerSet {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchProcCtrlRunnerSet {
    #[must_use]
    pub fn new() -> Self {
        Self {
            controls: EPCtrlSet::new(),
            poll_timeout: BATCH_PROCESS_POLL_TIMEOUT,
        }
    }

    #[must_use]
    pub fn with_poll_timeout(poll_timeout: Duration) -> Self {
        Self {
            controls: EPCtrlSet::new(),
            poll_timeout,
        }
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.controls.cardinality()
    }

    pub fn add_control(&mut self, control: EPCtrl) -> Result<BatchSpawnedRunner, Diagnostic> {
        let spawned = spawned_runner_from_control(&control);
        let _previous = self.controls.add_proc(control)?;
        Ok(spawned)
    }

    pub fn spawn_runner_from_file(
        &mut self,
        request: &BatchRunnerRequest,
        input_file: impl Into<PathBuf>,
    ) -> Result<BatchSpawnedRunner, Diagnostic> {
        let control = EPCtrl::create_generic(
            &request.executable,
            &request.name,
            &request.options,
            &request.extra_options,
            request.cpu_time,
            input_file,
        )?;
        self.add_control(control)
    }

    pub fn spawn_temp_runner(
        &mut self,
        request: BatchRunnerTempRequest,
    ) -> Result<BatchSpawnedRunner, Diagnostic> {
        self.spawn_runner_from_file(&request.request, request.input_file)
    }

    pub fn poll_runners<W: Write>(
        &mut self,
        output: &mut W,
    ) -> Result<Option<BatchCompletedRunner>, Diagnostic> {
        let descriptor =
            self.controls
                .get_result_from_pipes_timeout(self.poll_timeout, true, output)?;
        descriptor
            .map(|descriptor| self.completed_runner(descriptor))
            .transpose()
    }

    pub fn poll_runners_from_ready<W, F>(
        &mut self,
        ready: &DescriptorInterestSet,
        delete_files: bool,
        output: &mut W,
        read_result: F,
    ) -> Result<Option<BatchCompletedRunner>, Diagnostic>
    where
        W: Write,
        F: FnMut(&mut EPCtrl, &mut String) -> Result<bool, Diagnostic>,
    {
        let descriptor =
            self.controls
                .get_result_from_ready(ready, delete_files, output, read_result)?;
        descriptor
            .map(|descriptor| self.completed_runner(descriptor))
            .transpose()
    }

    pub fn clear(&mut self, delete_files: bool) -> Result<(), Diagnostic> {
        self.controls.clear(delete_files)
    }

    fn completed_runner(&self, descriptor: Descriptor) -> Result<BatchCompletedRunner, Diagnostic> {
        let control = self
            .controls
            .find_proc(descriptor)
            .ok_or_else(|| batch_process_error("Missing completed batch runner"))?;
        Ok(BatchCompletedRunner {
            runner: spawned_runner_from_control(control),
            result: control.result(),
            output: control.output().view().into_owned(),
        })
    }
}

impl BatchRunnerBackend for BatchProcCtrlRunnerSet {
    fn active_count(&self) -> usize {
        self.active_count()
    }

    fn spawn_runner(
        &mut self,
        request: BatchRunnerTempRequest,
    ) -> Result<BatchSpawnedRunner, Diagnostic> {
        self.spawn_temp_runner(request)
    }

    fn poll_runner<W: Write>(
        &mut self,
        output: &mut W,
    ) -> Result<Option<BatchCompletedRunner>, Diagnostic> {
        self.poll_runners(output)
    }

    fn clear(&mut self, delete_files: bool) -> Result<(), Diagnostic> {
        self.clear(delete_files)
    }
}

impl Drop for BatchProcCtrlRunnerSet {
    fn drop(&mut self) {
        let _cleanup_result = self.controls.clear(true);
    }
}

impl BatchProcessProblemsReport {
    #[must_use]
    pub const fn c_return_value(&self) -> i64 {
        self.solved
    }
}

impl BatchSpec {
    #[must_use]
    pub fn new(executable: impl Into<String>, format: IoFormat) -> Self {
        Self {
            executable: executable.into(),
            format,
            category: None,
            train_dir: None,
            ordered: false,
            res_assurance: BatchOutputType::NoOutput,
            res_proof: BatchOutputType::NoOutput,
            res_model: BatchOutputType::NoOutput,
            res_answer: BatchOutputType::NoOutput,
            res_list_fof: BatchOutputType::NoOutput,
            per_prob_limit: 0,
            total_wtc_limit: 0,
            includes: Vec::new(),
            source_files: Vec::new(),
            dest_files: Vec::new(),
        }
    }

    pub fn parse(
        scanner: &mut Scanner,
        executable: impl Into<String>,
        category: &str,
        train_dir: Option<&str>,
        format: IoFormat,
    ) -> Result<Self, Diagnostic> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        Self::parse_with_include_output(
            scanner,
            executable,
            category,
            train_dir,
            format,
            &mut stdout,
        )
    }

    pub fn parse_with_include_output<W: Write + ?Sized>(
        scanner: &mut Scanner,
        executable: impl Into<String>,
        category: &str,
        train_dir: Option<&str>,
        format: IoFormat,
        include_output: &mut W,
    ) -> Result<Self, Diagnostic> {
        let mut spec = Self::new(executable, format);
        spec.category = Some(category.to_owned());
        spec.train_dir = train_dir.map(str::to_owned);

        if scanner.test_id("execution") {
            accept_dotted_id(scanner, "execution.order")?;
            spec.ordered = scanner.test_id("ordered");
            scanner.accept_id("ordered|unordered")?;
        }

        accept_dotted_id(scanner, "output.required")?;
        parse_output_line(scanner, &mut spec, BatchOutputType::Required)?;

        if scanner.test_id("output") {
            accept_dotted_id(scanner, "output.desired")?;
            parse_output_line(scanner, &mut spec, BatchOutputType::Desired)?;
        }

        accept_dotted_id(scanner, "limit.time.problem.wc")?;
        spec.per_prob_limit = parse_int(scanner)?;

        if scanner.test_id("limit") {
            accept_dotted_id(scanner, "limit.time.overall.wc")?;
            spec.total_wtc_limit = parse_int(scanner)?;
        }

        while scanner.test_id("include") {
            let include = parse_basic_include(scanner)?;
            writeln!(include_output, "% Accepted {include} for parsing")
                .map_err(|error| output_error(&error))?;
            spec.includes.push(include);
        }

        while scanner.test_tok(TokenType::SLASH) || scanner.test_id("Problem|Problems") {
            let source = parse_filename(scanner)?;
            let dest = parse_filename(scanner)?;
            spec.source_files.push(source);
            spec.dest_files.push(dest);
        }

        Ok(spec)
    }

    #[must_use]
    pub fn problem_no(&self) -> usize {
        self.source_files.len()
    }

    #[must_use]
    pub const fn answer_options(&self) -> &'static str {
        if matches!(self.res_answer, BatchOutputType::NoOutput) {
            ""
        } else {
            "--conjectures-are-questions"
        }
    }

    pub fn write_to<W: Write + ?Sized>(&self, output: &mut W) -> Result<(), Diagnostic> {
        writeln!(output, "% SZS start BatchConfiguration").map_err(|error| output_error(&error))?;
        writeln!(
            output,
            "division.category {}",
            self.category.as_deref().unwrap_or("")
        )
        .map_err(|error| output_error(&error))?;
        if let Some(train_dir) = &self.train_dir {
            writeln!(output, "division.category.training_directory {train_dir}")
                .map_err(|error| output_error(&error))?;
        }
        if self.ordered {
            writeln!(output, "execution.order ordered").map_err(|error| output_error(&error))?;
        }

        write!(output, "output.required").map_err(|error| output_error(&error))?;
        self.write_output_line(output, BatchOutputType::Required)?;
        writeln!(output).map_err(|error| output_error(&error))?;

        write!(output, "output.desired").map_err(|error| output_error(&error))?;
        self.write_output_line(output, BatchOutputType::Desired)?;
        writeln!(output).map_err(|error| output_error(&error))?;

        writeln!(output, "limit.time.problem.wc {}", self.per_prob_limit)
            .map_err(|error| output_error(&error))?;
        writeln!(output, "limit.time.overall.wc {}", self.total_wtc_limit)
            .map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS end BatchConfiguration").map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS start BatchIncludes").map_err(|error| output_error(&error))?;
        for include in &self.includes {
            writeln!(output, "include('{include}').").map_err(|error| output_error(&error))?;
        }
        writeln!(output, "% SZS end BatchIncludes").map_err(|error| output_error(&error))?;
        writeln!(output, "% SZS start BatchProblems").map_err(|error| output_error(&error))?;
        for (source, dest) in self.source_files.iter().zip(&self.dest_files) {
            writeln!(output, "{source} {dest}").map_err(|error| output_error(&error))?;
        }
        writeln!(output, "% SZS end BatchProblems").map_err(|error| output_error(&error))
    }

    pub fn print_string(&self) -> Result<String, Diagnostic> {
        let mut output = Vec::new();
        self.write_to(&mut output)?;
        String::from_utf8(output).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Could not build batch specification output: {error}"),
            )
        })
    }

    pub fn process_problems_with<F, C>(
        &self,
        config: BatchProcessProblemsConfig<'_>,
        mut clock_seconds: C,
        mut process_file: F,
    ) -> Result<BatchProcessProblemsReport, Diagnostic>
    where
        F: for<'a> FnMut(BatchProcessProblemJob<'a>) -> Result<bool, Diagnostic>,
        C: FnMut() -> i64,
    {
        if self.source_files.len() != self.dest_files.len() {
            return Err(batch_process_error(format!(
                "Batch spec has {} source files but {} destination files",
                self.source_files.len(),
                self.dest_files.len()
            )));
        }

        let start = clock_seconds();
        let mut report = BatchProcessProblemsReport::default();
        let problem_count = self.source_files.len();

        for (index, (source, dest)) in self.source_files.iter().zip(&self.dest_files).enumerate() {
            let now = if config.total_wtc_limit != 0 {
                clock_seconds()
            } else {
                start
            };
            let wct_limit = self.problem_wct_limit(
                config.total_wtc_limit,
                start,
                now,
                problem_count.saturating_sub(index),
            );
            let dest_name = batch_problem_dest_name(config.dest_dir, dest);
            let solved = process_file(BatchProcessProblemJob {
                index,
                wct_limit,
                default_dir: config.default_dir,
                source,
                dest: &dest_name,
            })?;
            if solved {
                report.solved += 1;
            }
            report.records.push(BatchProcessProblemRecord {
                index,
                source: source.clone(),
                dest: dest_name,
                wct_limit,
                solved,
            });
        }

        Ok(report)
    }

    pub fn init_struct_fof_spec_from_files<W: Write + ?Sized>(
        &self,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        default_dir: Option<&str>,
        output: &mut W,
    ) -> Result<i64, Diagnostic> {
        let mut parsed = 0;
        for include in &self.includes {
            if ctrl.has_parsed_include(include) {
                continue;
            }
            let request = BatchProblemLoadRequest {
                source: include,
                default_dir,
                format: self.format,
            };
            let Some(problem) = load_include_problem_from_file(bank, ctrl, request, output)? else {
                continue;
            };
            if !problem.clauses.is_empty() {
                return Err(batch_process_error(format!(
                    "Batch include '{include}' produced {} watchlist clauses",
                    problem.clauses.len()
                )));
            }
            parsed += problem.formulas.cardinality();
            ctrl.push_problem_sets_without_distribution_with_type(
                problem.clauses,
                problem.formulas,
                problem.problem_type,
            );
            ctrl.mark_include_parsed(include);
        }
        ctrl.mark_shared_axioms(bank.signature());
        ctrl.init_distrib(bank.signature(), false);
        Ok(parsed)
    }

    pub fn init_concrete_struct_fof_spec_from_files<W: Write + ?Sized>(
        &self,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        default_dir: Option<&str>,
        variant: &str,
        output: &mut W,
    ) -> Result<i64, Diagnostic> {
        let mut concrete_spec = self.clone();
        concrete_spec.includes = self
            .includes
            .iter()
            .map(|include| abstract_to_concrete(include, variant, ".ax"))
            .collect();
        concrete_spec.init_struct_fof_spec_from_files(bank, ctrl, default_dir, output)
    }

    pub fn load_problem_from_file(
        &self,
        bank: &mut TermBank,
        ctrl: &StructFofSpec,
        request: BatchProblemLoadRequest<'_>,
    ) -> Result<BatchProblemData, Diagnostic> {
        let mut scanner = open_batch_problem_scanner(request)?;
        self.load_problem_from_scanner(bank, ctrl, &mut scanner)
    }

    pub fn load_problem_from_scanner(
        &self,
        bank: &mut TermBank,
        ctrl: &StructFofSpec,
        scanner: &mut Scanner,
    ) -> Result<BatchProblemData, Diagnostic> {
        parse_batch_problem_entries_root(scanner, bank, ctrl)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The staged BatchProcessFile port keeps parser, clock, and runner hooks injectable"
    )]
    pub fn process_file_with<WGlobal, WDest, L, C, S, P>(
        &self,
        signature: &Signature,
        ctrl: &mut StructFofSpec,
        config: BatchProcessFileConfig<'_>,
        outputs: &mut BatchProcessFileOutputs<'_, WGlobal, WDest>,
        mut load_problem: L,
        clock_seconds: C,
        spawn_runner: S,
        poll_runners: P,
    ) -> Result<BatchProcessFileReport, Diagnostic>
    where
        WGlobal: Write + ?Sized,
        WDest: Write + ?Sized,
        L: FnMut(BatchProblemLoadRequest<'_>) -> Result<BatchProblemData, Diagnostic>,
        C: FnMut() -> i64,
        S: FnMut(BatchRunnerRequest) -> Result<BatchSpawnedRunner, Diagnostic>,
        P: FnMut(&mut Vec<BatchSpawnedRunner>) -> Result<Option<BatchCompletedRunner>, Diagnostic>,
    {
        outputs
            .global_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
        let problem = load_problem(BatchProblemLoadRequest {
            source: config.source,
            default_dir: config.default_dir,
            format: IoFormat::Tstp,
        })?;
        let problem = self.process_problem_with(
            signature,
            ctrl,
            problem,
            BatchProcessProblemConfig {
                wct_limit: config.wct_limit,
                jobname: config.source,
                interactive: false,
            },
            BatchProcessProblemOutputs {
                global_output: &mut *outputs.global_output,
                external_output: Some(&mut *outputs.dest_output),
                socket_output: None,
            },
            clock_seconds,
            spawn_runner,
            poll_runners,
        )?;

        Ok(BatchProcessFileReport {
            source: config.source.to_owned(),
            dest: config.dest.to_owned(),
            solved: problem.solved,
            problem,
        })
    }

    pub fn process_problems_with_runner_backend<WGlobal, C, B>(
        &self,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        config: BatchProcessProblemsConfig<'_>,
        global_output: &mut WGlobal,
        mut clock_seconds: C,
        backend: &mut B,
    ) -> Result<BatchProcessProblemsReport, Diagnostic>
    where
        WGlobal: Write,
        C: FnMut() -> i64,
        B: BatchRunnerBackend,
    {
        if self.source_files.len() != self.dest_files.len() {
            return Err(batch_process_error(format!(
                "Batch spec has {} source files but {} destination files",
                self.source_files.len(),
                self.dest_files.len()
            )));
        }

        let start = clock_seconds();
        let mut report = BatchProcessProblemsReport::default();
        let problem_count = self.source_files.len();

        for (index, (source, dest)) in self.source_files.iter().zip(&self.dest_files).enumerate() {
            let now = if config.total_wtc_limit != 0 {
                clock_seconds()
            } else {
                start
            };
            let wct_limit = self.problem_wct_limit(
                config.total_wtc_limit,
                start,
                now,
                problem_count.saturating_sub(index),
            );
            let dest_name = batch_problem_dest_name(config.dest_dir, dest);
            let file = self.process_file_with_runner_backend(
                bank,
                ctrl,
                BatchProcessFileConfig {
                    wct_limit,
                    default_dir: config.default_dir,
                    source,
                    dest: &dest_name,
                },
                global_output,
                &mut clock_seconds,
                backend,
            )?;
            if file.solved {
                report.solved += 1;
            }
            report.records.push(BatchProcessProblemRecord {
                index,
                source: source.clone(),
                dest: dest_name,
                wct_limit,
                solved: file.solved,
            });
        }

        Ok(report)
    }

    pub fn process_file_with_runner_backend<WGlobal, C, B>(
        &self,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        config: BatchProcessFileConfig<'_>,
        global_output: &mut WGlobal,
        clock_seconds: C,
        backend: &mut B,
    ) -> Result<BatchProcessFileReport, Diagnostic>
    where
        WGlobal: Write,
        C: FnMut() -> i64,
        B: BatchRunnerBackend,
    {
        let mut scanner = open_batch_problem_scanner(BatchProblemLoadRequest {
            source: config.source,
            default_dir: config.default_dir,
            format: IoFormat::Tstp,
        })?;
        global_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
        let problem = self.load_problem_from_scanner(bank, ctrl, &mut scanner)?;
        let mut dest = open_batch_dest_file(config.dest)?;
        let problem = self.process_problem_with_runner_backend(
            bank,
            ctrl,
            problem,
            BatchProcessProblemConfig {
                wct_limit: config.wct_limit,
                jobname: config.source,
                interactive: false,
            },
            BatchRunnerProblemConfig::default(),
            BatchProcessProblemOutputs {
                global_output,
                external_output: Some(&mut dest),
                socket_output: None,
            },
            clock_seconds,
            backend,
        )?;

        Ok(BatchProcessFileReport {
            source: config.source.to_owned(),
            dest: config.dest.to_owned(),
            solved: problem.solved,
            problem,
        })
    }

    pub fn process_variants_with<W, C, F>(
        &self,
        config: BatchProcessVariantsConfig<'_>,
        output: &mut W,
        mut clock_seconds: C,
        mut process_problem: F,
    ) -> Result<BatchProcessVariantsReport, Diagnostic>
    where
        W: Write + ?Sized,
        C: FnMut() -> i64,
        F: FnMut(BatchVariantProblemJob) -> Result<BatchVariantProblemOutcome, Diagnostic>,
    {
        self.validate_process_variants_config(&config)?;

        let problem_count = self.source_files.len();
        let variant_count = config.variants.len();
        let problem_count_i64 = usize_to_i64_c(problem_count);
        let variant_count_i64 = usize_to_i64_c(variant_count);
        let mut solved = vec![false; problem_count];
        let mut solved_count = 0;
        let initial_concrete_count = problem_count_i64 * variant_count_i64;
        let mut report = BatchProcessVariantsReport::default();

        write_variant_initial(
            output,
            problem_count_i64,
            variant_count_i64,
            initial_concrete_count,
        )?;

        for (variant_index, (&variant, &prover)) in
            config.variants.iter().zip(config.provers).enumerate()
        {
            let now = clock_seconds();
            let remaining = self.total_wtc_limit - (now - config.start);
            let remaining_variant_count = variant_count_i64 - usize_to_i64_c(variant_index);
            let mut concrete_prob_count =
                (problem_count_i64 - solved_count) * remaining_variant_count;

            write_variant_round(
                output,
                variant_index,
                variant,
                remaining,
                problem_count_i64 - solved_count,
                remaining_variant_count,
                concrete_prob_count,
            )?;

            for (problem_index, (abstract_source, dest_file)) in
                self.source_files.iter().zip(&self.dest_files).enumerate()
            {
                if solved[problem_index] {
                    write_variant_already_solved(output, abstract_source)?;
                    continue;
                }

                if concrete_prob_count == 0 {
                    return Err(batch_process_error(
                        "No concrete variant problems remain for an unsolved abstract problem",
                    ));
                }

                let now = clock_seconds();
                let remaining = self.total_wtc_limit - (now - config.start);
                let per_prob_time = (remaining / concrete_prob_count) + 1;
                let concrete_source = abstract_to_concrete(abstract_source, variant, ".p");
                let dest = batch_problem_dest_name(config.outdir, dest_file);

                write_variant_started(
                    output,
                    abstract_source,
                    &concrete_source,
                    &dest,
                    per_prob_time,
                )?;

                let outcome = process_problem(BatchVariantProblemJob {
                    variant_index,
                    problem_index,
                    variant: variant.to_owned(),
                    prover: prover.to_owned(),
                    abstract_source: abstract_source.clone(),
                    concrete_source: concrete_source.clone(),
                    dest: dest.clone(),
                    wct_limit: per_prob_time,
                    default_dir: config.default_dir.map(str::to_owned),
                })?;

                write!(output, "{}", outcome.output)
                    .map_err(|error| batch_process_output_error(&error))?;
                if outcome.solved {
                    solved_count += 1;
                    solved[problem_index] = true;
                    concrete_prob_count -= remaining_variant_count;
                } else {
                    concrete_prob_count -= 1;
                }
                write_variant_ended(output, &concrete_source)?;

                report.attempted += 1;
                report.records.push(BatchVariantProblemRecord {
                    variant_index,
                    problem_index,
                    variant: variant.to_owned(),
                    prover: prover.to_owned(),
                    abstract_source: abstract_source.clone(),
                    concrete_source,
                    dest,
                    wct_limit: per_prob_time,
                    solved: outcome.solved,
                });
            }
        }

        report.solved = solved_count;
        Ok(report)
    }

    pub fn process_variants_with_child_processes<W, C, S>(
        &self,
        config: BatchProcessVariantsConfig<'_>,
        output: &mut W,
        clock_seconds: C,
        mut spawn_child: S,
    ) -> Result<BatchProcessVariantsReport, Diagnostic>
    where
        W: Write + ?Sized,
        C: FnMut() -> i64,
        S: FnMut(&BatchVariantProblemJob, &mut dyn Write) -> Result<EGPCtrl, Diagnostic>,
    {
        self.process_variants_with(config, output, clock_seconds, |job| {
            run_variant_child_process(&job, &mut spawn_child)
        })
    }

    fn validate_process_variants_config(
        &self,
        config: &BatchProcessVariantsConfig<'_>,
    ) -> Result<(), Diagnostic> {
        if config.variants.len() != config.provers.len() {
            return Err(batch_process_error(format!(
                "Batch variant run has {} variants but {} prover commands",
                config.variants.len(),
                config.provers.len()
            )));
        }
        if self.source_files.len() != self.dest_files.len() {
            return Err(batch_process_error(format!(
                "Batch spec has {} source files but {} destination files",
                self.source_files.len(),
                self.dest_files.len()
            )));
        }
        Ok(())
    }

    pub fn create_runner_request_with<W, C>(
        &self,
        ctrl: &mut StructFofSpec,
        signature: &Signature,
        filter: &AxFilter,
        config: BatchRunnerCreateConfig<'_>,
        output: &mut W,
        mut clock_seconds_mod: C,
    ) -> Result<BatchRunnerRequest, Diagnostic>
    where
        W: Write + ?Sized,
        C: FnMut() -> i64,
    {
        let filter_text = batch_filter_print_string(filter)?;
        writeln!(
            output,
            "% Filtering for {filter_text} ({})",
            clock_seconds_mod()
        )
        .map_err(|error| batch_runner_output_error(&error))?;

        let selection = ctrl.get_problem(signature, filter)?;
        let selected_count = selection.selected_count;
        let selected_clauses = selection.clauses.len();
        let selected_formulas = selection.formulas.len();
        drop(selection);

        writeln!(
            output,
            "% Spec has {selected_clauses} clauses and {selected_formulas} formulas ({})",
            clock_seconds_mod()
        )
        .map_err(|error| batch_runner_output_error(&error))?;
        writeln!(output, "% Written new problem ({})", clock_seconds_mod())
            .map_err(|error| batch_runner_output_error(&error))?;

        Ok(BatchRunnerRequest {
            executable: self.executable.clone(),
            name: batch_filter_runner_name(filter)?,
            options: config.options.to_owned(),
            extra_options: config.extra_options.to_owned(),
            cpu_time: config.cpu_time,
            selected_count,
            selected_clauses,
            selected_formulas,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The staged batch runner file emission mirrors C selection, logging, and TSTP rendering"
    )]
    pub fn create_runner_prepared_request_with<W, C>(
        &self,
        ctrl: &mut StructFofSpec,
        bank: &mut TermBank,
        filter: &AxFilter,
        config: BatchRunnerCreateConfig<'_>,
        problem_config: BatchRunnerProblemConfig,
        output: &mut W,
        mut clock_seconds_mod: C,
    ) -> Result<BatchRunnerPreparedRequest, Diagnostic>
    where
        W: Write + ?Sized,
        C: FnMut() -> i64,
    {
        let filter_text = batch_filter_print_string(filter)?;
        writeln!(
            output,
            "% Filtering for {filter_text} ({})",
            clock_seconds_mod()
        )
        .map_err(|error| batch_runner_output_error(&error))?;

        let selection = ctrl.get_problem(bank.signature(), filter)?;
        let selected_count = selection.selected_count;
        let selected_clauses = selection.clauses.len();
        let selected_formulas = selection.formulas.len();
        let problem_tstp = render_batch_runner_problem_tstp(bank, &selection, problem_config)?;
        drop(selection);

        writeln!(
            output,
            "% Spec has {selected_clauses} clauses and {selected_formulas} formulas ({})",
            clock_seconds_mod()
        )
        .map_err(|error| batch_runner_output_error(&error))?;
        writeln!(output, "% Written new problem ({})", clock_seconds_mod())
            .map_err(|error| batch_runner_output_error(&error))?;

        Ok(BatchRunnerPreparedRequest {
            request: BatchRunnerRequest {
                executable: self.executable.clone(),
                name: batch_filter_runner_name(filter)?,
                options: config.options.to_owned(),
                extra_options: config.extra_options.to_owned(),
                cpu_time: config.cpu_time,
                selected_count,
                selected_clauses,
                selected_formulas,
            },
            problem_tstp,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The staged batch runner temp-file path keeps the C helper boundary explicit"
    )]
    pub fn create_runner_temp_request_with<W, C>(
        &self,
        ctrl: &mut StructFofSpec,
        bank: &mut TermBank,
        filter: &AxFilter,
        config: BatchRunnerCreateConfig<'_>,
        problem_config: BatchRunnerProblemConfig,
        output: &mut W,
        clock_seconds_mod: C,
    ) -> Result<BatchRunnerTempRequest, Diagnostic>
    where
        W: Write + ?Sized,
        C: FnMut() -> i64,
    {
        let prepared = self.create_runner_prepared_request_with(
            ctrl,
            bank,
            filter,
            config,
            problem_config,
            output,
            clock_seconds_mod,
        )?;
        let mut source = Cursor::new(prepared.problem_tstp.into_bytes());
        let input_file = temp_file_create(&mut source)?;
        Ok(BatchRunnerTempRequest {
            request: prepared.request,
            input_file,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The direct BatchProcessProblem port must thread C config, output, clock, and runner backend state"
    )]
    pub fn process_problem_with_runner_backend<WGlobal, WExternal, C, B>(
        &self,
        bank: &mut TermBank,
        ctrl: &mut StructFofSpec,
        problem: BatchProblemData,
        config: BatchProcessProblemConfig<'_>,
        problem_config: BatchRunnerProblemConfig,
        mut outputs: BatchProcessProblemOutputs<'_, WGlobal, WExternal>,
        mut clock_seconds: C,
        backend: &mut B,
    ) -> Result<BatchProcessProblemReport, Diagnostic>
    where
        WGlobal: Write,
        WExternal: Write + ?Sized,
        C: FnMut() -> i64,
        B: BatchRunnerBackend,
    {
        let filters = crate::heuristics::axfilter::AxFilterSet::default_set()?;
        let _pre_add_start = clock_seconds();
        ctrl.add_problem_with_type(
            bank.signature(),
            problem.clauses,
            problem.formulas,
            false,
            problem.problem_type,
        );
        let problem_config = BatchRunnerProblemConfig {
            problem_type: normalize_batch_problem_type(ctrl.problem_type()),
            ..problem_config
        };

        let mut spawn_count = 0;
        let process_result = (|| {
            let start = clock_seconds();
            let end = start + config.wct_limit;
            let mut filter_index = 0;
            let mut completed = None;

            while completed.is_none() && clock_seconds() <= end {
                while filter_index < BATCH_FILTERS.len() && backend.active_count() < MAX_CORES {
                    let now = clock_seconds();
                    if now > end {
                        break;
                    }
                    let used = now - start;
                    let filter_name = BATCH_FILTERS[filter_index];
                    let filter = filters.find_filter(filter_name).ok_or_else(|| {
                        batch_process_error(format!(
                            "Batch filter '{filter_name}' is missing from the default filter set"
                        ))
                    })?;
                    let request = self.create_runner_temp_request_with(
                        ctrl,
                        bank,
                        filter,
                        BatchRunnerCreateConfig {
                            options: BATCH_STRATEGIES[filter_index],
                            extra_options: self.answer_options(),
                            cpu_time: batch_runner_cpu_time(config.wct_limit, used),
                        },
                        problem_config,
                        &mut *outputs.global_output,
                        || clock_seconds() % 1000,
                    )?;
                    let _spawned = backend.spawn_runner(request)?;
                    spawn_count += 1;
                    filter_index += 1;
                }

                if let Some(done) = backend.poll_runner(&mut *outputs.global_output)? {
                    completed = Some(done);
                    break;
                }
            }

            if let Some(completed_runner) = completed {
                write_problem_success(config, &completed_runner, &mut outputs, clock_seconds())?;
                Ok((true, Some(completed_runner)))
            } else {
                write_problem_gave_up(config, &mut outputs)?;
                Ok((false, None))
            }
        })();

        let backtrack = ctrl.backtrack_to_spec(bank.signature());
        let cleanup = backend.clear(true);
        let (solved, completed) = process_result?;
        cleanup?;

        Ok(BatchProcessProblemReport {
            solved,
            spawned: spawn_count,
            completed,
            backtrack,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "The staged BatchProcessProblem port keeps C runner and clock seams injectable"
    )]
    pub fn process_problem_with<WGlobal, WExternal, C, S, P>(
        &self,
        signature: &Signature,
        ctrl: &mut StructFofSpec,
        problem: BatchProblemData,
        config: BatchProcessProblemConfig<'_>,
        mut outputs: BatchProcessProblemOutputs<'_, WGlobal, WExternal>,
        mut clock_seconds: C,
        mut spawn_runner: S,
        mut poll_runners: P,
    ) -> Result<BatchProcessProblemReport, Diagnostic>
    where
        WGlobal: Write + ?Sized,
        WExternal: Write + ?Sized,
        C: FnMut() -> i64,
        S: FnMut(BatchRunnerRequest) -> Result<BatchSpawnedRunner, Diagnostic>,
        P: FnMut(&mut Vec<BatchSpawnedRunner>) -> Result<Option<BatchCompletedRunner>, Diagnostic>,
    {
        let filters = crate::heuristics::axfilter::AxFilterSet::default_set()?;
        let _pre_add_start = clock_seconds();
        ctrl.add_problem_with_type(
            signature,
            problem.clauses,
            problem.formulas,
            false,
            problem.problem_type,
        );

        let mut active = Vec::new();
        let mut spawn_count = 0;
        let process_result = (|| {
            let start = clock_seconds();
            let end = start + config.wct_limit;
            let mut filter_index = 0;
            let mut completed = None;

            while completed.is_none() && clock_seconds() <= end {
                while filter_index < BATCH_FILTERS.len() && active.len() < MAX_CORES {
                    let now = clock_seconds();
                    if now > end {
                        break;
                    }
                    let used = now - start;
                    let filter_name = BATCH_FILTERS[filter_index];
                    let filter = filters.find_filter(filter_name).ok_or_else(|| {
                        batch_process_error(format!(
                            "Batch filter '{filter_name}' is missing from the default filter set"
                        ))
                    })?;
                    let request = self.create_runner_request_with(
                        ctrl,
                        signature,
                        filter,
                        BatchRunnerCreateConfig {
                            options: BATCH_STRATEGIES[filter_index],
                            extra_options: self.answer_options(),
                            cpu_time: batch_runner_cpu_time(config.wct_limit, used),
                        },
                        &mut *outputs.global_output,
                        || clock_seconds() % 1000,
                    )?;
                    active.push(spawn_runner(request)?);
                    spawn_count += 1;
                    filter_index += 1;
                }

                if let Some(done) = poll_runners(&mut active)? {
                    completed = Some(done);
                    break;
                }
            }

            if let Some(completed_runner) = completed {
                write_problem_success(config, &completed_runner, &mut outputs, clock_seconds())?;
                Ok((true, Some(completed_runner)))
            } else {
                write_problem_gave_up(config, &mut outputs)?;
                Ok((false, None))
            }
        })();

        let backtrack = ctrl.backtrack_to_spec(signature);
        let (solved, completed) = process_result?;

        Ok(BatchProcessProblemReport {
            solved,
            spawned: spawn_count,
            completed,
            backtrack,
        })
    }

    fn problem_wct_limit(
        &self,
        total_wtc_limit: i64,
        start: i64,
        now: i64,
        remaining_problems: usize,
    ) -> i64 {
        if total_wtc_limit != 0 {
            let used = now - start;
            let rest = total_wtc_limit - used;
            let prop_time = rest / usize_to_i64_c(remaining_problems) + 1;
            if self.per_prob_limit != 0 {
                prop_time.min(self.per_prob_limit)
            } else {
                prop_time
            }
        } else {
            self.per_prob_limit
        }
    }

    fn write_output_line<W: Write + ?Sized>(
        &self,
        output: &mut W,
        state: BatchOutputType,
    ) -> Result<(), Diagnostic> {
        if self.res_assurance == state {
            write!(output, " Assurance").map_err(|error| output_error(&error))?;
        }
        if self.res_proof == state {
            write!(output, " Proof").map_err(|error| output_error(&error))?;
        }
        if self.res_model == state {
            write!(output, " Model").map_err(|error| output_error(&error))?;
        }
        if self.res_answer == state {
            write!(output, " Answer").map_err(|error| output_error(&error))?;
        }
        if self.res_list_fof == state {
            write!(output, " ListOfFOF").map_err(|error| output_error(&error))?;
        }
        Ok(())
    }
}

fn spawned_runner_from_control(control: &EPCtrl) -> BatchSpawnedRunner {
    BatchSpawnedRunner {
        name: control.name().to_owned(),
        start_time: control.start_time(),
        prob_time: control.prob_time(),
    }
}

fn render_batch_runner_problem_tstp(
    bank: &mut TermBank,
    selection: &StructFofSpecSelection<'_>,
    config: BatchRunnerProblemConfig,
) -> Result<String, Diagnostic> {
    let mut type_decls = Vec::new();
    bank.signature()
        .print_type_decls_tstp(&mut type_decls, config.problem_type)
        .map_err(|error| batch_runner_problem_output_error(&error))?;
    let mut output = String::from_utf8(type_decls).map_err(|error| {
        batch_process_error(format!(
            "Batch runner type declarations are not valid UTF-8: {error}"
        ))
    })?;
    pstack_clause_write_tstp(&mut output, bank, &selection.clauses, config.problem_type)?;
    pstack_formula_write_tstp(
        &mut output,
        bank,
        &selection.formulas,
        config.problem_type,
        config.keep_input_names,
    )?;
    Ok(output)
}

fn write_variant_initial<W: Write + ?Sized>(
    output: &mut W,
    problem_count: i64,
    variant_count: i64,
    concrete_count: i64,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "% Initial: {problem_count} abstract problems, {variant_count} variants, {concrete_count} concrete problems"
    )
    .map_err(|error| batch_process_output_error(&error))
}

fn write_variant_round<W: Write + ?Sized>(
    output: &mut W,
    variant_index: usize,
    variant: &str,
    remaining: i64,
    unsolved_count: i64,
    remaining_variant_count: i64,
    concrete_prob_count: i64,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "% Round {variant_index}, working on variant {variant}, remaining time {remaining}s"
    )
    .map_err(|error| batch_process_output_error(&error))?;
    writeln!(
        output,
        "% {unsolved_count} unsolved abstract problems, {remaining_variant_count} remaining variants, {concrete_prob_count} concrete problems"
    )
    .map_err(|error| batch_process_output_error(&error))
}

fn write_variant_already_solved<W: Write + ?Sized>(
    output: &mut W,
    abstract_source: &str,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "% Abstract problem {abstract_source} already solved"
    )
    .map_err(|error| batch_process_output_error(&error))
}

fn write_variant_started<W: Write + ?Sized>(
    output: &mut W,
    abstract_source: &str,
    concrete_source: &str,
    dest: &str,
    per_prob_time: i64,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "% Trying abstract problem {abstract_source} via {concrete_source} for {per_prob_time}s"
    )
    .map_err(|error| batch_process_output_error(&error))?;
    writeln!(output, "\n% Processing {concrete_source} -> {dest}")
        .map_err(|error| batch_process_output_error(&error))?;
    writeln!(output, "% SZS status Started for {concrete_source}")
        .map_err(|error| batch_process_output_error(&error))?;
    output
        .flush()
        .map_err(|error| batch_process_output_error(&error))
}

fn write_variant_ended<W: Write + ?Sized>(
    output: &mut W,
    concrete_source: &str,
) -> Result<(), Diagnostic> {
    writeln!(output, "% SZS status Ended for {concrete_source}\n")
        .map_err(|error| batch_process_output_error(&error))?;
    output
        .flush()
        .map_err(|error| batch_process_output_error(&error))
}

fn run_variant_child_process<S>(
    job: &BatchVariantProblemJob,
    spawn_child: &mut S,
) -> Result<BatchVariantProblemOutcome, Diagnostic>
where
    S: FnMut(&BatchVariantProblemJob, &mut dyn Write) -> Result<EGPCtrl, Diagnostic>,
{
    let mut output = Vec::new();
    let mut child = spawn_child(job, &mut output)?;
    let mut buffer = Vec::new();
    while !child.read_result_chunk(&mut buffer, &mut output)? {}
    let solved = variant_child_result_is_success(child.result());
    output.extend_from_slice(child.output().view_bytes());
    child.cleanup()?;
    Ok(BatchVariantProblemOutcome {
        solved,
        output: String::from_utf8_lossy(&output).into_owned(),
    })
}

const fn variant_child_result_is_success(result: ProverResult) -> bool {
    matches!(result, ProverResult::Theorem | ProverResult::Unsatisfiable)
}

fn write_problem_success<WGlobal, WExternal>(
    config: BatchProcessProblemConfig<'_>,
    completed: &BatchCompletedRunner,
    outputs: &mut BatchProcessProblemOutputs<'_, WGlobal, WExternal>,
    now: i64,
) -> Result<(), Diagnostic>
where
    WGlobal: Write + ?Sized,
    WExternal: Write + ?Sized,
{
    let result = prover_result_table_entry(completed.result).ok_or_else(|| {
        batch_process_error("Completed batch runner has no printable prover result")
    })?;
    writeln!(outputs.global_output, "{result} for {}", config.jobname)
        .map_err(|error| batch_process_output_error(&error))?;
    let used = now - completed.runner.start_time;
    let remaining = completed.runner.prob_time - used;
    writeln!(
        outputs.global_output,
        "% Solution found by {} (started {}, remaining {})",
        completed.runner.name, completed.runner.start_time, remaining
    )
    .map_err(|error| batch_process_output_error(&error))?;

    if let Some(socket_output) = outputs.socket_output.as_deref_mut() {
        write!(socket_output, "{}", completed.output)
            .map_err(|error| batch_process_output_error(&error))?;
        socket_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
    } else if let Some(external_output) = outputs.external_output.as_deref_mut() {
        writeln!(external_output, "{result} for {}", config.jobname)
            .map_err(|error| batch_process_output_error(&error))?;
        write!(external_output, "{}", completed.output)
            .map_err(|error| batch_process_output_error(&error))?;
        external_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
    }

    if config.interactive {
        write!(outputs.global_output, "{}", completed.output)
            .map_err(|error| batch_process_output_error(&error))?;
    }
    Ok(())
}

fn write_problem_gave_up<WGlobal, WExternal>(
    config: BatchProcessProblemConfig<'_>,
    outputs: &mut BatchProcessProblemOutputs<'_, WGlobal, WExternal>,
) -> Result<(), Diagnostic>
where
    WGlobal: Write + ?Sized,
    WExternal: Write + ?Sized,
{
    writeln!(
        outputs.global_output,
        "% SZS status GaveUp for {}",
        config.jobname
    )
    .map_err(|error| batch_process_output_error(&error))?;
    let external_status = format!("% SZS status GaveUp for {}\n", config.jobname);
    if let Some(socket_output) = outputs.socket_output.as_deref_mut() {
        write!(socket_output, "{external_status}")
            .map_err(|error| batch_process_output_error(&error))?;
        socket_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
    } else if let Some(external_output) = outputs.external_output.as_deref_mut() {
        write!(external_output, "{external_status}")
            .map_err(|error| batch_process_output_error(&error))?;
        external_output
            .flush()
            .map_err(|error| batch_process_output_error(&error))?;
    }
    Ok(())
}

#[must_use]
pub fn batch_problem_dest_name(dest_dir: Option<&str>, dest_file: &str) -> String {
    let Some(dest_dir) = dest_dir else {
        return dest_file.to_owned();
    };
    let mut result = String::with_capacity(dest_dir.len() + 1 + dest_file.len());
    result.push_str(dest_dir);
    result.push('/');
    result.push_str(dest_file);
    result
}

pub fn parse_ltb_header(scanner: &mut Scanner) -> Result<BatchSpecHeader, Diagnostic> {
    accept_dotted_id(scanner, "division.category")?;
    let category = parse_dotted_id(scanner)?;
    let train_dir = if scanner.test_id("division") {
        accept_dotted_id(scanner, "division.category.training_data")?;
        Some(parse_continuous(scanner)?)
    } else {
        None
    };

    Ok(BatchSpecHeader {
        category,
        train_dir,
    })
}

#[must_use]
pub fn abstract_to_concrete(name: &str, variant: &str, postfix: &str) -> String {
    let prefix = name.split_once('*').map_or(name, |(prefix, _)| prefix);
    let mut result = String::with_capacity(prefix.len() + variant.len() + postfix.len());
    result.push_str(prefix);
    result.push_str(variant);
    result.push_str(postfix);
    result
}

fn open_batch_problem_scanner(request: BatchProblemLoadRequest<'_>) -> Result<Scanner, Diagnostic> {
    let mut scanner =
        Scanner::from_file_with_default_dir(Path::new(request.source), true, request.default_dir)?;
    scanner.set_format(request.format);
    Ok(scanner)
}

fn open_batch_dest_file(dest: &str) -> Result<File, Diagnostic> {
    let path = Path::new(dest);
    File::create(path).map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Cannot open file {}: {error}", path.display()),
        )
    })
}

fn load_include_problem_from_file<W: Write + ?Sized>(
    bank: &mut TermBank,
    ctrl: &StructFofSpec,
    request: BatchProblemLoadRequest<'_>,
    output: &mut W,
) -> Result<Option<BatchProblemData>, Diagnostic> {
    let mut scanner = match open_batch_problem_scanner(request) {
        Ok(scanner) => scanner,
        Err(error) if error.code() == ErrorCode::FILE_ERROR => {
            writeln!(output, "% Could not find {}", request.source)
                .map_err(|error| batch_process_output_error(&error))?;
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    writeln!(output, "% Parsing {}", request.source)
        .map_err(|error| batch_process_output_error(&error))?;
    parse_batch_problem_entries_root(&mut scanner, bank, ctrl).map(Some)
}

fn parse_batch_problem_entries_root(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    ctrl: &StructFofSpec,
) -> Result<BatchProblemData, Diagnostic> {
    reset_problem_type();
    let mut problem = parse_batch_problem_entries(scanner, bank, ctrl, None)?;
    problem.problem_type = normalize_batch_problem_type(problem.problem_type);
    Ok(problem)
}

fn parse_batch_problem_entries(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    ctrl: &StructFofSpec,
    mut selectors: Option<&mut StrTree<i64, i64>>,
) -> Result<BatchProblemData, Diagnostic> {
    let mut clauses = ClauseSet::new();
    let mut formulas = FormulaSet::new();
    let mut problem_type = ProblemType::NotInitialized;

    while !scanner.test_tok(TokenType::NO_TOKEN) {
        if scanner.test_id("cnf") {
            set_problem_type(ProblemType::FirstOrder)?;
            problem_type = combine_batch_problem_types(problem_type, ProblemType::FirstOrder)?;
            let clause = clause_parse(scanner, bank, ProblemType::FirstOrder)?;
            if batch_entry_selected(
                clause.info().and_then(ClauseInfo::name),
                selectors.as_deref_mut(),
            ) {
                insert_batch_clause(bank, &mut clauses, &mut formulas, clause)?;
            }
        } else if scanner.test_id("fof|tff|tcf|thf") {
            let parsed = parse_batch_tstp_formula(scanner, bank)?;
            problem_type = combine_batch_problem_types(problem_type, parsed.problem_type)?;
            if batch_entry_selected(Some(parsed.name.as_str()), selectors.as_deref_mut()) {
                insert_batch_formula(bank, &mut clauses, &mut formulas, parsed.formula)?;
            }
        } else if scanner.test_id("include") {
            let mut include_selectors = StrTree::new();
            let skip_includes = parsed_include_skip_tree(ctrl);
            if let Some(mut included) =
                scanner.parse_include(&mut include_selectors, &skip_includes)?
            {
                let mut included_data = parse_batch_problem_entries(
                    &mut included,
                    bank,
                    ctrl,
                    Some(&mut include_selectors),
                )?;
                problem_type =
                    combine_batch_problem_types(problem_type, included_data.problem_type)?;
                clauses.insert_set(&mut included_data.clauses);
                formulas.insert_set(&mut included_data.formulas);
            }
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): LTB batch input currently supports cnf clauses, fof/tff/tcf/thf formula entries, and include directives",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    }

    if let Some(selector_tree) = selectors.as_ref() {
        check_batch_include_selectors_found(scanner, selector_tree)?;
    }

    Ok(BatchProblemData {
        clauses,
        formulas,
        problem_type,
    })
}

fn batch_tstp_wrapper_problem_type(kind: &str) -> ProblemType {
    if kind == "thf" {
        ProblemType::HigherOrder
    } else {
        ProblemType::FirstOrder
    }
}

fn combine_batch_problem_types(
    current: ProblemType,
    next: ProblemType,
) -> Result<ProblemType, Diagnostic> {
    match (current, next) {
        (ProblemType::NotInitialized, next) => Ok(next),
        (current, ProblemType::NotInitialized) => Ok(current),
        (current, next) if current == next => Ok(current),
        _ => Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Mixing of first order and higher order syntax is not allowed.",
        )),
    }
}

fn normalize_batch_problem_type(problem_type: ProblemType) -> ProblemType {
    if problem_type == ProblemType::NotInitialized {
        ProblemType::FirstOrder
    } else {
        problem_type
    }
}

fn insert_batch_clause(
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    formulas: &mut FormulaSet,
    clause: Clause,
) -> Result<(), Diagnostic> {
    if clause.query_tptp_type() == CP_TYPE_WATCH_CLAUSE {
        clauses.insert(clause);
    } else {
        formulas.insert(WrappedFormula::form_clause_alloc(
            bank,
            clause,
            ProblemType::FirstOrder,
        )?);
    }
    Ok(())
}

fn insert_batch_formula(
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    formulas: &mut FormulaSet,
    formula: WrappedFormula,
) -> Result<(), Diagnostic> {
    if formula.query_tptp_type() == CP_TYPE_WATCH_CLAUSE {
        if !formula.is_clause() {
            return Err(batch_process_error(
                "LTB watchlist formula is not clause-backed like C FormulaAndClauseSetParse expects",
            ));
        }
        clauses.insert(formula.form_clause_to_clause(bank)?);
    } else {
        formulas.insert(formula);
    }
    Ok(())
}

struct ParsedBatchFormula {
    name: String,
    formula: WrappedFormula,
    problem_type: ProblemType,
}

fn parse_batch_tstp_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<ParsedBatchFormula, Diagnostic> {
    bank.vars().clear_ext_names();
    let start_source = String::from_utf8_lossy(scanner.current_token().source_bytes()).into_owned();
    let start_line = usize_to_i64_c(scanner.current_token().line());
    let start_column = usize_to_i64_c(scanner.current_token().column());
    let formula_kind = scanner.current_token().literal();
    let problem_type = batch_tstp_wrapper_problem_type(&formula_kind);
    let is_tcf = formula_kind == "tcf";

    set_problem_type(problem_type)?;
    scanner.accept_id("fof|tff|tcf|thf")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
    scanner.accept_tok(TokenType::COMMA)?;

    let (formula, type_) = if scanner.test_id("type") {
        scanner.accept_id("type")?;
        scanner.accept_tok(TokenType::COMMA)?;
        bank.signature_mut()
            .parse_tff_type_declaration(scanner, problem_type)?;
        (
            bank.true_term().clone(),
            clause_type_from_identifier("axiom", problem_type),
        )
    } else {
        let roles = if is_tcf {
            "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question|watchlist"
        } else {
            "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question"
        };
        scanner.check_id(roles)?;
        let role = scanner.current_token().literal();
        scanner.accept_tok(TokenType::IDENT)?;
        scanner.accept_tok(TokenType::COMMA)?;
        let type_ = clause_type_from_identifier(&role, problem_type);
        let formula_position = token_pos_rep(scanner.current_token());
        let formula = if scanner.test_id("$distinct") {
            bank.parse_tstp_distinct(scanner)?
        } else if is_tcf {
            tcf_tstp_parse(scanner, bank, problem_type)?
        } else {
            bank.parse_tformula_tstp(scanner)?
        };
        if type_ != CP_TYPE_WATCH_CLAUSE && tformula_has_free_vars(bank, &formula).is_some() {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{formula_position} Formula has free variables (check parentheses and quantifier precedence)"
                ),
            ));
        }
        (formula, type_)
    };

    parse_batch_tstp_optional_source(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;

    let mut formula = if type_ == CP_TYPE_WATCH_CLAUSE && is_tcf {
        batch_tcf_watchlist_formula(bank, &formula, problem_type)?
    } else {
        WrappedFormula::wt_formula_alloc(formula)
    };
    formula.set_tptp_type(type_);
    formula.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
    formula.set_info(Some(ClauseInfo::new(
        Some(name.as_str()),
        Some(start_source.as_str()),
        start_line,
        start_column,
    )));
    Ok(ParsedBatchFormula {
        name,
        formula,
        problem_type,
    })
}

fn batch_tcf_watchlist_formula(
    bank: &mut TermBank,
    formula: &Term,
    problem_type: ProblemType,
) -> Result<WrappedFormula, Diagnostic> {
    let qall_code = bank.signature().qall_code();
    let mut body = formula.clone();
    while body.f_code() == qall_code && body.arity() == 2 {
        body = body
            .argument(1)
            .expect("TCF universal formula must have a body argument");
    }
    let clause = tformula_collect_clause(bank, &body, None)?;
    WrappedFormula::form_clause_alloc(bank, clause, problem_type)
}

fn parse_batch_tstp_optional_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        batch_tstp_skip_source(scanner)?;
        if scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            scanner.check_tok(TokenType::OPEN_SQUARE)?;
            parse_skip_parenthesized_expr(scanner)?;
        }
    }
    Ok(())
}

fn batch_tstp_skip_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::OPEN_SQUARE) {
        parse_skip_parenthesized_expr(scanner)
    } else {
        scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            parse_skip_parenthesized_expr(scanner)?;
        }
        Ok(())
    }
}

fn parsed_include_skip_tree(ctrl: &StructFofSpec) -> StrTree<i64, i64> {
    let mut skip = StrTree::new();
    for include in ctrl.parsed_includes() {
        skip.store(include, 1, 0);
    }
    skip
}

fn batch_entry_selected(name: Option<&str>, selectors: Option<&mut StrTree<i64, i64>>) -> bool {
    let Some(selectors) = selectors else {
        return true;
    };
    if selectors.is_empty() {
        return true;
    }
    if selectors.find(EMPTY_INCLUDE_SELECTOR_SENTINEL).is_some() {
        return false;
    }
    let Some(name) = name else {
        return false;
    };
    let Some(entry) = selectors.find_mut(name) else {
        return false;
    };
    entry.val1 = 1;
    true
}

fn check_batch_include_selectors_found(
    scanner: &Scanner,
    selectors: &StrTree<i64, i64>,
) -> Result<(), Diagnostic> {
    let missing = selectors
        .iter()
        .filter(|(name, entry)| *name != EMPTY_INCLUDE_SELECTOR_SENTINEL && entry.val1 == 0)
        .map(|(name, _)| name.to_owned())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let mut message = String::new();
    if let Some(include_pos) = scanner.include_pos() {
        message.push_str(include_pos);
    }
    message.push_str("\"include\" statement cannot find requested clauses/formulae: ");
    message.push_str(&missing.join(", "));
    Err(Diagnostic::new(ErrorCode::SYNTAX_ERROR, message))
}

fn parse_output_line(
    scanner: &mut Scanner,
    spec: &mut BatchSpec,
    state: BatchOutputType,
) -> Result<(), Diagnostic> {
    while scanner.test_id("Assurance|Proof|Model|Answer|ListOfFOF") {
        match scanner.current_token().literal().as_str() {
            "Assurance" => spec.res_assurance = state,
            "Proof" => spec.res_proof = state,
            "Model" => spec.res_model = state,
            "Answer" => spec.res_answer = state,
            "ListOfFOF" => spec.res_list_fof = state,
            _ => {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    format!(
                        "{} Unknown batch output field {}",
                        token_pos_rep(scanner.current_token()),
                        scanner.current_token().literal()
                    ),
                ));
            }
        }
        scanner.accept_tok(TokenType::IDENT)?;
    }
    Ok(())
}

fn output_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write batch specification output: {error}"),
    )
}

fn batch_runner_output_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write batch runner output: {error}"),
    )
}

fn batch_runner_problem_output_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write batch runner problem output: {error}"),
    )
}

fn batch_process_output_error(error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write batch process output: {error}"),
    )
}

fn batch_process_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

fn batch_filter_print_string(filter: &AxFilter) -> Result<String, Diagnostic> {
    if matches!(filter.type_, AxFilterType::NoFilter) {
        return Err(batch_process_error(
            "Cannot create a batch runner without an axiom filter",
        ));
    }
    Ok(filter.print_string())
}

fn batch_filter_runner_name(filter: &AxFilter) -> Result<String, Diagnostic> {
    filter.print_buf_string(320).ok_or_else(|| {
        batch_process_error("Batch runner filter name does not fit the C 320-byte buffer")
    })
}

fn usize_to_i64_c(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

const fn batch_runner_cpu_time(wct_limit: i64, used: i64) -> i64 {
    let half_limit = (wct_limit + 1) / 2;
    let remaining = wct_limit - used;
    if half_limit < remaining {
        half_limit
    } else {
        remaining
    }
}

#[cfg(test)]
mod tests {
    use super::{
        abstract_to_concrete, batch_problem_dest_name, parse_ltb_header, BatchCompletedRunner,
        BatchOutputType, BatchProblemData, BatchProblemLoadRequest, BatchProcCtrlRunnerSet,
        BatchProcessFileConfig, BatchProcessFileOutputs, BatchProcessProblemConfig,
        BatchProcessProblemJob, BatchProcessProblemOutputs, BatchProcessProblemsConfig,
        BatchProcessVariantsConfig, BatchRunnerBackend, BatchRunnerCreateConfig,
        BatchRunnerProblemConfig, BatchRunnerRequest, BatchRunnerTempRequest, BatchSpawnedRunner,
        BatchSpec, BatchVariantProblemOutcome, BATCH_FILTERS, BATCH_FILTERS_DIV, BATCH_STRATEGIES,
        BATCH_STRATEGIES_DIV,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
    use crate::basics::simple_stuff::{ProblemType, ProverResult};
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_INITIAL, CP_INPUT_FORMULA, CP_TYPE_WATCH_CLAUSE};
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::formulasets::FormulaSet;
    use crate::control::esession::{Descriptor, DescriptorInterestSet};
    use crate::control::gproc_ctrl::EGPCtrl;
    use crate::control::proc_ctrl::{EPCtrl, MAX_CORES};
    use crate::control::sine::StructFofSpec;
    use crate::heuristics::axfilter::AxFilter;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::inout::tempfile::{temp_file_remove, temp_file_test_lock};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::typebanks::TypeBank;
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::process::Command;
    use std::rc::Rc;

    #[test]
    fn batch_spec_defaults_match_c_allocation_shape() {
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);

        assert_eq!(spec.executable, "eprover");
        assert_eq!(spec.format, IoFormat::Tstp);
        assert_eq!(spec.category, None);
        assert_eq!(spec.train_dir, None);
        assert!(!spec.ordered);
        assert_eq!(spec.res_assurance, BatchOutputType::NoOutput);
        assert_eq!(spec.res_proof, BatchOutputType::NoOutput);
        assert_eq!(spec.res_model, BatchOutputType::NoOutput);
        assert_eq!(spec.res_answer, BatchOutputType::NoOutput);
        assert_eq!(spec.res_list_fof, BatchOutputType::NoOutput);
        assert_eq!(spec.per_prob_limit, 0);
        assert_eq!(spec.total_wtc_limit, 0);
        assert_eq!(spec.problem_no(), 0);
    }

    #[test]
    fn parse_ltb_header_preserves_training_data_input_spelling() {
        let mut scanner = Scanner::from_user_string(
            "division.category LTB.SAT\n\
             division.category.training_data /tmp/train/set-01\n\
             output.required Proof\n",
            true,
        )
        .unwrap();

        let header = parse_ltb_header(&mut scanner).unwrap();

        assert_eq!(header.category, "LTB.SAT");
        assert_eq!(header.train_dir.as_deref(), Some("/tmp/train/set-01"));
        assert!(scanner.test_id("output"));
    }

    #[test]
    fn parse_batch_spec_preserves_loose_c_control_flow() {
        let mut scanner = Scanner::from_user_string(
            "execution.order unordered\n\
             output.required Assurance Proof ListOfFOF\n\
             output.desired Model Answer\n\
             limit.time.problem.wc 17\n\
             limit.time.overall.wc 90\n\
             include('Axioms/SET001.ax').\n\
             /tmp/prob1.p /tmp/out1\n\
             Problems/TSTP/prob2.p Problems/Out/prob2.out\n\
             tail\n",
            true,
        )
        .unwrap();
        let mut notices = Vec::new();

        let spec = BatchSpec::parse_with_include_output(
            &mut scanner,
            "eprover",
            "LTB.SAT",
            Some("/train"),
            IoFormat::Tstp,
            &mut notices,
        )
        .unwrap();

        assert!(!spec.ordered);
        assert_eq!(spec.res_assurance, BatchOutputType::Required);
        assert_eq!(spec.res_proof, BatchOutputType::Required);
        assert_eq!(spec.res_model, BatchOutputType::Desired);
        assert_eq!(spec.res_answer, BatchOutputType::Desired);
        assert_eq!(spec.res_list_fof, BatchOutputType::Required);
        assert_eq!(spec.per_prob_limit, 17);
        assert_eq!(spec.total_wtc_limit, 90);
        assert_eq!(spec.includes, ["Axioms/SET001.ax"]);
        assert_eq!(spec.source_files, ["/tmp/prob1.p", "Problems/TSTP/prob2.p"]);
        assert_eq!(spec.dest_files, ["/tmp/out1", "Problems/Out/prob2.out"]);
        assert_eq!(spec.problem_no(), 2);
        assert_eq!(
            String::from_utf8(notices).unwrap(),
            "% Accepted Axioms/SET001.ax for parsing\n"
        );
        assert!(scanner.test_id("tail"));
    }

    #[test]
    fn print_batch_spec_uses_c_field_order_and_training_directory_spelling() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.category = Some("LTB.SAT".to_owned());
        spec.train_dir = Some("/train".to_owned());
        spec.ordered = true;
        spec.res_assurance = BatchOutputType::Required;
        spec.res_proof = BatchOutputType::Desired;
        spec.res_model = BatchOutputType::Required;
        spec.per_prob_limit = 11;
        spec.total_wtc_limit = 22;
        spec.includes.push("Axioms/SET001.ax".to_owned());
        spec.source_files.push("Problems/TSTP/prob.p".to_owned());
        spec.dest_files.push("Problems/Out/prob.out".to_owned());

        assert_eq!(
            spec.print_string().unwrap(),
            "% SZS start BatchConfiguration\n\
             division.category LTB.SAT\n\
             division.category.training_directory /train\n\
             execution.order ordered\n\
             output.required Assurance Model\n\
             output.desired Proof\n\
             limit.time.problem.wc 11\n\
             limit.time.overall.wc 22\n\
             % SZS end BatchConfiguration\n\
             % SZS start BatchIncludes\n\
             include('Axioms/SET001.ax').\n\
             % SZS end BatchIncludes\n\
             % SZS start BatchProblems\n\
             Problems/TSTP/prob.p Problems/Out/prob.out\n\
             % SZS end BatchProblems\n"
        );
    }

    #[test]
    fn abstract_to_concrete_ignores_text_after_star() {
        assert_eq!(
            abstract_to_concrete("Problems/*/ignored.p", "ALG001", ".p"),
            "Problems/ALG001.p"
        );
        assert_eq!(abstract_to_concrete("plain", "VAR", ".ax"), "plainVAR.ax");
    }

    #[test]
    fn batch_variant_tables_preserve_c_lengths_and_pairing() {
        assert_eq!(BATCH_FILTERS.len(), BATCH_STRATEGIES.len());
        assert_eq!(BATCH_FILTERS_DIV.len(), BATCH_STRATEGIES_DIV.len());
        assert_eq!(BATCH_FILTERS[0], "threshold010000");
        assert_eq!(
            BATCH_STRATEGIES_DIV[0],
            "--auto-schedule --assume-incompleteness"
        );
        assert_eq!(
            BATCH_STRATEGIES_DIV[13],
            "-xAutoSched2 -tAutoSched2 --assume-incompleteness"
        );
    }

    #[test]
    fn output_type_values_match_c_enum_order() {
        assert_eq!(BatchOutputType::NoOutput.c_value(), 0);
        assert_eq!(BatchOutputType::Desired.c_value(), 1);
        assert_eq!(BatchOutputType::Required.c_value(), 2);
    }

    #[test]
    fn answer_options_match_batch_process_problem_switch() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);

        assert_eq!(spec.answer_options(), "");
        spec.res_answer = BatchOutputType::Desired;
        assert_eq!(spec.answer_options(), "--conjectures-are-questions");
        spec.res_answer = BatchOutputType::Required;
        assert_eq!(spec.answer_options(), "--conjectures-are-questions");
    }

    #[test]
    fn create_runner_request_logs_filtering_and_selected_problem_counts() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        ctrl.add_problem(
            &signature,
            ClauseSet::from_clauses([Clause::empty()]),
            FormulaSet::new(),
            false,
        );
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut ticks = [11, 12, 13].into_iter();
        let mut output = Vec::new();

        let request = spec
            .create_runner_request_with(
                &mut ctrl,
                &signature,
                &AxFilter::threshold(10),
                BatchRunnerCreateConfig {
                    options: "--satauto-schedule --assume-incompleteness",
                    extra_options: "--conjectures-are-questions",
                    cpu_time: 7,
                },
                &mut output,
                || ticks.next().unwrap(),
            )
            .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Filtering for Threshold(10) (11)\n\
             % Spec has 1 clauses and 0 formulas (12)\n\
             % Written new problem (13)\n"
        );
        assert_eq!(request.executable, "eprover");
        assert_eq!(request.name, "Threshold(10)");
        assert_eq!(
            request.options,
            "--satauto-schedule --assume-incompleteness"
        );
        assert_eq!(request.extra_options, "--conjectures-are-questions");
        assert_eq!(request.cpu_time, 7);
        assert_eq!(request.selected_count, 1);
        assert_eq!(request.selected_clauses, 1);
        assert_eq!(request.selected_formulas, 0);
    }

    #[test]
    fn create_runner_request_rejects_missing_filter_without_panicking() {
        let signature = test_signature();
        let mut ctrl = StructFofSpec::new(&signature);
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut output = Vec::new();

        let error = spec
            .create_runner_request_with(
                &mut ctrl,
                &signature,
                &AxFilter::new(),
                BatchRunnerCreateConfig::default(),
                &mut output,
                || 0,
            )
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert!(output.is_empty());
    }

    #[test]
    fn create_runner_prepared_request_renders_selected_problem_tstp() {
        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        ctrl.add_problem(
            bank.signature(),
            ClauseSet::from_clauses([Clause::empty()]),
            FormulaSet::new(),
            false,
        );
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut ticks = [21, 22, 23].into_iter();
        let mut output = Vec::new();

        let prepared = spec
            .create_runner_prepared_request_with(
                &mut ctrl,
                &mut bank,
                &AxFilter::threshold(10),
                BatchRunnerCreateConfig {
                    options: "--auto",
                    extra_options: "",
                    cpu_time: 5,
                },
                BatchRunnerProblemConfig {
                    problem_type: ProblemType::FirstOrder,
                    keep_input_names: true,
                },
                &mut output,
                || ticks.next().unwrap(),
            )
            .unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Filtering for Threshold(10) (21)\n\
             % Spec has 1 clauses and 0 formulas (22)\n\
             % Written new problem (23)\n"
        );
        assert_eq!(prepared.request.name, "Threshold(10)");
        assert_eq!(prepared.request.options, "--auto");
        assert_eq!(prepared.request.cpu_time, 5);
        assert_eq!(prepared.request.selected_clauses, 1);
        assert_eq!(prepared.request.selected_formulas, 0);
        assert!(prepared.problem_tstp.contains("cnf("));
        assert!(prepared.problem_tstp.contains("$false"));
        assert!(prepared.problem_tstp.ends_with('\n'));
    }

    #[test]
    fn create_runner_temp_request_writes_selected_problem_file() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        ctrl.add_problem(
            bank.signature(),
            ClauseSet::from_clauses([Clause::empty()]),
            FormulaSet::new(),
            false,
        );
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut output = Vec::new();

        let temp = spec
            .create_runner_temp_request_with(
                &mut ctrl,
                &mut bank,
                &AxFilter::threshold(10),
                BatchRunnerCreateConfig {
                    options: "--auto",
                    extra_options: "",
                    cpu_time: 5,
                },
                BatchRunnerProblemConfig::default(),
                &mut output,
                || 30,
            )
            .unwrap();

        let payload = fs::read_to_string(&temp.input_file).unwrap();
        assert_eq!(temp.request.name, "Threshold(10)");
        assert!(payload.contains("cnf("));
        assert!(payload.contains("$false"));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Filtering for Threshold(10) (30)\n\
             % Spec has 1 clauses and 0 formulas (30)\n\
             % Written new problem (30)\n"
        );
        assert!(temp_file_remove(&temp.input_file).unwrap());
    }

    #[test]
    fn proc_ctrl_runner_set_reports_completed_proof_and_cleans_failed_runner() {
        let mut runners = BatchProcCtrlRunnerSet::new();
        let mut proof = EPCtrl::with_descriptor("proof => --auto", Descriptor::new(2));
        proof.set_start_time(30);
        proof.set_prob_time(40);
        let failure = EPCtrl::with_descriptor("failure => --auto", Descriptor::new(5));

        let spawned = runners.add_control(proof).unwrap();
        let _failure_spawned = runners.add_control(failure).unwrap();
        let mut ready = DescriptorInterestSet::default();
        ready.set_read(Descriptor::new(2));
        ready.set_read(Descriptor::new(5));
        let mut output = Vec::new();

        let completed = runners
            .poll_runners_from_ready(&ready, false, &mut output, |control, _buffer| {
                if control.name() == "proof => --auto" {
                    let _done = control
                        .get_result_from_optional_line(Some("% SZS status Theorem for job.p\n"));
                }
                Ok(control.get_result_from_optional_line(None))
            })
            .unwrap()
            .unwrap();

        assert_eq!(spawned.name, "proof => --auto");
        assert_eq!(spawned.start_time, 30);
        assert_eq!(spawned.prob_time, 40);
        assert_eq!(completed.runner, spawned);
        assert_eq!(completed.result, ProverResult::Theorem);
        assert_eq!(completed.output, "% SZS status Theorem for job.p\n");
        assert_eq!(runners.active_count(), 1);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% No proof found by failure => --auto\n"
        );
        runners.clear(false).unwrap();
        assert_eq!(runners.active_count(), 0);
    }

    #[test]
    fn proc_ctrl_runner_set_rejects_control_without_descriptor() {
        let mut runners = BatchProcCtrlRunnerSet::new();
        let mut control = EPCtrl::new("missing descriptor");
        control.set_start_time(7);
        control.set_prob_time(11);

        let error = runners.add_control(control).unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert_eq!(runners.active_count(), 0);
    }

    #[test]
    fn process_problem_with_runner_backend_writes_temp_files_and_reports_success() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.res_answer = BatchOutputType::Desired;
        let mut global = Vec::new();
        let mut external = Vec::new();
        let mut backend = FakeRunnerBackend::new(Some(BatchCompletedRunner {
            runner: BatchSpawnedRunner {
                name: "Threshold(10000) => --satauto-schedule --assume-incompleteness".to_owned(),
                start_time: 100,
                prob_time: 10,
            },
            result: ProverResult::Theorem,
            output: "% backend proof\n".to_owned(),
        }));

        let report = spec
            .process_problem_with_runner_backend(
                &mut bank,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: 20,
                    jobname: "job.p",
                    interactive: true,
                },
                BatchRunnerProblemConfig::default(),
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: None,
                },
                || 100,
                &mut backend,
            )
            .unwrap();

        assert!(report.solved);
        assert_eq!(report.spawned, MAX_CORES);
        assert_eq!(report.backtrack.removed_clause_sets, 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(backend.requests.len(), MAX_CORES);
        assert_eq!(backend.requests[0].name, "Threshold(10000)");
        assert_eq!(
            backend.requests[0].extra_options,
            "--conjectures-are-questions"
        );
        assert_eq!(backend.requests[0].cpu_time, 10);
        assert_eq!(backend.payloads.len(), MAX_CORES);
        assert!(backend.payloads[0].contains("cnf("));
        assert!(backend.payloads[0].contains("$false"));
        assert_eq!(backend.polls, 1);
        assert_eq!(backend.active, 0);

        let global = String::from_utf8(global).unwrap();
        assert!(global.contains("% Filtering for Threshold(10000) (100)\n"));
        assert!(global.contains("% SZS status Theorem for job.p\n"));
        assert!(global.ends_with("% backend proof\n"));
        assert_eq!(
            String::from_utf8(external).unwrap(),
            "% SZS status Theorem for job.p\n% backend proof\n"
        );
    }

    #[test]
    fn process_problem_with_runner_backend_mirrors_success_to_socket_only() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut external = Vec::new();
        let mut socket = Vec::new();
        let mut backend = FakeRunnerBackend::new(Some(theorem_completion("% socket proof\n")));

        let report = spec
            .process_problem_with_runner_backend(
                &mut bank,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: 20,
                    jobname: "socket.p",
                    interactive: false,
                },
                BatchRunnerProblemConfig::default(),
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: Some(&mut socket),
                },
                || 100,
                &mut backend,
            )
            .unwrap();

        assert!(report.solved);
        let global = String::from_utf8(global).unwrap();
        assert!(global.contains("% SZS status Theorem for socket.p\n"));
        assert!(global.contains("% Solution found by "));
        assert_eq!(String::from_utf8(external).unwrap(), "");
        assert_eq!(String::from_utf8(socket).unwrap(), "% socket proof\n");
    }

    #[test]
    fn process_problem_with_runner_backend_reports_gave_up_after_expired_limit() {
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut external = Vec::new();
        let mut backend = FakeRunnerBackend::new(None);

        let report = spec
            .process_problem_with_runner_backend(
                &mut bank,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: -1,
                    jobname: "late.p",
                    interactive: false,
                },
                BatchRunnerProblemConfig::default(),
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: None,
                },
                || 50,
                &mut backend,
            )
            .unwrap();

        assert!(!report.solved);
        assert_eq!(report.spawned, 0);
        assert_eq!(backend.requests.len(), 0);
        assert_eq!(report.backtrack.removed_clause_sets, 1);
        assert_eq!(
            String::from_utf8(global).unwrap(),
            "% SZS status GaveUp for late.p\n"
        );
        assert_eq!(
            String::from_utf8(external).unwrap(),
            "% SZS status GaveUp for late.p\n"
        );
    }

    #[test]
    fn process_problem_with_runner_backend_mirrors_gave_up_to_socket_only() {
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut external = Vec::new();
        let mut socket = Vec::new();
        let mut backend = FakeRunnerBackend::new(None);

        let report = spec
            .process_problem_with_runner_backend(
                &mut bank,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: -1,
                    jobname: "socket-late.p",
                    interactive: false,
                },
                BatchRunnerProblemConfig::default(),
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: Some(&mut socket),
                },
                || 50,
                &mut backend,
            )
            .unwrap();

        assert!(!report.solved);
        assert_eq!(
            String::from_utf8(global).unwrap(),
            "% SZS status GaveUp for socket-late.p\n"
        );
        assert_eq!(String::from_utf8(external).unwrap(), "");
        assert_eq!(
            String::from_utf8(socket).unwrap(),
            "% SZS status GaveUp for socket-late.p\n"
        );
    }

    #[test]
    fn process_problem_spawns_up_to_max_cores_then_reports_success_and_backtracks() {
        let signature = test_signature();
        let mut ctrl = shared_spec(&signature);
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.res_answer = BatchOutputType::Desired;
        let mut global = Vec::new();
        let mut external = Vec::new();
        let mut requests = Vec::new();

        let report = spec
            .process_problem_with(
                &signature,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: 20,
                    jobname: "job.p",
                    interactive: true,
                },
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: None,
                },
                || 100,
                |request| {
                    requests.push(request.clone());
                    Ok(BatchSpawnedRunner {
                        name: request.name,
                        start_time: 100,
                        prob_time: request.cpu_time,
                    })
                },
                |active| {
                    assert_eq!(active.len(), MAX_CORES);
                    Ok(Some(BatchCompletedRunner {
                        runner: active[1].clone(),
                        result: ProverResult::Theorem,
                        output: "% proof object\n".to_owned(),
                    }))
                },
            )
            .unwrap();

        assert!(report.solved);
        assert_eq!(report.spawned, MAX_CORES);
        assert_eq!(report.backtrack.removed_clause_sets, 1);
        assert_eq!(report.backtrack.removed_formula_sets, 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(requests.len(), MAX_CORES);
        assert_eq!(requests[0].name, "Threshold(10000)");
        assert_eq!(
            requests[0].options,
            "--satauto-schedule --assume-incompleteness"
        );
        assert_eq!(requests[0].extra_options, "--conjectures-are-questions");
        assert_eq!(requests[0].cpu_time, 10);

        let global = String::from_utf8(global).unwrap();
        assert!(global.contains("% Filtering for Threshold(10000) (100)\n"));
        assert!(global.contains("% SZS status Theorem for job.p\n"));
        assert!(global.contains("% Solution found by "));
        assert!(global.ends_with("% proof object\n"));
        assert_eq!(
            String::from_utf8(external).unwrap(),
            "% SZS status Theorem for job.p\n% proof object\n"
        );
    }

    #[test]
    fn process_problem_reports_gave_up_when_time_expires_before_spawn() {
        let signature = test_signature();
        let mut ctrl = shared_spec(&signature);
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut external = Vec::new();

        let report = spec
            .process_problem_with(
                &signature,
                &mut ctrl,
                one_empty_clause_problem(),
                BatchProcessProblemConfig {
                    wct_limit: -1,
                    jobname: "late.p",
                    interactive: false,
                },
                BatchProcessProblemOutputs {
                    global_output: &mut global,
                    external_output: Some(&mut external),
                    socket_output: None,
                },
                || 50,
                |_| panic!("no runner should be spawned after an expired time limit"),
                |_| panic!("no runner set should be polled after an expired time limit"),
            )
            .unwrap();

        assert!(!report.solved);
        assert_eq!(report.spawned, 0);
        assert_eq!(report.completed, None);
        assert_eq!(report.backtrack.removed_clause_sets, 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(
            String::from_utf8(global).unwrap(),
            "% SZS status GaveUp for late.p\n"
        );
        assert_eq!(
            String::from_utf8(external).unwrap(),
            "% SZS status GaveUp for late.p\n"
        );
    }

    #[test]
    fn process_file_loads_tstp_problem_and_writes_destination_output() {
        let signature = test_signature();
        let mut ctrl = shared_spec(&signature);
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut dest = Vec::new();
        let mut load_requests = Vec::new();
        let mut runner_requests = Vec::new();
        let report = {
            let mut outputs = BatchProcessFileOutputs {
                global_output: &mut global,
                dest_output: &mut dest,
            };
            spec.process_file_with(
                &signature,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: Some("Problems"),
                    source: "SET001+1.p",
                    dest: "out/SET001+1.p",
                },
                &mut outputs,
                |request| {
                    load_requests.push((
                        request.source.to_owned(),
                        request.default_dir.map(str::to_owned),
                        request.format,
                    ));
                    Ok(one_empty_clause_problem())
                },
                || 200,
                |request| {
                    runner_requests.push(request.clone());
                    Ok(BatchSpawnedRunner {
                        name: request.name,
                        start_time: 200,
                        prob_time: request.cpu_time,
                    })
                },
                |active| {
                    Ok(Some(BatchCompletedRunner {
                        runner: active[0].clone(),
                        result: ProverResult::Theorem,
                        output: "% destination proof\n".to_owned(),
                    }))
                },
            )
            .unwrap()
        };

        assert!(report.solved);
        assert_eq!(report.source, "SET001+1.p");
        assert_eq!(report.dest, "out/SET001+1.p");
        assert!(report.problem.solved);
        assert_eq!(report.problem.backtrack.removed_clause_sets, 1);
        assert_eq!(load_requests.len(), 1);
        assert_eq!(load_requests[0].0, "SET001+1.p");
        assert_eq!(load_requests[0].1.as_deref(), Some("Problems"));
        assert_eq!(load_requests[0].2, IoFormat::Tstp);
        assert_eq!(runner_requests.len(), MAX_CORES);
        assert_eq!(runner_requests[0].cpu_time, 6);
        assert!(runner_requests[0].extra_options.is_empty());

        let global = String::from_utf8(global).unwrap();
        assert!(global.contains("% SZS status Theorem for SET001+1.p\n"));
        assert!(!global.contains("% destination proof\n"));
        assert_eq!(
            String::from_utf8(dest).unwrap(),
            "% SZS status Theorem for SET001+1.p\n% destination proof\n"
        );
    }

    #[test]
    fn process_file_flushes_global_output_before_injected_load() {
        let signature = test_signature();
        let mut ctrl = shared_spec(&signature);
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let flushes = Rc::new(Cell::new(0));
        let mut global = FlushCountingWriter::new(Rc::clone(&flushes));
        let mut dest = Vec::new();
        let report = {
            let mut outputs = BatchProcessFileOutputs {
                global_output: &mut global,
                dest_output: &mut dest,
            };
            spec.process_file_with(
                &signature,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: "flush-before-load.p",
                    dest: "flush-before-load.out",
                },
                &mut outputs,
                |_| {
                    assert_eq!(flushes.get(), 1);
                    Ok(one_empty_clause_problem())
                },
                || 200,
                |request| {
                    Ok(BatchSpawnedRunner {
                        name: request.name,
                        start_time: 200,
                        prob_time: request.cpu_time,
                    })
                },
                |active| {
                    Ok(Some(BatchCompletedRunner {
                        runner: active[0].clone(),
                        result: ProverResult::Theorem,
                        output: "% flushed proof\n".to_owned(),
                    }))
                },
            )
            .unwrap()
        };

        assert!(report.solved);
        assert_eq!(flushes.get(), 1);
        assert!(String::from_utf8(global.writes)
            .unwrap()
            .contains("% SZS status Theorem for flush-before-load.p\n"));
    }

    #[test]
    fn process_file_propagates_load_error_without_mutating_shared_spec() {
        let signature = test_signature();
        let mut ctrl = shared_spec(&signature);
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut dest = Vec::new();
        let error = {
            let mut outputs = BatchProcessFileOutputs {
                global_output: &mut global,
                dest_output: &mut dest,
            };
            spec.process_file_with(
                &signature,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: "bad.p",
                    dest: "bad.out",
                },
                &mut outputs,
                |_| Err(super::batch_process_error("loader failed")),
                || 200,
                |_| panic!("no runner should be spawned after a load error"),
                |_| panic!("no runner set should be polled after a load error"),
            )
            .unwrap_err()
        };

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert!(global.is_empty());
        assert!(dest.is_empty());
    }

    #[test]
    fn load_problem_from_file_keeps_c_formula_and_watchlist_split() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let source = test_path("batch-load-split.p");
        fs::write(
            &source,
            "cnf(ax_clause, axiom, p(a)).\n\
             cnf(watch_clause, watchlist, q(a)).\n\
             fof(goal_formula, conjecture, p(a)).\n",
        )
        .unwrap();
        let mut bank = test_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let source_name = source.to_string_lossy().into_owned();

        let problem = spec
            .load_problem_from_file(
                &mut bank,
                &ctrl,
                BatchProblemLoadRequest {
                    source: &source_name,
                    default_dir: None,
                    format: IoFormat::Tstp,
                },
            )
            .unwrap();

        assert_eq!(problem.problem_type, ProblemType::FirstOrder);
        assert_eq!(problem.clauses.len(), 1);
        assert_eq!(problem.formulas.cardinality(), 2);
        let formulas = problem.formulas.iter().collect::<Vec<_>>();
        assert!(formulas[0].is_clause());
        assert_eq!(
            formulas[0].info().and_then(ClauseInfo::name),
            Some("ax_clause")
        );
        assert!(!formulas[1].is_clause());
        assert_eq!(
            formulas[1].info().and_then(ClauseInfo::name),
            Some("goal_formula")
        );
    }

    #[test]
    fn load_problem_from_file_accepts_tcf_watchlist_formula_entries_as_clauses() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let source = test_path("batch-load-tcf-watchlist.p");
        fs::write(
            &source,
            "tcf(watch_free, watchlist, p(X)|q(X)).\n\
             tcf(watch_quant, watchlist, ![Y]:(r(Y)|s(Y))).\n\
             tcf(fact, axiom, p(a)).\n",
        )
        .unwrap();
        let mut bank = test_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let source_name = source.to_string_lossy().into_owned();

        let problem = spec
            .load_problem_from_file(
                &mut bank,
                &ctrl,
                BatchProblemLoadRequest {
                    source: &source_name,
                    default_dir: None,
                    format: IoFormat::Tstp,
                },
            )
            .unwrap();

        assert_eq!(problem.problem_type, ProblemType::FirstOrder);
        assert_eq!(problem.clauses.len(), 2);
        assert_eq!(problem.formulas.cardinality(), 1);
        let watch_names = problem
            .clauses
            .iter()
            .map(|clause| clause.info().and_then(ClauseInfo::name).unwrap_or(""))
            .collect::<Vec<_>>();
        assert_eq!(watch_names, ["watch_free", "watch_quant"]);
        assert!(problem
            .clauses
            .iter()
            .all(|clause| clause.query_tptp_type() == CP_TYPE_WATCH_CLAUSE));
        assert!(problem
            .clauses
            .iter()
            .all(|clause| clause.query_prop(CP_INITIAL | CP_INPUT_FORMULA)));
        assert!(problem
            .clauses
            .iter()
            .all(|clause| clause.literal_number() == 2));
    }

    #[test]
    fn load_problem_from_file_rejects_legacy_records_under_forced_tstp_like_c() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let source = test_path("batch-load-legacy-record.p");
        fs::write(&source, "input_clause(old_clause, axiom, [++p(a)]).\n").unwrap();
        let mut bank = test_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let source_name = source.to_string_lossy().into_owned();

        let error = spec
            .load_problem_from_file(
                &mut bank,
                &ctrl,
                BatchProblemLoadRequest {
                    source: &source_name,
                    default_dir: None,
                    format: IoFormat::Tstp,
                },
            )
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains(
            "LTB batch input currently supports cnf clauses, fof/tff/tcf/thf formula entries, and include directives"
        ));
    }

    #[test]
    fn load_problem_from_file_accepts_supported_thf_entries() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let source = test_path("batch-load-thf.p");
        fs::write(
            &source,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(goal_formula, conjecture, p @ a).\n",
        )
        .unwrap();
        let mut bank = test_bank();
        let ctrl = StructFofSpec::new(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let source_name = source.to_string_lossy().into_owned();

        let problem = spec
            .load_problem_from_file(
                &mut bank,
                &ctrl,
                BatchProblemLoadRequest {
                    source: &source_name,
                    default_dir: None,
                    format: IoFormat::Tstp,
                },
            )
            .unwrap();

        assert_eq!(problem.problem_type, ProblemType::HigherOrder);
        assert_eq!(problem.clauses.len(), 0);
        assert_eq!(problem.formulas.cardinality(), 4);
        assert!(problem
            .formulas
            .iter()
            .any(|formula| formula.info().and_then(ClauseInfo::name) == Some("goal_formula")));
    }

    #[test]
    fn process_file_with_runner_backend_renders_loaded_thf_problem() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let source = test_path("batch-real-thf.p");
        let dest = test_path("batch-real-thf.out");
        let _ = fs::remove_file(&dest);
        fs::write(
            &source,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(goal_formula, conjecture, p @ a).\n",
        )
        .unwrap();
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut backend = FakeRunnerBackend::new(Some(theorem_completion("% thf proof\n")));
        let source_name = source.to_string_lossy().into_owned();
        let dest_name = dest.to_string_lossy().into_owned();

        let report = spec
            .process_file_with_runner_backend(
                &mut bank,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: &source_name,
                    dest: &dest_name,
                },
                &mut global,
                || 100,
                &mut backend,
            )
            .unwrap();

        assert!(report.solved);
        assert_eq!(backend.requests.len(), MAX_CORES);
        assert!(backend.payloads[0].contains("thf("));
        assert!(backend.payloads[0].contains("thf(goal_formula"));
        assert!(!backend.payloads[0].contains("fof(goal_formula"));
        assert!(!backend.payloads[0].contains("tff(goal_formula"));
    }

    #[test]
    fn process_file_with_runner_backend_renders_higher_order_shared_problem() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let include_name = format!("batch-shared-thf-{}.ax", std::process::id());
        let source = test_path("batch-shared-thf-concrete.p");
        let dest = test_path("batch-shared-thf-concrete.out");
        let _ = fs::remove_file(&dest);
        fs::write(
            temp_dir.join(&include_name),
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(shared_formula, axiom, p @ a).\n",
        )
        .unwrap();
        fs::write(&source, "cnf(goal_clause, axiom, $false).\n").unwrap();
        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.includes = vec![include_name.clone()];
        let default_dir = format!("{}/", temp_dir.to_string_lossy().replace('\\', "/"));
        let mut init_output = Vec::new();

        let parsed = spec
            .init_struct_fof_spec_from_files(
                &mut bank,
                &mut ctrl,
                Some(&default_dir),
                &mut init_output,
            )
            .unwrap();

        assert_eq!(parsed, 4);
        assert_eq!(ctrl.problem_type(), ProblemType::HigherOrder);
        assert_eq!(ctrl.shared_ax_sp(), 1);
        let mut global = Vec::new();
        let mut backend = FakeRunnerBackend::new(Some(theorem_completion("% shared thf proof\n")));
        let source_name = source.to_string_lossy().into_owned();
        let dest_name = dest.to_string_lossy().into_owned();

        let report = spec
            .process_file_with_runner_backend(
                &mut bank,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: &source_name,
                    dest: &dest_name,
                },
                &mut global,
                || 100,
                &mut backend,
            )
            .unwrap();

        assert!(report.solved);
        assert_eq!(backend.requests.len(), MAX_CORES);
        let payload = &backend.payloads[0];
        assert!(payload.contains("thf(shared_formula"));
        assert!(payload.contains("thf(goal_clause"));
        assert!(!payload.contains("fof(shared_formula"));
        assert!(!payload.contains("tff(shared_formula"));
        assert_eq!(ctrl.problem_type(), ProblemType::HigherOrder);
    }

    #[test]
    fn init_struct_fof_spec_from_files_parses_includes_and_reports_missing() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let include = test_path("batch-include.ax");
        fs::write(&include, "fof(shared_formula, axiom, p(a)).\n").unwrap();
        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let include_name = include.file_name().unwrap().to_string_lossy().into_owned();
        spec.includes = vec![include_name.clone(), "definitely-missing.ax".to_owned()];
        let mut output = Vec::new();
        let default_dir = format!("{}/", dir.to_string_lossy().replace('\\', "/"));

        let parsed = spec
            .init_struct_fof_spec_from_files(&mut bank, &mut ctrl, Some(&default_dir), &mut output)
            .unwrap();

        assert_eq!(parsed, 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 1);
        let p_code = [
            bank.signature().find_f_code("p"),
            bank.signature().find_f_code("$p"),
        ]
        .into_iter()
        .find(|f_code| *f_code != 0)
        .expect("shared predicate was parsed");
        assert_eq!(
            ctrl.f_distrib()
                .entry(p_code)
                .expect("parsed predicate has a distribution entry")
                .term_freq(),
            1
        );
        assert!(ctrl.has_parsed_include(&include_name));
        assert!(!ctrl.has_parsed_include("definitely-missing.ax"));
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains(&format!("% Parsing {include_name}\n")));
        assert!(output.contains("% Could not find definitely-missing.ax\n"));
    }

    #[test]
    fn init_concrete_struct_fof_spec_from_files_rewrites_includes_like_c() {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let prefix = format!("batch-concrete-variant-{}-", std::process::id());
        let concrete_name = format!("{prefix}+1.ax");
        let abstract_name = format!("{prefix}*ignored.ax");
        let concrete_include = dir.join(&concrete_name);
        fs::write(&concrete_include, "fof(concrete_shared, axiom, p(a)).\n").unwrap();
        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.includes = vec![abstract_name.clone()];
        let mut output = Vec::new();
        let default_dir = format!("{}/", dir.to_string_lossy().replace('\\', "/"));

        let parsed = spec
            .init_concrete_struct_fof_spec_from_files(
                &mut bank,
                &mut ctrl,
                Some(&default_dir),
                "+1",
                &mut output,
            )
            .unwrap();

        assert_eq!(parsed, 1);
        assert_eq!(spec.includes, [abstract_name]);
        assert!(ctrl.has_parsed_include(&concrete_name));
        assert!(!ctrl.has_parsed_include(&spec.includes[0]));
        assert_eq!(ctrl.shared_ax_sp(), 1);
        assert!(String::from_utf8(output)
            .unwrap()
            .contains(&format!("% Parsing {concrete_name}\n")));
    }

    #[test]
    fn process_file_with_runner_backend_parses_file_and_writes_destination() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let source = test_path("batch-real-process.p");
        let dest = test_path("batch-real-process.out");
        let _ = fs::remove_file(&dest);
        fs::write(&source, "cnf(goal_clause, axiom, $false).\n").unwrap();
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let mut global = Vec::new();
        let mut backend = FakeRunnerBackend::new(Some(BatchCompletedRunner {
            runner: BatchSpawnedRunner {
                name: "Threshold(10000) => --satauto-schedule --assume-incompleteness".to_owned(),
                start_time: 100,
                prob_time: 6,
            },
            result: ProverResult::Theorem,
            output: "% real destination proof\n".to_owned(),
        }));
        let source_name = source.to_string_lossy().into_owned();
        let dest_name = dest.to_string_lossy().into_owned();

        let report = spec
            .process_file_with_runner_backend(
                &mut bank,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: &source_name,
                    dest: &dest_name,
                },
                &mut global,
                || 100,
                &mut backend,
            )
            .unwrap();

        assert!(report.solved);
        assert_eq!(report.source, source_name);
        assert_eq!(report.dest, dest_name);
        assert_eq!(report.problem.backtrack.removed_clause_sets, 1);
        assert_eq!(backend.requests.len(), MAX_CORES);
        assert!(backend.payloads[0].contains("goal_clause"));
        assert_eq!(
            fs::read_to_string(&dest).unwrap(),
            format!(
                "% SZS status Theorem for {}\n% real destination proof\n",
                source.to_string_lossy()
            )
        );
    }

    #[test]
    fn process_file_with_runner_backend_does_not_create_dest_after_parse_error() {
        let source = test_path("batch-real-parse-error.p");
        let dest = test_path("batch-real-parse-error.out");
        let _ = fs::remove_file(&dest);
        fs::write(&source, "not_a_tstp_entry.\n").unwrap();
        let mut bank = test_bank();
        let mut ctrl = shared_spec(bank.signature());
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let flushes = Rc::new(Cell::new(0));
        let mut global = FlushCountingWriter::new(Rc::clone(&flushes));
        let mut backend = FakeRunnerBackend::new(None);
        let source_name = source.to_string_lossy().into_owned();
        let dest_name = dest.to_string_lossy().into_owned();

        let error = spec
            .process_file_with_runner_backend(
                &mut bank,
                &mut ctrl,
                BatchProcessFileConfig {
                    wct_limit: 12,
                    default_dir: None,
                    source: &source_name,
                    dest: &dest_name,
                },
                &mut global,
                || 100,
                &mut backend,
            )
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(!dest.exists());
        assert_eq!(backend.requests.len(), 0);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(flushes.get(), 1);
        assert!(global.writes.is_empty());
    }

    #[test]
    fn process_problems_with_runner_backend_processes_concrete_files() {
        let _guard = temp_file_test_lock();
        let temp_dir = test_temp_dir();
        let _tmpdir_guard = TmpDirGuard::set(&temp_dir);
        let out_dir = temp_dir.join(format!("batch-real-out-{}", std::process::id()));
        fs::create_dir_all(&out_dir).unwrap();
        let include_name = format!("batch-shared-{}.ax", std::process::id());
        let first_name = format!("batch-concrete-one-{}.p", std::process::id());
        let second_name = format!("batch-concrete-two-{}.p", std::process::id());
        fs::write(temp_dir.join(&include_name), "fof(shared, axiom, p(a)).\n").unwrap();
        fs::write(
            temp_dir.join(&first_name),
            "cnf(goal_one, axiom, $false).\n",
        )
        .unwrap();
        fs::write(
            temp_dir.join(&second_name),
            "cnf(goal_two, axiom, $false).\n",
        )
        .unwrap();

        let mut bank = test_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.per_prob_limit = 12;
        spec.includes = vec![include_name.clone()];
        spec.source_files = vec![first_name.clone(), second_name.clone()];
        spec.dest_files = vec!["one.out".to_owned(), "two.out".to_owned()];
        let mut global = Vec::new();
        let mut init_output = Vec::new();
        let default_dir = format!("{}/", temp_dir.to_string_lossy().replace('\\', "/"));
        let out_dir_name = out_dir.to_string_lossy().replace('\\', "/");

        spec.init_struct_fof_spec_from_files(
            &mut bank,
            &mut ctrl,
            Some(&default_dir),
            &mut init_output,
        )
        .unwrap();

        let mut backend = FakeRunnerBackend::with_completions([
            theorem_completion("% proof one\n"),
            theorem_completion("% proof two\n"),
        ]);

        let report = spec
            .process_problems_with_runner_backend(
                &mut bank,
                &mut ctrl,
                BatchProcessProblemsConfig {
                    default_dir: Some(&default_dir),
                    dest_dir: Some(&out_dir_name),
                    ..BatchProcessProblemsConfig::default()
                },
                &mut global,
                || 100,
                &mut backend,
            )
            .unwrap();

        assert_eq!(report.solved, 2);
        assert_eq!(report.records.len(), 2);
        assert_eq!(report.records[0].source, first_name);
        assert_eq!(report.records[0].wct_limit, 12);
        assert_eq!(report.records[0].dest, format!("{out_dir_name}/one.out"));
        assert!(report.records[0].solved);
        assert_eq!(report.records[1].source, second_name);
        assert_eq!(report.records[1].dest, format!("{out_dir_name}/two.out"));
        assert!(report.records[1].solved);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(backend.requests.len(), MAX_CORES * 2);
        assert_eq!(backend.polls, 2);
        assert_eq!(backend.active, 0);
        assert!(backend
            .payloads
            .iter()
            .any(|payload| payload.contains("goal_one")));
        assert!(backend
            .payloads
            .iter()
            .any(|payload| payload.contains("goal_two")));
        assert_eq!(
            fs::read_to_string(out_dir.join("one.out")).unwrap(),
            format!("% SZS status Theorem for {first_name}\n% proof one\n")
        );
        assert_eq!(
            fs::read_to_string(out_dir.join("two.out")).unwrap(),
            format!("% SZS status Theorem for {second_name}\n% proof two\n")
        );
    }

    #[test]
    fn batch_problem_dest_name_preserves_c_dest_dir_joining() {
        assert_eq!(batch_problem_dest_name(None, "out.p"), "out.p");
        assert_eq!(
            batch_problem_dest_name(Some("Results"), "out.p"),
            "Results/out.p"
        );
        assert_eq!(
            batch_problem_dest_name(Some("Results/"), "out.p"),
            "Results//out.p"
        );
        assert_eq!(batch_problem_dest_name(Some(""), "out.p"), "/out.p");
    }

    #[test]
    fn process_variants_uses_c_round_time_accounting_and_skips_solved_abstracts() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.total_wtc_limit = 100;
        spec.source_files = vec!["prob_*".to_owned(), "other_*suffix".to_owned()];
        spec.dest_files = vec!["prob.out".to_owned(), "other.out".to_owned()];
        let variants = ["A", "B"];
        let provers = ["e-a", "e-b"];
        let mut output = Vec::new();
        let mut jobs = Vec::new();
        let mut times = [10, 11, 12, 13, 14].into_iter();

        let report = spec
            .process_variants_with(
                BatchProcessVariantsConfig {
                    variants: &variants,
                    provers: &provers,
                    start: 0,
                    default_dir: Some("Problems"),
                    outdir: Some("Results"),
                },
                &mut output,
                || times.next().unwrap(),
                |job| {
                    jobs.push((
                        job.variant_index,
                        job.problem_index,
                        job.variant,
                        job.prover,
                        job.abstract_source,
                        job.concrete_source,
                        job.dest,
                        job.wct_limit,
                        job.default_dir,
                    ));
                    let solved = jobs.len() == 1 || jobs.len() == 3;
                    Ok(BatchVariantProblemOutcome {
                        solved,
                        output: format!("% child output {}\n", jobs.len()),
                    })
                },
            )
            .unwrap();

        assert_eq!(report.solved, 2);
        assert_eq!(report.attempted, 3);
        assert_eq!(report.records.len(), 3);
        assert_eq!(report.records[0].wct_limit, 23);
        assert_eq!(report.records[1].wct_limit, 45);
        assert_eq!(report.records[2].wct_limit, 87);
        assert!(report.records[0].solved);
        assert!(!report.records[1].solved);
        assert!(report.records[2].solved);
        assert_eq!(jobs[0].0, 0);
        assert_eq!(jobs[0].2, "A");
        assert_eq!(jobs[0].3, "e-a");
        assert_eq!(jobs[0].4, "prob_*");
        assert_eq!(jobs[0].5, "prob_A.p");
        assert_eq!(jobs[0].6, "Results/prob.out");
        assert_eq!(jobs[0].7, 23);
        assert_eq!(jobs[0].8.as_deref(), Some("Problems"));
        assert_eq!(jobs[1].5, "other_A.p");
        assert_eq!(jobs[2].0, 1);
        assert_eq!(jobs[2].2, "B");
        assert_eq!(jobs[2].3, "e-b");
        assert_eq!(jobs[2].5, "other_B.p");

        let output = String::from_utf8(output).unwrap();
        assert!(
            output.contains("% Initial: 2 abstract problems, 2 variants, 4 concrete problems\n")
        );
        assert!(output.contains("% Round 0, working on variant A, remaining time 90s\n"));
        assert!(output.contains("% Trying abstract problem prob_* via prob_A.p for 23s\n"));
        assert!(output.contains("\n% Processing prob_A.p -> Results/prob.out\n"));
        assert!(output.contains("% child output 1\n% SZS status Ended for prob_A.p\n\n"));
        assert!(output.contains("% Abstract problem prob_* already solved\n"));
        assert!(output.contains("% Trying abstract problem other_*suffix via other_B.p for 87s\n"));
    }

    #[test]
    fn process_variants_with_child_processes_captures_generic_child_output() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.total_wtc_limit = 50;
        spec.source_files = vec!["prob_*".to_owned()];
        spec.dest_files = vec!["prob.out".to_owned()];
        let variants = ["A"];
        let provers = ["e-a"];
        let mut output = Vec::new();
        let mut times = [10, 11].into_iter();

        let report = spec
            .process_variants_with_child_processes(
                BatchProcessVariantsConfig {
                    variants: &variants,
                    provers: &provers,
                    start: 0,
                    default_dir: Some("Problems"),
                    outdir: Some("Results"),
                },
                &mut output,
                || times.next().unwrap_or(12),
                |job, startup_output| {
                    assert_eq!(job.concrete_source, "prob_A.p");
                    assert_eq!(job.prover, "e-a");
                    assert_eq!(job.dest, "Results/prob.out");
                    assert_eq!(job.wct_limit, 40);
                    EGPCtrl::spawn_command_reporting(
                        status_command("% SZS status Unsatisfiable", 0),
                        "E-LTB wrapper",
                        1,
                        1_000_000,
                        startup_output,
                    )
                },
            )
            .unwrap();

        assert_eq!(report.solved, 1);
        assert_eq!(report.attempted, 1);
        assert!(report.records[0].solved);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("% Starting E-LTB wrapper with 1000000s (1) cores\n"));
        assert!(output.contains("% E-LTB wrapper with pid "));
        assert!(output.contains("% SZS status Unsatisfiable"));
        assert!(output.contains("% SZS status Ended for prob_A.p\n\n"));
    }

    #[test]
    fn process_variants_with_child_processes_uses_c_success_status_set() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.total_wtc_limit = 20;
        spec.source_files = vec!["prob_*".to_owned()];
        spec.dest_files = vec!["prob.out".to_owned()];
        let variants = ["SAT"];
        let provers = ["e-sat"];
        let mut output = Vec::new();

        let report = spec
            .process_variants_with_child_processes(
                BatchProcessVariantsConfig {
                    variants: &variants,
                    provers: &provers,
                    start: 0,
                    default_dir: None,
                    outdir: None,
                },
                &mut output,
                || 0,
                |_job, startup_output| {
                    EGPCtrl::spawn_command_reporting(
                        status_command("% SZS status Satisfiable", 0),
                        "E-LTB wrapper",
                        1,
                        1_000_000,
                        startup_output,
                    )
                },
            )
            .unwrap();

        assert_eq!(report.solved, 0);
        assert_eq!(report.attempted, 1);
        assert!(!report.records[0].solved);
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("% SZS status Satisfiable"));
        assert!(output.contains("% SZS status Ended for prob_SAT.p\n\n"));
    }

    #[test]
    fn process_variants_rejects_mismatched_variant_prover_lists() {
        let spec = BatchSpec::new("eprover", IoFormat::Tstp);
        let variants = ["A", "B"];
        let provers = ["e-a"];
        let mut output = Vec::new();

        let error = spec
            .process_variants_with(
                BatchProcessVariantsConfig {
                    variants: &variants,
                    provers: &provers,
                    start: 0,
                    default_dir: None,
                    outdir: None,
                },
                &mut output,
                || 0,
                |_| panic!("no variant problem should be processed for mismatched inputs"),
            )
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert!(output.is_empty());
    }

    #[test]
    fn process_problems_uses_per_problem_limit_without_total_limit() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.per_prob_limit = 17;
        spec.source_files = vec!["p1.p".to_owned(), "p2.p".to_owned()];
        spec.dest_files = vec!["o1".to_owned(), "o2".to_owned()];
        let mut jobs = Vec::new();

        let report = spec
            .process_problems_with(
                BatchProcessProblemsConfig {
                    default_dir: Some("Problems"),
                    ..BatchProcessProblemsConfig::default()
                },
                || 100,
                |job| {
                    jobs.push(job_to_tuple(job));
                    Ok(job.index == 1)
                },
            )
            .unwrap();

        assert_eq!(report.c_return_value(), 1);
        assert_eq!(
            jobs,
            [
                (
                    0,
                    17,
                    Some("Problems".to_owned()),
                    "p1.p".to_owned(),
                    "o1".to_owned()
                ),
                (
                    1,
                    17,
                    Some("Problems".to_owned()),
                    "p2.p".to_owned(),
                    "o2".to_owned()
                )
            ]
        );
        assert_eq!(report.records.len(), 2);
        assert!(!report.records[0].solved);
        assert!(report.records[1].solved);
    }

    #[test]
    fn process_problems_biases_total_limit_up_and_caps_by_per_problem_limit() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.per_prob_limit = 20;
        spec.source_files = vec!["p1.p".to_owned(), "p2.p".to_owned(), "p3.p".to_owned()];
        spec.dest_files = vec!["o1".to_owned(), "o2".to_owned(), "o3".to_owned()];
        let mut times = [100, 100, 110, 160].into_iter();
        let mut limits = Vec::new();

        let report = spec
            .process_problems_with(
                BatchProcessProblemsConfig {
                    total_wtc_limit: 90,
                    dest_dir: Some("Out"),
                    ..BatchProcessProblemsConfig::default()
                },
                || times.next().unwrap(),
                |job| {
                    limits.push((job.wct_limit, job.dest.to_owned()));
                    Ok(true)
                },
            )
            .unwrap();

        assert_eq!(report.solved, 3);
        assert_eq!(
            limits,
            [
                (20, "Out/o1".to_owned()),
                (20, "Out/o2".to_owned()),
                (20, "Out/o3".to_owned())
            ]
        );
    }

    #[test]
    fn process_problems_uses_proportional_total_limit_without_per_problem_cap() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.source_files = vec!["p1.p".to_owned(), "p2.p".to_owned(), "p3.p".to_owned()];
        spec.dest_files = vec!["o1".to_owned(), "o2".to_owned(), "o3".to_owned()];
        let mut times = [100, 100, 110, 160].into_iter();
        let mut limits = Vec::new();

        let report = spec
            .process_problems_with(
                BatchProcessProblemsConfig {
                    total_wtc_limit: 90,
                    ..BatchProcessProblemsConfig::default()
                },
                || times.next().unwrap(),
                |job| {
                    limits.push(job.wct_limit);
                    Ok(job.index != 1)
                },
            )
            .unwrap();

        assert_eq!(report.solved, 2);
        assert_eq!(limits, [31, 41, 31]);
        assert_eq!(
            report
                .records
                .iter()
                .map(|record| record.wct_limit)
                .collect::<Vec<_>>(),
            [31, 41, 31]
        );
    }

    #[test]
    fn process_problems_preserves_expired_total_limit_signed_arithmetic() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.source_files.push("late.p".to_owned());
        spec.dest_files.push("late.out".to_owned());
        let mut times = [10, 20].into_iter();
        let mut limit = 0;

        spec.process_problems_with(
            BatchProcessProblemsConfig {
                total_wtc_limit: 5,
                ..BatchProcessProblemsConfig::default()
            },
            || times.next().unwrap(),
            |job| {
                limit = job.wct_limit;
                Ok(false)
            },
        )
        .unwrap();

        assert_eq!(limit, -4);
    }

    #[test]
    fn process_problems_rejects_mismatched_source_and_dest_lists() {
        let mut spec = BatchSpec::new("eprover", IoFormat::Tstp);
        spec.source_files.push("p.p".to_owned());
        let error = spec
            .process_problems_with(BatchProcessProblemsConfig::default(), || 0, |_| Ok(false))
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert!(error.message().contains("source files"));
    }

    fn job_to_tuple(
        job: BatchProcessProblemJob<'_>,
    ) -> (usize, i64, Option<String>, String, String) {
        (
            job.index,
            job.wct_limit,
            job.default_dir.map(str::to_owned),
            job.source.to_owned(),
            job.dest.to_owned(),
        )
    }

    fn test_signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        signature
    }

    fn test_bank() -> TermBank {
        TermBank::new(test_signature()).unwrap()
    }

    struct FlushCountingWriter {
        writes: Vec<u8>,
        flushes: Rc<Cell<usize>>,
    }

    impl FlushCountingWriter {
        fn new(flushes: Rc<Cell<usize>>) -> Self {
            Self {
                writes: Vec::new(),
                flushes,
            }
        }
    }

    impl Write for FlushCountingWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes.set(self.flushes.get() + 1);
            Ok(())
        }
    }

    struct FakeRunnerBackend {
        active: usize,
        polls: usize,
        requests: Vec<BatchRunnerRequest>,
        payloads: Vec<String>,
        completions: VecDeque<BatchCompletedRunner>,
    }

    impl FakeRunnerBackend {
        fn new(completed: Option<BatchCompletedRunner>) -> Self {
            Self::with_completions(completed)
        }

        fn with_completions(completions: impl IntoIterator<Item = BatchCompletedRunner>) -> Self {
            Self {
                active: 0,
                polls: 0,
                requests: Vec::new(),
                payloads: Vec::new(),
                completions: completions.into_iter().collect(),
            }
        }
    }

    fn theorem_completion(output: &str) -> BatchCompletedRunner {
        BatchCompletedRunner {
            runner: BatchSpawnedRunner {
                name: "Threshold(10000) => --satauto-schedule --assume-incompleteness".to_owned(),
                start_time: 100,
                prob_time: 6,
            },
            result: ProverResult::Theorem,
            output: output.to_owned(),
        }
    }

    impl BatchRunnerBackend for FakeRunnerBackend {
        fn active_count(&self) -> usize {
            self.active
        }

        fn spawn_runner(
            &mut self,
            request: BatchRunnerTempRequest,
        ) -> Result<BatchSpawnedRunner, Diagnostic> {
            let payload = fs::read_to_string(&request.input_file).unwrap();
            let _removed = temp_file_remove(&request.input_file).unwrap();
            let spawned = BatchSpawnedRunner {
                name: request.request.name.clone(),
                start_time: 100,
                prob_time: request.request.cpu_time,
            };
            self.active += 1;
            self.payloads.push(payload);
            self.requests.push(request.request);
            Ok(spawned)
        }

        fn poll_runner<W: Write>(
            &mut self,
            _output: &mut W,
        ) -> Result<Option<BatchCompletedRunner>, Diagnostic> {
            self.polls += 1;
            let completed = self.completions.pop_front();
            if completed.is_some() {
                self.active = self.active.saturating_sub(1);
            }
            Ok(completed)
        }

        fn clear(&mut self, _delete_files: bool) -> Result<(), Diagnostic> {
            self.active = 0;
            Ok(())
        }
    }

    struct TmpDirGuard {
        previous: Option<OsString>,
    }

    impl TmpDirGuard {
        fn set(path: &PathBuf) -> Self {
            fs::create_dir_all(path).unwrap();
            let previous = std::env::var_os("TMPDIR");
            std::env::set_var("TMPDIR", path);
            Self { previous }
        }
    }

    impl Drop for TmpDirGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var("TMPDIR", value),
                None => std::env::remove_var("TMPDIR"),
            }
        }
    }

    fn test_temp_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join("batch-spec-temp")
    }

    fn test_path(name: &str) -> PathBuf {
        let dir = test_temp_dir();
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("{name}-{}", std::process::id()))
    }

    fn shared_spec(signature: &Signature) -> StructFofSpec {
        let mut ctrl = StructFofSpec::new(signature);
        ctrl.add_problem(
            signature,
            ClauseSet::from_clauses([Clause::empty()]),
            FormulaSet::new(),
            false,
        );
        ctrl.mark_shared_axioms(signature);
        ctrl
    }

    fn one_empty_clause_problem() -> BatchProblemData {
        BatchProblemData {
            clauses: ClauseSet::from_clauses([Clause::empty()]),
            formulas: FormulaSet::new(),
            problem_type: ProblemType::FirstOrder,
        }
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
        let exit_code = exit_code.to_string();
        command.args([
            "-c",
            "printf '%s\\n' \"$1\"; exit \"$2\"",
            "sh",
            status,
            &exit_code,
        ]);
        command
    }
}
