use crate::basics::defines::MEGA;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{
    current_resource_usage, format_resource_usage, get_system_phys_memory, set_memory_limit,
};
use crate::basics::partial_orderings::HoOrderKind;
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::sysdate::{SysDate, SysDateIncrement};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause::{
    clause_parse, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_tstp_string, Clause, ClauseParseOptions,
};
use crate::clauses::clause_props::{
    clause_type_from_identifier, FormulaProperties, CP_INITIAL, CP_INPUT_FORMULA, CP_TYPE_AXIOM,
};
use crate::clauses::clausefunc::{
    parse_tstp_top_level_distinct_formula, tcf_tstp_parse, tformula_has_free_vars,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::EqnPrintOptions;
use crate::clauses::eqn_props::EP_IS_ORIENTED;
use crate::clauses::formulasets::{
    FormulaPrintFormat, FormulaSet, FormulaSetCnfOptions, WrappedFormula,
};
use crate::clauses::rewrite::{clause_compute_li_normalform_plain, term_li_normalform_plain};
use crate::heuristics::to_params::TermOrdering;
use crate::inout::basicparser::parse_skip_parenthesized_expr;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{token_pos_rep, IoFormat, Scanner, TokenType};
use crate::inout::signals::{configure_time_limits, RLIM_INFINITY_COMPAT};
use crate::orderings::ocb::OrderControlBlock;
use crate::prover::eprover::{
    parse_clause_scanner_into_formula_set_with_options, FoolUnroll, FormulaPreprocessing,
};
use crate::prover::version::{footer, VERSION};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{DerefType, RewriteLevel, Term};
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "enormalizer";
const ENORMALIZER_CNF_MINISCOPE_LIMIT: i64 = 1000;
const ENORMALIZER_CNF_DEF_LIMIT: i64 = 24;
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";
const TSTP_FORMULA_FREE_VARIABLES_MESSAGE: &str =
    "Formula has free variables (check parentheses and quantifier precedence)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Terms,
    Clauses,
    Formulas,
    Output,
    Silent,
    OutputLevel,
    PrintStatistics,
    ResourcesInfo,
    LopIn,
    TptpIn,
    TptpOut,
    TptpFormat,
    TstpIn,
    TstpOut,
    TstpFormat,
    MemoryLimit,
    CpuLimit,
    SoftCpuLimit,
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
        OptionCode::Terms,
        Some('t'),
        Some("terms"),
        OptArgType::ReqArg,
        None,
        "Name of the files containing terms to be normalized. If '-' is used as the argument, terms are read from standard input.",
    ),
    OptCell::new(
        OptionCode::Clauses,
        Some('c'),
        Some("clauses"),
        OptArgType::ReqArg,
        None,
        "Name of the files containing clauses to be normalized. If '-' is used as the argument, clauses are read from standard input.",
    ),
    OptCell::new(
        OptionCode::Formulas,
        Some('f'),
        Some("formulas"),
        OptArgType::ReqArg,
        None,
        "Name of the files containing fomulas to be normalized. If '-' is used as the argument, formulas are read from standard input. Note that formula-syntax is not supported in LOP syntax, but requires --tptp2-format or --tptp3-format",
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
        "Select an output level, greater values imply more verbose output. Level 0 produces nearly no output except for the final clauses, level 1 produces minimal additional output. Higher levels are without meaning in enormalizer (I think).",
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
        "Give some information about the resources used by the system. You will usually get CPU time information. On systems returning more information with the rusage() system call, you will also get information about memory consumption.",
    ),
    OptCell::new(
        OptionCode::LopIn,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Set E-LOP as the input format. If no input format is selected by this or one of the following options, E will guess the input format based on the first token. It will almost always correctly recognize TPTP-3, but it may misidentify E-LOP files that use TPTP meta-identifiers as logical symbols.",
    ),
    OptCell::new(
        OptionCode::TptpIn,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-2 format instead of E-LOP (except includes, which are handles as in TPTP-3, as TPTP-2 include syntax is considered harmful).",
    ),
    OptCell::new(
        OptionCode::TptpOut,
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
        OptionCode::TptpIn,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpOut,
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
        OptionCode::TstpIn,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still under development, and the version implemented may not be fully conformant at all times. It works on all TPTP 3.0.1 input files (including includes).",
    ),
    OptCell::new(
        OptionCode::TstpOut,
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
        OptionCode::TstpIn,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpOut,
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
        OptionCode::MemoryLimit,
        Some('m'),
        Some("memory-limit"),
        OptArgType::ReqArg,
        None,
        "Limit the memory the system may use. The argument is the allowed amount of memory in MB. This option may not work everywhere, due to broken and/or strange behaviour of setrlimit() in some UNIX implementations. It does work under all tested versions of Solaris and GNU/Linux.",
    ),
    OptCell::new(
        OptionCode::CpuLimit,
        None,
        Some("cpu-limit"),
        OptArgType::OptArg,
        Some("300"),
        "Limit the cpu time the program should run. The optional argument is the CPU time in seconds. The program will terminate immediately after reaching the time limit, regardless of internal state. This option may not work everywhere, due to broken and/or strange behaviour of setrlimit() in some UNIX implementations. It does work under all tested versions of Solaris, HP-UX and GNU/Linux. As a side effect, this option will inhibit core file writing.",
    ),
    OptCell::new(
        OptionCode::SoftCpuLimit,
        None,
        Some("soft-cpu-limit"),
        OptArgType::OptArg,
        Some("310"),
        "Limit the cpu time spend in grounding. After the time expires, the prover will print an partial system.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnormalizerConfig {
    term_file: Option<String>,
    clause_file: Option<String>,
    formula_file: Option<String>,
    output_file: Option<PathBuf>,
    parse_format: IoFormat,
    output_format: IoFormat,
    eqn_options: EqnPrintOptions,
    verbose_level: i64,
    output_level: i64,
    print_statistics: bool,
    print_rusage: bool,
    memory_limit: u64,
    hard_cpu_limit: Option<i64>,
    soft_cpu_limit: Option<i64>,
    rule_files: Vec<String>,
}

impl Default for EnormalizerConfig {
    fn default() -> Self {
        Self {
            term_file: None,
            clause_file: None,
            formula_file: None,
            output_file: None,
            parse_format: IoFormat::Auto,
            output_format: IoFormat::Lop,
            eqn_options: EqnPrintOptions::lop(),
            verbose_level: 0,
            output_level: 1,
            print_statistics: false,
            print_rusage: false,
            memory_limit: 0,
            hard_cpu_limit: None,
            soft_cpu_limit: None,
            rule_files: Vec::new(),
        }
    }
}

enum RunCommand {
    Execute(Box<EnormalizerConfig>),
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
        RunCommand::Execute(config) => execute_config(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EnormalizerConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
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
                config.verbose_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::Terms => {
                config.term_file = parsed.arg().map(ToOwned::to_owned);
            }
            OptionCode::Clauses => {
                config.clause_file = parsed.arg().map(ToOwned::to_owned);
            }
            OptionCode::Formulas => {
                config.formula_file = parsed.arg().map(ToOwned::to_owned);
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Silent => {
                config.output_level = 0;
            }
            OptionCode::OutputLevel => {
                config.output_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::PrintStatistics => {
                config.print_statistics = true;
            }
            OptionCode::ResourcesInfo => {
                config.print_rusage = true;
            }
            OptionCode::LopIn => {
                config.parse_format = IoFormat::Lop;
            }
            OptionCode::TptpIn => {
                config.parse_format = IoFormat::Tptp;
            }
            OptionCode::TptpOut => {
                config.output_format = IoFormat::Tptp;
                config.eqn_options = EqnPrintOptions::tptp();
            }
            OptionCode::TptpFormat => {
                config.parse_format = IoFormat::Tptp;
                config.output_format = IoFormat::Tptp;
                config.eqn_options = EqnPrintOptions::tptp();
            }
            OptionCode::TstpIn => {
                config.parse_format = IoFormat::Tstp;
            }
            OptionCode::TstpOut => {
                config.output_format = IoFormat::Tstp;
                config.eqn_options = tstp_eqn_options();
            }
            OptionCode::TstpFormat => {
                config.parse_format = IoFormat::Tstp;
                config.output_format = IoFormat::Tstp;
                config.eqn_options = tstp_eqn_options();
            }
            OptionCode::MemoryLimit => {
                config.memory_limit = parse_memory_limit(parsed.option(), parsed.arg())?;
            }
            OptionCode::CpuLimit => {
                let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if let Some(soft_limit) = config.soft_cpu_limit {
                    check_hard_soft_limits(limit, soft_limit, true)?;
                }
                config.hard_cpu_limit = Some(limit);
            }
            OptionCode::SoftCpuLimit => {
                let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if let Some(hard_limit) = config.hard_cpu_limit {
                    check_hard_soft_limits(hard_limit, limit, false)?;
                }
                config.soft_cpu_limit = Some(limit);
            }
        }
    }

    config.rule_files = state.remaining_args().to_vec();
    if config.rule_files.is_empty() {
        config.rule_files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(Box::new(config)))
}

fn execute_config(
    config: &EnormalizerConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    apply_resource_config(config);
    set_verbose_level(i64_to_i32_saturating(config.verbose_level));
    let _ = set_output_level(config.output_level);

    let mut output = EnormalizerOutput::open(config.output_file.as_deref(), stdout)?;
    let mut bank = new_term_bank()?;
    let mut clauses = ClauseSet::new();
    let mut formulas = FormulaSet::new();
    let mut ignored_watchlist = ClauseSet::new();
    let mut parsed_rule_problem_type = ProblemType::FirstOrder;

    for file in &config.rule_files {
        parsed_rule_problem_type = combine_rule_problem_types(
            parsed_rule_problem_type,
            parse_rule_file(
                config,
                file,
                stdin,
                &mut bank,
                &mut formulas,
                &mut ignored_watchlist,
            )?,
        );
    }
    clausify_rule_formulas(
        &mut bank,
        &mut formulas,
        &mut clauses,
        parsed_rule_problem_type,
    )?;

    let demodulators = build_rw_system(
        &mut clauses,
        &bank,
        config,
        stderr,
        parsed_rule_problem_type,
    )?;
    let mut ocb = OrderControlBlock::alloc(
        TermOrdering::Empty,
        false,
        bank.signature(),
        HoOrderKind::LambdaOrder,
    );
    let mut runtime = RewriteRuntime {
        bank: &mut bank,
        ocb: &mut ocb,
        demodulators: &demodulators,
        problem_type: parsed_rule_problem_type,
    };

    if let Some(name) = config.term_file.as_deref() {
        process_terms(name, config, stdin, &mut output, &mut runtime)?;
    }
    if let Some(name) = config.clause_file.as_deref() {
        process_clauses(name, config, stdin, &mut output, &mut runtime)?;
    }
    if let Some(name) = config.formula_file.as_deref() {
        process_formulas(name, config, stdin, &mut output, &mut runtime)?;
    }
    if config.print_rusage {
        write_all(
            &mut output,
            format_resource_usage(current_resource_usage()).as_bytes(),
        )?;
    }
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn new_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

fn parse_rule_file(
    config: &EnormalizerConfig,
    file: &str,
    stdin: &mut impl Read,
    bank: &mut TermBank,
    formulas: &mut FormulaSet,
    ignored_watchlist: &mut ClauseSet,
) -> Result<ProblemType, Diagnostic> {
    let mut scanner = scanner_for_input(file, stdin)?;
    let parsed_file = parse_clause_scanner_into_formula_set_with_options(
        &mut scanner,
        config.parse_format,
        FormulaPreprocessing::parse_only(FoolUnroll::Enabled),
        ClauseParseOptions::default(),
        bank,
        formulas,
        ignored_watchlist,
    )?;
    Ok(parsed_file.problem_type)
}

const fn combine_rule_problem_types(left: ProblemType, right: ProblemType) -> ProblemType {
    if matches!(left, ProblemType::HigherOrder) || matches!(right, ProblemType::HigherOrder) {
        ProblemType::HigherOrder
    } else {
        ProblemType::FirstOrder
    }
}

fn clausify_rule_formulas(
    bank: &mut TermBank,
    formulas: &mut FormulaSet,
    clauses: &mut ClauseSet,
    problem_type: ProblemType,
) -> Result<(), Diagnostic> {
    let mut archive = FormulaSet::new();
    let _preprocessed = formulas.preproc_conjectures(bank, false, false)?;
    let fresh_vars = VarBank::new(bank.signature().type_bank());
    let options = FormulaSetCnfOptions::new(ENORMALIZER_CNF_MINISCOPE_LIMIT, true, problem_type)
        .with_def_limit(ENORMALIZER_CNF_DEF_LIMIT);
    let _cnf = formulas.cnf2_into(&mut archive, clauses, bank, &fresh_vars, options)?;
    Ok(())
}

fn build_rw_system(
    clauses: &mut ClauseSet,
    bank: &TermBank,
    config: &EnormalizerConfig,
    stderr: &mut impl Write,
    problem_type: ProblemType,
) -> Result<ClauseSet, Diagnostic> {
    let mut demodulators = ClauseSet::new();
    while let Some(mut clause) = clauses.extract_first() {
        if clause.is_demodulator() {
            let next_date = increment_demodulator_date(demodulators.date())?;
            demodulators.set_date(next_date);
            clause.set_date(next_date);
            for literal in clause.literals_mut().as_mut_slice() {
                literal.set_prop(EP_IS_ORIENTED);
            }
            demodulators.insert(clause);
        } else {
            let rendered = render_clause(bank, &clause, problem_type, config)?;
            writeln_diag(
                stderr,
                &format!("{PROGRAM_NAME}: Clause is not a rewrite rule: {rendered} -- ignoring"),
            )?;
        }
    }
    Ok(demodulators)
}

struct RewriteRuntime<'a> {
    bank: &'a mut TermBank,
    ocb: &'a mut OrderControlBlock,
    demodulators: &'a ClauseSet,
    problem_type: ProblemType,
}

fn increment_demodulator_date(mut date: SysDate) -> Result<SysDate, Diagnostic> {
    match date.increment() {
        SysDateIncrement::Advanced => Ok(date),
        SysDateIncrement::CAssertionWouldFail => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "rewrite date increment would violate C SysDate assertion",
        )),
        SysDateIncrement::Overflow => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "rewrite date increment overflowed",
        )),
    }
}

fn process_terms(
    name: &str,
    config: &EnormalizerConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
    runtime: &mut RewriteRuntime<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let original = parse_target_term(
            runtime.bank,
            &mut scanner,
            config.parse_format,
            runtime.problem_type,
        )?;
        let normalized =
            normalize_term(runtime.bank, runtime.ocb, &original, runtime.demodulators)?;
        writeln_diag(
            output,
            &format!(
                "{} ==> {}",
                render_term(runtime.bank, &original, runtime.problem_type),
                render_term(runtime.bank, &normalized, runtime.problem_type)
            ),
        )?;
    }
    Ok(())
}

fn process_clauses(
    name: &str,
    config: &EnormalizerConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
    runtime: &mut RewriteRuntime<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let mut clause = clause_parse(&mut scanner, runtime.bank, runtime.problem_type)?;
        let original = render_clause(runtime.bank, &clause, runtime.problem_type, config)?;
        clause_compute_li_normalform_plain(
            runtime.bank,
            runtime.ocb,
            &mut clause,
            &[runtime.demodulators],
            RewriteLevel::RuleRewrite,
            false,
            false,
        )?;
        let normalized = render_clause(runtime.bank, &clause, runtime.problem_type, config)?;
        writeln_diag(output, &format!("{original} ==> {normalized}"))?;
    }
    Ok(())
}

fn parse_target_term(
    bank: &mut TermBank,
    scanner: &mut Scanner,
    format: IoFormat,
    problem_type: ProblemType,
) -> Result<Term, Diagnostic> {
    if format == IoFormat::Tstp && problem_type == ProblemType::HigherOrder {
        bank.parse_tstp_application_term(scanner)
    } else {
        bank.parse_term_simple(scanner)
    }
}

fn process_formulas(
    name: &str,
    config: &EnormalizerConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
    runtime: &mut RewriteRuntime<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let mut target = parse_wrapped_formula(&mut scanner, runtime.bank)?;
        let original = render_formula(runtime.bank, &target.formula, target.problem_type, config)?;
        let normalized = normalize_term(
            runtime.bank,
            runtime.ocb,
            target.formula.formula(),
            runtime.demodulators,
        )?;
        target.formula.set_formula(normalized);
        let normalized =
            render_formula(runtime.bank, &target.formula, target.problem_type, config)?;
        writeln_diag(output, &format!("{original} ==> {normalized}"))?;
    }
    Ok(())
}

fn normalize_term(
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    term: &Term,
    demodulators: &ClauseSet,
) -> Result<Term, Diagnostic> {
    term_li_normalform_plain(
        bank,
        ocb,
        term,
        &[demodulators],
        RewriteLevel::RuleRewrite,
        false,
        false,
        false,
    )
}

fn parse_wrapped_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<FormulaTarget, Diagnostic> {
    match scanner.format() {
        IoFormat::Tptp => parse_old_tptp_wrapped_formula(scanner, bank),
        IoFormat::Tstp => parse_tstp_wrapped_formula(scanner, bank),
        IoFormat::Lop | IoFormat::Auto => Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Formula parsing is only supported for TPTP/TSTP input",
        )),
    }
}

struct FormulaTarget {
    formula: WrappedFormula,
    problem_type: ProblemType,
}

fn parse_old_tptp_wrapped_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<FormulaTarget, Diagnostic> {
    set_problem_type(ProblemType::FirstOrder)?;
    let start_source = token_source_string(scanner.current_token().source_bytes());
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());
    scanner.accept_id("input_formula")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.check_id("axiom|hypothesis|negated_conjecture|conjecture|question|lemma|unknown")?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let formula = bank.parse_tformula_tptp(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(FormulaTarget {
        formula: wrapped_formula(
            formula,
            old_tptp_input_formula_type(&role),
            &name,
            &start_source,
            start_line,
            start_column,
        ),
        problem_type: ProblemType::FirstOrder,
    })
}

fn parse_tstp_wrapped_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<FormulaTarget, Diagnostic> {
    let start_source = token_source_string(scanner.current_token().source_bytes());
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());
    let formula_kind = scanner.current_token().literal();
    let formula_problem_type = tstp_formula_kind_problem_type(&formula_kind);
    set_problem_type(formula_problem_type)?;
    mark_typed_symbols_for_tstp_formula_kind(bank, &formula_kind);
    scanner.accept_id("fof|tff|tcf|thf")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
    scanner.accept_tok(TokenType::COMMA)?;
    if scanner.test_id("type") {
        scanner.accept_id("type")?;
        scanner.accept_tok(TokenType::COMMA)?;
        bank.signature_mut()
            .parse_tff_type_declaration(scanner, formula_problem_type)?;
        skip_tstp_optional_source(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        scanner.accept_tok(TokenType::FULLSTOP)?;
        return Ok(FormulaTarget {
            formula: wrapped_formula(
                bank.true_term().clone(),
                CP_TYPE_AXIOM,
                &name,
                &start_source,
                start_line,
                start_column,
            ),
            problem_type: formula_problem_type,
        });
    }
    scanner.check_id(tstp_formula_roles(&formula_kind))?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let formula_position = token_pos_rep(scanner.current_token());
    let formula = if let Some(distinct) = parse_tstp_top_level_distinct_formula(scanner, bank)? {
        distinct
    } else if formula_kind == "tcf" {
        tcf_tstp_parse(scanner, bank, formula_problem_type)?
    } else {
        bank.parse_tformula_tstp(scanner)?
    };
    if tformula_has_free_vars(bank, &formula).is_some() {
        return Err(tstp_formula_free_variables_error(&formula_position));
    }
    skip_tstp_optional_source(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(FormulaTarget {
        formula: wrapped_formula(
            formula,
            clause_type_from_identifier(&role, formula_problem_type),
            &name,
            &start_source,
            start_line,
            start_column,
        ),
        problem_type: formula_problem_type,
    })
}

fn wrapped_formula(
    formula: Term,
    type_: FormulaProperties,
    name: &str,
    source: &str,
    line: i64,
    column: i64,
) -> WrappedFormula {
    let mut wrapper = WrappedFormula::wt_formula_alloc(formula);
    wrapper.set_tptp_type(type_);
    wrapper.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
    wrapper.set_info(Some(ClauseInfo::new(
        Some(name),
        Some(source),
        line,
        column,
    )));
    wrapper
}

fn render_formula(
    bank: &mut TermBank,
    formula: &WrappedFormula,
    problem_type: ProblemType,
    config: &EnormalizerConfig,
) -> Result<String, Diagnostic> {
    formula.print_string(
        bank,
        true,
        problem_type,
        formula_print_format(config.output_format),
        true,
    )
}

fn render_term(bank: &TermBank, term: &Term, problem_type: ProblemType) -> String {
    let mut output = String::new();
    let _ = bank.write_term_deref_for_problem(&mut output, term, problem_type, DerefType::Never);
    output
}

fn render_clause(
    bank: &TermBank,
    clause: &Clause,
    problem_type: ProblemType,
    config: &EnormalizerConfig,
) -> Result<String, Diagnostic> {
    match config.output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank,
            clause,
            config.eqn_options,
        )),
        IoFormat::Tstp => clause_tstp_string(bank, clause, true, true, problem_type),
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string_with_options(
            bank,
            clause,
            true,
            config.eqn_options,
        )),
    }
}

fn formatted_scanner_for_input(
    name: &str,
    stdin: &mut impl Read,
    format: IoFormat,
) -> Result<Scanner, Diagnostic> {
    let mut scanner = scanner_for_input(name, stdin)?;
    scanner.set_format(format);
    Ok(scanner)
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        return Scanner::from_file_content("-", data, true);
    }
    Scanner::from_file(Path::new(name), true).map_err(enormalizer_scanner_open_diagnostic)
}

fn apply_resource_config(config: &EnormalizerConfig) {
    let hard_limit = config
        .hard_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let soft_limit = config
        .soft_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    configure_time_limits(hard_limit, soft_limit, 0);
    let _ = set_memory_limit(config.memory_limit);
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
Usage: {PROGRAM_NAME} [options] [rulefiles]\n\
\n\
Read a set of rewrite rules (in the form of unit clauses and/or\n\
formulas) with a single positive literal) and sets of terms, clauses,\n\
and/or formulas (the \"normalization targets\", from files specified\n\
with the proper options - see below) to rewrite. Rewrite rules are read\n\
from the left to right as specified in the input, without regard to any\n\
term ordering.\n\
\n\
The normalization targets are rewritten using these rewrite rules until\n\
a normal form is reached. If the rule system is not confluent, the\n\
results are deterministic but unspecified. If the rule system is not\n\
terminating, rewriting might get stuck into an infinite loop. \n\
\n\
The rewrite strategy is leftmost-innermost. The order of rewrite rules\n\
tried at each subterm is deterministic, but unspecified and\n\
independent of input order (it depends on the order in which rules are\n\
returned from the perfect discrimination tree index).\n\
\n\
The normalized terms/clauses/formulas are printed.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

fn tstp_eqn_options() -> EqnPrintOptions {
    EqnPrintOptions {
        output_format: IoFormat::Tstp,
        use_infix: true,
        ..EqnPrintOptions::lop()
    }
}

const fn formula_print_format(format: IoFormat) -> FormulaPrintFormat {
    match format {
        IoFormat::Tptp => FormulaPrintFormat::Tptp,
        IoFormat::Tstp => FormulaPrintFormat::Tstp,
        IoFormat::Lop | IoFormat::Auto => FormulaPrintFormat::Lop,
    }
}

fn old_tptp_input_formula_type(role: &str) -> FormulaProperties {
    match role {
        "hypothesis" | "conjecture" | "negated_conjecture" | "question" => {
            clause_type_from_identifier(role, ProblemType::FirstOrder)
        }
        _ => CP_TYPE_AXIOM,
    }
}

fn tstp_formula_roles(formula_kind: &str) -> &'static str {
    if formula_kind == "tcf" {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question|watchlist"
    } else {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question"
    }
}

fn tstp_formula_kind_problem_type(kind: &str) -> ProblemType {
    if kind == "thf" {
        ProblemType::HigherOrder
    } else {
        ProblemType::FirstOrder
    }
}

fn mark_typed_symbols_for_tstp_formula_kind(bank: &mut TermBank, kind: &str) {
    if matches!(kind, "tff" | "tcf" | "thf") {
        bank.signature_mut().set_typed_symbols(true);
    }
}

fn skip_tstp_optional_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if !scanner.test_tok(TokenType::COMMA) {
        return Ok(());
    }
    scanner.accept_tok(TokenType::COMMA)?;
    skip_tstp_source(scanner)?;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        scanner.check_tok(TokenType::OPEN_SQUARE)?;
        parse_skip_parenthesized_expr(scanner)?;
    }
    Ok(())
}

fn skip_tstp_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::OPEN_SQUARE) {
        return parse_skip_parenthesized_expr(scanner);
    }
    scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;
    if scanner.test_tok(TokenType::OPEN_BRACKET) {
        parse_skip_parenthesized_expr(scanner)?;
    }
    Ok(())
}

fn tstp_formula_free_variables_error(position: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{position} {TSTP_FORMULA_FREE_VARIABLES_MESSAGE}"),
    )
}

fn token_source_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
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

fn enormalizer_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn enormalizer_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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

enum EnormalizerOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> EnormalizerOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            enormalizer_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for EnormalizerOutput<'_, W> {
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

#[cfg(test)]
mod tests {
    use super::{
        clausify_rule_formulas, memory_limit_bytes_from_mb, new_term_bank, parse_rule_file,
        parse_wrapped_formula, print_help, process_options, run, EnormalizerConfig, RunCommand,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME, TSTP_FORMULA_FREE_VARIABLES_MESSAGE, VERSION,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::clauses::clause_props::{CP_INITIAL, CP_INPUT_FORMULA, CP_TYPE_AXIOM};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::formulasets::FormulaSet;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::prover::version::footer;
    use crate::test_support::global_state_lock;
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

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
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("e_rust_port_enormalizer_{name}_{id}.p"))
    }

    fn empty_stdin() -> Vec<u8> {
        Vec::new()
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        let mut expected = format!(
            concat!(
                "\n",
                "\n",
                "enormalizer {version}\n",
                "\n",
                "Usage: enormalizer [options] [rulefiles]\n",
                "\n",
                "Read a set of rewrite rules (in the form of unit clauses and/or\n",
                "formulas) with a single positive literal) and sets of terms, clauses,\n",
                "and/or formulas (the \"normalization targets\", from files specified\n",
                "with the proper options - see below) to rewrite. Rewrite rules are read\n",
                "from the left to right as specified in the input, without regard to any\n",
                "term ordering.\n",
                "\n",
                "The normalization targets are rewritten using these rewrite rules until\n",
                "a normal form is reached. If the rule system is not confluent, the\n",
                "results are deterministic but unspecified. If the rule system is not\n",
                "terminating, rewriting might get stuck into an infinite loop. \n",
                "\n",
                "The rewrite strategy is leftmost-innermost. The order of rewrite rules\n",
                "tried at each subterm is deterministic, but unspecified and\n",
                "independent of input order (it depends on the order in which rules are\n",
                "returned from the perfect discrimination tree index).\n",
                "\n",
                "The normalized terms/clauses/formulas are printed.\n",
                "\n",
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
                "    Verbose comments on the progress of the program by printing technical\n",
                "    information to stderr. The short form or the long form without the\n",
                "    optional argument is equivalent to --verbose=1.\n",
                "\n",
                "   -t <arg>\n",
                "  --terms=<arg>\n",
                "    Name of the files containing terms to be normalized. If '-' is used as\n",
                "    the argument, terms are read from standard input.\n",
                "\n",
                "   -c <arg>\n",
                "  --clauses=<arg>\n",
                "    Name of the files containing clauses to be normalized. If '-' is used as\n",
                "    the argument, clauses are read from standard input.\n",
                "\n",
                "   -f <arg>\n",
                "  --formulas=<arg>\n",
                "    Name of the files containing fomulas to be normalized. If '-' is used as\n",
                "    the argument, formulas are read from standard input. Note that\n",
                "    formula-syntax is not supported in LOP syntax, but requires\n",
                "    --tptp2-format or --tptp3-format\n",
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
                "    produces nearly no output except for the final clauses, level 1 produces\n",
                "    minimal additional output. Higher levels are without meaning in\n",
                "    enormalizer (I think).\n",
                "\n",
                "  --print-statistics\n",
                "    Print a short statistical summary of clauses read and generated.\n",
                "\n",
                "   -R\n",
                "  --resources-info\n",
                "    Give some information about the resources used by the system. You will\n",
                "    usually get CPU time information. On systems returning more information\n",
                "    with the rusage() system call, you will also get information about memory\n",
                "    consumption.\n",
                "\n",
                "  --lop-in\n",
                "    Set E-LOP as the input format. If no input format is selected by this or\n",
                "    one of the following options, E will guess the input format based on the\n",
                "    first token. It will almost always correctly recognize TPTP-3, but it may\n",
                "    misidentify E-LOP files that use TPTP meta-identifiers as logical\n",
                "    symbols.\n",
                "\n",
                "  --tptp-in\n",
                "    Parse TPTP-2 format instead of E-LOP (except includes, which are handles\n",
                "    as in TPTP-3, as TPTP-2 include syntax is considered harmful).\n",
                "\n",
                "  --tptp-out\n",
                "    Print TPTP-2 format instead of E-LOP.\n",
                "\n",
                "  --tptp-format\n",
                "    Equivalent to --tptp-in and --tptp-out.\n",
                "\n",
                "  --tptp2-in\n",
                "    Synonymous with --tptp-in.\n",
                "\n",
                "  --tptp2-out\n",
                "    Synonymous with --tptp-out.\n",
                "\n",
                "  --tptp2-format\n",
                "    Synonymous with --tptp-format.\n",
                "\n",
                "  --tstp-in\n",
                "    Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still\n",
                "    under development, and the version implemented may not be fully\n",
                "    conformant at all times. It works on all TPTP 3.0.1 input files\n",
                "    (including includes).\n",
                "\n",
                "  --tstp-out\n",
                "    Print output clauses in TPTP-3 syntax.\n",
                "\n",
                "  --tstp-format\n",
                "    Equivalent to --tstp-in and --tstp-out.\n",
                "\n",
                "  --tptp3-in\n",
                "    Synonymous with --tstp-in.\n",
                "\n",
                "  --tptp3-out\n",
                "    Synonymous with --tstp-out.\n",
                "\n",
                "  --tptp3-format\n",
                "    Synonymous with --tstp-format.\n",
                "\n",
                "   -m <arg>\n",
                "  --memory-limit=<arg>\n",
                "    Limit the memory the system may use. The argument is the allowed amount\n",
                "    of memory in MB. This option may not work everywhere, due to broken\n",
                "    and/or strange behaviour of setrlimit() in some UNIX implementations. It\n",
                "    does work under all tested versions of Solaris and GNU/Linux.\n",
                "\n",
                "  --cpu-limit[=<arg>]\n",
                "    Limit the cpu time the program should run. The optional argument is the\n",
                "    CPU time in seconds. The program will terminate immediately after\n",
                "    reaching the time limit, regardless of internal state. This option may\n",
                "    not work everywhere, due to broken and/or strange behaviour of\n",
                "    setrlimit() in some UNIX implementations. It does work under all tested\n",
                "    versions of Solaris, HP-UX and GNU/Linux. As a side effect, this option\n",
                "    will inhibit core file writing. The option without the optional argument\n",
                "    is equivalent to --cpu-limit=300.\n",
                "\n",
                "  --soft-cpu-limit[=<arg>]\n",
                "    Limit the cpu time spend in grounding. After the time expires, the prover\n",
                "    will print an partial system. The option without the optional argument is\n",
                "    equivalent to --soft-cpu-limit=310.\n",
                "\n",
                "\n",
                "\n",
            ),
            version = VERSION,
        );
        expected.push_str(&footer());
        expected
    }

    #[test]
    fn process_options_records_targets_formats_and_limits() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--terms=terms.p",
                "--clauses=clauses.p",
                "--formulas=forms.p",
                "--tstp-format",
                "--tptp-out",
                "--memory-limit=12",
                "--cpu-limit=20",
                "--soft-cpu-limit=10",
                "rules.p",
            ],
            &mut stdout,
        )
        .expect("options parse");
        let RunCommand::Execute(config) = command else {
            panic!("expected execute");
        };
        assert_eq!(config.term_file.as_deref(), Some("terms.p"));
        assert_eq!(config.clause_file.as_deref(), Some("clauses.p"));
        assert_eq!(config.formula_file.as_deref(), Some("forms.p"));
        assert_eq!(config.parse_format, IoFormat::Tstp);
        assert_eq!(config.output_format, IoFormat::Tptp);
        assert_eq!(config.memory_limit, memory_limit_bytes_from_mb(12));
        assert_eq!(config.hard_cpu_limit, Some(20));
        assert_eq!(config.soft_cpu_limit, Some(10));
        assert_eq!(config.rule_files, vec!["rules.p"]);
    }

    #[test]
    fn process_options_defaults_rules_to_stdin() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let RunCommand::Execute(config) =
            process_options([PROGRAM_NAME, "--terms=terms.p"], &mut stdout).expect("options parse")
        else {
            panic!("expected execute");
        };
        assert_eq!(config.rule_files, vec!["-"]);
    }

    #[test]
    fn thf_rule_cnf_uses_returned_problem_type_after_global_reset() {
        let _guard = global_state_lock();
        let _problem_type_guard = super::ProblemTypeRunGuard::new();
        let config = EnormalizerConfig {
            parse_format: IoFormat::Tstp,
            rule_files: vec!["-".to_owned()],
            ..EnormalizerConfig::default()
        };
        let mut bank = new_term_bank().expect("term bank");
        let mut formulas = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let mut ignored_watchlist = ClauseSet::new();
        let mut stdin: &[u8] = b"thf(person_type, type, person: $tType).\n\
            thf(a_type, type, a: person).\n\
            thf(f_type, type, f: person > person).\n\
            thf(g_type, type, g: person > person).\n\
            thf(lambda_rule, axiom, ((^[X: person]: f @ X) @ a) = (g @ a)).\n";

        let parsed_problem_type = parse_rule_file(
            &config,
            "-",
            &mut stdin,
            &mut bank,
            &mut formulas,
            &mut ignored_watchlist,
        )
        .expect("THF rule parsing succeeds");
        assert_eq!(parsed_problem_type, ProblemType::HigherOrder);

        reset_problem_type();
        set_problem_type(ProblemType::FirstOrder).expect("test global can be reset to first-order");

        clausify_rule_formulas(&mut bank, &mut formulas, &mut clauses, parsed_problem_type)
            .expect("THF rule CNF uses returned parsed problem type");
        assert!(clauses.members() > 0);
        assert!(clauses.iter().all(|clause| clause
            .literals()
            .as_slice()
            .iter()
            .all(|literal| !literal.left().has_lambda_subterm()
                && !literal.right().has_lambda_subterm())));
    }

    #[test]
    fn fool_rule_formula_term_let_uses_formula_owner_cnf_path() {
        let _guard = global_state_lock();
        let _problem_type_guard = super::ProblemTypeRunGuard::new();
        let config = EnormalizerConfig {
            parse_format: IoFormat::Tstp,
            rule_files: vec!["-".to_owned()],
            ..EnormalizerConfig::default()
        };
        let mut bank = new_term_bank().expect("term bank");
        let mut formulas = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let mut ignored_watchlist = ClauseSet::new();
        let mut stdin: &[u8] = b"tff(a_type, type, a: $i).\n\
            tff(p_type, type, p: $i > $o).\n\
            fof(fool_owner, axiom, p($let(f:$i, f := a, f))).\n";

        let parsed_problem_type = parse_rule_file(
            &config,
            "-",
            &mut stdin,
            &mut bank,
            &mut formulas,
            &mut ignored_watchlist,
        )
        .expect("FOOL rule parsing succeeds");

        assert_eq!(parsed_problem_type, ProblemType::HigherOrder);
        let formula = formulas
            .iter()
            .find(|formula| formula.get_id(true) == "fool_owner")
            .expect("FOOL formula owner exists");
        assert!(!formula.is_clause());
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_AXIOM);

        clausify_rule_formulas(&mut bank, &mut formulas, &mut clauses, parsed_problem_type)
            .expect("FOOL formula-owner CNF succeeds");
        assert!(clauses.members() > 0);
        assert_eq!(formulas.cardinality(), 0);
    }

    #[test]
    fn fool_rule_formula_term_let_equality_uses_formula_owner_cnf_path() {
        let _guard = global_state_lock();
        let _problem_type_guard = super::ProblemTypeRunGuard::new();
        let config = EnormalizerConfig {
            parse_format: IoFormat::Tstp,
            rule_files: vec!["-".to_owned()],
            ..EnormalizerConfig::default()
        };
        let mut bank = new_term_bank().expect("term bank");
        let mut formulas = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let mut ignored_watchlist = ClauseSet::new();
        let mut stdin: &[u8] = b"tff(a_type, type, a: $i).\n\
            tff(b_type, type, b: $i).\n\
            fof(fool_eq, axiom, ($let(f:$i, f := a, f) = b)).\n";

        let parsed_problem_type = parse_rule_file(
            &config,
            "-",
            &mut stdin,
            &mut bank,
            &mut formulas,
            &mut ignored_watchlist,
        )
        .expect("FOOL equality rule parsing succeeds");

        assert_eq!(parsed_problem_type, ProblemType::HigherOrder);
        let formula = formulas
            .iter()
            .find(|formula| formula.get_id(true) == "fool_eq")
            .expect("FOOL equality formula owner exists");
        assert!(!formula.is_clause());
        assert!(formula.query_prop(CP_INPUT_FORMULA));
        assert_eq!(formula.query_tptp_type(), CP_TYPE_AXIOM);

        clausify_rule_formulas(&mut bank, &mut formulas, &mut clauses, parsed_problem_type)
            .expect("FOOL equality formula-owner CNF succeeds");
        assert!(clauses.members() > 0);
        assert_eq!(formulas.cardinality(), 0);
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        assert!(process_options([PROGRAM_NAME, "-V"], &mut stdout).is_err());

        let command = process_options([PROGRAM_NAME, "--help"], &mut stdout).expect("help option");
        assert!(matches!(command, RunCommand::Exit(0)));
        assert_eq!(
            String::from_utf8(std::mem::take(&mut stdout)).expect("utf8"),
            expected_help()
        );

        let command =
            process_options([PROGRAM_NAME, "--version"], &mut stdout).expect("version option");
        assert!(matches!(command, RunCommand::Exit(0)));
        assert_eq!(
            String::from_utf8(stdout).expect("utf8"),
            format!("{PROGRAM_NAME} {VERSION}\n")
        );
    }

    #[test]
    fn parsed_formula_targets_set_initial_and_input_formula_props_like_c() {
        let _guard = global_state_lock();
        for (format, input) in [
            (IoFormat::Tptp, "input_formula(form1,axiom,p(a))."),
            (IoFormat::Tstp, "fof(form1, axiom, p(a))."),
            (IoFormat::Tstp, "tff(person_type, type, person: $tType)."),
        ] {
            let mut bank = new_term_bank().expect("term bank");
            let mut scanner = Scanner::from_user_string(input, true).expect("scanner");
            scanner.set_format(format);

            let target = parse_wrapped_formula(&mut scanner, &mut bank).expect("formula target");

            assert!(target.formula.query_prop(CP_INITIAL));
            assert!(target.formula.query_prop(CP_INPUT_FORMULA));
        }
    }

    #[test]
    fn normalizes_terms_with_lop_rule_file() {
        let _guard = global_state_lock();
        let rule_path = temp_path("rules");
        let term_path = temp_path("terms");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-t",
                term_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "f(b) ==> a\n");

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn print_statistics_is_c_compatible_noop() {
        let _guard = global_state_lock();
        let rule_path = temp_path("stats_rules");
        let term_path = temp_path("stats_terms");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let plain_stdin_data = empty_stdin();
        let mut plain_stdin = plain_stdin_data.as_slice();
        let mut plain_stdout = Vec::new();
        let mut plain_stderr = Vec::new();
        let plain_status = run(
            [
                PROGRAM_NAME,
                "-t",
                term_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut plain_stdin,
            &mut plain_stdout,
            &mut plain_stderr,
        )
        .expect("plain normalizer run");

        let stats_stdin_data = empty_stdin();
        let mut stats_stdin = stats_stdin_data.as_slice();
        let mut stats_stdout = Vec::new();
        let mut stats_stderr = Vec::new();
        let stats_status = run(
            [
                PROGRAM_NAME,
                "--print-statistics",
                "-t",
                term_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stats_stdin,
            &mut stats_stdout,
            &mut stats_stderr,
        )
        .expect("print-statistics normalizer run");

        assert_eq!(plain_status, 0);
        assert_eq!(stats_status, 0);
        assert!(plain_stderr.is_empty());
        assert!(stats_stderr.is_empty());
        assert_eq!(plain_stdout, stats_stdout);
        assert_eq!(
            String::from_utf8(stats_stdout).expect("utf8"),
            "f(b) ==> a\n"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn normalizes_clauses_with_lop_rule_file() {
        let _guard = global_state_lock();
        let rule_path = temp_path("clause_rules");
        let clause_path = temp_path("clauses");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        fs::write(&clause_path, "p(f(b)).\n").expect("clauses written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-c",
                clause_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("p(f(b))"));
        assert!(rendered.contains("p(a)"));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(clause_path);
    }

    #[test]
    fn normalizes_terms_with_tstp_formula_rule_file() {
        let _guard = global_state_lock();
        let rule_path = temp_path("formula_owner_rules");
        let term_path = temp_path("formula_owner_terms");
        fs::write(&rule_path, "fof(rule, axiom, ! [X] : (f(X)=a)).\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-t",
                term_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "f(b) ==> a\n");

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn thf_rule_normalizes_higher_order_tstp_term_targets() {
        let _guard = global_state_lock();
        let rule_path = temp_path("thf_term_rules");
        let term_path = temp_path("thf_term_targets");
        fs::write(
            &rule_path,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(f_type, type, f: person > person).\n\
             thf(rule, axiom, (f @ a) = a).\n",
        )
        .expect("rules written");
        fs::write(&term_path, "f @ a\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let terms_arg = format!("--terms={}", term_path.to_str().expect("utf8 path"));
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                terms_arg.as_str(),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "f @ a ==> a\n");

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn warns_and_ignores_non_rewrite_rules() {
        let _guard = global_state_lock();
        let rule_path = temp_path("bad_rules");
        let term_path = temp_path("bad_terms");
        fs::write(&rule_path, "p(a) <- q(a).\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-t",
                term_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "f(b) ==> f(b)\n");
        assert!(String::from_utf8(stderr)
            .expect("utf8")
            .contains("Clause is not a rewrite rule"));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn normalizes_tstp_formula_targets() {
        let _guard = global_state_lock();
        let rule_path = temp_path("formula_rules");
        let formula_path = temp_path("formulas");
        fs::write(&rule_path, "cnf(rule, axiom, f(X)=a).\n").expect("rules written");
        fs::write(&formula_path, "fof(form1, axiom, p(f(b))).\n").expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("fof(form1, axiom, "));
        assert!(rendered.contains("p(a)"));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tstp_formula_targets_reuse_external_name_map_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("formula_variable_map_rules");
        let formula_path = temp_path("formula_variable_map_targets");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "fof(first, axiom, ?[X3,X4,X1,X2]:p(X3,X4,X1,X2)).\n\
             fof(second, axiom, ?[X1,X2,X3,X4,X5]:q(X1,X2,X3,X4,X5)).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        let rendered = String::from_utf8(stdout).expect("utf8");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(
            rendered.contains("?[X3, X4, X1, X2, X5]:(q(X3,X4,X1,X2,X5))"),
            "{rendered}"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tstp_formula_targets_accept_bracketed_useful_info_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tstp_useful_info_rules");
        let formula_path = temp_path("tstp_useful_info_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "fof(with_info, axiom, p(a), file('x.p', with_info), [status(thm)]).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).expect("utf8"),
            "fof(with_info, axiom, p(a)). ==> fof(with_info, axiom, p(a)).\n"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tff_type_declaration_formula_targets_print_true_wrapper() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tff_type_decl_rules");
        let formula_path = temp_path("tff_type_decl_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(&formula_path, "tff(person_type, type, person: $tType).\n")
            .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("fof(person_type, axiom, $true)."));
        assert!(rendered.contains(" ==> fof(person_type, axiom, $true)."));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn thf_type_declaration_formula_targets_keep_thf_output_kind() {
        let _guard = global_state_lock();
        let rule_path = temp_path("thf_type_decl_rules");
        let formula_path = temp_path("thf_type_decl_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(&formula_path, "thf(person_type, type, person: $tType).\n")
            .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("thf(person_type, axiom, $true)."));
        assert!(rendered.contains(" ==> thf(person_type, axiom, $true)."));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn thf_formula_targets_parse_typed_let_under_higher_order_problem_type() {
        let _guard = global_state_lock();
        let rule_path = temp_path("thf_let_rules");
        let formula_path = temp_path("thf_let_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(let_fact, axiom, $let(f: person > $o, f(X) := p @ X, f @ a)).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("THF $let formula target is accepted");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("thf(let_fact, axiom, "));
        assert!(rendered.contains("$let("));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn non_tcf_formula_targets_reject_watchlist_role_like_c() {
        let _guard = global_state_lock();
        for formula_kind in ["fof", "tff", "thf"] {
            let rule_path = temp_path(&format!("{formula_kind}_watchlist_rules"));
            let formula_path = temp_path(&format!("{formula_kind}_watchlist_formulas"));
            fs::write(&rule_path, "").expect("rules written");
            fs::write(
                &formula_path,
                format!("{formula_kind}(watch, watchlist, p(a)).\n"),
            )
            .expect("formulas written");

            let stdin_data = empty_stdin();
            let mut stdin = stdin_data.as_slice();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let error = run(
                [
                    PROGRAM_NAME,
                    "--tstp-in",
                    "-f",
                    formula_path.to_str().expect("utf8 path"),
                    rule_path.to_str().expect("utf8 path"),
                ],
                &mut stdin,
                &mut stdout,
                &mut stderr,
            )
            .expect_err("non-tcf watchlist roles are rejected");

            assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
            assert!(error.message().contains("watchlist"));
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());

            let _ = fs::remove_file(rule_path);
            let _ = fs::remove_file(formula_path);
        }
    }

    #[test]
    fn tstp_formula_targets_reject_non_bracketed_useful_info_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tstp_bad_useful_info_rules");
        let formula_path = temp_path("tstp_bad_useful_info_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "fof(bad_info, axiom, p(a), file('x.p', bad_info), status(thm)).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("non-bracketed useful info is rejected");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Opening square brace"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tstp_formula_targets_reject_free_variables_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tstp_free_var_rules");
        let formula_path = temp_path("tstp_free_var_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(&formula_path, "fof(free_var, axiom, p(X)).\n").expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("TSTP formula target free variables are rejected");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains(TSTP_FORMULA_FREE_VARIABLES_MESSAGE));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tstp_formula_targets_parse_distinct_body_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tstp_distinct_rules");
        let formula_path = temp_path("tstp_distinct_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "fof(distinct_caps, axiom, $distinct(X,Y)).\n\
             fof(distinct_wrapped, axiom, (($distinct(a,b)))).\n\
             fof(distinct_negated, axiom, (~($distinct(a,b)))).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--tstp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("TSTP $distinct formula target is accepted");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let rendered = String::from_utf8(stdout).expect("utf8");
        assert!(rendered.contains("fof(distinct_caps, axiom, $distinct("));
        assert!(rendered.contains(" ==> fof(distinct_caps, axiom, $distinct("));
        assert!(rendered.contains("fof(distinct_wrapped, axiom, $distinct("));
        assert!(rendered.contains("fof(distinct_negated, axiom, ~("));

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn tcf_formula_targets_use_clause_body_parser_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("tcf_bad_body_rules");
        let formula_path = temp_path("tcf_bad_body_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "tcf(bad_tcf_body, axiom, ![X]:(p(X)&q(X))).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("TCF parenthesized bodies are parsed as clauses");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn normalizes_old_tptp_formula_targets() {
        let _guard = global_state_lock();
        let rule_path = temp_path("old_tptp_formula_rules");
        let formula_path = temp_path("old_tptp_formulas");
        fs::write(&rule_path, "input_clause(rule,axiom,[++equal(f(X),a)]).\n")
            .expect("rules written");
        fs::write(&formula_path, "input_formula(form1,axiom,p(f(b))).\n")
            .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tptp-in",
                "--tptp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).expect("utf8"),
            "input_formula(form1,axiom,p(f(b))). ==> input_formula(form1,axiom,p(a)).\n"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn old_tptp_formula_targets_map_lemma_and_unknown_roles_to_axiom_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("old_tptp_role_rules");
        let formula_path = temp_path("old_tptp_role_formulas");
        fs::write(&rule_path, "").expect("rules written");
        fs::write(
            &formula_path,
            "input_formula(lemma_form,lemma,p(a)).\n\
             input_formula(unknown_form,unknown,q(a)).\n",
        )
        .expect("formulas written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--tptp-in",
                "--tptp-out",
                "-f",
                formula_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).expect("utf8"),
            "input_formula(lemma_form,axiom,p(a)). ==> input_formula(lemma_form,axiom,p(a)).\n\
             input_formula(unknown_form,axiom,q(a)). ==> input_formula(unknown_form,axiom,q(a)).\n"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(formula_path);
    }

    #[test]
    fn output_file_redirects_results() {
        let _guard = global_state_lock();
        let rule_path = temp_path("out_rules");
        let term_path = temp_path("out_terms");
        let output_path = temp_path("out");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-t",
                term_path.to_str().expect("utf8 path"),
                "-o",
                output_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");
        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        assert_eq!(
            fs::read_to_string(&output_path).expect("output read"),
            "f(b) ==> a\n"
        );

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn output_dash_routes_results_to_stdout_like_c() {
        let _guard = global_state_lock();
        let rule_path = temp_path("dash_rules");
        let term_path = temp_path("dash_terms");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        fs::write(&term_path, "f(b)\n").expect("terms written");

        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-t",
                term_path.to_str().expect("utf8 path"),
                "-o",
                "-",
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("normalizer run");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(String::from_utf8(stdout).expect("utf8"), "f(b) ==> a\n");

        let _ = fs::remove_file(rule_path);
        let _ = fs::remove_file(term_path);
    }

    #[test]
    fn rule_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing_rules");
        let _ = fs::remove_file(&missing_path);
        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, missing_path.to_str().expect("utf8 path")],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing rule file is reported");

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
    fn target_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let rule_path = temp_path("target_missing_rules");
        let missing_path = temp_path("missing_terms");
        fs::write(&rule_path, "f(X)=a.\n").expect("rules written");
        let _ = fs::remove_file(&missing_path);
        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-t",
                missing_path.to_str().expect("utf8 path"),
                rule_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing target file is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(rule_path);
    }

    #[test]
    fn output_file_is_created_before_later_rule_open_failure() {
        let _guard = global_state_lock();
        let output_path = temp_path("early_output");
        let missing_path = temp_path("missing_rules_after_output");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(&missing_path);
        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("utf8 path"),
                missing_path.to_str().expect("utf8 path"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing rule file is reported after output creation");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(output_path.exists());
        assert_eq!(fs::read_to_string(&output_path).expect("output read"), "");
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output_dir");
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_dir(&output_path);
        fs::create_dir(&output_path).expect("output fixture directory created");
        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "-o", output_path.to_str().expect("utf8 path")],
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

        fs::remove_dir(output_path).expect("output fixture directory removed");
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let stdin_data = empty_stdin();
        let mut stdin = stdin_data.as_slice();
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn print_help_preserves_full_c_text() {
        let rendered = print_help();
        assert_eq!(rendered, expected_help());
    }
}
