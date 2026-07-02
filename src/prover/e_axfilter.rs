use std::fs::File;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use crate::basics::error::{check_option_letter_string, Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::basics::verbose::set_verbose_level;
use crate::clauses::sine::{pstack_clause_print_tstp_string, pstack_formula_print_tstp_string};
use crate::control::batch_spec::BatchSpec;
use crate::control::sine::StructFofSpec;
use crate::heuristics::axfilter::{AxFilter, AxFilterSet};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell, ParsedOpt,
};
use crate::inout::fileops::file_name_strip;
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner};
use crate::prover::version::{footer, E_NICKNAME, VERSION};
use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};

pub const PROGRAM_NAME: &str = "e_axfilter";
const C_USAGE_ERROR: &str = "Usage: e_axfilter <problem> [<options>]\n";
const SEEDED_FILTERING_PENDING: &str = "e_axfilter artificial seed filtering is not yet ported";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Filter,
    SeedSymbols,
    Seeds,
    SeedSubsample,
    SeedMethod,
    DumpFilter,
    Silent,
    OutputLevel,
    LopParse,
    LopFormat,
    TptpParse,
    TptpFormat,
    TstpParse,
    TstpFormat,
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
        "Verbose comments on the progress of the program. This technical information is printed to stderr.",
    ),
    OptCell::new(
        OptionCode::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file (this affects only some output, as most is written to automatically generated files based on the input and filter names.",
    ),
    OptCell::new(
        OptionCode::Silent,
        Some('s'),
        Some("silent"),
        OptArgType::NoArg,
        None,
        "Equivalent to --output-level=0.",
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
        OptionCode::Filter,
        Some('f'),
        Some("filter"),
        OptArgType::ReqArg,
        None,
        "Specify the filter definition file. If not set, the system will uses the built-in default.",
    ),
    OptCell::new(
        OptionCode::SeedSymbols,
        Some('S'),
        Some("seed-symbols"),
        OptArgType::OptArg,
        Some("p"),
        "Enable artificial seeding of the axiom selection process and determine which symbol classes should be used to generate different sets.The argument is a string of letters, each indicating one class of symbols to use. 'p' indicates predicate symbols, 'f' non-constant function symbols, and 'c' constants. Note that this will create potentially multiple output files for each activated symbols.",
    ),
    OptCell::new(
        OptionCode::Seeds,
        None,
        Some("seeds"),
        OptArgType::ReqArg,
        None,
        "Explicitly specify the symbols that should be used as seed symbols for axiom extraction. This overwrites --seed-subsample and --seed-symbols.",
    ),
    OptCell::new(
        OptionCode::SeedSubsample,
        None,
        Some("seed-subsample"),
        OptArgType::OptArg,
        Some("r1000"),
        "Subsample from the set of eligible seed symbols. The argument is a one-character designator for the method ('m' uses the symbols that occur in the most input formulas, 'l' uses the symbols that occur in the least number of formulas, and 'r' samples randomly), followed by the number of symbols to select.",
    ),
    OptCell::new(
        OptionCode::SeedMethod,
        Some('m'),
        Some("seed-method"),
        OptArgType::OptArg,
        Some("lda"),
        "Specify how to select seed axioms when artificially seeding is used.The argument is a string of letters, each indicating one method to use. The letters are: \n'l': use the syntactically largest axiom in which the seed symbol occurs.\n'd': use the most diverse axiom in which the seed symbol occurs, i.e. the symbol with the largest set of different symbols.\n'a': use all axioms in which the seed symbol occurs.\nFor 'l' and 'd', if there are multiple candidates, use the first one.If the option is not set, 'a' is assumed.",
    ),
    OptCell::new(
        OptionCode::DumpFilter,
        Some('d'),
        Some("dump-filter"),
        OptArgType::NoArg,
        None,
        "Print the filter definition in force.",
    ),
    OptCell::new(
        OptionCode::LopParse,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Set E-LOP as the input format. If no input format is selected by this or one of the following options, E will guess the input format based on the first token. It will almost always correctly recognize TPTP-3, but it may misidentify E-LOP files that use TPTP meta-identifiers as logical symbols.",
    ),
    OptCell::new(
        OptionCode::LopFormat,
        None,
        Some("lop-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --lop-in.",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-2 format instead of E-LOP (but note that includes are handled according to TPTP-3 semantics).",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp2-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still under development, and the version in E may not be fully conforming at all times. E works on all TPTP 6.3.0 FOF and CNF input files (including includes).",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SubsampleMethod {
    #[default]
    None,
    Most,
    Least,
    Random,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "The fields mirror e_axfilter.c process-wide option globals."
)]
struct EAxFilterConfig {
    output_file: Option<PathBuf>,
    filter_file: Option<PathBuf>,
    parse_format: IoFormat,
    verbose_level: i64,
    output_level: i64,
    dump_filter: bool,
    seed_preds: bool,
    seed_funs: bool,
    seed_consts: bool,
    seed_large: bool,
    seed_diverse: bool,
    seed_all: bool,
    seedstr: Option<String>,
    subsample: SubsampleMethod,
    sample_size: i64,
    files: Vec<String>,
}

impl Default for EAxFilterConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            filter_file: None,
            parse_format: IoFormat::Auto,
            verbose_level: 0,
            output_level: 1,
            dump_filter: false,
            seed_preds: false,
            seed_funs: false,
            seed_consts: false,
            seed_large: false,
            seed_diverse: false,
            seed_all: true,
            seedstr: None,
            subsample: SubsampleMethod::None,
            sample_size: i64::MAX,
            files: Vec::new(),
        }
    }
}

impl EAxFilterConfig {
    #[must_use]
    const fn seed_filtering_requested(&self) -> bool {
        self.seed_preds || self.seed_funs || self.seed_consts || self.seedstr.is_some()
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EAxFilterConfig),
    Exit(u8),
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl IoWrite,
    stderr: &mut impl IoWrite,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    init_io(PROGRAM_NAME);
    set_verbose_level(0);
    let result = run_inner(argv, stdout, stderr);
    exit_io();
    stdout
        .flush()
        .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(format!("Cannot flush stderr: {error}")))?;
    result
}

fn run_inner<I, S>(
    argv: I,
    stdout: &mut impl IoWrite,
    stderr: &mut impl IoWrite,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout, stderr)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_config(&config, stdout),
    }
}

fn process_options<I, S>(
    argv: I,
    stdout: &mut impl IoWrite,
    _stderr: &mut impl IoWrite,
) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EAxFilterConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Verbose => {
                config.verbose_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Silent => config.output_level = 0,
            OptionCode::OutputLevel => {
                config.output_level =
                    get_int_arg(parsed.option(), required_arg(&parsed, "output-level")?)?;
            }
            OptionCode::Filter => {
                config.filter_file = Some(PathBuf::from(required_arg(&parsed, "filter")?));
            }
            OptionCode::SeedMethod => {
                let arg = parsed.arg().unwrap_or("lda");
                config.seed_all = false;
                check_option_letter_string(arg, "lda", "-m (--seed-methods)")?;
                for byte in arg.bytes() {
                    match byte {
                        b'l' => config.seed_large = true,
                        b'd' => config.seed_diverse = true,
                        b'a' => config.seed_all = true,
                        _ => unreachable!("validated option letter"),
                    }
                }
            }
            OptionCode::Seeds => {
                config.seedstr = Some(required_arg(&parsed, "seeds")?.to_owned());
            }
            OptionCode::SeedSubsample => {
                let (subsample, sample_size) =
                    parse_seed_subsample_arg(parsed.arg().unwrap_or("r1000"))?;
                config.subsample = subsample;
                config.sample_size = sample_size;
            }
            OptionCode::SeedSymbols => {
                let arg = parsed.arg().unwrap_or("p");
                check_option_letter_string(arg, "pfc", "-S (--seed-symbols)")?;
                for byte in arg.bytes() {
                    match byte {
                        b'p' => config.seed_preds = true,
                        b'f' => config.seed_funs = true,
                        b'c' => config.seed_consts = true,
                        _ => unreachable!("validated option letter"),
                    }
                }
            }
            OptionCode::DumpFilter => config.dump_filter = true,
            OptionCode::LopParse | OptionCode::LopFormat => {
                config.parse_format = IoFormat::Lop;
            }
            OptionCode::TptpParse | OptionCode::TptpFormat => {
                config.parse_format = IoFormat::Tptp;
            }
            OptionCode::TstpParse | OptionCode::TstpFormat => {
                config.parse_format = IoFormat::Tstp;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    Ok(RunCommand::Execute(config))
}

fn execute_config(config: &EAxFilterConfig, stdout: &mut impl IoWrite) -> Result<u8, Diagnostic> {
    apply_global_options(config);
    let mut output_file = open_output_file(config.output_file.as_deref())?;
    if let Some(output_file) = output_file.as_mut() {
        execute_with_output(config, output_file)?;
        output_file
            .flush()
            .map_err(|error| io_diagnostic(format!("Cannot flush output file: {error}")))?;
    } else {
        execute_with_output(config, stdout)?;
    }
    Ok(ErrorCode::NO_ERROR.exit_status())
}

fn execute_with_output<W: IoWrite + ?Sized>(
    config: &EAxFilterConfig,
    output: &mut W,
) -> Result<(), Diagnostic> {
    let filters = load_filters(config.filter_file.as_deref())?;
    if config.dump_filter {
        write_all(output, filters.print_string().as_bytes())?;
    }

    if config.files.is_empty() {
        return Err(Diagnostic::new(ErrorCode::USAGE_ERROR, C_USAGE_ERROR));
    }

    let (mut bank, mut ctrl, _parsed) =
        init_struct_fof_spec(config.parse_format, &config.files, output)?;
    if config.seed_filtering_requested() {
        return Err(Diagnostic::new(
            ErrorCode::INTERFACE_ERROR,
            SEEDED_FILTERING_PENDING,
        ));
    }

    let corename = file_name_strip(&config.files[0]);
    all_filters_problem(
        &mut bank, &mut ctrl, &filters, &corename, false, None, output,
    )
}

fn all_filters_problem<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filters: &AxFilterSet,
    corename: &str,
    hypo_filter_only: bool,
    desc: Option<&str>,
    output: &mut W,
) -> Result<(), Diagnostic> {
    for index in 0..filters.elements() {
        let Some(filter) = filters.get_filter(index) else {
            return Err(Diagnostic::new(
                ErrorCode::INTERFACE_ERROR,
                "AxFilterSet index missing while applying filters",
            ));
        };
        if !hypo_filter_only || filter.use_hypotheses {
            filter_problem(bank, ctrl, filter, corename, desc, output)?;
        }
    }
    Ok(())
}

fn filter_problem<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filter: &AxFilter,
    corename: &str,
    desc: Option<&str>,
    output: &mut W,
) -> Result<(), Diagnostic> {
    let filter_name = filter.name.as_deref().unwrap_or("");
    let filename = format!("{corename}_{filter_name}.p");
    let selection = ctrl.get_problem(bank.signature(), filter)?;

    writeln_diag(
        output,
        &format!("% Filter: {filter_name} goes into file {filename}"),
    )?;

    let mut rendered = Vec::new();
    writeln_diag(
        &mut rendered,
        &format!("% Filter {filter_name} on file {corename}"),
    )?;
    if let Some(desc) = desc {
        write_all(&mut rendered, desc.as_bytes())?;
    }
    bank.signature()
        .print_type_decls_tstp(&mut rendered, ProblemType::FirstOrder)
        .map_err(|error| io_diagnostic(format!("Cannot write TSTP type declarations: {error}")))?;

    let clauses =
        pstack_clause_print_tstp_string(bank, &selection.clauses, ProblemType::FirstOrder)?;
    write_all(&mut rendered, clauses.as_bytes())?;
    let formulas =
        pstack_formula_print_tstp_string(bank, &selection.formulas, ProblemType::FirstOrder, true)?;
    write_all(&mut rendered, formulas.as_bytes())?;

    let mut file = File::create(&filename)
        .map_err(|error| io_diagnostic(format!("Cannot open file {filename}: {error}")))?;
    file.write_all(&rendered)
        .map_err(|error| io_diagnostic(format!("Cannot write file {filename}: {error}")))
}

fn load_filters(filter_file: Option<&Path>) -> Result<AxFilterSet, Diagnostic> {
    let Some(path) = filter_file else {
        return AxFilterSet::default_set();
    };
    let mut scanner = Scanner::from_file(path, true)?;
    let mut filters = AxFilterSet::new();
    filters.parse(&mut scanner)?;
    Ok(filters)
}

fn init_struct_fof_spec<W: IoWrite + ?Sized>(
    parse_format: IoFormat,
    files: &[String],
    output: &mut W,
) -> Result<(TermBank, StructFofSpec, i64), Diagnostic> {
    let mut spec = BatchSpec::new(PROGRAM_NAME, parse_format);
    spec.includes = files.to_vec();
    let mut bank = new_term_bank()?;
    let mut ctrl = StructFofSpec::new(bank.signature());
    let parsed = spec.init_struct_fof_spec_from_files(&mut bank, &mut ctrl, None, output)?;
    ctrl.reset_shared();
    Ok((bank, ctrl, parsed))
}

fn new_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

fn apply_global_options(config: &EAxFilterConfig) {
    set_verbose_level(i64_to_i32_saturating(config.verbose_level));
    let _old_output_level = set_output_level(config.output_level);
}

fn parse_seed_subsample_arg(arg: &str) -> Result<(SubsampleMethod, i64), Diagnostic> {
    let bytes = arg.as_bytes();
    if bytes.len() < 2 || !matches!(bytes[0], b'm' | b'l' | b'r') || !bytes[1].is_ascii_digit() {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --seed-subsample) expects argument of the form [mlr][0-9]+",
        ));
    }
    let subsample = match bytes[0] {
        b'm' => SubsampleMethod::Most,
        b'l' => SubsampleMethod::Least,
        b'r' => SubsampleMethod::Random,
        _ => unreachable!("validated seed-subsample method"),
    };
    Ok((subsample, atol_decimal_prefix(&arg[1..])))
}

fn atol_decimal_prefix(arg: &str) -> i64 {
    let mut value = 0_i64;
    for byte in arg.bytes() {
        if !byte.is_ascii_digit() {
            break;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'));
    }
    value
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
{PROGRAM_NAME} {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} [options] [files]\n\
\n\
This program applies SinE-like goal-directed filters to a problem\n\
specification (a set of clauses and/or formulas) to generate reduced\n\
problem specifications that are easier to handle for a theorem prover,\n\
but still are likely to contain all the axioms necessary for a proof\n\
(if one exists).\n\
\n\
In default mode, the program reads a problem specification and an\n\
(optional) filter specification, and produces one reduced output file \n\
for each filter given. Note that while all standard input formats (LOP,\n\
TPTP-2 and TPTP-3 are supported, output is only and automatically in\n\
TPTP-3. Also note that unlike most of the other tools in the E\n\
distribution, this program does not support pipe-based input and output,\n\
since it uses file names generated from the input file name and filter\n\
names to store the different result files\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

fn required_arg<'a>(
    parsed: &'a ParsedOpt<'a, OptionCode>,
    name: &str,
) -> Result<&'a str, Diagnostic> {
    parsed.arg().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Option {name} requires an argument"),
        )
    })
}

fn open_output_file(path: Option<&Path>) -> Result<Option<File>, Diagnostic> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path == Path::new("-") {
        return Ok(None);
    }
    File::create(path)
        .map(Some)
        .map_err(|error| io_diagnostic(format!("Cannot open file {}: {error}", path.display())))
}

fn write_all(output: &mut (impl IoWrite + ?Sized), bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn writeln_diag(output: &mut (impl IoWrite + ?Sized), line: &str) -> Result<(), Diagnostic> {
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
    use std::path::{Path, PathBuf};

    use super::{
        init_struct_fof_spec, parse_seed_subsample_arg, process_options, run, EAxFilterConfig,
        RunCommand, SubsampleMethod, C_USAGE_ERROR, PROGRAM_NAME, SEEDED_FILTERING_PENDING,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::output::output_level;
    use crate::inout::scanner::IoFormat;
    use crate::test_support::global_state_lock;

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("e-axfilter-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn slash_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn generated_path(input: &Path, filter: &str) -> PathBuf {
        let stem = input
            .file_stem()
            .expect("test input has a stem")
            .to_string_lossy();
        PathBuf::from(format!("{stem}_{filter}.p"))
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).expect("help");

        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).expect("help is utf8");
        assert!(help.starts_with("\ne_axfilter "));
        assert!(help.contains("Usage: e_axfilter [options] [files]"));
        assert!(help.contains("This program applies SinE-like goal-directed filters"));
        assert!(help.contains("Bug reports for the first-order prover"));
        assert!(stderr.is_empty());

        let mut stdout = Vec::new();
        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).expect("version");

        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert!(String::from_utf8(stdout)
            .expect("version utf8")
            .starts_with("e_axfilter "));
    }

    #[test]
    fn process_options_records_formats_and_seed_quirks() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--output-file=filter.out",
                "--filter=filters.axf",
                "--seed-symbols=pc",
                "--seed-subsample=m25extra",
                "--seed-method=ld",
                "--tptp-in",
                "--output-level=3",
                "problem.p",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("options");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        assert_eq!(
            config,
            EAxFilterConfig {
                output_file: Some(PathBuf::from("filter.out")),
                filter_file: Some(PathBuf::from("filters.axf")),
                parse_format: IoFormat::Tptp,
                verbose_level: 2,
                output_level: 3,
                dump_filter: false,
                seed_preds: true,
                seed_funs: false,
                seed_consts: true,
                seed_large: true,
                seed_diverse: true,
                seed_all: false,
                seedstr: None,
                subsample: SubsampleMethod::Most,
                sample_size: 25,
                files: vec!["problem.p".to_owned()],
            }
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn defaults_match_c_globals() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options([PROGRAM_NAME, "problem.p"], &mut stdout, &mut stderr)
            .expect("options");
        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };

        assert_eq!(config.parse_format, IoFormat::Auto);
        assert_eq!(config.output_level, 1);
        assert!(config.seed_all);
        assert_eq!(config.subsample, SubsampleMethod::None);
        assert_eq!(config.sample_size, i64::MAX);
        assert_eq!(config.files, ["problem.p"]);
    }

    #[test]
    fn invalid_seed_options_report_usage_errors() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = process_options(
            [PROGRAM_NAME, "--seed-symbols=z", "problem.p"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option -S (--seed-symbols)"
        );

        let error = parse_seed_subsample_arg("x1").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --seed-subsample) expects argument of the form [mlr][0-9]+"
        );
    }

    #[test]
    fn dump_filter_happens_before_missing_problem_usage_error() {
        let _guard = global_state_lock();
        let output_path = temp_path("dump-before-usage");
        remove_if_present(&output_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--dump-filter",
                "-o",
                output_path.to_str().expect("test path is utf8"),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), C_USAGE_ERROR);
        let output = std::fs::read_to_string(&output_path).expect("dump output exists");
        assert!(output.contains("threshold010000 = Threshold(10000)"));
        assert!(stdout.is_empty());
        remove_if_present(&output_path);
    }

    #[test]
    fn custom_non_seeded_filter_generates_tstp_problem_file() {
        let _guard = global_state_lock();
        let problem_path = temp_path("problem");
        let filter_path = temp_path("filters");
        let output_path = temp_path("global-output");
        let generated_path = generated_path(&problem_path, "tiny");
        for path in [&problem_path, &filter_path, &output_path, &generated_path] {
            remove_if_present(path);
        }
        std::fs::write(&problem_path, "fof(a, axiom, p(a)).\n").expect("problem written");
        std::fs::write(&filter_path, "tiny=Threshold(10000)\n").expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                "-o",
                output_path.to_str().expect("test path is utf8"),
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("filter run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let global_output = std::fs::read_to_string(&output_path).expect("global output exists");
        assert!(global_output.contains("% Parsing "));
        assert!(global_output.contains("% Filter: tiny goes into file "));
        let generated = std::fs::read_to_string(&generated_path).expect("generated output exists");
        assert!(generated.starts_with("% Filter tiny on file "));
        assert!(generated.contains("fof(") || generated.contains("cnf("));

        for path in [&problem_path, &filter_path, &output_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_mode_is_parsed_but_reports_pending_after_problem_parse() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-problem");
        remove_if_present(&problem_path);
        std::fs::write(&problem_path, "fof(a, axiom, p(a)).\n").expect("problem written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--seed-symbols", &slash_path(&problem_path)],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert_eq!(error.message(), SEEDED_FILTERING_PENDING);
        assert!(String::from_utf8(stdout)
            .expect("stdout is utf8")
            .contains("% Parsing "));
        remove_if_present(&problem_path);
    }

    #[test]
    fn run_applies_verbose_and_output_globals_before_usage_error() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let _error = run(
            [PROGRAM_NAME, "--verbose=4", "--output-level=5"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(verbose_level(), 4);
        assert_eq!(output_level(), 5);
    }

    #[test]
    fn init_spec_parses_files_and_resets_shared_boundary() {
        let problem_path = temp_path("init");
        remove_if_present(&problem_path);
        std::fs::write(&problem_path, "fof(a, axiom, p(a)).\n").expect("problem written");
        let mut output = Vec::new();

        let (_bank, ctrl, parsed) =
            init_struct_fof_spec(IoFormat::Tstp, &[slash_path(&problem_path)], &mut output)
                .expect("problem parses");

        assert_eq!(parsed, 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 0);
        assert!(String::from_utf8(output)
            .expect("parse output utf8")
            .contains("% Parsing "));
        remove_if_present(&problem_path);
    }
}
