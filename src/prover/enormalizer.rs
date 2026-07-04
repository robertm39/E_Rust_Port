use crate::basics::defines::MEGA;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{
    current_resource_usage, format_resource_usage, get_system_phys_memory, set_memory_limit,
};
use crate::basics::partial_orderings::HoOrderKind;
use crate::basics::simple_stuff::{
    problem_type, reset_problem_type, set_problem_type, ProblemType,
};
use crate::basics::sysdate::{SysDate, SysDateIncrement};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause::{
    clause_parse, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_tstp_string, Clause,
};
use crate::clauses::clause_props::{
    clause_type_from_identifier, FormulaProperties, CP_INPUT_FORMULA, CP_TYPE_AXIOM,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::EqnPrintOptions;
use crate::clauses::eqn_props::EP_IS_ORIENTED;
use crate::clauses::formulasets::{
    FormulaPrintFormat, FormulaSet, FormulaSetCnfOptions, WrappedFormula,
};
use crate::clauses::rewrite::{clause_compute_li_normalform_plain, term_li_normalform_plain};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::inout::signals::{configure_time_limits, RLIM_INFINITY_COMPAT};
use crate::orderings::ocb::OrderControlBlock;
use crate::prover::eprover::{
    parse_clause_scanner_into_formula_set_with_options, FoolUnroll, FormulaPreprocessing,
};
use crate::prover::version::{footer, VERSION};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::{RewriteLevel, Term};
use crate::terms::termvars::VarBank;
use crate::terms::typebanks::TypeBank;
use crate::{heuristics::to_params::TermOrdering, terms::termfunc::term_is_untyped};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "enormalizer";
const ENORMALIZER_CNF_MINISCOPE_LIMIT: i64 = 1000;
const ENORMALIZER_CNF_DEF_LIMIT: i64 = 24;
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

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
        "Read terms from the named file and normalize them.",
    ),
    OptCell::new(
        OptionCode::Clauses,
        Some('c'),
        Some("clauses"),
        OptArgType::ReqArg,
        None,
        "Read clauses from the named file and normalize them.",
    ),
    OptCell::new(
        OptionCode::Formulas,
        Some('f'),
        Some("formulas"),
        OptArgType::ReqArg,
        None,
        "Read formulas from the named file and normalize them.",
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
        "Print additional statistics.",
    ),
    OptCell::new(
        OptionCode::ResourcesInfo,
        Some('R'),
        Some("resources-info"),
        OptArgType::NoArg,
        None,
        "Print resource usage information.",
    ),
    OptCell::new(
        OptionCode::LopIn,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Parse LOP input.",
    ),
    OptCell::new(
        OptionCode::TptpIn,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Parse old TPTP input.",
    ),
    OptCell::new(
        OptionCode::TptpOut,
        None,
        Some("tptp-out"),
        OptArgType::NoArg,
        None,
        "Print old TPTP output.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Parse and print old TPTP format.",
    ),
    OptCell::new(
        OptionCode::TptpIn,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Alias for --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpOut,
        None,
        Some("tptp2-out"),
        OptArgType::NoArg,
        None,
        "Alias for --tptp-out.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp2-format"),
        OptArgType::NoArg,
        None,
        "Alias for --tptp-format.",
    ),
    OptCell::new(
        OptionCode::TstpIn,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TSTP input.",
    ),
    OptCell::new(
        OptionCode::TstpOut,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "Print TSTP output.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Parse and print TSTP format.",
    ),
    OptCell::new(
        OptionCode::TstpIn,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Alias for --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpOut,
        None,
        Some("tptp3-out"),
        OptArgType::NoArg,
        None,
        "Alias for --tstp-out.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Alias for --tstp-format.",
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
        "Set the soft cpu time limit.",
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
    set_problem_type(ProblemType::FirstOrder)?;
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

    for file in &config.rule_files {
        let mut scanner = scanner_for_input(file, stdin)?;
        parse_clause_scanner_into_formula_set_with_options(
            &mut scanner,
            config.parse_format,
            FormulaPreprocessing::parse_only(FoolUnroll::Enabled),
            Default::default(),
            &mut bank,
            &mut formulas,
            &mut ignored_watchlist,
        )?;
    }
    clausify_rule_formulas(&mut bank, &mut formulas, &mut clauses)?;

    let demodulators = build_rw_system(&mut clauses, &bank, config, stderr)?;
    let mut ocb = OrderControlBlock::alloc(
        TermOrdering::Empty,
        false,
        bank.signature(),
        HoOrderKind::LambdaOrder,
    );

    if let Some(name) = config.term_file.as_deref() {
        process_terms(
            name,
            config,
            stdin,
            &mut output,
            &mut bank,
            &mut ocb,
            &demodulators,
        )?;
    }
    if let Some(name) = config.clause_file.as_deref() {
        process_clauses(
            name,
            config,
            stdin,
            &mut output,
            &mut bank,
            &mut ocb,
            &demodulators,
        )?;
    }
    if let Some(name) = config.formula_file.as_deref() {
        process_formulas(
            name,
            config,
            stdin,
            &mut output,
            &mut bank,
            &mut ocb,
            &demodulators,
        )?;
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

fn clausify_rule_formulas(
    bank: &mut TermBank,
    formulas: &mut FormulaSet,
    clauses: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    let mut archive = FormulaSet::new();
    let _preprocessed = formulas.preproc_conjectures(bank, false, false)?;
    let fresh_vars = VarBank::new(bank.signature().type_bank());
    let options = FormulaSetCnfOptions::new(ENORMALIZER_CNF_MINISCOPE_LIMIT, true, problem_type())
        .with_def_limit(ENORMALIZER_CNF_DEF_LIMIT);
    let _cnf = formulas.cnf2_into(&mut archive, clauses, bank, &fresh_vars, options)?;
    Ok(())
}

fn build_rw_system(
    clauses: &mut ClauseSet,
    bank: &TermBank,
    config: &EnormalizerConfig,
    stderr: &mut impl Write,
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
            let rendered = render_clause(bank, &clause, config)?;
            writeln_diag(
                stderr,
                &format!("{PROGRAM_NAME}: Clause is not a rewrite rule: {rendered} -- ignoring"),
            )?;
        }
    }
    Ok(demodulators)
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
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    demodulators: &ClauseSet,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let original = bank.parse_term_simple(&mut scanner)?;
        let normalized = normalize_term(bank, ocb, &original, demodulators)?;
        writeln_diag(
            output,
            &format!(
                "{} ==> {}",
                bank.term_string(&original, true),
                bank.term_string(&normalized, true)
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
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    demodulators: &ClauseSet,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let mut clause = clause_parse(&mut scanner, bank, ProblemType::FirstOrder)?;
        let original = render_clause(bank, &clause, config)?;
        clause_compute_li_normalform_plain(
            bank,
            ocb,
            &mut clause,
            &[demodulators],
            RewriteLevel::RuleRewrite,
            false,
            false,
        )?;
        let normalized = render_clause(bank, &clause, config)?;
        writeln_diag(output, &format!("{original} ==> {normalized}"))?;
    }
    Ok(())
}

fn process_formulas(
    name: &str,
    config: &EnormalizerConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
    bank: &mut TermBank,
    ocb: &mut OrderControlBlock,
    demodulators: &ClauseSet,
) -> Result<(), Diagnostic> {
    let mut scanner = formatted_scanner_for_input(name, stdin, config.parse_format)?;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        let mut formula = parse_wrapped_formula(&mut scanner, bank)?;
        let original = render_formula(bank, &formula, config)?;
        let normalized = normalize_term(bank, ocb, formula.formula(), demodulators)?;
        formula.set_formula(normalized);
        let normalized = render_formula(bank, &formula, config)?;
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
) -> Result<WrappedFormula, Diagnostic> {
    match scanner.format() {
        IoFormat::Tptp => parse_old_tptp_wrapped_formula(scanner, bank),
        IoFormat::Tstp => parse_tstp_wrapped_formula(scanner, bank),
        IoFormat::Lop | IoFormat::Auto => Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Formula parsing is only supported for TPTP/TSTP input",
        )),
    }
}

fn parse_old_tptp_wrapped_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<WrappedFormula, Diagnostic> {
    bank.vars().clear_ext_names();
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
    Ok(wrapped_formula(
        formula,
        old_tptp_input_formula_type(&role),
        &name,
        &start_source,
        start_line,
        start_column,
    ))
}

fn parse_tstp_wrapped_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<WrappedFormula, Diagnostic> {
    bank.vars().clear_ext_names();
    let start_source = token_source_string(scanner.current_token().source_bytes());
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());
    let formula_kind = scanner.current_token().literal();
    let formula_problem_type = tstp_formula_kind_problem_type(&formula_kind);
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
        return parse_wrapped_formula(scanner, bank);
    }
    scanner.check_id(
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question|watchlist",
    )?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    let formula = bank.parse_tformula_tstp(scanner)?;
    skip_tstp_optional_source(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(wrapped_formula(
        formula,
        clause_type_from_identifier(&role, formula_problem_type),
        &name,
        &start_source,
        start_line,
        start_column,
    ))
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
    wrapper.set_prop(CP_INPUT_FORMULA);
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
    config: &EnormalizerConfig,
) -> Result<String, Diagnostic> {
    formula.print_string(
        bank,
        true,
        if term_is_untyped(formula.formula()) {
            ProblemType::FirstOrder
        } else {
            ProblemType::HigherOrder
        },
        formula_print_format(config.output_format),
        true,
    )
}

fn render_clause(
    bank: &TermBank,
    clause: &Clause,
    config: &EnormalizerConfig,
) -> Result<String, Diagnostic> {
    match config.output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank,
            clause,
            config.eqn_options,
        )),
        IoFormat::Tstp => clause_tstp_string(bank, clause, true, true, ProblemType::FirstOrder),
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
Read a set of rewrite rules and use them to normalize terms, clauses, or formulas.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
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
        "axiom" | "hypothesis" | "conjecture" | "negated_conjecture" | "question" | "lemma"
        | "unknown" => clause_type_from_identifier(role, ProblemType::FirstOrder),
        _ => CP_TYPE_AXIOM,
    }
}

fn tstp_formula_kind_problem_type(kind: &str) -> ProblemType {
    if kind == "thf" {
        ProblemType::HigherOrder
    } else {
        ProblemType::FirstOrder
    }
}

fn skip_tstp_optional_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if !scanner.test_tok(TokenType::COMMA) {
        return Ok(());
    }
    scanner.accept_tok(TokenType::COMMA)?;
    skip_parenthesized_or_atom(scanner)?;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        skip_parenthesized_or_atom(scanner)?;
    }
    Ok(())
}

fn skip_parenthesized_or_atom(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    let mut depth = 0_i32;
    loop {
        if scanner.test_tok(TokenType::NO_TOKEN) {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Unexpected end of input while skipping TSTP source",
            ));
        }
        if depth == 0 && scanner.test_tok(TokenType::COMMA | TokenType::CLOSE_BRACKET) {
            return Ok(());
        }
        if scanner.test_tok(TokenType::OPEN_BRACKET | TokenType::OPEN_SQUARE) {
            depth += 1;
        } else if scanner.test_tok(TokenType::CLOSE_BRACKET | TokenType::CLOSE_SQUARE) {
            if depth == 0 {
                return Ok(());
            }
            depth -= 1;
        }
        scanner.next_token()?;
    }
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
        memory_limit_bytes_from_mb, print_help, process_options, run, RunCommand,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::IoFormat;
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
    fn version_is_long_only_like_c_tool() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        assert!(process_options([PROGRAM_NAME, "-V"], &mut stdout).is_err());
        let command =
            process_options([PROGRAM_NAME, "--version"], &mut stdout).expect("version option");
        assert!(matches!(command, RunCommand::Exit(0)));
        assert!(String::from_utf8(stdout)
            .expect("utf8")
            .contains("enormalizer "));
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
    fn print_help_mentions_rulefiles() {
        let rendered = print_help();
        assert!(rendered.contains("Usage: enormalizer [options] [rulefiles]"));
        assert!(rendered.contains("--terms"));
        assert!(rendered.contains("--clauses"));
        assert!(rendered.contains("--formulas"));
    }
}
