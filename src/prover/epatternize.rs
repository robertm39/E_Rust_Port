use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::proofstate::proof_state_alloc;
use crate::heuristics::clausesetfeatures::create_default_spec_limits;
use crate::inout::commandline::{
    get_bool_arg, get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{IoFormat, Scanner};
use crate::learn::clauseenc::flat_encode_clause_list_rep;
use crate::learn::patterns::{pattern_clause_compute, pattern_term_print_string, PatternSubst};
use crate::prover::eprover::{
    apply_proof_state_sine_silent, parse_clause_scanner_into_sets, FoolUnroll, FormulaPreprocessing,
};
use crate::prover::version::{E_URL, STS_MAIL, VERSION};
use crate::terms::signature::{
    FunctionProperties, FP_IGNORE_PROPS, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "epatternize";
const TFORM_RENAME_LIMIT_STR: &str = "24";
const TFORM_MINISCOPE_LIMIT_STR: &str = "1000";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    ParseFeatures,
    LopParse,
    TptpParse,
    TptpPrint,
    TptpFormat,
    TstpParse,
    TstpPrint,
    TstpFormat,
    RawClass,
    SpecSigFeatures,
    GenerateTptpHeader,
    NoPreprocessing,
    EqUnfoldLimit,
    EqUnfoldMaxClauses,
    NoEqUnfold,
    Sine,
    FreeNumbers,
    FreeObjects,
    DefinitionalCnf,
    MiniscopeLimit,
    ClassMask,
    RawMask,
    NguAbsolute,
    NguFewLimit,
    NguManyLimit,
    GpcAbsolute,
    GpcFewLimit,
    GpcManyLimit,
    AxiomManyLimit,
    AxiomSomeLimit,
    LitManyLimit,
    LitSomeLimit,
    TermMediumLimit,
    TermLargeLimit,
    FarSumMediumLimit,
    FarSumLargeLimit,
    MaxDepthMediumLimit,
    MaxDepthDeepLimit,
    SigMediumLimit,
    SigLargeLimit,
    PredConstMediumLimit,
    PredConstLargeLimit,
    PredMediumLimit,
    PredLargeLimit,
    FuncConstMediumLimit,
    FuncConstLargeLimit,
    FunMediumLimit,
    FunLargeLimit,
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
        "Verbose comments on the progress of the program. This differs from the output level (below) in that technical information is printed to stderr, while the output level determines which logical manipulations of the clauses are printed to stdout.",
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
        OptionCode::ParseFeatures,
        Some('p'),
        Some("parse-features"),
        OptArgType::NoArg,
        None,
        "Parse precomputed feature lines, not real formulae.",
    ),
    OptCell::new(
        OptionCode::LopParse,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Set E-LOP as the input format.",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Set TPTP-2 as the input format.",
    ),
    OptCell::new(
        OptionCode::TptpPrint,
        None,
        Some("tptp-out"),
        OptArgType::NoArg,
        None,
        "No effect (since not clauses/formulas are printed).",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tptp-in and --tptp-out.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TSTP format instead of E-LOP.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "No effect (since not clauses/formulas are printed).",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-in and --tstp-out.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tptp3-out"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-out.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-format.",
    ),
    OptCell::new(
        OptionCode::RawClass,
        Some('r'),
        Some("raw-class"),
        OptArgType::NoArg,
        None,
        "Perform a raw and rough classification on the unclausified and unpreprocessed problem.",
    ),
    OptCell::new(
        OptionCode::SpecSigFeatures,
        None,
        Some("specsig"),
        OptArgType::NoArg,
        None,
        "Compute and print new-style features based on the distribution of symbols of different arities.",
    ),
    OptCell::new(
        OptionCode::GenerateTptpHeader,
        Some('H'),
        Some("generate-tptp-header"),
        OptArgType::NoArg,
        None,
        "Generate the statistics part of a TPTP header for the problem.",
    ),
    OptCell::new(
        OptionCode::NoPreprocessing,
        None,
        Some("no-preprocessing"),
        OptArgType::NoArg,
        None,
        "Do not perform preprocessing on the initial clause set.",
    ),
    OptCell::new(
        OptionCode::EqUnfoldLimit,
        None,
        Some("eq-unfold-limit"),
        OptArgType::ReqArg,
        None,
        "During preprocessing, limit unfolding of equational definitions.",
    ),
    OptCell::new(
        OptionCode::EqUnfoldMaxClauses,
        None,
        Some("eq-unfold-maxclauses"),
        OptArgType::ReqArg,
        None,
        "During preprocessing, don't try unfolding of equational definitions if the problem has more than this limit of clauses.",
    ),
    OptCell::new(
        OptionCode::NoEqUnfold,
        None,
        Some("no-eq-unfolding"),
        OptArgType::NoArg,
        None,
        "During preprocessing, abstain from unfolding equational definitions.",
    ),
    OptCell::new(
        OptionCode::Sine,
        None,
        Some("sine"),
        OptArgType::OptArg,
        Some("Auto"),
        "Apply SInE to prune the unprocessed axioms with the specified filter.",
    ),
    OptCell::new(
        OptionCode::FreeNumbers,
        None,
        Some("free-numbers"),
        OptArgType::NoArg,
        None,
        "Treat numbers as normal free function symbols in the input.",
    ),
    OptCell::new(
        OptionCode::FreeObjects,
        None,
        Some("free-objects"),
        OptArgType::NoArg,
        None,
        "Treat object identifiers as normal free function symbols in the input.",
    ),
    OptCell::new(
        OptionCode::DefinitionalCnf,
        None,
        Some("definitional-cnf"),
        OptArgType::OptArg,
        Some(TFORM_RENAME_LIMIT_STR),
        "Tune the clausification algorithm to introduce definitions for subformulae.",
    ),
    OptCell::new(
        OptionCode::MiniscopeLimit,
        None,
        Some("miniscope-limit"),
        OptArgType::OptArg,
        Some(TFORM_MINISCOPE_LIMIT_STR),
        "Set the limit of variables to miniscope per input formula.",
    ),
    OptCell::new(
        OptionCode::ClassMask,
        Some('c'),
        Some("class-mask"),
        OptArgType::ReqArg,
        None,
        "Provide a mask for the class description.",
    ),
    OptCell::new(
        OptionCode::RawMask,
        None,
        Some("raw-mask"),
        OptArgType::ReqArg,
        None,
        "Provide a mask for the rawclass description.",
    ),
    OptCell::new(
        OptionCode::NguAbsolute,
        Some('a'),
        Some("ngu-absolute"),
        OptArgType::OptArg,
        Some("true"),
        "Use absolute values to determine non-ground unit classes.",
    ),
    OptCell::new(
        OptionCode::NguFewLimit,
        Some('f'),
        Some("ngu-few-limit"),
        OptArgType::ReqArg,
        None,
        "Set the few limit for non-ground unit clauses.",
    ),
    OptCell::new(
        OptionCode::NguManyLimit,
        Some('m'),
        Some("ngu-many-limit"),
        OptArgType::ReqArg,
        None,
        "Set the many limit for non-ground unit clauses.",
    ),
    OptCell::new(
        OptionCode::GpcAbsolute,
        None,
        Some("gpc-absolute"),
        OptArgType::OptArg,
        Some("true"),
        "Use absolute values to determine ground positive classes.",
    ),
    OptCell::new(
        OptionCode::GpcFewLimit,
        None,
        Some("gpc-few-limit"),
        OptArgType::ReqArg,
        None,
        "Set the few limit for ground positive clauses.",
    ),
    OptCell::new(
        OptionCode::GpcManyLimit,
        None,
        Some("gpc-many-limit"),
        OptArgType::ReqArg,
        None,
        "Set the many limit for ground positive clauses.",
    ),
    OptCell::new(
        OptionCode::AxiomSomeLimit,
        None,
        Some("ax-some-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium-size clause limit.",
    ),
    OptCell::new(
        OptionCode::AxiomManyLimit,
        None,
        Some("ax-many-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large-size clause limit.",
    ),
    OptCell::new(
        OptionCode::LitSomeLimit,
        None,
        Some("lit-some-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium-size literal limit.",
    ),
    OptCell::new(
        OptionCode::LitManyLimit,
        None,
        Some("lit-many-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large-size literal limit.",
    ),
    OptCell::new(
        OptionCode::TermMediumLimit,
        None,
        Some("term-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium-size subterm limit.",
    ),
    OptCell::new(
        OptionCode::TermLargeLimit,
        None,
        Some("term-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large-size subterm limit.",
    ),
    OptCell::new(
        OptionCode::FarSumMediumLimit,
        None,
        Some("farity-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium sum of function-symbol arities.",
    ),
    OptCell::new(
        OptionCode::FarSumLargeLimit,
        None,
        Some("farity-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large sum of function-symbol arities.",
    ),
    OptCell::new(
        OptionCode::MaxDepthMediumLimit,
        None,
        Some("max-depth-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the minimum maximal clause depth for medium depth specifications.",
    ),
    OptCell::new(
        OptionCode::MaxDepthDeepLimit,
        None,
        Some("max-depth-deep-limit"),
        OptArgType::ReqArg,
        None,
        "Set the minimum maximal clause depth for deep depth specifications.",
    ),
    OptCell::new(
        OptionCode::SigMediumLimit,
        None,
        Some("sig-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the minimum signature size for medium sized signatures.",
    ),
    OptCell::new(
        OptionCode::SigLargeLimit,
        None,
        Some("sig-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the minimum signature size for large signatures.",
    ),
    OptCell::new(
        OptionCode::PredConstMediumLimit,
        None,
        Some("pred-const-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium number of constant predicate symbols.",
    ),
    OptCell::new(
        OptionCode::PredConstLargeLimit,
        None,
        Some("pred-const-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large number of constant predicate symbols.",
    ),
    OptCell::new(
        OptionCode::PredMediumLimit,
        None,
        Some("pred-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium number of non-constant predicate symbols.",
    ),
    OptCell::new(
        OptionCode::PredLargeLimit,
        None,
        Some("pred-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large number of non-constant predicate symbols.",
    ),
    OptCell::new(
        OptionCode::FuncConstMediumLimit,
        None,
        Some("fun-const-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium number of constant function symbols.",
    ),
    OptCell::new(
        OptionCode::FuncConstLargeLimit,
        None,
        Some("fun-const-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large number of constant function symbols.",
    ),
    OptCell::new(
        OptionCode::FunMediumLimit,
        None,
        Some("fun-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium number of non-constant function symbols.",
    ),
    OptCell::new(
        OptionCode::FunLargeLimit,
        None,
        Some("fun-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large number of non-constant function symbols.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EpatternizeConfig {
    output_file: Option<PathBuf>,
    parse_format: IoFormat,
    sine: Option<String>,
    free_symbol_properties: FunctionProperties,
    files: Vec<String>,
}

impl Default for EpatternizeConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            parse_format: IoFormat::Auto,
            sine: None,
            free_symbol_properties: FP_IGNORE_PROPS,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EpatternizeConfig),
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
    let result = run_inner(argv, stdin, stdout);
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
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_epatternize(&config, stdin, stdout),
    }
}

#[allow(clippy::too_many_lines)]
fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EpatternizeConfig::default();
    let mut compatibility_limits = create_default_spec_limits();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        let arg = parsed.arg().unwrap_or("");
        match parsed.option().option_code {
            OptionCode::Verbose => {
                set_verbose_level(i64_to_i32_saturating(get_int_arg(parsed.option(), arg)?));
            }
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION}"))?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Output => config.output_file = Some(PathBuf::from(arg)),
            OptionCode::LopParse => config.parse_format = IoFormat::Lop,
            OptionCode::TptpParse | OptionCode::TptpFormat => {
                config.parse_format = IoFormat::Tptp;
            }
            OptionCode::TstpParse | OptionCode::TstpFormat => config.parse_format = IoFormat::Tstp,
            OptionCode::Sine => config.sine = Some(arg.to_owned()),
            OptionCode::FreeNumbers => {
                config.free_symbol_properties |= FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT;
            }
            OptionCode::FreeObjects => {
                config.free_symbol_properties |= FP_IS_OBJECT;
            }
            OptionCode::DefinitionalCnf
            | OptionCode::MiniscopeLimit
            | OptionCode::EqUnfoldMaxClauses
            | OptionCode::EqUnfoldLimit => {
                let _ = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::ClassMask => validate_exact_mask_len(
                arg,
                13,
                "Option -c (--class-mask) requires 13-letter string as an argument",
            )?,
            OptionCode::RawMask => validate_exact_mask_len(
                arg,
                7,
                "Option -c (--class-mask) requires 7-letter string as an argument",
            )?,
            OptionCode::NguAbsolute => {
                compatibility_limits.ngu_absolute = get_bool_arg(parsed.option(), arg)?;
            }
            OptionCode::NguFewLimit => {
                compatibility_limits.ngu_few_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::NguManyLimit => {
                compatibility_limits.ngu_many_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcAbsolute => {
                compatibility_limits.gpc_absolute = get_bool_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcFewLimit => {
                compatibility_limits.gpc_few_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcManyLimit => {
                compatibility_limits.gpc_many_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::AxiomSomeLimit => {
                compatibility_limits.ax_some_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::AxiomManyLimit => {
                compatibility_limits.ax_many_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::LitSomeLimit => {
                compatibility_limits.lit_some_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::LitManyLimit => {
                compatibility_limits.lit_many_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::TermMediumLimit => {
                compatibility_limits.term_medium_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::TermLargeLimit => {
                compatibility_limits.term_large_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::FarSumMediumLimit => {
                compatibility_limits.far_sum_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FarSumLargeLimit => {
                compatibility_limits.far_sum_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::MaxDepthMediumLimit => {
                compatibility_limits.depth_medium_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::MaxDepthDeepLimit => {
                compatibility_limits.depth_deep_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::SigMediumLimit => {
                compatibility_limits.symbols_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::SigLargeLimit => {
                compatibility_limits.symbols_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredConstMediumLimit => {
                compatibility_limits.predc_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredConstLargeLimit => {
                compatibility_limits.predc_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredMediumLimit => {
                compatibility_limits.pred_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredLargeLimit => {
                compatibility_limits.pred_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FuncConstMediumLimit => {
                compatibility_limits.func_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FuncConstLargeLimit => {
                compatibility_limits.func_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FunMediumLimit => {
                compatibility_limits.fun_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FunLargeLimit => {
                compatibility_limits.fun_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::ParseFeatures
            | OptionCode::TptpPrint
            | OptionCode::TstpPrint
            | OptionCode::RawClass
            | OptionCode::SpecSigFeatures
            | OptionCode::GenerateTptpHeader
            | OptionCode::NoPreprocessing
            | OptionCode::NoEqUnfold => {}
        }
    }

    let _ = compatibility_limits;

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_epatternize(
    config: &EpatternizeConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = EpatternizeOutput::open(config.output_file.as_deref(), stdout)?;

    for file in &config.files {
        let mut state = proof_state_alloc(config.free_symbol_properties)?;
        let mut scanner = scanner_for_input(file, stdin)?;
        let mut parsed = ClauseSet::new();
        let mut parsed_watchlist = ClauseSet::new();
        parse_clause_scanner_into_sets(
            &mut scanner,
            config.parse_format,
            FormulaPreprocessing::parse_only(FoolUnroll::Enabled),
            state.terms_mut(),
            &mut parsed,
            &mut parsed_watchlist,
        )?;
        state.axioms_mut().insert_set(&mut parsed);
        apply_proof_state_sine_silent(config.sine.as_deref(), &mut state)?;
        write_epatternized_axioms(&mut output, &mut state)?;
    }

    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn write_epatternized_axioms(
    output: &mut impl Write,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<(), Diagnostic> {
    let (bank, axioms) = state.terms_and_axioms_mut();
    ensure_flat_clause_encoding_symbols(bank, axioms)?;
    for clause in axioms.iter() {
        let mut pattern =
            pattern_clause_compute(clause, PatternSubst::default_subst(bank.signature()));
        if pattern.tries() <= 0 {
            continue;
        }
        let clause_rep = flat_encode_clause_list_rep(bank, pattern.listrep())?;
        let rendered =
            pattern_term_print_string(pattern.subst_mut(), &clause_rep, bank.signature());
        writeln_diag(output, &rendered)?;
    }
    Ok(())
}

fn ensure_flat_clause_encoding_symbols(
    bank: &mut crate::terms::termbanks::TermBank,
    axioms: &ClauseSet,
) -> Result<(), Diagnostic> {
    bank.signature_mut().get_eqn_code(true);
    bank.signature_mut().get_eqn_code(false);
    for clause in axioms.iter() {
        let arity = i32::try_from(clause.literals().len()).map_err(|_| {
            Diagnostic::new(
                ErrorCode::RESOURCE_OUT,
                "Clause has too many literals for flat pattern encoding",
            )
        })?;
        bank.signature_mut().get_or_n_code(arity);
    }
    Ok(())
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, false)
    } else {
        Scanner::from_file(Path::new(name), false).map_err(epatternize_scanner_open_diagnostic)
    }
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
\n\
{PROGRAM_NAME} {VERSION}\n\
\n\
Usage: classify_problem [options] [files]\n\
\n\
Read sets of clauses/formulas, perform cnfization, then convert \n\
the clauses to patterns and print them.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push_str(&legacy_footer());
    result
}

fn legacy_footer() -> String {
    format!(
        "\n\
Copyright (C) 1998-2009 by Stephan Schulz, {STS_MAIL}\n\
\n\
This program is a part of the support structure for the E equational\n\
theorem prover. You can find the latest version of the E distribution\n\
as well as additional information at\n\
{E_URL}\n\
This program is free software; you can redistribute it and/or modify\n\
it under the terms of the GNU General Public License as published by\n\
the Free Software Foundation; either version 2 of the License, or\n\
(at your option) any later version.\n\
\n\
This program is distributed in the hope that it will be useful,\n\
but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n\
GNU General Public License for more details.\n\
\n\
You should have received a copy of the GNU General Public License\n\
along with this program (it should be contained in the top level\n\
directory of the distribution in the file COPYING); if not, write to\n\
the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n\
Boston, MA  02111-1307 USA\n\
\n\
The original copyright holder can be contacted as\n\
\n\
Stephan Schulz\n\
DHBW Stuttgart\n\
Fakultaet Technik\n\
Informatik\n\
Lerchenstrasse 1\n\
70174 Stuttgart\n\
Germany\n"
    )
}

enum EpatternizeOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> EpatternizeOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        let file = File::create(path).map_err(|error| {
            epatternize_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })?;
        Ok(Self::File(file))
    }
}

impl<W: Write> Write for EpatternizeOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(output) => output.write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(output) => output.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

fn validate_exact_mask_len(
    mask: &str,
    expected_len: usize,
    message: &str,
) -> Result<(), Diagnostic> {
    if mask.len() == expected_len {
        Ok(())
    } else {
        Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
    }
}

fn get_i32_arg<Code>(option: &OptCell<Code>, arg: &str) -> Result<i32, Diagnostic> {
    let value = get_int_arg(option, arg)?;
    i32::try_from(value)
        .map_err(|_| Diagnostic::new(ErrorCode::USAGE_ERROR, "Option argument out of int range"))
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

fn epatternize_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn epatternize_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
    if error.code() != ErrorCode::FILE_ERROR || !error.message().starts_with("Cannot open file ") {
        return error;
    }
    let Some((prefix, source_error)) = error.message().split_once(": ") else {
        return error;
    };
    Diagnostic::new(
        error.code(),
        format!("{prefix}\n{PROGRAM_NAME}: {source_error}"),
    )
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{
        process_options, run, EpatternizeConfig, RunCommand, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::VERSION;
    use crate::terms::signature::{FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL};
    use crate::test_support::global_state_lock;
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    #[test]
    fn help_preserves_c_usage_typo() {
        let mut stdout = Vec::new();
        let command = process_options(["epatternize", "--help"], &mut stdout).unwrap();

        assert!(matches!(command, RunCommand::Exit(0)));
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(&format!("{PROGRAM_NAME} {VERSION}")));
        assert!(printed.contains("Usage: classify_problem [options] [files]"));
    }

    #[test]
    fn version_is_long_only() {
        let mut stdout = Vec::new();
        let command = process_options(["epatternize", "--version"], &mut stdout).unwrap();

        assert!(matches!(command, RunCommand::Exit(0)));
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("{PROGRAM_NAME} {VERSION}\n")
        );

        let err = process_options(["epatternize", "-V"], &mut Vec::new()).unwrap_err();
        assert_eq!(err.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn options_default_to_stdin_and_accept_compatibility_noops() {
        let mut stdout = Vec::new();
        let command = process_options(
            [
                "epatternize",
                "--tstp-format",
                "--parse-features",
                "--raw-class",
                "--specsig",
                "-H",
                "--no-preprocessing",
                "--eq-unfold-limit=7",
                "--eq-unfold-maxclauses=13",
                "--no-eq-unfolding",
                "--definitional-cnf=5",
                "--miniscope-limit=11",
                "--class-mask=aaaaaaaaaaaaa",
                "--raw-mask=aaaaaaa",
                "--free-numbers",
                "--free-objects",
            ],
            &mut stdout,
        )
        .unwrap();

        let RunCommand::Execute(config) = command else {
            panic!("expected execution command");
        };
        assert_eq!(
            config,
            EpatternizeConfig {
                output_file: None,
                parse_format: IoFormat::Tstp,
                sine: None,
                free_symbol_properties: FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT | FP_IS_OBJECT,
                files: vec!["-".to_owned()],
            }
        );
    }

    #[test]
    fn masks_require_c_exact_lengths() {
        let class_err = process_options(
            ["epatternize", "--class-mask=aaaaaaaaaaaaaa"],
            &mut Vec::new(),
        )
        .unwrap_err();
        assert_eq!(class_err.code(), ErrorCode::USAGE_ERROR);

        let raw_err =
            process_options(["epatternize", "--raw-mask=aaaaaaaa"], &mut Vec::new()).unwrap_err();
        assert_eq!(raw_err.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn patternizes_lop_stdin() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            ["epatternize", "--lop-in"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(printed, "$or1($eq(f1_1(f0_1),$true))\n");
    }

    #[test]
    fn patternizes_tstp_file_to_output_file() {
        let _guard = global_state_lock();
        let input_path = temp_path("epatternize-input");
        let output_path = temp_path("epatternize-output");
        fs::write(&input_path, "cnf(c1, axiom, (p(a))).\n").unwrap();

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"";
        let status = run(
            [
                "epatternize",
                "--tstp-in",
                "-o",
                output_path.to_str().unwrap(),
                input_path.to_str().unwrap(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let printed = fs::read_to_string(&output_path).unwrap();
        assert_eq!(printed, "$or1($eq(f1_1(f0_1),$true))\n");

        let _ = fs::remove_file(input_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            ["epatternize", "--lop-in", "-o", "-"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "$or1($eq(f1_1(f0_1),$true))\n"
        );
    }

    #[test]
    fn explicit_no_sine_keeps_patternization_path() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"cnf(c1, axiom, (p(a))).\n";

        let status = run(
            ["epatternize", "--tstp-in", "--sine=NoSInE"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "$or1($eq(f1_1(f0_1),$true))\n"
        );
    }

    #[test]
    fn input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("epatternize-missing-input");
        let _ = fs::remove_file(&missing_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"";

        let error = run(
            ["epatternize", missing_path.to_str().unwrap()],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_is_created_before_later_input_open_failure() {
        let _guard = global_state_lock();
        let output_path = temp_path("epatternize-early-output");
        let missing_path = temp_path("epatternize-missing-after-output");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(&missing_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"";

        let error = run(
            [
                "epatternize",
                "-o",
                output_path.to_str().unwrap(),
                missing_path.to_str().unwrap(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(output_path.exists());
        assert_eq!(fs::read_to_string(&output_path).unwrap(), "");
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("epatternize-output-dir");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_dir(&output_path);
        fs::create_dir(&output_path).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let error = run(
            ["epatternize", "-o", output_path.to_str().unwrap()],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error
            .message()
            .starts_with(&format!("Cannot open file {}", output_path.display())));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        fs::remove_dir(output_path).unwrap();
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let error = run(
            ["epatternize", "--lop-in"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    fn temp_path(label: &str) -> PathBuf {
        let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{label}-{}-{serial}.tmp", std::process::id()))
    }
}
