use std::fmt;
use std::io::{self, Write};

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::commandline::{
    get_int_arg, get_int_arg_check_range, print_options, CommandLineState,
};
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
    pub cpu_limit: Option<i64>,
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
            cpu_limit: None,
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
        match parsed.option().option_code {
            EProverOption::Help => return Ok(EProverAction::Help),
            EProverOption::Version => return Ok(EProverAction::Version),
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
            EProverOption::ProofObject => {
                config.proof_object_level =
                    get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 3)?;
            }
            EProverOption::CpuLimit => {
                config.cpu_limit = Some(get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?);
            }
            EProverOption::MemoryLimit => {
                config.memory_limit = Some(parsed.arg().unwrap_or("").to_owned());
            }
            EProverOption::SyntaxOnly => {
                config.flags.set(EProverFlag::SyntaxOnly);
            }
            EProverOption::PrintPid => {
                config.flags.set(EProverFlag::PrintPid);
            }
            EProverOption::PrintVersion => {
                config.flags.set(EProverFlag::PrintVersion);
            }
            EProverOption::Auto => {
                config.flags.set(EProverFlag::Auto);
            }
            EProverOption::DeterministicRewriteSort => {
                config.flags.set(EProverFlag::DeterministicRewriteSort);
            }
            EProverOption::DeterministicNewSort => {
                config.flags.set(EProverFlag::DeterministicNewSort);
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(EProverAction::Run(config))
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
    if config.flags.contains(EProverFlag::PrintPid) {
        writeln!(stdout, "# Pid: {}", std::process::id())?;
    }
    if config.flags.contains(EProverFlag::PrintVersion) {
        writeln!(stdout, "# Version: {VERSION}")?;
    }

    Err(Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "Rust eprover proof search is not implemented yet",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{process_options, run, EProverAction};
    use crate::basics::error::ErrorCode;

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
}
