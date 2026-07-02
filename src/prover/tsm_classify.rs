use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{Scanner, TokenType};
use crate::learn::annoterms::{anno_set_compute_pattern_subst, anno_set_parse};
use crate::learn::classification::tsm_classify_set_write;
use crate::learn::flatannoterms::{
    flat_anno_set_alloc, flat_anno_set_size, flat_anno_set_translate,
};
use crate::learn::indexfunctions::{get_index_type, IndexType, INDEX_FUN_NAMES};
use crate::learn::patterns::PatternSubst;
use crate::learn::tsm::{
    get_tsm_type, tsm_admin_alloc, tsm_admin_build_tsm, tsm_compute_average_eval,
    tsm_compute_classification_limit, TsmType,
};
use crate::prover::version::{footer, VERSION};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "tsm_classify";
const DEFAULT_WEIGHTS: [f64; 6] = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    OutputLevel,
    Output,
    IndexFun,
    IndexDepth,
    TsmType,
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
        OptionCode::OutputLevel,
        Some('l'),
        Some("output-level"),
        OptArgType::ReqArg,
        None,
        "Select an output level, greater values imply more verbose output.",
    ),
    OptCell::new(
        OptionCode::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file.",
    ),
    OptCell::new(
        OptionCode::IndexFun,
        Some('i'),
        Some("index-type"),
        OptArgType::ReqArg,
        None,
        "Select an index function type. Run tsm_classify -iNone for a list of possible functions.",
    ),
    OptCell::new(
        OptionCode::IndexDepth,
        Some('d'),
        Some("index-depth"),
        OptArgType::ReqArg,
        None,
        "Set the term top depth for the index. A depth of 0 denotes dynamic depth selection.",
    ),
    OptCell::new(
        OptionCode::TsmType,
        Some('t'),
        Some("tsm-type"),
        OptArgType::ReqArg,
        None,
        "Select the type of the TSM (Flat, Recursive, Reccurent or RecLocal).",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct TsmClassifyConfig {
    output_file: Option<PathBuf>,
    index_type: IndexType,
    index_depth: i32,
    tsm_type: TsmType,
    files: Vec<String>,
}

impl Default for TsmClassifyConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            index_type: IndexType::ARITY,
            index_depth: 1,
            tsm_type: TsmType::Recursive,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(TsmClassifyConfig),
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
    let _ = set_output_level(1);
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
        RunCommand::Execute(config) => execute_tsm_classify(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = TsmClassifyConfig::default();

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
            OptionCode::OutputLevel => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                let _ = set_output_level(level);
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::IndexFun => {
                config.index_type = parse_index_type(parsed.arg().unwrap_or(""), stdout)?;
            }
            OptionCode::IndexDepth => {
                let depth = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if depth < 0 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Argument for -d (--index-depth) has to be an integer number greater than or equal to 0.",
                    ));
                }
                config.index_depth = i64_to_i32_saturating(depth);
            }
            OptionCode::TsmType => {
                config.tsm_type = parse_tsm_type(parsed.arg().unwrap_or(""))?;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn parse_index_type(arg: &str, stdout: &mut impl Write) -> Result<IndexType, Diagnostic> {
    let Some(index_type) = get_index_type(arg) else {
        writeln_diag(stdout, "% Index type: -1")?;
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Wrong argument to option -i (--index-type). Possible values: {}",
                INDEX_FUN_NAMES.join(", ")
            ),
        ));
    };
    writeln_diag(stdout, &format!("% Index type: {}", index_type.bits()))?;
    if index_type == IndexType::NO_INDEX {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Sorry, need to select a real index type!",
        ));
    }
    Ok(index_type)
}

fn parse_tsm_type(arg: &str) -> Result<TsmType, Diagnostic> {
    match get_tsm_type(arg) {
        Some(tsm_type @ (TsmType::Flat | TsmType::Recursive | TsmType::Recurrent | TsmType::RecurrentLocal)) => Ok(tsm_type),
        Some(TsmType::NoType) | None => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Only Flat, Recursive, Recurrent and RecLocal allowed asTSM types in option -t (--tsm-type)",
        )),
    }
}

fn execute_tsm_classify(
    config: &TsmClassifyConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut summary_file = open_output_file(config.output_file.as_deref())?;
    let input = concat_inputs(&config.files, stdin)?;
    let mut scanner = Scanner::from_file_content("tsm_classify-input", input, true)?;

    let mut bank = TermBank::new(Signature::new(TypeBank::new()))?;

    scanner.accept_id("Training")?;
    scanner.accept_tok(TokenType::COLON)?;
    let mut training_set = anno_set_parse(&mut scanner, &mut bank, 2)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    training_set.flatten(None);

    scanner.accept_id("Test")?;
    scanner.accept_tok(TokenType::COLON)?;
    let mut test_set = anno_set_parse(&mut scanner, &mut bank, 2)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    test_set.flatten(None);

    bank.signature_mut().set_all_special(true);
    let mut ftrain_set = flat_anno_set_alloc();
    let mut ftest_set = flat_anno_set_alloc();
    flat_anno_set_translate(&mut ftrain_set, &training_set, &DEFAULT_WEIGHTS);
    flat_anno_set_translate(&mut ftest_set, &test_set, &DEFAULT_WEIGHTS);

    let mut admin = tsm_admin_alloc(bank.signature().clone(), config.tsm_type)?;
    verbout_diag(stderr, "Parsing and preprocessing done\n")?;

    let mut subst = PatternSubst::default_subst(bank.signature());
    anno_set_compute_pattern_subst(&mut subst, &training_set);
    anno_set_compute_pattern_subst(&mut subst, &test_set);
    verbout_diag(stderr, "PatternSubst generated\n")?;

    tsm_admin_build_tsm(
        &mut admin,
        &ftrain_set,
        config.index_type,
        config.index_depth,
        subst,
    )?;
    let eval_limit = tsm_compute_classification_limit(&mut admin, &ftrain_set);
    admin.set_eval_limit(eval_limit);
    let unmapped_eval = tsm_compute_average_eval(&mut admin, &ftrain_set);
    admin.set_unmapped_eval(unmapped_eval);
    verbout_diag(stderr, "TSM build\n")?;

    let mut classification_trace = String::new();
    let successes = tsm_classify_set_write(&mut admin, &ftest_set, &mut classification_trace)
        .map_err(|error| io_diagnostic(error.to_string()))?;
    write_all(stdout, classification_trace.as_bytes())?;
    let nodes = flat_anno_set_size(&ftest_set);
    let percent = success_percent(successes, nodes);
    let summary = format!(
        "{} terms, {successes} successes, {percent:5.3} percent",
        c_space_signed(nodes)
    );
    write_summary(summary_file.as_mut(), stdout, &summary)?;

    verbout_diag(stderr, "TSM freed\n")?;
    flush_summary(summary_file.as_mut(), stdout)?;
    Ok(0)
}

fn concat_inputs(files: &[String], stdin: &mut impl Read) -> Result<Vec<u8>, Diagnostic> {
    let mut result = Vec::new();
    for file in files {
        if file == "-" {
            stdin
                .read_to_end(&mut result)
                .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        } else {
            let mut data = std::fs::read(Path::new(file))
                .map_err(|error| io_diagnostic(format!("Cannot read file {file}: {error}")))?;
            result.append(&mut data);
        }
    }
    Ok(result)
}

#[allow(clippy::cast_precision_loss)]
fn success_percent(successes: i64, nodes: i64) -> f64 {
    100.0 * successes as f64 / nodes as f64
}

fn c_space_signed(value: i64) -> String {
    if value >= 0 {
        format!(" {value}")
    } else {
        value.to_string()
    }
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
Usage: {PROGRAM_NAME} [options] [files]\n\
\n\
Parse a classification problem specification file and return\n\
results. This is an experimental program and does not have all the\n\
usual error checking and hand holding features as E proper!\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

fn open_output_file(path: Option<&Path>) -> Result<Option<std::fs::File>, Diagnostic> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path == Path::new("-") {
        return Ok(None);
    }
    std::fs::File::create(path)
        .map(Some)
        .map_err(|error| io_diagnostic(format!("Cannot open file {}: {error}", path.display())))
}

fn write_summary(
    file: Option<&mut std::fs::File>,
    stdout: &mut impl Write,
    line: &str,
) -> Result<(), Diagnostic> {
    if let Some(file) = file {
        writeln_diag(file, line)
    } else {
        writeln_diag(stdout, line)
    }
}

fn flush_summary(
    file: Option<&mut std::fs::File>,
    stdout: &mut impl Write,
) -> Result<(), Diagnostic> {
    let result = if let Some(file) = file {
        file.flush()
    } else {
        stdout.flush()
    };
    result.map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))
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
        c_space_signed, parse_tsm_type, print_help, process_options, run, success_percent,
        RunCommand, TsmClassifyConfig, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::output::output_level;
    use crate::learn::indexfunctions::IndexType;
    use crate::learn::tsm::TsmType;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    const CLASSIFICATION_INPUT: &str = "\
Training:
a : 1:(1,-1).
f(a) : 2:(1,1).
.
Test:
a : 1:(1,-1).
f(a) : 2:(1,1).
.
";
    const CLASSIFICATION_TRACE: &str = "\
Evaluation: -1.0000  Termeval: -1.0000 OKOK a
Evaluation:  1.0000  Termeval:  1.0000 OKOK f(a)
";

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("tsm-classify-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("tsm_classify run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_stdin(&[PROGRAM_NAME, "--help"], "ignored");

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: tsm_classify [options] [files]"));
        assert!(help.contains("experimental program"));
        assert!(help.contains("--tsm-type"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "--version"], "ignored");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_learning_options_and_prints_index_type() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--output-level=4",
                "--index-type=IndexSymbol",
                "--index-depth=3",
                "--tsm-type=Flat",
                "problem.tsm",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(TsmClassifyConfig {
            index_type,
            index_depth,
            tsm_type,
            files,
            ..
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(index_type, IndexType::SYMBOL);
        assert_eq!(index_depth, 3);
        assert_eq!(tsm_type, TsmType::Flat);
        assert_eq!(files, ["problem.tsm"]);
        assert_eq!(verbose_level(), 2);
        assert_eq!(output_level(), 4);
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            "% Index type: 2\n"
        );
    }

    #[test]
    fn invalid_index_options_keep_c_partial_output() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME, "--index-type=None"], &mut stdout).unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("Wrong argument to option -i"));
        assert!(error.message().contains("IndexArity"));
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            "% Index type: -1\n"
        );

        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME, "-i", "IndexNoIndex"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Sorry, need to select a real index type!");
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            "% Index type: 0\n"
        );
    }

    #[test]
    fn invalid_depth_and_tsm_type_match_c_diagnostics() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME, "--index-depth=-1"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Argument for -d (--index-depth) has to be an integer number greater than or equal to 0."
        );

        let error = parse_tsm_type("NoType").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Only Flat, Recursive, Recurrent and RecLocal allowed asTSM types in option -t (--tsm-type)"
        );
    }

    #[test]
    fn stdin_run_classifies_test_set() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--index-type=IndexIdentity",
                "--tsm-type=Flat",
            ],
            CLASSIFICATION_INPUT,
        );

        assert_eq!(status, 0);
        assert_eq!(
            output,
            format!(
                "% Index type: 64\n{CLASSIFICATION_TRACE} 2 terms, 2 successes, 100.000 percent\n"
            )
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_receives_summary_after_concatenated_inputs() {
        let _guard = global_state_lock();
        let input_a_path = temp_path("input-a");
        let input_b_path = temp_path("input-b");
        let output_path = temp_path("output");
        remove_if_present(&input_a_path);
        remove_if_present(&input_b_path);
        remove_if_present(&output_path);
        std::fs::write(
            &input_a_path,
            "Training:\na : 1:(1,-1).\nf(a) : 2:(1,1).\n.\n",
        )
        .expect("first input fixture is written");
        std::fs::write(&input_b_path, "Test:\na : 1:(1,-1).\nf(a) : 2:(1,1).\n.\n")
            .expect("second input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--index-type=IndexIdentity",
                "--tsm-type=Flat",
                "-o",
                output_path.to_str().expect("path is utf8"),
                input_a_path.to_str().expect("path is utf8"),
                input_b_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("file run succeeds");

        assert_eq!(status, 0);
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            format!("% Index type: 64\n{CLASSIFICATION_TRACE}")
        );
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert_eq!(output, " 2 terms, 2 successes, 100.000 percent\n");

        remove_if_present(&input_a_path);
        remove_if_present(&input_b_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn verbose_run_reports_c_progress_messages() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--verbose",
                "--index-type=IndexIdentity",
                "--tsm-type=Flat",
            ],
            CLASSIFICATION_INPUT,
        );

        assert_eq!(status, 0);
        assert!(output.ends_with(" 2 terms, 2 successes, 100.000 percent\n"));
        assert_eq!(
            stderr,
            "tsm_classify: Parsing and preprocessing done\n\
tsm_classify: PatternSubst generated\n\
tsm_classify: TSM build\n\
tsm_classify: TSM freed\n"
        );
    }

    #[test]
    fn helpers_preserve_c_summary_format_edges() {
        assert_eq!(c_space_signed(0), " 0");
        assert_eq!(c_space_signed(42), " 42");
        assert_eq!(c_space_signed(-1), "-1");
        assert!(success_percent(1, 0).is_infinite());
    }

    #[test]
    fn print_help_mentions_experimental_c_tool_status() {
        assert!(print_help().contains("experimental program"));
    }
}
