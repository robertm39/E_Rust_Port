use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{
    problem_type, reset_problem_type, set_problem_type, ProblemType,
};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clausefunc::clause_set_archive_copy;
use crate::clauses::formulasets::FormulaSetCnfOptions;
use crate::clauses::proofstate::{proof_state_alloc, ProofState};
use crate::clauses::unfold_defs::{clause_set_preprocess, clause_set_unfold_eq_def_normalize};
use crate::heuristics::clausesetfeatures::{
    clause_set_count_eqn_literals, clause_set_count_range_restricted, clause_set_count_singletons,
    clause_set_count_variables, clause_set_max_literal_number, create_default_spec_limits,
    spec_features_add_eval, spec_features_compute_with_choice_recognition, spec_features_parse,
    spec_features_print_string, spec_type_print_string, SpecFeatureCell, SpecFeatureClass,
    SpecLimits, SPEC_STRING_MEM,
};
use crate::heuristics::new_autoschedule::DEFAULT_MASK as MERGED_CLASSIFY_MASK;
use crate::heuristics::rawspecfeatures::{
    raw_spec_features_classify, raw_spec_features_compute, raw_spec_features_format,
    raw_spec_features_parse, RawSpecFeatureCell, RAW_DEFAULT_MASK,
};
use crate::heuristics::specsigfeatures::{
    clause_set_collect_sig_features, spec_sig_feature_format, SpecSigFeatureCell,
};
use crate::inout::basicparser::parse_plain_filename;
use crate::inout::commandline::{
    get_bool_arg, get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::prover::eprover::{
    apply_proof_state_sine_silent, parse_clause_scanner_into_formula_set_with_options, FoolUnroll,
    FormulaPreprocessing,
};
use crate::prover::version::{E_URL, STS_MAIL, VERSION};
use crate::terms::signature::{
    FunctionProperties, FP_IGNORE_PROPS, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
};
use crate::terms::termbanks::TermBank;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "classify_problem";

const DEFAULT_CLASSIFY_MASK: &str = "aaaa-aaaaaa-a";
const DEFAULT_RAW_MASK: &str = "aaaaaaaaaa";
const FORMULA_DEF_LIMIT_DEFAULT: i64 = 24;
const MINISCOPE_LIMIT_DEFAULT: i64 = 1_000;
const MERGED_CNF_MINISCOPE_LIMIT: i64 = 1_048_576;
const DEFAULT_EQDEF_MAXCLAUSES: i64 = 200;
const DEFAULT_EQDEF_INCRLIMIT: i64 = 20;
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
    OldCnf,
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
    MergedClassification,
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
        "Parse precomputed feature lines, not real formulae. This conflicts with the '--generate-tptp-header' option, as not all information needed for this is stored in feature lines.",
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
        "Generate the statistics (\"Syntax\") part of a TPTP header for the problem.",
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
        OptionCode::OldCnf,
        None,
        Some("old-cnf"),
        OptArgType::OptArg,
        Some(TFORM_RENAME_LIMIT_STR),
        "Use the classical clausification algorithm.",
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
        "Set the medium function-arity sum limit.",
    ),
    OptCell::new(
        OptionCode::FarSumLargeLimit,
        None,
        Some("farity-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large function-arity sum limit.",
    ),
    OptCell::new(
        OptionCode::MaxDepthMediumLimit,
        None,
        Some("max-depth-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium maximal clause depth limit.",
    ),
    OptCell::new(
        OptionCode::MaxDepthDeepLimit,
        None,
        Some("max-depth-deep-limit"),
        OptArgType::ReqArg,
        None,
        "Set the deep maximal clause depth limit.",
    ),
    OptCell::new(
        OptionCode::SigMediumLimit,
        None,
        Some("sig-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium signature-size limit.",
    ),
    OptCell::new(
        OptionCode::SigLargeLimit,
        None,
        Some("sig-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large signature-size limit.",
    ),
    OptCell::new(
        OptionCode::PredConstMediumLimit,
        None,
        Some("pred-const-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium predicate-constant limit.",
    ),
    OptCell::new(
        OptionCode::PredConstLargeLimit,
        None,
        Some("pred-const-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large predicate-constant limit.",
    ),
    OptCell::new(
        OptionCode::PredMediumLimit,
        None,
        Some("pred-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium non-constant predicate limit.",
    ),
    OptCell::new(
        OptionCode::PredLargeLimit,
        None,
        Some("pred-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large non-constant predicate limit.",
    ),
    OptCell::new(
        OptionCode::FuncConstMediumLimit,
        None,
        Some("fun-const-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium function-constant limit.",
    ),
    OptCell::new(
        OptionCode::FuncConstLargeLimit,
        None,
        Some("fun-const-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large function-constant limit.",
    ),
    OptCell::new(
        OptionCode::FunMediumLimit,
        None,
        Some("fun-medium-limit"),
        OptArgType::ReqArg,
        None,
        "Set the medium non-constant function limit.",
    ),
    OptCell::new(
        OptionCode::FunLargeLimit,
        None,
        Some("fun-large-limit"),
        OptArgType::ReqArg,
        None,
        "Set the large non-constant function limit.",
    ),
    OptCell::new(
        OptionCode::MergedClassification,
        None,
        Some("merged-classification"),
        OptArgType::ReqArg,
        None,
        "Perform classification that merges formula and clause properties.",
    ),
];

#[derive(Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "C-compatible executable configuration mirrors classify_problem.c globals"
)]
struct ClassifyProblemConfig {
    output_file: Option<PathBuf>,
    parse_features: bool,
    raw_classify: bool,
    specsig_classify: bool,
    tptp_header: bool,
    no_preprocessing: bool,
    parse_format: IoFormat,
    mask: String,
    raw_mask: String,
    sine: Option<String>,
    eqdef_maxclauses: i64,
    eqdef_incrlimit: i64,
    miniscope_limit: i64,
    formula_def_limit: i64,
    cnf_timeout: Option<i64>,
    free_symbol_properties: FunctionProperties,
    files: Vec<String>,
    limits: SpecLimits,
}

impl Default for ClassifyProblemConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            parse_features: false,
            raw_classify: false,
            specsig_classify: false,
            tptp_header: false,
            no_preprocessing: false,
            parse_format: IoFormat::Auto,
            mask: DEFAULT_CLASSIFY_MASK.to_owned(),
            raw_mask: DEFAULT_RAW_MASK.to_owned(),
            sine: None,
            eqdef_maxclauses: DEFAULT_EQDEF_MAXCLAUSES,
            eqdef_incrlimit: DEFAULT_EQDEF_INCRLIMIT,
            miniscope_limit: MINISCOPE_LIMIT_DEFAULT,
            formula_def_limit: FORMULA_DEF_LIMIT_DEFAULT,
            cnf_timeout: None,
            free_symbol_properties: FP_IGNORE_PROPS,
            files: Vec::new(),
            limits: create_default_spec_limits(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(Box<ClassifyProblemConfig>),
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
        RunCommand::Execute(config) => execute_classify_problem(&config, stdin, stdout),
    }
}

#[allow(clippy::too_many_lines)]
fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = ClassifyProblemConfig::default();

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
            OptionCode::ParseFeatures => config.parse_features = true,
            OptionCode::LopParse => config.parse_format = IoFormat::Lop,
            OptionCode::TptpParse | OptionCode::TptpFormat => {
                config.parse_format = IoFormat::Tptp;
            }
            OptionCode::TptpPrint | OptionCode::TstpPrint | OptionCode::OldCnf => {}
            OptionCode::FreeNumbers => {
                config.free_symbol_properties |= FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT;
            }
            OptionCode::FreeObjects => {
                config.free_symbol_properties |= FP_IS_OBJECT;
            }
            OptionCode::TstpParse | OptionCode::TstpFormat => config.parse_format = IoFormat::Tstp,
            OptionCode::RawClass => config.raw_classify = true,
            OptionCode::SpecSigFeatures => config.specsig_classify = true,
            OptionCode::GenerateTptpHeader => config.tptp_header = true,
            OptionCode::NoPreprocessing => config.no_preprocessing = true,
            OptionCode::EqUnfoldMaxClauses => {
                config.eqdef_maxclauses = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::EqUnfoldLimit => {
                config.eqdef_incrlimit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::NoEqUnfold => config.eqdef_incrlimit = i64::MIN,
            OptionCode::Sine => config.sine = Some(arg.to_owned()),
            OptionCode::DefinitionalCnf => {
                config.formula_def_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::MiniscopeLimit => {
                config.miniscope_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::ClassMask => {
                validate_mask_len(
                    arg,
                    13,
                    "Option -c (--class-mask) requires at least 13-letter string as an argument",
                )?;
                arg.clone_into(&mut config.mask);
            }
            OptionCode::RawMask => {
                validate_mask_len(
                    arg,
                    11,
                    "Option -c (--class-mask) requires at least 11-letter string as an argument",
                )?;
                arg.clone_into(&mut config.raw_mask);
            }
            OptionCode::NguAbsolute => {
                config.limits.ngu_absolute = get_bool_arg(parsed.option(), arg)?;
            }
            OptionCode::NguFewLimit => {
                config.limits.ngu_few_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::NguManyLimit => {
                config.limits.ngu_many_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcAbsolute => {
                config.limits.gpc_absolute = get_bool_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcFewLimit => {
                config.limits.gpc_few_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::GpcManyLimit => {
                config.limits.gpc_many_limit = get_float_arg(parsed.option(), arg)?;
            }
            OptionCode::AxiomSomeLimit => {
                config.limits.ax_some_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::AxiomManyLimit => {
                config.limits.ax_many_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::LitSomeLimit => {
                config.limits.lit_some_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::LitManyLimit => {
                config.limits.lit_many_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::TermMediumLimit => {
                config.limits.term_medium_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::TermLargeLimit => {
                config.limits.term_large_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::FarSumMediumLimit => {
                config.limits.far_sum_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FarSumLargeLimit => {
                config.limits.far_sum_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::MaxDepthMediumLimit => {
                config.limits.depth_medium_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::MaxDepthDeepLimit => {
                config.limits.depth_deep_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::SigMediumLimit => {
                config.limits.symbols_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::SigLargeLimit => {
                config.limits.symbols_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredConstMediumLimit => {
                config.limits.predc_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredConstLargeLimit => {
                config.limits.predc_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredMediumLimit => {
                config.limits.pred_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::PredLargeLimit => {
                config.limits.pred_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FuncConstMediumLimit => {
                config.limits.func_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FuncConstLargeLimit => {
                config.limits.func_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FunMediumLimit => {
                config.limits.fun_medium_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::FunLargeLimit => {
                config.limits.fun_large_limit = get_i32_arg(parsed.option(), arg)?;
            }
            OptionCode::MergedClassification => {
                config.cnf_timeout = Some(get_int_arg(parsed.option(), arg)?);
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(Box::new(config)))
}

fn execute_classify_problem(
    config: &ClassifyProblemConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = ClassifyOutput::open(config.output_file.as_deref(), stdout)?;
    if config.parse_features {
        if config.raw_classify {
            process_raw_feature_files(config, stdin, &mut output)?;
        } else {
            process_feature_files(config, stdin, &mut output)?;
        }
        output
            .flush()
            .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
        return Ok(0);
    }

    if let Some(timeout) = config.cnf_timeout {
        if timeout != -1 {
            process_merged_real_input_files(config, timeout, stdin, &mut output)?;
            output
                .flush()
                .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
            return Ok(0);
        }
    }
    if config.raw_classify {
        process_raw_real_input_files(config, stdin, &mut output)?;
        output
            .flush()
            .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
        return Ok(0);
    }

    process_standard_real_input_files(config, stdin, &mut output)?;
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn process_feature_files(
    config: &ClassifyProblemConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        while !scanner.test_tok(TokenType::NO_TOKEN) {
            let line = parse_feature_line(&mut scanner, &config.limits)?;
            write_all(output, line.name.as_bytes())?;
            write_all(output, b" : ")?;
            let mut features = line.features;
            spec_features_add_eval(&mut features, &config.limits);
            write_all(output, spec_features_print_string(&features).as_bytes())?;
            write_all(output, b" : ")?;
            write_all(
                output,
                spec_type_print_string(&features, &config.mask).as_bytes(),
            )?;
            write_all(output, b"\n")?;
        }
    }
    Ok(())
}

fn process_raw_feature_files(
    config: &ClassifyProblemConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        while !scanner.test_tok(TokenType::NO_TOKEN) {
            let mut line = parse_raw_feature_line(&mut scanner)?;
            raw_spec_features_classify(&mut line.features, &config.limits, Some(&config.raw_mask));
            write_all(output, line.name.as_bytes())?;
            write_all(output, b" : ")?;
            write_all(output, raw_spec_features_format(&line.features).as_bytes())?;
            write_all(output, b"\n")?;
        }
    }
    Ok(())
}

fn process_merged_real_input_files(
    config: &ClassifyProblemConfig,
    cnf_timeout: i64,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    for file in &config.files {
        let mut state = proof_state_alloc(config.free_symbol_properties)?;
        parse_real_input_file(config, file, stdin, &mut state)?;
        apply_proof_state_sine_silent(config.sine.as_deref(), &mut state)?;
        let raw_features = raw_features_for_standard_classification(config, &state);
        let cnf_class = classify_current_cnf_state(cnf_timeout, &mut state)?;
        write_all(output, file.as_bytes())?;
        write_all(output, b" : (NULL) : ")?;
        write_all(output, raw_features.class.as_bytes())?;
        write_all(output, cnf_class.as_bytes())?;
        write_all(output, b"\n")?;
    }
    Ok(())
}

fn process_standard_real_input_files(
    config: &ClassifyProblemConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    for file in &config.files {
        let mut state = proof_state_alloc(config.free_symbol_properties)?;
        parse_real_input_file(config, file, stdin, &mut state)?;
        apply_proof_state_sine_silent(config.sine.as_deref(), &mut state)?;
        let raw_features = raw_features_for_standard_classification(config, &state);
        preprocess_real_input_clauses(config, &mut state)?;
        if config.specsig_classify {
            write_specs_sig_real_input(file, output, &mut state)?;
        } else {
            write_standard_real_input_classification(
                config,
                file,
                output,
                &mut state,
                &raw_features,
            )?;
        }
    }
    Ok(())
}

fn process_raw_real_input_files(
    config: &ClassifyProblemConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    for file in &config.files {
        let mut state = proof_state_alloc(config.free_symbol_properties)?;
        parse_real_input_file(config, file, stdin, &mut state)?;
        apply_proof_state_sine_silent(config.sine.as_deref(), &mut state)?;
        let mut features = RawSpecFeatureCell::default();
        raw_spec_features_compute(&mut features, &state);
        raw_spec_features_classify(&mut features, &config.limits, Some(&config.raw_mask));
        write_all(output, file.as_bytes())?;
        write_all(output, b" : ")?;
        write_all(output, raw_spec_features_format(&features).as_bytes())?;
        write_all(output, b"\n")?;
    }
    Ok(())
}

fn raw_features_for_standard_classification(
    config: &ClassifyProblemConfig,
    state: &ProofState,
) -> RawSpecFeatureCell {
    let mut features = RawSpecFeatureCell::default();
    raw_spec_features_compute(&mut features, state);
    raw_spec_features_classify(&mut features, &config.limits, Some(RAW_DEFAULT_MASK));
    features
}

fn classify_current_cnf_state(
    cnf_timeout: i64,
    state: &mut ProofState,
) -> Result<String, Diagnostic> {
    if cnf_timeout <= 0 {
        return Ok("-".repeat(SPEC_STRING_MEM - 1));
    }

    let cnf_options = FormulaSetCnfOptions::new(MERGED_CNF_MINISCOPE_LIMIT, true, problem_type())
        .with_def_limit(FORMULA_DEF_LIMIT_DEFAULT)
        .with_lift_lambdas(false);
    clausify_real_input_formula_axioms_with_options(state, cnf_options)?;

    let mut features = SpecFeatureCell::default();
    {
        let (bank, axioms, f_axioms, f_ax_archive) = state.terms_axioms_formula_sets_mut();
        spec_features_compute_with_choice_recognition(
            &mut features,
            axioms,
            Some(f_axioms),
            Some(f_ax_archive),
            bank,
        )?;
    }
    let limits = create_default_spec_limits();
    spec_features_add_eval(&mut features, &limits);
    Ok(spec_type_print_string(&features, MERGED_CLASSIFY_MASK))
}

fn preprocess_real_input_clauses(
    config: &ClassifyProblemConfig,
    state: &mut ProofState,
) -> Result<(), Diagnostic> {
    clausify_real_input_formula_axioms(config, state)?;

    let mut tmp_bank = TermBank::new(state.terms().signature().clone())?;
    {
        let (bank, axioms, ax_archive) = state.terms_axioms_ax_archive_mut();
        let _archived = clause_set_archive_copy(ax_archive, axioms, bank)?;
    }
    let (bank, axioms, watchlist, archive) = state.terms_axioms_watchlist_archive_mut();
    if !config.no_preprocessing {
        let _removed = clause_set_preprocess(
            axioms,
            archive,
            &mut tmp_bank,
            bank,
            false,
            config.eqdef_incrlimit,
            config.eqdef_maxclauses,
        )?;
    }
    let _unfolded = clause_set_unfold_eq_def_normalize(
        axioms,
        watchlist,
        archive,
        &mut tmp_bank,
        bank,
        config.eqdef_incrlimit,
        config.eqdef_maxclauses,
    )?;
    Ok(())
}

fn clausify_real_input_formula_axioms(
    config: &ClassifyProblemConfig,
    state: &mut ProofState,
) -> Result<(), Diagnostic> {
    let options = FormulaSetCnfOptions::new(config.miniscope_limit, true, problem_type())
        .with_def_limit(config.formula_def_limit);
    clausify_real_input_formula_axioms_with_options(state, options)
}

fn clausify_real_input_formula_axioms_with_options(
    state: &mut ProofState,
    options: FormulaSetCnfOptions,
) -> Result<(), Diagnostic> {
    let fresh_vars = state.fresh_vars().clone();
    let (bank, axioms, f_axioms, f_ax_archive) = state.terms_axioms_formula_sets_cnf_mut();
    let _preprocessed = f_axioms.preproc_conjectures(bank, false, false)?;
    let _cnf = f_axioms.cnf2_into(f_ax_archive, axioms, bank, &fresh_vars, options)?;
    Ok(())
}

fn write_standard_real_input_classification(
    config: &ClassifyProblemConfig,
    file: &str,
    output: &mut impl Write,
    state: &mut ProofState,
    raw_features: &RawSpecFeatureCell,
) -> Result<(), Diagnostic> {
    let mut features = SpecFeatureCell::default();
    {
        let (bank, axioms, f_axioms, f_ax_archive) = state.terms_axioms_formula_sets_mut();
        spec_features_compute_with_choice_recognition(
            &mut features,
            axioms,
            Some(f_axioms),
            Some(f_ax_archive),
            bank,
        )?;
    }
    features.order = raw_features.order;
    features.goal_order = raw_features.conj_order;
    features.num_of_definitions = raw_features.num_of_definitions;
    features.perc_of_form_defs = raw_features.perc_of_form_defs;
    spec_features_add_eval(&mut features, &config.limits);

    if config.tptp_header {
        write_tptp_header(output, state, &features)
    } else {
        write_all(output, file.as_bytes())?;
        write_all(output, b" : ")?;
        write_all(output, spec_features_print_string(&features).as_bytes())?;
        write_all(output, b" : ")?;
        write_all(
            output,
            spec_type_print_string(&features, &config.mask).as_bytes(),
        )?;
        write_all(output, b"\n")
    }
}

fn write_specs_sig_real_input(
    file: &str,
    output: &mut impl Write,
    state: &mut ProofState,
) -> Result<(), Diagnostic> {
    let mut features = SpecSigFeatureCell::new();
    let (bank, axioms) = state.terms_and_axioms_mut();
    clause_set_collect_sig_features(bank, axioms, &mut features);
    write_all(output, file.as_bytes())?;
    write_all(output, b" : ")?;
    write_all(output, spec_sig_feature_format(&features).as_bytes())?;
    write_all(output, b" : \n")
}

fn write_tptp_header(
    output: &mut impl Write,
    state: &ProofState,
    features: &SpecFeatureCell,
) -> Result<(), Diagnostic> {
    let axioms = state.axioms();
    let signature = state.terms().signature();
    writeln_diag(
        output,
        &format!(
            "% Syntax   : Number of clauses    : {:4} ({:4} non-Horn; {:3} unit; {:3} RR)",
            features.clauses,
            features.clauses - features.horn,
            features.unit,
            clause_set_count_range_restricted(axioms)
        ),
    )?;
    writeln_diag(
        output,
        &format!(
            "%            Number of literals   : {:4} ({:4} equality)",
            features.literals,
            clause_set_count_eqn_literals(axioms)
        ),
    )?;
    let average_clause_size = if features.clauses == 0 {
        "-".to_owned()
    } else {
        format!("{:4}", features.literals / features.clauses)
    };
    writeln_diag(
        output,
        &format!(
            "%            Maximal clause size  : {:4} ({average_clause_size} average)",
            clause_set_max_literal_number(axioms)
        ),
    )?;

    let mut predicate_count = signature.count_symbols(true);
    let mut min_predicate_arity = signature.find_min_predicate_arity();
    let mut max_predicate_arity = signature.find_max_predicate_arity();
    if features.eq_content != SpecFeatureClass::NoEq {
        predicate_count += 1;
        min_predicate_arity = min_predicate_arity.min(2);
        max_predicate_arity = max_predicate_arity.max(2);
    }
    write_symbol_arity_line(
        output,
        "predicates",
        predicate_count,
        signature.count_arity_symbols(0, true),
        min_predicate_arity,
        max_predicate_arity,
        "propositional",
    )?;

    write_symbol_arity_line(
        output,
        "functors",
        signature.count_symbols(false),
        signature.count_arity_symbols(0, false),
        signature.find_min_function_arity(),
        signature.find_max_function_arity(),
        "constant",
    )?;

    writeln_diag(
        output,
        &format!(
            "%            Number of variables  : {:4} ({:4} singleton)",
            clause_set_count_variables(axioms),
            clause_set_count_singletons(axioms)
        ),
    )?;
    if features.literals != 0 {
        writeln_diag(
            output,
            &format!(
                "%            Maximal term depth   : {:4} ({:4} average)",
                features.clause_max_depth, features.clause_avg_depth
            ),
        )
    } else {
        writeln_diag(
            output,
            "%            Maximal term depth   :    - (   - average)",
        )
    }
}

fn write_symbol_arity_line(
    output: &mut impl Write,
    label: &str,
    symbol_count: i32,
    arity_zero_count: i32,
    min_arity: i32,
    max_arity: i32,
    zero_label: &str,
) -> Result<(), Diagnostic> {
    if symbol_count == 0 {
        writeln_diag(
            output,
            &format!(
                "%            Number of {label:<10} : {symbol_count:4} ({arity_zero_count:4} {zero_label}; --- arity)"
            ),
        )
    } else {
        writeln_diag(
            output,
            &format!(
                "%            Number of {label:<10} : {symbol_count:4} ({arity_zero_count:4} {zero_label}; {min_arity}-{max_arity} arity)"
            ),
        )
    }
}

fn parse_real_input_file(
    config: &ClassifyProblemConfig,
    file: &str,
    stdin: &mut impl Read,
    state: &mut ProofState,
) -> Result<(), Diagnostic> {
    let mut scanner = real_input_scanner(file, stdin)?;
    let (terms, f_axioms, watchlist) = state.terms_f_axioms_watchlist_mut();
    let watchlist = watchlist.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Cannot store inline watchlist clauses after the watchlist has been disabled",
        )
    })?;
    let parsed_file = parse_clause_scanner_into_formula_set_with_options(
        &mut scanner,
        config.parse_format,
        FormulaPreprocessing::parse_only(FoolUnroll::Enabled),
        Default::default(),
        terms,
        f_axioms,
        watchlist,
    )?;
    state.add_raw_formula_features(parsed_file.raw_formula_features);
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedFeatureLine {
    name: String,
    features: SpecFeatureCell,
}

#[derive(Clone, Debug, PartialEq)]
struct ParsedRawFeatureLine {
    name: String,
    features: RawSpecFeatureCell,
}

fn parse_feature_line(
    scanner: &mut Scanner,
    _limits: &SpecLimits,
) -> Result<ParsedFeatureLine, Diagnostic> {
    let name = parse_plain_filename(scanner)?;
    scanner.accept_tok(TokenType::COLON)?;
    let mut features = SpecFeatureCell::default();
    spec_features_parse(scanner, &mut features)?;
    Ok(ParsedFeatureLine { name, features })
}

fn parse_raw_feature_line(scanner: &mut Scanner) -> Result<ParsedRawFeatureLine, Diagnostic> {
    let name = parse_plain_filename(scanner)?;
    scanner.accept_tok(TokenType::COLON)?;
    let mut features = RawSpecFeatureCell::default();
    raw_spec_features_parse(scanner, &mut features)?;
    Ok(ParsedRawFeatureLine { name, features })
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, true)
    } else {
        Scanner::from_file(Path::new(name), true).map_err(classify_scanner_open_diagnostic)
    }
}

fn real_input_scanner(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, false)
    } else {
        Scanner::from_file(Path::new(name), false).map_err(classify_scanner_open_diagnostic)
    }
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
Read sets of clauses and classify them according to predefined criteria.\n\
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

enum ClassifyOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> ClassifyOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            classify_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for ClassifyOutput<'_, W> {
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

fn validate_mask_len(mask: &str, min_len: usize, message: &str) -> Result<(), Diagnostic> {
    if mask.len() < min_len {
        Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
    } else {
        Ok(())
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

fn classify_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn classify_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
        clausify_real_input_formula_axioms, parse_feature_line, parse_raw_feature_line,
        parse_real_input_file, process_options, run, ClassifyProblemConfig, RunCommand,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::basics::verbose::verbose_level;
    use crate::clauses::clause_props::{CP_INPUT_FORMULA, CP_TYPE_AXIOM};
    use crate::clauses::clausefunc::tformula_lit_alloc;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::formulasets::WrappedFormula;
    use crate::clauses::proofstate::proof_state_alloc;
    use crate::heuristics::clausesetfeatures::{
        spec_features_add_eval, spec_features_print_string, spec_type_string_for_problem,
    };
    use crate::heuristics::rawspecfeatures::{
        raw_spec_features_classify_for_problem_type, raw_spec_features_format,
    };
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::prover::version::VERSION;
    use crate::terms::signature::{
        FP_IGNORE_PROPS, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
    };
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::test_support::global_state_lock;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!(
                "classify-problem-{name}-{}.tmp",
                std::process::id()
            ))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn feature_line(name: &str) -> String {
        format!("{name} : (1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): UHSMG\n")
    }

    fn raw_feature_line(name: &str) -> String {
        format!("{name} : (1, 2, 3, 4, 5, 6, 7, 8, 0.125, 9, true, 2, 0, false): FSSMMLLCCSSNAA\n")
    }

    fn object_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().i_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .expect("test symbol type declaration succeeds");
        bank.create_const_term(f_code)
            .expect("test constant term is shared")
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> Result<(u8, String, String), ErrorCode> {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        match run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr) {
            Ok(status) => Ok((
                status,
                String::from_utf8(stdout).expect("stdout is utf8"),
                String::from_utf8(stderr).expect("stderr is utf8"),
            )),
            Err(error) => Err(error.code()),
        }
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--help"], "").expect("help succeeds");

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: classify_problem [options] [files]"));
        assert!(help.contains("--parse-features"));
        assert!(help.contains("Copyright (C) 1998-2009 by Stephan Schulz"));
        assert!(stderr.is_empty());

        let (status, version, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--version"], "").expect("version succeeds");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_feature_mode_masks_limits_and_files() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--parse-features",
                "--raw-class",
                "--verbose=2",
                "--tstp-format",
                "--free-numbers",
                "--free-objects",
                "--class-mask=aaaaaaaaaaaaa",
                "--raw-mask=aaaaaaaaaaaaaaa",
                "--ngu-absolute=false",
                "--ngu-few-limit=0.125",
                "--ngu-many-limit=0.875",
                "--ax-some-limit=7",
                "--farity-large-limit=13",
                "features.txt",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        let ClassifyProblemConfig {
            parse_features,
            raw_classify,
            parse_format,
            mask,
            raw_mask,
            free_symbol_properties,
            files,
            limits,
            ..
        } = *config;
        assert!(parse_features);
        assert!(raw_classify);
        assert_eq!(parse_format, IoFormat::Tstp);
        assert_eq!(mask, "aaaaaaaaaaaaa");
        assert_eq!(raw_mask, "aaaaaaaaaaaaaaa");
        assert!(free_symbol_properties
            .contains_all(FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT | FP_IS_OBJECT));
        assert_eq!(files, ["features.txt"]);
        assert!(!limits.ngu_absolute);
        assert!((limits.ngu_few_limit - 0.125).abs() < f64::EPSILON);
        assert!((limits.ngu_many_limit - 0.875).abs() < f64::EPSILON);
        assert_eq!(limits.ax_some_limit, 7);
        assert_eq!(limits.far_sum_large_limit, 13);
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());
    }

    #[test]
    fn invalid_mask_lengths_match_c_diagnostics() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME, "--class-mask=short"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option -c (--class-mask) requires at least 13-letter string as an argument"
        );

        let error = process_options([PROGRAM_NAME, "--raw-mask=short"], &mut stdout).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option -c (--class-mask) requires at least 11-letter string as an argument"
        );
    }

    #[test]
    fn parse_feature_line_reads_name_features_and_old_class() {
        let _guard = global_state_lock();
        let mut scanner =
            Scanner::from_user_string(&format!("{} tail", feature_line("prob/name")), false)
                .expect("scanner allocation");
        let limits = crate::heuristics::clausesetfeatures::create_default_spec_limits();

        let parsed = parse_feature_line(&mut scanner, &limits).expect("feature line parses");

        assert_eq!(parsed.name, "prob/name");
        assert_eq!(parsed.features.goals, 1);
        assert_eq!(parsed.features.clause_avg_depth, 20);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn parse_raw_feature_line_reads_name_features_and_old_class() {
        let _guard = global_state_lock();
        let mut scanner =
            Scanner::from_user_string(&format!("{} tail", raw_feature_line("raw/name")), false)
                .expect("scanner allocation");

        let parsed = parse_raw_feature_line(&mut scanner).expect("raw feature line parses");

        assert_eq!(parsed.name, "raw/name");
        assert_eq!(parsed.features.sentence_no, 1);
        assert_eq!(parsed.features.class, "FSSMMLLCCSSNAA");
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn stdin_parse_features_reclassifies_standard_feature_lines() {
        let _guard = global_state_lock();
        let input = feature_line("prob");
        let mut expected_scanner = Scanner::from_user_string(&input, false).unwrap();
        let limits = crate::heuristics::clausesetfeatures::create_default_spec_limits();
        let mut parsed = parse_feature_line(&mut expected_scanner, &limits).unwrap();
        spec_features_add_eval(&mut parsed.features, &limits);
        let expected = format!(
            "prob : {} : {}\n",
            spec_features_print_string(&parsed.features),
            spec_type_string_for_problem(
                &parsed.features,
                super::DEFAULT_CLASSIFY_MASK,
                ProblemType::FirstOrder,
            )
        );

        let (status, stdout, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--parse-features"], &input).expect("run succeeds");

        assert_eq!(status, 0);
        assert_eq!(stdout, expected);
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_parse_features_raw_class_reclassifies_raw_feature_lines() {
        let _guard = global_state_lock();
        let input = raw_feature_line("rawprob");
        let mut expected_scanner = Scanner::from_user_string(&input, false).unwrap();
        let limits = crate::heuristics::clausesetfeatures::create_default_spec_limits();
        let mut parsed = parse_raw_feature_line(&mut expected_scanner).unwrap();
        raw_spec_features_classify_for_problem_type(
            &mut parsed.features,
            &limits,
            Some(super::DEFAULT_RAW_MASK),
            ProblemType::FirstOrder,
        );
        let expected = format!("rawprob : {}\n", raw_spec_features_format(&parsed.features));

        let (status, stdout, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--parse-features", "--raw-class"], &input)
                .expect("run succeeds");

        assert_eq!(status, 0);
        assert_eq!(stdout, expected);
        assert!(stderr.is_empty());
    }

    #[test]
    fn file_inputs_and_output_file_follow_c_flow() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, feature_line("fromfile")).expect("input file is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--parse-features",
                "-o",
                output_path.to_str().expect("path is utf8"),
                input_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("file run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(output.starts_with("fromfile : ("));
        assert!(output.contains(" : F"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let input = feature_line("dashout");
        let mut stdin = Cursor::new(input.into_bytes());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--parse-features", "-o", "-"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("dash output run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(output.starts_with("dashout : ("));
        assert!(output.contains(" : F"));
    }

    #[test]
    fn output_file_is_created_before_later_input_open_failure() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-after-output-open");
        let output_path = temp_path("early-output");
        remove_if_present(&missing_path);
        remove_if_present(&output_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--parse-features",
                "-o",
                output_path.to_str().expect("path is utf8"),
                missing_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing input is still reported after output creation");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(
            std::fs::read_to_string(&output_path).expect("output file was created"),
            ""
        );

        remove_if_present(&output_path);
    }

    #[test]
    fn feature_input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-feature-input");
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--parse-features",
                missing_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing feature input file is reported");

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
    fn real_input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-real-input");
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-format",
                missing_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing real input file is reported");

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
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        remove_if_present(&output_path);
        _ = std::fs::remove_dir(&output_path);
        std::fs::create_dir(&output_path).expect("output fixture directory is created");
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--parse-features",
                "-o",
                output_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("directory output path is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error
            .message()
            .starts_with(&format!("Cannot open file {}", output_path.display())));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        std::fs::remove_dir(&output_path).expect("output fixture directory is removed");
    }

    #[test]
    fn malformed_feature_line_reports_syntax_error_without_output() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"broken-feature-line\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--parse-features"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("malformed feature line is reported");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Colon"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn formula_axioms_are_clausified_before_standard_clause_preprocessing() {
        let _guard = global_state_lock();
        let _problem_type_guard = super::ProblemTypeRunGuard::new();
        set_problem_type(ProblemType::FirstOrder).expect("problem type is initialized");
        let mut state =
            proof_state_alloc(FP_IGNORE_PROPS).expect("proof state allocation succeeds");
        let formula = {
            let bank = state.terms_mut();
            let left = object_const(bank, "classify_formula_left");
            let right = object_const(bank, "classify_formula_right");
            let literal = Eqn::alloc(left, right, bank, true).expect("literal allocation succeeds");
            tformula_lit_alloc(bank, &literal, ProblemType::FirstOrder)
                .expect("formula literal allocation succeeds")
        };
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_properties(CP_TYPE_AXIOM);
        state.f_axioms_mut().insert(wrapped);

        clausify_real_input_formula_axioms(&ClassifyProblemConfig::default(), &mut state)
            .expect("formula clausification succeeds");

        assert_eq!(state.axioms().members(), 1);
        assert_eq!(state.f_axioms().cardinality(), 0);
        assert_eq!(state.f_ax_archive().cardinality(), 2);
    }

    #[test]
    fn real_input_parser_preserves_supported_formula_owner_before_cnf() {
        let _guard = global_state_lock();
        let _problem_type_guard = super::ProblemTypeRunGuard::new();
        set_problem_type(ProblemType::FirstOrder).expect("problem type is initialized");
        let config = ClassifyProblemConfig {
            parse_format: IoFormat::Tstp,
            ..ClassifyProblemConfig::default()
        };
        let mut state =
            proof_state_alloc(FP_IGNORE_PROPS).expect("proof state allocation succeeds");
        let mut stdin: &[u8] = b"fof(classify_owner_ax, axiom, (p(a) | q(a))).\n";

        parse_real_input_file(&config, "-", &mut stdin, &mut state)
            .expect("real-input parsing succeeds");

        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.f_axioms().cardinality(), 1);
        let formula = state
            .f_axioms()
            .iter()
            .next()
            .expect("formula owner exists");
        assert!(!formula.is_clause());
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_AXIOM);

        clausify_real_input_formula_axioms(&config, &mut state)
            .expect("formula-owner CNF succeeds");
        assert_eq!(state.axioms().members(), 1);
        assert_eq!(state.f_axioms().cardinality(), 0);
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(feature_line("prob").into_bytes());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--parse-features"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_raw_class_parses_real_tstp_problem() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--raw-class", "--tstp-format"], input)
                .expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.starts_with("- : ("));
        assert!(stdout.ends_with('\n'));
        assert!(stdout.contains(" : F"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_standard_real_problem_classifies_supported_tstp_input() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--tstp-format"], input).expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.starts_with("- : ("));
        assert!(stdout.contains(" : F"));
        assert!(stdout.ends_with('\n'));
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_standard_real_problem_can_print_specs_sig_features() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--tstp-format", "--specsig"], input)
                .expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.starts_with("- : "));
        assert!(stdout.ends_with(" : \n"));
        assert!(stdout.matches(',').count() > 80);
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_standard_real_problem_can_print_tptp_header() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--tstp-format", "--generate-tptp-header"],
            input,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.starts_with("% Syntax   : Number of clauses"));
        assert!(stdout.contains("%            Number of predicates"));
        assert!(stdout.contains("%            Maximal term depth"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn merged_real_problem_classification_combines_raw_and_cnf_classes() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--tstp-format", "--merged-classification=1"],
            input,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        let classes = stdout
            .strip_prefix("- : (NULL) : ")
            .expect("merged output prefix")
            .trim_end();
        assert_eq!(classes.len(), 36);
        assert!(classes.starts_with('F'));
        assert!(stderr.is_empty());
    }

    #[test]
    fn merged_real_problem_zero_timeout_uses_c_hyphen_fallback_shape() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--tstp-format", "--merged-classification=0"],
            input,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.trim_end().ends_with(&"-".repeat(21)));
        assert!(stderr.is_empty());
    }

    #[test]
    fn merged_classification_minus_one_uses_standard_real_problem_path() {
        let _guard = global_state_lock();
        let input = "cnf(c1, axiom, (p(a))).\n";

        let (status, stdout, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--tstp-format", "--merged-classification=-1"],
            input,
        )
        .expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.starts_with("- : ("));
        assert!(!stdout.contains("(NULL)"));
        assert!(stderr.is_empty());
    }
}
