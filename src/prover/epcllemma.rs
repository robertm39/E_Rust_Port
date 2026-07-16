use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause::ClauseParseOptions;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::commandline::{
    get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::pcl2::expressions::PclOpCode;
use crate::pcl2::lemmas::{
    protocol_compute_lemma_weights, protocol_compute_proof_size, protocol_flat_find_lemmas,
    protocol_rec_find_lemmas, protocol_seq_find_lemmas, InferenceWeights, LemmaParams,
};
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStepParseOptions, PCL_IS_LEMMA};
use crate::prover::version::{E_URL, STS_MAIL, VERSION};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "epcllemma";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    TptpPrint,
    TptpFormat,
    TstpPrint,
    TstpFormat,
    LopPrint,
    IterativeLemmas,
    RecursiveLemmas,
    FlatLemmas,
    AbsLemmaLimit,
    RelLemmaLimit,
    AbsLemmaQualityLimit,
    RelLemmaQualityLimit,
    LemmaTreeBaseWeight,
    LemmaSizeBaseWeight,
    LemmaActivePmWeight,
    LemmaOtherGeneratingWeight,
    LemmaActiveSimplifyingWeight,
    LemmaPassiveSimplifiedWeight,
    NoReferenceWeights,
    LemmaHornBonus,
    InitialWeight,
    QuoteWeight,
    ParamodWeight,
    EResolutionWeight,
    EFactoringWeight,
    SimplifyReflectWeight,
    AcResolutionWeight,
    RewriteWeight,
    URewriteWeight,
    ClauseNormalizeWeight,
    SplitClauseWeight,
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
        OptionCode::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file.",
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
        "Select an output level, greater values imply more verbose output. Level 0 produces nearly no output, level 1 and 2 will print just lemmas, level 3 and higher will give a full protocol with lemmas marked as such.",
    ),
    OptCell::new(
        OptionCode::TptpPrint,
        None,
        Some("tptp-out"),
        OptArgType::NoArg,
        None,
        "Print lemma sets in TPTP-2 format instead of lop.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tptp-out (supplied for consistency in the E toolchain.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "Print lemma sets in TPTP-3 (TSTP) format instead of lop.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-out (supplied for consistency in the E toolchain. Note that this does not enable parsing of TPTP-3 proofs.",
    ),
    OptCell::new(
        OptionCode::LopPrint,
        None,
        Some("lop-out"),
        OptArgType::NoArg,
        None,
        "Print output in LOP format. This is only useful for output level 1, as LOP has no way of distinguishing lemmas and other clauses/formulas. It also is problematic for non-CNF first order proofs, as LOP has no good syntax for full first-order formulae.",
    ),
    OptCell::new(
        OptionCode::IterativeLemmas,
        Some('i'),
        Some("iterative-lemmas"),
        OptArgType::NoArg,
        None,
        "Use a simple iterative lemma generation algorithm that will traverse the PCL listing in a topological ordering (from axioms to leaf nodes), picking out lemmas that reach a certain score. Good for getting a reasonably even distribution of lemmata for proof presentation. This is the default behaviour (the option exists just for documentation purposes).",
    ),
    OptCell::new(
        OptionCode::RecursiveLemmas,
        Some('r'),
        Some("recursive-lemmas"),
        OptArgType::NoArg,
        None,
        "Use a recursive lemma generation algorithm that will pick out the lemma with the highest score, recompute scores, and repeat for a given number of steps. This may lead to very irregular proofs (because later lemmata may change the score of previous ones), but ensures that the lemma with the highest score is chosen.",
    ),
    OptCell::new(
        OptionCode::FlatLemmas,
        Some('f'),
        Some("flat-lemmas"),
        OptArgType::NoArg,
        None,
        "Compute lemma scores once and pick the N lemmas with the highest score. These are bound to be nodes that are close to the derivation graph boundary, so they are not necessarily good for strucuring the proof. They may be good for theory exploration, though. This algorithm is also O(n) in the number of PCL steps (well, there is a small O(log(n)) component, but its close enough), while the others may end up O(n^2) in the (unexpected) worst case.",
    ),
    OptCell::new(
        OptionCode::AbsLemmaLimit,
        Some('A'),
        Some("max-lemmas"),
        OptArgType::ReqArg,
        None,
        "Set the maximal number of lemmas to be selected absolutely.",
    ),
    OptCell::new(
        OptionCode::RelLemmaLimit,
        Some('R'),
        Some("max-lemmas-rel"),
        OptArgType::ReqArg,
        None,
        "Set the maximal number of lemmas to be selected as a fraction of the total number of PCL steps in the protocol (always overwritten if an absolute value is also provided).",
    ),
    OptCell::new(
        OptionCode::AbsLemmaQualityLimit,
        Some('q'),
        Some("min-lemma-quality"),
        OptArgType::ReqArg,
        None,
        "Set a mimimum lemma score absolutely. Steps with this or a higher score become lemmata unless another limit prohibits that.",
    ),
    OptCell::new(
        OptionCode::RelLemmaQualityLimit,
        Some('Q'),
        Some("min-lemma-quality-rel"),
        OptArgType::ReqArg,
        None,
        "Set a mimimum lemma score as a fraction of the best possible lemma score in the proof tree.",
    ),
    OptCell::new(
        OptionCode::LemmaTreeBaseWeight,
        Some('b'),
        Some("lemma-tree-base-weight"),
        OptArgType::ReqArg,
        None,
        "Set the base weight for the influence of references in the lemma quality evaluation. The larger it is in relation to the inference weights (below), the less important is the actual number of references. If you want to use only the lemma size, set this to 1 and the individual reference weights to 0 (using e.g. the --no-reference-weights option).",
    ),
    OptCell::new(
        OptionCode::LemmaSizeBaseWeight,
        None,
        Some("lemma-size-base-weight"),
        OptArgType::ReqArg,
        None,
        "Set the base weight for the influence of size in the lemma quality evaluation. The larger this is, the less important the actual size of the lemma becomes.",
    ),
    OptCell::new(
        OptionCode::LemmaActivePmWeight,
        Some('a'),
        Some("active-pm-weight"),
        OptArgType::ReqArg,
        None,
        "Determine the weight to use for each use of the clause as an active paramodulation partner (i.e. in a conditional rewrite step (if you follow a strictly equational paradigm (which I do))).",
    ),
    OptCell::new(
        OptionCode::LemmaOtherGeneratingWeight,
        Some('g'),
        Some("generating-inference-weight"),
        OptArgType::ReqArg,
        None,
        "Detemine the weight to give to references in generating infences other than active paramodulation inferences.",
    ),
    OptCell::new(
        OptionCode::LemmaActiveSimplifyingWeight,
        Some('S'),
        Some("simplifying-weight"),
        OptArgType::ReqArg,
        None,
        "Determine the weight to give to a reference to a clause used as a simplifying clause.",
    ),
    OptCell::new(
        OptionCode::LemmaPassiveSimplifiedWeight,
        Some('p'),
        Some("simplified-weight"),
        OptArgType::ReqArg,
        None,
        "Determine the weight of a reference where a clause is being simplified.",
    ),
    OptCell::new(
        OptionCode::NoReferenceWeights,
        Some('N'),
        Some("no-reference-weights"),
        OptArgType::NoArg,
        None,
        "Set all the weights given to references to 0. If the base weight (see above) is not 0, this leads to a pure size/prooftree evaluation.",
    ),
    OptCell::new(
        OptionCode::LemmaHornBonus,
        Some('H'),
        Some("horn-bonus"),
        OptArgType::ReqArg,
        None,
        "Weight factor to apply to the evaluation of Horn clauses. Use 1 to be fair, 2.5 if you think Horn clauses are 2.5 times more dandy than non-Horn clauses. Yes, nice lemmas _are_ amatter of taste ;-).",
    ),
    OptCell::new(
        OptionCode::InitialWeight,
        None,
        Some("pcl-initial-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of an 'initial' pseudo-inference for computing the weight of a PLC proof tree. This is probably best left untouched.",
    ),
    OptCell::new(
        OptionCode::QuoteWeight,
        None,
        Some("pcl-quote-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a  quote  pseudo-inference for computing the weight of a PLC proof tree. This is probably best left untouched.",
    ),
    OptCell::new(
        OptionCode::ParamodWeight,
        None,
        Some("pcl-paramod-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a paramodulation inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::EResolutionWeight,
        None,
        Some("pcl-eres-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of an equality resolution inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::EFactoringWeight,
        None,
        Some("pcl-efact-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of an equality factoring inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::SimplifyReflectWeight,
        None,
        Some("pcl-sr-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a simplify-reflect inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::AcResolutionWeight,
        None,
        Some("pcl-acres-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of an AC resolution inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::RewriteWeight,
        None,
        Some("pcl-rw-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a rewrite inference for computing the weight of a PLC proof tree.",
    ),
    OptCell::new(
        OptionCode::URewriteWeight,
        None,
        Some("pcl-urw-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a underspecified rewrite inference for computing the weight of a PLC proof tree. Such an inference describes an unspecified number of rewrite steps using the same unit clause as a rewrite rule. Normal E PCL listings should no longer contain such inferences.",
    ),
    OptCell::new(
        OptionCode::ClauseNormalizeWeight,
        None,
        Some("pcl-cn-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a clause normalization inference for computing the weight of a PLC proof tree. This is probably best left alone, since most clause normalization is implicit anyways.",
    ),
    OptCell::new(
        OptionCode::SplitClauseWeight,
        None,
        Some("pcl-split-weight"),
        OptArgType::ReqArg,
        None,
        "Set the weight of a splitting pseudo-inference for computing the weight of a PLC proof tree.",
    ),
];

#[derive(Clone, Debug, PartialEq)]
struct EpclLemmaConfig {
    output_file: Option<PathBuf>,
    output_level: i64,
    output_format: ProofDocOutputFormat,
    algorithm: LemmaAlgorithm,
    params: LemmaParams,
    weights: InferenceWeights,
    max_lemmas: i64,
    max_lemmas_rel: f32,
    max_lemmas_rel_enabled: bool,
    min_quality: f32,
    min_quality_rel: f32,
    min_quality_rel_enabled: bool,
    files: Vec<String>,
}

impl Default for EpclLemmaConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            output_level: 1,
            output_format: ProofDocOutputFormat::Pcl,
            algorithm: LemmaAlgorithm::Iterative,
            params: LemmaParams::default(),
            weights: InferenceWeights::default(),
            max_lemmas: 0,
            max_lemmas_rel: 0.001,
            max_lemmas_rel_enabled: true,
            min_quality: 100.0,
            min_quality_rel: 0.3,
            min_quality_rel_enabled: false,
            files: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LemmaAlgorithm {
    Iterative,
    Recursive,
    Flat,
}

enum RunCommand {
    Execute(Box<EpclLemmaConfig>),
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
        RunCommand::Execute(config) => execute_epcllemma(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EpclLemmaConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        if let Some(command) = handle_option(&mut config, parsed.option(), parsed.arg(), stdout)? {
            return Ok(command);
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(Box::new(config)))
}

fn handle_option(
    config: &mut EpclLemmaConfig,
    option: &OptCell<OptionCode>,
    arg: Option<&str>,
    stdout: &mut impl Write,
) -> Result<Option<RunCommand>, Diagnostic> {
    match option.option_code {
        OptionCode::Help
        | OptionCode::Version
        | OptionCode::Verbose
        | OptionCode::Output
        | OptionCode::Silent
        | OptionCode::OutputLevel => handle_general_option(config, option, arg, stdout),
        OptionCode::TptpPrint
        | OptionCode::TptpFormat
        | OptionCode::TstpPrint
        | OptionCode::TstpFormat
        | OptionCode::LopPrint
        | OptionCode::IterativeLemmas
        | OptionCode::RecursiveLemmas
        | OptionCode::FlatLemmas => {
            handle_format_algorithm_option(config, option.option_code);
            Ok(None)
        }
        OptionCode::AbsLemmaLimit
        | OptionCode::RelLemmaLimit
        | OptionCode::AbsLemmaQualityLimit
        | OptionCode::RelLemmaQualityLimit => {
            handle_limit_option(config, option, arg)?;
            Ok(None)
        }
        _ => {
            handle_weight_option(config, option, arg)?;
            Ok(None)
        }
    }
}

fn handle_general_option(
    config: &mut EpclLemmaConfig,
    option: &OptCell<OptionCode>,
    arg: Option<&str>,
    stdout: &mut impl Write,
) -> Result<Option<RunCommand>, Diagnostic> {
    match option.option_code {
        OptionCode::Verbose => {
            let level = get_int_arg(option, arg.unwrap_or(""))?;
            set_verbose_level(i64_to_i32_saturating(level));
            Ok(None)
        }
        OptionCode::Help => {
            write_all(stdout, print_help().as_bytes())?;
            Ok(Some(RunCommand::Exit(0)))
        }
        OptionCode::Version => {
            writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION}"))?;
            Ok(Some(RunCommand::Exit(0)))
        }
        OptionCode::Output => {
            config.output_file = arg.map(PathBuf::from);
            Ok(None)
        }
        OptionCode::Silent => {
            config.output_level = 0;
            let _ = set_output_level(0);
            Ok(None)
        }
        OptionCode::OutputLevel => {
            let level = get_int_arg(option, arg.unwrap_or(""))?;
            config.output_level = level;
            let _ = set_output_level(level);
            Ok(None)
        }
        _ => unreachable!("non-general option routed to handle_general_option"),
    }
}

fn handle_format_algorithm_option(config: &mut EpclLemmaConfig, code: OptionCode) {
    match code {
        OptionCode::TptpPrint | OptionCode::TptpFormat => {
            config.output_format = ProofDocOutputFormat::Tptp;
        }
        OptionCode::TstpPrint | OptionCode::TstpFormat => {
            config.output_format = ProofDocOutputFormat::Tstp;
        }
        OptionCode::LopPrint => {
            config.output_format = ProofDocOutputFormat::Lop;
            config.algorithm = LemmaAlgorithm::Iterative;
        }
        OptionCode::IterativeLemmas => {
            config.algorithm = LemmaAlgorithm::Iterative;
        }
        OptionCode::RecursiveLemmas => {
            config.algorithm = LemmaAlgorithm::Recursive;
        }
        OptionCode::FlatLemmas => {
            config.algorithm = LemmaAlgorithm::Flat;
        }
        _ => unreachable!("non-format/algorithm option routed to handler"),
    }
}

fn handle_limit_option(
    config: &mut EpclLemmaConfig,
    option: &OptCell<OptionCode>,
    arg: Option<&str>,
) -> Result<(), Diagnostic> {
    match option.option_code {
        OptionCode::AbsLemmaLimit => {
            config.max_lemmas = get_int_arg(option, arg.unwrap_or(""))?;
            config.max_lemmas_rel_enabled = false;
        }
        OptionCode::RelLemmaLimit => {
            config.max_lemmas_rel = get_float_arg_f32(option, arg)?;
            config.max_lemmas_rel_enabled = true;
        }
        OptionCode::AbsLemmaQualityLimit => {
            config.min_quality = get_float_arg_f32(option, arg)?;
            config.min_quality_rel_enabled = false;
        }
        OptionCode::RelLemmaQualityLimit => {
            config.min_quality_rel = get_float_arg_f32(option, arg)?;
            config.min_quality_rel_enabled = true;
        }
        _ => unreachable!("non-limit option routed to handle_limit_option"),
    }
    Ok(())
}

fn handle_weight_option(
    config: &mut EpclLemmaConfig,
    option: &OptCell<OptionCode>,
    arg: Option<&str>,
) -> Result<(), Diagnostic> {
    match option.option_code {
        OptionCode::LemmaTreeBaseWeight => {
            config.params.tree_base_weight = get_int_arg(option, arg.unwrap_or(""))?;
        }
        OptionCode::LemmaSizeBaseWeight => {
            config.params.size_base_weight = get_int_arg(option, arg.unwrap_or(""))?;
        }
        OptionCode::LemmaActivePmWeight => {
            config.params.act_pm_w = get_float_arg_f32(option, arg)?;
        }
        OptionCode::LemmaOtherGeneratingWeight => {
            config.params.o_gen_w = get_float_arg_f32(option, arg)?;
        }
        OptionCode::LemmaActiveSimplifyingWeight => {
            config.params.act_simpl_w = get_float_arg_f32(option, arg)?;
        }
        OptionCode::LemmaPassiveSimplifiedWeight => {
            config.params.pas_simpl_w = get_float_arg_f32(option, arg)?;
        }
        OptionCode::NoReferenceWeights => {
            config.params.act_pm_w = 0.0;
            config.params.o_gen_w = 0.0;
            config.params.pas_simpl_w = 0.0;
        }
        OptionCode::LemmaHornBonus => {
            config.params.horn_bonus = get_float_arg_f32(option, arg)?;
        }
        _ => handle_inference_weight_option(config, option, arg)?,
    }
    Ok(())
}

fn handle_inference_weight_option(
    config: &mut EpclLemmaConfig,
    option: &OptCell<OptionCode>,
    arg: Option<&str>,
) -> Result<(), Diagnostic> {
    let op = match option.option_code {
        OptionCode::InitialWeight => PclOpCode::Initial,
        OptionCode::QuoteWeight => PclOpCode::Quote,
        OptionCode::ParamodWeight => PclOpCode::Paramod,
        OptionCode::EResolutionWeight => PclOpCode::EResolution,
        OptionCode::EFactoringWeight => PclOpCode::EFactoring,
        OptionCode::SimplifyReflectWeight => PclOpCode::SimplifyReflect,
        OptionCode::AcResolutionWeight => PclOpCode::ACResolution,
        OptionCode::RewriteWeight => PclOpCode::Rewrite,
        OptionCode::URewriteWeight => PclOpCode::URewrite,
        OptionCode::ClauseNormalizeWeight => PclOpCode::ClauseNormalize,
        OptionCode::SplitClauseWeight => PclOpCode::SplitClause,
        _ => unreachable!("non-weight option routed to handle_inference_weight_option"),
    };
    set_weight_from_arg(&mut config.weights, op, option, arg)
}

fn execute_epcllemma(
    config: &EpclLemmaConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = LemmaOutput::open(config.output_file.as_deref())?;
    let mut protocol = PclProtocol::new()?;

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        protocol.parse(&mut scanner, parse_options())?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }

    let max_lemmas = if config.max_lemmas_rel_enabled {
        relative_lemma_limit(protocol.step_count(), config.max_lemmas_rel)
    } else {
        config.max_lemmas
    };
    writeln_diag(stdout, &format!("% Selecting at most {max_lemmas} lemmas"))?;

    let min_quality = if config.min_quality_rel_enabled {
        protocol_compute_proof_size(&mut protocol, config.weights, false)?;
        let best = protocol_compute_lemma_weights(&mut protocol, config.params);
        best.and_then(|id| {
            protocol
                .find_step(&id)
                .map(|step| step.tree_data().lemma_quality * config.min_quality_rel)
        })
        .unwrap_or(0.0)
    } else {
        config.min_quality
    };
    writeln_diag(
        stdout,
        &format!(
            "% Minimum lemma quality: {}",
            format_c_fixed_f32(min_quality)
        ),
    )?;

    match config.algorithm {
        LemmaAlgorithm::Recursive => {
            protocol_rec_find_lemmas(
                &mut protocol,
                config.params,
                config.weights,
                max_lemmas,
                min_quality,
            )?;
        }
        LemmaAlgorithm::Iterative => {
            protocol_seq_find_lemmas(
                &mut protocol,
                config.params,
                config.weights,
                max_lemmas,
                min_quality,
            )?;
        }
        LemmaAlgorithm::Flat => {
            protocol_flat_find_lemmas(
                &mut protocol,
                config.params,
                config.weights,
                max_lemmas,
                min_quality,
            )?;
        }
    }

    match config.output_level {
        0 => {}
        1 | 2 => output.write_all(
            stdout,
            protocol
                .print_property_steps_string(
                    PCL_IS_LEMMA,
                    config.output_format,
                    ProblemType::FirstOrder,
                )?
                .as_bytes(),
        )?,
        _ => output.write_all(
            stdout,
            protocol
                .print_extra_string(false, config.output_format, ProblemType::FirstOrder)?
                .as_bytes(),
        )?,
    }

    output.flush(stdout)?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(0)
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
        Scanner::from_file(Path::new(name), true).map_err(epcllemma_scanner_open_diagnostic)?
    };
    scanner.set_format(IoFormat::Tptp);
    Ok(scanner)
}

fn set_weight_from_arg<Code>(
    weights: &mut InferenceWeights,
    op: PclOpCode,
    option: &OptCell<Code>,
    arg: Option<&str>,
) -> Result<(), Diagnostic> {
    weights.set(op, get_int_arg(option, arg.unwrap_or(""))?);
    Ok(())
}

fn get_float_arg_f32<Code>(option: &OptCell<Code>, arg: Option<&str>) -> Result<f32, Diagnostic> {
    Ok(f64_to_f32(get_float_arg(option, arg.unwrap_or(""))?))
}

#[allow(clippy::cast_possible_truncation)]
fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn relative_lemma_limit(step_count: usize, relative: f32) -> i64 {
    (f64::from(step_count as f32 * relative) + 0.99) as i64
}

fn format_c_fixed_f32(value: f32) -> String {
    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan".to_owned()
        } else {
            "nan".to_owned()
        };
    }
    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_owned()
        } else {
            "inf".to_owned()
        };
    }
    format!("{value:.6}")
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
Read an UPCL2 protocol and suggest certain steps as lemmas.\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str(&legacy_footer());
    result
}

fn legacy_footer() -> String {
    format!(
        concat!(
            "\n",
            "Copyright (C) 2003-2005 by Stephan Schulz, {sts_mail}\n",
            "\n",
            "                                                                      This program is a part of the support structure for the E equational\n",
            "  theorem prover. You can find the latest version of the E distribution\n",
            " as well as additional information at\n",
            "{e_url}\n",
            "\n",
            "This program is free software; you can redistribute it and/or modify\n",
            "it under the terms of the GNU General Public License as published by\n",
            "  the Free Software Foundation; either version 2 of the License, or\n",
            "     (at your option) any later version.\n",
            "\n",
            "                                                                      This program is distributed in the hope that it will be useful,\n",
            "       but WITHOUT ANY WARRANTY; without even the implied warranty of\n",
            "        MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n",
            "         GNU General Public License for more details.\n",
            "\n",
            "                                                                      You should have received a copy of the GNU General Public License\n",
            "     along with this program (it should be contained in the top level\n",
            "      directory of the distribution in the file COPYING); if not, write to\n",
            "  the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n",
            "       Boston, MA  02111-1307 USA\n",
            "\n",
            "                                                                      The original copyright holder can be contacted as\n",
            "\n",
            "Stephan Schulz\n",
            "DHBW Stuttgart\n",
            "Fakultaet Technik\n",
            "Informatik\n",
            "Lerchenstrasse 1\n",
            "70174 Stuttgart\n",
            "Germany\n",
            "\n",
        ),
        sts_mail = STS_MAIL,
        e_url = E_URL,
    )
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

struct LemmaOutput {
    file: Option<File>,
}

impl LemmaOutput {
    fn open(path: Option<&Path>) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self { file: None });
        };
        if path == Path::new("-") {
            return Ok(Self { file: None });
        }
        File::create(path)
            .map(|file| Self { file: Some(file) })
            .map_err(|error| {
                epcllemma_sys_error_diagnostic(
                    format!("Cannot open file {}", path.display()),
                    &error,
                )
            })
    }

    fn write_all(&mut self, stdout: &mut impl Write, bytes: &[u8]) -> Result<(), Diagnostic> {
        match &mut self.file {
            Some(file) => write_all(file, bytes),
            None => write_all(stdout, bytes),
        }
    }

    fn flush(&mut self, stdout: &mut impl Write) -> Result<(), Diagnostic> {
        match &mut self.file {
            Some(file) => file.flush(),
            None => stdout.flush(),
        }
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))
    }
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

fn epcllemma_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn epcllemma_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
        parse_options, print_help, relative_lemma_limit, run, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::VERSION;
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

    const SAMPLE_PROTOCOL: &str = "\
1 : : [++p(a)] : initial
2 : : [++q(a)] : initial
3 : : [++r(a)] : pm(1,2)
4 : : [++s(a)] : pm(1,3)
5 : : [++t(a)] : er(4)
";

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("epcllemma-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        format!(
            concat!(
                "\n",
                "\n",
                "epcllemma {version}\n",
                "\n",
                "Usage: epcllemma [options] [files]\n",
                "\n",
                "Read an UPCL2 protocol and suggest certain steps as lemmas.\n",
                "Options\n",
                "\n",
                "   -h\n",
                "  --help\n",
                "    Print a short description of program usage and options.\n",
                "\n",
                "  --version\n",
                "    Print the version number of the program.\n",
                "\n",
                "   -v\n",
                "  --verbose[=<arg>]\n",
                "    Verbose comments on the progress of the program. The short form or the\n",
                "    long form without the optional argument is equivalent to --verbose=1.\n",
                "\n",
                "   -o <arg>\n",
                "  --output-file=<arg>\n",
                "    Redirect output into the named file.\n",
                "\n",
                "   -s\n",
                "  --silent\n",
                "    Equivalent to --output-level=0.\n",
                "\n",
                "   -l <arg>\n",
                "  --output-level=<arg>\n",
                "    Select an output level, greater values imply more verbose output. Level 0\n",
                "    produces nearly no output, level 1 and 2 will print just lemmas, level 3\n",
                "    and higher will give a full protocol with lemmas marked as such.\n",
                "\n",
                "  --tptp-out\n",
                "    Print lemma sets in TPTP-2 format instead of lop.\n",
                "\n",
                "  --tptp-format\n",
                "    Equivalent to --tptp-out (supplied for consistency in the E toolchain.\n",
                "\n",
                "  --tstp-out\n",
                "    Print lemma sets in TPTP-3 (TSTP) format instead of lop.\n",
                "\n",
                "  --tstp-format\n",
                "    Equivalent to --tstp-out (supplied for consistency in the E toolchain.\n",
                "    Note that this does not enable parsing of TPTP-3 proofs.\n",
                "\n",
                "  --lop-out\n",
                "    Print output in LOP format. This is only useful for output level 1, as\n",
                "    LOP has no way of distinguishing lemmas and other clauses/formulas. It\n",
                "    also is problematic for non-CNF first order proofs, as LOP has no good\n",
                "    syntax for full first-order formulae.\n",
                "\n",
                "   -i\n",
                "  --iterative-lemmas\n",
                "    Use a simple iterative lemma generation algorithm that will traverse the\n",
                "    PCL listing in a topological ordering (from axioms to leaf nodes),\n",
                "    picking out lemmas that reach a certain score. Good for getting a\n",
                "    reasonably even distribution of lemmata for proof presentation. This is\n",
                "    the default behaviour (the option exists just for documentation\n",
                "    purposes).\n",
                "\n",
                "   -r\n",
                "  --recursive-lemmas\n",
                "    Use a recursive lemma generation algorithm that will pick out the lemma\n",
                "    with the highest score, recompute scores, and repeat for a given number\n",
                "    of steps. This may lead to very irregular proofs (because later lemmata\n",
                "    may change the score of previous ones), but ensures that the lemma with\n",
                "    the highest score is chosen.\n",
                "\n",
                "   -f\n",
                "  --flat-lemmas\n",
                "    Compute lemma scores once and pick the N lemmas with the highest score.\n",
                "    These are bound to be nodes that are close to the derivation graph\n",
                "    boundary, so they are not necessarily good for strucuring the proof. They\n",
                "    may be good for theory exploration, though. This algorithm is also O(n)\n",
                "    in the number of PCL steps (well, there is a small O(log(n)) component,\n",
                "    but its close enough), while the others may end up O(n^2) in the\n",
                "    (unexpected) worst case.\n",
                "\n",
                "   -A <arg>\n",
                "  --max-lemmas=<arg>\n",
                "    Set the maximal number of lemmas to be selected absolutely.\n",
                "\n",
                "   -R <arg>\n",
                "  --max-lemmas-rel=<arg>\n",
                "    Set the maximal number of lemmas to be selected as a fraction of the\n",
                "    total number of PCL steps in the protocol (always overwritten if an\n",
                "    absolute value is also provided).\n",
                "\n",
                "   -q <arg>\n",
                "  --min-lemma-quality=<arg>\n",
                "    Set a mimimum lemma score absolutely. Steps with this or a higher score\n",
                "    become lemmata unless another limit prohibits that.\n",
                "\n",
                "   -Q <arg>\n",
                "  --min-lemma-quality-rel=<arg>\n",
                "    Set a mimimum lemma score as a fraction of the best possible lemma score\n",
                "    in the proof tree.\n",
                "\n",
                "   -b <arg>\n",
                "  --lemma-tree-base-weight=<arg>\n",
                "    Set the base weight for the influence of references in the lemma quality\n",
                "    evaluation. The larger it is in relation to the inference weights\n",
                "    (below), the less important is the actual number of references. If you\n",
                "    want to use only the lemma size, set this to 1 and the individual\n",
                "    reference weights to 0 (using e.g. the --no-reference-weights option).\n",
                "\n",
                "  --lemma-size-base-weight=<arg>\n",
                "    Set the base weight for the influence of size in the lemma quality\n",
                "    evaluation. The larger this is, the less important the actual size of the\n",
                "    lemma becomes.\n",
                "\n",
                "   -a <arg>\n",
                "  --active-pm-weight=<arg>\n",
                "    Determine the weight to use for each use of the clause as an active\n",
                "    paramodulation partner (i.e. in a conditional rewrite step (if you follow\n",
                "    a strictly equational paradigm (which I do))).\n",
                "\n",
                "   -g <arg>\n",
                "  --generating-inference-weight=<arg>\n",
                "    Detemine the weight to give to references in generating infences other\n",
                "    than active paramodulation inferences.\n",
                "\n",
                "   -S <arg>\n",
                "  --simplifying-weight=<arg>\n",
                "    Determine the weight to give to a reference to a clause used as a\n",
                "    simplifying clause.\n",
                "\n",
                "   -p <arg>\n",
                "  --simplified-weight=<arg>\n",
                "    Determine the weight of a reference where a clause is being simplified.\n",
                "\n",
                "   -N\n",
                "  --no-reference-weights\n",
                "    Set all the weights given to references to 0. If the base weight (see\n",
                "    above) is not 0, this leads to a pure size/prooftree evaluation.\n",
                "\n",
                "   -H <arg>\n",
                "  --horn-bonus=<arg>\n",
                "    Weight factor to apply to the evaluation of Horn clauses. Use 1 to be\n",
                "    fair, 2.5 if you think Horn clauses are 2.5 times more dandy than\n",
                "    non-Horn clauses. Yes, nice lemmas _are_ amatter of taste ;-).\n",
                "\n",
                "  --pcl-initial-weight=<arg>\n",
                "    Set the weight of an 'initial' pseudo-inference for computing the weight\n",
                "    of a PLC proof tree. This is probably best left untouched.\n",
                "\n",
                "  --pcl-quote-weight=<arg>\n",
                "    Set the weight of a  quote  pseudo-inference for computing the weight of\n",
                "    a PLC proof tree. This is probably best left untouched.\n",
                "\n",
                "  --pcl-paramod-weight=<arg>\n",
                "    Set the weight of a paramodulation inference for computing the weight of\n",
                "    a PLC proof tree.\n",
                "\n",
                "  --pcl-eres-weight=<arg>\n",
                "    Set the weight of an equality resolution inference for computing the\n",
                "    weight of a PLC proof tree.\n",
                "\n",
                "  --pcl-efact-weight=<arg>\n",
                "    Set the weight of an equality factoring inference for computing the\n",
                "    weight of a PLC proof tree.\n",
                "\n",
                "  --pcl-sr-weight=<arg>\n",
                "    Set the weight of a simplify-reflect inference for computing the weight\n",
                "    of a PLC proof tree.\n",
                "\n",
                "  --pcl-acres-weight=<arg>\n",
                "    Set the weight of an AC resolution inference for computing the weight of\n",
                "    a PLC proof tree.\n",
                "\n",
                "  --pcl-rw-weight=<arg>\n",
                "    Set the weight of a rewrite inference for computing the weight of a PLC\n",
                "    proof tree.\n",
                "\n",
                "  --pcl-urw-weight=<arg>\n",
                "    Set the weight of a underspecified rewrite inference for computing the\n",
                "    weight of a PLC proof tree. Such an inference describes an unspecified\n",
                "    number of rewrite steps using the same unit clause as a rewrite rule.\n",
                "    Normal E PCL listings should no longer contain such inferences.\n",
                "\n",
                "  --pcl-cn-weight=<arg>\n",
                "    Set the weight of a clause normalization inference for computing the\n",
                "    weight of a PLC proof tree. This is probably best left alone, since most\n",
                "    clause normalization is implicit anyways.\n",
                "\n",
                "  --pcl-split-weight=<arg>\n",
                "    Set the weight of a splitting pseudo-inference for computing the weight\n",
                "    of a PLC proof tree.\n",
                "\n",
                "\n",
                "Copyright (C) 2003-2005 by Stephan Schulz, schulz@eprover.org\n",
                "\n",
                "                                                                      This program is a part of the support structure for the E equational\n",
                "  theorem prover. You can find the latest version of the E distribution\n",
                " as well as additional information at\n",
                "http://www.eprover.org\n",
                "\n",
                "This program is free software; you can redistribute it and/or modify\n",
                "it under the terms of the GNU General Public License as published by\n",
                "  the Free Software Foundation; either version 2 of the License, or\n",
                "     (at your option) any later version.\n",
                "\n",
                "                                                                      This program is distributed in the hope that it will be useful,\n",
                "       but WITHOUT ANY WARRANTY; without even the implied warranty of\n",
                "        MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n",
                "         GNU General Public License for more details.\n",
                "\n",
                "                                                                      You should have received a copy of the GNU General Public License\n",
                "     along with this program (it should be contained in the top level\n",
                "      directory of the distribution in the file COPYING); if not, write to\n",
                "  the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n",
                "       Boston, MA  02111-1307 USA\n",
                "\n",
                "                                                                      The original copyright holder can be contacted as\n",
                "\n",
                "Stephan Schulz\n",
                "DHBW Stuttgart\n",
                "Fakultaet Technik\n",
                "Informatik\n",
                "Lerchenstrasse 1\n",
                "70174 Stuttgart\n",
                "Germany\n",
                "\n",
            ),
            version = VERSION,
        )
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("epcllemma run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_exit_before_processing_input() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_stdin(&[PROGRAM_NAME, "--help"], "not pcl");
        assert_eq!(status, 0);
        assert_eq!(help, expected_help());
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "--version"], "not pcl");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn parse_options_match_c_pcl_global_switches() {
        assert!(!parse_options().support_shell_pcl);
        assert!(
            !parse_options()
                .clause_parse_options
                .clauses_have_local_variables
        );
    }

    #[test]
    fn short_v_is_not_a_version_option() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME, "-V"], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("C epcllemma has no -V shorthand");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("Unknown Option: -V"));
        assert!(stdout.is_empty());
    }

    #[test]
    fn iterative_default_selects_and_prints_lemma_steps() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--max-lemmas=0", "--min-lemma-quality=0"],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(
            output.starts_with("% Selecting at most 0 lemmas\n% Minimum lemma quality: 0.000000\n")
        );
        assert!(output.contains("      1 : lemma : [++p(a)] : initial : 'lemma'\n"));
        assert!(!output.contains("      5 :"));
    }

    #[test]
    fn output_file_receives_lemmas_but_status_stays_on_stdout() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, SAMPLE_PROTOCOL).expect("input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--max-lemmas=0",
                "--min-lemma-quality=0",
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
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            "% Selecting at most 0 lemmas\n% Minimum lemma quality: 0.000000\n"
        );
        assert!(stderr.is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(output.contains("      1 : lemma : [++p(a)] : initial : 'lemma'\n"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_lemmas_and_status_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(SAMPLE_PROTOCOL.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--max-lemmas=0",
                "--min-lemma-quality=0",
                "-o",
                "-",
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("dash output run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(
            output.starts_with("% Selecting at most 0 lemmas\n% Minimum lemma quality: 0.000000\n")
        );
        assert!(output.contains("      1 : lemma : [++p(a)] : initial : 'lemma'\n"));
    }

    #[test]
    fn input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-input");
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, missing_path.to_str().expect("path is utf8")],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing input file is reported");

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
        let output_path = temp_path("early-output");
        let missing_path = temp_path("missing-after-output");
        remove_if_present(&output_path);
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("path is utf8"),
                missing_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing input file is reported after output creation");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(output_path.exists());
        assert_eq!(
            std::fs::read_to_string(&output_path).expect("output file is readable"),
            ""
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        remove_if_present(&output_path);
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
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(SAMPLE_PROTOCOL.as_bytes().to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--max-lemmas=0", "--min-lemma-quality=0"],
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
    fn silent_output_level_keeps_status_lines_only() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--silent",
                "--max-lemmas=0",
                "--min-lemma-quality=0",
            ],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert_eq!(
            output,
            "% Selecting at most 0 lemmas\n% Minimum lemma quality: 0.000000\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn high_output_level_prints_full_protocol_with_marked_lemmas() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--max-lemmas=0",
                "--min-lemma-quality=0",
                "--output-level=3",
            ],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("      1 : lemma : [++p(a)] : initial : 'lemma'\n"));
        assert!(output.contains("      5 :  : [++t(a)] : er(4)\n"));
    }

    #[test]
    fn empty_input_preserves_status_lines_without_lemmas() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], "");

        assert_eq!(status, 0);
        assert_eq!(
            output,
            "% Selecting at most 0 lemmas\n% Minimum lemma quality: 100.000000\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn large_protocol_uses_c_single_precision_relative_limit() {
        let _guard = global_state_lock();
        let input = (1..=1_010)
            .map(|id| format!("{id} : : [++p(a)] : initial\n"))
            .collect::<String>();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--min-lemma-quality=0"], &input);

        assert_eq!(relative_lemma_limit(1_010, 0.001), 1);
        assert_eq!(status, 0);
        assert!(
            output.starts_with("% Selecting at most 1 lemmas\n% Minimum lemma quality: 0.000000\n")
        );
        assert_eq!(output.matches(" : lemma : ").count(), 2);
        assert!(output.contains("      1 : lemma : [++p(a)] : initial : 'lemma'\n"));
        assert!(output.contains("      2 : lemma : [++p(a)] : initial : 'lemma'\n"));
        assert!(!output.contains("      3 : lemma :"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn unusual_minimum_quality_values_use_c_printf_spelling() {
        let _guard = global_state_lock();
        for (argument, expected) in [
            ("--min-lemma-quality=nan", "nan"),
            ("--min-lemma-quality=inf", "inf"),
            ("--min-lemma-quality=-inf", "-inf"),
            ("--min-lemma-quality=-0", "-0.000000"),
        ] {
            let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, argument], "");

            assert_eq!(status, 0, "argument {argument}");
            assert_eq!(
                output,
                format!("% Selecting at most 0 lemmas\n% Minimum lemma quality: {expected}\n"),
                "argument {argument}"
            );
            assert!(stderr.is_empty(), "argument {argument}");
        }
    }

    #[test]
    fn high_output_level_prints_formula_steps() {
        let _guard = global_state_lock();
        let input = "1 : : p(a) : initial\n";
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--output-level=3"], input);

        assert_eq!(status, 0);
        assert!(output.contains("% Selecting at most 0 lemmas\n"));
        assert!(output.contains("      1 :  : p(a) : initial\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn formula_lemmas_render_in_every_c_output_format() {
        let _guard = global_state_lock();
        let input = "1 : : p(a) : initial\n2 : : q(a) : 1\n";
        for (format_argument, expected) in [
            (None, "      1 : lemma : p(a) : initial : 'lemma'\n"),
            (Some("--tptp-out"), "input_formula(1,lemma,p(a))\n"),
            (Some("--tstp-out"), "fof(1,lemma,p(a),unknown()).\n"),
            (Some("--lop-out"), "p(a)\n"),
        ] {
            let mut arguments = vec![PROGRAM_NAME, "--max-lemmas=0", "--min-lemma-quality=0"];
            if let Some(format_argument) = format_argument {
                arguments.push(format_argument);
            }
            let (status, output, stderr) = run_with_stdin(&arguments, input);

            assert_eq!(status, 0, "format {format_argument:?}");
            assert!(
                output.ends_with(expected),
                "format {format_argument:?}: {output}"
            );
            assert!(!output.contains("q(a)"), "format {format_argument:?}");
            assert!(stderr.is_empty(), "format {format_argument:?}");
        }
    }

    #[test]
    fn relative_quality_and_tstp_output_use_best_weight_fraction() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--max-lemmas=1",
                "--min-lemma-quality-rel=0.0",
                "--tstp-out",
            ],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(output.contains("% Minimum lemma quality: 0.000000\n"));
        assert!(output.contains("cnf("));
        assert!(output.contains("lemma"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn lop_out_preserves_c_fallthrough_to_iterative_algorithm() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--recursive-lemmas",
                "--lop-out",
                "--max-lemmas=0",
                "--min-lemma-quality=0",
            ],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(output.contains("p(a) <- ."));
        assert!(!output.contains("t(a) <- ."));
        assert!(stderr.is_empty());
    }

    #[test]
    fn verbose_and_weight_options_are_accepted() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[
                PROGRAM_NAME,
                "--verbose=3",
                "--max-lemmas=0",
                "--min-lemma-quality=0",
                "--lemma-tree-base-weight=2",
                "--lemma-size-base-weight=2",
                "--active-pm-weight=1.5",
                "--generating-inference-weight=1.25",
                "--simplifying-weight=0.5",
                "--simplified-weight=0.75",
                "--no-reference-weights",
                "--horn-bonus=1.0",
                "--pcl-initial-weight=2",
                "--pcl-quote-weight=3",
                "--pcl-paramod-weight=4",
                "--pcl-eres-weight=5",
                "--pcl-efact-weight=6",
                "--pcl-sr-weight=7",
                "--pcl-acres-weight=8",
                "--pcl-rw-weight=9",
                "--pcl-urw-weight=10",
                "--pcl-cn-weight=11",
                "--pcl-split-weight=12",
            ],
            SAMPLE_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 3);
        assert!(output.contains("% Selecting at most 0 lemmas\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn trailing_input_reports_syntax_error() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"1 : : [++p] : initial\ntrailing\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("trailing input is rejected");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("No token"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn shell_pcl_stays_disabled_like_c_epcllemma() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"2 : : : 1 : final\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("C epcllemma does not enable SupportShellPCL");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_text_preserves_c_usage_summary() {
        let rendered = print_help();

        assert_eq!(rendered, expected_help());
    }
}
