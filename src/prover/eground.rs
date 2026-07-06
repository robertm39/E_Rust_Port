use crate::basics::defines::{DEFAULT_COMCHAR_RAW, MEGA};
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{
    current_resource_usage, format_resource_usage, get_system_phys_memory, set_memory_limit,
};
use crate::basics::simple_stuff::{problem_type, reset_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause::ClauseParseOptions;
use crate::clauses::clausefunc::clause_set_remove_superfluous_literals;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::formulasets::{FormulaSet, FormulaSetCnfOptions};
use crate::clauses::grounding::{
    clause_cmp_by_len, clause_set_create_constrained_ground_instances_with_output,
    clause_set_create_ground_instances_with_output, clause_set_eqlit_recode,
    print_dimacs_header_string, GroundInstanceOutcome, GroundInstancePrintOptions, GroundSet,
    GroundSetState,
};
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::clauses::splitting::clause_set_split_clauses_general_fresh;
use crate::heuristics::clausesetfeatures::{
    clause_set_is_ground, spec_features_compute, SpecFeatureCell,
};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner};
use crate::inout::signals::{configure_time_limits, RLIM_INFINITY_COMPAT};
use crate::prover::eprover::{
    parse_clause_scanner_into_formula_set_with_options, FoolUnroll, FormulaPreprocessing,
};
use crate::prover::version::{footer, VERSION};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "eground";
const TFORM_RENAME_LIMIT_STR: &str = "24";
const TFORM_MINISCOPE_LIMIT_STR: &str = "1000";
const EGROUND_CNF_MINISCOPE_LIMIT: i64 = 1_048_576;
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    PrintStatistics,
    ResourcesInfo,
    SuppressResult,
    LopParse,
    TptpParse,
    TptpPrint,
    TptpFormat,
    TstpParse,
    TstpPrint,
    TstpFormat,
    DefinitionalCnf,
    MiniscopeLimit,
    DimacsPrint,
    SplitTries,
    DisableUnitSubsumption,
    DisableUnitResolution,
    DisableTautologyDetection,
    MemoryLimit,
    CpuLimit,
    SoftCpuLimit,
    PartComplete,
    GiveUp,
    Constraints,
    LocalConstraints,
    FixMinisat,
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
        "Verbose comments on the progress of the program by printing technical information to stderr.",
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
        "Select an output level, greater values imply more verbose output.",
    ),
    OptCell::new(
        OptionCode::PrintStatistics,
        None,
        Some("print-statistics"),
        OptArgType::NoArg,
        None,
        "Print a short statistical summary of clauses read and generated.",
    ),
    OptCell::new(
        OptionCode::ResourcesInfo,
        Some('R'),
        Some("resources-info"),
        OptArgType::NoArg,
        None,
        "Give some information about the resources used by the system.",
    ),
    OptCell::new(
        OptionCode::SuppressResult,
        None,
        Some("suppress-result"),
        OptArgType::NoArg,
        None,
        "Suppress actual printing of the result, just give a short message about success.",
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
        "Parse TPTP-2 format instead of E-LOP.",
    ),
    OptCell::new(
        OptionCode::TptpPrint,
        None,
        Some("tptp-out"),
        OptArgType::NoArg,
        None,
        "Print TPTP-2 format instead of E-LOP.",
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
        OptionCode::TptpParse,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpPrint,
        None,
        Some("tptp2-out"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-out.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp2-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-format.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-3 format instead of E-LOP.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "Print output clauses in TPTP-3 syntax.",
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
        "Synonymous with --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tptp3-out"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-out.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-format.",
    ),
    OptCell::new(
        OptionCode::DimacsPrint,
        Some('d'),
        Some("dimacs"),
        OptArgType::NoArg,
        None,
        "Print output in the DIMACS format suitable for many propositional provers.",
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
        OptionCode::SplitTries,
        None,
        Some("split-tries"),
        OptArgType::OptArg,
        Some("1"),
        "Determine the number of tries for splitting.",
    ),
    OptCell::new(
        OptionCode::DisableUnitSubsumption,
        Some('U'),
        Some("no-unit-subsumption"),
        OptArgType::NoArg,
        None,
        "Do not check if clauses are subsumed by previously encountered unit clauses.",
    ),
    OptCell::new(
        OptionCode::DisableUnitResolution,
        Some('r'),
        Some("no-unit-resolution"),
        OptArgType::NoArg,
        None,
        "Do not perform forward-unit-resolution on new clauses.",
    ),
    OptCell::new(
        OptionCode::DisableTautologyDetection,
        Some('t'),
        Some("no-tautology-detection"),
        OptArgType::NoArg,
        None,
        "Do not perform tautology deletion on new clauses.",
    ),
    OptCell::new(
        OptionCode::MemoryLimit,
        Some('m'),
        Some("memory-limit"),
        OptArgType::ReqArg,
        None,
        "Limit the memory the system may use. The argument is the allowed amount of memory in MB.",
    ),
    OptCell::new(
        OptionCode::CpuLimit,
        None,
        Some("cpu-limit"),
        OptArgType::OptArg,
        Some("300"),
        "Limit the cpu time the program should run.",
    ),
    OptCell::new(
        OptionCode::SoftCpuLimit,
        None,
        Some("soft-cpu-limit"),
        OptArgType::OptArg,
        Some("310"),
        "Limit the cpu time spent in grounding.",
    ),
    OptCell::new(
        OptionCode::PartComplete,
        Some('i'),
        Some("add-one-instance"),
        OptArgType::NoArg,
        None,
        "If grounding runs out of time or memory, try to add at least one instance of each clause.",
    ),
    OptCell::new(
        OptionCode::GiveUp,
        Some('g'),
        Some("give-up"),
        OptArgType::ReqArg,
        None,
        "Give up early if the problem is unlikely to be reasonably small.",
    ),
    OptCell::new(
        OptionCode::Constraints,
        Some('c'),
        Some("constraints"),
        OptArgType::NoArg,
        None,
        "Use global purity constraints to restrict the number of instantiations done.",
    ),
    OptCell::new(
        OptionCode::LocalConstraints,
        Some('C'),
        Some("local-constraints"),
        OptArgType::NoArg,
        None,
        "Use local purity constraints to further restrict the number of instantiations done.",
    ),
    OptCell::new(
        OptionCode::FixMinisat,
        Some('M'),
        Some("fix-minisat"),
        OptArgType::NoArg,
        None,
        "Fix the preamble to include only the maximum variable index.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "The fields mirror eground.c process-wide option globals."
)]
struct EgroundConfig {
    output_file: Option<PathBuf>,
    parse_format: IoFormat,
    output_format: IoFormat,
    verbose_level: i64,
    output_level: i64,
    print_statistics: bool,
    print_rusage: bool,
    print_result: bool,
    dimacs_format: bool,
    split_tries: i64,
    unit_subsumption: bool,
    unit_resolution: bool,
    tautology_detection: bool,
    add_single_instance: bool,
    constraints: bool,
    local_constraints: bool,
    fix_minisat: bool,
    give_up: i64,
    formula_def_limit: i64,
    memory_limit: u64,
    hard_cpu_limit: Option<i64>,
    soft_cpu_limit: Option<i64>,
    files: Vec<String>,
}

impl Default for EgroundConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            parse_format: IoFormat::Auto,
            output_format: IoFormat::Lop,
            verbose_level: 0,
            output_level: 1,
            print_statistics: false,
            print_rusage: false,
            print_result: true,
            dimacs_format: false,
            split_tries: 0,
            unit_subsumption: true,
            unit_resolution: true,
            tautology_detection: true,
            add_single_instance: false,
            constraints: false,
            local_constraints: false,
            fix_minisat: false,
            give_up: 0,
            formula_def_limit: 24,
            memory_limit: 0,
            hard_cpu_limit: None,
            soft_cpu_limit: None,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(Box<EgroundConfig>),
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
    set_verbose_level(0);
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
        RunCommand::Execute(config) => execute_eground(&config, stdin, stdout, stderr),
    }
}

#[allow(clippy::too_many_lines)]
fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EgroundConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        let arg = parsed.arg().unwrap_or("");
        match parsed.option().option_code {
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION}"))?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Verbose => {
                config.verbose_level = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::Output => config.output_file = Some(PathBuf::from(arg)),
            OptionCode::Silent => config.output_level = 0,
            OptionCode::OutputLevel => {
                config.output_level = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::PrintStatistics => config.print_statistics = true,
            OptionCode::ResourcesInfo => config.print_rusage = true,
            OptionCode::SuppressResult => config.print_result = false,
            OptionCode::LopParse => config.parse_format = IoFormat::Lop,
            OptionCode::TptpParse => config.parse_format = IoFormat::Tptp,
            OptionCode::TptpPrint => config.output_format = IoFormat::Tptp,
            OptionCode::TptpFormat => {
                config.parse_format = IoFormat::Tptp;
                config.output_format = IoFormat::Tptp;
            }
            OptionCode::TstpParse => config.parse_format = IoFormat::Tstp,
            OptionCode::TstpPrint => config.output_format = IoFormat::Tstp,
            OptionCode::TstpFormat => {
                config.parse_format = IoFormat::Tstp;
                config.output_format = IoFormat::Tstp;
            }
            OptionCode::DimacsPrint => config.dimacs_format = true,
            OptionCode::DefinitionalCnf => {
                config.formula_def_limit = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::MiniscopeLimit => {
                let _ = get_int_arg(parsed.option(), arg)?;
            }
            OptionCode::SplitTries => {
                config.split_tries = get_int_arg(parsed.option(), arg)?;
                if config.split_tries < 0 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Argument to option --split-tries has to be value greater than or equal to 0 ",
                    ));
                }
            }
            OptionCode::DisableUnitSubsumption => config.unit_subsumption = false,
            OptionCode::DisableUnitResolution => config.unit_resolution = false,
            OptionCode::DisableTautologyDetection => config.tautology_detection = false,
            OptionCode::MemoryLimit => {
                config.memory_limit = parse_memory_limit(parsed.option(), parsed.arg())?;
            }
            OptionCode::CpuLimit => {
                let limit = get_int_arg(parsed.option(), arg)?;
                if let Some(soft_limit) = config.soft_cpu_limit {
                    check_hard_soft_limits(limit, soft_limit, true)?;
                }
                config.hard_cpu_limit = Some(limit);
            }
            OptionCode::SoftCpuLimit => {
                let limit = get_int_arg(parsed.option(), arg)?;
                if let Some(hard_limit) = config.hard_cpu_limit {
                    check_hard_soft_limits(hard_limit, limit, false)?;
                }
                config.soft_cpu_limit = Some(limit);
            }
            OptionCode::PartComplete => config.add_single_instance = true,
            OptionCode::GiveUp => config.give_up = get_int_arg(parsed.option(), arg)?,
            OptionCode::Constraints => config.constraints = true,
            OptionCode::LocalConstraints => {
                config.constraints = true;
                config.local_constraints = true;
            }
            OptionCode::FixMinisat => config.fix_minisat = true,
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(Box::new(config)))
}

fn execute_eground(
    config: &EgroundConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    apply_resource_config(config);
    set_verbose_level(i64_to_i32_saturating(config.verbose_level));
    let _ = set_output_level(config.output_level);

    let mut output = EgroundOutput::open(config.output_file.as_deref(), stdout)?;
    let mut bank = eground_term_bank()?;
    let mut clauses = ClauseSet::new();

    let output_format = parse_input_files(config, stdin, &mut bank, &mut clauses)?;
    let preparation = prepare_clauses_for_grounding(config, stderr, &mut bank, &mut clauses)?;
    let groundset = match create_groundset(
        config,
        &mut output,
        &mut bank,
        &clauses,
        output_format,
        preparation.selected_symbol,
    )? {
        GroundingRunResult::Grounded(groundset) => groundset,
        GroundingRunResult::EstimateLimitExceeded => {
            write_give_up_failure(&mut output)?;
            output
                .flush()
                .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
            stderr
                .flush()
                .map_err(|error| io_diagnostic(error.to_string()))?;
            return Ok(0);
        }
    };

    write_eground_result(
        config,
        &mut output,
        &mut bank,
        output_format,
        &groundset,
        preparation,
    )?;
    if config.print_rusage {
        write_all(
            &mut output,
            format_resource_usage(current_resource_usage()).as_bytes(),
        )?;
    }
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(0)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GroundingPreparation {
    selected_symbol: FunCode,
    initial_clauses: i64,
    initial_literals: i64,
}

enum GroundingRunResult {
    Grounded(GroundSet),
    EstimateLimitExceeded,
}

fn parse_input_files(
    config: &EgroundConfig,
    stdin: &mut impl Read,
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
) -> Result<IoFormat, Diagnostic> {
    let mut formulas = FormulaSet::new();
    let output_format = parse_input_files_to_formula_set(config, stdin, bank, &mut formulas)?;
    clausify_input_formulas(config, bank, &mut formulas, clauses)?;
    Ok(output_format)
}

fn parse_input_files_to_formula_set(
    config: &EgroundConfig,
    stdin: &mut impl Read,
    bank: &mut TermBank,
    formulas: &mut FormulaSet,
) -> Result<IoFormat, Diagnostic> {
    let mut ignored_watchlist = ClauseSet::new();
    let mut output_format = config.output_format;
    let clause_parse_options = clause_parse_options(config);

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        let parsed = parse_clause_scanner_into_formula_set_with_options(
            &mut scanner,
            config.parse_format,
            FormulaPreprocessing::parse_only(FoolUnroll::Enabled),
            clause_parse_options,
            bank,
            formulas,
            &mut ignored_watchlist,
        )?;
        if config.parse_format == IoFormat::Auto && parsed.detected_format == IoFormat::Tstp {
            output_format = IoFormat::Tstp;
        }
    }
    Ok(output_format)
}

fn clausify_input_formulas(
    config: &EgroundConfig,
    bank: &mut TermBank,
    formulas: &mut FormulaSet,
    clauses: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    let mut archive = FormulaSet::new();
    let _preprocessed = formulas.preproc_conjectures(bank, false, false)?;
    let fresh_vars = VarBank::new(bank.signature().type_bank());
    let options = FormulaSetCnfOptions::new(EGROUND_CNF_MINISCOPE_LIMIT, true, problem_type())
        .with_def_limit(config.formula_def_limit);
    let _cnf = formulas.cnf2_into(&mut archive, clauses, bank, &fresh_vars, options)?;
    Ok(())
}

fn clause_parse_options(config: &EgroundConfig) -> ClauseParseOptions {
    if config.local_constraints {
        ClauseParseOptions {
            clauses_have_local_variables: false,
            clauses_have_disjoint_variables: true,
        }
    } else {
        ClauseParseOptions::default()
    }
}

fn prepare_clauses_for_grounding(
    config: &EgroundConfig,
    stderr: &mut impl Write,
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
) -> Result<GroundingPreparation, Diagnostic> {
    let _removed_literals = clause_set_remove_superfluous_literals(clauses, bank);
    let mut features = SpecFeatureCell::default();
    spec_features_compute(&mut features, clauses, None, None, bank, |_| false);
    if features.eq_clauses != 0 {
        writeln_diag(
            stderr,
            "Warning: Recoding equational literals. Be sure to include equality axioms!",
        )?;
        let _recoded = clause_set_eqlit_recode(clauses, bank)?;
    }
    if bank.signature().find_max_function_arity() != 0 && !clause_set_is_ground(clauses) {
        return Err(Diagnostic::new(
            ErrorCode::INPUT_SEMANTIC_ERROR,
            "Grounding not possible: Specification is not near-propositional. There is an infinite Herbrand universe and there are non-ground clauses in the specification!",
        ));
    }

    let selected_symbol = if config.add_single_instance {
        clauses.find_freq_symbol(bank.signature(), 0, false)
    } else {
        0
    };
    let initial_clauses = clauses.members();
    let initial_literals = clauses.literals();

    if config.split_tries != 0 {
        let mut tmp = ClauseSet::new();
        let tries = config.split_tries.saturating_sub(1);
        let _split = clause_set_split_clauses_general_fresh(bank, clauses, &mut tmp, tries)?;
        *clauses = tmp;
    }
    clauses.sort_by(|left, right| clause_cmp_by_len(left, right).cmp(&0));
    Ok(GroundingPreparation {
        selected_symbol,
        initial_clauses,
        initial_literals,
    })
}

fn create_groundset(
    config: &EgroundConfig,
    output: &mut impl Write,
    bank: &mut TermBank,
    clauses: &ClauseSet,
    output_format: IoFormat,
    selected_symbol: FunCode,
) -> Result<GroundingRunResult, Diagnostic> {
    let mut groundset = GroundSet::new();
    let print_options = GroundInstancePrintOptions::new(
        config.output_level,
        proof_doc_output_format(output_format),
        ProblemType::FirstOrder,
        config.unit_subsumption,
        config.unit_resolution,
        config.tautology_detection,
    );
    let give_up = Some(config.give_up);
    let outcome = if config.constraints {
        clause_set_create_constrained_ground_instances_with_output(
            &mut *output,
            print_options,
            &mut *bank,
            clauses,
            &mut groundset,
            give_up,
            None,
        )?
    } else {
        clause_set_create_ground_instances_with_output(
            &mut *output,
            print_options,
            &mut *bank,
            clauses,
            &mut groundset,
            give_up,
        )?
    };
    if outcome == GroundInstanceOutcome::EstimateLimitExceeded {
        return Ok(GroundingRunResult::EstimateLimitExceeded);
    }

    if groundset.complete() != GroundSetState::Complete && config.add_single_instance {
        let cached_state = groundset.complete();
        let retry_outcome = clause_set_create_constrained_ground_instances_with_output(
            &mut *output,
            print_options,
            &mut *bank,
            clauses,
            &mut groundset,
            give_up,
            Some(selected_symbol),
        )?;
        if retry_outcome == GroundInstanceOutcome::EstimateLimitExceeded {
            return Ok(GroundingRunResult::EstimateLimitExceeded);
        }
        groundset.set_complete(cached_state);
    }
    Ok(GroundingRunResult::Grounded(groundset))
}

fn write_eground_result(
    config: &EgroundConfig,
    output: &mut EgroundOutput<'_, impl Write>,
    bank: &mut TermBank,
    output_format: IoFormat,
    groundset: &GroundSet,
    preparation: GroundingPreparation,
) -> Result<(), Diagnostic> {
    if config.output_level == 1 {
        writeln_diag(output, "")?;
    }
    if config.print_result {
        if config.dimacs_format {
            write_dimacs_result(output, groundset, config.fix_minisat)?;
        } else {
            write_all(
                output,
                groundset
                    .print_format_string(
                        bank,
                        proof_doc_output_format(output_format),
                        ProblemType::FirstOrder,
                    )?
                    .as_bytes(),
            )?;
        }
        write_completion_message(output, groundset.complete())?;
    } else {
        writeln_diag(output, &format!("{DEFAULT_COMCHAR_RAW} Success!"))?;
    }
    if config.print_statistics {
        write_statistics(
            output,
            preparation.initial_clauses,
            preparation.initial_literals,
            groundset.members(),
            groundset.literal_count(),
        )?;
    }
    Ok(())
}

fn eground_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

fn write_dimacs_result(
    output: &mut EgroundOutput<'_, impl Write>,
    groundset: &GroundSet,
    fix_minisat: bool,
) -> Result<(), Diagnostic> {
    let max_lit = if fix_minisat {
        groundset.max_var()
    } else {
        groundset.max_literal()
    };
    let header = print_dimacs_header_string(max_lit, groundset.dimacs_print_members());
    match output {
        EgroundOutput::File { file, stdout } => {
            let mut out_string = header;
            let mut stdout_string = String::new();
            groundset
                .print_dimacs_to_writers(&mut out_string, &mut stdout_string)
                .map_err(fmt_diagnostic)?;
            write_all(file, out_string.as_bytes())?;
            write_all(stdout, stdout_string.as_bytes())
        }
        EgroundOutput::Stdout(stdout) => {
            write_all(*stdout, header.as_bytes())?;
            write_all(*stdout, groundset.dimacs_string().as_bytes())
        }
    }
}

fn write_completion_message(
    output: &mut impl Write,
    state: GroundSetState,
) -> Result<(), Diagnostic> {
    let message = match state {
        GroundSetState::Complete => "Full and complete proof state written!",
        GroundSetState::LowMemory => "Out of memory: Proof state incomplete!",
        GroundSetState::Timeout => "Timeout: Proof state incomplete!",
        GroundSetState::Unknown => "Proof state incomplete!",
    };
    writeln_diag(output, &format!("{DEFAULT_COMCHAR_RAW} {message}"))
}

fn write_give_up_failure(output: &mut impl Write) -> Result<(), Diagnostic> {
    writeln_diag(
        output,
        &format!(
            "\n{DEFAULT_COMCHAR_RAW} Failure: User resource limit exceeded (estimated number of instances)!"
        ),
    )
}

fn write_statistics(
    output: &mut impl Write,
    initial_clauses: i64,
    initial_literals: i64,
    generated_clauses: i64,
    generated_literals: i64,
) -> Result<(), Diagnostic> {
    writeln_diag(output, "")?;
    writeln_diag(
        output,
        &format!("{DEFAULT_COMCHAR_RAW} Initial clauses                      : {initial_clauses}"),
    )?;
    writeln_diag(
        output,
        &format!("{DEFAULT_COMCHAR_RAW} Initial literals                     : {initial_literals}"),
    )?;
    writeln_diag(
        output,
        &format!(
            "{DEFAULT_COMCHAR_RAW} Generated clauses                    : {generated_clauses}"
        ),
    )?;
    writeln_diag(
        output,
        &format!(
            "{DEFAULT_COMCHAR_RAW} Generated literals                   : {generated_literals}"
        ),
    )
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, false)
    } else {
        Scanner::from_file(Path::new(name), false).map_err(eground_scanner_open_diagnostic)
    }
}

fn proof_doc_output_format(format: IoFormat) -> ProofDocOutputFormat {
    match format {
        IoFormat::Tptp => ProofDocOutputFormat::Tptp,
        IoFormat::Tstp => ProofDocOutputFormat::Tstp,
        _ => ProofDocOutputFormat::Lop,
    }
}

fn parse_memory_limit<Code>(option: &OptCell<Code>, arg: Option<&str>) -> Result<u64, Diagnostic> {
    let arg = arg.unwrap_or("");
    if arg == "Auto" {
        let system_memory = get_system_phys_memory();
        if system_memory == -1 {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Cannot find physical memory automatically. Give explicit value to --memory-limit",
            ));
        }
        return Ok(memory_limit_bytes_from_mb(auto_memory_mb(system_memory)));
    }
    get_int_arg(option, arg).map(memory_limit_bytes_from_mb)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn auto_memory_mb(system_memory_mb: i64) -> i64 {
    (system_memory_mb as f64 * 0.8) as i64
}

fn apply_resource_config(config: &EgroundConfig) {
    let hard_limit = config
        .hard_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let soft_limit = config
        .soft_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    configure_time_limits(hard_limit, soft_limit, 0);
    let _ = set_memory_limit(config.memory_limit);
}

#[allow(clippy::cast_sign_loss)]
const fn c_rlimit_from_arg(value: i64) -> u64 {
    value as u64
}

const fn memory_limit_bytes_from_mb(memory_mb: i64) -> u64 {
    c_rlimit_from_arg(memory_mb).wrapping_mul(MEGA)
}

fn check_hard_soft_limits(
    hard: i64,
    soft: i64,
    hard_option_changed: bool,
) -> Result<(), Diagnostic> {
    if c_rlimit_from_arg(hard) > c_rlimit_from_arg(soft) {
        return Ok(());
    }
    let message = if hard_option_changed {
        "Hard time limit has to be larger than softtime limit"
    } else {
        "Soft time limit has to be smaller than hardtime limit"
    };
    Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
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
Read a set of clauses and determine if it can be grounded (i.e. is\n\
either already ground or has no non-constant function symbols). If\n\
this is the case, print sufficiently many ground instances of the\n\
clauses to guarantee that a ground refutation can be found for\n\
unsatisfiable clause sets.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

enum EgroundOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File { file: File, stdout: &'a mut W },
}

impl<'a, W: Write> EgroundOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        let file = File::create(path).map_err(|error| {
            eground_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })?;
        Ok(Self::File { file, stdout })
    }
}

impl<W: Write> Write for EgroundOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(output) => output.write(buffer),
            Self::File { file, .. } => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(output) => output.flush(),
            Self::File { file, .. } => file.flush(),
        }
    }
}

fn write_all(output: &mut (impl Write + ?Sized), bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn writeln_diag(output: &mut (impl Write + ?Sized), line: &str) -> Result<(), Diagnostic> {
    write_all(output, line.as_bytes())?;
    write_all(output, b"\n")
}

fn io_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn eground_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn eground_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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

fn fmt_diagnostic(error: fmt::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, error.to_string())
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{
        clause_parse_options, clausify_input_formulas, eground_term_bank,
        parse_input_files_to_formula_set, process_options, run, EgroundConfig, RunCommand,
        DEFAULT_COMCHAR_RAW, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::clauses::clause::ClauseParseOptions;
    use crate::clauses::clause_props::{CP_INPUT_FORMULA, CP_TYPE_AXIOM};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::formulasets::FormulaSet;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::VERSION;
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
    fn help_and_version_are_c_shaped() {
        let mut stdout = Vec::new();
        let command = process_options([PROGRAM_NAME, "--help"], &mut stdout).unwrap();

        assert!(matches!(command, RunCommand::Exit(0)));
        let help = String::from_utf8(stdout).unwrap();
        assert!(help.contains(&format!("{PROGRAM_NAME} {VERSION}")));
        assert!(help.contains("Usage: eground [options] [files]"));

        let mut version = Vec::new();
        let command = process_options([PROGRAM_NAME, "--version"], &mut version).unwrap();
        assert!(matches!(command, RunCommand::Exit(0)));
        assert_eq!(
            String::from_utf8(version).unwrap(),
            format!("{PROGRAM_NAME} {VERSION}\n")
        );
    }

    #[test]
    fn option_parsing_preserves_defaults_and_aliases() {
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--tptp2-format",
                "--tptp3-out",
                "--dimacs",
                "--split-tries",
                "--no-unit-subsumption",
                "--no-unit-resolution",
                "--no-tautology-detection",
                "--suppress-result",
                "--print-statistics",
                "--constraints",
                "--local-constraints",
                "--fix-minisat",
                "--give-up=12",
                "--definitional-cnf=7",
            ],
            &mut stdout,
        )
        .unwrap();

        let RunCommand::Execute(config) = command else {
            panic!("expected execution command");
        };
        assert_eq!(
            *config,
            EgroundConfig {
                output_file: None,
                parse_format: IoFormat::Tptp,
                output_format: IoFormat::Tstp,
                verbose_level: 0,
                output_level: 1,
                print_statistics: true,
                print_rusage: false,
                print_result: false,
                dimacs_format: true,
                split_tries: 1,
                unit_subsumption: false,
                unit_resolution: false,
                tautology_detection: false,
                add_single_instance: false,
                constraints: true,
                local_constraints: true,
                fix_minisat: true,
                give_up: 12,
                formula_def_limit: 7,
                memory_limit: 0,
                hard_cpu_limit: None,
                soft_cpu_limit: None,
                files: vec!["-".to_owned()],
            }
        );
    }

    #[test]
    fn local_constraints_select_c_disjoint_clause_variable_policy() {
        let config = EgroundConfig {
            local_constraints: true,
            constraints: true,
            ..EgroundConfig::default()
        };
        let options = clause_parse_options(&config);
        assert!(!options.clauses_have_local_variables);
        assert!(options.clauses_have_disjoint_variables);

        assert_eq!(
            clause_parse_options(&EgroundConfig::default()),
            ClauseParseOptions::default()
        );
    }

    #[test]
    fn split_tries_rejects_negative_values() {
        let err = process_options([PROGRAM_NAME, "--split-tries=-1"], &mut Vec::new()).unwrap_err();
        assert_eq!(err.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn grounds_lop_stdin() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            [PROGRAM_NAME, "--lop-in"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("p(a)"));
        assert!(output.contains(&format!(
            "{DEFAULT_COMCHAR_RAW} Full and complete proof state written!"
        )));
    }

    #[test]
    fn tstp_formula_input_is_preserved_as_formula_owner_before_cnf() {
        let _guard = global_state_lock();
        let mut bank = eground_term_bank().expect("test term bank allocation succeeds");
        let mut formulas = FormulaSet::new();
        let config = EgroundConfig {
            parse_format: IoFormat::Tstp,
            files: vec!["-".to_owned()],
            ..EgroundConfig::default()
        };
        let mut stdin: &[u8] = b"fof(ax, axiom, (p(a) | q(a))).\n";

        let output_format =
            parse_input_files_to_formula_set(&config, &mut stdin, &mut bank, &mut formulas)
                .expect("formula-owner parsing succeeds");

        assert_eq!(output_format, IoFormat::Lop);
        assert_eq!(formulas.cardinality(), 1);
        let formula = formulas.iter().next().expect("formula owner exists");
        assert!(!formula.is_clause());
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_AXIOM);

        let mut clauses = ClauseSet::new();
        clausify_input_formulas(&config, &mut bank, &mut formulas, &mut clauses)
            .expect("formula-owner CNF succeeds");
        assert_eq!(formulas.cardinality(), 0);
        assert_eq!(clauses.members(), 1);
    }

    #[test]
    fn run_tstp_thf_input_uses_higher_order_parser_context() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"thf(person_type, type, person: $tType).\n\
            thf(a_type, type, a: person).\n\
            thf(p_type, type, p: person > $o).\n\
            thf(fact, axiom, p @ a).\n";

        let status = run(
            [PROGRAM_NAME, "--tstp-in", "--silent", "--suppress-result"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("THF input parses and grounds");

        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(stdout).unwrap(), "% Success!\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn tstp_include_selector_feeds_formula_owner_cnf_path() {
        let _guard = global_state_lock();
        let include_path = temp_path("eground-include-selected-inc");
        let main_path = temp_path("eground-include-selected-main");
        let include_arg = include_path.to_string_lossy().replace('\\', "/");
        fs::write(
            &include_path,
            "fof(selected, axiom, p(a)).\nfof(skipped, axiom, q(a)).\n",
        )
        .unwrap();
        fs::write(
            &main_path,
            format!("include('{include_arg}',[selected]).\n"),
        )
        .unwrap();

        let mut bank = eground_term_bank().expect("test term bank allocation succeeds");
        let mut formulas = FormulaSet::new();
        let config = EgroundConfig {
            parse_format: IoFormat::Tstp,
            files: vec![main_path.to_string_lossy().into_owned()],
            ..EgroundConfig::default()
        };
        let mut stdin: &[u8] = b"";

        parse_input_files_to_formula_set(&config, &mut stdin, &mut bank, &mut formulas)
            .expect("selected include parsing succeeds");

        assert_eq!(formulas.cardinality(), 1);
        let formula = formulas.iter().next().expect("selected formula is kept");
        assert_eq!(formula.get_id(true), "selected");

        let mut clauses = ClauseSet::new();
        clausify_input_formulas(&config, &mut bank, &mut formulas, &mut clauses)
            .expect("selected included formula CNF succeeds");
        assert_eq!(formulas.cardinality(), 0);
        assert_eq!(clauses.members(), 1);

        let _ = fs::remove_file(include_path);
        let _ = fs::remove_file(main_path);
    }

    #[test]
    fn dimacs_output_prints_header_and_complete_status() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            [PROGRAM_NAME, "--lop-in", "--dimacs"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.starts_with(&format!("{DEFAULT_COMCHAR_RAW}\np cnf ")));
        assert!(output.contains(" 0\n"));
        assert!(output.contains(&format!(
            "{DEFAULT_COMCHAR_RAW} Full and complete proof state written!"
        )));
    }

    #[test]
    fn give_up_estimate_limit_exits_with_c_failure_status() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\np(b).\nq(X).\n";

        let status = run(
            [PROGRAM_NAME, "--lop-in", "--silent", "--give-up=1"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "\n{DEFAULT_COMCHAR_RAW} Failure: User resource limit exceeded (estimated number of instances)!\n"
            )
        );
    }

    #[test]
    fn constrained_give_up_failure_uses_configured_output() {
        let _guard = global_state_lock();
        let output_path = temp_path("eground-give-up-output");
        let _ = fs::remove_file(&output_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\np(b).\n~q(Y).\nq(X).\n";

        let status = run(
            [
                PROGRAM_NAME,
                "--lop-in",
                "--silent",
                "--constraints",
                "--give-up=2",
                "-o",
                output_path.to_str().unwrap(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(
            fs::read_to_string(&output_path).unwrap(),
            format!(
                "\n{DEFAULT_COMCHAR_RAW} Failure: User resource limit exceeded (estimated number of instances)!\n"
            )
        );

        fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn suppress_result_still_prints_success_and_statistics() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            [
                PROGRAM_NAME,
                "--lop-in",
                "--suppress-result",
                "--print-statistics",
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains(&format!("{DEFAULT_COMCHAR_RAW} Success!")));
        assert!(output.contains(&format!("{DEFAULT_COMCHAR_RAW} Initial clauses")));
        assert!(output.contains(&format!("{DEFAULT_COMCHAR_RAW} Generated clauses")));
    }

    #[test]
    fn output_file_redirects_main_stream() {
        let _guard = global_state_lock();
        let output_path = temp_path("eground-output");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            [
                PROGRAM_NAME,
                "--lop-in",
                "-o",
                output_path.to_str().unwrap(),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let output = fs::read_to_string(&output_path).unwrap();
        assert!(output.contains("p(a)"));
        assert!(output.contains(&format!(
            "{DEFAULT_COMCHAR_RAW} Full and complete proof state written!"
        )));
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let status = run(
            [PROGRAM_NAME, "--lop-in", "-o", "-"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("p(a)"));
        assert!(output.contains(&format!(
            "{DEFAULT_COMCHAR_RAW} Full and complete proof state written!"
        )));
    }

    #[test]
    fn input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-input");
        let _ = fs::remove_file(&missing_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"";

        let error = run(
            [PROGRAM_NAME, missing_path.to_str().unwrap()],
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
        let output_path = temp_path("eground-early-output");
        let missing_path = temp_path("eground-missing-after-output");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(&missing_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"";

        let error = run(
            [
                PROGRAM_NAME,
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

        fs::remove_file(output_path).unwrap();
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_dir(&output_path);
        fs::create_dir(&output_path).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(a).\n";

        let error = run(
            [PROGRAM_NAME, "-o", output_path.to_str().unwrap()],
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
            [PROGRAM_NAME, "--lop-in", "--silent"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn rejects_non_ground_infinite_herbrand_universe() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut stdin: &[u8] = b"p(f(X)).\n";

        let error = run(
            [PROGRAM_NAME, "--lop-in"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(String::from_utf8(stdout).unwrap().is_empty());
    }

    fn temp_path(label: &str) -> PathBuf {
        let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{label}-{}-{serial}.tmp", std::process::id()))
    }
}
