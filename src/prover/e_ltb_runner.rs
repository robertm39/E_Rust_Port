use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::set_verbose_level;
use crate::control::batch_spec::{
    parse_ltb_header, BatchProcCtrlRunnerSet, BatchProcessFileConfig, BatchProcessProblemsConfig,
    BatchProcessVariantsConfig, BatchRunnerBackend, BatchSpec, BatchVariantProblemJob,
};
use crate::control::gproc_ctrl::EGPCtrl;
use crate::control::sine::StructFofSpec;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::prover::version::{footer, E_NICKNAME, VERSION};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROGRAM_NAME: &str = "e_ltb_runner";

const DEFAULT_PROVER: &str = "eprover";
const C_USAGE_ERROR: &str = "Usage: e_ltb_runner <spec> [<path-to-eprover>]";
const INTERNAL_VARIANT_CHILD_ARG: &str = "--internal-ltb-variant-child";
const VARIANT_CHILD_NAME: &str = "E-LTB wrapper";
const VARIANT_CHILD_CORES: usize = 1;
const VARIANT_CHILD_CPU_LIMIT: u64 = 1_000_000;

const VARIANTS27: &[&str] = &["+4", "+5", "_4", "_5"];
const PROVERS27: &[&str] = &["./eprover", "./eprover", "./eprover", "./eprover"];
const VARIANTS28: &[&str] = &["+1", "_1"];
const PROVERS28: &[&str] = &["./eprover", "./eprover"];
const VARIANTS28_HO: &[&str] = &["+1", "_1", "^1"];
const PROVERS28_HO: &[&str] = &["./eprover", "./eprover", "./eprover-ho"];
const VARIANTS28_25: &[&str] = &["+1", "_1"];
const PROVERS28_25: &[&str] = &["./eprover-25", "./eprover-25"];
const VARIANTSJ11: &[&str] = &["_1", "_3", "^1"];
const PROVERSJ11: &[&str] = &["./eprover-ho", "./eprover-ho", "./eprover-ho"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    OutputFile,
    OutputDir,
    Variants27,
    Variants28,
    Variants28Ho,
    Variants28_25,
    VariantsJ11,
    Interactive,
    Silent,
    OutputLevel,
    GlobalWtcLimit,
}

const OPTIONS: &[OptCell<OptionCode>] = &[
    OptCell::new(
        OptionCode::Help,
        Some('h'),
        Some("help"),
        OptArgType::NoArg,
        None,
        "Print a short description of program usage and options.",
    ),
    OptCell::new(
        OptionCode::Version,
        Some('V'),
        Some("version"),
        OptArgType::NoArg,
        None,
        "Print the version number of the prover. Please include this with all bug reports (if any).",
    ),
    OptCell::new(
        OptionCode::Verbose,
        Some('v'),
        Some("verbose"),
        OptArgType::OptArg,
        Some("1"),
        "Verbose comments on the progress of the program.",
    ),
    OptCell::new(
        OptionCode::OutputFile,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file.",
    ),
    OptCell::new(
        OptionCode::OutputDir,
        Some('d'),
        Some("output-dir"),
        OptArgType::ReqArg,
        None,
        "Prefix generated per-problem output files with the named directory.",
    ),
    OptCell::new(
        OptionCode::Variants27,
        None,
        Some("variants27"),
        OptArgType::NoArg,
        None,
        "Use the CASC-27 variant problem setup.",
    ),
    OptCell::new(
        OptionCode::Variants28,
        None,
        Some("variants28"),
        OptArgType::NoArg,
        None,
        "Use the CASC-28 variant problem setup.",
    ),
    OptCell::new(
        OptionCode::Variants28Ho,
        None,
        Some("variants28-ho"),
        OptArgType::NoArg,
        None,
        "Use the CASC-28 higher-order variant problem setup.",
    ),
    OptCell::new(
        OptionCode::Variants28_25,
        None,
        Some("variants28-25"),
        OptArgType::NoArg,
        None,
        "Use the CASC-28 25-variant problem setup.",
    ),
    OptCell::new(
        OptionCode::VariantsJ11,
        None,
        Some("variantsj11"),
        OptArgType::NoArg,
        None,
        "Use the CASC-J11 variant problem setup.",
    ),
    OptCell::new(
        OptionCode::Interactive,
        Some('i'),
        Some("interactive"),
        OptArgType::NoArg,
        None,
        "Enter interactive mode after each batch.",
    ),
    OptCell::new(
        OptionCode::Silent,
        Some('s'),
        Some("silent"),
        OptArgType::NoArg,
        None,
        "Suppress nonessential output.",
    ),
    OptCell::new(
        OptionCode::OutputLevel,
        Some('l'),
        Some("output-level"),
        OptArgType::ReqArg,
        None,
        "Set the output level.",
    ),
    OptCell::new(
        OptionCode::GlobalWtcLimit,
        Some('w'),
        Some("wtc-limit"),
        OptArgType::ReqArg,
        None,
        "Set a global wall-clock limit for specs that omit one.",
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LtbVariantMode {
    Variants27,
    Variants28,
    Variants28Ho,
    Variants28_25,
    VariantsJ11,
}

impl LtbVariantMode {
    #[must_use]
    const fn variants(self) -> &'static [&'static str] {
        match self {
            Self::Variants27 => VARIANTS27,
            Self::Variants28 => VARIANTS28,
            Self::Variants28Ho => VARIANTS28_HO,
            Self::Variants28_25 => VARIANTS28_25,
            Self::VariantsJ11 => VARIANTSJ11,
        }
    }

    #[must_use]
    const fn provers(self) -> &'static [&'static str] {
        match self {
            Self::Variants27 => PROVERS27,
            Self::Variants28 => PROVERS28,
            Self::Variants28Ho => PROVERS28_HO,
            Self::Variants28_25 => PROVERS28_25,
            Self::VariantsJ11 => PROVERSJ11,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LtbRunnerConfig {
    spec_file: String,
    prover: String,
    output_file: Option<String>,
    output_dir: Option<String>,
    total_wtc_limit: i64,
    verbose_level: i64,
    output_level: i64,
    interactive: bool,
    variant_mode: Option<LtbVariantMode>,
}

#[derive(Debug)]
enum RunCommand {
    Execute(LtbRunnerConfig),
    Exit(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LtbBatchJob<'a> {
    spec_file: &'a str,
    batch_index: usize,
    default_dir: Option<&'a str>,
    output_dir: Option<&'a str>,
    start: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LtbVariantChildConfig {
    spec_file: String,
    batch_index: usize,
    variant: String,
    prover: String,
    source: String,
    dest: String,
    wct_limit: i64,
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    init_io(PROGRAM_NAME);
    let result = run_inner(argv, stdout);
    exit_io();
    result
}

fn run_inner<I, S>(argv: I, stdout: &mut impl Write) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let argv = argv.into_iter().map(Into::into).collect::<Vec<_>>();
    if argv
        .get(1)
        .is_some_and(|arg| arg == INTERNAL_VARIANT_CHILD_ARG)
    {
        return run_variant_child_from_args(&argv[2..], stdout);
    }

    match process_options(argv, stdout)? {
        RunCommand::Execute(config) => execute_config_to_configured_output(&config, stdout),
        RunCommand::Exit(status) => Ok(status),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut output_file = None;
    let mut output_dir = None;
    let mut total_wtc_limit = 0;
    let mut verbose_level = 0;
    let mut output_level = 1;
    let mut interactive = false;
    let mut variant_mode = None;

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Verbose => {
                verbose_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::OutputFile => {
                output_file = Some(required_arg(&parsed, "output-file")?.to_owned());
            }
            OptionCode::OutputDir => {
                output_dir = Some(required_arg(&parsed, "output-dir")?.to_owned());
            }
            OptionCode::Variants27 => variant_mode = Some(LtbVariantMode::Variants27),
            OptionCode::Variants28 => variant_mode = Some(LtbVariantMode::Variants28),
            OptionCode::Variants28Ho => variant_mode = Some(LtbVariantMode::Variants28Ho),
            OptionCode::Variants28_25 => variant_mode = Some(LtbVariantMode::Variants28_25),
            OptionCode::VariantsJ11 => variant_mode = Some(LtbVariantMode::VariantsJ11),
            OptionCode::Interactive => interactive = true,
            OptionCode::Silent => output_level = 0,
            OptionCode::OutputLevel => {
                output_level =
                    get_int_arg(parsed.option(), required_arg(&parsed, "output-level")?)?;
            }
            OptionCode::GlobalWtcLimit => {
                total_wtc_limit =
                    get_int_arg(parsed.option(), required_arg(&parsed, "wtc-limit")?)?;
            }
        }
    }

    let positional = state.remaining_args();
    if positional.is_empty() || positional.len() > 2 {
        return Err(Diagnostic::new(ErrorCode::USAGE_ERROR, C_USAGE_ERROR));
    }

    Ok(RunCommand::Execute(LtbRunnerConfig {
        spec_file: positional[0].clone(),
        prover: positional
            .get(1)
            .cloned()
            .unwrap_or_else(|| DEFAULT_PROVER.to_owned()),
        output_file,
        output_dir,
        total_wtc_limit,
        verbose_level,
        output_level,
        interactive,
        variant_mode,
    }))
}

fn execute_config_to_configured_output(
    config: &LtbRunnerConfig,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    if let Some(path) = &config.output_file {
        if path == "-" {
            return execute_config(config, stdout);
        }
        let mut output = File::create(path)
            .map_err(|error| io_diagnostic(format!("Cannot open file {path}: {error}")))?;
        execute_config(config, &mut output)
    } else {
        execute_config(config, stdout)
    }
}

fn execute_config(config: &LtbRunnerConfig, output: &mut impl Write) -> Result<u8, Diagnostic> {
    execute_config_with_processors(
        config,
        output,
        current_time_seconds,
        process_non_variant_batch,
        process_variant_batch,
    )
}

#[cfg(test)]
fn execute_config_with_processor<W, C, P>(
    config: &LtbRunnerConfig,
    output: &mut W,
    clock_seconds: C,
    process_batch: P,
) -> Result<u8, Diagnostic>
where
    W: Write + ?Sized,
    C: FnMut() -> i64,
    P: FnMut(&mut BatchSpec, LtbBatchJob<'_>, &mut C, &mut W) -> Result<i64, Diagnostic>,
{
    execute_config_with_processors(
        config,
        output,
        clock_seconds,
        process_batch,
        process_variant_batch,
    )
}

fn execute_config_with_processors<W, C, P, V>(
    config: &LtbRunnerConfig,
    output: &mut W,
    mut clock_seconds: C,
    mut process_batch: P,
    mut process_variants: V,
) -> Result<u8, Diagnostic>
where
    W: Write + ?Sized,
    C: FnMut() -> i64,
    P: FnMut(&mut BatchSpec, LtbBatchJob<'_>, &mut C, &mut W) -> Result<i64, Diagnostic>,
    V: FnMut(
        &mut BatchSpec,
        LtbBatchJob<'_>,
        LtbVariantMode,
        &mut C,
        &mut W,
    ) -> Result<(), Diagnostic>,
{
    apply_global_options(config)?;

    if config.interactive {
        return Err(Diagnostic::new(
            ErrorCode::INTERFACE_ERROR,
            "e_ltb_runner interactive mode is not wired yet",
        ));
    }

    let mut scanner = Scanner::from_file(Path::new(&config.spec_file), true)?;
    scanner.set_format(IoFormat::Tstp);
    let default_dir = scanner.default_dir().to_owned();
    let header = parse_ltb_header(&mut scanner)?;
    let mut batch_index = 0;

    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let start = clock_seconds();
        let mut spec = BatchSpec::parse_with_include_output(
            &mut scanner,
            &config.prover,
            &header.category,
            header.train_dir.as_deref(),
            IoFormat::Tstp,
            output,
        )?;
        if config.total_wtc_limit != 0 && spec.total_wtc_limit == 0 {
            spec.total_wtc_limit = config.total_wtc_limit;
        }
        if spec.per_prob_limit <= 0 && spec.total_wtc_limit <= 0 {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Batch specification must set either a per-problem or total wall-clock limit",
            ));
        }

        let job = LtbBatchJob {
            spec_file: &config.spec_file,
            batch_index,
            default_dir: Some(default_dir.as_str()),
            output_dir: config.output_dir.as_deref(),
            start,
        };
        if let Some(variant_mode) = config.variant_mode {
            process_variants(&mut spec, job, variant_mode, &mut clock_seconds, output)?;
        } else {
            let solved = process_batch(&mut spec, job, &mut clock_seconds, output)?;
            let elapsed = clock_seconds() - start;
            write_batch_done(output, elapsed, solved, spec.problem_no())?;
        }
        batch_index += 1;
    }

    Ok(ErrorCode::NO_ERROR.exit_status())
}

fn process_non_variant_batch<W, C>(
    spec: &mut BatchSpec,
    job: LtbBatchJob<'_>,
    clock_seconds: &mut C,
    output: &mut W,
) -> Result<i64, Diagnostic>
where
    W: Write,
    C: FnMut() -> i64,
{
    let mut bank = new_term_bank()?;
    let mut ctrl = StructFofSpec::new(bank.signature());
    spec.init_struct_fof_spec_from_files(&mut bank, &mut ctrl, job.default_dir, output)?;
    let now = clock_seconds();
    let remaining = remaining_total_wtc_limit(spec.total_wtc_limit, job.start, now);
    let mut backend = BatchProcCtrlRunnerSet::new();
    let report = spec.process_problems_with_runner_backend(
        &mut bank,
        &mut ctrl,
        BatchProcessProblemsConfig {
            total_wtc_limit: remaining,
            default_dir: job.default_dir,
            dest_dir: job.output_dir,
        },
        output,
        clock_seconds,
        &mut backend,
    )?;
    Ok(report.c_return_value())
}

fn process_variant_batch<W, C>(
    spec: &mut BatchSpec,
    job: LtbBatchJob<'_>,
    variant_mode: LtbVariantMode,
    clock_seconds: &mut C,
    output: &mut W,
) -> Result<(), Diagnostic>
where
    W: Write + ?Sized,
    C: FnMut() -> i64,
{
    spec.process_variants_with_child_processes(
        BatchProcessVariantsConfig {
            variants: variant_mode.variants(),
            provers: variant_mode.provers(),
            start: job.start,
            default_dir: job.default_dir,
            outdir: job.output_dir,
        },
        output,
        clock_seconds,
        |variant_job, startup_output| {
            spawn_ltb_variant_child(job.spec_file, job.batch_index, variant_job, startup_output)
        },
    )?;
    writeln!(output, "% =============== Variant batch done ===========\n")
        .map_err(|error| io_diagnostic(format!("Cannot write variant batch summary: {error}")))
}

fn spawn_ltb_variant_child<W: Write + ?Sized>(
    spec_file: &str,
    batch_index: usize,
    job: &BatchVariantProblemJob,
    startup_output: &mut W,
) -> Result<EGPCtrl, Diagnostic> {
    let current_exe = std::env::current_exe().map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Cannot locate e_ltb_runner executable: {error}"),
        )
    })?;
    let mut command = Command::new(current_exe);
    command
        .arg(INTERNAL_VARIANT_CHILD_ARG)
        .arg(spec_file)
        .arg(batch_index.to_string())
        .arg(&job.variant)
        .arg(&job.prover)
        .arg(&job.concrete_source)
        .arg(&job.dest)
        .arg(job.wct_limit.to_string());
    EGPCtrl::spawn_command_reporting(
        command,
        VARIANT_CHILD_NAME,
        VARIANT_CHILD_CORES,
        VARIANT_CHILD_CPU_LIMIT,
        startup_output,
    )
}

fn run_variant_child_from_args(args: &[String], stdout: &mut impl Write) -> Result<u8, Diagnostic> {
    let config = parse_variant_child_args(args)?;
    execute_variant_child(&config, stdout)
}

fn parse_variant_child_args(args: &[String]) -> Result<LtbVariantChildConfig, Diagnostic> {
    if args.len() != 7 {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Usage: {PROGRAM_NAME} {INTERNAL_VARIANT_CHILD_ARG} <spec> <batch-index> <variant> <prover> <source> <dest> <wct-limit>"
            ),
        ));
    }
    let batch_index = args[1].parse::<usize>().map_err(|error| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Invalid LTB variant child batch index '{}': {error}",
                args[1]
            ),
        )
    })?;
    let wct_limit = args[6].parse::<i64>().map_err(|error| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Invalid LTB variant child wall-clock limit '{}': {error}",
                args[6]
            ),
        )
    })?;

    Ok(LtbVariantChildConfig {
        spec_file: args[0].clone(),
        batch_index,
        variant: args[2].clone(),
        prover: args[3].clone(),
        source: args[4].clone(),
        dest: args[5].clone(),
        wct_limit,
    })
}

fn execute_variant_child(
    config: &LtbVariantChildConfig,
    output: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut backend = BatchProcCtrlRunnerSet::new();
    execute_variant_child_with_backend(config, output, current_time_seconds, &mut backend)
}

fn execute_variant_child_with_backend<W, C, B>(
    config: &LtbVariantChildConfig,
    output: &mut W,
    clock_seconds: C,
    backend: &mut B,
) -> Result<u8, Diagnostic>
where
    W: Write,
    C: FnMut() -> i64,
    B: BatchRunnerBackend,
{
    let (mut spec, default_dir) =
        parse_ltb_batch_spec_at(&config.spec_file, config.batch_index, &config.prover)?;
    spec.executable.clone_from(&config.prover);
    let mut bank = new_term_bank()?;
    let mut ctrl = StructFofSpec::new(bank.signature());
    spec.init_concrete_struct_fof_spec_from_files(
        &mut bank,
        &mut ctrl,
        Some(default_dir.as_str()),
        &config.variant,
        output,
    )?;
    let report = spec.process_file_with_runner_backend(
        &mut bank,
        &mut ctrl,
        BatchProcessFileConfig {
            wct_limit: config.wct_limit,
            default_dir: Some(default_dir.as_str()),
            source: &config.source,
            dest: &config.dest,
        },
        output,
        clock_seconds,
        backend,
    )?;
    if report.solved {
        Ok(1)
    } else {
        Ok(0)
    }
}

fn parse_ltb_batch_spec_at(
    spec_file: &str,
    batch_index: usize,
    prover: &str,
) -> Result<(BatchSpec, String), Diagnostic> {
    let mut scanner = Scanner::from_file(Path::new(spec_file), true)?;
    scanner.set_format(IoFormat::Tstp);
    let default_dir = scanner.default_dir().to_owned();
    let header = parse_ltb_header(&mut scanner)?;
    let mut include_sink = std::io::sink();

    for index in 0..=batch_index {
        if scanner.test_tok(TokenType::NO_TOKEN) {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!("LTB child requested missing batch index {batch_index}"),
            ));
        }
        let spec = BatchSpec::parse_with_include_output(
            &mut scanner,
            prover,
            &header.category,
            header.train_dir.as_deref(),
            IoFormat::Tstp,
            &mut include_sink,
        )?;
        if index == batch_index {
            return Ok((spec, default_dir));
        }
    }

    Err(Diagnostic::new(
        ErrorCode::INTERFACE_ERROR,
        "LTB child batch parser did not return the requested batch",
    ))
}

fn apply_global_options(config: &LtbRunnerConfig) -> Result<(), Diagnostic> {
    let verbose = i32::try_from(config.verbose_level).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "--verbose argument {} is out of int range",
                config.verbose_level
            ),
        )
    })?;
    set_verbose_level(verbose);
    let _old_output_level = set_output_level(config.output_level);
    Ok(())
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
{PROGRAM_NAME} {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} [options] <spec> [<path-to-eprover>]\n\
\n\
Run E on a CASC LTB batch specification.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

fn write_batch_done<W: Write + ?Sized>(
    output: &mut W,
    elapsed: i64,
    solved: i64,
    problem_count: usize,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "\n\n% == WCT: {elapsed:4}s, Solved: {solved:4}/{problem_count:4}    =="
    )
    .map_err(|error| io_diagnostic(format!("Cannot write batch summary: {error}")))?;
    writeln!(output, "% =============== Batch done ===========\n")
        .map_err(|error| io_diagnostic(format!("Cannot write batch summary: {error}")))
}

fn new_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

#[must_use]
const fn remaining_total_wtc_limit(total: i64, start: i64, now: i64) -> i64 {
    let remaining = total - (now - start);
    if remaining < 0 {
        0
    } else {
        remaining
    }
}

fn required_arg<'a>(
    parsed: &'a crate::inout::commandline::ParsedOpt<'a, OptionCode>,
    name: &str,
) -> Result<&'a str, Diagnostic> {
    parsed.arg().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Option {name} requires an argument"),
        )
    })
}

fn current_time_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
}

fn write_all(output: &mut impl Write, bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn writeln_diag(output: &mut impl Write, line: &str) -> Result<(), Diagnostic> {
    write_all(output, line.as_bytes())?;
    write_all(output, b"\n")
}

fn io_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        execute_config_with_processor, execute_config_with_processors,
        execute_variant_child_with_backend, parse_variant_child_args, print_help, process_options,
        remaining_total_wtc_limit, run, LtbBatchJob, LtbRunnerConfig, LtbVariantChildConfig,
        LtbVariantMode, RunCommand, C_USAGE_ERROR, DEFAULT_PROVER, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProverResult;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::control::batch_spec::{
        BatchCompletedRunner, BatchRunnerBackend, BatchRunnerRequest, BatchRunnerTempRequest,
        BatchSpawnedRunner, BatchSpec,
    };
    use crate::inout::output::{output_level, set_output_level};
    use crate::inout::tempfile::{temp_file_remove, temp_file_test_lock};
    use crate::test_support::global_state_lock;
    use std::ffi::OsString;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    #[test]
    fn help_and_version_exit_before_processing_specs() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).unwrap();
        assert!(help.contains("Usage: e_ltb_runner [options] <spec> [<path-to-eprover>]"));
        assert!(help.contains("Run E on a CASC LTB batch specification."));

        let mut stdout = Vec::new();
        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert!(String::from_utf8(stdout)
            .unwrap()
            .starts_with("e_ltb_runner "));
    }

    #[test]
    fn process_options_parses_non_variant_runner_args() {
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "-o",
                "all.out",
                "--output-dir=Results",
                "-w",
                "90",
                "--verbose",
                "--output-level=2",
                "batch.spec",
                "custom-eprover",
            ],
            &mut stdout,
        )
        .unwrap();
        let RunCommand::Execute(config) = command else {
            panic!("valid e_ltb_runner arguments should execute");
        };

        assert_eq!(config.spec_file, "batch.spec");
        assert_eq!(config.prover, "custom-eprover");
        assert_eq!(config.output_file.as_deref(), Some("all.out"));
        assert_eq!(config.output_dir.as_deref(), Some("Results"));
        assert_eq!(config.total_wtc_limit, 90);
        assert_eq!(config.verbose_level, 1);
        assert_eq!(config.output_level, 2);
        assert!(!config.interactive);
        assert_eq!(config.variant_mode, None);
    }

    #[test]
    fn default_prover_and_variant_options_follow_c_globals() {
        let mut stdout = Vec::new();
        let command = process_options(
            [PROGRAM_NAME, "--variants28-ho", "-i", "batch.spec"],
            &mut stdout,
        )
        .unwrap();
        let RunCommand::Execute(config) = command else {
            panic!("valid variant e_ltb_runner arguments should execute");
        };

        assert_eq!(config.spec_file, "batch.spec");
        assert_eq!(config.prover, DEFAULT_PROVER);
        assert_eq!(config.verbose_level, 0);
        assert_eq!(config.output_level, 1);
        assert!(config.interactive);
        assert_eq!(config.variant_mode, Some(LtbVariantMode::Variants28Ho));
    }

    #[test]
    fn variant_mode_tables_match_c_ltb_runner_globals() {
        assert_eq!(
            LtbVariantMode::Variants27.variants(),
            ["+4", "+5", "_4", "_5"]
        );
        assert_eq!(
            LtbVariantMode::Variants27.provers(),
            ["./eprover", "./eprover", "./eprover", "./eprover"]
        );
        assert_eq!(LtbVariantMode::Variants28.variants(), ["+1", "_1"]);
        assert_eq!(
            LtbVariantMode::Variants28.provers(),
            ["./eprover", "./eprover"]
        );
        assert_eq!(LtbVariantMode::Variants28Ho.variants(), ["+1", "_1", "^1"]);
        assert_eq!(
            LtbVariantMode::Variants28Ho.provers(),
            ["./eprover", "./eprover", "./eprover-ho"]
        );
        assert_eq!(LtbVariantMode::Variants28_25.variants(), ["+1", "_1"]);
        assert_eq!(
            LtbVariantMode::Variants28_25.provers(),
            ["./eprover-25", "./eprover-25"]
        );
        assert_eq!(LtbVariantMode::VariantsJ11.variants(), ["_1", "_3", "^1"]);
        assert_eq!(
            LtbVariantMode::VariantsJ11.provers(),
            ["./eprover-ho", "./eprover-ho", "./eprover-ho"]
        );
    }

    #[test]
    fn usage_rejects_missing_and_extra_arguments() {
        let mut stdout = Vec::new();
        let missing = process_options([PROGRAM_NAME], &mut stdout).unwrap_err();
        assert_eq!(missing.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(missing.message(), C_USAGE_ERROR);

        let extra =
            process_options([PROGRAM_NAME, "spec", "eprover", "extra"], &mut stdout).unwrap_err();
        assert_eq!(extra.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(extra.message(), C_USAGE_ERROR);
    }

    #[test]
    fn execute_with_fake_processor_parses_header_and_batches() {
        let _guard = global_state_lock();
        let path = write_temp_spec(
            "runner-batch.spec",
            "division.category LTB.SAT\n\
             division.category.training_data /tmp/train\n\
             output.required Assurance Proof\n\
             limit.time.problem.wc 12\n\
             include('Axioms/SET001.ax').\n\
             Problems/TSTP/prob1.p Results/prob1.out\n",
        );
        let config = LtbRunnerConfig {
            spec_file: path.to_string_lossy().into_owned(),
            prover: "custom-e".to_owned(),
            output_file: None,
            output_dir: Some("Out".to_owned()),
            total_wtc_limit: 40,
            verbose_level: 0,
            output_level: 1,
            interactive: false,
            variant_mode: None,
        };
        let mut output = Vec::new();
        let mut times = [100, 105].into_iter();
        let mut seen = Vec::new();

        let status = execute_config_with_processor(
            &config,
            &mut output,
            || times.next().unwrap_or(105),
            |spec: &mut BatchSpec, job: LtbBatchJob<'_>, _clock, _output| {
                seen.push((
                    spec.executable.clone(),
                    spec.category.clone(),
                    spec.train_dir.clone(),
                    spec.per_prob_limit,
                    spec.total_wtc_limit,
                    spec.source_files.clone(),
                    spec.dest_files.clone(),
                    job.default_dir.map(str::to_owned),
                    job.output_dir.map(str::to_owned),
                    job.start,
                ));
                Ok(1)
            },
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "custom-e");
        assert_eq!(seen[0].1.as_deref(), Some("LTB.SAT"));
        assert_eq!(seen[0].2.as_deref(), Some("/tmp/train"));
        assert_eq!(seen[0].3, 12);
        assert_eq!(seen[0].4, 40);
        assert_eq!(seen[0].5, ["Problems/TSTP/prob1.p"]);
        assert_eq!(seen[0].6, ["Results/prob1.out"]);
        assert_eq!(seen[0].8.as_deref(), Some("Out"));
        assert_eq!(seen[0].9, 100);

        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("% Accepted Axioms/SET001.ax for parsing"));
        assert!(printed.contains("% == WCT:    5s, Solved:    1/   1    =="));
        assert!(printed.contains("% =============== Batch done ==========="));
    }

    #[test]
    fn execute_with_fake_variant_processor_dispatches_variant_mode() {
        let _guard = global_state_lock();
        let path = write_temp_spec(
            "runner-variant-dispatch.spec",
            "division.category LTB.SAT\n\
             output.required Proof\n\
             limit.time.problem.wc 12\n\
             limit.time.overall.wc 40\n\
             Problems/TSTP/prob_*ignored.p Results/prob.out\n",
        );
        let config = LtbRunnerConfig {
            spec_file: path.to_string_lossy().into_owned(),
            prover: DEFAULT_PROVER.to_owned(),
            output_file: None,
            output_dir: Some("Out".to_owned()),
            total_wtc_limit: 0,
            verbose_level: 0,
            output_level: 1,
            interactive: false,
            variant_mode: Some(LtbVariantMode::Variants28),
        };
        let mut output = Vec::new();
        let mut seen = Vec::new();

        let status = execute_config_with_processors(
            &config,
            &mut output,
            || 100,
            |_spec, _job, _clock, _output| {
                panic!("variant mode should not use the non-variant processor")
            },
            |spec, job, mode, _clock, output| {
                seen.push((
                    spec.problem_no(),
                    spec.total_wtc_limit,
                    job.spec_file.to_owned(),
                    job.batch_index,
                    job.output_dir.map(str::to_owned),
                    job.start,
                    mode,
                ));
                writeln!(output, "% fake variant dispatch").unwrap();
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, 1);
        assert_eq!(seen[0].1, 40);
        assert_eq!(seen[0].2, config.spec_file);
        assert_eq!(seen[0].3, 0);
        assert_eq!(seen[0].4.as_deref(), Some("Out"));
        assert_eq!(seen[0].5, 100);
        assert_eq!(seen[0].6, LtbVariantMode::Variants28);
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("% fake variant dispatch"));
    }

    #[test]
    fn execute_rejects_interactive_mode_explicitly() {
        let mut output = Vec::new();
        let config = LtbRunnerConfig {
            spec_file: "missing.spec".to_owned(),
            prover: DEFAULT_PROVER.to_owned(),
            output_file: None,
            output_dir: None,
            total_wtc_limit: 0,
            verbose_level: 0,
            output_level: 1,
            interactive: true,
            variant_mode: None,
        };

        let interactive = execute_config_with_processor(
            &config,
            &mut output,
            || 0,
            |_spec, _job, _clock, _output| Ok(0),
        )
        .unwrap_err();
        assert_eq!(interactive.code(), ErrorCode::INTERFACE_ERROR);
        assert_eq!(
            interactive.message(),
            "e_ltb_runner interactive mode is not wired yet"
        );
    }

    #[test]
    fn missing_limits_are_reported_before_processing() {
        let _guard = global_state_lock();
        let path = write_temp_spec(
            "runner-no-limit.spec",
            "division.category LTB.SAT\n\
             output.required Proof\n\
             limit.time.problem.wc 0\n\
             Problems/TSTP/prob1.p Results/prob1.out\n",
        );
        let config = LtbRunnerConfig {
            spec_file: path.to_string_lossy().into_owned(),
            prover: DEFAULT_PROVER.to_owned(),
            output_file: None,
            output_dir: None,
            total_wtc_limit: 0,
            verbose_level: 0,
            output_level: 1,
            interactive: false,
            variant_mode: None,
        };
        let mut output = Vec::new();

        let error = execute_config_with_processor(
            &config,
            &mut output,
            || 0,
            |_spec, _job, _clock, _output| Ok(0),
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("wall-clock limit"));
    }

    #[test]
    fn execute_applies_verbose_and_output_level_globals() {
        let _guard = global_state_lock();
        let path = write_temp_spec(
            "runner-global-output.spec",
            "division.category LTB.SAT\n\
             output.required Proof\n\
             limit.time.problem.wc 7\n\
             Problems/TSTP/prob1.p Results/prob1.out\n",
        );
        let config = LtbRunnerConfig {
            spec_file: path.to_string_lossy().into_owned(),
            prover: DEFAULT_PROVER.to_owned(),
            output_file: None,
            output_dir: None,
            total_wtc_limit: 0,
            verbose_level: 3,
            output_level: 0,
            interactive: false,
            variant_mode: None,
        };
        let _old_verbose = set_verbose_level(0);
        let _old_output = set_output_level(1);
        let mut output = Vec::new();

        let status = execute_config_with_processor(
            &config,
            &mut output,
            || 0,
            |_spec, _job, _clock, _output| Ok(0),
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(verbose_level(), 3);
        assert_eq!(output_level(), 0);
        let _old_verbose = set_verbose_level(0);
        let _old_output = set_output_level(1);
    }

    #[test]
    fn parse_variant_child_args_uses_fixed_internal_shape() {
        let args = [
            "spec.ltb".to_owned(),
            "2".to_owned(),
            "+1".to_owned(),
            "./eprover".to_owned(),
            "Problems/prob_+1.p".to_owned(),
            "Results/prob.out".to_owned(),
            "17".to_owned(),
        ];

        let config = parse_variant_child_args(&args).unwrap();

        assert_eq!(config.spec_file, "spec.ltb");
        assert_eq!(config.batch_index, 2);
        assert_eq!(config.variant, "+1");
        assert_eq!(config.prover, "./eprover");
        assert_eq!(config.source, "Problems/prob_+1.p");
        assert_eq!(config.dest, "Results/prob.out");
        assert_eq!(config.wct_limit, 17);

        let invalid = parse_variant_child_args(&args[..6]).unwrap_err();
        assert_eq!(invalid.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn execute_variant_child_with_fake_backend_processes_concrete_problem() {
        let _guard = global_state_lock();
        let _temp_guard = temp_file_test_lock();
        let dir = test_temp_dir();
        fs::create_dir_all(dir.join("Problems")).unwrap();
        let _tmpdir_guard = TmpDirGuard::set(&dir);
        let prefix = format!("runner-child-{}-", std::process::id());
        let abstract_include = format!("{prefix}*ignored.ax");
        let concrete_include = format!("{prefix}+1.ax");
        fs::write(
            dir.join(&concrete_include),
            "fof(concrete_shared, axiom, p(a)).\n",
        )
        .unwrap();
        fs::write(
            dir.join("Problems").join("prob_+1.p"),
            "cnf(goal_clause, axiom, $false).\n",
        )
        .unwrap();
        let dest = dir.join(format!("runner-child-{}.out", std::process::id()));
        let _ = fs::remove_file(&dest);
        let spec_path = write_temp_spec(
            "runner-child.spec",
            &format!(
                "division.category LTB.SAT\n\
                 output.required Proof\n\
                 limit.time.problem.wc 12\n\
                 limit.time.overall.wc 40\n\
                 include('{abstract_include}').\n\
                 Problems/prob_*ignored.p Results/prob.out\n"
            ),
        );
        let child = LtbVariantChildConfig {
            spec_file: spec_path.to_string_lossy().replace('\\', "/"),
            batch_index: 0,
            variant: "+1".to_owned(),
            prover: "variant-prover".to_owned(),
            source: "Problems/prob_+1.p".to_owned(),
            dest: dest.to_string_lossy().into_owned(),
            wct_limit: 11,
        };
        let mut output = Vec::new();
        let mut backend = FakeRunnerBackend::new(ProverResult::Theorem, "% child proof\n");

        let status =
            execute_variant_child_with_backend(&child, &mut output, || 100, &mut backend).unwrap();

        assert_eq!(status, 1);
        assert_eq!(backend.requests.len(), 1);
        assert_eq!(backend.requests[0].executable, "variant-prover");
        assert_eq!(backend.requests[0].cpu_time, 6);
        assert!(!backend.payloads[0].is_empty());
        assert!(String::from_utf8(output)
            .unwrap()
            .contains(&format!("% Parsing {concrete_include}\n")));
        assert_eq!(
            fs::read_to_string(dest).unwrap(),
            "% SZS status Theorem for Problems/prob_+1.p\n% child proof\n"
        );
    }

    #[test]
    fn remaining_total_wtc_limit_matches_c_max_zero_boundary() {
        assert_eq!(remaining_total_wtc_limit(30, 10, 17), 23);
        assert_eq!(remaining_total_wtc_limit(30, 10, 50), 0);
    }

    #[test]
    fn help_text_contains_current_version_and_footer() {
        let help = print_help();

        assert!(help.contains("e_ltb_runner "));
        assert!(help.contains("Options:"));
        assert!(help.contains("Copyright 1998-2026 by Stephan Schulz"));
    }

    fn write_temp_spec(name: &str, contents: &str) -> PathBuf {
        let dir = std::env::current_dir()
            .unwrap()
            .join("target")
            .join("e-ltb-runner-tests");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}-{}", std::process::id()));
        fs::write(&path, contents).unwrap();
        path
    }

    fn test_temp_dir() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join("e-ltb-runner-tests")
    }

    struct FakeRunnerBackend {
        active: usize,
        completed: Option<BatchCompletedRunner>,
        requests: Vec<BatchRunnerRequest>,
        payloads: Vec<String>,
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

    impl FakeRunnerBackend {
        fn new(result: ProverResult, output: &str) -> Self {
            Self {
                active: 0,
                completed: Some(BatchCompletedRunner {
                    runner: BatchSpawnedRunner {
                        name: "Threshold(10000) => --satauto-schedule --assume-incompleteness"
                            .to_owned(),
                        start_time: 100,
                        prob_time: 6,
                    },
                    result,
                    output: output.to_owned(),
                }),
                requests: Vec::new(),
                payloads: Vec::new(),
            }
        }
    }

    impl BatchRunnerBackend for FakeRunnerBackend {
        fn active_count(&self) -> usize {
            self.active
        }

        fn spawn_runner(
            &mut self,
            request: BatchRunnerTempRequest,
        ) -> Result<BatchSpawnedRunner, crate::basics::error::Diagnostic> {
            let payload = fs::read_to_string(&request.input_file).unwrap();
            let _removed = temp_file_remove(&request.input_file).unwrap();
            self.payloads.push(payload);
            self.requests.push(request.request);
            self.active = crate::control::proc_ctrl::MAX_CORES;
            Ok(BatchSpawnedRunner {
                name: "Threshold(10000) => --satauto-schedule --assume-incompleteness".to_owned(),
                start_time: 100,
                prob_time: 6,
            })
        }

        fn poll_runner<W: std::io::Write>(
            &mut self,
            _output: &mut W,
        ) -> Result<Option<BatchCompletedRunner>, crate::basics::error::Diagnostic> {
            self.active = 0;
            Ok(self.completed.take())
        }

        fn clear(&mut self, _delete_files: bool) -> Result<(), crate::basics::error::Diagnostic> {
            self.active = 0;
            Ok(())
        }
    }
}
