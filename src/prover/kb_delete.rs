use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout, verbout_arg};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::Scanner;
use crate::learn::annoterms::{anno_set_parse_clause_patterns, anno_set_print_string};
use crate::learn::examplerep::{
    example_set_delete_id, example_set_find_name, example_set_parse, example_set_print_string,
    ExampleSet,
};
use crate::learn::kbdesc::KB_ANNOTATION_NO;
use crate::learn::kbinsert::kb_pattern_signature;
use crate::prover::version::{footer, VERSION};
use crate::terms::termbanks::TermBank;
use std::io::{Read, Write};
use std::path::Path;

pub const PROGRAM_NAME: &str = "umlaut-kb-delete";
const DEFAULT_KB_NAME: &str = "E_KNOWLEDGE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    KnowledgeBase,
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
        None,
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
        OptionCode::KnowledgeBase,
        Some('k'),
        Some("knowledge-base"),
        OptArgType::ReqArg,
        None,
        "Select the knowledge base. If not given, select E_KNOWLEDGE.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EkbDeleteConfig {
    kb_name: String,
    ex_name: String,
}

impl Default for EkbDeleteConfig {
    fn default() -> Self {
        Self {
            kb_name: DEFAULT_KB_NAME.to_owned(),
            ex_name: String::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EkbDeleteConfig),
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
        RunCommand::Execute(config) => execute_ekb_delete(&config, stderr),
    }
}

fn process_options<I, S>(command_line: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(command_line);
    let mut config = EkbDeleteConfig::default();

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
            OptionCode::KnowledgeBase => {
                parsed.arg().unwrap_or("").clone_into(&mut config.kb_name);
            }
        }
    }

    let args = state.remaining_args();
    if args.len() != 1 {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "One argument (name of the problem to remove) required",
        ));
    }
    config.ex_name.clone_from(&args[0]);

    Ok(RunCommand::Execute(config))
}

fn execute_ekb_delete(config: &EkbDeleteConfig, stderr: &mut impl Write) -> Result<u8, Diagnostic> {
    let mut proof_examples = ExampleSet::new();
    let problems_path = kb_path(&config.kb_name, "problems");
    let mut problems_scanner = Scanner::from_file(Path::new(&problems_path), true)?;
    example_set_parse(&mut problems_scanner, &mut proof_examples)?;

    let mut bank = TermBank::new(kb_pattern_signature())?;
    let clausepatterns_path = kb_path(&config.kb_name, "clausepatterns");
    let mut clausepatterns_scanner = Scanner::from_file(Path::new(&clausepatterns_path), true)?;
    let mut clause_examples =
        anno_set_parse_clause_patterns(&mut clausepatterns_scanner, &mut bank, KB_ANNOTATION_NO)?;

    verbout_diag(stderr, "Old knowledge base files parsed successfully\n")?;

    let Some(to_delete) = example_set_find_name(&proof_examples, &config.ex_name) else {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Example name '{}' does not exist in knowledge base",
                config.ex_name
            ),
        ));
    };
    let ident = to_delete.ident();

    clause_examples.remove_by_ident(ident);
    example_set_delete_id(&mut proof_examples, ident);

    let store_file = stored_example_path(&config.kb_name, &config.ex_name);
    remove_file(&store_file, stderr)?;

    write_kb_file(
        &config.kb_name,
        "clausepatterns",
        &anno_set_print_string(&clause_examples, &bank),
    )?;
    write_kb_file(
        &config.kb_name,
        "problems",
        &example_set_print_string(&proof_examples),
    )?;

    Ok(0)
}

fn remove_file(path: &str, stderr: &mut impl Write) -> Result<(), Diagnostic> {
    verbout_arg_diag(stderr, "Removing ", path)?;
    std::fs::remove_file(Path::new(path))
        .map_err(|error| io_diagnostic(format!("Cannot remove file {path}: {error}")))
}

fn write_kb_file(basename: &str, file: &str, contents: &str) -> Result<(), Diagnostic> {
    let path = kb_path(basename, file);
    std::fs::write(Path::new(&path), contents)
        .map_err(|error| io_diagnostic(format!("Cannot write file {path}: {error}")))
}

fn kb_path(basename: &str, file: &str) -> String {
    format!("{basename}/{file}")
}

fn stored_example_path(basename: &str, ex_name: &str) -> String {
    format!("{basename}/FILES/{ex_name}")
}

fn verbout_diag(output: &mut impl Write, message: &str) -> Result<(), Diagnostic> {
    let _ =
        verbout(output, PROGRAM_NAME, message).map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(())
}

fn verbout_arg_diag(output: &mut impl Write, first: &str, second: &str) -> Result<(), Diagnostic> {
    let _ = verbout_arg(output, PROGRAM_NAME, first, second)
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(())
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
\n\
{PROGRAM_NAME} {VERSION}\n\
\n\
Usage: {PROGRAM_NAME} [options] <name>\n\
\n\
Remove the example <name> from an Umlaut knowledge base.\n\n"
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
        print_help, process_options, run, stored_example_path, EkbDeleteConfig, RunCommand,
        DEFAULT_KB_NAME, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("ekb-delete-{name}-{}.tmp", std::process::id()))
    }

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir_all(path);
    }

    fn run_with_args(args: &[&str]) -> (u8, String, String) {
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("umlaut-kb-delete run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    fn feature_source(first_value: f64) -> String {
        let mut values = [0.0; crate::learn::numfeatures::FEATURE_NUMBER];
        values[0] = first_value;
        let mut result = String::from("PA: () FA: () (");
        result.push_str(&values[0].to_string());
        for value in &values[1..] {
            result.push_str(", ");
            result.push_str(&value.to_string());
        }
        result.push(')');
        result
    }

    fn problem_entry(ident: i64, name: &str, first_value: f64) -> String {
        format!("{ident}: \"{name}\"\n{}\n", feature_source(first_value))
    }

    fn create_test_kb(kb_path: &Path) {
        remove_dir_if_present(kb_path);
        std::fs::create_dir(kb_path).expect("kb directory is created");
        std::fs::create_dir(kb_path.join("FILES")).expect("FILES directory is created");
        std::fs::write(
            kb_path.join("problems"),
            format!(
                "% Example names and features. \n{}{}",
                problem_entry(1, "drop", 1.0),
                problem_entry(2, "keep", 2.0)
            ),
        )
        .expect("problems file is written");
        std::fs::write(
            kb_path.join("clausepatterns"),
            "% Individual annotated patterns. \n\
p(a) : 1:(1,0,0,0,0,0,0),2:(1,0,0,0,0,0,0).\n\
q(a) : 1:(1,0,0,0,0,0,0).\n\
r(a) : 2:(1,0,0,0,0,0,0).\n",
        )
        .expect("clausepatterns file is written");
        std::fs::write(kb_path.join("FILES").join("drop"), "drop problem")
            .expect("drop example file is written");
        std::fs::write(kb_path.join("FILES").join("keep"), "keep problem")
            .expect("keep example file is written");
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_args(&[PROGRAM_NAME, "--help"]);

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: umlaut-kb-delete [options] <name>"));
        assert!(help.contains("Remove the example <name>"));
        assert!(help.contains("--knowledge-base"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_args(&[PROGRAM_NAME, "--version"]);
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_defaults_kb_and_verbose() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--knowledge-base=KB",
                "example",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(EkbDeleteConfig { kb_name, ex_name }) = command else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, "KB");
        assert_eq!(ex_name, "example");
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());

        let mut stdout = Vec::new();
        let command =
            process_options([PROGRAM_NAME, "example"], &mut stdout).expect("options parse");
        let RunCommand::Execute(EkbDeleteConfig { kb_name, .. }) = command else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, DEFAULT_KB_NAME);
    }

    #[test]
    fn invalid_arg_count_matches_c_diagnostic() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "One argument (name of the problem to remove) required"
        );

        let error = process_options([PROGRAM_NAME, "one", "two"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "One argument (name of the problem to remove) required"
        );
    }

    #[test]
    fn deletes_example_and_rewrites_kb_files() {
        let _guard = global_state_lock();
        let kb_path = temp_path("kb");
        create_test_kb(&kb_path);

        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let (status, stdout, stderr) = run_with_args(&[PROGRAM_NAME, &kb_option, "drop"]);

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert!(!kb_path.join("FILES").join("drop").exists());
        assert_eq!(
            std::fs::read_to_string(kb_path.join("FILES").join("keep"))
                .expect("kept file is readable"),
            "keep problem"
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("problems")).expect("problems is readable"),
            "2: \"keep\"\n\
PA: ()  FA: ()\n\
(2.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000)\n\n"
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("clausepatterns"))
                .expect("clausepatterns is readable"),
            "\n% Annotated terms:\n\
p(a) : 2:(1.000000,0.000000,0.000000,0.000000,0.000000,0.000000,0.000000).\n\
r(a) : 2:(1.000000,0.000000,0.000000,0.000000,0.000000,0.000000,0.000000).\n"
        );

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn unknown_example_is_rejected_before_file_removal() {
        let _guard = global_state_lock();
        let kb_path = temp_path("unknown-kb");
        create_test_kb(&kb_path);

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let kb_option = format!(
            "--knowledge-base={}",
            kb_path.to_str().expect("path is utf8")
        );
        let error = run(
            [PROGRAM_NAME, &kb_option, "missing"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Example name 'missing' does not exist in knowledge base"
        );
        assert!(kb_path.join("FILES").join("drop").exists());
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn verbose_run_reports_parse_and_remove_messages() {
        let _guard = global_state_lock();
        let kb_path = temp_path("verbose-kb");
        create_test_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let removed_path = stored_example_path(kb_arg, "drop");

        let kb_option = format!("--knowledge-base={kb_arg}");
        let (status, stdout, stderr) =
            run_with_args(&[PROGRAM_NAME, "--verbose", &kb_option, "drop"]);

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            format!(
                "umlaut-kb-delete: Old knowledge base files parsed successfully\n\
umlaut-kb-delete: Removing {removed_path}\n"
            )
        );

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn print_help_mentions_delete_action() {
        assert!(print_help().contains("Remove the example"));
    }
}
