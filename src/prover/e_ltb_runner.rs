use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::set_verbose_level;
use crate::control::batch_spec::{
    parse_ltb_header, BatchProcCtrlRunnerSet, BatchProcessProblemsConfig, BatchSpec,
};
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
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROGRAM_NAME: &str = "e_ltb_runner";

const DEFAULT_PROVER: &str = "eprover";
const C_USAGE_ERROR: &str = "Usage: e_ltb_runner <spec> [<path-to-eprover>]";

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
    default_dir: Option<&'a str>,
    output_dir: Option<&'a str>,
    start: i64,
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
    execute_config_with_processor(
        config,
        output,
        current_time_seconds,
        process_non_variant_batch,
    )
}

fn execute_config_with_processor<W, C, P>(
    config: &LtbRunnerConfig,
    output: &mut W,
    mut clock_seconds: C,
    mut process_batch: P,
) -> Result<u8, Diagnostic>
where
    W: Write + ?Sized,
    C: FnMut() -> i64,
    P: FnMut(&mut BatchSpec, LtbBatchJob<'_>, &mut C, &mut W) -> Result<i64, Diagnostic>,
{
    apply_global_options(config)?;

    if let Some(variant_mode) = config.variant_mode {
        return Err(Diagnostic::new(
            ErrorCode::INTERFACE_ERROR,
            format!("e_ltb_runner variant mode {variant_mode:?} is not wired yet"),
        ));
    }
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

        let solved = process_batch(
            &mut spec,
            LtbBatchJob {
                default_dir: Some(default_dir.as_str()),
                output_dir: config.output_dir.as_deref(),
                start,
            },
            &mut clock_seconds,
            output,
        )?;
        let elapsed = clock_seconds() - start;
        write_batch_done(output, elapsed, solved, spec.problem_no())?;
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
        execute_config_with_processor, print_help, process_options, remaining_total_wtc_limit, run,
        LtbBatchJob, LtbRunnerConfig, LtbVariantMode, RunCommand, C_USAGE_ERROR, DEFAULT_PROVER,
        PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::control::batch_spec::BatchSpec;
    use crate::inout::output::{output_level, set_output_level};
    use crate::test_support::global_state_lock;
    use std::fs;
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
    fn execute_rejects_not_yet_wired_modes_explicitly() {
        let mut output = Vec::new();
        let mut config = LtbRunnerConfig {
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

        config.interactive = false;
        config.variant_mode = Some(LtbVariantMode::VariantsJ11);
        let variant = execute_config_with_processor(
            &config,
            &mut output,
            || 0,
            |_spec, _job, _clock, _output| Ok(0),
        )
        .unwrap_err();
        assert_eq!(variant.code(), ErrorCode::INTERFACE_ERROR);
        assert!(variant.message().contains("variant mode VariantsJ11"));
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
}
