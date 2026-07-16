use std::fs::File;
use std::io;
use std::io::Write as IoWrite;
use std::path::{Path, PathBuf};

use crate::basics::dstrings::DynamicString;
use crate::basics::error::{check_option_letter_string, Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{
    jkiss_rand_double, sort_weighted_objects, ProblemType, WeightedObject,
};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause_props::{
    FormulaProperties, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
};
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
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::prover::version::{footer, E_NICKNAME, VERSION};
use crate::terms::{
    functypes::{func_symb_parse, FunCode},
    signature::Signature,
    termbanks::TermBank,
    typebanks::TypeBank,
};

pub const PROGRAM_NAME: &str = "e_axfilter";
const C_USAGE_ERROR: &str = "Usage: e_axfilter <problem> [<options>]\n";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

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
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
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
        execute_with_output(config, output_file, Some(stdout))?;
        output_file
            .flush()
            .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    } else {
        execute_with_output(config, stdout, None)?;
    }
    Ok(ErrorCode::NO_ERROR.exit_status())
}

fn execute_with_output<W: IoWrite + ?Sized>(
    config: &EAxFilterConfig,
    output: &mut W,
    seed_name_output: Option<&mut dyn IoWrite>,
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
    let corename = file_name_strip(&config.files[0]);
    if config.seed_filtering_requested() {
        seeded_filters(
            &mut bank,
            &mut ctrl,
            &filters,
            &corename,
            config,
            output,
            seed_name_output,
        )
    } else {
        all_filters_problem(
            &mut bank, &mut ctrl, &filters, &corename, false, None, output,
        )
    }
}

fn seeded_filters<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filters: &AxFilterSet,
    corename: &str,
    config: &EAxFilterConfig,
    output: &mut W,
    mut seed_name_output: Option<&mut dyn IoWrite>,
) -> Result<(), Diagnostic> {
    let mut seed_symbols = if let Some(seedstr) = &config.seedstr {
        decode_seed_symbols(bank.signature(), seedstr)?
    } else {
        let mut seed_symbols = find_seed_symbols(bank.signature(), config);
        subsample_seed_symbols(ctrl, &mut seed_symbols, config);
        seed_symbols
    };

    while let Some(seed_symbol) = seed_symbols.pop() {
        let mut formula_ids = Vec::new();
        let _matches = ctrl.collect_f_code(seed_symbol, &mut formula_ids);

        if config.seed_all {
            seeded_filter_all(
                bank,
                ctrl,
                filters,
                corename,
                seed_symbol,
                &formula_ids,
                output,
                &mut seed_name_output,
            )?;
        }
        if config.seed_large {
            seeded_filter_largest(
                bank,
                ctrl,
                filters,
                corename,
                seed_symbol,
                &formula_ids,
                output,
                &mut seed_name_output,
            )?;
        }
        if config.seed_diverse {
            seeded_filter_diverse(
                bank,
                ctrl,
                filters,
                corename,
                seed_symbol,
                &formula_ids,
                output,
                &mut seed_name_output,
            )?;
        }
    }
    Ok(())
}

fn find_seed_symbols(signature: &Signature, config: &EAxFilterConfig) -> Vec<FunCode> {
    let mut result = Vec::new();
    for f_code in signature.internal_symbols() + 1..=signature.f_count() {
        let is_predicate = signature.is_predicate(f_code);
        let arity = signature.find_arity(f_code).unwrap_or(0);
        let selected = if is_predicate {
            config.seed_preds
        } else if arity > 0 {
            config.seed_funs
        } else {
            config.seed_consts
        };
        if selected {
            result.push(f_code);
        }
    }
    result
}

#[allow(clippy::cast_precision_loss)]
fn subsample_seed_symbols(
    ctrl: &StructFofSpec,
    seed_symbols: &mut Vec<FunCode>,
    config: &EAxFilterConfig,
) {
    if config.subsample == SubsampleMethod::None {
        return;
    }

    let mut weighted = Vec::with_capacity(seed_symbols.len());
    while let Some(symbol) = seed_symbols.pop() {
        let weight = match config.subsample {
            SubsampleMethod::None => unreachable!("handled before sampling"),
            SubsampleMethod::Random => jkiss_rand_double(None),
            SubsampleMethod::Most => ctrl
                .f_distrib()
                .entry(symbol)
                .map_or(0.0, |entry| entry.fc_freq() as f64),
            SubsampleMethod::Least => ctrl
                .f_distrib()
                .entry(symbol)
                .map_or(0.0, |entry| -(entry.fc_freq() as f64)),
        };
        weighted.push(WeightedObject {
            weight,
            object: symbol,
        });
    }

    sort_weighted_objects(&mut weighted);
    let limit = usize::try_from(config.sample_size)
        .unwrap_or(usize::MAX)
        .min(weighted.len());
    seed_symbols.extend(weighted.into_iter().take(limit).map(|entry| entry.object));
}

fn decode_seed_symbols(signature: &Signature, seedstr: &str) -> Result<Vec<FunCode>, Diagnostic> {
    let mut scanner = Scanner::from_user_string(seedstr, true)?;
    let mut result = Vec::new();

    loop {
        let symbol = parse_seed_symbol(signature, &mut scanner)?;
        result.push(symbol);
        if !scanner.test_tok(TokenType::COMMA) {
            break;
        }
        scanner.accept_tok(TokenType::COMMA)?;
    }

    Ok(result)
}

fn parse_seed_symbol(signature: &Signature, scanner: &mut Scanner) -> Result<FunCode, Diagnostic> {
    let mut id = DynamicString::new();
    func_symb_parse(scanner, &mut id)?;
    let name = id.view();
    let f_code = signature.find_f_code(name.as_ref());
    if f_code == 0 {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("User-requested symbol {name} unknown while parsing option --seeds"),
        ));
    }
    Ok(f_code)
}

#[allow(
    clippy::too_many_arguments,
    reason = "The helper shape mirrors e_axfilter.c seeded_filter_all inputs."
)]
fn seeded_filter_all<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filters: &AxFilterSet,
    corename: &str,
    seed_symbol: FunCode,
    formula_ids: &[u64],
    output: &mut W,
    seed_name_output: &mut Option<&mut dyn IoWrite>,
) -> Result<(), Diagnostic> {
    formula_stack_cond_set_type(ctrl, formula_ids, CP_TYPE_HYPOTHESIS)?;
    let desc = seed_desc(bank.signature(), seed_symbol, "All")?;
    let name = seed_problem_name(bank.signature(), corename, "SA", seed_symbol)?;
    write_seed_name(output, seed_name_output, &name)?;
    all_filters_problem(bank, ctrl, filters, &name, true, Some(&desc), output)?;
    formula_stack_cond_set_type(ctrl, formula_ids, CP_TYPE_AXIOM)
}

#[allow(
    clippy::too_many_arguments,
    reason = "The helper shape mirrors e_axfilter.c seeded_filter_largest inputs."
)]
fn seeded_filter_largest<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filters: &AxFilterSet,
    corename: &str,
    seed_symbol: FunCode,
    formula_ids: &[u64],
    output: &mut W,
    seed_name_output: &mut Option<&mut dyn IoWrite>,
) -> Result<(), Diagnostic> {
    let mut largest = None;
    let mut last = None;
    let mut max_size = 0_i64;
    for &entry_id in formula_ids {
        let formula = lookup_formula(ctrl, entry_id)?;
        let size = formula.standard_weight();
        if size > max_size {
            largest = Some(entry_id);
            max_size = size;
        }
        last = Some(entry_id);
    }

    if let Some(entry_id) = largest {
        if lookup_formula(ctrl, entry_id)?.query_tptp_type() == CP_TYPE_AXIOM {
            set_formula_type(ctrl, entry_id, CP_TYPE_HYPOTHESIS)?;
        }
    }

    let desc = seed_desc(bank.signature(), seed_symbol, "Largest")?;
    let name = seed_problem_name(bank.signature(), corename, "SL", seed_symbol)?;
    write_seed_name(output, seed_name_output, &name)?;
    all_filters_problem(bank, ctrl, filters, &name, true, Some(&desc), output)?;

    if let Some(entry_id) = largest {
        if lookup_formula(ctrl, entry_id)?.query_tptp_type() == CP_TYPE_HYPOTHESIS {
            if let Some(last_id) = last {
                // Preserve e_axfilter.c: it restores the last scanned handle,
                // not necessarily the largest one that was changed.
                set_formula_type(ctrl, last_id, CP_TYPE_AXIOM)?;
            }
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "The helper shape mirrors e_axfilter.c seeded_filter_diverse inputs."
)]
fn seeded_filter_diverse<W: IoWrite + ?Sized>(
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    filters: &AxFilterSet,
    corename: &str,
    seed_symbol: FunCode,
    formula_ids: &[u64],
    output: &mut W,
    seed_name_output: &mut Option<&mut dyn IoWrite>,
) -> Result<(), Diagnostic> {
    let mut largest = None;
    let mut last = None;
    let mut max_size = 0_i64;
    for &entry_id in formula_ids {
        let formula = lookup_formula(ctrl, entry_id)?;
        let size = formula.symbol_diversity();
        if size > max_size {
            largest = Some(entry_id);
            max_size = size;
        }
        last = Some(entry_id);
    }

    if let Some(entry_id) = largest {
        if lookup_formula(ctrl, entry_id)?.query_tptp_type() == CP_TYPE_AXIOM {
            if let Some(last_id) = last {
                // Preserve e_axfilter.c: the diverse mode marks the last
                // scanned handle after selecting the most diverse formula.
                set_formula_type(ctrl, last_id, CP_TYPE_HYPOTHESIS)?;
            }
        }
    }

    let desc = seed_desc(bank.signature(), seed_symbol, "Diverse")?;
    let name = seed_problem_name(bank.signature(), corename, "SD", seed_symbol)?;
    write_seed_name(output, seed_name_output, &name)?;
    all_filters_problem(bank, ctrl, filters, &name, true, Some(&desc), output)?;

    if let Some(entry_id) = largest {
        if lookup_formula(ctrl, entry_id)?.query_tptp_type() == CP_TYPE_HYPOTHESIS {
            set_formula_type(ctrl, entry_id, CP_TYPE_AXIOM)?;
        }
    }
    Ok(())
}

fn write_seed_name<W: IoWrite + ?Sized>(
    fallback: &mut W,
    seed_name_output: &mut Option<&mut dyn IoWrite>,
    name: &str,
) -> Result<(), Diagnostic> {
    if let Some(output) = seed_name_output.as_deref_mut() {
        writeln_diag(output, &format!("Name: {name}"))
    } else {
        writeln_diag(fallback, &format!("Name: {name}"))
    }
}

fn formula_stack_cond_set_type(
    ctrl: &mut StructFofSpec,
    formula_ids: &[u64],
    type_: FormulaProperties,
) -> Result<(), Diagnostic> {
    for &entry_id in formula_ids {
        let formula = lookup_formula_mut(ctrl, entry_id)?;
        if formula.query_tptp_type() != CP_TYPE_CONJECTURE || type_ == CP_TYPE_CONJECTURE {
            formula.set_tptp_type(type_);
        }
    }
    Ok(())
}

fn seed_desc(
    signature: &Signature,
    seed_symbol: FunCode,
    method: &str,
) -> Result<String, Diagnostic> {
    let symbol_name = signature
        .find_name(seed_symbol)
        .ok_or_else(|| unknown_seed_symbol_diagnostic(seed_symbol))?;
    let arity = signature
        .find_arity(seed_symbol)
        .ok_or_else(|| unknown_seed_symbol_diagnostic(seed_symbol))?;
    let symbol_type = if signature.is_predicate(seed_symbol) {
        "Predicate"
    } else {
        "Function"
    };
    Ok(format!(
        "% Seed symbol: {symbol_name} = {seed_symbol}\n\
% Seeds      : {method}\n\
% Arity      : {arity}\n\
% Type       : {symbol_type}\n"
    ))
}

fn seed_problem_name(
    signature: &Signature,
    corename: &str,
    method_code: &str,
    seed_symbol: FunCode,
) -> Result<String, Diagnostic> {
    let symbol_kind = if signature.is_predicate(seed_symbol) {
        "P"
    } else {
        "F"
    };
    let arity = signature
        .find_arity(seed_symbol)
        .ok_or_else(|| unknown_seed_symbol_diagnostic(seed_symbol))?;
    Ok(format!(
        "{corename}_{method_code}_{symbol_kind}{arity}_{seed_symbol}"
    ))
}

fn lookup_formula(
    ctrl: &StructFofSpec,
    entry_id: u64,
) -> Result<&crate::clauses::formulasets::WrappedFormula, Diagnostic> {
    ctrl.formula_by_entry_id(entry_id).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::INTERFACE_ERROR,
            format!("Formula entry {entry_id} missing during seeded filtering"),
        )
    })
}

fn lookup_formula_mut(
    ctrl: &mut StructFofSpec,
    entry_id: u64,
) -> Result<&mut crate::clauses::formulasets::WrappedFormula, Diagnostic> {
    ctrl.formula_by_entry_id_mut(entry_id).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::INTERFACE_ERROR,
            format!("Formula entry {entry_id} missing during seeded filtering"),
        )
    })
}

fn set_formula_type(
    ctrl: &mut StructFofSpec,
    entry_id: u64,
    type_: FormulaProperties,
) -> Result<(), Diagnostic> {
    lookup_formula_mut(ctrl, entry_id)?.set_tptp_type(type_);
    Ok(())
}

fn unknown_seed_symbol_diagnostic(seed_symbol: FunCode) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::INTERFACE_ERROR,
        format!("Seed symbol {seed_symbol} missing from signature"),
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
    let problem_type = match ctrl.problem_type() {
        ProblemType::NotInitialized => ProblemType::FirstOrder,
        problem_type => problem_type,
    };
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
        .print_type_decls_tstp(&mut rendered, problem_type)
        .map_err(|error| io_diagnostic(format!("Cannot write TSTP type declarations: {error}")))?;

    let clauses = pstack_clause_print_tstp_string(bank, &selection.clauses, problem_type)?;
    write_all(&mut rendered, clauses.as_bytes())?;
    let formulas = pstack_formula_print_tstp_string(bank, &selection.formulas, problem_type, true)?;
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
    let mut scanner = Scanner::from_file(path, true).map_err(e_axfilter_scanner_open_diagnostic)?;
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
    File::create(path).map(Some).map_err(|error| {
        e_axfilter_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
    })
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

fn e_axfilter_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn e_axfilter_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};

    use super::{
        init_struct_fof_spec, parse_seed_subsample_arg, print_help, process_options, run,
        subsample_seed_symbols, EAxFilterConfig, RunCommand, SubsampleMethod, C_USAGE_ERROR,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::reset_jkiss_for_tests;
    use crate::basics::verbose::verbose_level;
    use crate::control::sine::StructFofSpec;
    use crate::inout::output::output_level;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::{footer, E_NICKNAME, VERSION};
    use crate::terms::signature::Signature;
    use crate::terms::typebanks::TypeBank;
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

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
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

    fn seed_names(output: &str) -> Vec<String> {
        output
            .lines()
            .filter_map(|line| line.strip_prefix("Name: "))
            .map(str::to_owned)
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        let mut expected = format!(
            concat!(
                "\n",
                "e_axfilter {version} \"{nickname}\"\n",
                "\n",
                "Usage: e_axfilter [options] [files]\n",
                "\n",
                "This program applies SinE-like goal-directed filters to a problem\n",
                "specification (a set of clauses and/or formulas) to generate reduced\n",
                "problem specifications that are easier to handle for a theorem prover,\n",
                "but still are likely to contain all the axioms necessary for a proof\n",
                "(if one exists).\n",
                "\n",
                "In default mode, the program reads a problem specification and an\n",
                "(optional) filter specification, and produces one reduced output file \n",
                "for each filter given. Note that while all standard input formats (LOP,\n",
                "TPTP-2 and TPTP-3 are supported, output is only and automatically in\n",
                "TPTP-3. Also note that unlike most of the other tools in the E\n",
                "distribution, this program does not support pipe-based input and output,\n",
                "since it uses file names generated from the input file name and filter\n",
                "names to store the different result files\n",
                "\n",
                "Options:\n",
                "\n",
                "   -h\n",
                "  --help\n",
                "    Print a short description of program usage and options.\n",
                "\n",
                "   -V\n",
                "  --version\n",
                "    Print the version number of the prover. Please include this with all bug\n",
                "    reports (if any).\n",
                "\n",
                "   -v\n",
                "  --verbose[=<arg>]\n",
                "    Verbose comments on the progress of the program. This technical\n",
                "    information is printed to stderr. The short form or the long form without\n",
                "    the optional argument is equivalent to --verbose=1.\n",
                "\n",
                "   -o <arg>\n",
                "  --output-file=<arg>\n",
                "    Redirect output into the named file (this affects only some output, as\n",
                "    most is written to automatically generated files based on the input and\n",
                "    filter names.\n",
                "\n",
                "   -s\n",
                "  --silent\n",
                "    Equivalent to --output-level=0.\n",
                "\n",
                "   -l <arg>\n",
                "  --output-level=<arg>\n",
                "    Select an output level, greater values imply more verbose output.\n",
                "\n",
                "   -f <arg>\n",
                "  --filter=<arg>\n",
                "    Specify the filter definition file. If not set, the system will uses the\n",
                "    built-in default.\n",
                "\n",
                "   -S\n",
                "  --seed-symbols[=<arg>]\n",
                "    Enable artificial seeding of the axiom selection process and determine\n",
                "    which symbol classes should be used to generate different sets.The\n",
                "    argument is a string of letters, each indicating one class of symbols to\n",
                "    use. 'p' indicates predicate symbols, 'f' non-constant function symbols,\n",
                "    and 'c' constants. Note that this will create potentially multiple output\n",
                "    files for each activated symbols. The short form or the long form without\n",
                "    the optional argument is equivalent to --seed-symbols=p.\n",
                "\n",
                "  --seeds=<arg>\n",
                "    Explicitly specify the symbols that should be used as seed symbols for\n",
                "    axiom extraction. This overwrites --seed-subsample and --seed-symbols.\n",
                "\n",
                "  --seed-subsample[=<arg>]\n",
                "    Subsample from the set of eligible seed symbols. The argument is a\n",
                "    one-character designator for the method ('m' uses the symbols that occur\n",
                "    in the most input formulas, 'l' uses the symbols that occur in the least\n",
                "    number of formulas, and 'r' samples randomly), followed by the number of\n",
                "    symbols to select. The option without the optional argument is equivalent\n",
                "    to --seed-subsample=r1000.\n",
                "\n",
                "   -m\n",
                "  --seed-method[=<arg>]\n",
                "    Specify how to select seed axioms when artificially seeding is used.The\n",
                "    argument is a string of letters, each indicating one method to use. The\n",
                "    letters are: \n",
                "    'l': use the syntactically largest axiom in which the seed symbol occurs.\n",
                "    'd': use the most diverse axiom in which the seed symbol occurs, i.e. the\n",
                "    symbol with the largest set of different symbols.\n",
                "    'a': use all axioms in which the seed symbol occurs.\n",
                "    For 'l' and 'd', if there are multiple candidates, use the first one.If\n",
                "    the option is not set, 'a' is assumed. The short form or the long form\n",
                "    without the optional argument is equivalent to --seed-method=lda.\n",
                "\n",
                "   -d\n",
                "  --dump-filter\n",
                "    Print the filter definition in force.\n",
                "\n",
                "  --lop-in\n",
                "    Set E-LOP as the input format. If no input format is selected by this or\n",
                "    one of the following options, E will guess the input format based on the\n",
                "    first token. It will almost always correctly recognize TPTP-3, but it may\n",
                "    misidentify E-LOP files that use TPTP meta-identifiers as logical\n",
                "    symbols.\n",
                "\n",
                "  --lop-format\n",
                "    Equivalent to --lop-in.\n",
                "\n",
                "  --tptp-in\n",
                "    Parse TPTP-2 format instead of E-LOP (but note that includes are handled\n",
                "    according to TPTP-3 semantics).\n",
                "\n",
                "  --tptp-format\n",
                "    Equivalent to --tptp-in.\n",
                "\n",
                "  --tptp2-in\n",
                "    Synonymous with --tptp-in.\n",
                "\n",
                "  --tptp2-format\n",
                "    Synonymous with --tptp-in.\n",
                "\n",
                "  --tstp-in\n",
                "    Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still\n",
                "    under development, and the version in E may not be fully conforming at\n",
                "    all times. E works on all TPTP 6.3.0 FOF and CNF input files (including\n",
                "    includes).\n",
                "\n",
                "  --tstp-format\n",
                "    Equivalent to --tstp-in.\n",
                "\n",
                "  --tptp3-in\n",
                "    Synonymous with --tstp-in.\n",
                "\n",
                "  --tptp3-format\n",
                "    Synonymous with --tstp-in.\n",
                "\n",
                "\n",
                "\n",
            ),
            version = VERSION,
            nickname = E_NICKNAME,
        );
        expected.push_str(&footer());
        expected
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).expect("help");

        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).expect("help is utf8");
        assert_eq!(help, expected_help());
        assert!(stderr.is_empty());

        let mut stdout = Vec::new();
        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).expect("version");

        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).expect("version utf8"),
            format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}\n")
        );
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
    fn output_dash_routes_configured_output_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--dump-filter", "-o", "-"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), C_USAGE_ERROR);
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(output.contains("threshold010000 = Threshold(10000)"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        remove_if_present(&output_path);
        _ = std::fs::remove_dir(&output_path);
        std::fs::create_dir(&output_path).expect("output fixture directory is created");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("path is utf8"),
            ],
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

        std::fs::remove_dir(output_path).expect("output fixture directory is removed");
    }

    #[test]
    fn filter_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_filter = temp_path("missing-filter");
        remove_if_present(&missing_filter);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-f",
                missing_filter.to_str().expect("path is utf8"),
                "problem.p",
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_filter.display()
        )));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
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
    fn thf_filter_output_preserves_higher_order_tstp_wrappers() {
        let _guard = global_state_lock();
        let problem_path = temp_path("thf-problem");
        let filter_path = temp_path("thf-filters");
        let generated_path = generated_path(&problem_path, "tiny");
        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
        std::fs::write(
            &problem_path,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(fact, axiom, p @ a).\n",
        )
        .expect("problem written");
        std::fs::write(&filter_path, "tiny=Threshold(10000)\n").expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("filter run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let generated = std::fs::read_to_string(&generated_path).expect("generated output exists");
        assert!(generated.contains("thf(fact, axiom"));
        assert!(!generated.contains("tff(fact, axiom"));
        assert!(!generated.contains("fof(fact, axiom"));

        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn formula_gsine_filter_selects_related_formula_owners_only() {
        let _guard = global_state_lock();
        let problem_path = temp_path("formula-gsine-problem");
        let filter_path = temp_path("formula-gsine-filters");
        let generated_path = generated_path(&problem_path, "formulas");
        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
        std::fs::write(
            &problem_path,
            "fof(goal, conjecture, p(goal_a)).\n\
             fof(link, axiom, (p(goal_a) => q(link_b))).\n\
             fof(far, axiom, r(far_c)).\n",
        )
        .expect("problem written");
        std::fs::write(
            &filter_path,
            "formulas=GSinE(CountTerms, ,false,10.0,100,100,10000,1.0)\n",
        )
        .expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("formula GSinE run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let generated = std::fs::read_to_string(&generated_path).expect("generated output exists");
        assert!(generated.contains("fof(goal, conjecture"));
        assert!(generated.contains("fof(link, axiom"));
        assert!(!generated.contains("fof(far,"));

        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn lambda_def_filter_prints_definition_and_goal_formula_owners_only() {
        let _guard = global_state_lock();
        let problem_path = temp_path("lambda-def-problem");
        let filter_path = temp_path("lambda-def-filters");
        let generated_path = generated_path(&problem_path, "defs");
        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
        std::fs::write(
            &problem_path,
            "thf(person_type, type, person: $tType).\n\
             thf(a_type, type, a: person).\n\
             thf(p_type, type, p: person > $o).\n\
             thf(q_type, type, q: person > $o).\n\
             thf(lambda_def, definition, p = (^[X: person]: q @ X)).\n\
             thf(goal, conjecture, p @ a).\n\
             thf(far, axiom, q @ a).\n",
        )
        .expect("problem written");
        std::fs::write(&filter_path, "defs=LambdaDef\n").expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("LambdaDef run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let generated = std::fs::read_to_string(&generated_path).expect("generated output exists");
        assert!(generated.contains("thf(lambda_def,"));
        assert!(generated.contains("thf(goal, conjecture"));
        assert!(!generated.contains("thf(far,"));

        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_explicit_symbol_generates_hypothesis_seeded_filter_file() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-problem");
        let filter_path = temp_path("seeded-filters");
        remove_if_present(&problem_path);
        remove_if_present(&filter_path);
        std::fs::write(
            &problem_path,
            "fof(seed, axiom, p(a)).\nfof(other, axiom, q(a)).\n",
        )
        .expect("problem written");
        std::fs::write(
            &filter_path,
            "seed=GSinE(CountTerms,hypos,false,1.0,100,100,10000,1.0)\n",
        )
        .expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                "--seeds=p",
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("seeded run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(output.contains("% Parsing "));
        let seeded_name = output
            .lines()
            .find_map(|line| line.strip_prefix("Name: "))
            .expect("seeded name is printed")
            .to_owned();
        assert!(seeded_name.contains("_SA_P1_"));
        assert!(output.contains(&format!(
            "% Filter: seed goes into file {seeded_name}_seed.p"
        )));

        let generated_path = PathBuf::from(format!("{seeded_name}_seed.p"));
        let generated = std::fs::read_to_string(&generated_path).expect("generated output exists");
        assert!(generated.contains("% Seeds      : All"));
        assert!(generated.contains("% Type       : Predicate"));
        assert!(generated.contains("hypothesis"));
        assert!(generated.contains("p(a)"));

        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_name_uses_stdout_even_with_output_file() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-output-problem");
        let filter_path = temp_path("seeded-output-filters");
        let output_path = temp_path("seeded-global-output");
        for path in [&problem_path, &filter_path, &output_path] {
            remove_if_present(path);
        }
        std::fs::write(&problem_path, "fof(seed, axiom, p(a)).\n").expect("problem written");
        std::fs::write(
            &filter_path,
            "seed=GSinE(CountTerms,hypos,false,1.0,100,100,10000,1.0)\n",
        )
        .expect("filters written");
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
                "--seeds=p",
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("seeded run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).expect("stdout is utf8");
        let seeded_name = stdout
            .lines()
            .find_map(|line| line.strip_prefix("Name: "))
            .expect("seeded name is printed")
            .to_owned();
        assert!(!stdout.contains("% Filter: seed goes into file"));

        let global_output = std::fs::read_to_string(&output_path).expect("global output exists");
        assert!(global_output.contains("% Parsing "));
        assert!(global_output.contains(&format!(
            "% Filter: seed goes into file {seeded_name}_seed.p"
        )));
        assert!(!global_output.contains("Name: "));

        let generated_path = PathBuf::from(format!("{seeded_name}_seed.p"));
        for path in [&problem_path, &filter_path, &output_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_duplicate_explicit_symbols_repeat_generated_seed_work() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-duplicate-problem");
        let filter_path = temp_path("seeded-duplicate-filters");
        for path in [&problem_path, &filter_path] {
            remove_if_present(path);
        }
        std::fs::write(&problem_path, "fof(seed, axiom, p(a)).\n").expect("problem written");
        std::fs::write(
            &filter_path,
            "seed=GSinE(CountTerms,hypos,false,1.0,100,100,10000,1.0)\n",
        )
        .expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                "--seeds=p,p",
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("seeded run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        let names = seed_names(&output);
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], names[1]);
        assert_eq!(
            output.matches("% Filter: seed goes into file").count(),
            2,
            "duplicate explicit seed symbols should rerun the same generated work"
        );

        let generated_path = PathBuf::from(format!("{}_seed.p", names[0]));
        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_large_and_diverse_methods_generate_distinct_outputs() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-ld-problem");
        let filter_path = temp_path("seeded-ld-filters");
        for path in [&problem_path, &filter_path] {
            remove_if_present(path);
        }
        std::fs::write(
            &problem_path,
            "fof(small, axiom, p(a)).\nfof(large, axiom, p(f(g(a)))).\n",
        )
        .expect("problem written");
        std::fs::write(
            &filter_path,
            "seed=GSinE(CountTerms,hypos,false,1.0,100,100,10000,1.0)\n",
        )
        .expect("filters written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                "--seed-method=ld",
                "--seeds=p",
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("seeded run succeeds");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        let names = seed_names(&output);
        assert_eq!(names.len(), 2);
        assert!(names[0].contains("_SL_P1_"));
        assert!(names[1].contains("_SD_P1_"));

        let largest_path = PathBuf::from(format!("{}_seed.p", names[0]));
        let diverse_path = PathBuf::from(format!("{}_seed.p", names[1]));
        let largest = std::fs::read_to_string(&largest_path).expect("largest output exists");
        let diverse = std::fs::read_to_string(&diverse_path).expect("diverse output exists");
        assert!(largest.contains("% Seeds      : Largest"));
        assert!(diverse.contains("% Seeds      : Diverse"));

        for path in [&problem_path, &filter_path, &largest_path, &diverse_path] {
            remove_if_present(path);
        }
    }

    #[test]
    fn seeded_random_subsample_uses_global_jkiss_order() {
        let _guard = global_state_lock();
        reset_jkiss_for_tests();
        let signature = Signature::new(TypeBank::new());
        let ctrl = StructFofSpec::new(&signature);
        let config = EAxFilterConfig {
            subsample: SubsampleMethod::Random,
            sample_size: 2,
            ..EAxFilterConfig::default()
        };
        let mut seed_symbols = vec![11, 12, 13];

        subsample_seed_symbols(&ctrl, &mut seed_symbols, &config);

        assert_eq!(
            seed_symbols,
            vec![13, 11],
            "C pops seeds before assigning JKISS weights, sorts ascending, and pushes the selected symbols back"
        );
    }

    #[test]
    fn seeded_explicit_unknown_symbol_reports_usage_error() {
        let _guard = global_state_lock();
        let problem_path = temp_path("seeded-unknown-problem");
        remove_if_present(&problem_path);
        std::fs::write(&problem_path, "fof(a, axiom, p(a)).\n").expect("problem written");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "--seeds=missing",
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "User-requested symbol missing unknown while parsing option --seeds"
        );
        assert!(String::from_utf8(stdout)
            .expect("stdout is utf8")
            .contains("% Parsing "));
        remove_if_present(&problem_path);
    }

    #[test]
    fn configured_output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let problem_path = temp_path("flush-problem");
        let filter_path = temp_path("flush-filters");
        let generated_path = generated_path(&problem_path, "tiny");
        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
        std::fs::write(&problem_path, "fof(a, axiom, p(a)).\n").expect("problem written");
        std::fs::write(&filter_path, "tiny=Threshold(10000)\n").expect("filters written");
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "--tstp-in",
                "-f",
                &slash_path(&filter_path),
                &slash_path(&problem_path),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());

        for path in [&problem_path, &filter_path, &generated_path] {
            remove_if_present(path);
        }
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

    #[test]
    fn print_help_preserves_full_c_text() {
        assert_eq!(print_help(), expected_help());
    }
}
