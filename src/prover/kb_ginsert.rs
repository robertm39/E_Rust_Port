use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout, verbout2, verbout_arg};
use crate::clauses::clause::ClauseParseOptions;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::fileops::file_find_base_name;
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::learn::annoterms::{anno_set_parse_clause_patterns, anno_set_print_string};
use crate::learn::examplerep::{
    example_set_find_name, example_set_parse, example_set_print_string, ExampleSet,
};
use crate::learn::kbdesc::{kb_desc_parse, KbDesc, KB_ANNOTATION_NO};
use crate::learn::kbinsert::{kb_parse_example_file, kb_pattern_signature};
use crate::pcl2::analysis::{
    protocol_proof_distance, protocol_select_examples, protocol_update_grefs,
};
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStepParseOptions, PCL_IS_INITIAL, PCL_IS_PROOF_STEP};
use crate::prover::version::{footer, VERSION};
use crate::terms::termbanks::TermBank;
use std::io::{Read, Write};
use std::path::Path;

pub const PROGRAM_NAME: &str = "umlaut-kb-ginsert";
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
struct EkbGinsertConfig {
    kb_name: String,
    ex_name: Option<String>,
    input_files: Vec<String>,
}

impl Default for EkbGinsertConfig {
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
    Execute(EkbGinsertConfig),
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
    let _ = set_output_level(0);
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
        RunCommand::Execute(config) => execute_ekb_ginsert(&config, stdin, stderr),
    }
}

fn process_options<I, S>(command_line: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(command_line);
    let mut config = EkbGinsertConfig::default();

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

    let remaining_args = state.remaining_args();
    if config.ex_name.is_none() {
        if let Some(first_input) = remaining_args.first().filter(|name| name.as_str() != "-") {
            config.ex_name = Some(file_find_base_name(first_input).to_owned());
        }
    }
    config.input_files = if remaining_args.is_empty() {
        vec!["-".to_owned()]
    } else {
        remaining_args.to_vec()
    };
    Ok(RunCommand::Execute(config))
}

fn execute_ekb_ginsert(
    config: &EkbGinsertConfig,
    stdin: &mut impl Read,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let (mut proof_examples, kb_desc) = read_existing_parameters(&config.kb_name)?;
    verbout_diag(stderr, "Parameter files parsed successfully\n")?;

    let ex_name = config
        .ex_name
        .clone()
        .unwrap_or_else(|| format!("__problem__{}", proof_examples.count() + 1));
    if example_set_find_name(&proof_examples, &ex_name).is_some() {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Example name '{ex_name}' already in use"),
        ));
    }
    verbout_arg_diag(stderr, "New example will use name ", &ex_name)?;

    verbout_diag(stderr, "Generating training examples\n")?;
    let store_file = stored_example_path(&config.kb_name, &ex_name);
    generate_training_example(&store_file, &config.input_files, &kb_desc, stdin, stderr)?;

    verbout_diag(stderr, "Parsing data files\n")?;
    let mut internal_terms = parse_reserved_data(&config.kb_name)?;
    let clausepatterns_path = kb_path(&config.kb_name, "clausepatterns");
    let mut clausepatterns_scanner = Scanner::from_file(Path::new(&clausepatterns_path), true)?;
    let mut clause_examples = anno_set_parse_clause_patterns(
        &mut clausepatterns_scanner,
        &mut internal_terms,
        KB_ANNOTATION_NO,
    )?;

    verbout_diag(stderr, "Integrating new examples\n")?;
    let mut generated_scanner = Scanner::from_file(Path::new(&store_file), true)?;
    let mut parse_terms = TermBank::new(internal_terms.signature().clone())?;
    kb_parse_example_file(
        &mut generated_scanner,
        ex_name,
        &mut proof_examples,
        &mut clause_examples,
        &mut parse_terms,
        &mut internal_terms,
        ProblemType::FirstOrder,
    )?;

    verbout_diag(stderr, "Writing example files\n")?;
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

fn read_existing_parameters(basename: &str) -> Result<(ExampleSet, KbDesc), Diagnostic> {
    let mut proof_examples = ExampleSet::new();
    let problems_path = kb_path(basename, "problems");
    let mut problems_scanner = Scanner::from_file(Path::new(&problems_path), true)?;
    example_set_parse(&mut problems_scanner, &mut proof_examples)?;

    let description_path = kb_path(basename, "description");
    let mut description_scanner = Scanner::from_file(Path::new(&description_path), true)?;
    let kb_desc = kb_desc_parse(&mut description_scanner)?;

    Ok((proof_examples, kb_desc))
}

fn parse_reserved_data(basename: &str) -> Result<TermBank, Diagnostic> {
    let mut reserved_symbols = kb_pattern_signature();
    let signature_path = kb_path(basename, "signature");
    let mut signature_scanner = Scanner::from_file(Path::new(&signature_path), true)?;
    reserved_symbols.parse_declarations(&mut signature_scanner, true)?;
    TermBank::new(reserved_symbols)
}

fn generate_training_example(
    target: &str,
    input_files: &[String],
    kb_desc: &KbDesc,
    stdin: &mut impl Read,
    stderr: &mut impl Write,
) -> Result<(), Diagnostic> {
    let mut protocol = PclProtocol::new()?;
    for input_file in input_files {
        let mut scanner = scanner_for_input(input_file, stdin)?;
        protocol.parse(&mut scanner, parse_options())?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }
    verbout2_diag(stderr, "PCL input read\n")?;

    protocol.strip_fof()?;
    protocol.reset_tree_data(false);
    protocol.mark_proof_clauses()?;
    protocol_proof_distance(&mut protocol)?;
    protocol_update_grefs(&mut protocol);
    let proof_steps = protocol.count_property(PCL_IS_PROOF_STEP);
    let neg_steps = negative_example_budget(kb_desc, proof_steps);
    let _visited = protocol_select_examples(&mut protocol, neg_steps);

    let mut generated = String::new();
    generated.push_str(DEFAULT_COMCHAR_RAW);
    generated.push_str(" Axioms:\n");
    generated.push_str(&protocol.print_property_steps_string(
        PCL_IS_INITIAL,
        ProofDocOutputFormat::Lop,
        ProblemType::FirstOrder,
    )?);
    generated.push_str(".\n\n");
    generated.push_str(DEFAULT_COMCHAR_RAW);
    generated.push_str(" Examples:\n");
    generated.push_str(&protocol.print_examples_string()?);

    std::fs::write(Path::new(target), generated)
        .map_err(|error| io_diagnostic(format!("Cannot write file {target}: {error}")))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn negative_example_budget(kb_desc: &KbDesc, proof_steps: i64) -> i64 {
    if proof_steps == 0 {
        kb_desc.fail_neg_examples()
    } else {
        (kb_desc.neg_proportion() * proof_steps as f64) as i64
    }
}

fn parse_options() -> PclStepParseOptions {
    PclStepParseOptions {
        problem_type: ProblemType::FirstOrder,
        support_shell_pcl: false,
        clause_parse_options: ClauseParseOptions {
            clauses_have_local_variables: false,
            ..ClauseParseOptions::default()
        },
    }
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    let mut scanner = if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, true)?
    } else {
        Scanner::from_file(Path::new(name), true)?
    };
    scanner.set_format(IoFormat::Tptp);
    Ok(scanner)
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

fn verbout2_diag(output: &mut impl Write, message: &str) -> Result<(), Diagnostic> {
    let _ = verbout2(output, PROGRAM_NAME, message)
        .map_err(|error| io_diagnostic(error.to_string()))?;
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
Usage: {PROGRAM_NAME} [options] [name]\n\
\n\
Generate a set of training examples from an E-compatible inference list (i.e. an\n\
EPCL trace of a proof run) and insert it into a knowledge base.\n\n"
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
        parse_options, print_help, process_options, run, stored_example_path, EkbGinsertConfig,
        RunCommand, DEFAULT_KB_NAME, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::output::{output_level, set_output_level};
    use crate::learn::kbdesc::{KbDesc, KB_VERSION};
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("ekb-ginsert-{name}-{}.tmp", std::process::id()))
    }

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir_all(path);
    }

    fn create_empty_kb(kb_path: &Path) {
        remove_dir_if_present(kb_path);
        std::fs::create_dir(kb_path).expect("kb directory is created");
        std::fs::create_dir(kb_path.join("FILES")).expect("FILES directory is created");
        std::fs::write(kb_path.join("signature"), "").expect("signature file is written");
        std::fs::write(kb_path.join("problems"), "").expect("problems file is written");
        std::fs::write(kb_path.join("clausepatterns"), "").expect("clausepatterns file is written");
        std::fs::write(
            kb_path.join("description"),
            KbDesc::new(KB_VERSION, 1.0, 2).print_string(),
        )
        .expect("description file is written");
    }

    fn protocol_source() -> &'static str {
        "\
1 : : [++p(a)] : initial : 'proof'
2 : : [++q(a)] : initial
3 : : [++r(a)] : 2
"
    }

    fn run_with_args(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("umlaut-kb-ginsert run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn run_resets_global_output_level_to_zero_like_c() {
        let _guard = global_state_lock();
        let _ = set_output_level(7);

        let (status, _help, stderr) = run_with_args(&[PROGRAM_NAME, "--help"], "");

        assert_eq!(status, 0);
        assert_eq!(output_level(), 0);
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_args(&[PROGRAM_NAME, "--help"], "");

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: umlaut-kb-ginsert [options] [name]"));
        assert!(help.contains("Generate a set of training examples"));
        assert!(help.contains("-V"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_args(&[PROGRAM_NAME, "-V"], "");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_name_kb_inputs_stdin_default_and_verbose() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--name=explicit",
                "--knowledge-base=KB",
                "a.pcl",
                "b.pcl",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(EkbGinsertConfig {
            kb_name,
            ex_name,
            input_files,
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, "KB");
        assert_eq!(ex_name.as_deref(), Some("explicit"));
        assert_eq!(input_files, ["a.pcl", "b.pcl"]);
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());

        let mut stdout = Vec::new();
        let command = process_options([PROGRAM_NAME], &mut stdout).expect("options parse");
        let RunCommand::Execute(EkbGinsertConfig {
            kb_name,
            ex_name,
            input_files,
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(kb_name, DEFAULT_KB_NAME);
        assert!(ex_name.is_none());
        assert_eq!(input_files, ["-"]);
    }

    #[test]
    fn first_file_basename_is_selected_before_stdin_default() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command =
            process_options([PROGRAM_NAME, "dir/trace.pcl"], &mut stdout).expect("options parse");

        let RunCommand::Execute(EkbGinsertConfig { ex_name, .. }) = command else {
            panic!("expected execute command");
        };
        assert_eq!(ex_name.as_deref(), Some("trace.pcl"));
    }

    #[test]
    fn stdin_protocol_generates_default_example_and_rewrites_kb_files() {
        let _guard = global_state_lock();
        let kb_path = temp_path("stdin-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");

        let (status, stdout, stderr) =
            run_with_args(&[PROGRAM_NAME, &kb_option], protocol_source());

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let stored = std::fs::read_to_string(kb_path.join("FILES").join("__problem__1"))
            .expect("stored generated example is readable");
        assert!(stored.starts_with("% Axioms:\n"));
        assert!(stored.contains("p(a) <- ."));
        assert!(stored.contains(".\n\n% Examples:\n"));
        assert!(stored.contains("0:(0,"));
        assert!(std::fs::read_to_string(kb_path.join("problems"))
            .expect("problems is readable")
            .contains("1: \"__problem__1\""));
        let clausepatterns =
            std::fs::read_to_string(kb_path.join("clausepatterns")).expect("patterns readable");
        assert!(clausepatterns.starts_with("\n% Annotated terms:\n"));
        assert!(clausepatterns.contains("1:("));

        let (status, stdout, stderr) = run_with_args(
            &[PROGRAM_NAME, "--name=second", &kb_option],
            protocol_source(),
        );
        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert!(std::fs::read_to_string(kb_path.join("problems"))
            .expect("updated problems are readable")
            .contains("2: \"second\""));
        assert!(kb_path.join("FILES").join("second").is_file());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn generated_protocol_examples_share_external_variable_names_like_c() {
        let _guard = global_state_lock();
        let kb_path = temp_path("shared-vars-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");

        let (status, stdout, stderr) = run_with_args(
            &[PROGRAM_NAME, "--name=shared_vars", &kb_option],
            "1 : : [++p(X,Y)] : initial\n2 : : [++q(Y,X)] : initial\n",
        );

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let stored = std::fs::read_to_string(kb_path.join("FILES").join("shared_vars"))
            .expect("stored generated example is readable");
        assert!(stored.contains("p(X1,X2) <- ."));
        assert!(stored.contains("q(X2,X1) <- ."));

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn parse_options_disable_local_clause_variables_like_c() {
        assert!(
            !parse_options()
                .clause_parse_options
                .clauses_have_local_variables
        );
    }

    #[test]
    fn file_input_uses_first_file_basename_for_generated_example() {
        let _guard = global_state_lock();
        let kb_path = temp_path("file-kb");
        let sources_dir = temp_path("sources");
        create_empty_kb(&kb_path);
        remove_dir_if_present(&sources_dir);
        std::fs::create_dir(&sources_dir).expect("sources directory is created");
        let source = sources_dir.join("trace.pcl");
        std::fs::write(&source, protocol_source()).expect("source is written");
        let source_arg = source.to_string_lossy().replace('\\', "/");
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");

        let (status, stdout, stderr) = run_with_args(&[PROGRAM_NAME, &kb_option, &source_arg], "");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert!(kb_path.join("FILES").join("trace.pcl").exists());
        assert!(std::fs::read_to_string(kb_path.join("problems"))
            .expect("problems is readable")
            .contains("1: \"trace.pcl\""));

        remove_dir_if_present(&kb_path);
        remove_dir_if_present(&sources_dir);
    }

    #[test]
    fn duplicate_name_is_rejected_before_generation() {
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

        let mut stdin = Cursor::new(protocol_source().as_bytes().to_vec());
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
    fn verbose_run_reports_c_progress_messages() {
        let _guard = global_state_lock();
        let kb_path = temp_path("verbose-kb");
        create_empty_kb(&kb_path);
        let kb_arg = kb_path.to_str().expect("path is utf8");
        let kb_option = format!("--knowledge-base={kb_arg}");
        let stored = stored_example_path(kb_arg, "__problem__1");

        let (status, stdout, stderr) = run_with_args(
            [PROGRAM_NAME, "--verbose=2", &kb_option].as_slice(),
            protocol_source(),
        );

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert_eq!(
            stderr,
            "umlaut-kb-ginsert: Parameter files parsed successfully\n\
umlaut-kb-ginsert: New example will use name __problem__1\n\
umlaut-kb-ginsert: Generating training examples\n\
umlaut-kb-ginsert: PCL input read\n\
umlaut-kb-ginsert: Parsing data files\n\
umlaut-kb-ginsert: Integrating new examples\n\
umlaut-kb-ginsert: Writing example files\n"
        );
        assert!(Path::new(&stored).exists());

        remove_dir_if_present(&kb_path);
    }

    #[test]
    fn print_help_mentions_protocol_generation() {
        assert!(print_help().contains("EPCL trace of a proof run"));
    }
}
