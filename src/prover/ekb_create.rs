use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout};
use crate::inout::commandline::{
    get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::learn::kbdesc::{kb_desc_alloc, kb_desc_print_string, KB_VERSION};
use crate::prover::version::{footer, VERSION};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "ekb_create";
const DEFAULT_KB_NAME: &str = "E_KNOWLEDGE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    NegNo,
    NegProp,
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
        "Print the version number of the program.",
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
        OptionCode::NegNo,
        Some('n'),
        Some("negative-example-number"),
        OptArgType::ReqArg,
        None,
        "Set the (maximum) number of negative examples to pick if the proof listing does not describe a successful proof.",
    ),
    OptCell::new(
        OptionCode::NegProp,
        Some('p'),
        Some("negative-example-proportion"),
        OptArgType::ReqArg,
        None,
        "Set the maximum number of negative examples (expressed as a proportion of the positive examples) to pick if the proof listing does describe a successful proof",
    ),
];

#[derive(Clone, Debug, PartialEq)]
struct EkbCreateConfig {
    basename: String,
    neg_proportion: f64,
    neg_examples: i64,
}

impl Default for EkbCreateConfig {
    fn default() -> Self {
        Self {
            basename: DEFAULT_KB_NAME.to_owned(),
            neg_proportion: 1.0,
            neg_examples: 0,
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EkbCreateConfig),
    Exit(u8),
}

struct ProblemTypeRunGuard;

impl ProblemTypeRunGuard {
    fn new() -> Self {
        reset_problem_type();
        Self
    }
}

impl Drop for ProblemTypeRunGuard {
    fn drop(&mut self) {
        reset_problem_type();
    }
}

pub fn run<I, S>(
    argv: I,
    _stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let _problem_type_guard = ProblemTypeRunGuard::new();
    init_io(PROGRAM_NAME);
    set_problem_type(ProblemType::FirstOrder)?;
    set_verbose_level(0);
    let result = run_inner(argv, stdout, stderr);
    exit_io();
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    result
}

fn run_inner<I, S>(
    argv: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_ekb_create(&config, stderr),
    }
}

fn process_options<I, S>(command_line: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(command_line);
    let mut config = EkbCreateConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Verbose => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                set_verbose_level(i64_to_i32_saturating(level));
            }
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION}"))?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::NegNo => {
                config.neg_examples = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::NegProp => {
                let value = get_float_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if value < 0.0 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Option -p (--negative-example-proportion)requires positive argument.}",
                    ));
                }
                config.neg_proportion = value;
            }
        }
    }

    let args = state.remaining_args();
    if args.len() > 1 {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Only one non-option argument (name of the knowledge base) expected",
        ));
    }
    if let Some(basename) = args.first() {
        config.basename.clone_from(basename);
    }

    Ok(RunCommand::Execute(config))
}

fn execute_ekb_create(config: &EkbCreateConfig, stderr: &mut impl Write) -> Result<u8, Diagnostic> {
    if config.basename == DEFAULT_KB_NAME {
        verbout_diag(stderr, "Using default name\n")?;
    }
    verbout_diag(stderr, "Creating base directory...\n")?;
    create_base_dir(&config.basename)?;
    verbout_diag(stderr, "...successful.\nCreating files...\n")?;

    let description = kb_desc_print_string(&kb_desc_alloc(
        KB_VERSION,
        config.neg_proportion,
        config.neg_examples,
    ));
    write_kb_file(&config.basename, "description", &description)?;
    write_kb_file(
        &config.basename,
        "signature",
        &format!(
            "{DEFAULT_COMCHAR_RAW} Special function symbols that are not generalized.\n\
{DEFAULT_COMCHAR_RAW} You need to hand-hack this at the moment.\n"
        ),
    )?;
    write_kb_file(
        &config.basename,
        "problems",
        &format!("{DEFAULT_COMCHAR_RAW} Example names and features. \n"),
    )?;
    write_kb_file(
        &config.basename,
        "clausepatterns",
        &format!("{DEFAULT_COMCHAR_RAW} Individual annotated patterns. \n"),
    )?;

    verbout_diag(stderr, "...done.\nCreating subdirectory FILES...\n")?;
    create_files_dir(&config.basename)?;
    verbout_diag(stderr, "...done.\nNew knowledge base complete.\n")?;
    Ok(0)
}

fn create_base_dir(basename: &str) -> Result<(), Diagnostic> {
    std::fs::create_dir(Path::new(basename)).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!("Cannot create base directory '{basename}': {error}"),
        )
    })
}

fn create_files_dir(basename: &str) -> Result<(), Diagnostic> {
    let path = kb_path(basename, "FILES");
    std::fs::create_dir(Path::new(&path)).map_err(|error| {
        Diagnostic::new(
            ErrorCode::FILE_ERROR,
            format!("Cannot create base directory '{basename}': {error}"),
        )
    })
}

fn write_kb_file(basename: &str, file: &str, contents: &str) -> Result<(), Diagnostic> {
    let path = kb_path(basename, file);
    std::fs::write(Path::new(&path), contents)
        .map_err(|error| io_diagnostic(format!("Cannot write file {path}: {error}")))
}

fn kb_path(basename: &str, file: &str) -> String {
    let mut path = PathBuf::from(basename);
    path.push(file);
    path.to_string_lossy().into_owned()
}

fn verbout_diag(output: &mut impl Write, message: &str) -> Result<(), Diagnostic> {
    let _ =
        verbout(output, PROGRAM_NAME, message).map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(())
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
\n\
{PROGRAM_NAME} {VERSION}\n\
\n\
Usage: {PROGRAM_NAME} [options] [<name>]\n\
\n\
Create an empty knowledge base with name <name> for E.\n\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
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

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{
        print_help, process_options, run, EkbCreateConfig, RunCommand, DEFAULT_KB_NAME,
        PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::learn::kbdesc::KB_VERSION;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("ekb-create-{name}-{}.tmp", std::process::id()))
    }

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir_all(path);
    }

    fn run_with_args(args: &[&str]) -> (u8, String, String) {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("ekb_create run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_args(&[PROGRAM_NAME, "--help"]);

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: ekb_create [options] [<name>]"));
        assert!(help.contains("Create an empty knowledge base"));
        assert!(help.contains("--negative-example-proportion"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_args(&[PROGRAM_NAME, "-V"]);
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_defaults_and_learning_knobs() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose",
                "--negative-example-number=-4",
                "--negative-example-proportion=0.25",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(EkbCreateConfig {
            basename,
            neg_proportion,
            neg_examples,
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(basename, DEFAULT_KB_NAME);
        assert_close(neg_proportion, 0.25);
        assert_eq!(neg_examples, -4);
        assert_eq!(verbose_level(), 1);
        assert!(stdout.is_empty());
    }

    #[test]
    fn invalid_args_match_c_diagnostics() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options(
            [PROGRAM_NAME, "--negative-example-proportion=-0.01", "kb"],
            &mut stdout,
        )
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option -p (--negative-example-proportion)requires positive argument.}"
        );

        let error = process_options([PROGRAM_NAME, "one", "two"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Only one non-option argument (name of the knowledge base) expected"
        );
    }

    #[test]
    fn creates_empty_knowledge_base_files() {
        let _guard = global_state_lock();
        let kb_path = temp_path("kb");
        remove_dir_if_present(&kb_path);

        let kb_arg = kb_path.to_str().expect("path is utf8");
        let (status, stdout, stderr) = run_with_args(&[
            PROGRAM_NAME,
            "--negative-example-number=7",
            "--negative-example-proportion=0.5",
            kb_arg,
        ]);

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert!(kb_path.is_dir());
        assert!(kb_path.join("FILES").is_dir());
        assert_eq!(
            std::fs::read_to_string(kb_path.join("description")).expect("description is readable"),
            format!(
                "% E theorem prover knowledge base description\n\
Version     : \"{KB_VERSION}\"\n\
NegProp     : 0.500000  % Negative example proportion (successful proof search)\n\
FailExamples:        7  % Number of clauses from a failed proof search\n"
            )
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("signature")).expect("signature is readable"),
            "% Special function symbols that are not generalized.\n\
% You need to hand-hack this at the moment.\n"
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("problems")).expect("problems is readable"),
            "% Example names and features. \n"
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("clausepatterns"))
                .expect("clausepatterns is readable"),
            "% Individual annotated patterns. \n"
        );

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn verbose_run_reports_c_progress_messages() {
        let _guard = global_state_lock();
        let kb_path = temp_path("verbose-kb");
        remove_dir_if_present(&kb_path);

        let kb_arg = kb_path.to_str().expect("path is utf8");
        let (status, stdout, stderr) = run_with_args(&[PROGRAM_NAME, "--verbose", kb_arg]);

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            "ekb_create: Creating base directory...\n\
ekb_create: ...successful.\n\
Creating files...\n\
ekb_create: ...done.\n\
Creating subdirectory FILES...\n\
ekb_create: ...done.\n\
New knowledge base complete.\n"
        );

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn existing_directory_is_rejected_like_base_mkdir_failure() {
        let _guard = global_state_lock();
        let kb_path = temp_path("existing-kb");
        remove_dir_if_present(&kb_path);
        std::fs::create_dir(&kb_path).expect("temporary kb directory exists");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [PROGRAM_NAME, kb_path.to_str().expect("path is utf8")],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Cannot create base directory"));
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn print_help_mentions_empty_knowledge_base() {
        assert!(print_help().contains("Create an empty knowledge base"));
    }
}
