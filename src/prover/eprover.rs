use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_int_arg, get_int_arg_check_range, print_options, CommandLineState, ParsedOpt,
};
use crate::inout::output::set_output_level;
use crate::inout::signals::{set_hard_time_limit, set_schedule_time_limit, set_soft_time_limit};
use crate::prover::options::{EProverOption, EPROVER_OPTIONS};
use crate::prover::version::{self, E_NICKNAME, PROGRAM_NAME, VERSION};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EProverAction {
    Help,
    Version,
    Run(EProverConfig),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EProverConfig {
    pub files: Vec<String>,
    pub output_file: Option<String>,
    pub output_level: i64,
    pub verbose: i64,
    pub proof_object_level: i64,
    pub proof_output: i64,
    pub force_derivation_output: i64,
    pub training_examples: Option<i64>,
    pub cpu_limit: Option<i64>,
    pub soft_cpu_limit: Option<i64>,
    pub schedule_time_limit: Option<i64>,
    pub memory_limit: Option<String>,
    pub flags: EProverFlags,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EProverFlags {
    bits: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EProverFlag {
    SyntaxOnly = 1 << 0,
    PrintPid = 1 << 1,
    PrintVersion = 1 << 2,
    Auto = 1 << 3,
    DeterministicRewriteSort = 1 << 4,
    DeterministicNewSort = 1 << 5,
    PrintFormulas = 1 << 6,
    PruneOnly = 1 << 7,
    CnfOnly = 1 << 8,
    RequireNonempty = 1 << 9,
    ProofStatistics = 1 << 10,
    FullDerivation = 1 << 11,
    RecordGivenClauses = 1 << 12,
}

impl EProverFlags {
    pub fn set(&mut self, flag: EProverFlag) {
        self.bits |= flag as u16;
    }

    #[must_use]
    pub const fn contains(self, flag: EProverFlag) -> bool {
        (self.bits & flag as u16) != 0
    }
}

impl Default for EProverConfig {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            output_file: None,
            output_level: 1,
            verbose: 0,
            proof_object_level: 0,
            proof_output: 0,
            force_derivation_output: 0,
            training_examples: None,
            cpu_limit: None,
            soft_cpu_limit: None,
            schedule_time_limit: None,
            memory_limit: None,
            flags: EProverFlags::default(),
        }
    }
}

#[derive(Debug)]
pub enum EProverError {
    Diagnostic(Diagnostic),
    Io(io::Error),
}

impl EProverError {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.code(),
            Self::Io(_) => ErrorCode::OTHER_ERROR,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.message().to_owned(),
            Self::Io(error) => error.to_string(),
        }
    }
}

impl fmt::Display for EProverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for EProverError {}

impl From<Diagnostic> for EProverError {
    fn from(value: Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<io::Error> for EProverError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

enum ConfiguredOutput<'a, W: Write + ?Sized> {
    Writer(&'a mut W),
    File(File),
}

impl<W: Write + ?Sized> Write for ConfiguredOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Writer(writer) => writer.write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Writer(writer) => writer.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

fn open_configured_output<'a, W: Write + ?Sized>(
    stdout: &'a mut W,
    output_file: Option<&str>,
) -> Result<ConfiguredOutput<'a, W>, Diagnostic> {
    let Some(name) = output_file else {
        return Ok(ConfiguredOutput::Writer(stdout));
    };
    if name == "-" {
        return Ok(ConfiguredOutput::Writer(stdout));
    }

    let path = Path::new(name);
    File::create(path)
        .map(ConfiguredOutput::File)
        .map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot open file {}: {error}", path.display()),
            )
        })
}

#[allow(clippy::cast_sign_loss)]
const fn c_rlimit_from_arg(value: i64) -> u64 {
    // The C path assigns CLStateGetIntArg()'s signed long to rlim_t.
    value as u64
}

fn check_hard_soft_limits(
    hard: i64,
    soft: i64,
    hard_option_changed: bool,
) -> Result<(), Diagnostic> {
    if c_rlimit_from_arg(hard) > c_rlimit_from_arg(soft) {
        return Ok(());
    }
    let message = if hard_option_changed {
        "Hard time limit has to be larger than softtime limit"
    } else {
        "Soft time limit has to be smaller than hardtime limit"
    };
    Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
}

fn apply_time_limit_state(config: &EProverConfig) {
    if let Some(limit) = config.cpu_limit {
        let _ = set_hard_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.soft_cpu_limit {
        let _ = set_soft_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.schedule_time_limit {
        let _ = set_schedule_time_limit(c_rlimit_from_arg(limit));
    }
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<u8, EProverError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv)? {
        EProverAction::Help => {
            stdout.write_all(print_help().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Version => {
            stdout.write_all(version::version_line().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Run(config) => run_config(stdout, &config),
    }
}

pub fn process_options<I, S>(argv: I) -> Result<EProverAction, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EProverConfig::default();
    while let Some(parsed) = state.next_opt(EPROVER_OPTIONS)? {
        if let Some(action) = apply_parsed_option(&mut config, &parsed)? {
            return Ok(action);
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(EProverAction::Run(config))
}

fn apply_parsed_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<Option<EProverAction>, Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Help => Ok(Some(EProverAction::Help)),
        EProverOption::Version => Ok(Some(EProverAction::Version)),
        EProverOption::Verbose
        | EProverOption::Output
        | EProverOption::Silent
        | EProverOption::OutputLevel => {
            apply_output_option(config, parsed)?;
            Ok(None)
        }
        EProverOption::ProofObject
        | EProverOption::ProofGraph
        | EProverOption::ProofStatistics
        | EProverOption::FullDerivation
        | EProverOption::ForceDerivation
        | EProverOption::RecordGivenClauses
        | EProverOption::TrainingExamples => {
            apply_proof_option(config, parsed)?;
            Ok(None)
        }
        EProverOption::CpuLimit | EProverOption::SoftCpuLimit | EProverOption::MemoryLimit => {
            apply_resource_option(config, parsed)?;
            Ok(None)
        }
        EProverOption::SyntaxOnly
        | EProverOption::PrintFormulas
        | EProverOption::PruneOnly
        | EProverOption::CnfOnly => {
            apply_input_mode_option(config, parsed);
            Ok(None)
        }
        EProverOption::PrintPid
        | EProverOption::PrintVersion
        | EProverOption::RequireNonempty
        | EProverOption::Auto
        | EProverOption::DeterministicRewriteSort
        | EProverOption::DeterministicNewSort => {
            apply_simple_flag(config, parsed.option().option_code);
            Ok(None)
        }
    }
}

fn apply_output_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Verbose => {
            config.verbose = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::Output => {
            config.output_file = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::Silent => {
            config.output_level = 0;
        }
        EProverOption::OutputLevel => {
            config.output_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        _ => unreachable!("non-output option routed to output handler"),
    }
    Ok(())
}

fn apply_proof_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::ProofObject => {
            let level = get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 3)?;
            config.proof_object_level = config.proof_object_level.max(level);
            config.proof_output = config.proof_output.max(1);
        }
        EProverOption::ProofGraph => {
            let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            config.proof_object_level = config.proof_object_level.max(1);
            config.proof_output = level + 1;
        }
        EProverOption::ProofStatistics => config.flags.set(EProverFlag::ProofStatistics),
        EProverOption::FullDerivation => config.flags.set(EProverFlag::FullDerivation),
        EProverOption::ForceDerivation => {
            config.force_derivation_output =
                get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 2)?;
            config.proof_object_level = config.proof_object_level.max(1);
        }
        EProverOption::RecordGivenClauses => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
        }
        EProverOption::TrainingExamples => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
            config.training_examples =
                Some(get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?);
        }
        _ => unreachable!("non-proof option routed to proof handler"),
    }
    Ok(())
}

fn apply_resource_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::CpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(soft_limit) = config.soft_cpu_limit {
                check_hard_soft_limits(limit, soft_limit, true)?;
            }
            config.cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::SoftCpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(hard_limit) = config.cpu_limit {
                check_hard_soft_limits(hard_limit, limit, false)?;
            }
            config.soft_cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::MemoryLimit => {
            config.memory_limit = Some(parsed.arg().unwrap_or("").to_owned());
        }
        _ => unreachable!("non-resource option routed to resource handler"),
    }
    Ok(())
}

fn apply_input_mode_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::SyntaxOnly => config.flags.set(EProverFlag::SyntaxOnly),
        EProverOption::PrintFormulas => {
            config.flags.set(EProverFlag::SyntaxOnly);
            config.flags.set(EProverFlag::PrintFormulas);
        }
        EProverOption::PruneOnly => {
            config.output_level = 4;
            config.flags.set(EProverFlag::PruneOnly);
        }
        EProverOption::CnfOnly => config.flags.set(EProverFlag::CnfOnly),
        _ => unreachable!("non-input-mode option routed to input-mode handler"),
    }
}

fn apply_simple_flag(config: &mut EProverConfig, option: EProverOption) {
    match option {
        EProverOption::PrintPid => config.flags.set(EProverFlag::PrintPid),
        EProverOption::PrintVersion => config.flags.set(EProverFlag::PrintVersion),
        EProverOption::RequireNonempty => config.flags.set(EProverFlag::RequireNonempty),
        EProverOption::Auto => config.flags.set(EProverFlag::Auto),
        EProverOption::DeterministicRewriteSort => {
            config.flags.set(EProverFlag::DeterministicRewriteSort);
        }
        EProverOption::DeterministicNewSort => {
            config.flags.set(EProverFlag::DeterministicNewSort);
        }
        _ => unreachable!("non-simple flag routed to simple-flag handler"),
    }
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\nE {VERSION} \"{E_NICKNAME}\"\n\n\
Usage: {PROGRAM_NAME} [options] [files]\n\n\
Read a set of first-order (or, in the -ho-version, higher-order)\n\
clauses and formulae and try to prove the conjecture (if given)\n\
or show the set unsatisfiable.\n\n"
    );
    result.push_str(&print_options(EPROVER_OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&version::footer());
    result
}

fn run_config(stdout: &mut impl Write, config: &EProverConfig) -> Result<u8, EProverError> {
    let verbose = i32::try_from(config.verbose).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("--verbose argument {} is out of int range", config.verbose),
        )
    })?;
    set_verbose_level(verbose);
    let _ = set_output_level(config.output_level);
    apply_time_limit_state(config);
    let mut output = open_configured_output(stdout, config.output_file.as_deref())?;

    if config.flags.contains(EProverFlag::PrintPid) {
        writeln!(output, "# Pid: {}", std::process::id())?;
    }
    if config.flags.contains(EProverFlag::PrintVersion) {
        writeln!(output, "# Version: {VERSION}")?;
    }
    output.flush()?;

    Err(Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "Rust eprover proof search is not implemented yet",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{process_options, run, EProverAction, EProverFlag};
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::inout::output::{output_level, set_output_level};
    use crate::inout::signals::{
        hard_time_limit, schedule_time_limit, set_hard_time_limit, set_schedule_time_limit,
        set_soft_time_limit, soft_time_limit, RLIM_INFINITY_COMPAT,
    };
    use crate::prover::version::VERSION;
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("eprover-{name}-{}.out", std::process::id()))
    }

    #[test]
    fn process_options_recognizes_version_action() {
        let action = process_options(["eprover", "--version"]).unwrap();
        assert_eq!(action, EProverAction::Version);
    }

    #[test]
    fn process_options_keeps_non_option_files_and_inserts_stdin_default() {
        let action = process_options(["eprover", "a.p", "--silent", "b.p"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 0);
        assert_eq!(config.files, ["a.p", "b.p"]);

        let action = process_options(["eprover", "--silent"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.files, ["-"]);
    }

    #[test]
    fn process_options_tracks_cpu_limit_schedule_state_like_c() {
        let action = process_options(["eprover", "--cpu-limit"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(300));
        assert_eq!(config.soft_cpu_limit, None);
        assert_eq!(config.schedule_time_limit, Some(300));

        let action =
            process_options(["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(100));
        assert_eq!(config.soft_cpu_limit, Some(25));
        assert_eq!(config.schedule_time_limit, Some(100));

        let action =
            process_options(["eprover", "--cpu-limit=100", "--soft-cpu-limit=25"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_time_limit, Some(25));
    }

    #[test]
    fn process_options_records_input_mode_flags_like_c() {
        let action = process_options(["eprover", "--print-formulas"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::SyntaxOnly));
        assert!(config.flags.contains(EProverFlag::PrintFormulas));

        let action = process_options(["eprover", "--prune"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 4);
        assert!(config.flags.contains(EProverFlag::PruneOnly));

        let action = process_options(["eprover", "--cnf", "--error-on-empty"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::CnfOnly));
        assert!(config.flags.contains(EProverFlag::RequireNonempty));
    }

    #[test]
    fn process_options_records_proof_output_state_like_c() {
        let action = process_options(["eprover", "--proof-object=3", "--proof-object=1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 3);
        assert_eq!(config.proof_output, 1);

        let action = process_options(["eprover", "--proof-graph=2"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 1);
        assert_eq!(config.proof_output, 3);

        let action =
            process_options(["eprover", "-d", "--proof-statistics", "--record-gcs"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::FullDerivation));
        assert!(config.flags.contains(EProverFlag::ProofStatistics));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);

        let action =
            process_options(["eprover", "--force-deriv=2", "--training-examples=3"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.force_derivation_output, 2);
        assert_eq!(config.training_examples, Some(3));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);
    }

    #[test]
    fn process_options_rejects_force_derivation_outside_c_range() {
        let error = process_options(["eprover", "--force-deriv=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_rejects_non_increasing_cpu_limits() {
        let error =
            process_options(["eprover", "--cpu-limit=10", "--soft-cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Soft time limit has to be smaller than hardtime limit"
        );

        let error =
            process_options(["eprover", "--soft-cpu-limit=20", "--cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Hard time limit has to be larger than softtime limit"
        );
    }

    #[test]
    fn run_version_prints_c_compatible_version_line() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "E 3.3.5 Countess Grey (facc36eaf92d70896d830140efc4382df9e8dcdb)\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_help_prints_usage() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-h"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Usage: eprover [options] [files]"));
        assert!(output.contains("--version"));
    }

    #[test]
    fn run_applies_verbose_option_to_global_gate() {
        let _guard = global_test_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--verbose=2"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(verbose_level(), 2);
        set_verbose_level(0);
    }

    #[test]
    fn run_rejects_verbose_values_outside_c_int_range() {
        let _guard = global_test_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--verbose=2147483648"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("out of int range"));
        assert_eq!(verbose_level(), 0);
    }

    #[test]
    fn run_applies_cpu_limit_options_to_signal_state() {
        let _guard = global_test_lock();
        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(hard_time_limit(), 100);
        assert_eq!(soft_time_limit(), 25);
        assert_eq!(schedule_time_limit(), 100);

        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
    }

    #[test]
    fn run_applies_output_level_options_to_global_gate() {
        let _guard = global_test_lock();
        let _ = set_output_level(1);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--silent"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 0);

        let error = run(["eprover", "--output-level=3"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 3);
        let _ = set_output_level(1);
    }

    #[test]
    fn run_print_info_uses_configured_output_target() {
        let _guard = global_test_lock();
        let path = temp_path("print-info");
        let _ = std::fs::remove_file(&path);
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--print-version", "-o", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(stdout.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("# Version: {VERSION}\n")
        );
        std::fs::remove_file(&path).unwrap();

        let error = run(
            ["eprover", "--print-version", "-o", "-"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("# Version: {VERSION}\n")
        );
    }
}
