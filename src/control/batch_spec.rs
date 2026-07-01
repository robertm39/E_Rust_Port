use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProverResult;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::formulasets::FormulaSet;
use crate::control::proc_ctrl::{prover_result_table_entry, MAX_CORES};
use crate::control::sine::StructFofSpec;
use crate::heuristics::axfilter::{AxFilter, AxFilterType};
use crate::inout::basicparser::{
    accept_dotted_id, parse_basic_include, parse_continuous, parse_dotted_id, parse_filename,
    parse_int,
};
use crate::inout::scanner::{token_pos_rep, IoFormat, Scanner, TokenType};
use crate::terms::signature::Signature;
use std::io::{self, Write};

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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchProcessProblemConfig<'a> {
    pub wct_limit: i64,
    pub jobname: &'a str,
    pub interactive: bool,
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
        ctrl.add_problem(signature, problem.clauses, problem.formulas, false);

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

    if let Some(external_output) = outputs.external_output.as_deref_mut() {
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
    if let Some(external_output) = outputs.external_output.as_deref_mut() {
        writeln!(
            external_output,
            "% SZS status GaveUp for {}",
            config.jobname
        )
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
        BatchOutputType, BatchProblemData, BatchProcessProblemConfig, BatchProcessProblemJob,
        BatchProcessProblemOutputs, BatchProcessProblemsConfig, BatchRunnerCreateConfig,
        BatchSpawnedRunner, BatchSpec, BATCH_FILTERS, BATCH_FILTERS_DIV, BATCH_STRATEGIES,
        BATCH_STRATEGIES_DIV,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProverResult;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::formulasets::FormulaSet;
    use crate::control::proc_ctrl::MAX_CORES;
    use crate::control::sine::StructFofSpec;
    use crate::heuristics::axfilter::AxFilter;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::terms::signature::Signature;
    use crate::terms::typebanks::TypeBank;

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
        }
    }
}
