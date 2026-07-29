use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout, verbout_arg};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::fileops::{copy_file, file_find_base_name};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::Scanner;
use crate::learn::annoterms::{anno_set_parse_clause_patterns, anno_set_print_string};
use crate::learn::examplerep::{
    example_set_find_name, example_set_parse, example_set_print_string, ExampleSet,
};
use crate::learn::kbdesc::KB_ANNOTATION_NO;
use crate::learn::kbinsert::{kb_parse_example_file, kb_pattern_signature};
use crate::prover::version::{footer, VERSION};
use crate::terms::termbanks::TermBank;
use std::io::{Read, Write};
use std::path::Path;

pub const PROGRAM_NAME: &str = "umlaut-kb-insert";
const DEFAULT_KB_NAME: &str = "E_KNOWLEDGE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    KnowledgeBase,
    Name,
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
        OptionCode::Name,
        Some('n'),
        Some("name"),
        OptArgType::ReqArg,
        None,
        "Give the name of the new problem. If not given, the program will take the name of the first input file, or, if <stdin> is read, a name of the form '__problem__i', where i is magically computed  from the existing knowledge base.",
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
struct EkbInsertConfig {
    kb_name: String,
    ex_name: Option<String>,
    input_files: Vec<String>,
}

impl Default for EkbInsertConfig {
    fn default() -> Self {
        Self {
            kb_name: DEFAULT_KB_NAME.to_owned(),
            ex_name: None,
            input_files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EkbInsertConfig),
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
    stdin: &mut impl Read,
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
    let result = run_inner(argv, stdin, stdout, stderr);
    exit_io();
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    result
}

fn run_inner<I, S>(
    argv: I,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_ekb_insert(&config, stdin, stderr),
    }
}

fn process_options<I, S>(command_line: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(command_line);
    let mut config = EkbInsertConfig::default();

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
            OptionCode::Name => {
                config.ex_name = Some(parsed.arg().unwrap_or("").to_owned());
            }
        }
    }

    config.input_files = state.remaining_args().to_vec();
    Ok(RunCommand::Execute(config))
}

fn execute_ekb_insert(
    config: &EkbInsertConfig,
    stdin: &mut impl Read,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut proof_examples = ExampleSet::new();
    let problems_path = kb_path(&config.kb_name, "problems");
    let mut problems_scanner = Scanner::from_file(Path::new(&problems_path), true)?;
    example_set_parse(&mut problems_scanner, &mut proof_examples)?;

    let mut reserved_symbols = kb_pattern_signature();
    let signature_path = kb_path(&config.kb_name, "signature");
    let mut signature_scanner = Scanner::from_file(Path::new(&signature_path), true)?;
    reserved_symbols.parse_declarations(&mut signature_scanner, true)?;

    let mut internal_terms = TermBank::new(reserved_symbols)?;
    let clausepatterns_path = kb_path(&config.kb_name, "clausepatterns");
    let mut clausepatterns_scanner = Scanner::from_file(Path::new(&clausepatterns_path), true)?;
    let mut clause_examples = anno_set_parse_clause_patterns(
        &mut clausepatterns_scanner,
        &mut internal_terms,
        KB_ANNOTATION_NO,
    )?;

    verbout_diag(stderr, "Old knowledge base files parsed successfully\n")?;

    let input_files = input_files_with_stdin_default(&config.input_files);
    let mut override_name = config.ex_name.clone();
    for input_file in &input_files {
        let ex_name = select_example_name(&mut override_name, input_file, &proof_examples);
        if example_set_find_name(&proof_examples, &ex_name).is_some() {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!("Example name '{ex_name}' already in use"),
            ));
        }
        verbout_arg_diag(stderr, "New example will use name ", &ex_name)?;

        let store_file = stored_example_path(&config.kb_name, &ex_name);
        copy_input_file(&store_file, input_file, stdin)?;

        let mut scanner = Scanner::from_file(Path::new(&store_file), true)?;
        let mut parse_terms = TermBank::new(internal_terms.signature().clone())?;
        kb_parse_example_file(
            &mut scanner,
            ex_name,
            &mut proof_examples,
            &mut clause_examples,
            &mut parse_terms,
            &mut internal_terms,
            ProblemType::FirstOrder,
        )?;
    }

    write_kb_file(
        &config.kb_name,
        "clausepatterns",
        &anno_set_print_string(&clause_examples, &internal_terms),
    )?;
    write_kb_file(
        &config.kb_name,
        "problems",
        &example_set_print_string(&proof_examples),
    )?;

    Ok(0)
}

fn input_files_with_stdin_default(input_files: &[String]) -> Vec<String> {
    if input_files.is_empty() {
        vec!["-".to_owned()]
    } else {
        input_files.to_vec()
    }
}

fn select_example_name(
    override_name: &mut Option<String>,
    input_file: &str,
    proof_examples: &ExampleSet,
) -> String {
    if let Some(name) = override_name.take() {
        return name;
    }
    if input_file != "-" {
        return file_find_base_name(input_file).to_owned();
    }
    format!("__problem__{}", proof_examples.count() + 1)
}

fn copy_input_file(target: &str, source: &str, stdin: &mut impl Read) -> Result<(), Diagnostic> {
    if source == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read <stdin>: {error}")))?;
        std::fs::write(Path::new(target), data)
            .map_err(|error| io_diagnostic(format!("Cannot write file {target}: {error}")))?;
        Ok(())
    } else {
        copy_file(Path::new(target), Path::new(source)).map(|_count| ())
    }
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
Usage: {PROGRAM_NAME} [options] [names]\n\
\n\
Insert example files into an Umlaut knowledge base. Each non-option argument\n\
is considered as one individual example file. For most applications\n\
this is obsolete, use umlaut-kb-ginsert instead.\n\n"
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
        print_help, process_options, run, stored_example_path, EkbInsertConfig, RunCommand,
        DEFAULT_KB_NAME, PROGRAM_NAME,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("ekb-insert-{name}-{}.tmp", std::process::id()))
    }

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir_all(path);
    }

    fn example_source(constant: &str) -> String {
        format!(
            "\
{constant}=b.
.
0:(0): {constant}=b.
"
        )
    }

    fn create_empty_kb(kb_path: &Path) {
        remove_dir_if_present(kb_path);
        std::fs::create_dir(kb_path).expect("kb directory is created");
        std::fs::create_dir(kb_path.join("FILES")).expect("FILES directory is created");
        std::fs::write(kb_path.join("signature"), "").expect("signature file is written");
        std::fs::write(kb_path.join("problems"), "").expect("problems file is written");
        std::fs::write(kb_path.join("clausepatterns"), "").expect("clausepatterns file is written");
    }

    fn run_with_args(args: &[&str], stdin_data: &str) -> Result<(u8, String, String), Diagnostic> {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)?;
        Ok((
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        ))
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) =
            run_with_args(&[PROGRAM_NAME, "--help"], "").expect("help succeeds");

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: umlaut-kb-insert [options] [names]"));
        assert!(help.contains("use umlaut-kb-ginsert instead"));
        assert!(help.contains("-V"));
        assert!(stderr.is_empty());

        let (status, version, stderr) =
            run_with_args(&[PROGRAM_NAME, "-V"], "").expect("version succeeds");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_kb_name_override_inputs_and_verbose() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--name=explicit",
                "--knowledge-base=KB",
                "a.p",
                "b.p",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(EkbInsertConfig {
            kb_name,
            ex_name,
            input_files,
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, "KB");
        assert_eq!(ex_name.as_deref(), Some("explicit"));
        assert_eq!(input_files, ["a.p", "b.p"]);
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());

        let mut stdout = Vec::new();
        let command = process_options([PROGRAM_NAME], &mut stdout).expect("options parse");
        let RunCommand::Execute(EkbInsertConfig {
            kb_name,
            input_files,
            ..
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, DEFAULT_KB_NAME);
        assert!(input_files.is_empty());
    }

    #[test]
    fn stdin_input_uses_default_problem_name_and_rewrites_kb_files() {
        let _guard = global_state_lock();
        let kb_path = temp_path("stdin-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let source = example_source("a");

        let (status, stdout, stderr) =
            run_with_args(&[PROGRAM_NAME, &kb_option], &source).expect("insert succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(
            std::fs::read_to_string(kb_path.join("FILES").join("__problem__1"))
                .expect("stored stdin example is readable"),
            source
        );
        assert!(std::fs::read_to_string(kb_path.join("problems"))
            .expect("problems is readable")
            .contains("1: \"__problem__1\""));
        let clausepatterns =
            std::fs::read_to_string(kb_path.join("clausepatterns")).expect("patterns readable");
        assert!(clausepatterns.starts_with("\n% Annotated terms:\n"));
        assert!(clausepatterns.contains("1:(1.000000,1.000000,0.000000)"));

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn file_inputs_use_name_override_once_then_file_basename() {
        let _guard = global_state_lock();
        let kb_path = temp_path("file-kb");
        let sources_dir = temp_path("sources");
        create_empty_kb(&kb_path);
        remove_dir_if_present(&sources_dir);
        std::fs::create_dir(&sources_dir).expect("sources directory is created");
        let first = sources_dir.join("first.p");
        let second = sources_dir.join("second.p");
        std::fs::write(&first, example_source("a")).expect("first source is written");
        std::fs::write(&second, example_source("c")).expect("second source is written");
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let first_arg = first.to_string_lossy().replace('\\', "/");
        let second_arg = second.to_string_lossy().replace('\\', "/");

        let (status, stdout, stderr) = run_with_args(
            [
                PROGRAM_NAME,
                "--name=explicit",
                &kb_option,
                &first_arg,
                &second_arg,
            ]
            .as_slice(),
            "",
        )
        .expect("insert succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(
            std::fs::read_to_string(kb_path.join("FILES").join("explicit"))
                .expect("explicit stored file is readable"),
            example_source("a")
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("FILES").join("second.p"))
                .expect("basename stored file is readable"),
            example_source("c")
        );
        let problems = std::fs::read_to_string(kb_path.join("problems")).expect("problems read");
        assert!(problems.contains("1: \"explicit\""));
        assert!(problems.contains("2: \"second.p\""));

        remove_dir_if_present(&kb_path);
        remove_dir_if_present(&sources_dir);
    }

    #[test]
    fn duplicate_example_name_is_rejected_before_file_copy() {
        let _guard = global_state_lock();
        let kb_path = temp_path("duplicate-kb");
        create_empty_kb(&kb_path);
        std::fs::write(
            kb_path.join("problems"),
            "1: \"dup\"\nPA: () FA: () (0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n",
        )
        .expect("existing problems file is written");
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");

        let mut stdin = Cursor::new(example_source("a").into_bytes());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [PROGRAM_NAME, "--name=dup", &kb_option],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Example name 'dup' already in use");
        assert!(!kb_path.join("FILES").join("dup").exists());
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn malformed_stdin_leaves_c_copy_before_parse_orphan() {
        let _guard = global_state_lock();
        let kb_path = temp_path("malformed-copy-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let malformed_source = "this is not a valid learned example\n";

        let mut stdin = Cursor::new(malformed_source.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [PROGRAM_NAME, &kb_option],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            std::fs::read_to_string(kb_path.join("FILES").join("__problem__1"))
                .expect("stored malformed file is readable"),
            malformed_source
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("problems")).expect("problems is readable"),
            ""
        );
        assert_eq!(
            std::fs::read_to_string(kb_path.join("clausepatterns"))
                .expect("clausepatterns is readable"),
            ""
        );
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn verbose_run_reports_c_progress_messages() {
        let _guard = global_state_lock();
        let kb_path = temp_path("verbose-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let stored = stored_example_path(kb_arg, "__problem__1");

        let (status, stdout, stderr) = run_with_args(
            [PROGRAM_NAME, "--verbose", &kb_option].as_slice(),
            &example_source("a"),
        )
        .expect("insert succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            format!(
                "umlaut-kb-insert: Old knowledge base files parsed successfully\n\
umlaut-kb-insert: New example will use name __problem__1\n"
            )
        );
        assert_eq!(
            std::fs::read_to_string(Path::new(&stored)).expect("stored file is readable"),
            example_source("a")
        );

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn print_help_mentions_obsolete_guidance() {
        assert!(print_help().contains("use umlaut-kb-ginsert instead"));
    }
}
